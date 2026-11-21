//go:build !windows

package desktop

import (
	"net/http"
	"net/url"
)

// proxyFromSystem follows the environment's proxy settings.
func proxyFromSystem(r *http.Request) (*url.URL, error) { return http.ProxyFromEnvironment(r) }
