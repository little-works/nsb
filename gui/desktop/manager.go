package desktop

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"nsb/core"
)

func makeExecutable(path string) error { return os.Chmod(path, 0700) }

type Snapshot struct {
	Config                            core.Config
	Running                           bool
	Logs                              string
	Proxies, Connections, ClashConfig core.Object
	Version                           string
	Error                             string
	UploadRate, DownloadRate          float64
	Uptime                            time.Duration
}
type Manager struct {
	mu                       sync.Mutex
	store                    *core.Store
	service                  core.Service
	config                   core.Config
	kernel                   *Kernel
	version                  string
	Progress                 DownloadProgress
	lastPoll                 time.Time
	lastUpload, lastDownload float64
}

func Open(configDir, dataDir string) (*Manager, error) {
	configDir, err := filepath.Abs(configDir)
	if err != nil {
		return nil, fmt.Errorf("configuration directory: %w", err)
	}
	dataDir, err = filepath.Abs(dataDir)
	if err != nil {
		return nil, fmt.Errorf("data directory: %w", err)
	}
	s := &core.Store{ConfigDir: configDir, DataDir: dataDir}
	c, e := s.Load()
	if e != nil {
		return nil, e
	}
	m := &Manager{store: s, service: core.Service{Store: s}, config: c, kernel: NewKernel(dataDir)}
	if e = m.kernel.Adopt(); e != nil {
		return nil, e
	}
	return m, nil
}
func (m *Manager) Config() core.Config { m.mu.Lock(); defer m.mu.Unlock(); return core.Clone(m.config) }
func (m *Manager) Save(c core.Config) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.save(c)
}

// Edit keeps configuration changes atomic with scheduled updates.
func (m *Manager) Edit(change func(*core.Config) error) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	c := core.Clone(m.config)
	if e := change(&c); e != nil {
		return e
	}
	return m.save(c)
}
func definition(c core.Config, p core.Profile) string {
	p.UpdatedAt = 0
	p.NextUpdateAt = 0
	p.LastAttemptAt = 0
	p.LastUpdateError = nil
	p.Revision = 0
	p.UpdateCron = nil
	p.UpdateIntervalHours = nil
	template, _ := c.TemplateFor(p)
	return core.Pretty(p) + template
}
func (m *Manager) save(c core.Config) error {
	if e := c.Validate(); e != nil {
		return e
	}
	for i := range c.Profiles {
		p := &c.Profiles[i]
		for _, old := range m.config.Profiles {
			if old.ID != p.ID {
				continue
			}
			if definition(c, *p) != definition(m.config, old) {
				if e := m.store.InvalidateProfile(p.ID); e != nil {
					return e
				}
				p.Revision = old.Revision + 1
				p.LastUpdateError = nil
			}
			if p.Revision < old.Revision {
				p.Revision = old.Revision
				p.UpdatedAt = old.UpdatedAt
				p.LastAttemptAt = old.LastAttemptAt
				p.LastUpdateError = old.LastUpdateError
			}
			if core.Pretty(p.UpdateCron) != core.Pretty(old.UpdateCron) || core.Pretty(p.UpdateIntervalHours) != core.Pretty(old.UpdateIntervalHours) {
				p.NextUpdateAt, _ = core.NextUpdate(*p, time.Now())
			}
			break
		}
	}
	old := m.config.SystemProxyEnabled
	if c.SystemProxyEnabled != old {
		if e := m.kernel.SetProxy(c.SystemProxyEnabled); e != nil {
			_ = m.kernel.SetProxy(old)
			return e
		}
	}
	if e := m.store.Save(c); e != nil {
		_ = m.kernel.SetProxy(old)
		return e
	}
	m.config = core.Clone(c)
	return nil
}
func (m *Manager) Poll(ctx context.Context) Snapshot {
	m.mu.Lock()
	defer m.mu.Unlock()
	s := Snapshot{Config: core.Clone(m.config), Running: m.kernel.Running(), Logs: m.kernel.LogTail(), Version: m.version}
	if s.Running {
		if !m.kernel.startedAt.IsZero() {
			s.Uptime = time.Since(m.kernel.startedAt).Truncate(time.Second)
		}
		var errs []error
		errs = append(errs, m.kernel.Request(ctx, "GET", "/proxies", nil, &s.Proxies), m.kernel.Request(ctx, "GET", "/connections", nil, &s.Connections), m.kernel.Request(ctx, "GET", "/configs", nil, &s.ClashConfig))
		for _, e := range errs {
			if e != nil {
				s.Error = e.Error()
				break
			}
		}
		upload, _ := s.Connections["uploadTotal"].(float64)
		download, _ := s.Connections["downloadTotal"].(float64)
		now := time.Now()
		if !m.lastPoll.IsZero() {
			seconds := now.Sub(m.lastPoll).Seconds()
			if seconds > 0 && upload >= m.lastUpload && download >= m.lastDownload {
				s.UploadRate = (upload - m.lastUpload) / seconds
				s.DownloadRate = (download - m.lastDownload) / seconds
			}
		}
		m.lastPoll = now
		m.lastUpload = upload
		m.lastDownload = download
	} else {
		m.lastPoll = time.Time{}
	}
	return s
}
func (m *Manager) current() (core.Profile, error) {
	for _, p := range m.config.Profiles {
		if m.config.CurrentProfileID != nil && p.ID == *m.config.CurrentProfileID {
			return p, nil
		}
	}
	return core.Profile{}, fmt.Errorf("select a profile first")
}
func (m *Manager) start(ctx context.Context) error {
	p, e := m.current()
	if e != nil {
		return e
	}
	body, e := m.store.ReadProfile(p.ID)
	if e != nil || m.store.ProfileInvalid(p.ID) {
		body, _, e = m.service.Generate(ctx, m.config, p, false)
		if e != nil {
			return e
		}
		if e = m.store.WriteProfile(p.ID, body); e != nil {
			return e
		}
	}
	hook := ""
	if p.Hook != nil {
		hook = *p.Hook
	}
	return m.kernel.Start(ctx, body, hook, m.config.SystemProxyEnabled)
}
func (m *Manager) Start(ctx context.Context) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.kernel.Running() {
		return nil
	}
	return m.start(ctx)
}
func (m *Manager) Stop() error { m.mu.Lock(); defer m.mu.Unlock(); return m.kernel.Stop() }
func (m *Manager) Restart(ctx context.Context) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	if e := m.kernel.Stop(); e != nil {
		return e
	}
	return m.start(ctx)
}
func (m *Manager) Shutdown() error { m.mu.Lock(); defer m.mu.Unlock(); return m.kernel.Shutdown() }
func (m *Manager) Activate(ctx context.Context, id string) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	found := false
	for _, p := range m.config.Profiles {
		if p.ID == id {
			found = true
		}
	}
	if !found {
		return fmt.Errorf("profile not found")
	}
	running := m.kernel.Running()
	if running {
		if e := m.kernel.Stop(); e != nil {
			return e
		}
	}
	old := m.config.CurrentProfileID
	m.config.CurrentProfileID = &id
	if e := m.store.Save(m.config); e != nil {
		m.config.CurrentProfileID = old
		return e
	}
	if running {
		return m.start(ctx)
	}
	return nil
}
func (m *Manager) Generate(ctx context.Context, id string, refresh bool) (string, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.generate(ctx, id, refresh)
}
func (m *Manager) generate(ctx context.Context, id string, refresh bool) (string, error) {
	i := -1
	for j, p := range m.config.Profiles {
		if p.ID == id {
			i = j
			break
		}
	}
	if i < 0 {
		return "", fmt.Errorf("profile not found")
	}
	p := m.config.Profiles[i]
	body, warnings, e := m.service.Generate(ctx, m.config, p, refresh)
	p.LastAttemptAt = uint64(time.Now().Unix())
	p.NextUpdateAt, _ = core.NextUpdate(p, time.Now())
	if e != nil {
		msg := e.Error()
		p.LastUpdateError = &msg
	} else {
		if e = m.store.WriteProfile(id, body); e != nil {
			return "", e
		}
		p.UpdatedAt = p.LastAttemptAt
		p.LastUpdateError = nil
		p.Revision++
		if len(warnings) > 0 {
			p.LastUpdateError = nil
		}
	}
	m.config.Profiles[i] = p
	if saveErr := m.store.Save(m.config); saveErr != nil {
		return "", saveErr
	}
	if e != nil {
		return "", e
	}
	if m.kernel.Running() && m.config.CurrentProfileID != nil && *m.config.CurrentProfileID == id {
		if e = m.kernel.Stop(); e != nil {
			return "", e
		}
		if e = m.start(ctx); e != nil {
			return "", e
		}
	}
	return body, nil
}
func (m *Manager) Tick(ctx context.Context) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	now := uint64(time.Now().Unix())
	changed := false
	for i := range m.config.Profiles {
		p := &m.config.Profiles[i]
		if p.NextUpdateAt == 0 {
			p.NextUpdateAt, _ = core.NextUpdate(*p, time.Now())
			if p.NextUpdateAt > 0 {
				changed = true
			}
		}
		if p.NextUpdateAt > 0 && p.NextUpdateAt <= now {
			if _, e := m.generate(ctx, p.ID, true); e != nil {
				return e
			}
		}
	}
	if changed {
		return m.store.Save(m.config)
	}
	return nil
}
func (m *Manager) SaveRuntime(ctx context.Context, id, body string) error {
	var o core.Object
	if e := json.Unmarshal([]byte(body), &o); e != nil || o == nil {
		return fmt.Errorf("runtime must be a JSON object")
	}
	m.mu.Lock()
	defer m.mu.Unlock()
	if e := m.store.WriteProfile(id, body); e != nil {
		return e
	}
	if m.config.CurrentProfileID != nil && *m.config.CurrentProfileID == id && m.kernel.Running() {
		if e := m.kernel.Stop(); e != nil {
			return e
		}
		return m.start(ctx)
	}
	return nil
}
func (m *Manager) ReadRuntime(id string) (string, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.store.ReadProfile(id)
}
func (m *Manager) GeneratedConfig(ctx context.Context, id string) (string, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	body, e := m.store.ReadProfile(id)
	if e != nil || m.store.ProfileInvalid(id) {
		return m.generate(ctx, id, false)
	}
	return body, nil
}
func (m *Manager) SelectProxy(ctx context.Context, group, node string) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.kernel.Request(ctx, "PUT", "/proxies/"+escape(group), core.Object{"name": node}, nil)
}
func (m *Manager) Mode(ctx context.Context, mode string) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.kernel.Request(ctx, "PATCH", "/configs", core.Object{"mode": mode}, nil)
}
func (m *Manager) CloseConnection(ctx context.Context, id string) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	path := "/connections"
	if id != "" {
		path += "/" + escape(id)
	}
	return m.kernel.Request(ctx, "DELETE", path, nil, nil)
}
func (m *Manager) Latency(ctx context.Context, node string) (int, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	var out struct {
		Delay int `json:"delay"`
	}
	e := m.kernel.Request(ctx, "GET", "/proxies/"+escape(node)+"/delay?timeout=5000&url=https%3A%2F%2Fcp.cloudflare.com%2Fgenerate_204", nil, &out)
	return out.Delay, e
}
func (m *Manager) replaceKernel(ctx context.Context, install func() error) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	running := m.kernel.Running()
	if running {
		if e := m.kernel.Stop(); e != nil {
			return e
		}
	}
	err := install()
	m.version = m.kernel.Version(ctx)
	if running {
		err = errors.Join(err, m.start(ctx))
	}
	return err
}
func (m *Manager) Download(ctx context.Context) error {
	return m.replaceKernel(ctx, func() error { return m.kernel.Download(ctx, &m.Progress) })
}
func (m *Manager) ImportKernel(ctx context.Context, path string) error {
	f, err := os.Open(path)
	if err != nil {
		return err
	}
	binary, err := io.ReadAll(io.LimitReader(f, (256<<20)+1))
	closeErr := f.Close()
	if err == nil {
		err = closeErr
	}
	if err != nil {
		return err
	}
	if len(binary) > 256<<20 {
		return fmt.Errorf("kernel file exceeds 256 MiB")
	}
	lower := strings.ToLower(path)
	if strings.HasSuffix(lower, ".zip") {
		binary, err = extractBinary(binary, ".zip")
	} else if strings.HasSuffix(lower, ".tar.gz") || strings.HasSuffix(lower, ".tgz") {
		binary, err = extractBinary(binary, ".tar.gz")
	}
	if err != nil {
		return err
	}
	return m.replaceKernel(ctx, func() error { return m.kernel.Install(ctx, binary) })
}
func (m *Manager) ClearLogs() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	info, err := os.Stat(m.kernel.LogPath())
	if errors.Is(err, os.ErrNotExist) {
		m.kernel.logOffset = 0
		return nil
	}
	if err != nil {
		return err
	}
	m.kernel.logOffset = info.Size()
	return nil
}

func (m *Manager) ReadVersion(ctx context.Context) string {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.version = m.kernel.Version(ctx)
	return m.version
}
func DefaultPaths(dev bool) (string, string, error) {
	if dev {
		return filepath.Join("gui", "dev-config"), filepath.Join("gui", "dev-data"), nil
	}
	home, e := os.UserHomeDir()
	if e != nil {
		return "", "", e
	}
	return filepath.Join(home, ".config", "nsb"), filepath.Join(home, ".nsb"), nil
}
