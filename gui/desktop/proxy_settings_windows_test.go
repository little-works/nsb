//go:build windows

package desktop

import "testing"

func TestSystemProxy(t *testing.T) {
	cases := []struct{ name, scheme, host, server, override, want string }{
		{"endpoint", "https", "example.com", "127.0.0.1:7890", "", "http://127.0.0.1:7890"},
		{"scheme prefix", "http", "example.com", "http://127.0.0.1:7890", "", "http://127.0.0.1:7890"},
		{"per scheme", "https", "example.com", "http=127.0.0.1:1;https=127.0.0.1:2", "", "http://127.0.0.1:2"},
		{"https falls back to http", "https", "example.com", "http=127.0.0.1:1", "", "http://127.0.0.1:1"},
		{"other scheme only", "https", "example.com", "ftp=127.0.0.1:21", "", ""},
		{"no server", "https", "example.com", "", "", ""},
		{"local bypass", "https", "intranet", "127.0.0.1:7890", "<local>", ""},
		{"wildcard bypass", "https", "10.1.2.3", "127.0.0.1:7890", "<local>;10.*", ""},
		{"domain bypass", "https", "a.example.com", "127.0.0.1:7890", "example.com", ""},
	}
	for _, c := range cases {
		got, err := resolveProxy(c.scheme, c.host, c.server, c.override)
		if err != nil {
			t.Fatalf("%s: %v", c.name, err)
		}
		url := ""
		if got != nil {
			url = got.String()
		}
		if url != c.want {
			t.Errorf("%s: got %q, want %q", c.name, url, c.want)
		}
	}
}
