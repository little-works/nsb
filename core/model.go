package core

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net/url"
	"strconv"
	"strings"
)

type Object = map[string]any

type Header struct {
	Key   string `json:"key"`
	Value string `json:"value"`
}
type Remote struct {
	Name    string   `json:"name"`
	URL     string   `json:"url"`
	Headers []Header `json:"headers"`
	Format  string   `json:"format"`
	Keep    Object   `json:"keep"`
}

func DefaultKeep() Object { return Object{"outbounds": true, "clash": Object{"proxies": true}} }
func (r *Remote) UnmarshalJSON(b []byte) error {
	type plain Remote
	p := plain{Format: "clash", Keep: DefaultKeep(), Headers: []Header{}}
	if err := json.Unmarshal(b, &p); err != nil {
		return err
	}
	keep := DefaultKeep()
	merge(keep, p.Keep)
	p.Keep = keep
	*r = Remote(p)
	return nil
}
func NewID() string {
	var b [16]byte
	if _, err := rand.Read(b[:]); err != nil {
		panic(err)
	}
	return hex.EncodeToString(b[:])
}

type Profile struct {
	ID                  string   `json:"id"`
	Name                string   `json:"name"`
	TemplateID          string   `json:"template_id"`
	InlineTemplate      *string  `json:"inline_template"`
	Remotes             []Remote `json:"remotes"`
	Hook                *string  `json:"hook"`
	UpdateIntervalHours *uint32  `json:"update_interval_hours"`
	UpdateCron          *string  `json:"update_cron"`
	UpdatedAt           uint64   `json:"updated_at"`
	NextUpdateAt        uint64   `json:"next_update_at"`
	LastAttemptAt       uint64   `json:"last_attempt_at"`
	LastUpdateError     *string  `json:"last_update_error"`
	Revision            uint64   `json:"revision"`
}
type Template struct {
	ID        string `json:"id"`
	Name      string `json:"name"`
	Content   string `json:"content"`
	UpdatedAt uint64 `json:"updated_at"`
}
type Config struct {
	AppPort            uint16     `json:"app_port"`
	SystemProxyEnabled bool       `json:"system_proxy_enabled"`
	AppLanguage        string     `json:"app_language"`
	AppTheme           string     `json:"app_theme,omitempty"`
	Profiles           []Profile  `json:"profiles"`
	Templates          []Template `json:"templates"`
	CurrentProfileID   *string    `json:"current_profile_id"`
}

func DefaultConfig() Config {
	return Config{AppPort: 8787, AppLanguage: "zh-CN", AppTheme: "system", Profiles: []Profile{}, Templates: []Template{}}
}

func legacyFields(b []byte, timestamps ...string) ([]byte, error) {
	var fields map[string]json.RawMessage
	if e := json.Unmarshal(b, &fields); e != nil {
		return nil, e
	}
	for _, k := range timestamps {
		if raw, ok := fields[k]; ok {
			var s string
			if string(raw) == "null" {
				fields[k] = json.RawMessage("0")
			} else if json.Unmarshal(raw, &s) == nil {
				n, _ := strconv.ParseUint(strings.TrimSpace(s), 10, 64)
				fields[k] = json.RawMessage(strconv.FormatUint(n, 10))
			}
		}
	}
	if raw, ok := fields["id"]; ok {
		var id string
		if e := json.Unmarshal(raw, &id); e != nil {
			return nil, e
		}
		id = strings.TrimSpace(id)
		if id == "" {
			id = NewID()
		}
		fields["id"], _ = json.Marshal(id)
	} else {
		fields["id"], _ = json.Marshal(NewID())
	}
	return json.Marshal(fields)
}
func (p *Profile) UnmarshalJSON(b []byte) error {
	b, e := legacyFields(b, "updated_at", "next_update_at", "last_attempt_at")
	if e != nil {
		return e
	}
	type plain Profile
	var v plain
	if e = json.Unmarshal(b, &v); e != nil {
		return e
	}
	*p = Profile(v)
	return nil
}
func (t *Template) UnmarshalJSON(b []byte) error {
	b, e := legacyFields(b, "updated_at")
	if e != nil {
		return e
	}
	type plain Template
	var v plain
	if e = json.Unmarshal(b, &v); e != nil {
		return e
	}
	*t = Template(v)
	return nil
}
func (c Config) TemplateFor(p Profile) (string, error) {
	if p.InlineTemplate != nil {
		return *p.InlineTemplate, nil
	}
	for _, t := range c.Templates {
		if t.ID == p.TemplateID {
			return t.Content, nil
		}
	}
	return "", fmt.Errorf("template for %q not found", p.Name)
}
func (c Config) Validate() error {
	ids := map[string]bool{}
	names := map[string]bool{}
	for _, t := range c.Templates {
		if t.ID == "" || ids[t.ID] || strings.TrimSpace(t.Name) == "" || names[t.Name] {
			return fmt.Errorf("invalid or duplicate template: %q", t.Name)
		}
		ids[t.ID] = true
		names[t.Name] = true
		var o Object
		if err := json.Unmarshal([]byte(t.Content), &o); err != nil || o == nil {
			return fmt.Errorf("invalid template %q", t.Name)
		}
	}
	ids = map[string]bool{}
	names = map[string]bool{}
	for _, p := range c.Profiles {
		if p.ID == "" || ids[p.ID] || strings.TrimSpace(p.Name) == "" || names[p.Name] {
			return fmt.Errorf("invalid or duplicate profile: %q", p.Name)
		}
		ids[p.ID] = true
		names[p.Name] = true
		template, err := c.TemplateFor(p)
		if err != nil {
			return err
		}
		var o Object
		if json.Unmarshal([]byte(template), &o) != nil || o == nil {
			return fmt.Errorf("profile %s requires a JSON object template", p.Name)
		}
		seen := map[string]bool{}
		for _, r := range p.Remotes {
			u, e := url.Parse(r.URL)
			if e != nil || u.Host == "" || (u.Scheme != "http" && u.Scheme != "https") {
				return fmt.Errorf("invalid source URL for %s", r.Name)
			}
			if strings.TrimSpace(r.Name) == "" || seen[r.Name] {
				return fmt.Errorf("source names must be nonempty and unique")
			}
			seen[r.Name] = true
			switch r.Format {
			case "", "clash", "singbox", "links":
			default:
				return fmt.Errorf("unsupported source format %q", r.Format)
			}
		}
		if err := ValidateSchedule(p); err != nil {
			return err
		}
	}
	return nil
}
func Clone[T any](v T) T {
	b, err := json.Marshal(v)
	if err != nil {
		panic(err)
	}
	var out T
	if err = json.Unmarshal(b, &out); err != nil {
		panic(err)
	}
	return out
}
func str(v any) string { s, _ := v.(string); return s }
func obj(v any) Object { o, _ := v.(map[string]any); return o }
func arr(v any) []any  { a, _ := v.([]any); return a }
func enabled(o Object, path ...string) bool {
	var v any = o
	for _, k := range path {
		v = obj(v)[k]
	}
	b, _ := v.(bool)
	return b
}
func Pretty(v any) string { b, _ := json.MarshalIndent(v, "", "  "); return string(b) }
