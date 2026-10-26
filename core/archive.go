package core

import (
	_ "embed"
	"encoding/json"
	"fmt"
	"strings"
	"time"
)

//go:embed default-template.json
var DefaultTemplate string

type PortableTemplate struct {
	Name    string `json:"name"`
	Content string `json:"content"`
}
type PortableProfile struct {
	Name                string   `json:"name"`
	TemplateName        *string  `json:"template_name"`
	InlineTemplate      *string  `json:"inline_template"`
	Remotes             []Remote `json:"remotes"`
	Hook                *string  `json:"hook"`
	UpdateIntervalHours *uint32  `json:"update_interval_hours"`
	UpdateCron          *string  `json:"update_cron"`
}
type Archive struct {
	Format    string             `json:"format"`
	Version   int                `json:"version"`
	Templates []PortableTemplate `json:"templates"`
	Profiles  []PortableProfile  `json:"profiles"`
}

func Export(c Config) Archive {
	a := Archive{Format: "nsb-portable-data", Version: 1, Templates: []PortableTemplate{}, Profiles: []PortableProfile{}}
	for _, t := range c.Templates {
		a.Templates = append(a.Templates, PortableTemplate{t.Name, t.Content})
	}
	for _, p := range c.Profiles {
		var name *string
		for _, t := range c.Templates {
			if t.ID == p.TemplateID {
				v := t.Name
				name = &v
				break
			}
		}
		a.Profiles = append(a.Profiles, PortableProfile{p.Name, name, p.InlineTemplate, p.Remotes, p.Hook, p.UpdateIntervalHours, p.UpdateCron})
	}
	return a
}

type ImportReport struct {
	Added, Overwritten, Skipped int
	Issues                      []string
}

func Import(data []byte, current Config) (Config, ImportReport, error) {
	c := Clone(current)
	report := ImportReport{}
	var raw struct {
		Format    string            `json:"format"`
		Version   int               `json:"version"`
		Templates []json.RawMessage `json:"templates"`
		Profiles  []json.RawMessage `json:"profiles"`
	}
	if e := json.Unmarshal(data, &raw); e != nil {
		return c, report, e
	}
	if raw.Format != "nsb-portable-data" || raw.Version != 1 {
		return c, report, fmt.Errorf("unsupported archive format or version")
	}
	skip := func(name string, err error) {
		report.Skipped++
		report.Issues = append(report.Issues, name+": "+err.Error())
	}
	for _, b := range raw.Templates {
		var t PortableTemplate
		if e := json.Unmarshal(b, &t); e != nil {
			skip("template", e)
			continue
		}
		var o Object
		if strings.TrimSpace(t.Name) == "" || json.Unmarshal([]byte(t.Content), &o) != nil || o == nil {
			skip(t.Name, fmt.Errorf("invalid template"))
			continue
		}
		i := -1
		for j, v := range c.Templates {
			if v.Name == t.Name {
				i = j
				break
			}
		}
		if i < 0 {
			c.Templates = append(c.Templates, Template{ID: NewID()})
			i = len(c.Templates) - 1
			report.Added++
		} else {
			report.Overwritten++
		}
		c.Templates[i].Name = t.Name
		c.Templates[i].Content = t.Content
		c.Templates[i].UpdatedAt = uint64(time.Now().Unix())
	}
	for _, b := range raw.Profiles {
		var pp PortableProfile
		if e := json.Unmarshal(b, &pp); e != nil {
			skip("profile", e)
			continue
		}
		p := Profile{ID: NewID(), Name: pp.Name, InlineTemplate: pp.InlineTemplate, Remotes: pp.Remotes, Hook: pp.Hook, UpdateIntervalHours: pp.UpdateIntervalHours, UpdateCron: pp.UpdateCron}
		if pp.TemplateName != nil {
			for _, t := range c.Templates {
				if t.Name == *pp.TemplateName {
					p.TemplateID = t.ID
					break
				}
			}
		}
		if strings.TrimSpace(p.Name) == "" {
			skip(p.Name, fmt.Errorf("missing name"))
			continue
		}
		if _, e := c.TemplateFor(p); e != nil {
			skip(p.Name, e)
			continue
		}
		if e := ValidateSchedule(p); e != nil {
			skip(p.Name, e)
			continue
		}
		if e := (Config{Templates: c.Templates, Profiles: []Profile{p}}).Validate(); e != nil {
			skip(p.Name, e)
			continue
		}
		i := -1
		for j, v := range c.Profiles {
			if v.Name == p.Name {
				i = j
				break
			}
		}
		if i < 0 {
			c.Profiles = append(c.Profiles, p)
			report.Added++
		} else {
			p.ID = c.Profiles[i].ID
			p.Revision = c.Profiles[i].Revision + 1
			c.Profiles[i] = p
			report.Overwritten++
		}
	}
	if e := c.Validate(); e != nil {
		return current, report, e
	}
	return c, report, nil
}
