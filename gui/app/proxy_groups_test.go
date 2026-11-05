package app

import (
	"context"
	"errors"
	"fmt"
	"reflect"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	"github.com/egoist/mygo/ui"
	"nsb/core"
)

func TestProxyMemberOrderAndValidity(t *testing.T) {
	a := testApp(t)
	proxies := map[string]any{"group": core.Object{"now": "failed-current", "all": []any{"z-fast", "b-unknown", "failed", "A-slow", "failed-current"}}}
	a.delays = map[string]int{"z-fast": 12, "A-slow": 90, "failed": -1, "failed-current": -1}
	for _, test := range []struct {
		order int
		hide  bool
		query string
		want  []string
	}{
		{0, false, "", []string{"z-fast", "b-unknown", "failed", "A-slow", "failed-current"}},
		{1, false, "", []string{"A-slow", "b-unknown", "failed", "failed-current", "z-fast"}},
		{2, false, "", []string{"z-fast", "A-slow", "b-unknown", "failed", "failed-current"}},
		{2, true, "", []string{"z-fast", "A-slow", "b-unknown", "failed-current"}},
		{0, false, "SLOW", []string{"A-slow"}},
		{0, true, "GROUP", []string{"z-fast", "b-unknown", "A-slow", "failed-current"}},
		{0, false, "missing", []string{}},
	} {
		a.proxySort, a.proxyHideInvalid, a.proxyFilter = test.order, test.hide, test.query
		if got := a.proxyMembers(proxies, "group"); !reflect.DeepEqual(got, test.want) {
			t.Fatalf("order=%d hide=%v query=%q: %v, want %v", test.order, test.hide, test.query, got, test.want)
		}
	}
	if got := array(object(proxies["group"])["all"])[0]; got != "z-fast" {
		t.Fatal("sorting changed kernel group order")
	}
}

func TestTextIconButtonHover(t *testing.T) {
	view := ui.NewTester(func(c *ui.Context) {
		textIconButton(c, icons["speed"], "Test speed")
	}, 80, 60)
	view.Frame()
	button, ok := view.Find("Test speed")
	if !ok {
		t.Fatal("icon button must expose its accessible name")
	}
	x, y := int(button.X+4), int(button.Y+button.H/2)
	normal := view.Image().At(x, y)
	view.Move(button.X+button.W/2, button.Y+button.H/2)
	view.Frame()
	if reflect.DeepEqual(normal, view.Image().At(x, y)) {
		t.Fatal("hover must show a background")
	}
	view.Move(70, 50)
	view.Frame()
	if !reflect.DeepEqual(normal, view.Image().At(x, y)) {
		t.Fatal("hover background must clear on pointer exit")
	}
}

func TestMeasureProxiesBoundsConcurrencyAndPublishesFailures(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	started := make(chan struct{}, 8)
	release := make(chan struct{})
	done := make(chan struct{})
	var active, peak atomic.Int32
	var mu sync.Mutex
	results := map[string]int{}
	nodes := []string{"a", "b", "c", "d", "e", "f", "failed", "zero"}
	go func() {
		defer close(done)
		measureProxies(ctx, nodes, func(ctx context.Context, node string) (int, error) {
			n := active.Add(1)
			defer active.Add(-1)
			for old := peak.Load(); n > old && !peak.CompareAndSwap(old, n); old = peak.Load() {
			}
			started <- struct{}{}
			select {
			case <-release:
			case <-ctx.Done():
				return 0, ctx.Err()
			}
			if node == "failed" {
				return 0, errors.New("timeout")
			}
			if node == "zero" {
				return 0, nil
			}
			return 42, nil
		}, func(node string, delay int) {
			mu.Lock()
			defer mu.Unlock()
			results[node] = delay
		})
	}()
	for range 4 {
		select {
		case <-started:
		case <-ctx.Done():
			t.Fatal("workers did not start")
		}
	}
	if peak.Load() != 4 {
		t.Fatal("expected four concurrent probes", peak.Load())
	}
	close(release)
	select {
	case <-done:
	case <-ctx.Done():
		t.Fatal("probes did not complete")
	}
	if peak.Load() > 4 || len(results) != len(nodes) || results["a"] != 42 || results["failed"] != -1 || results["zero"] != -1 {
		t.Fatalf("unexpected probe results: %v (peak %d)", results, peak.Load())
	}
}

func TestMeasureProxiesCancellation(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	measureProxies(ctx, []string{"a", "b"}, func(context.Context, string) (int, error) {
		t.Error("cancelled test started a probe")
		return 1, nil
	}, func(string, int) { t.Error("cancelled result published") })
}

func TestProxyResultsResetOnProfileOrKernelChange(t *testing.T) {
	a := testApp(t)
	a.snapshot.Running = true
	a.snapshot.Uptime = time.Minute
	for _, change := range []string{"profile", "restart", "stop"} {
		a.delays["node"] = 42
		a.proxyTesting["node"] = true
		cancelled := false
		a.proxyTestCancel = func() { cancelled = true }
		s := a.snapshot
		switch change {
		case "profile":
			id := "new"
			s.Config.CurrentProfileID = &id
		case "restart":
			s.Uptime = 0
		case "stop":
			s.Running = false
		}
		a.updateSnapshot(s)
		if !cancelled || len(a.delays) != 0 || len(a.proxyTesting) != 0 {
			t.Fatalf("%s retained stale results", change)
		}
	}
}

func TestLocateCurrentProxyAfterFiltering(t *testing.T) {
	a := testApp(t)
	a.page = "proxies"
	a.snapshot.Running = true
	members := []any{}
	proxies := core.Object{}
	for i := range 100 {
		name := fmt.Sprintf("node-%03d", i)
		members = append(members, name)
		proxies[name] = core.Object{"type": "Shadowsocks"}
	}
	proxies["PROXY"] = core.Object{"type": "Selector", "now": "node-099", "all": members}
	a.snapshot.Proxies = core.Object{"proxies": proxies}
	a.proxyFilter = "node-000"
	a.proxyHideInvalid = true
	a.delays["node-099"] = -1
	view := ui.NewTester(a.View, 960, 700)
	view.Frame()
	if _, ok := view.Find("定位当前"); ok {
		t.Fatal("page toolbar must not expose a global locate action")
	}
	click(t, view, "定位当前：PROXY")
	view.Frame()
	view.Frame()
	viewport, ok := view.Find("代理分组")
	if !ok {
		t.Fatal("proxy viewport missing")
	}
	selected, ok := view.Find("node-099")
	if !ok || selected.Y < viewport.Y || selected.Y+selected.H > viewport.Y+viewport.H {
		t.Fatalf("selected node did not scroll into view: %+v in %+v", selected, viewport)
	}
	if a.proxyFilter != "" || !a.proxyHideInvalid {
		t.Fatal("locate must clear search and preserve validity filtering")
	}
	view.Scroll(viewport.X+20, viewport.Y+20, 0, -500)
	view.Frame()
	selected, _ = view.Find("node-099")
	if selected.Y < viewport.Y+viewport.H {
		t.Fatal("one-shot locate prevented subsequent scrolling")
	}
}

func TestProxyGroupNavigation(t *testing.T) {
	a := testApp(t)
	a.page = "proxies"
	a.snapshot.Running = true
	proxies := core.Object{}
	members := []any{}
	for i := range 100 {
		name := fmt.Sprintf("node-%03d", i)
		members = append(members, name)
		proxies[name] = core.Object{"type": "Shadowsocks"}
	}
	proxies["PROXY"] = core.Object{"type": "Selector", "now": "node-000", "all": members}
	proxies["Z group"] = core.Object{"type": "Selector", "now": "last node", "all": []any{"last node"}}
	proxies["last node"] = core.Object{"type": "Shadowsocks"}
	a.snapshot.Proxies = core.Object{"proxies": proxies}
	view := ui.NewTester(a.View, 960, 700)
	view.Frame()
	click(t, view, "组导航")
	view.Key(0, ui.KeyEnd)
	view.Key(0, ui.KeyEnter)
	view.Frame()
	if a.proxyGroup != "Z group" {
		t.Fatal("group selector did not update navigation")
	}
	viewport, _ := view.Find("代理分组")
	button, ok := view.Find("组测速：Z group")
	if !ok || button.Y < viewport.Y || button.Y+button.H > viewport.Y+viewport.H {
		t.Fatalf("target group header is not visible: %+v in %+v", button, viewport)
	}
}

func TestSingleProxyTestOnlyDisablesItsOwnButton(t *testing.T) {
	a := testApp(t)
	a.snapshot.Running = true
	a.snapshot.Proxies = core.Object{"proxies": core.Object{
		"PROXY": core.Object{"type": "Selector", "now": "first", "all": []any{"first", "second"}},
		"first": core.Object{"type": "Shadowsocks"}, "second": core.Object{"type": "Shadowsocks"},
	}}
	a.proxyTesting["first"] = true
	view := ui.NewTester(a.View, 960, 700)
	view.Frame()
	foundSecond := false
	for range 30 {
		view.Key(0, ui.KeyTab)
		view.Frame()
		if view.Focused("测速：first") && !view.Focused("first") {
			t.Fatal("pending node's test button must be disabled")
		}
		if view.Focused("测速：second") && !view.Focused("second") {
			foundSecond = true
		}
	}
	if !foundSecond {
		t.Fatal("other nodes must remain available while one node is testing")
	}
}

func TestProxyToolsFitNarrowWindow(t *testing.T) {
	a := testApp(t)
	a.page = "proxies"
	a.snapshot.Running = true
	a.snapshot.Proxies = core.Object{"proxies": core.Object{"PROXY": core.Object{"type": "Selector", "now": "node", "all": []any{"node"}}, "node": core.Object{"type": "Shadowsocks"}}}
	view := ui.NewTester(a.View, 960, 650)
	for _, language := range []string{"zh-CN", "en-US"} {
		a.language = language
		for _, testing := range []bool{false, true} {
			if testing {
				a.proxyTestCancel = func() {}
				a.proxyTestDone, a.proxyTestTotal = 123, 999
			} else {
				a.proxyTestCancel = nil
			}
			view.Frame()
			view.Frame()
			search, _ := view.Find(a.t("Search nodes or groups…", "搜索节点或代理组…"))
			for _, label := range []string{a.t("Group navigation", "组导航"), a.t("Node order", "节点排序"), a.t("Hide failed", "隐藏无效节点")} {
				r, ok := view.Find(label)
				if !ok || r.X < 232 || r.X+r.W > 960 {
					t.Fatalf("%s tool overflows: %+v", language, r)
				}
				if r.Y < search.Y || r.Y+r.H > search.Y+search.H {
					t.Fatalf("%s filter must share the search row: %+v vs %+v", language, r, search)
				}
			}
			label := a.t("Test all", "全部测速")
			if testing {
				label = a.t("Stop testing", "停止测速")
			}
			r, ok := view.Find(label)
			if !ok || r.X+r.W > 960 {
				t.Fatalf("%s test action overflows: %+v", language, r)
			}
		}
	}
}
