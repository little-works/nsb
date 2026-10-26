package app

import (
	"fmt"
	"strings"

	"github.com/egoist/mygo/ui"
	"github.com/egoist/mygo/ui/editor"

	assets "nsb/gui"
)

var logo, _ = ui.DecodeBitmap(assets.Icon)

func svg(path string) *ui.SVG {
	return ui.MustParseSVG([]byte(`<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">` + path + `</svg>`))
}

var icons = map[string]*ui.SVG{
	"proxies":     svg(`<path d="M4 13a8 8 0 0 1 16 0v5H4zM12 13l4-5"/><circle cx="12" cy="13" r="1"/>`),
	"profiles":    svg(`<path d="M4 7h16M4 12h16M4 17h10"/><circle cx="19" cy="17" r="2"/>`),
	"templates":   svg(`<path d="m12 3 9 5-9 5-9-5 9-5zm-9 9 9 5 9-5M3 16l9 5 9-5"/>`),
	"logs":        svg(`<rect x="3" y="4" width="18" height="16" rx="3"/><path d="m7 9 3 3-3 3m6 0h4"/>`),
	"connections": svg(`<circle cx="5" cy="12" r="2"/><circle cx="19" cy="5" r="2"/><circle cx="19" cy="19" r="2"/><path d="M7 12h5V5h5m-5 7v7h5"/>`),
	"settings":    svg(`<path d="M5 4v16m7-16v16m7-16v16M2 8h6m1 8h6m1-9h6"/><circle cx="5" cy="8" r="2"/><circle cx="12" cy="16" r="2"/><circle cx="19" cy="7" r="2"/>`),
	"refresh":     svg(`<path d="M20 7v5h-5M4 17v-5h5m-4-5a8 8 0 0 1 13-2l2 2M4 17l2 2a8 8 0 0 0 13-2"/>`),
	"power":       svg(`<path d="M12 3v9m-6-7a9 9 0 1 0 12 0"/>`),
	"file":        svg(`<path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9zm0 0v6h6M8 13h8m-8 4h5"/>`),
	"check":       svg(`<path d="m5 12 4 4L19 6"/>`),
}

func setAppTheme(c *ui.Context) {
	t := *c.Theme()
	t.Radius = 7
	t.FontSize = 14
	t.Spacing = 4
	if strings.HasPrefix(c.Locale(), "zh") {
		t.Font = "Microsoft YaHei UI"
	}
	if t.Dark {
		t.Background = ui.Hex("#101722")
		t.Surface = ui.Hex("#192230")
		t.SurfaceHover = ui.Hex("#243249")
		t.SurfacePressed = ui.Hex("#2b3e5b")
		t.Border = ui.Hex("#2b3749")
		t.Text = ui.Hex("#e6edf7")
		t.TextMuted = ui.Hex("#94a4ba")
		t.Accent = ui.Hex("#7caaff")
		t.AccentText = ui.Hex("#10213c")
	} else {
		t.Background = ui.Hex("#f4f6fb")
		t.Surface = ui.Hex("#ffffff")
		t.SurfaceHover = ui.Hex("#edf3ff")
		t.SurfacePressed = ui.Hex("#e2ebfc")
		t.Border = ui.Hex("#e0e6f0")
		t.Text = ui.Hex("#1c2940")
		t.TextMuted = ui.Hex("#77859a")
		t.Accent = ui.Hex("#2463ce")
		t.AccentText = ui.Hex("#ffffff")
	}
	t.AccentHover = t.Accent
	t.AccentPressed = t.Accent
	t.Focus = t.Accent
	c.SetTheme(&t)
}
func panel(c *ui.Context) ui.Element {
	return ui.Column(c).FillWidth().Padding(20).Gap(14).Background(c.Theme().Surface).Border(1, c.Theme().Border).Radius(12)
}
func pageContent(c *ui.Context) ui.Element {
	return ui.Column(c).FillWidth().MaxWidth(1320).AlignSelf(ui.Center).Padding(0, 28, 28, 28)
}
func pageToolbar(c *ui.Context) ui.Element {
	return ui.Row(c).FillWidth().MaxWidth(1320).AlignSelf(ui.Center).Padding(0, 28).Gap(10)
}
func pageScroll(c *ui.Context) ui.Element {
	return ui.Scroll(c).FillWidth().Grow(1).MinHeight(0).Label("Page content")
}
func muted(c *ui.Context, s string) ui.Element {
	return ui.Text(c, s).FontSize(12).TextColor(c.Theme().TextMuted)
}
func iconButton(c *ui.Context, icon *ui.SVG, label string) ui.Element {
	return ui.Button(c, "").Label(label).Gap(7).Children(func() {
		ui.Icon(c, icon).Size(15, 15)
		if label != "" {
			ui.Text(c, label)
		}
	})
}
func section(c *ui.Context, icon *ui.SVG, title, description string) {
	ui.Row(c).Gap(12).AlignItems(ui.Center).Children(func() {
		ui.Icon(c, icon).Size(20, 20).TextColor(c.Theme().Accent)
		ui.Column(c).Gap(4).Children(func() {
			ui.Text(c, title).FontSize(16).FontWeight(600)
			if description != "" {
				muted(c, description).Wrap()
			}
		})
	})
}
func emptyState(c *ui.Context, icon *ui.SVG, title, description string) {
	ui.Column(c).FillWidth().Padding(36).Gap(12).AlignItems(ui.Center).Children(func() {
		ui.Icon(c, icon).Size(38, 38).TextColor(c.Theme().TextMuted)
		ui.Text(c, title).FontSize(17).FontWeight(600)
		muted(c, description).Wrap().TextAlign(ui.Center)
	})
}
func bytesLabel(v any) string {
	n, _ := v.(float64)
	switch {
	case n >= 1<<30:
		return fmt.Sprintf("%.1f GB", n/(1<<30))
	case n >= 1<<20:
		return fmt.Sprintf("%.1f MB", n/(1<<20))
	case n >= 1<<10:
		return fmt.Sprintf("%.1f KB", n/(1<<10))
	default:
		return fmt.Sprintf("%.0f B", n)
	}
}

type codeState struct {
	target *string
	source string
	editor editor.State
}

func (a *App) codeEditor(c *ui.Context, key string, value *string, language string) ui.Element {
	if a.code == nil {
		a.code = map[string]*codeState{}
	}
	s := a.code[key]
	if s == nil || s.target != value {
		s = &codeState{target: value, source: *value, editor: editor.State{Buffer: editor.NewBuffer(*value), SoftWrap: true, TabSize: 2, NoScrollPastEnd: true, Label: language}}
		s.editor.Buffer.SetLanguage(editor.LanguageFor("source." + language))
		a.code[key] = s
	} else if *value != s.source {
		s.editor.Buffer.Reset(*value)
		s.source = *value
	}
	c.Key(key)
	e := editor.View(c, &s.editor).Border(1, c.Theme().Border).Radius(8)
	if current := s.editor.Buffer.String(); current != s.source {
		*value = current
		s.source = current
	}
	return e
}
