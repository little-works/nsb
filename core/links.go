package core

import (
	"encoding/base64"
	"fmt"
	"net/url"
	"strconv"
	"strings"
	"unicode"
)

func parseLinks(r Remote, body string, multi bool, s Snapshot) (Snapshot, error) {
	compact := strings.Map(func(c rune) rune {
		if unicode.IsSpace(c) {
			return -1
		}
		return c
	}, body)
	for _, enc := range []*base64.Encoding{base64.StdEncoding, base64.RawStdEncoding, base64.URLEncoding, base64.RawURLEncoding} {
		if b, err := enc.DecodeString(compact); err == nil && strings.Contains(string(b), "://") {
			body = string(b)
			break
		}
	}
	seen := map[string]bool{}
	valid := 0
	for i, line := range strings.Split(body, "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		p, err := parseLink(line)
		if err != nil {
			kind := "invalid"
			if err.Error() == "unsupported link scheme" {
				kind = "unsupported"
			}
			s.Warnings = append(s.Warnings, fmt.Sprintf("Skipped %s link on line %d", kind, i+1))
			continue
		}
		u, _ := url.Parse(line)
		tag := strings.TrimSpace(strings.Map(func(c rune) rune {
			if unicode.IsControl(c) {
				return -1
			}
			return c
		}, u.Fragment))
		if tag == "" {
			tag = fmt.Sprintf("%s-%s-%v", u.Scheme, u.Hostname(), p["server_port"])
		}
		base := tag
		for n := 1; ; n++ {
			if n > 1 {
				tag = fmt.Sprintf("%s-%d", base, n)
			}
			q := qualified(tag, r, multi)
			if !seen[q] {
				seen[q] = true
				p["tag"] = q
				break
			}
		}
		valid++
		if enabled(r.Keep, "outbounds") {
			s.Nodes = append(s.Nodes, p)
		}
	}
	if valid == 0 {
		return s, fmt.Errorf("remote contains no valid supported links")
	}
	return s, nil
}
func parseLink(link string) (Object, error) {
	u, err := url.Parse(link)
	if err != nil || u.Hostname() == "" {
		return nil, fmt.Errorf("invalid link")
	}
	q := u.Query()
	get := func(keys ...string) string {
		for k, vs := range q {
			for _, key := range keys {
				if strings.EqualFold(k, key) && len(vs) > 0 {
					return vs[0]
				}
			}
		}
		return ""
	}
	port := 443
	if u.Port() != "" {
		port, err = strconv.Atoi(u.Port())
		if err != nil || port < 1 || port > 65535 {
			return nil, fmt.Errorf("invalid port")
		}
	}
	kind := strings.ToLower(u.Scheme)
	if kind == "hy2" {
		kind = "hysteria2"
	}
	p := Object{"type": kind, "server": u.Hostname(), "server_port": port}
	user, pass := "", ""
	if u.User != nil {
		user = u.User.Username()
		pass, _ = u.User.Password()
	}
	password := get("password")
	security := get("security")
	reality := strings.EqualFold(security, "reality")
	defTLS := true
	switch kind {
	case "vless":
		id := get("uuid")
		if id == "" {
			id = user
		}
		if id == "" {
			return nil, fmt.Errorf("missing uuid")
		}
		p["uuid"] = id
		if f := get("flow"); f != "" {
			p["flow"] = f
		}
		defTLS = false
		if reality && get("pbk", "public-key") == "" {
			return nil, fmt.Errorf("missing reality public key")
		}
		if mode := get("mode"); mode == "multi" || mode == "multiplex" {
			p["multiplex"] = Object{"enabled": true}
		}
	case "hysteria2":
		if password == "" {
			password = user
			if pass != "" {
				password = pass
				if user != "" {
					password = user + ":" + pass
				}
			}
		}
		p["password"] = password
		if obfs := get("obfs"); obfs != "" && !strings.EqualFold(obfs, "none") {
			b := Object{"type": obfs}
			if v := get("obfs-password"); v != "" {
				b["password"] = v
			}
			p["obfs"] = b
		}
	case "anytls":
		if password == "" {
			password = pass
		}
		if password == "" {
			password = user
		}
		p["password"] = password
	case "tuic":
		if password == "" {
			password = pass
		}
		if user == "" {
			return nil, fmt.Errorf("missing uuid")
		}
		p["uuid"] = user
		p["password"] = password
		for _, key := range []string{"congestion_control", "udp_relay_mode"} {
			v := get(key, strings.ReplaceAll(key, "_", "-"))
			if key == "congestion_control" && v == "" {
				v = get("congestion-controller")
			}
			if v != "" {
				p[key] = v
			}
		}
	default:
		return nil, fmt.Errorf("unsupported link scheme")
	}
	if kind != "vless" && password == "" {
		return nil, fmt.Errorf("missing password")
	}
	if kind == "vless" {
		network := strings.ToLower(get("type", "transport"))
		switch network {
		case "", "tcp", "udp":
			if network != "" {
				p["network"] = network
			}
		case "ws", "http", "h2", "grpc", "quic", "httpupgrade":
			typ := network
			if typ == "h2" {
				typ = "http"
			}
			t := Object{"type": typ}
			if path := get("path"); path != "" {
				t["path"] = path
			}
			if host := get("host", "authority"); host != "" {
				switch typ {
				case "ws":
					t["headers"] = Object{"Host": host}
				case "http":
					t["host"] = []any{host}
				case "httpupgrade":
					t["host"] = host
				}
			}
			if typ == "grpc" {
				if v := get("serviceName", "service-name", "service_name"); v != "" {
					t["service_name"] = v
				}
			}
			p["transport"] = t
		default:
			return nil, fmt.Errorf("invalid transport")
		}
	}
	on := defTLS || security == "tls" || reality
	if strings.EqualFold(security, "none") {
		on = false
	}
	tls := Object{"enabled": on}
	sni := get("sni", "servername", "server_name")
	if sni != "" {
		tls["server_name"] = sni
	}
	insecure := get("insecure", "allow_insecure", "skip-cert-verify")
	if insecure != "" {
		switch strings.ToLower(insecure) {
		case "1", "true", "yes", "on":
			tls["insecure"] = true
		case "0", "false", "no", "off":
			tls["insecure"] = false
		}
	}
	if alpn := get("alpn"); alpn != "" {
		tls["alpn"] = strings.Split(alpn, ",")
	}
	if fp := get("fp", "client-fingerprint"); fp != "" {
		tls["utls"] = Object{"enabled": true, "fingerprint": fp}
	} else if reality {
		tls["utls"] = Object{"enabled": true}
	}
	pbk, sid := get("pbk", "public-key"), get("sid", "short-id")
	if reality || pbk != "" || sid != "" {
		b := Object{"enabled": true}
		if pbk != "" {
			b["public_key"] = pbk
		}
		if sid != "" {
			b["short_id"] = sid
		}
		tls["reality"] = b
	}
	if on || len(tls) > 1 {
		p["tls"] = tls
	}
	return p, nil
}
