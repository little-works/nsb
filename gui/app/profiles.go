package app

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/egoist/mygo/ui"
	"nsb/core"
)

type remoteEditor struct {
	ID            string
	Remote        core.Remote
	Headers, Keep string
	Advanced      bool
}
type profileEditor struct {
	Profile                            core.Profile
	TemplateName, Hook, Interval, Cron string
	Remotes                            []*remoteEditor
	HookOpen                           bool
}

func newProfileEditor(p core.Profile, c core.Config) *profileEditor {
	e := &profileEditor{Profile: core.Clone(p)}
	for _, t := range c.Templates {
		if t.ID == p.TemplateID {
			e.TemplateName = t.Name
		}
	}
	if p.Hook != nil {
		e.Hook = *p.Hook
	}
	if p.UpdateCron != nil {
		e.Cron = *p.UpdateCron
	}
	if p.UpdateIntervalHours != nil {
		e.Interval = strconv.Itoa(int(*p.UpdateIntervalHours))
	}
	for _, r := range p.Remotes {
		e.addRemote(r)
	}
	e.HookOpen = strings.TrimSpace(e.Hook) != ""
	return e
}
func (e *profileEditor) addRemote(r core.Remote) {
	e.Remotes = append(e.Remotes, &remoteEditor{ID: core.NewID(), Remote: core.Clone(r), Headers: core.Pretty(r.Headers), Keep: core.Pretty(r.Keep)})
}
func (e *profileEditor) value(c core.Config) (core.Profile, error) {
	p := core.Clone(e.Profile)
	p.Name = strings.TrimSpace(p.Name)
	p.TemplateID = ""
	for _, t := range c.Templates {
		if t.Name == e.TemplateName {
			p.TemplateID = t.ID
		}
	}
	p.Hook = nil
	if strings.TrimSpace(e.Hook) != "" {
		h := e.Hook
		p.Hook = &h
	}
	p.UpdateCron = nil
	if strings.TrimSpace(e.Cron) != "" {
		v := e.Cron
		p.UpdateCron = &v
	}
	p.UpdateIntervalHours = nil
	if strings.TrimSpace(e.Interval) != "" {
		n, err := strconv.ParseUint(e.Interval, 10, 32)
		if err != nil || n == 0 {
			return p, fmt.Errorf("update interval must be a positive number of hours")
		}
		v := uint32(n)
		p.UpdateIntervalHours = &v
	}
	p.Remotes = []core.Remote{}
	seen := map[string]bool{}
	for _, r := range e.Remotes {
		v := core.Clone(r.Remote)
		v.Name = strings.TrimSpace(v.Name)
		v.URL = strings.TrimSpace(v.URL)
		if v.Name == "" || v.URL == "" || seen[v.Name] {
			return p, fmt.Errorf("source names and URLs are required; names must be unique")
		}
		seen[v.Name] = true
		if err := json.Unmarshal([]byte(r.Headers), &v.Headers); err != nil {
			return p, fmt.Errorf("headers: %w", err)
		}
		if err := json.Unmarshal([]byte(r.Keep), &v.Keep); err != nil || v.Keep == nil {
			return p, fmt.Errorf("keep fields must be a JSON object")
		}
		p.Remotes = append(p.Remotes, v)
	}
	return p, core.ValidateSchedule(p)
}
func field(c *ui.Context, label string, value *string) {
	ui.Text(c, label).FontSize(13).FontWeight(500)
	ui.TextInput(c, value).FillWidth().Height(36).Label(label)
}
func (a *App) profilesView(c *ui.Context) {
	if a.runtimeID != "" {
		a.runtimeView(c)
		return
	}
	if a.editor != nil {
		a.profileEditorView(c)
		return
	}
	ui.Column(c).Fill().Gap(16).Children(func() {
		pageToolbar(c).Children(func() {
			ui.SearchField(c, &a.profileFilter).Label(a.t("Search subscriptions", "搜索订阅")).Width(360).MaxWidth(420).Height(36)
			ui.Spacer(c)
			ui.PrimaryButton(c, a.t("New profile", "新建订阅")).Height(36).OnClick(func() {
				p := core.Profile{ID: core.NewID(), Name: a.t("New profile", "新订阅"), Remotes: []core.Remote{}}
				if len(a.snapshot.Config.Templates) > 0 {
					p.TemplateID = a.snapshot.Config.Templates[0].ID
				} else {
					v := core.DefaultTemplate
					p.InlineTemplate = &v
				}
				a.editor = newProfileEditor(p, a.snapshot.Config)
			})
			ui.Button(c, a.t("Update all", "更新全部")).Height(36).OnClick(func() {
				a.run(func() (string, error) {
					for _, p := range a.manager.Config().Profiles {
						if _, e := a.manager.Generate(a.ctx, p.ID, true); e != nil {
							return "", e
						}
					}
					return "", nil
				}, nil)
			})
		})
		pageScroll(c).Children(func() {
			pageContent(c).Gap(12).Children(func() {
				if len(a.snapshot.Config.Profiles) == 0 {
					panel(c).Children(func() {
						emptyState(c, icons["profiles"], a.t("No profiles", "暂无订阅"), a.t("Create a profile or import an archive in Settings.", "新建订阅，或在设置中导入备份。"))
					})
				}
				for _, p := range a.snapshot.Config.Profiles {
					if a.profileFilter != "" && !strings.Contains(strings.ToLower(p.Name), strings.ToLower(a.profileFilter)) {
						continue
					}
					c.Key(p.ID)
					active := a.snapshot.Config.CurrentProfileID != nil && *a.snapshot.Config.CurrentProfileID == p.ID
					border := c.Theme().Border
					if active {
						border = c.Theme().Accent
					}
					panel(c).Padding(16, 18).Gap(8).BorderColor(border).Children(func() {
						ui.Row(c).Gap(16).AlignItems(ui.Center).Children(func() {
							ui.Column(c).Grow(1).MinWidth(0).Gap(5).Children(func() {
								ui.Row(c).Gap(8).Children(func() {
									ui.Text(c, p.Name).FontWeight(600).FontSize(15)
									if active {
										ui.Badge(c, a.t("Active", "当前使用")).Background(c.Theme().SurfacePressed).TextColor(c.Theme().Accent)
									}
								})
								info := fmt.Sprintf(a.t("%d sources", "%d 个来源"), len(p.Remotes))
								if p.UpdatedAt > 0 {
									info += " · " + time.Unix(int64(p.UpdatedAt), 0).Format("01-02 15:04")
								}
								muted(c, info)
							})
							a.profileActions(c, p, active)
						})
						if p.LastUpdateError != nil {
							ui.Text(c, *p.LastUpdateError).FontSize(12).TextColor(c.Theme().Danger).Wrap()
						}
					})
				}
			})
		})
	})
}
func (a *App) profileActions(c *ui.Context, p core.Profile, active bool) {
	ui.Row(c).Gap(6).Children(func() {
		textIconButton(c, icons["check"], a.t("Use", "使用")).Disabled(active).OnClick(func() { a.run(func() (string, error) { return "", a.manager.Activate(a.ctx, p.ID) }, nil) })
		textIconButton(c, icons["refresh"], a.t("Update", "更新")).OnClick(func() { a.run(func() (string, error) { return a.manager.Generate(a.ctx, p.ID, true) }, nil) })
		textIconButton(c, icons["edit"], a.t("Edit", "编辑")).OnClick(func() { a.editor = newProfileEditor(p, a.snapshot.Config) })
		textIconButton(c, icons["code"], a.t("Generated config", "生成配置")).OnClick(func() {
			a.run(func() (string, error) { return a.manager.GeneratedConfig(a.ctx, p.ID) }, func(body string) { a.runtimeID = p.ID; a.runtimeText = body })
		})
		textIconButton(c, icons["delete"], a.t("Delete", "删除")).OnClick(func() {
			a.confirm(a.t("Delete profile ", "删除订阅 ")+p.Name+"?", func() {
				if a.snapshot.Running && active {
					a.status = a.t("Switch to another profile before deleting the active profile.", "请先切换到其他订阅，再删除当前使用的订阅。")
					return
				}
				a.saveConfig(func(cfg *core.Config) {
					for i, v := range cfg.Profiles {
						if v.ID == p.ID {
							cfg.Profiles = append(cfg.Profiles[:i], cfg.Profiles[i+1:]...)
							break
						}
					}
					if cfg.CurrentProfileID != nil && *cfg.CurrentProfileID == p.ID {
						cfg.CurrentProfileID = nil
					}
				})
			})
		})
	})
}
func (a *App) profileEditorView(c *ui.Context) {
	e := a.editor
	ui.Column(c).Fill().Gap(10).Children(func() {
		pageToolbar(c).Gap(8).Children(func() {
			ui.PrimaryButton(c, a.t("Save", "保存")).OnClick(func() {
				p, err := e.value(a.snapshot.Config)
				if err != nil {
					a.status = err.Error()
					return
				}
				a.run(func() (string, error) {
					return "", a.manager.Edit(func(cfg *core.Config) error {
						found := false
						for i, v := range cfg.Profiles {
							if v.ID == p.ID {
								p.Revision = v.Revision + 1
								p.UpdatedAt = v.UpdatedAt
								p.NextUpdateAt = v.NextUpdateAt
								p.LastAttemptAt = v.LastAttemptAt
								p.LastUpdateError = v.LastUpdateError
								cfg.Profiles[i] = p
								found = true
								break
							}
						}
						if !found {
							cfg.Profiles = append(cfg.Profiles, p)
						}
						return nil
					})
				}, func(string) { a.editor = nil })
			})
			ui.Button(c, a.t("Cancel", "取消")).OnClick(func() { a.editor = nil })
		})
		pageScroll(c).Children(func() {
			pageContent(c).Children(func() {
				panel(c).Gap(12).Children(func() {
					section(c, icons["profiles"], a.t("Profile configuration", "订阅配置"))
					ui.Divider(c)
					ui.Row(c).FillWidth().Gap(16).AlignItems(ui.End).Children(func() {
						ui.Column(c).Grow(1).MinWidth(0).MaxWidth(320).Gap(6).Children(func() {
							field(c, a.t("Name", "名称"), &e.Profile.Name)
						})
						if e.Profile.InlineTemplate == nil {
							ui.Column(c).Grow(1).MinWidth(0).Gap(6).Children(func() {
								ui.Text(c, a.t("Template", "模板")).FontSize(13).FontWeight(500)
								options := []string{}
								for _, t := range a.snapshot.Config.Templates {
									options = append(options, t.Name)
								}
								ui.Row(c).FillWidth().Gap(8).AlignItems(ui.Center).Children(func() {
									ui.Select(c, &e.TemplateName, options).Grow(1).MinWidth(0).Height(36).Label(a.t("Template", "模板"))
									ui.Button(c, a.t("Use inline template", "使用内联模板")).OnClick(func() { v := core.DefaultTemplate; e.Profile.InlineTemplate = &v })
								})
							})
						}
					})
					if e.Profile.InlineTemplate != nil {
						ui.Text(c, a.t("Inline sing-box template (JSON)", "内联 sing-box 模板（JSON）"))
						a.codeEditor(c, "inline-template", e.Profile.InlineTemplate, "json").Height(200).FillWidth()
						if len(a.snapshot.Config.Templates) > 0 {
							ui.Button(c, a.t("Use a saved template", "改用已有模板")).OnClick(func() { e.Profile.InlineTemplate = nil; e.TemplateName = a.snapshot.Config.Templates[0].Name })
						}
					}
					ui.Divider(c)
					ui.Row(c).FillWidth().Gap(16).Children(func() {
						ui.Column(c).Width(180).Gap(6).Children(func() {
							field(c, a.t("Update interval (hours)", "更新间隔（小时）"), &e.Interval)
						})
						ui.Column(c).Grow(1).MinWidth(0).MaxWidth(420).Gap(6).Children(func() {
							field(c, "Cron", &e.Cron)
						})
					})
					muted(c, a.t("Blank disables updates. Cron: second minute hour day month weekday; overrides interval.", "留空禁用自动更新。Cron：秒 分 时 日 月 星期，优先于间隔。")).Wrap()
					ui.Divider(c)
					section(c, icons["connections"], a.t("Subscription sources", "订阅来源"))
					muted(c, a.t("Combined in the order shown.", "按下方顺序组合。"))
					for _, r := range e.Remotes {
						c.Key(r.ID)
						panel(c).Padding(16).Gap(10).Background(c.Theme().Background).Children(func() {
							ui.Row(c).FillWidth().Gap(16).Children(func() {
								ui.Column(c).Grow(1).MinWidth(0).MaxWidth(320).Gap(6).Children(func() {
									field(c, a.t("Source name", "来源名称"), &r.Remote.Name)
								})
								ui.Column(c).Width(140).Gap(6).Children(func() {
									ui.Text(c, a.t("Format", "格式")).FontSize(13).FontWeight(500)
									ui.Select(c, &r.Remote.Format, []string{"clash", "singbox", "links"}).Height(36).FillWidth().Label(a.t("Format", "格式"))
								})
							})
							ui.Row(c).FillWidth().Gap(12).AlignItems(ui.Center).Children(func() {
								ui.Text(c, "URL").FontSize(13).FontWeight(500)
								ui.TextInput(c, &r.Remote.URL).Grow(1).MinWidth(0).Height(36).Label("URL")
							})
							ui.Collapsible(c, a.t("Advanced options", "高级选项"), &r.Advanced, func() {
								ui.Column(c).FillWidth().Gap(10).Children(func() {
									muted(c, a.t("Headers (JSON array of key/value)", "请求头（key/value JSON 数组）"))
									a.codeEditor(c, r.ID+"-headers", &r.Headers, "json").Height(100).FillWidth()
									muted(c, a.t("Keep fields (JSON; nested flags supported)", "保留字段（JSON，支持嵌套开关）"))
									a.codeEditor(c, r.ID+"-keep", &r.Keep, "json").Height(150).FillWidth()
								})
							}).FillWidth()
							ui.Row(c).Gap(8).Children(func() {
								ui.Button(c, a.t("Move up", "上移")).OnClick(func() {
									for i, v := range e.Remotes {
										if v.ID == r.ID && i > 0 {
											e.Remotes[i], e.Remotes[i-1] = e.Remotes[i-1], e.Remotes[i]
											break
										}
									}
								})
								ui.Button(c, a.t("Move down", "下移")).OnClick(func() {
									for i, v := range e.Remotes {
										if v.ID == r.ID && i+1 < len(e.Remotes) {
											e.Remotes[i], e.Remotes[i+1] = e.Remotes[i+1], e.Remotes[i]
											break
										}
									}
								})
								ui.Spacer(c)
								ui.Button(c, a.t("Remove source", "移除来源")).OnClick(func() {
									for i, v := range e.Remotes {
										if v.ID == r.ID {
											e.Remotes = append(e.Remotes[:i], e.Remotes[i+1:]...)
											break
										}
									}
								})
							})
						})
					}
					ui.Button(c, a.t("Add source", "添加来源")).OnClick(func() {
						e.addRemote(core.Remote{Name: fmt.Sprintf("source-%d", len(e.Remotes)+1), Format: "clash", Keep: core.DefaultKeep(), Headers: []core.Header{}})
					})
					ui.Divider(c)
					ui.Collapsible(c, a.t("JavaScript hooks", "JavaScript 钩子"), &e.HookOpen, func() {
						ui.Column(c).FillWidth().Gap(10).Children(func() {
							muted(c, "onGenerate({singbox, remote/remotes}) · onFinalize({singbox})")
							a.codeEditor(c, "profile-hook", &e.Hook, "js").Height(230).FillWidth()
						})
					}).FillWidth()
				})

			})
		})
	})
}
func (a *App) runtimeView(c *ui.Context) {
	pageContent(c).FillHeight().Gap(10).Children(func() {
		ui.Row(c).Gap(8).Children(func() {
			ui.Button(c, a.t("Back", "返回")).OnClick(func() { a.runtimeID = "" })
			ui.Button(c, a.t("Regenerate", "重新生成")).OnClick(func() {
				id := a.runtimeID
				a.run(func() (string, error) { return a.manager.Generate(a.ctx, id, false) }, func(body string) { a.runtimeText = body })
			})
			ui.PrimaryButton(c, a.t("Save config", "保存配置")).OnClick(func() {
				id, body := a.runtimeID, a.runtimeText
				a.run(func() (string, error) { return "", a.manager.SaveRuntime(a.ctx, id, body) }, nil)
			})
		})
		a.codeEditor(c, "runtime", &a.runtimeText, "json").Fill()
	})
}
