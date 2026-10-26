package app

import (
	"fmt"
	"strings"
	"time"

	"github.com/egoist/mygo/ui"
	"nsb/core"
)

type templateEditor struct{ Template core.Template }

func (a *App) templatesView(c *ui.Context) {
	if a.template != nil {
		e := a.template
		pageContent(c).FillHeight().Gap(10).Children(func() {
			ui.Row(c).Gap(8).Children(func() {
				ui.PrimaryButton(c, a.t("Save", "保存")).OnClick(func() {
					t := e.Template
					t.UpdatedAt = uint64(time.Now().Unix())
					a.run(func() (string, error) {
						return "", a.manager.Edit(func(cfg *core.Config) error {
							found := false
							for i, v := range cfg.Templates {
								if v.ID == t.ID {
									cfg.Templates[i] = t
									found = true
									break
								}
							}
							if !found {
								cfg.Templates = append(cfg.Templates, t)
							}
							return nil
						})
					}, func(string) { a.template = nil })
				})
				ui.Button(c, a.t("Cancel", "取消")).OnClick(func() { a.template = nil })
			})
			field(c, a.t("Name", "名称"), &e.Template.Name)
			a.codeEditor(c, "template", &e.Template.Content, "json").Fill()
		})
		return
	}
	ui.Column(c).Fill().Gap(12).Children(func() {
		pageToolbar(c).Children(func() {
			ui.SearchField(c, &a.templateFilter).Label(a.t("Search templates…", "搜索模板…")).Width(360).MaxWidth(420).Height(35)
			ui.Spacer(c)
			ui.PrimaryButton(c, a.t("New template", "新建模板")).OnClick(func() {
				a.template = &templateEditor{Template: core.Template{ID: core.NewID(), Name: a.t("New template", "新模板"), Content: core.DefaultTemplate}}
			})
		})
		pageScroll(c).Children(func() {
			pageContent(c).Gap(14).Children(func() {
				if len(a.snapshot.Config.Templates) == 0 {
					panel(c).Children(func() {
						emptyState(c, icons["templates"], a.t("Build your first template", "创建你的第一个模板"), a.t("Share routing and DNS rules across subscriptions.", "在多个订阅之间复用路由和 DNS 规则。"))
					})
				}
				for _, t := range a.snapshot.Config.Templates {
					if a.templateFilter != "" && !strings.Contains(strings.ToLower(t.Name), strings.ToLower(a.templateFilter)) {
						continue
					}
					c.Key(t.ID)
					panel(c).Children(func() {
						ui.Row(c).Gap(12).Children(func() {
							ui.Icon(c, icons["templates"]).Size(23, 23).TextColor(c.Theme().Accent)
							ui.Column(c).Grow(1).Gap(6).Children(func() {
								ui.Text(c, t.Name).FontSize(17).FontWeight(600)
								references := 0
								for _, p := range a.snapshot.Config.Profiles {
									if p.TemplateID == t.ID {
										references++
									}
								}
								muted(c, fmt.Sprintf(a.t("%d profiles · JSON configuration", "%d 个订阅引用 · JSON 配置"), references))
							})
						})
						ui.Row(c).Gap(8).Children(func() {
							ui.Button(c, a.t("Edit", "编辑")).OnClick(func() { a.template = &templateEditor{Template: t} })
							ui.Button(c, a.t("Delete", "删除")).OnClick(func() {
								a.confirm(a.t("Delete template ", "删除模板 ")+t.Name+"?", func() {
									a.saveConfig(func(cfg *core.Config) {
										for i, v := range cfg.Templates {
											if v.ID == t.ID {
												cfg.Templates = append(cfg.Templates[:i], cfg.Templates[i+1:]...)
												break
											}
										}
									})
								})
							})
						})
					})
				}
			})
		})
	})
}
