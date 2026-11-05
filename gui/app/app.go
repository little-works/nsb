package app

import (
	"context"
	"fmt"
	"log"
	"sync"
	"time"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
	"nsb/core"
	assets "nsb/gui"
	"nsb/gui/desktop"
)

type App struct {
	manager                       *desktop.Manager
	window                        *mygo.Window
	ctx                           context.Context
	cancel                        context.CancelFunc
	workers                       sync.WaitGroup
	snapshot                      desktop.Snapshot
	page                          string
	busy, quitting, quitReady     bool
	status                        string
	editor                        *profileEditor
	template                      *templateEditor
	runtimeID, runtimeText        string
	logFilter                     string
	connectionFilter              string
	proxy                         bool
	language                      string
	startup                       bool
	theme                         string
	progress                      string
	delays                        map[string]string
	proxyFilter                   string
	modeChoice                    int
	modeSource                    string
	connectionsTable              ui.ListState
	code                          map[string]*codeState
	themeSource, languageSource   string
	themeChoice, languageChoice   int
	profileFilter, templateFilter string
	logLevels                     logSelection
}

func appVersion() string {
	if v := mygo.App.Version(); v != "" {
		return v
	}
	return "dev"
}

func New(m *desktop.Manager) *App {
	ctx, cancel := context.WithCancel(context.Background())
	c := m.Config()
	return &App{manager: m, ctx: ctx, cancel: cancel, snapshot: desktop.Snapshot{Config: c}, page: "proxies", proxy: c.SystemProxyEnabled, language: c.AppLanguage, theme: c.AppTheme, logLevels: logSelection{true, true, true, true, true, true}, delays: map[string]string{}}
}
func (a *App) t(en, zh string) string {
	if a.language == "zh-CN" {
		return zh
	}
	return en
}
func (a *App) Run() error {
	mygo.App.SetName("NSB")
	if !mygo.App.RequestSingleInstanceLock() {
		a.cancel()
		return nil
	}
	mygo.App.OnSecondInstance(func([]string, string) {
		if a.window != nil {
			a.window.Show()
			a.window.Focus()
		}
	})
	mygo.App.WhenReady(func() {
		log.Print("NSB native UI ready")
		applyTheme(a.theme)
		mygo.App.SetLocale(a.language)
		a.startup = mygo.App.OpenAtLogin()
		a.window = mygo.NewWindow(mygo.WindowOptions{Title: "Not Sing-Box", Width: 1180, Height: 820, MinWidth: 960, MinHeight: 650, Content: ui.View(a.View)})
		tray, e := mygo.NewTray(mygo.TrayOptions{Icon: assets.Icon, ToolTip: "Not Sing-Box", Menu: mygo.NewMenu([]*mygo.MenuItem{
			{Label: "Show NSB / 显示窗口", Click: func(*mygo.MenuItem, *mygo.Window) { a.window.Show(); a.window.Focus() }},
			{Label: "Start / 启动", Click: func(*mygo.MenuItem, *mygo.Window) {
				a.run(func() (string, error) { return "", a.manager.Start(a.ctx) }, nil)
			}},
			{Label: "Stop / 停止", Click: func(*mygo.MenuItem, *mygo.Window) { a.run(func() (string, error) { return "", a.manager.Stop() }, nil) }},
			mygo.Separator(), {Role: mygo.RoleQuit},
		})})
		a.window.Show()
		if e == nil {
			tray.OnClick(func() { a.window.Show(); a.window.Focus() })
			a.window.OnClose(func(e *mygo.CloseEvent) {
				if !a.quitting {
					e.PreventDefault()
					a.window.Hide()
				}
			})
		} else {
			a.status = e.Error()
		}
		mygo.App.OnBeforeQuit(func(e *mygo.QuitEvent) {
			if a.quitReady {
				return
			}
			e.PreventDefault()
			if a.quitting {
				return
			}
			a.quitting = true
			a.busy = true
			a.status = a.t("Shutting down…", "正在退出…")
			a.cancel()
			go func() {
				a.workers.Wait()
				err := a.manager.Shutdown()
				a.window.Update(func() {
					if err != nil {
						log.Print(err)
						_, _ = mygo.Dialog.Message(mygo.MessageOptions{Parent: a.window, Type: mygo.MessageError, Message: a.t("NSB exit cleanup failed", "NSB 退出清理失败"), Detail: err.Error(), Buttons: []string{"OK"}})
					}
					a.quitReady = true
					mygo.App.Quit()
				})
			}()
		})
		a.startWorkers()
		a.run(func() (string, error) {
			a.manager.ReadVersion(a.ctx)
			c := a.manager.Config()
			if c.CurrentProfileID != nil {
				return "", a.manager.Start(a.ctx)
			}
			return "", nil
		}, nil)
	})
	mygo.App.OnWindowAllClosed(func() { mygo.App.Quit() })
	return mygo.App.Run()
}
func (a *App) run(work func() (string, error), done func(string)) {
	if a.busy || a.quitting {
		return
	}
	a.busy = true
	a.status = a.t("Working…", "处理中…")
	a.workers.Add(1)
	go func() {
		defer a.workers.Done()
		value, err := work()
		s := a.manager.Poll(a.ctx)
		a.window.Update(func() {
			if a.quitting {
				return
			}
			a.busy = false
			a.snapshot = s
			a.modeSource = ""
			a.proxy = s.Config.SystemProxyEnabled
			a.language = s.Config.AppLanguage
			mygo.App.SetLocale(a.language)
			a.theme = s.Config.AppTheme
			applyTheme(a.theme)
			if err != nil {
				a.status = err.Error()
			} else {
				a.status = a.t("Done", "已完成")
				if done != nil {
					done(value)
				}
			}
		})
	}()
}
func (a *App) startWorkers() {
	a.workers.Add(2)
	go func() {
		defer a.workers.Done()
		tick := time.NewTicker(2 * time.Second)
		defer tick.Stop()
		for {
			select {
			case <-a.ctx.Done():
				return
			case <-tick.C:
				s := a.manager.Poll(a.ctx)
				a.window.Update(func() {
					if !a.busy && !a.quitting {
						a.snapshot = s
					}
				})
			}
		}
	}()
	go func() {
		defer a.workers.Done()
		tick := time.NewTicker(time.Second)
		defer tick.Stop()
		count := 0
		for {
			select {
			case <-a.ctx.Done():
				return
			case <-tick.C:
				n, total := a.manager.Progress.Downloaded.Load(), a.manager.Progress.Total.Load()
				a.window.Update(func() {
					if total > 0 {
						a.progress = fmt.Sprintf("%.1f / %.1f MiB", float64(n)/(1<<20), float64(total)/(1<<20))
					}
				})
				count++
				if count%10 == 0 {
					if e := a.manager.Tick(a.ctx); e != nil {
						a.window.Update(func() {
							if !a.busy {
								a.status = e.Error()
							}
						})
					}
				}
			}
		}
	}()
}
func (a *App) View(c *ui.Context) {
	setAppTheme(c)
	ui.Row(c).Fill().Background(c.Theme().Background).Children(func() {
		ui.Column(c).Width(232).FillHeight().Padding(16).Gap(24).Background(c.Theme().Surface).BorderWidth(0, 1, 0, 0).BorderColor(c.Theme().Border).Children(func() {
			ui.Row(c).Padding(8, 4).Gap(12).AlignItems(ui.Center).Children(func() {
				if logo != nil {
					ui.Image(c, logo).Size(36, 36).Radius(8)
				}
				ui.Column(c).Gap(4).Children(func() { ui.Text(c, "Not Sing-Box").FontSize(16).FontWeight(600); muted(c, appVersion()) })
			})
			ui.Sidebar(c, &a.page, func() {
				for _, p := range []struct{ id, en, zh string }{{"proxies", "Proxies", "代理"}, {"profiles", "Profiles", "订阅"}, {"templates", "Templates", "模板"}, {"logs", "Logs", "日志"}, {"connections", "Connections", "连接"}, {"settings", "Settings", "设置"}} {
					ui.SidebarItem(c, p.id, icons[p.id], a.t(p.en, p.zh)).Height(42).Radius(6).Padding(10, 12).Gap(12)
				}
			}).FillWidth().Grow(1).Padding(0).Gap(5).Background(ui.Transparent).OnChange(func() { a.runtimeID = ""; a.editor = nil; a.template = nil; a.code = nil })
			panel(c).Padding(14).Gap(12).Background(c.Theme().Background).Children(func() {
				ui.Row(c).Gap(8).AlignItems(ui.Center).Children(func() {
					color := c.Theme().TextMuted
					state := a.t("Stopped", "未运行")
					if a.snapshot.Running {
						color = c.Theme().Success
						state = a.t("Running", "运行中")
					}
					ui.Box(c).Size(7, 7).Radius(4).Background(color)
					ui.Text(c, state).FontSize(13).FontWeight(600).Grow(1)
					if a.snapshot.Running {
						muted(c, a.snapshot.Uptime.String())
					}
				})
				ui.Divider(c)
				ui.Row(c).FillWidth().Children(func() {
					ui.Column(c).Grow(1).Gap(5).Children(func() {
						muted(c, a.t("Upload", "上传速率"))
						ui.Text(c, fmt.Sprintf("↑ %.1f KB/s", a.snapshot.UploadRate/1024)).FontSize(13)
					})
					ui.Column(c).Grow(1).Gap(5).Children(func() {
						muted(c, a.t("Download", "下载速率"))
						ui.Text(c, fmt.Sprintf("↓ %.1f KB/s", a.snapshot.DownloadRate/1024)).FontSize(13)
					})
				})
				iconButton(c, icons["refresh"], a.t("Restart kernel", "重启内核")).FillWidth().Height(34).Disabled(a.busy).OnClick(func() { a.run(func() (string, error) { return "", a.manager.Restart(a.ctx) }, nil) })
			})
		})
		ui.Column(c).Grow(1).MinWidth(0).FillHeight().Gap(20).Children(func() {
			header := ui.Row(c).FillWidth().MaxWidth(1320).AlignSelf(ui.Center).Padding(28, 28, 0, 28).AlignItems(ui.Center).Gap(12)
			header.Children(func() {
				ui.Text(c, a.pageTitle()).FontSize(22).FontWeight(600).Grow(1)
				if a.busy {
					ui.Spinner(c).Size(16, 16)
				}
				if id := a.snapshot.Config.CurrentProfileID; id != nil {
					for _, p := range a.snapshot.Config.Profiles {
						if p.ID == *id {
							ui.Badge(c, p.Name).Padding(6, 10).Background(c.Theme().SurfacePressed).TextColor(c.Theme().Accent)
							break
						}
					}
				}
			})
			body := ui.Column(c).Grow(1).MinHeight(0).FillWidth().Disabled(a.busy)
			body.Children(func() {
				c.Key(a.page)
				switch a.page {
				case "profiles":
					a.profilesView(c)
				case "templates":
					a.templatesView(c)
				case "logs":
					a.logsView(c)
				case "connections":
					a.connectionsView(c)
				case "settings":
					a.settingsView(c)
				default:
					a.proxiesView(c)
				}
			})
			if a.status != "" && a.status != a.t("Done", "已完成") {
				ui.Column(c).FillWidth().MaxWidth(1320).AlignSelf(ui.Center).Padding(0, 28, 16, 28).Children(func() {
					ui.Row(c).FillWidth().Padding(10, 14).Background(c.Theme().Surface).Radius(8).Gap(10).AlignItems(ui.Center).Label("Status message").Children(func() {
						muted(c, a.status).Grow(1).MinWidth(0).SingleLine()
						if a.page != "logs" && a.snapshot.Logs != "" {
							ui.Button(c, a.t("View logs", "查看日志")).OnClick(func() { a.page = "logs" })
						}
					})
				})
			}
		})
	})
}
func (a *App) pageTitle() string {
	switch a.page {
	case "profiles":
		return a.t("Profiles", "订阅管理")
	case "templates":
		return a.t("Templates", "模板管理")
	case "logs":
		return a.t("Kernel logs", "内核日志")
	case "connections":
		return a.t("Connections", "连接管理")
	case "settings":
		return a.t("Settings", "设置")
	default:
		return a.t("Proxy manager", "代理管理")
	}
}
func (a *App) confirm(message string, action func()) {
	r, e := mygo.Dialog.Message(mygo.MessageOptions{Parent: a.window, Type: mygo.MessageQuestion, Message: message, Buttons: []string{a.t("Cancel", "取消"), a.t("Delete", "删除")}, DefaultButton: 0, CancelButton: 0})
	if e != nil {
		a.status = e.Error()
		return
	}
	if r.Button == 1 {
		action()
	}
}
func (a *App) saveConfig(change func(*core.Config)) {
	a.run(func() (string, error) {
		return "", a.manager.Edit(func(c *core.Config) error { change(c); return nil })
	}, nil)
}
