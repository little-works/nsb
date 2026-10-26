package core

import (
	"crypto/sha256"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
)

type Store struct {
	ConfigDir, DataDir string
	mu                 sync.Mutex
	writable           bool
}
type profileRuntime struct {
	UpdatedAt       uint64  `json:"updated_at"`
	NextUpdateAt    uint64  `json:"next_update_at"`
	LastAttemptAt   uint64  `json:"last_attempt_at"`
	LastUpdateError *string `json:"last_update_error"`
	Revision        uint64  `json:"revision"`
}
type runtimeState struct {
	CurrentProfileID *string                   `json:"current_profile_id"`
	Profiles         map[string]profileRuntime `json:"profiles"`
}

func (p *profileRuntime) UnmarshalJSON(b []byte) error {
	b, e := legacyFields(b, "updated_at", "next_update_at", "last_attempt_at")
	if e != nil {
		return e
	}
	type plain profileRuntime
	var v plain
	if e = json.Unmarshal(b, &v); e != nil {
		return e
	}
	*p = profileRuntime(v)
	return nil
}

func (s *Store) Load() (Config, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.writable = false
	c := DefaultConfig()
	b, err := os.ReadFile(filepath.Join(s.ConfigDir, "app-config.json"))
	if errors.Is(err, os.ErrNotExist) {
		s.writable = true
		return c, nil
	}
	if err != nil {
		return c, err
	}
	var root Object
	if err = json.Unmarshal(b, &root); err != nil || root == nil {
		return c, fmt.Errorf("configuration must be a JSON object; original file was not modified")
	}
	if err = json.Unmarshal(b, &c); err != nil {
		return c, fmt.Errorf("configuration was not modified: %w", err)
	}
	rt := runtimeState{}
	b, err = os.ReadFile(filepath.Join(s.DataDir, "app-runtime.json"))
	if err == nil {
		if err = json.Unmarshal(b, &root); err != nil || root == nil {
			return c, fmt.Errorf("runtime state must be a JSON object")
		}
		if err = json.Unmarshal(b, &rt); err != nil {
			return c, err
		}
	} else if !errors.Is(err, os.ErrNotExist) {
		return c, err
	}
	c.CurrentProfileID = rt.CurrentProfileID
	validCurrent := false
	for i := range c.Profiles {
		p := &c.Profiles[i]
		if p.ID == "" {
			p.ID = NewID()
		}
		if v, ok := rt.Profiles[p.ID]; ok {
			p.UpdatedAt = v.UpdatedAt
			p.NextUpdateAt = v.NextUpdateAt
			p.LastAttemptAt = v.LastAttemptAt
			p.LastUpdateError = v.LastUpdateError
			p.Revision = v.Revision
		}
		if c.CurrentProfileID != nil && p.ID == *c.CurrentProfileID {
			validCurrent = true
		}
	}
	if !validCurrent {
		c.CurrentProfileID = nil
	}
	names := map[string]bool{}
	unique := func(name, fallback string) string {
		base := strings.TrimSpace(name)
		if base == "" {
			base = fallback
		}
		v := base
		for n := 2; names[v]; n++ {
			v = fmt.Sprintf("%s %d", base, n)
		}
		names[v] = true
		return v
	}
	for i := range c.Templates {
		c.Templates[i].Name = unique(c.Templates[i].Name, "Template")
	}
	names = map[string]bool{}
	for i := range c.Profiles {
		c.Profiles[i].Name = unique(c.Profiles[i].Name, "Profile")
	}
	s.writable = true
	return c, nil
}
func (s *Store) Save(c Config) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if !s.writable {
		return fmt.Errorf("configuration cannot be saved until successfully loaded")
	}
	if err := c.Validate(); err != nil {
		return err
	}
	portable := objJSON(c)
	delete(portable, "current_profile_id")
	profiles := arr(portable["profiles"])
	for _, v := range profiles {
		for _, k := range []string{"updated_at", "next_update_at", "last_attempt_at", "last_update_error", "revision"} {
			delete(obj(v), k)
		}
	}
	rt := runtimeState{CurrentProfileID: c.CurrentProfileID, Profiles: map[string]profileRuntime{}}
	for _, p := range c.Profiles {
		rt.Profiles[p.ID] = profileRuntime{p.UpdatedAt, p.NextUpdateAt, p.LastAttemptAt, p.LastUpdateError, p.Revision}
	}
	for _, entry := range []struct {
		path  string
		value any
	}{{filepath.Join(s.ConfigDir, "app-config.json"), portable}, {filepath.Join(s.DataDir, "app-runtime.json"), rt}} {
		if err := backup(entry.path); err != nil {
			return err
		}
		if err := AtomicWrite(entry.path, []byte(Pretty(entry.value))); err != nil {
			return err
		}
	}
	return nil
}
func objJSON(v any) Object {
	b, _ := json.Marshal(v)
	var o Object
	_ = json.Unmarshal(b, &o)
	return o
}
func backup(path string) error {
	b, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) {
		return nil
	}
	if err != nil {
		return err
	}
	f, err := os.OpenFile(path+".pre-go.bak", os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0600)
	if errors.Is(err, os.ErrExist) {
		return nil
	}
	if err != nil {
		return err
	}
	_, err = f.Write(b)
	closeErr := f.Close()
	if err != nil {
		return err
	}
	return closeErr
}
func AtomicWrite(path string, b []byte) error {
	if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		return err
	}
	f, err := os.CreateTemp(filepath.Dir(path), ".nsb-*")
	if err != nil {
		return err
	}
	tmp := f.Name()
	defer os.Remove(tmp)
	if _, err = f.Write(b); err != nil {
		f.Close()
		return err
	}
	if err = f.Sync(); err != nil {
		f.Close()
		return err
	}
	if err = f.Close(); err != nil {
		return err
	}
	return replaceFile(tmp, path)
}
func safeID(id string) bool {
	return id != "" && id != "." && id != ".." && !strings.ContainsAny(id, "/\\:")
}
func (s *Store) ProfilePath(id string) (string, error) {
	if !safeID(id) {
		return "", fmt.Errorf("invalid profile id")
	}
	return filepath.Join(s.DataDir, "runtime", "profiles", id+".json"), nil
}
func (s *Store) ReadProfile(id string) (string, error) {
	p, e := s.ProfilePath(id)
	if e != nil {
		return "", e
	}
	b, e := os.ReadFile(p)
	if errors.Is(e, os.ErrNotExist) {
		b, e = os.ReadFile(strings.TrimSuffix(p, ".json"))
	}
	return string(b), e
}
func (s *Store) WriteProfile(id, content string) error {
	p, e := s.ProfilePath(id)
	if e != nil {
		return e
	}
	if e = AtomicWrite(p, []byte(content)); e != nil {
		return e
	}
	if e = os.Remove(p + ".invalid"); e != nil && !errors.Is(e, os.ErrNotExist) {
		return e
	}
	return nil
}
func (s *Store) InvalidateProfile(id string) error {
	p, e := s.ProfilePath(id)
	if e != nil {
		return e
	}
	return AtomicWrite(p+".invalid", []byte("definition changed"))
}
func (s *Store) ProfileInvalid(id string) bool {
	p, e := s.ProfilePath(id)
	if e != nil {
		return true
	}
	_, e = os.Stat(p + ".invalid")
	return !errors.Is(e, os.ErrNotExist)
}
func (s *Store) remotePath(url string) string {
	return filepath.Join(s.DataDir, "runtime", "remotes", fmt.Sprintf("%x.raw.txt", sha256.Sum256([]byte(url))))
}
func (s *Store) ReadRemote(url string) (string, error) {
	b, e := os.ReadFile(s.remotePath(url))
	return string(b), e
}
func (s *Store) WriteRemote(url, body string) error {
	return AtomicWrite(s.remotePath(url), []byte(body))
}
