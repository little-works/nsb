package core

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func exampleConfig() Config {
	c := DefaultConfig()
	c.Templates = append(c.Templates, Template{ID: "template-1", Name: "Default", Content: DefaultTemplate})
	c.Profiles = append(c.Profiles, Profile{ID: "profile-1", Name: "Main", TemplateID: "template-1", Remotes: []Remote{}, UpdatedAt: 12, NextUpdateAt: 34, LastAttemptAt: 56, Revision: 7})
	id := "profile-1"
	c.CurrentProfileID = &id
	return c
}
func testStore(t *testing.T) *Store {
	t.Helper()
	dir := t.TempDir()
	return &Store{ConfigDir: filepath.Join(dir, "config"), DataDir: filepath.Join(dir, "data")}
}
func TestStoreCompatibility(t *testing.T) {
	s := testStore(t)
	if _, e := s.Load(); e != nil {
		t.Fatal(e)
	}
	c := exampleConfig()
	if e := s.Save(c); e != nil {
		t.Fatal(e)
	}
	b, e := os.ReadFile(filepath.Join(s.ConfigDir, "app-config.json"))
	if e != nil {
		t.Fatal(e)
	}
	for _, key := range []string{"current_profile_id", "next_update_at", "last_attempt_at", "last_update_error", "revision"} {
		if bytes.Contains(b, []byte(key)) {
			t.Fatalf("portable config leaks %s", key)
		}
	}
	loaded, e := s.Load()
	if e != nil {
		t.Fatal(e)
	}
	if Pretty(loaded) != Pretty(c) {
		t.Fatalf("round trip: %s", Pretty(loaded))
	}
	c.AppLanguage = "en-US"
	if e = s.Save(c); e != nil {
		t.Fatal(e)
	}
	backup, e := os.ReadFile(filepath.Join(s.ConfigDir, "app-config.json.pre-go.bak"))
	if e != nil || !bytes.Equal(backup, b) {
		t.Fatalf("backup mismatch: %v", e)
	}
	if e = s.Save(c); e != nil {
		t.Fatal(e)
	}
	again, _ := os.ReadFile(filepath.Join(s.ConfigDir, "app-config.json.pre-go.bak"))
	if !bytes.Equal(again, backup) {
		t.Fatal("backup overwritten")
	}
}
func TestCorruptConfigNotOverwritten(t *testing.T) {
	s := testStore(t)
	path := filepath.Join(s.ConfigDir, "app-config.json")
	if e := AtomicWrite(path, []byte("{broken")); e != nil {
		t.Fatal(e)
	}
	if _, e := s.Load(); e == nil {
		t.Fatal("corrupt config accepted")
	}
	if e := s.Save(DefaultConfig()); e == nil {
		t.Fatal("save after failed load allowed")
	}
	b, _ := os.ReadFile(path)
	if string(b) != "{broken" {
		t.Fatal("corrupt file was overwritten")
	}
}
func TestNullConfigIsNotOverwritten(t *testing.T) {
	s := testStore(t)
	path := filepath.Join(s.ConfigDir, "app-config.json")
	if e := AtomicWrite(path, []byte("null")); e != nil {
		t.Fatal(e)
	}
	if _, e := s.Load(); e == nil {
		t.Fatal("null config accepted")
	}
	if e := s.Save(DefaultConfig()); e == nil {
		t.Fatal("save after invalid config allowed")
	}
	b, _ := os.ReadFile(path)
	if string(b) != "null" {
		t.Fatal("invalid config overwritten")
	}
}
func TestCacheAndUnsafeIDs(t *testing.T) {
	s := testStore(t)
	for _, id := range []string{"../escape", "..", `a\b`, "C:foo", ""} {
		if e := s.WriteProfile(id, "{}"); e == nil {
			t.Fatalf("unsafe id accepted %q", id)
		}
	}
	if e := s.WriteRemote("https://example.test", "hello"); e != nil {
		t.Fatal(e)
	}
	b, e := s.ReadRemote("https://example.test")
	if e != nil || b != "hello" {
		t.Fatal(e)
	}
	if e = s.WriteProfile("test", "{}"); e != nil {
		t.Fatal(e)
	}
	b, e = s.ReadProfile("test")
	if e != nil || b != "{}" {
		t.Fatal(e)
	}
}
func TestArchive(t *testing.T) {
	c := exampleConfig()
	archive := Export(c)
	b := []byte(Pretty(archive))
	for _, k := range []string{"revision", "current_profile_id", "updated_at", "template_id"} {
		if bytes.Contains(b, []byte(k)) {
			t.Fatalf("archive leaks %s", k)
		}
	}
	got, r, e := Import(b, DefaultConfig())
	if e != nil || r.Added != 2 || len(got.Profiles) != 1 {
		t.Fatalf("%+v %v", r, e)
	}
	id := got.Profiles[0].ID
	again, r, e := Import(b, got)
	if e != nil || r.Overwritten != 2 || again.Profiles[0].ID != id {
		t.Fatalf("%+v %v", r, e)
	}
	archive.Profiles = append(archive.Profiles, PortableProfile{Name: "Broken"})
	_, r, e = Import([]byte(Pretty(archive)), got)
	if e != nil || r.Skipped != 1 {
		t.Fatalf("%+v %v", r, e)
	}
	archive.Version = 2
	if _, _, e = Import([]byte(Pretty(archive)), c); e == nil {
		t.Fatal("unknown version accepted")
	}
}
func TestRemoteDefaults(t *testing.T) {
	var r Remote
	if e := json.Unmarshal([]byte(`{"name":"r","url":"https://example.test","keep":{"dns":{"servers":true}}}`), &r); e != nil {
		t.Fatal(e)
	}
	if r.Format != "clash" || !enabled(r.Keep, "outbounds") || !enabled(r.Keep, "clash", "proxies") {
		t.Fatalf("defaults lost: %s", Pretty(r))
	}
}
func TestSchedules(t *testing.T) {
	hours := uint32(2)
	p := Profile{UpdateIntervalHours: &hours}
	now := time.Date(2026, 10, 26, 12, 0, 0, 0, time.UTC)
	next, e := NextUpdate(p, now)
	if e != nil || next != uint64(now.Add(2*time.Hour).Unix()) {
		t.Fatal(next, e)
	}
	cron := "0 30 * * * * *"
	p.UpdateCron = &cron
	next, e = NextUpdate(p, now)
	if e != nil || next != uint64(now.Add(30*time.Minute).Unix()) {
		t.Fatal(next, e)
	}
	bad := "bad"
	p.UpdateCron = &bad
	if ValidateSchedule(p) == nil {
		t.Fatal("invalid cron accepted")
	}
}
func TestLegacyTimestampsAndIDs(t *testing.T) {
	var p Profile
	if e := json.Unmarshal([]byte(`{"id":" old-id ","name":"old","updated_at":" 42 ","next_update_at":null,"last_attempt_at":"bad"}`), &p); e != nil {
		t.Fatal(e)
	}
	if p.ID != "old-id" || p.UpdatedAt != 42 || p.NextUpdateAt != 0 || p.LastAttemptAt != 0 {
		t.Fatalf("%+v", p)
	}
	var r profileRuntime
	if e := json.Unmarshal([]byte(`{"updated_at":"42","next_update_at":null}`), &r); e != nil || r.UpdatedAt != 42 {
		t.Fatal(r, e)
	}
}
func TestCronYearAndWeekdayCompatibility(t *testing.T) {
	now := time.Date(2026, 10, 26, 12, 0, 0, 0, time.UTC)
	for _, tc := range []struct {
		expression string
		want       time.Time
	}{{"0 0 0 1 1 * 2028", time.Date(2028, 1, 1, 0, 0, 0, 0, time.UTC)}, {"0 0 0 * * 1", time.Date(2026, 11, 1, 0, 0, 0, 0, time.UTC)}, {"0 0 0 1 * 2", time.Date(2027, 2, 1, 0, 0, 0, 0, time.UTC)}, {"0 0 0 29 2 2", time.Date(2044, 2, 29, 0, 0, 0, 0, time.UTC)}} {
		s, e := parseSchedule(tc.expression)
		if e != nil {
			t.Fatal(e)
		}
		if got := s.Next(now); !got.Equal(tc.want) {
			t.Errorf("%s: %s, want %s", tc.expression, got, tc.want)
		}
	}
}
