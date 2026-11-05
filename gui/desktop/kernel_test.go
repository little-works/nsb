package desktop

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"nsb/core"
)

func TestClearLogsPreservesFileAndShowsNewRecords(t *testing.T) {
	dir := t.TempDir()
	m, err := Open(filepath.Join(dir, "config"), filepath.Join(dir, "data"))
	if err != nil {
		t.Fatal(err)
	}
	if err := m.ClearLogs(); err != nil {
		t.Fatal(err)
	}
	path := m.kernel.LogPath()
	if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte("INFO old record\n"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := m.ClearLogs(); err != nil {
		t.Fatal(err)
	}
	if got := m.kernel.LogTail(); got != "" {
		t.Fatalf("old records remain: %q", got)
	}
	f, err := os.OpenFile(path, os.O_APPEND|os.O_WRONLY, 0600)
	if err != nil {
		t.Fatal(err)
	}
	_, err = f.WriteString("INFO new record\n")
	f.Close()
	if err != nil {
		t.Fatal(err)
	}
	if got := m.kernel.LogTail(); got != "INFO new record\n" {
		t.Fatalf("new record missing: %q", got)
	}
	body, err := os.ReadFile(path)
	if err != nil || string(body) != "INFO old record\nINFO new record\n" {
		t.Fatalf("disk log changed: %q, %v", body, err)
	}
	if err := os.WriteFile(path, []byte("reset\n"), 0600); err != nil {
		t.Fatal(err)
	}
	if got := m.kernel.LogTail(); got != "reset\n" {
		t.Fatalf("truncated log missing: %q", got)
	}
}

// The test executable doubles as a local sing-box process without network access.
func TestMain(m *testing.M) {
	if os.Getenv("NSB_TEST_KERNEL") == "1" && len(os.Args) > 1 {
		switch os.Args[1] {
		case "version":
			fmt.Println("sing-box version test")
			os.Exit(0)
		case "check":
			os.Exit(0)
		case "run":
			b, e := os.ReadFile(os.Args[3])
			if e != nil {
				os.Exit(2)
			}
			var config map[string]any
			if json.Unmarshal(b, &config) != nil {
				os.Exit(3)
			}
			api := config["experimental"].(map[string]any)["clash_api"].(map[string]any)
			address := api["external_controller"].(string)
			secret := api["secret"].(string)
			fmt.Println("INFO test kernel started")
			h := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				if r.Header.Get("Authorization") != "Bearer "+secret {
					w.WriteHeader(401)
					return
				}
				w.Header().Set("Content-Type", "application/json")
				switch r.URL.Path {
				case "/version":
					fmt.Fprint(w, `{"version":"test"}`)
				case "/proxies":
					fmt.Fprint(w, `{"proxies":{"PROXY":{"type":"Selector","now":"node","all":["node"]}}}`)
				case "/connections":
					fmt.Fprint(w, `{"connections":[]}`)
				case "/configs":
					fmt.Fprint(w, `{"mode":"rule"}`)
				default:
					if strings.HasSuffix(r.URL.Path, "/delay") {
						fmt.Fprint(w, `{"delay":12}`)
					} else {
						w.WriteHeader(204)
					}
				}
			})
			if http.ListenAndServe(address, h) != nil {
				os.Exit(4)
			}
			return
		}
	}
	os.Exit(m.Run())
}
func helperKernel(t *testing.T) *Kernel {
	t.Helper()
	t.Setenv("NSB_TEST_KERNEL", "1")
	k := NewKernel(t.TempDir())
	binary, e := os.Executable()
	if e != nil {
		t.Fatal(e)
	}
	b, e := os.ReadFile(binary)
	if e != nil {
		t.Fatal(e)
	}
	if e = core.AtomicWrite(k.Binary(), b); e != nil {
		t.Fatal(e)
	}
	if e = makeExecutable(k.Binary()); e != nil {
		t.Fatal(e)
	}
	t.Cleanup(func() { _ = k.Shutdown() })
	return k
}
func TestManagerStartsKernelWithRelativeDirectories(t *testing.T) {
	k := helperKernel(t)
	cwd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	relative, err := filepath.Rel(cwd, k.dir)
	if err != nil {
		t.Fatal(err)
	}
	m, err := Open(filepath.Join(relative, "config"), relative)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = m.Shutdown() })
	if !filepath.IsAbs(m.store.ConfigDir) || !filepath.IsAbs(m.store.DataDir) || !filepath.IsAbs(m.kernel.Binary()) {
		t.Fatal("manager paths must be absolute")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()
	if err := m.kernel.Start(ctx, core.DefaultTemplate, "", false); err != nil {
		t.Fatal(err)
	}
	if !m.kernel.Running() || !strings.Contains(m.kernel.LogTail(), "test kernel started") {
		t.Fatal("relative directories must support kernel startup and log capture")
	}
}

func TestKernelLifecycleAndAdoption(t *testing.T) {
	k := helperKernel(t)
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()
	if e := k.Start(ctx, core.DefaultTemplate, "", false); e != nil {
		t.Fatal(e)
	}
	if !k.Running() {
		t.Fatal("kernel not running")
	}
	if !strings.Contains(k.LogTail(), "test kernel started") {
		t.Fatal("logs not captured")
	}
	adopted := NewKernel(k.dir)
	if e := adopted.Adopt(); e != nil || !adopted.Running() || adopted.owned {
		t.Fatal("adoption failed", e)
	}
	if e := adopted.Shutdown(); e != nil || !k.Running() {
		t.Fatal("shutdown killed adopted process", e)
	}
	var p core.Object
	if e := k.Request(ctx, "GET", "/proxies", nil, &p); e != nil || p["proxies"] == nil {
		t.Fatal(e)
	}
	if e := k.Stop(); e != nil {
		t.Fatal(e)
	}
	if k.Running() {
		t.Fatal("kernel still running")
	}
	if _, e := os.Stat(k.workspace("pid.txt")); !os.IsNotExist(e) {
		t.Fatal("marker not cleared")
	}
}
func TestProcessIdentityMismatch(t *testing.T) {
	k := helperKernel(t)
	k.mark = marker{PID: os.Getpid(), CreatedAt: 1}
	k.running = true
	if k.Running() {
		t.Fatal("PID reuse accepted")
	}
	if e := core.AtomicWrite(k.workspace("pid.txt"), []byte(core.Pretty(k.mark))); e != nil {
		t.Fatal(e)
	}
	k = NewKernel(k.dir)
	if e := k.Adopt(); e != nil || k.Running() {
		t.Fatal("stale marker adopted", e)
	}
}
func TestManagerGenerateActivateAndSchedule(t *testing.T) {
	k := helperKernel(t)
	m, e := Open(filepath.Join(k.dir, "config"), k.dir)
	if e != nil {
		t.Fatal(e)
	}
	m.kernel = k
	c := m.Config()
	inline := core.DefaultTemplate
	hours := uint32(1)
	c.Profiles = []core.Profile{{ID: "p", Name: "Main", InlineTemplate: &inline, Remotes: []core.Remote{}, UpdateIntervalHours: &hours}}
	if e = m.Save(c); e != nil {
		t.Fatal(e)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()
	if _, e = m.Generate(ctx, "p", false); e != nil {
		t.Fatal(e)
	}
	if e = m.Activate(ctx, "p"); e != nil {
		t.Fatal(e)
	}
	if e = m.Start(ctx); e != nil {
		t.Fatal(e)
	}
	s := m.Poll(ctx)
	if !s.Running || s.Proxies == nil || s.Connections == nil {
		t.Fatalf("bad snapshot %+v", s)
	}
	if e = m.Mode(ctx, "global"); e != nil {
		t.Fatal(e)
	}
	if e = m.SelectProxy(ctx, "PROXY", "node"); e != nil {
		t.Fatal(e)
	}
	if n, e := m.Latency(ctx, "node"); e != nil || n != 12 {
		t.Fatal(n, e)
	}
	if e = m.CloseConnection(ctx, ""); e != nil {
		t.Fatal(e)
	}
	if e = m.Restart(ctx); e != nil {
		t.Fatal(e)
	}
	if e = m.ImportKernel(ctx, k.Binary()); e != nil {
		t.Fatal(e)
	}
	if e = m.Shutdown(); e != nil {
		t.Fatal(e)
	}
	c = m.Config()
	c.Profiles[0].NextUpdateAt = 1
	if e = m.Save(c); e != nil {
		t.Fatal(e)
	}
	if e = m.Tick(ctx); e != nil {
		t.Fatal(e)
	}
	if m.Config().Profiles[0].NextUpdateAt <= uint64(time.Now().Unix()) {
		t.Fatal("schedule was not advanced")
	}
}
func TestEditingInvalidatesGeneratedCache(t *testing.T) {
	dir := t.TempDir()
	m, e := Open(filepath.Join(dir, "config"), filepath.Join(dir, "data"))
	if e != nil {
		t.Fatal(e)
	}
	cfg := m.Config()
	inline := `{"outbounds":[],"log":{"level":"info"}}`
	cfg.Profiles = []core.Profile{{ID: "p", Name: "Main", InlineTemplate: &inline, Remotes: []core.Remote{}}}
	if e = m.Save(cfg); e != nil {
		t.Fatal(e)
	}
	if _, e = m.Generate(context.Background(), "p", false); e != nil {
		t.Fatal(e)
	}
	if e = m.Edit(func(c *core.Config) error {
		value := `{"outbounds":[],"log":{"level":"debug"}}`
		c.Profiles[0].InlineTemplate = &value
		return nil
	}); e != nil {
		t.Fatal(e)
	}
	if !m.store.ProfileInvalid("p") {
		t.Fatal("changed definition did not invalidate cache")
	}
	body, e := m.GeneratedConfig(context.Background(), "p")
	if e != nil || !strings.Contains(body, "debug") || m.store.ProfileInvalid("p") {
		t.Fatal(body, e)
	}
}
func TestKernelImportRejectsInvalidBinary(t *testing.T) {
	k := helperKernel(t)
	m, e := Open(filepath.Join(k.dir, "config"), k.dir)
	if e != nil {
		t.Fatal(e)
	}
	m.kernel = k
	before, e := os.ReadFile(k.Binary())
	if e != nil {
		t.Fatal(e)
	}
	path := filepath.Join(t.TempDir(), "invalid.exe")
	if e = os.WriteFile(path, []byte("invalid kernel"), 0600); e != nil {
		t.Fatal(e)
	}
	if e = m.ImportKernel(context.Background(), path); e == nil {
		t.Fatal("invalid kernel was imported")
	}
	after, e := os.ReadFile(k.Binary())
	if e != nil || string(after) != string(before) {
		t.Fatal("working kernel was overwritten", e)
	}
}
