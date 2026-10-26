package app

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
