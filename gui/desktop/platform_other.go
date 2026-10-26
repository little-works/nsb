//go:build !windows

package desktop

import (
	"fmt"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"time"
)

func hideCommand(c *exec.Cmd) {}
func creationTime(pid int) (uint64, error) {
	b, e := exec.Command("ps", "-p", strconv.Itoa(pid), "-o", "lstart=").Output()
	if e != nil {
		return 0, e
	}
	var h uint64 = 14695981039346656037
	for _, v := range strings.TrimSpace(string(b)) {
		h ^= uint64(v)
		h *= 1099511628211
	}
	return h, nil
}

func processStartedAt(pid int) time.Time {
	b, e := exec.Command("ps", "-p", strconv.Itoa(pid), "-o", "lstart=").Output()
	if e != nil {
		return time.Time{}
	}
	t, _ := time.ParseInLocation(time.ANSIC, strings.TrimSpace(string(b)), time.Local)
	return t
}
func terminateVerified(pid int, stamp uint64) error {
	p, e := os.FindProcess(pid)
	if e != nil {
		return e
	}
	defer p.Release()
	current, e := creationTime(pid)
	if e != nil {
		return e
	}
	if current != stamp {
		return fmt.Errorf("kernel process identity changed")
	}
	return p.Kill()
}

type systemProxy struct{}

func (*systemProxy) Set(endpoint string) error {
	return fmt.Errorf("system proxy configuration is not supported on this platform")
}
func (*systemProxy) Restore() error { return nil }
