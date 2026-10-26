package app

import (
	"fmt"
	"os"
	"strings"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
	"nsb/core"
)

func preference(c *ui.Context, title, description string, control func()) {
	ui.Row(c).FillWidth().Gap(24).AlignItems(ui.Center).Padding(2, 0).Children(func() {
		ui.Column(c).Grow(1).MinWidth(0).Gap(6).Children(func() { ui.Text(c, title).FontSize(14).FontWeight(500); muted(c, description).Wrap() })
		control()
	})
}
func (a *App) settingsView(c *ui.Context) {
	pageScroll(c).Children(func() {
		pageContent(c).Gap(20).Children(func() {
			panel(c).Children(func() {
				section(c, icons["settings"], a.t("General", "常规偏好"), a.t("Desktop behavior and network integration.", "调整桌面行为与网络设置。"))
				ui.Divider(c)
				preference(c, a.t("System proxy", "系统代理"), a.t("Route system traffic through the active mixed inbound.", "通过当前订阅的 mixed 入站接管系统代理。"), func() {
					ui.Switch(c, &a.proxy).Label(a.t("System proxy", "系统代理")).OnChange(func() { on := a.proxy; a.saveConfig(func(cfg *core.Config) { cfg.SystemProxyEnabled = on }) })
				})
				ui.Divider(c)
				preference(c, a.t("Start at login", "开机自启动"), a.t("Launch NSB when you sign in to Windows.", "登录 Windows 时自动启动 NSB。"), func() {
					ui.Switch(c, &a.startup).Label(a.t("Start at login", "开机自启动")).OnChange(func() {
						if e := mygo.App.SetOpenAtLogin(a.startup); e != nil {
							a.status = e.Error()
							a.startup = mygo.App.OpenAtLogin()
						}
					})
				})
				ui.Divider(c)
				preference(c, a.t("Language", "显示语言"), a.t("Choose the language used throughout the app.", "选择应用界面使用的语言。"), func() {
					if a.languageSource != a.language {
						a.languageSource = a.language
						a.languageChoice = 0
						if a.language == "en-US" {
							a.languageChoice = 1
						}
					}
					ui.Segmented(c, &a.languageChoice, "中文", "English").OnChange(func() {
						lang := []string{"zh-CN", "en-US"}[a.languageChoice]
						a.language = lang
						a.saveConfig(func(cfg *core.Config) { cfg.AppLanguage = lang })
					})
				})
			})
			panel(c).Children(func() {
				section(c, icons["templates"], a.t("Appearance", "外观"), a.t("A comfortable workspace, day and night.", "为白天和夜晚选择舒适的界面。"))
				ui.Divider(c)
				preference(c, a.t("Theme", "主题模式"), a.t("Follow Windows or pick your preferred appearance.", "跟随系统，或选择你喜欢的外观。"), func() {
					if a.themeSource != a.theme {
						a.themeSource = a.theme
						a.themeChoice = 0
						switch a.theme {
						case "light":
							a.themeChoice = 1
						case "dark":
							a.themeChoice = 2
						}
					}
					ui.Segmented(c, &a.themeChoice, a.t("System", "跟随系统"), a.t("Light", "浅色"), a.t("Dark", "深色")).OnChange(func() {
						theme := []string{"system", "light", "dark"}[a.themeChoice]
						a.theme = theme
						applyTheme(theme)
						a.saveConfig(func(c *core.Config) { c.AppTheme = theme })
					})
				})
			})
			panel(c).Children(func() {
				section(c, icons["power"], a.t("sing-box kernel", "sing-box 内核"), a.t("The engine powering your connection.", "为网络连接提供支持的核心引擎。"))
				ui.Divider(c)
				preference(c, a.t("Kernel version", "内核版本"), func() string {
					version := strings.Split(a.snapshot.Version, "\n")[0]
					if !strings.HasPrefix(version, "sing-box") {
						return a.t("Not installed yet", "尚未安装内核")
					}
					return version
				}(), func() {
					ui.Row(c).Gap(8).Children(func() {
						ui.Button(c, a.t("Import kernel", "导入内核")).OnClick(a.importKernel)
						iconButton(c, icons["refresh"], a.t("Download / update", "下载 / 更新")).OnClick(func() { a.run(func() (string, error) { return "", a.manager.Download(a.ctx) }, nil) })
					})
				})
				if a.busy && a.progress != "" {
					muted(c, a.progress)
				}
			})
			panel(c).Children(func() {
				section(c, icons["file"], a.t("Data management", "数据管理"), a.t("Keep a backup or bring your existing configuration.", "备份你的配置，或从旧版导入数据。"))
				ui.Divider(c)
				preference(c, a.t("Profiles & templates", "订阅与模板"), a.t("Compatible with NSB portable data v1 archives.", "兼容 NSB portable data v1 备份文件。"), func() {
					ui.Row(c).Gap(8).Children(func() {
						ui.Button(c, a.t("Import archive", "导入备份")).OnClick(a.importArchive)
						ui.Button(c, a.t("Export archive", "导出备份")).OnClick(a.exportArchive)
					})
				})
			})
			ui.Row(c).FillWidth().Padding(0, 4).Children(func() {
				muted(c, "Not Sing-Box  ·  "+appVersion()).Grow(1)
				ui.Button(c, a.t("Quit NSB", "退出 NSB")).FontSize(12).OnClick(mygo.App.Quit)
			})
		})
	})
}
func applyTheme(theme string) {
	switch theme {
	case "light":
		mygo.Theme.SetSource(mygo.ThemeLight)
	case "dark":
		mygo.Theme.SetSource(mygo.ThemeDark)
	default:
		mygo.Theme.SetSource(mygo.ThemeSystem)
	}
}
func (a *App) exportArchive() {
	path, e := mygo.Dialog.Save(mygo.SaveDialogOptions{Parent: a.window, Title: a.t("Export NSB data", "导出 NSB 数据"), DefaultPath: "nsb-backup.json"})
	if e != nil {
		a.status = e.Error()
		return
	}
	if path == "" {
		return
	}
	a.run(func() (string, error) {
		return "", core.AtomicWrite(path, []byte(core.Pretty(core.Export(a.manager.Config()))))
	}, nil)
}
func (a *App) importArchive() {
	paths, e := mygo.Dialog.Open(mygo.OpenDialogOptions{Parent: a.window, Title: a.t("Import NSB data", "导入 NSB 数据")})
	if e != nil {
		a.status = e.Error()
		return
	}
	if len(paths) == 0 {
		return
	}
	path := paths[0]
	a.run(func() (string, error) {
		info, e := os.Stat(path)
		if e != nil {
			return "", e
		}
		if info.Size() > 32<<20 {
			return "", fmt.Errorf("archive exceeds 32 MiB")
		}
		b, e := os.ReadFile(path)
		if e != nil {
			return "", e
		}
		var report core.ImportReport
		e = a.manager.Edit(func(current *core.Config) error {
			cfg, r, err := core.Import(b, *current)
			if err != nil {
				return err
			}
			*current = cfg
			report = r
			return nil
		})
		if e != nil {
			return "", e
		}
		return fmt.Sprintf("Added: %d · Updated: %d · Skipped: %d\n%v", report.Added, report.Overwritten, report.Skipped, report.Issues), nil
	}, func(message string) { a.status = message })
}

func (a *App) importKernel() {
	paths, err := mygo.Dialog.Open(mygo.OpenDialogOptions{Parent: a.window, Title: a.t("Select sing-box executable or archive", "选择 sing-box 可执行文件或压缩包")})
	if err != nil {
		a.status = err.Error()
		return
	}
	if len(paths) == 0 {
		return
	}
	path := paths[0]
	a.run(func() (string, error) { return "", a.manager.ImportKernel(a.ctx, path) }, nil)
}
