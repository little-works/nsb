package core

import (
	"fmt"
	"strconv"
	"strings"
)

func move(o Object, from, to string) {
	if v, ok := o[from]; ok {
		o[to] = v
		delete(o, from)
	}
}
func take(o Object, k string) any { v := o[k]; delete(o, k); return v }
func parseClash(r Remote, o Object, multi bool, s Snapshot) (Snapshot, error) {
	for _, v := range arr(o["proxies"]) {
		p := obj(v)
		if p == nil {
			return s, fmt.Errorf("invalid Clash proxy")
		}
		kind := str(p["type"])
		switch kind {
		case "ss":
			kind = "shadowsocks"
		case "socks5":
			kind = "socks"
		}
		switch kind {
		case "shadowsocks", "vmess", "vless", "trojan", "anytls", "hysteria2", "tuic", "socks", "http":
		default:
			s.Warnings = append(s.Warnings, "Skipped unsupported Clash proxy type "+kind)
			continue
		}
		move(p, "port", "server_port")
		for _, k := range []string{"alterId", "udp", "encryption", "fingerprint"} {
			delete(p, k)
		}
		switch kind {
		case "shadowsocks":
			move(p, "cipher", "method")
		case "vmess":
			move(p, "cipher", "security")
		case "vless":
			delete(p, "cipher")
		case "tuic":
			move(p, "congestion-controller", "congestion_control")
			move(p, "udp-relay-mode", "udp_relay_mode")
		case "hysteria2":
			for _, k := range []string{"up", "down"} {
				v := take(p, k)
				if v != nil {
					txt := fmt.Sprint(v)
					end := 0
					for end < len(txt) && ((txt[end] >= '0' && txt[end] <= '9') || txt[end] == '.') {
						end++
					}
					n, err := strconv.ParseFloat(txt[:end], 64)
					if err == nil && n >= 0 {
						p[k+"_mbps"] = uint64(n)
					}
				}
			}
			obfs := take(p, "obfs")
			b := obj(obfs)
			if t, ok := obfs.(string); ok {
				b = Object{"type": t}
			}
			for _, k := range []string{"password", "min-packet-size", "max-packet-size"} {
				if v := take(p, "obfs-"+k); v != nil && b != nil {
					b[strings.ReplaceAll(k, "-", "_")] = v
				}
			}
			if b != nil {
				p["obfs"] = b
			}
		}
		network := str(take(p, "network"))
		ws := obj(take(p, "ws-opts"))
		wp, wh := take(p, "ws-path"), take(p, "ws-headers")
		httpOpts, h2, grpc := obj(take(p, "http-opts")), obj(take(p, "h2-opts")), obj(take(p, "grpc-opts"))
		t := Object{"type": network}
		switch network {
		case "ws":
			if ws["path"] != nil {
				t["path"] = ws["path"]
			} else if wp != nil {
				t["path"] = wp
			}
			if ws["headers"] != nil {
				t["headers"] = ws["headers"]
			} else if wh != nil {
				t["headers"] = wh
			}
			p["transport"] = t
		case "h2", "http":
			t["type"] = "http"
			opts := httpOpts
			if network == "h2" {
				opts = h2
			}
			for _, k := range []string{"host", "path", "method", "headers"} {
				if v := opts[k]; v != nil {
					if k == "path" && arr(v) != nil {
						if len(arr(v)) > 0 {
							v = arr(v)[0]
						} else {
							v = nil
						}
					}
					t[k] = v
				}
			}
			p["transport"] = t
		case "grpc":
			if v := grpc["grpc-service-name"]; v != nil {
				t["service_name"] = v
			}
			p["transport"] = t
		case "tcp", "udp":
			p["network"] = network
		}
		tlsValue := take(p, "tls")
		tls := obj(tlsValue)
		if tls == nil {
			tls = Object{}
		}
		if b, ok := tlsValue.(bool); ok {
			tls["enabled"] = b
		}
		for _, pair := range [][2]string{{"skip-cert-verify", "insecure"}, {"sni", "server_name"}, {"servername", "server_name"}, {"alpn", "alpn"}} {
			if v := take(p, pair[0]); v != nil {
				tls[pair[1]] = v
				if tls["enabled"] == nil {
					tls["enabled"] = true
				}
			}
		}
		if fp := take(p, "client-fingerprint"); fp != nil {
			tls["utls"] = Object{"enabled": true, "fingerprint": fp}
			if tls["enabled"] == nil {
				tls["enabled"] = true
			}
		}
		if reality := obj(take(p, "reality-opts")); reality != nil {
			b := Object{"enabled": true}
			if v := reality["public-key"]; v != nil {
				b["public_key"] = v
			}
			if v := reality["short-id"]; v != nil {
				b["short_id"] = v
			}
			tls["reality"] = b
			if tls["utls"] == nil {
				tls["utls"] = Object{"enabled": true}
			}
			if tls["enabled"] == nil {
				tls["enabled"] = true
			}
		}
		if kind == "hysteria2" || kind == "tuic" || kind == "anytls" {
			tls["enabled"] = true
		}
		if len(tls) > 0 {
			p["tls"] = tls
		}
		p["type"] = kind
		if enabled(r.Keep, "clash", "proxies") {
			if err := normalizeTag(p, r, multi); err != nil {
				return s, err
			}
			s.Nodes = append(s.Nodes, p)
		}
	}
	return s, nil
}
