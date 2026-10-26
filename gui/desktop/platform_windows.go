package desktop

import (
	"fmt"
	"golang.org/x/sys/windows"
	"golang.org/x/sys/windows/registry"
	"os/exec"
	"syscall"
	"time"
	"unsafe"
)

func hideCommand(c *exec.Cmd) { c.SysProcAttr = &syscall.SysProcAttr{HideWindow: true} }
func creationTime(pid int) (uint64, error) {
	h, e := windows.OpenProcess(windows.PROCESS_QUERY_LIMITED_INFORMATION, false, uint32(pid))
	if e != nil {
		return 0, e
	}
	defer windows.CloseHandle(h)
	var created, exit, kernel, user windows.Filetime
	if e = windows.GetProcessTimes(h, &created, &exit, &kernel, &user); e != nil {
		return 0, e
	}
	return uint64(created.HighDateTime)<<32 | uint64(created.LowDateTime), nil
}

func processStartedAt(pid int) time.Time {
	stamp, e := creationTime(pid)
	if e != nil {
		return time.Time{}
	}
	return time.Unix(0, int64(stamp-116444736000000000)*100)
}
func terminateVerified(pid int, stamp uint64) error {
	h, e := windows.OpenProcess(windows.PROCESS_QUERY_LIMITED_INFORMATION|windows.PROCESS_TERMINATE, false, uint32(pid))
	if e != nil {
		return e
	}
	defer windows.CloseHandle(h)
	var created, exit, kernel, user windows.Filetime
	if e = windows.GetProcessTimes(h, &created, &exit, &kernel, &user); e != nil {
		return e
	}
	if uint64(created.HighDateTime)<<32|uint64(created.LowDateTime) != stamp {
		return fmt.Errorf("kernel process identity changed")
	}
	return windows.TerminateProcess(h, 1)
}

type proxyValue struct {
	Name   string
	Kind   uint32
	Data   []byte
	Exists bool
}
type systemProxy struct {
	previous []proxyValue
	active   bool
}

const internetSettings = `Software\Microsoft\Windows\CurrentVersion\Internet Settings`

func (p *systemProxy) Set(endpoint string) error {
	k, e := registry.OpenKey(registry.CURRENT_USER, internetSettings, registry.QUERY_VALUE|registry.SET_VALUE)
	if e != nil {
		return e
	}
	defer k.Close()
	if !p.active {
		p.previous = nil
		for _, name := range []string{"ProxyEnable", "ProxyServer", "ProxyOverride"} {
			n, typ, e := k.GetValue(name, nil)
			if e == registry.ErrNotExist {
				p.previous = append(p.previous, proxyValue{Name: name})
				continue
			}
			if e != nil {
				return e
			}
			b := make([]byte, n)
			_, _, e = k.GetValue(name, b)
			if e != nil {
				return e
			}
			p.previous = append(p.previous, proxyValue{name, typ, b, true})
		}
		p.active = true
	}
	if e = k.SetStringValue("ProxyServer", endpoint); e != nil {
		return e
	}
	if e = k.SetStringValue("ProxyOverride", "<local>;localhost;127.*;[::1]"); e != nil {
		return e
	}
	if e = k.SetDWordValue("ProxyEnable", 1); e != nil {
		return e
	}
	notifyProxy()
	return nil
}
func (p *systemProxy) Restore() error {
	if !p.active {
		return nil
	}
	k, e := registry.OpenKey(registry.CURRENT_USER, internetSettings, registry.SET_VALUE)
	if e != nil {
		return e
	}
	defer k.Close()
	for _, v := range p.previous {
		if v.Exists {
			set := windows.NewLazySystemDLL("advapi32.dll").NewProc("RegSetValueExW")
			code, _, _ := set.Call(uintptr(k), uintptr(unsafe.Pointer(mustUTF16(v.Name))), 0, uintptr(v.Kind), uintptr(unsafe.Pointer(ptr(v.Data))), uintptr(len(v.Data)))
			if code != 0 {
				return fmt.Errorf("restore proxy: %w", syscall.Errno(code))
			}
		} else if e = k.DeleteValue(v.Name); e != nil && e != registry.ErrNotExist {
			return e
		}
	}
	p.active = false
	notifyProxy()
	return nil
}
func ptr(b []byte) *byte {
	if len(b) == 0 {
		return nil
	}
	return &b[0]
}
func mustUTF16(s string) *uint16 { p, _ := windows.UTF16PtrFromString(s); return p }
func notifyProxy() {
	dll := windows.NewLazySystemDLL("wininet.dll")
	fn := dll.NewProc("InternetSetOptionW")
	_, _, _ = fn.Call(0, 39, 0, 0)
	_, _, _ = fn.Call(0, 37, 0, 0)
}
