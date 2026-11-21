//go:build windows

package desktop

import (
	"net/http"
	"net/url"
	"path/filepath"
	"strings"

	"golang.org/x/sys/windows/registry"
)

// proxyFromSystem reads the current user's WinINET settings on every request, so a change shows at once.
func proxyFromSystem(r *http.Request) (*url.URL, error) {
	k, e := registry.OpenKey(registry.CURRENT_USER, internetSettings, registry.QUERY_VALUE)
	if e != nil {
		return nil, nil
	}
	defer k.Close()
	on, _, e := k.GetIntegerValue("ProxyEnable")
	if e != nil || on == 0 {
		return nil, nil
	}
	server, _, _ := k.GetStringValue("ProxyServer")
	override, _, _ := k.GetStringValue("ProxyOverride")
	return resolveProxy(r.URL.Scheme, r.URL.Hostname(), server, override)
}

// resolveProxy resolves a WinINET ProxyServer value: host:port, or per-scheme pairs such as http=host:port;https=host:port.
func resolveProxy(scheme, host, server, override string) (*url.URL, error) {
	if host == "" || bypassProxy(host, override) {
		return nil, nil
	}
	addr := strings.TrimSpace(server)
	if strings.Contains(addr, "=") {
		byScheme := map[string]string{}
		for _, entry := range strings.Split(server, ";") {
			if name, value, ok := strings.Cut(entry, "="); ok {
				byScheme[strings.TrimSpace(name)] = strings.TrimSpace(value)
			}
		}
		addr = byScheme[scheme]
		if addr == "" {
			addr = byScheme["http"]
		}
	}
	addr = strings.TrimPrefix(strings.TrimPrefix(addr, "http://"), "https://")
	if addr == "" {
		return nil, nil
	}
	return &url.URL{Scheme: "http", Host: addr}, nil
}

// bypassProxy reports whether the ProxyOverride list covers host.
func bypassProxy(host, override string) bool {
	host = strings.ToLower(host)
	for _, pattern := range strings.Split(override, ";") {
		pattern = strings.ToLower(strings.TrimSpace(pattern))
		switch {
		case pattern == "":
		case pattern == "<local>":
			if !strings.Contains(host, ".") {
				return true
			}
		case strings.Contains(pattern, "*"):
			if ok, _ := filepath.Match(pattern, host); ok {
				return true
			}
		default:
			if host == pattern || strings.HasSuffix(host, "."+pattern) {
				return true
			}
		}
	}
	return false
}
