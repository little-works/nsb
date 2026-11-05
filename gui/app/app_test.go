package app

import (
	"image/png"
	"math"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/egoist/mygo/ui"
	"nsb/core"
	"nsb/gui/desktop"
)

func testApp(t *testing.T) *App {
	t.Helper()
	dir := t.TempDir()
	m, e := desktop.Open(filepath.Join(dir, "config"), filepath.Join(dir, "data"))
	if e != nil {
		t.Fatal(e)
	}
	a := New(m)
	t.Cleanup(a.cancel)
	return a
}
func click(t *testing.T, view *ui.Tester, label string) {
	t.Helper()
	if e := view.Click(label); e != nil {
		t.Fatal(e)
	}
	view.Frame()
}
func TestNativeNavigationAndEditorLifetime(t *testing.T) {
	a := testApp(t)
	view := ui.NewTester(a.View, 1180, 820)
	view.Frame()
	for _, label := range []string{"订阅", "模板", "日志", "连接", "设置", "代理", "订阅"} {
		click(t, view, label)
		view.Frame()
	}
	click(t, view, "新建订阅")
	if a.editor == nil {
		t.Fatal("profile editor did not open")
	}
	view.Frame()
	view.Frame()
	if !view.HasText("内联 sing-box 模板（JSON）") {
		t.Fatal("inline template editor missing")
	}
	click(t, view, "取消")
	view.Frame()
	click(t, view, "新建订阅")
	view.Frame()
	if a.editor == nil {
		t.Fatal("editor did not return")
	}
	click(t, view, "取消")
	click(t, view, "模板")
	click(t, view, "新建模板")
	view.Frame()
	click(t, view, "取消")
	view.Frame()
	click(t, view, "日志")
	view.Frame()
	click(t, view, "代理")
	view.Frame()
}
func TestProfileTypingPersistsAcrossFrames(t *testing.T) {
	a := testApp(t)
	a.page = "profiles"
	a.editor = newProfileEditor(core.Profile{ID: "p", Name: "before"}, core.DefaultConfig())
	inline := core.DefaultTemplate
	a.editor.Profile.InlineTemplate = &inline
	view := ui.NewTester(a.View, 1180, 820)
	view.Frame()
	box, ok := view.Find("名称")
	if !ok {
		t.Fatal("name label missing")
	}
	// The label is followed by its input in the vertical form.
	view.ClickAt(box.X+80, box.Y+box.H+20)
	view.Key(ui.Ctrl, ui.KeyA)
	view.Type("after")
	view.Frame()
	view.Frame()
	if a.editor.Profile.Name != "after" {
		t.Fatalf("typed name lost: %q", a.editor.Profile.Name)
	}
	click(t, view, "取消")
	view.Frame()
	a.editor = newProfileEditor(core.Profile{ID: "p", Name: "return"}, core.DefaultConfig())
	a.editor.Profile.InlineTemplate = &inline
	view.Frame()
	if a.editor.Profile.Name != "return" {
		t.Fatal("editor did not rebuild")
	}
}
func TestCompactProfileForm(t *testing.T) {
	a := testApp(t)
	a.page = "profiles"
	a.snapshot.Config.Templates = []core.Template{{ID: "t", Name: "Default", Content: core.DefaultTemplate}}
	a.editor = newProfileEditor(core.Profile{ID: "p", Name: "Profile", TemplateID: "t", Remotes: []core.Remote{{Name: "Source", Format: "clash"}}}, a.snapshot.Config)
	view := ui.NewTester(a.profileEditorView, 960-232, 650)
	for _, language := range []string{"zh-CN", "en-US"} {
		a.language = language
		view.SetLocale(language)
		for _, width := range []int{960, 1180, 1920} {
			view.SetSize(width-232, 650)
			view.Frame()
			view.Frame()
			for _, pair := range [][2]string{
				{a.t("Name", "名称"), a.t("Template", "模板")},
				{a.t("Update interval (hours)", "更新间隔（小时）"), "Cron"},
				{a.t("Source name", "来源名称"), a.t("Format", "格式")},
			} {
				left, leftOK := view.Find(pair[0])
				right, rightOK := view.Find(pair[1])
				if !leftOK || !rightOK || math.Abs(float64(left.Y-right.Y)) > 1 || left.X+left.W >= right.X {
					t.Fatalf("%s width=%v fields not side by side: %v, %+v, %+v", language, width, pair, left, right)
				}
			}
		}
	}
	box, ok := view.Find("URL")
	if !ok {
		t.Fatal("source URL missing")
	}
	view.ClickAt(box.X+80, box.Y+box.H/2)
	view.Type("https://example.com/sub")
	view.Frame()
	view.Frame()
	if a.editor.Remotes[0].Remote.URL != "https://example.com/sub" {
		t.Fatal("source URL edit lost")
	}
	click(t, view, "Use inline template")
	view.Frame()
	if a.editor.Profile.InlineTemplate == nil || !view.HasText("Inline sing-box template (JSON)") {
		t.Fatal("inline template did not open")
	}
	click(t, view, "Use a saved template")
	view.Frame()
	if a.editor.Profile.InlineTemplate != nil || a.editor.TemplateName != "Default" {
		t.Fatal("saved template did not return")
	}
}

func TestAllNativePagesRender(t *testing.T) {
	a := testApp(t)
	a.language = "zh-CN"
	a.snapshot.Running = true
	a.snapshot.Uptime = 2*time.Hour + 36*time.Minute
	a.snapshot.UploadRate = 12400
	a.snapshot.DownloadRate = 328000
	a.snapshot.Version = "sing-box version 1.12.0"
	nodes := []any{"日本 · 东京 01", "日本 · 东京 02", "新加坡 · 狮城 01", "美国 · 洛杉矶 01", "德国 · 法兰克福 01", "香港 · 专线 01"}
	proxies := core.Object{"GLOBAL": core.Object{"type": "Fallback", "now": nodes[0], "all": nodes}, "PROXY": core.Object{"type": "Selector", "now": "主订阅", "all": []any{"主订阅", "备用订阅"}}, "主订阅": core.Object{"type": "Selector", "now": nodes[0], "all": nodes}, "备用订阅": core.Object{"type": "Selector", "now": nodes[3], "all": nodes[3:]}}
	for _, n := range nodes {
		proxies[n.(string)] = core.Object{"type": "Shadowsocks"}
	}
	a.snapshot.Proxies = core.Object{"proxies": proxies}
	a.snapshot.ClashConfig = core.Object{"mode": "rule"}
	a.delays[nodes[0].(string)] = 42
	a.delays[nodes[2].(string)] = 68
	a.snapshot.Connections = core.Object{"connections": []any{core.Object{"id": "conn1", "metadata": core.Object{"host": "www.example.com", "destinationPort": "443", "network": "tcp", "process": "msedge.exe"}, "upload": 102400.0, "download": 12600000.0, "chains": []any{"日本 · 东京 01", "PROXY"}}}}
	a.snapshot.Logs = "2026-10-26  DEBUG  router: loading rules\n2026-10-26  INFO  sing-box started\n2026-10-26  WARN  outbound: connection timeout, retrying\n2026-10-26  ERROR  dns: query failed\n2026-10-26  INFO  outbound: connected to example.com:443"
	a.snapshot.Config.Templates = []core.Template{{ID: "t", Name: "日常分流模板", Content: core.DefaultTemplate}, {ID: "t2", Name: "轻量直连模板", Content: `{"outbounds":[]}`}}
	a.snapshot.Config.Profiles = []core.Profile{{ID: "p", Name: "主力订阅", TemplateID: "t", UpdatedAt: 1793000000, Remotes: []core.Remote{{Name: "主订阅"}, {Name: "备用订阅"}}}, {ID: "p2", Name: "备用订阅", TemplateID: "t", Remotes: []core.Remote{}}}
	id := "p"
	a.snapshot.Config.CurrentProfileID = &id
	view := ui.NewTester(a.View, 1180, 820)
	view.SetLocale(a.language)
	for _, page := range []string{"proxies", "profiles", "templates", "logs", "connections", "settings", "template-editor", "profile-editor"} {
		a.page = page
		a.template = nil
		a.editor = nil
		if page == "template-editor" {
			a.page = "templates"
			a.template = &templateEditor{Template: a.snapshot.Config.Templates[0]}
		}
		if page == "profile-editor" {
			a.page = "profiles"
			a.editor = newProfileEditor(a.snapshot.Config.Profiles[0], a.snapshot.Config)
		}
		view.Frame()
		view.Frame()
		for _, dark := range []bool{false, true} {
			view.SetSize(1180, 820)
			view.SetDark(dark)
			view.Frame()
			if dir := os.Getenv("NSB_UI_PREVIEW_DIR"); dir != "" {
				if e := os.MkdirAll(dir, 0700); e != nil {
					t.Fatal(e)
				}
				name := page
				if dark {
					name += "-dark"
				}
				f, e := os.Create(filepath.Join(dir, name+".png"))
				if e != nil {
					t.Fatal(e)
				}
				e = png.Encode(f, view.Image())
				f.Close()
				if e != nil {
					t.Fatal(e)
				}
				view.SetSize(960, 700)
				view.Frame()
				f, e = os.Create(filepath.Join(dir, name+"-narrow.png"))
				if e != nil {
					t.Fatal(e)
				}
				e = png.Encode(f, view.Image())
				f.Close()
				if e != nil {
					t.Fatal(e)
				}
				if page == "proxies" || page == "profiles" {
					view.SetSize(1920, 1080)
					view.Frame()
					f, e = os.Create(filepath.Join(dir, name+"-wide.png"))
					if e != nil {
						t.Fatal(e)
					}
					e = png.Encode(f, view.Image())
					f.Close()
					if e != nil {
						t.Fatal(e)
					}
				}
			}
		}
	}
}

func TestCodeEditorSync(t *testing.T) {
	a := testApp(t)
	value := "{}"
	view := ui.NewTester(func(c *ui.Context) { a.codeEditor(c, "code", &value, "json").Fill() }, 600, 400)
	view.Frame()
	a.code["code"].editor.Buffer.Set(`{"test":true}`)
	view.Frame()
	view.Frame()
	if value != `{"test":true}` {
		t.Fatalf("editor changes lost: %s", value)
	}
	value = `{"external":true}`
	view.Frame()
	if a.code["code"].editor.Buffer.String() != value {
		t.Fatal("external update not shown")
	}
}

func TestNarrowConnectionActionVisible(t *testing.T) {
	a := testApp(t)
	a.page = "connections"
	a.snapshot.Running = true
	a.snapshot.Connections = core.Object{"connections": []any{core.Object{"id": "conn", "metadata": core.Object{"host": "example.com"}}}}
	view := ui.NewTester(a.View, 960, 650)
	for _, language := range []string{"zh-CN", "en-US"} {
		a.language = language
		view.SetLocale(language)
		view.Frame()
		view.Frame()
		bounds, ok := view.Find(a.t("Close", "关闭"))
		if !ok || bounds.X < 232 || bounds.X+bounds.W > 960-28 {
			t.Fatalf("%s connection action clipped: %+v, found=%v", language, bounds, ok)
		}
	}
}

func TestThemeTextContrast(t *testing.T) {
	luminance := func(color ui.Color) float64 {
		linear := func(value uint8) float64 {
			v := float64(value) / 255
			if v <= .04045 {
				return v / 12.92
			}
			return math.Pow((v+.055)/1.055, 2.4)
		}
		return .2126*linear(color.R) + .7152*linear(color.G) + .0722*linear(color.B)
	}
	var theme ui.Theme
	view := ui.NewTester(func(c *ui.Context) {
		setAppTheme(c)
		theme = *c.Theme()
	}, 200, 100)
	for _, dark := range []bool{false, true} {
		view.SetDark(dark)
		view.Frame()
		for _, pair := range []struct {
			name       string
			text, base ui.Color
		}{
			{"body", theme.Text, theme.Background},
			{"secondary", theme.TextMuted, theme.Surface},
			{"selected secondary", theme.TextMuted, theme.SurfacePressed},
			{"primary", theme.AccentText, theme.Accent},
			{"primary hover", theme.AccentText, theme.AccentHover},
			{"primary pressed", theme.AccentText, theme.AccentPressed},
			{"info", theme.Accent, theme.Surface},
			{"warning", theme.Warning, theme.Surface},
			{"error", theme.Danger, theme.Surface},
		} {
			lo, hi := luminance(pair.text), luminance(pair.base)
			contrast := (max(lo, hi) + .05) / (min(lo, hi) + .05)
			if contrast < 4.5 {
				t.Errorf("dark=%v %s contrast %.2f < 4.5", dark, pair.name, contrast)
			}
		}
	}
}

func TestProxyGroupsAndColumns(t *testing.T) {
	a := testApp(t)
	id := "p"
	a.snapshot.Config.CurrentProfileID = &id
	a.snapshot.Config.Profiles = []core.Profile{{ID: id, Remotes: []core.Remote{{Name: "second"}, {Name: "first"}}}}
	group := func() any { return core.Object{"all": []any{"node"}} }
	p := core.Object{"GLOBAL": group(), "PROXY": group(), "first": group(), "second": group()}
	got := strings.Join(a.proxyGroups(p), ",")
	if got != "PROXY,second,first,GLOBAL" {
		t.Fatal(got)
	}
	if got := a.proxyGroups(core.Object{"GLOBAL": group()}); len(got) != 1 || got[0] != "GLOBAL" {
		t.Fatal(got)
	}
	if !(proxyColumns(960) < proxyColumns(1180) && proxyColumns(1180) < proxyColumns(1920) && proxyColumns(1920) < proxyColumns(2560)) {
		t.Fatal("columns do not grow with window width")
	}
}
func TestLogFiltering(t *testing.T) {
	body := "2026-10-26 10:00:00 \x1b[32mINFO\x1b[0m connected\n2026-10-26 10:00:01 WARN timeout\n2026-10-26 10:00:02 ERROR failed\n2026-10-26 10:00:03 DEBUG routing"
	all := logSelection{true, true, true, true, true, true}
	lines := filterLogs(body, "", all)
	if len(lines) != 4 || lines[0].level != "DEBUG" || lines[3].level != "INFO" {
		t.Fatal(lines)
	}
	for _, line := range lines {
		if strings.Contains(line.text, "\x1b") {
			t.Fatal("ANSI escapes retained")
		}
	}
	selection := logSelection{}
	selection[3] = true
	selection[4] = true
	if got := filterLogs(body, "", selection); len(got) != 2 || got[0].level != "ERROR" || got[1].level != "WARN" {
		t.Fatal(got)
	}
	if got := filterLogs(body, "time", selection); len(got) != 1 || got[0].level != "WARN" {
		t.Fatal(got)
	}
	if got := filterLogs(body, "", logSelection{}); len(got) != 0 {
		t.Fatal("unchecked levels must not appear", got)
	}
}
func TestLongStatusKeepsPageUsable(t *testing.T) {
	a := testApp(t)
	a.status = strings.Repeat("kernel startup failed: long error output\n", 300)
	a.snapshot.Logs = "FATAL port already in use"
	view := ui.NewTester(a.View, 960, 700)
	view.Frame()
	status, ok := view.Find("Status message")
	if !ok || status.H > 56 || status.Y < 600 || status.Y+status.H > 700 {
		t.Fatalf("status must remain bounded at the bottom: %+v", status)
	}
	search, ok := view.Find("搜索节点或代理组…")
	if !ok || search.Y+search.H > status.Y {
		t.Fatal("status obscured page controls")
	}
	click(t, view, "查看日志")
	if a.page != "logs" || !view.HasText("FATAL port already in use") {
		t.Fatal("log details must remain accessible")
	}
}

func TestPageToolbarsShareAlignment(t *testing.T) {
	a := testApp(t)
	view := ui.NewTester(a.View, 1180, 820)
	for _, size := range [][2]int{{960, 700}, {1180, 820}, {1920, 1080}} {
		view.SetSize(size[0], size[1])
		var x, y float32
		for i, page := range []struct{ name, search string }{
			{"proxies", "搜索节点或代理组…"}, {"profiles", "搜索订阅"},
			{"templates", "搜索模板…"}, {"logs", "筛选日志"},
			{"connections", "搜索域名、进程或代理链…"},
		} {
			a.page = page.name
			view.Frame()
			view.Frame()
			r, ok := view.Find(page.search)
			if !ok || r.H != 36 || (page.name != "proxies" && r.W != 360) || (page.name == "proxies" && (r.W < 120 || r.W > 360)) {
				t.Fatalf("%s search has inconsistent size at %v: %+v", page.name, size, r)
			}
			if i == 0 {
				x, y = r.X, r.Y
			} else if r.X != x || r.Y != y {
				t.Fatalf("%s toolbar is misaligned at %v: %+v", page.name, size, r)
			}
		}
	}
}

func TestLogToolbarStaysVisibleAndClears(t *testing.T) {
	a := testApp(t)
	a.page = "logs"
	a.snapshot.Logs = strings.Repeat("INFO log record\n", 200)
	view := ui.NewTester(a.View, 1180, 820)
	view.Frame()
	before, ok := view.Find("清空")
	if !ok {
		t.Fatal("clear action missing")
	}
	viewport, ok := view.Find("Page content")
	if !ok || before.Y+before.H > viewport.Y {
		t.Fatal("toolbar must be above the scrolling log body")
	}
	border, ok := view.Find("Log panel")
	if !ok || viewport.Y <= border.Y || viewport.Y+viewport.H >= border.Y+border.H {
		t.Fatal("log viewport must stay inside panel borders")
	}
	view.Scroll(viewport.X+100, viewport.Y+100, 0, 500)
	view.Frame()
	after, ok := view.Find("清空")
	if !ok || before != after {
		t.Fatal("toolbar moved with log content")
	}
	if afterBorder, ok := view.Find("Log panel"); !ok || afterBorder != border {
		t.Fatal("log panel borders moved with content")
	}
	click(t, view, "清空")
	if a.snapshot.Logs != "" || view.HasText("INFO log record") {
		t.Fatal("clear action retained old records")
	}
}

func TestLogCheckboxes(t *testing.T) {
	a := testApp(t)
	a.page = "logs"
	a.snapshot.Logs = "INFO informational record\nERROR failure record"
	view := ui.NewTester(a.View, 1180, 820)
	view.Frame()
	if !view.HasText("INFO informational record") {
		t.Fatal("initial records missing")
	}
	click(t, view, "INFO")
	if a.logLevels[2] || view.HasText("INFO informational record") || !view.HasText("ERROR failure record") {
		t.Fatal("checkbox did not filter its level")
	}
	click(t, view, "INFO")
	if !view.HasText("INFO informational record") {
		t.Fatal("checked level did not return")
	}
}
func TestProxyViewportReachesWindowEdge(t *testing.T) {
	a := testApp(t)
	a.snapshot.Running = true
	a.snapshot.Proxies = core.Object{"proxies": core.Object{"PROXY": core.Object{"type": "Selector", "all": []any{"direct", "another node"}}, "direct": core.Object{"type": "Direct"}, "another node": core.Object{"type": "Shadowsocks"}}}
	view := ui.NewTester(a.View, 1180, 820)
	view.Frame()
	r, ok := view.Find("代理分组")
	if !ok || r.X+r.W < 1179 {
		t.Fatalf("proxy scroll ends inside content padding: %+v", r)
	}
	r, ok = view.Find("路由模式")
	if !ok || r.W > 210 {
		t.Fatalf("segmented control has trailing space: %+v", r)
	}
	r, ok = view.Find("direct")
	if !ok || r.H != 76 {
		t.Fatalf("proxy card must use the shared fixed height: %+v", r)
	}
	other, ok := view.Find("another node")
	if !ok || other.H != r.H || other.W != r.W {
		t.Fatalf("proxy cards must be equal sized: %+v, %+v", r, other)
	}
}
func TestPageScrollViewportsReachWindowEdge(t *testing.T) {
	a := testApp(t)
	a.snapshot.Logs = "INFO started"
	a.snapshot.Connections = core.Object{"connections": []any{core.Object{"id": "connection", "metadata": core.Object{"host": "example.com"}}}}
	view := ui.NewTester(a.View, 1920, 1080)
	for _, page := range []string{"profiles", "templates", "settings", "profile-editor"} {
		a.editor = nil
		a.page = page
		if page == "profile-editor" {
			a.page = "profiles"
			inline := core.DefaultTemplate
			a.editor = newProfileEditor(core.Profile{ID: "p", Name: "Test", InlineTemplate: &inline}, core.DefaultConfig())
		}
		view.Frame()
		view.Frame()
		r, ok := view.Find("Page content")
		if !ok || r.X+r.W < 1918 {
			t.Errorf("%s viewport ends inside the content: %+v", page, r)
		}
	}
}

func TestConnectionsContentIsBounded(t *testing.T) {
	a := testApp(t)
	a.page = "connections"
	a.snapshot.Connections = core.Object{"connections": []any{core.Object{"id": "connection", "metadata": core.Object{"host": "example.com"}}}}
	view := ui.NewTester(a.View, 1920, 1080)
	view.Frame()
	r, ok := view.Find("Connections table")
	if !ok || r.W > 1320 || r.X <= 232 || r.X+r.W >= 1920 {
		t.Fatalf("connection content is not centered and bounded: %+v", r)
	}
}
