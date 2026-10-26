package core

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

type Service struct {
	Client *http.Client
	Store  *Store
}

func (s Service) Download(ctx context.Context, r Remote) (string, error) {
	u, e := url.Parse(r.URL)
	if e != nil || u.Host == "" || (u.Scheme != "http" && u.Scheme != "https") {
		return "", fmt.Errorf("invalid subscription URL")
	}
	req, e := http.NewRequestWithContext(ctx, http.MethodGet, r.URL, nil)
	if e != nil {
		return "", e
	}
	req.Header.Set("User-Agent", "ClashforWindows/0.20.16 clash-verge/2.5.2 NSB/0.1.0")
	for _, h := range r.Headers {
		req.Header.Set(strings.TrimSpace(h.Key), h.Value)
	}
	client := s.Client
	if client == nil {
		client = &http.Client{Timeout: 60 * time.Second}
	}
	resp, e := client.Do(req)
	if e != nil {
		return "", fmt.Errorf("download remote %s failed: %w", r.Name, e)
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return "", fmt.Errorf("remote %s returned HTTP %d", r.Name, resp.StatusCode)
	}
	b, e := io.ReadAll(io.LimitReader(resp.Body, 32<<20+1))
	if e != nil {
		return "", e
	}
	if len(b) > 32<<20 {
		return "", fmt.Errorf("subscription exceeds 32 MiB")
	}
	return string(b), nil
}
func (s Service) Generate(ctx context.Context, c Config, p Profile, refresh bool) (string, []string, error) {
	template, e := c.TemplateFor(p)
	if e != nil {
		return "", nil, e
	}
	snapshots := []Snapshot{}
	warnings := []string{}
	for _, r := range p.Remotes {
		if e = ctx.Err(); e != nil {
			return "", nil, e
		}
		var body string
		if !refresh && s.Store != nil {
			body, _ = s.Store.ReadRemote(r.URL)
		}
		if body == "" {
			body, e = s.Download(ctx, r)
			if e != nil {
				return "", nil, e
			}
		}
		snap, e := ParseRemote(r, body, len(p.Remotes) > 1)
		if e != nil {
			return "", nil, fmt.Errorf("%s: %w", r.Name, e)
		}
		if s.Store != nil {
			if e = s.Store.WriteRemote(r.URL, body); e != nil {
				return "", nil, e
			}
		}
		snapshots = append(snapshots, snap)
		warnings = append(warnings, snap.Warnings...)
	}
	hook := ""
	if p.Hook != nil {
		hook = *p.Hook
	}
	result, e := BuildConfig(ctx, template, snapshots, hook, len(snapshots) > 1)
	return result, warnings, e
}

func ValidateSchedule(p Profile) error {
	if p.UpdateCron != nil && strings.TrimSpace(*p.UpdateCron) != "" {
		_, e := parseSchedule(*p.UpdateCron)
		return e
	}
	if p.UpdateIntervalHours != nil && *p.UpdateIntervalHours == 0 {
		return fmt.Errorf("update interval must be positive")
	}
	return nil
}
func NextUpdate(p Profile, now time.Time) (uint64, error) {
	if p.UpdateCron != nil && strings.TrimSpace(*p.UpdateCron) != "" {
		s, e := parseSchedule(*p.UpdateCron)
		if e != nil {
			return 0, e
		}
		next := s.Next(now)
		if next.IsZero() {
			return 0, nil
		}
		return uint64(next.Unix()), nil
	}
	if p.UpdateIntervalHours != nil {
		return uint64(now.Add(time.Duration(*p.UpdateIntervalHours) * time.Hour).Unix()), nil
	}
	return 0, nil
}
