package desktop

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"time"

	"nsb/core"
)

type marker struct {
	PID       int    `json:"pid"`
	CreatedAt uint64 `json:"created_at"`
}
type Kernel struct {
	dir                          string
	cmd                          *exec.Cmd
	done                         chan error
	mark                         marker
	owned                        bool
	controller, secret, endpoint string
	proxy                        systemProxy
	running                      bool
	startedAt                    time.Time
}

var controllerClient = &http.Client{Timeout: 7 * time.Second, Transport: &http.Transport{Proxy: nil}}

func NewKernel(dir string) *Kernel { return &Kernel{dir: dir} }
func (k *Kernel) Binary() string {
	name := "sing-box"
	if runtime.GOOS == "windows" {
		name += ".exe"
	}
	return filepath.Join(k.dir, "sing-box", name)
}
func (k *Kernel) workspace(name string) string {
	return filepath.Join(k.dir, "runtime", "workspace", name)
}
func (k *Kernel) LogPath() string { return filepath.Join(k.dir, "logs", "sing-box.log") }
func (k *Kernel) Running() bool {
	if !k.running {
		return false
	}
	stamp, e := creationTime(k.mark.PID)
	if e != nil || stamp != k.mark.CreatedAt {
		k.running = false
		_ = k.proxy.Restore()
	}
	return k.running
}
func (k *Kernel) Adopt() error {
	b, e := os.ReadFile(k.workspace("pid.txt"))
	if os.IsNotExist(e) {
		return nil
	}
	if e != nil {
		return e
	}
	var m marker
	if json.Unmarshal(b, &m) != nil || m.CreatedAt == 0 {
		return nil
	}
	stamp, e := creationTime(m.PID)
	if e != nil || stamp != m.CreatedAt {
		return nil
	}
	b, e = os.ReadFile(k.workspace("config.json"))
	if e != nil {
		return e
	}
	var o core.Object
	if e = json.Unmarshal(b, &o); e != nil {
		return e
	}
	if e = k.readLaunch(o); e != nil {
		return e
	}
	k.mark = m
	k.running = true
	k.startedAt = processStartedAt(m.PID)
	return nil
}
func (k *Kernel) readLaunch(o core.Object) error {
	experimental, _ := o["experimental"].(map[string]any)
	api, _ := experimental["clash_api"].(map[string]any)
	address, _ := api["external_controller"].(string)
	host, port, e := net.SplitHostPort(address)
	if e != nil {
		return fmt.Errorf("invalid Clash controller address")
	}
	ip := net.ParseIP(host)
	n, e := strconv.Atoi(port)
	if e != nil || n < 1 || n > 65535 || ip == nil || (!ip.IsLoopback() && !ip.IsUnspecified()) {
		return fmt.Errorf("Clash controller must use a local IP address")
	}
	k.controller = "http://" + net.JoinHostPort("127.0.0.1", port)
	k.secret, _ = api["secret"].(string)
	k.endpoint = ""
	inbounds, _ := o["inbounds"].([]any)
	for _, v := range inbounds {
		in, _ := v.(map[string]any)
		if in["type"] == "mixed" {
			host, _ := in["listen"].(string)
			if host == "" || host == "0.0.0.0" || host == "::" {
				host = "127.0.0.1"
			}
			if port, ok := in["listen_port"].(float64); ok && port > 0 && port < 65536 {
				k.endpoint = net.JoinHostPort(host, strconv.Itoa(int(port)))
				break
			}
		}
	}
	return nil
}
func (k *Kernel) Start(ctx context.Context, body, hook string, proxy bool) error {
	if k.Running() {
		return fmt.Errorf("kernel is already running")
	}
	var o core.Object
	if e := json.Unmarshal([]byte(body), &o); e != nil {
		return e
	}
	if o == nil {
		return fmt.Errorf("configuration must be an object")
	}
	experimental, _ := o["experimental"].(map[string]any)
	if experimental == nil {
		experimental = core.Object{}
		o["experimental"] = experimental
	}
	api, _ := experimental["clash_api"].(map[string]any)
	if api == nil {
		api = core.Object{}
		experimental["clash_api"] = api
	}
	if addr, _ := api["external_controller"].(string); addr == "" {
		l, e := net.Listen("tcp", "127.0.0.1:0")
		if e != nil {
			return e
		}
		api["external_controller"] = l.Addr().String()
		_ = l.Close()
	}
	if secret, _ := api["secret"].(string); secret == "" {
		api["secret"] = core.NewID()
	}
	route, _ := o["route"].(map[string]any)
	sets, _ := route["rule_set"].([]any)
	for _, v := range sets {
		if m, ok := v.(map[string]any); ok {
			delete(m, "download_detour")
		}
	}
	final, e := core.RunHook(ctx, hook, "onFinalize", core.Object{"singbox": o})
	if e != nil {
		return e
	}
	if e = k.readLaunch(final); e != nil {
		return e
	}
	if e = core.AtomicWrite(k.workspace("config.json"), []byte(core.Pretty(final))); e != nil {
		return e
	}
	checkCtx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	check := exec.CommandContext(checkCtx, k.Binary(), "check", "-c", k.workspace("config.json"))
	hideCommand(check)
	if b, e := check.CombinedOutput(); e != nil {
		return fmt.Errorf("sing-box check: %w: %s", e, strings.TrimSpace(string(b)))
	}
	if e = os.MkdirAll(filepath.Dir(k.LogPath()), 0700); e != nil {
		return e
	}
	log, e := os.OpenFile(k.LogPath(), os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0600)
	if e != nil {
		return e
	}
	cmd := exec.Command(k.Binary(), "run", "-c", k.workspace("config.json"))
	cmd.Dir = filepath.Dir(k.workspace("config.json"))
	cmd.Stdout = log
	cmd.Stderr = log
	hideCommand(cmd)
	if e = cmd.Start(); e != nil {
		log.Close()
		return e
	}
	k.cmd = cmd
	k.owned = true
	k.mark = marker{PID: cmd.Process.Pid}
	k.mark.CreatedAt, e = creationTime(k.mark.PID)
	if e != nil {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		log.Close()
		return e
	}
	k.done = make(chan error, 1)
	go func() { k.done <- cmd.Wait(); _ = log.Close() }()
	k.running = true
	k.startedAt = time.Now()
	if e = core.AtomicWrite(k.workspace("pid.txt"), []byte(core.Pretty(k.mark))); e != nil {
		_ = k.Stop()
		return e
	}
	deadline := time.NewTimer(60 * time.Second)
	defer deadline.Stop()
	tick := time.NewTicker(100 * time.Millisecond)
	defer tick.Stop()
	for {
		select {
		case e := <-k.done:
			k.running = false
			_ = os.Remove(k.workspace("pid.txt"))
			return fmt.Errorf("kernel exited during startup: %v; %s", e, k.LogTail())
		case <-ctx.Done():
			_ = k.Stop()
			return ctx.Err()
		case <-deadline.C:
			_ = k.Stop()
			return fmt.Errorf("kernel controller startup timed out")
		case <-tick.C:
			var version core.Object
			if e = k.Request(ctx, "GET", "/version", nil, &version); e == nil {
				if proxy {
					if k.endpoint == "" {
						_ = k.Stop()
						return fmt.Errorf("system proxy needs a mixed inbound")
					}
					if e = k.proxy.Set(k.endpoint); e != nil {
						_ = k.Stop()
						return e
					}
				}
				return nil
			}
		}
	}
}
func (k *Kernel) Stop() error {
	if !k.Running() {
		return k.proxy.Restore()
	}
	stamp, e := creationTime(k.mark.PID)
	if e != nil || stamp != k.mark.CreatedAt {
		return fmt.Errorf("kernel process identity changed")
	}
	if e = terminateVerified(k.mark.PID, k.mark.CreatedAt); e != nil {
		return e
	}
	if k.owned && k.done != nil {
		select {
		case <-k.done:
		case <-time.After(5 * time.Second):
			return fmt.Errorf("kernel did not exit")
		}
	}
	k.running = false
	k.owned = false
	k.cmd = nil
	_ = os.Remove(k.workspace("pid.txt"))
	return k.proxy.Restore()
}
func (k *Kernel) Shutdown() error {
	if k.owned {
		return k.Stop()
	}
	return k.proxy.Restore()
}
func (k *Kernel) SetProxy(on bool) error {
	if !on {
		return k.proxy.Restore()
	}
	if !k.Running() {
		return nil
	}
	if k.endpoint == "" {
		return fmt.Errorf("active configuration has no mixed inbound")
	}
	return k.proxy.Set(k.endpoint)
}
func (k *Kernel) Request(ctx context.Context, method, path string, body any, out any) error {
	if k.controller == "" {
		return fmt.Errorf("kernel is stopped")
	}
	ctx, cancel := context.WithTimeout(ctx, 7*time.Second)
	defer cancel()
	var reader io.Reader
	if body != nil {
		b, e := json.Marshal(body)
		if e != nil {
			return e
		}
		reader = bytes.NewReader(b)
	}
	req, e := http.NewRequestWithContext(ctx, method, k.controller+path, reader)
	if e != nil {
		return e
	}
	req.Header.Set("Authorization", "Bearer "+k.secret)
	req.Header.Set("Content-Type", "application/json")
	resp, e := controllerClient.Do(req)
	if e != nil {
		return e
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("Clash API HTTP %d", resp.StatusCode)
	}
	if out != nil {
		return json.NewDecoder(io.LimitReader(resp.Body, 8<<20)).Decode(out)
	}
	return nil
}
func (k *Kernel) LogTail() string {
	f, e := os.Open(k.LogPath())
	if e != nil {
		return ""
	}
	defer f.Close()
	info, e := f.Stat()
	if e != nil {
		return ""
	}
	if info.Size() > 64<<10 {
		_, _ = f.Seek(-(64 << 10), io.SeekEnd)
	}
	b, _ := io.ReadAll(f)
	return string(b)
}
func (k *Kernel) Version(ctx context.Context) string {
	ctx, cancel := context.WithTimeout(ctx, 5*time.Second)
	defer cancel()
	c := exec.CommandContext(ctx, k.Binary(), "version")
	hideCommand(c)
	b, e := c.Output()
	if e != nil {
		return e.Error()
	}
	return strings.TrimSpace(string(b))
}
func escape(s string) string { return url.PathEscape(s) }
