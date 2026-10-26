package core

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"time"

	"github.com/tidwall/jsonc"
	"gopkg.in/yaml.v3"
)

type Snapshot struct {
	Name         string   `json:"name"`
	URL          string   `json:"url"`
	Nodes        []any    `json:"proxy_nodes"`
	Groups       []any    `json:"proxy_groups"`
	Rules        []any    `json:"route_rules"`
	Final        *string  `json:"route_final"`
	DNS          Object   `json:"dns"`
	Inbounds     []any    `json:"inbounds"`
	Route        Object   `json:"route"`
	Experimental Object   `json:"experimental"`
	Warnings     []string `json:"warnings"`
	UpdatedAt    uint64   `json:"updated_at"`
}

func ParseRemote(r Remote, body string, multi bool) (Snapshot, error) {
	s := Snapshot{Name: r.Name, URL: r.URL, Nodes: []any{}, Groups: []any{}, Rules: []any{}, Inbounds: []any{}, Warnings: []string{}, UpdatedAt: uint64(time.Now().Unix())}
	var o Object
	switch r.Format {
	case "links":
		return parseLinks(r, body, multi, s)
	case "singbox":
		if err := json.Unmarshal(jsonc.ToJSON([]byte(body)), &o); err != nil {
			return s, err
		}
		if _, ok := o["outbounds"].([]any); !ok {
			return s, fmt.Errorf("remote %s is missing outbounds", r.Name)
		}
	case "clash", "":
		if err := yaml.Unmarshal([]byte(body), &o); err != nil {
			return s, err
		}
		if o == nil {
			return s, fmt.Errorf("remote must be an object")
		}
		return parseClash(r, o, multi, s)
	default:
		return s, fmt.Errorf("unsupported remote format %q", r.Format)
	}
	if enabled(r.Keep, "outbounds") {
		for _, v := range arr(o["outbounds"]) {
			item := obj(v)
			if item == nil {
				return s, fmt.Errorf("outbound must be an object")
			}
			kind := str(item["type"])
			if err := normalizeTag(item, r, multi); err != nil {
				return s, err
			}
			rewrite(item, r, multi)
			switch kind {
			case "selector", "urltest", "fallback", "loadbalance":
				s.Groups = append(s.Groups, item)
			default:
				s.Nodes = append(s.Nodes, item)
			}
		}
	}
	route := obj(o["route"])
	if enabled(r.Keep, "route", "rules") {
		s.Rules = arr(route["rules"])
		rewrite(s.Rules, r, multi)
	}
	if enabled(r.Keep, "route", "final") {
		if f, ok := route["final"].(string); ok {
			s.Final = &f
		}
	}
	s.DNS = selected(obj(o["dns"]), obj(r.Keep["dns"]), nil)
	rewrite(s.DNS, r, multi)
	s.Route = selected(route, obj(r.Keep["route"]), map[string]bool{"rules": true, "final": true})
	rewrite(s.Route, r, multi)
	s.Experimental = selected(obj(o["experimental"]), obj(r.Keep["experimental"]), nil)
	rewrite(s.Experimental, r, multi)
	if enabled(r.Keep, "inbounds") {
		s.Inbounds = arr(o["inbounds"])
		rewrite(s.Inbounds, r, multi)
	}
	return s, nil
}
func selected(source, keep Object, exclude map[string]bool) Object {
	out := Object{}
	for k, v := range keep {
		if exclude[k] {
			continue
		}
		if nested := obj(v); nested != nil {
			if x := selected(obj(source[k]), nested, nil); len(x) > 0 {
				out[k] = x
			}
		} else if v == true {
			if x, ok := source[k]; ok {
				out[k] = x
			}
		}
	}
	if len(out) == 0 {
		return nil
	}
	return out
}
func reserved(s string) bool { return s == "PROXY" || s == "direct" || s == "block" }
func qualified(tag string, r Remote, multi bool) string {
	v := tag
	if multi {
		v = r.Name + ":" + v
	}
	if reserved(tag) {
		v = "source:" + v
	}
	return v
}
func target(tag string, r Remote, multi bool) string {
	if strings.EqualFold(tag, "DIRECT") {
		return "direct"
	}
	if tag == "REJECT" || tag == "REJECT-DROP" {
		return "block"
	}
	return qualified(tag, r, multi)
}
func rewrite(v any, r Remote, multi bool) {
	switch x := v.(type) {
	case map[string]any:
		for k, v := range x {
			if k == "outbounds" || k == "outbound" || k == "detour" {
				switch y := v.(type) {
				case string:
					x[k] = target(y, r, multi)
				case []any:
					for i, t := range y {
						if s, ok := t.(string); ok {
							y[i] = target(s, r, multi)
						}
					}
				}
			}
			rewrite(v, r, multi)
		}
	case []any:
		for _, v := range x {
			rewrite(v, r, multi)
		}
	}
}
func normalizeTag(o Object, r Remote, multi bool) error {
	tag, ok := o["tag"].(string)
	if !ok {
		tag, ok = o["name"].(string)
	}
	if !ok {
		return fmt.Errorf("outbound in remote %s is missing name", r.Name)
	}
	delete(o, "name")
	o["tag"] = qualified(tag, r, multi)
	return nil
}
func merge(dst, src Object) {
	for k, v := range src {
		if a, b := obj(dst[k]), obj(v); a != nil && b != nil {
			merge(a, b)
		} else if a, b := arr(dst[k]), arr(v); a != nil && b != nil {
			dst[k] = append(a, b...)
		} else {
			dst[k] = v
		}
	}
}

func BuildConfig(ctx context.Context, template string, remotes []Snapshot, hook string, multi bool) (string, error) {
	hookRemotes := Clone(remotes)
	var config Object
	if err := json.Unmarshal([]byte(template), &config); err != nil || config == nil {
		return "", fmt.Errorf("template must be a JSON object")
	}
	out := arr(config["outbounds"])
	rules := append([]any{}, arr(obj(config["route"])["rules"])...)
	nodes, groups, selectors := []any{}, []any{}, []any{}
	members := []any{}
	usedNames := map[string]bool{}
	tags := map[string]bool{}
	for _, v := range out {
		tags[str(obj(v)["tag"])] = true
	}
	for _, r := range remotes {
		for _, v := range append(append([]any{}, r.Nodes...), r.Groups...) {
			tags[str(obj(v)["tag"])] = true
		}
	}
	for _, r := range remotes {
		if multi {
			if reserved(r.Name) || usedNames[r.Name] || tags[r.Name] {
				return "", fmt.Errorf("remote selector name conflict: %s", r.Name)
			}
			usedNames[r.Name] = true
			rm := []any{}
			seen := map[string]bool{}
			for _, v := range append(append([]any{}, r.Groups...), r.Nodes...) {
				o := obj(v)
				t := str(o["tag"])
				if t != "" && o["type"] != "direct" && o["type"] != "block" && !seen[t] {
					rm = append(rm, t)
					seen[t] = true
				}
			}
			selectors = append(selectors, Object{"type": "selector", "tag": r.Name, "outbounds": rm})
			members = append(members, r.Name)
		}
		nodes = append(nodes, r.Nodes...)
		groups = append(groups, r.Groups...)
		rules = append(rules, r.Rules...)
		fragment := Object{}
		if r.DNS != nil {
			fragment["dns"] = r.DNS
		}
		if len(r.Inbounds) > 0 {
			fragment["inbounds"] = r.Inbounds
		}
		if r.Route != nil {
			fragment["route"] = r.Route
		}
		if r.Experimental != nil {
			fragment["experimental"] = r.Experimental
		}
		merge(config, fragment)
		if r.Final != nil {
			if obj(config["route"]) == nil {
				config["route"] = Object{}
			}
			obj(config["route"])["final"] = *r.Final
		}
	}
	out = append(out, nodes...)
	out = append(out, groups...)
	out = append(out, selectors...)
	if !multi {
		for _, v := range append(append([]any{}, groups...), nodes...) {
			o := obj(v)
			if o["type"] != "direct" && o["type"] != "block" {
				if tag, ok := o["tag"].(string); ok {
					members = append(members, tag)
				}
			}
		}
		if len(remotes) == 0 && len(members) == 0 {
			members = append(members, "direct")
		}
	}
	found := false
	for _, v := range out {
		p := obj(v)
		if p["tag"] != "PROXY" {
			continue
		}
		found = true
		old, ok := p["outbounds"].([]any)
		if !ok {
			return "", fmt.Errorf("template PROXY must contain an outbounds array")
		}
		if multi {
			p["outbounds"] = members
			valid := false
			for _, m := range members {
				if m == p["default"] {
					valid = true
				}
			}
			if !valid {
				delete(p, "default")
			}
		} else {
			for _, m := range members {
				exists := false
				for _, x := range old {
					if x == m {
						exists = true
					}
				}
				if !exists {
					old = append(old, m)
				}
			}
			p["outbounds"] = old
		}
		break
	}
	if !found {
		out = append(out, Object{"type": "selector", "tag": "PROXY", "outbounds": members})
	}
	seen := map[string]bool{}
	dedup := []any{}
	for i := len(out) - 1; i >= 0; i-- {
		tag := str(obj(out[i])["tag"])
		key := "tag:" + tag
		if tag == "" {
			key = "value:" + Pretty(out[i])
		}
		if !seen[key] {
			dedup = append(dedup, out[i])
			seen[key] = true
		}
	}
	for i, j := 0, len(dedup)-1; i < j; i, j = i+1, j-1 {
		dedup[i], dedup[j] = dedup[j], dedup[i]
	}
	config["outbounds"] = dedup
	if len(rules) > 0 {
		if obj(config["route"]) == nil {
			config["route"] = Object{}
		}
		obj(config["route"])["rules"] = rules
	}
	input := Object{"singbox": config}
	if multi {
		input["remotes"] = hookRemotes
	} else if len(remotes) > 0 {
		input["remote"] = hookRemotes[0]
	}
	result, err := RunHook(ctx, hook, "onGenerate", input)
	if err != nil {
		return "", err
	}
	b, err := json.MarshalIndent(result, "", "  ")
	return string(b), err
}
