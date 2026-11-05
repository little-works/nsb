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
	return ui.MustParseSVG([]byte(`<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">` + path + `</svg>`))
}

var icons = map[string]*ui.SVG{
	"proxies":     svg(`<circle cx="6" cy="17" r="3"/><circle cx="18" cy="7" r="3"/><path d="M6 14V7a3 3 0 0 1 3-3h2m7 6v7a3 3 0 0 1-3 3h-2M9 17l6-10"/>`),
	"profiles":    svg(`<rect x="4" y="3" width="16" height="18" rx="3"/><path d="M8 8h8m-8 4h5m-5 4h3"/><circle cx="16" cy="16" r="1"/>`),
	"templates":   svg(`<rect x="4" y="4" width="16" height="16" rx="3"/><path d="M4 10h16m-10 0v10M8 7h.01m3 0h.01"/>`),
	"logs":        svg(`<rect x="3" y="4" width="18" height="16" rx="3"/><path d="m7 9 3 3-3 3m6 0h4"/>`),
	"connections": svg(`<rect x="3" y="9" width="5" height="6" rx="1.5"/><rect x="16" y="3" width="5" height="6" rx="1.5"/><rect x="16" y="15" width="5" height="6" rx="1.5"/><path d="M8 12h4m4-6h-4v12h4"/>`),
	"settings":    svg(`<path d="M4 7h4m4 0h8M4 17h8m4 0h4"/><circle cx="10" cy="7" r="2"/><circle cx="14" cy="17" r="2"/>`),
	"refresh":     svg(`<path d="M20 4v5h-5M4 20v-5h5m-4.2-6a7.5 7.5 0 0 1 12.4-3L20 9M4 15l2.8 3A7.5 7.5 0 0 0 19.2 15"/>`),
	"appearance":  svg(`<rect x="3" y="4" width="18" height="13" rx="3"/><path d="M9 21h6m-3-4v4M12 4v13"/>`),
	"kernel":      svg(`<rect x="6" y="6" width="12" height="12" rx="3"/><rect x="10" y="10" width="4" height="4" rx="1"/><path d="M9 3v3m6-3v3M9 18v3m6-3v3M3 9h3m-3 6h3m12-6h3m-3 6h3"/>`),
	"archive":     svg(`<rect x="3" y="4" width="18" height="5" rx="1.5"/><path d="M5 9v10a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V9m-9 4h4"/>`),
	"check":       svg(`<path d="m5 12 4 4L19 6"/>`),
	"speed":       svg(`<path d="M4 18a9 9 0 1 1 16 0M12 13l5-6M5 13h1m12 0h1M8 6l1 1"/><circle cx="12" cy="13" r="2"/>`),
	"locate":      svg(`<circle cx="12" cy="12" r="7"/><circle cx="12" cy="12" r="2"/><path d="M12 2v3m0 14v3M2 12h3m14 0h3"/>`),
	"stop":        svg(`<rect x="6" y="6" width="12" height="12" rx="2"/>`),
	"edit":        svg(`<path d="m15 4 5 5M4 20l5-1L20 8a2.1 2.1 0 0 0-5-5L4 14Z"/>`),
	"code":        svg(`<path d="m8 6-6 6 6 6m8-12 6 6-6 6m-3-15-2 18"/>`),
	"delete":      svg(`<path d="M3 6h18M9 6V3h6v3M5 6l1 14h12l1-14M10 10v6m4-6v6"/>`),
}

func setAppTheme(c *ui.Context) {
	t := *c.Theme()
	t.Radius = 6
	t.FontSize = 14
	t.Spacing = 4
	t.Font = "Segoe UI"
	if strings.HasPrefix(c.Locale(), "zh") {
		t.Font = "Microsoft YaHei UI"
	}
	if t.Dark {
		t.Background = ui.Hex("#152129")
		t.Surface = ui.Hex("#1D2D36")
		t.SurfaceHover = ui.Hex("#283F49")
		t.SurfacePressed = ui.Hex("#304E57")
		t.Border = ui.Hex("#3B515D")
		t.Text = ui.Hex("#E5EDF0")
		t.TextMuted = ui.Hex("#AEC0CA")
		t.Accent = ui.Hex("#65D4CF")
		t.AccentHover = ui.Hex("#8AE3DE")
		t.AccentPressed = ui.Hex("#4BBDB8")
		t.AccentText = ui.Hex("#102C32")
		t.Success = ui.Hex("#7AD6AC")
		t.Warning = ui.Hex("#EDBD72")
		t.Danger = ui.Hex("#FF9AA5")
	} else {
		t.Background = ui.Hex("#F5F7F8")
		t.Surface = ui.Hex("#FFFFFF")
		t.SurfaceHover = ui.Hex("#EAF3F3")
		t.SurfacePressed = ui.Hex("#DCECEC")
		t.Border = ui.Hex("#D5DFE3")
		t.Text = ui.Hex("#223442")
		t.TextMuted = ui.Hex("#566977")
		t.Accent = ui.Hex("#007F86")
		t.AccentHover = ui.Hex("#006D74")
		t.AccentPressed = ui.Hex("#005C63")
		t.AccentText = ui.Hex("#FFFFFF")
		t.Success = ui.Hex("#237A57")
		t.Warning = ui.Hex("#936018")
		t.Danger = ui.Hex("#BC3549")
	}
	t.Focus = t.Accent
	t.Selection = t.Accent.Alpha(.24)
	c.SetTheme(&t)
}
func panel(c *ui.Context) ui.Element {
	return ui.Column(c).FillWidth().Padding(20).Gap(16).Background(c.Theme().Surface).Border(1, c.Theme().Border).Radius(10)
}
func pageContent(c *ui.Context) ui.Element {
	return ui.Column(c).FillWidth().MaxWidth(1320).AlignSelf(ui.Center).Padding(0, 28, 28, 28)
}
func pageToolbar(c *ui.Context) ui.Element {
	return ui.Row(c).FillWidth().MaxWidth(1320).AlignSelf(ui.Center).Padding(0, 28).Gap(12).AlignItems(ui.Center)
}
func pageScroll(c *ui.Context) ui.Element {
	return ui.Scroll(c).FillWidth().Grow(1).MinHeight(0).Label("Page content")
}
func muted(c *ui.Context, s string) ui.Element {
	return ui.Text(c, s).FontSize(12).TextColor(c.Theme().TextMuted)
}
func iconButton(c *ui.Context, icon *ui.SVG, label string) ui.Element {
	return ui.Button(c, "").Label(label).Gap(7).Children(func() {
		ui.Icon(c, icon).Size(16, 16)
		if label != "" {
			ui.Text(c, label)
		}
	})
}

func textIconButton(c *ui.Context, icon *ui.SVG, label string) ui.Element {
	b := ui.ButtonBase(c).Size(36, 36).Radius(6).Label(label).Tooltip(label).TextColor(c.Theme().TextMuted)
	if b.Pressed() {
		b.Background(c.Theme().SurfacePressed)
	} else if b.Hovered() {
		b.Background(c.Theme().SurfaceHover)
	}
	return b.Children(func() { ui.Icon(c, icon).Size(18, 18) })
}
func section(c *ui.Context, icon *ui.SVG, title string) {
	ui.Row(c).Gap(10).AlignItems(ui.Center).Children(func() {
		ui.Icon(c, icon).Size(20, 20).TextColor(c.Theme().TextMuted)
		ui.Text(c, title).FontSize(16).FontWeight(600)
	})
}
func emptyState(c *ui.Context, icon *ui.SVG, title, description string) {
	ui.Column(c).FillWidth().Padding(36).Gap(12).AlignItems(ui.Center).Children(func() {
		ui.Icon(c, icon).Size(32, 32).TextColor(c.Theme().TextMuted)
		ui.Text(c, title).FontSize(16).FontWeight(600)
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
