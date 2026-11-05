package app

import (
	"context"
	"sort"
	"strings"
	"sync"

	"nsb/gui/desktop"
)

func (a *App) updateSnapshot(s desktop.Snapshot) {
	previous, current := "", ""
	if a.snapshot.Config.CurrentProfileID != nil {
		previous = *a.snapshot.Config.CurrentProfileID
	}
	if s.Config.CurrentProfileID != nil {
		current = *s.Config.CurrentProfileID
	}
	if previous != current || a.snapshot.Running != s.Running || s.Uptime < a.snapshot.Uptime {
		a.stopProxyTests()
		clear(a.delays)
	}
	a.snapshot = s
}

func (a *App) proxyMembers(proxies map[string]any, group string) []string {
	g := object(proxies[group])
	query := strings.ToLower(strings.TrimSpace(a.proxyFilter))
	members := []string{}
	for _, v := range array(g["all"]) {
		name := text(v)
		if query != "" && !strings.Contains(strings.ToLower(group+" "+name), query) {
			continue
		}
		if a.proxyHideInvalid && a.delays[name] < 0 && name != text(g["now"]) {
			continue
		}
		members = append(members, name)
	}
	sort.SliceStable(members, func(i, j int) bool {
		if a.proxySort == 1 {
			return strings.ToLower(members[i]) < strings.ToLower(members[j])
		}
		if a.proxySort == 2 {
			left, right := a.delays[members[i]], a.delays[members[j]]
			if (left > 0) != (right > 0) {
				return left > 0
			}
			if left <= 0 && right <= 0 {
				return left == 0 && right < 0
			}
			return left < right
		}
		return false
	})
	return members
}

func measureProxies(ctx context.Context, nodes []string, measure func(context.Context, string) (int, error), publish func(string, int)) {
	var workers sync.WaitGroup
	jobs := make(chan string)
	for range min(4, len(nodes)) {
		workers.Go(func() {
			for node := range jobs {
				if ctx.Err() != nil {
					return
				}
				delay, err := measure(ctx, node)
				if ctx.Err() != nil {
					return
				}
				if err != nil || delay <= 0 {
					delay = -1
				}
				publish(node, delay)
			}
		})
	}
	for _, node := range nodes {
		select {
		case jobs <- node:
		case <-ctx.Done():
			close(jobs)
			workers.Wait()
			return
		}
	}
	close(jobs)
	workers.Wait()
}

func (a *App) stopProxyTests() {
	if a.proxyTestCancel != nil {
		a.proxyTestCancel()
	}
	a.proxyTestCancel = nil
	for _, cancel := range a.proxySingleCancel {
		cancel()
	}
	clear(a.proxySingleCancel)
	a.proxyTestID++
	clear(a.proxyTesting)
}

func (a *App) measureProxy(ctx context.Context, node string) (int, error) {
	select {
	case a.proxyTestSlots <- struct{}{}:
		defer func() { <-a.proxyTestSlots }()
	case <-ctx.Done():
		return 0, ctx.Err()
	}
	return a.manager.Latency(ctx, node)
}

func (a *App) testProxy(node string) {
	if a.proxyTesting[node] || a.quitting || !a.snapshot.Running {
		return
	}
	ctx, cancel := context.WithCancel(a.ctx)
	a.proxySingleCancel[node] = cancel
	a.proxyTesting[node] = true
	a.workers.Go(func() {
		delay, err := a.measureProxy(ctx, node)
		a.window.Update(func() {
			if ctx.Err() != nil || a.quitting {
				return
			}
			cancel()
			delete(a.proxySingleCancel, node)
			delete(a.proxyTesting, node)
			if err != nil || delay <= 0 {
				delay = -1
			}
			a.delays[node] = delay
		})
	})
}

func (a *App) testProxies(nodes []string) {
	if a.proxyTestCancel != nil || a.quitting || !a.snapshot.Running {
		return
	}
	seen := map[string]bool{}
	unique := []string{}
	for _, node := range nodes {
		if !seen[node] && !a.proxyTesting[node] {
			seen[node] = true
			unique = append(unique, node)
			a.proxyTesting[node] = true
		}
	}
	if len(unique) == 0 {
		return
	}
	ctx, cancel := context.WithCancel(a.ctx)
	a.proxyTestCancel = cancel
	a.proxyTestID++
	id := a.proxyTestID
	profile := a.snapshot.Config.CurrentProfileID
	profileID := ""
	if profile != nil {
		profileID = *profile
	}
	a.proxyTestDone, a.proxyTestTotal = 0, len(unique)
	a.workers.Go(func() {
		defer cancel()
		measureProxies(ctx, unique, a.measureProxy, func(node string, delay int) {
			a.window.Update(func() {
				if a.quitting || a.proxyTestID != id {
					return
				}
				currentID := ""
				if a.snapshot.Config.CurrentProfileID != nil {
					currentID = *a.snapshot.Config.CurrentProfileID
				}
				if currentID != profileID || !a.snapshot.Running {
					a.stopProxyTests()
					return
				}
				a.delays[node] = delay
				delete(a.proxyTesting, node)
				a.proxyTestDone++
				if a.proxyTestDone == a.proxyTestTotal {
					a.proxyTestCancel = nil
				}
			})
		})
	})
}

func proxyColumns(windowWidth float32) int { return max(1, int((windowWidth-232-56-40+10)/250)) }

func (a *App) proxyGroups(proxies map[string]any) []string {
	names := []string{}
	seen := map[string]bool{}
	add := func(name string) {
		if _, ok := object(proxies[name])["all"].([]any); ok && !seen[name] {
			names = append(names, name)
			seen[name] = true
		}
	}
	add("PROXY")
	for _, p := range a.snapshot.Config.Profiles {
		if a.snapshot.Config.CurrentProfileID != nil && p.ID == *a.snapshot.Config.CurrentProfileID {
			for _, r := range p.Remotes {
				add(r.Name)
			}
			break
		}
	}
	for _, name := range sortedKeys(proxies) {
		if name != "GLOBAL" {
			add(name)
		}
	}
	add("GLOBAL")
	return names
}
