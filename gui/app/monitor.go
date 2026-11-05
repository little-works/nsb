package app

import (
	"fmt"
	"sort"
	"strings"

	"github.com/egoist/mygo/ui"
)

func object(v any) map[string]any { o, _ := v.(map[string]any); return o }
func array(v any) []any           { o, _ := v.([]any); return o }
func text(v any) string           { s, _ := v.(string); return s }
func sortedKeys(o map[string]any) []string {
	keys := make([]string, 0, len(o))
	for k := range o {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}
func (a *App) proxiesView(c *ui.Context) {
	ui.Column(c).Fill().Gap(16).Children(func() {
		pageToolbar(c).Children(func() {
			ui.SearchField(c, &a.proxyFilter).Label(a.t("Search nodes or groups…", "搜索节点或代理组…")).Width(360).MaxWidth(420).Height(36)
			ui.Spacer(c)
			actual := text(a.snapshot.ClashConfig["mode"])
			if actual != a.modeSource {
				a.modeSource = actual
				switch actual {
				case "global":
					a.modeChoice = 1
				case "direct":
					a.modeChoice = 2
				default:
					a.modeChoice = 0
				}
			}
			ui.Segmented(c, &a.modeChoice, a.t("Rule", "规则"), a.t("Global", "全局"), a.t("Direct", "直连")).Label(a.t("Routing mode", "路由模式")).Disabled(!a.snapshot.Running).OnChange(func() {
				mode := []string{"rule", "global", "direct"}[a.modeChoice]
				a.run(func() (string, error) { return "", a.manager.Mode(a.ctx, mode) }, nil)
			})
		})
		if !a.snapshot.Running {
			pageContent(c).Grow(1).MinHeight(0).Children(func() {
				panel(c).Grow(1).Justify(ui.Center).Children(func() {
					emptyState(c, icons["proxies"], a.t("Ready when you are", "准备连接网络"), a.t("Select a profile, then click Restart kernel in the sidebar.", "在「订阅」中选择配置，然后点击左下角的「重启内核」。"))
				})
			})
			return
		}
		proxies := object(a.snapshot.Proxies["proxies"])
		width, _ := c.Size()
		columns := proxyColumns(min(width, 1320))
		query := strings.ToLower(strings.TrimSpace(a.proxyFilter))
		shown := 0
		ui.Scroll(c).Grow(1).FillWidth().Label(a.t("Proxy groups", "代理分组")).Children(func() {
			pageContent(c).Gap(12).Children(func() {
				for _, name := range a.proxyGroups(proxies) {
					group := object(proxies[name])
					all := array(group["all"])
					members := []string{}
					for _, v := range all {
						node := text(v)
						if query == "" || strings.Contains(strings.ToLower(name+" "+node), query) {
							members = append(members, node)
						}
					}
					if len(members) == 0 && query != "" && !strings.Contains(strings.ToLower(name), query) {
						continue
					}
					shown++
					c.Key(name)
					panel(c).Children(func() {
						ui.Row(c).Gap(10).AlignItems(ui.Center).Children(func() {
							ui.Icon(c, icons["connections"]).Size(20, 20).TextColor(c.Theme().TextMuted)
							ui.Text(c, name).FontSize(16).FontWeight(600)
							ui.Badge(c, fmt.Sprint(len(all)))
							ui.Spacer(c)
							muted(c, text(group["type"]))
						})
						if len(all) == 0 {
							muted(c, a.t("No available nodes in this group.", "此分组暂无可用节点。"))
						}
						ui.Grid(c).Columns(columns).Gap(10).Children(func() {
							for _, node := range members {
								selected := text(group["now"]) == node
								bg, border := c.Theme().Surface, c.Theme().Border
								if selected {
									bg = c.Theme().SurfacePressed
									border = c.Theme().Accent
								}
								c.Key(node)
								ui.ButtonBase(c).Height(76).Padding(12, 14).Radius(6).Border(1, border).Background(bg).Label(node).OnClick(func() {
									if text(group["type"]) == "Selector" {
										a.run(func() (string, error) { return "", a.manager.SelectProxy(a.ctx, name, node) }, nil)
									}
								}).Children(func() {
									ui.Column(c).FillWidth().Gap(8).Children(func() {
										ui.Row(c).Gap(8).Children(func() {
											ui.Text(c, node).FontWeight(600).Grow(1).SingleLine()
											if selected {
												ui.Icon(c, icons["check"]).Size(16, 16).TextColor(c.Theme().Accent)
											}
										})
										ui.Row(c).Gap(8).Children(func() {
											kind := text(object(proxies[node])["type"])
											if kind == "" {
												kind = a.t("Proxy", "代理节点")
											}
											muted(c, kind).Grow(1)
											delay := a.delays[node]
											if delay != "" {
												ui.Text(c, delay).FontSize(12).TextColor(c.Theme().Success)
											}
											ui.Button(c, a.t("Test", "测速")).FontSize(11).Padding(3, 8).OnClick(func() {
												a.run(func() (string, error) { ms, e := a.manager.Latency(a.ctx, node); return fmt.Sprintf("%d ms", ms), e }, func(v string) { a.delays[node] = v })
											})
										})
									})
								})
							}
						})
					})
				}
				if shown == 0 {
					emptyState(c, icons["proxies"], a.t("No matching nodes", "没有匹配的节点"), a.t("Try another search or update your subscription.", "试试其他关键词，或更新你的订阅。"))
				}
			})
		})
	})
}
func (a *App) logsView(c *ui.Context) {
	ui.Column(c).Fill().Gap(16).Children(func() {
		pageContent(c).Padding(0, 28).Gap(12).Children(func() {
			ui.Row(c).Gap(12).AlignItems(ui.Center).Children(func() {
				ui.SearchField(c, &a.logFilter).Label(a.t("Filter logs", "筛选日志")).Width(360).MaxWidth(420).Height(36)
				ui.Spacer(c)
				ui.Button(c, a.t("Clear", "清空")).Height(36).Disabled(a.snapshot.Logs == "").OnClick(func() {
					if err := a.manager.ClearLogs(); err != nil {
						a.status = err.Error()
						return
					}
					a.snapshot.Logs = ""
				})
			})
			ui.Row(c).Gap(14).Wrap().Children(func() {
				muted(c, a.t("Level", "级别"))
				for i, name := range logLevelNames {
					c.Key(name)
					ui.Checkbox(c, &a.logLevels[i], name)
				}
			})
		})
		pageContent(c).Grow(1).MinHeight(0).Children(func() {
			panel(c).Grow(1).MinHeight(0).Padding(1).Gap(0).Label("Log panel").Children(func() {
				pageScroll(c).Children(func() {
					lines := filterLogs(a.snapshot.Logs, a.logFilter, a.logLevels)
					if len(lines) == 0 {
						ui.Column(c).Grow(1).Justify(ui.Center).Children(func() {
							emptyState(c, icons["logs"], a.t("No matching logs", "暂无匹配的日志"), a.t("Change your filter or wait for kernel output.", "调整筛选条件，或等待内核输出日志。"))
						})
						return
					}
					ui.Column(c).FillWidth().Padding(16, 18).Gap(5).Selectable().Children(func() {
						for _, line := range lines {
							color := c.Theme().TextMuted
							switch line.level {
							case "INFO":
								color = c.Theme().Accent
							case "WARN":
								color = c.Theme().Warning
							case "ERROR", "FATAL":
								color = c.Theme().Danger
							}
							ui.Text(c, line.text).Font("Consolas").FontSize(14).TextColor(color).Wrap()
						}
					})
				})
			})
		})
	})
}
func (a *App) connectionsView(c *ui.Context) {
	ui.Column(c).Fill().Gap(16).Children(func() {
		all := array(a.snapshot.Connections["connections"])
		connections := []map[string]any{}
		for _, v := range all {
			conn := object(v)
			if a.connectionFilter == "" || strings.Contains(strings.ToLower(fmt.Sprint(conn)), strings.ToLower(a.connectionFilter)) {
				connections = append(connections, conn)
			}
		}
		pageToolbar(c).Children(func() {
			ui.SearchField(c, &a.connectionFilter).Label(a.t("Search host, process or proxy…", "搜索域名、进程或代理链…")).Width(360).MaxWidth(420).Height(36)
			ui.Spacer(c)
			ui.Badge(c, fmt.Sprintf(a.t("%d active", "%d 个连接"), len(all)))
			ui.Button(c, a.t("Close all", "关闭全部")).Height(36).Disabled(!a.snapshot.Running || len(all) == 0).OnClick(func() { a.run(func() (string, error) { return "", a.manager.CloseConnection(a.ctx, "") }, nil) })
		})
		pageContent(c).Grow(1).MinHeight(0).Children(func() {
			panel(c).Grow(1).MinHeight(0).Padding(1).Gap(0).Children(func() {
				if len(connections) == 0 {
					ui.Column(c).Grow(1).Justify(ui.Center).Children(func() {
						emptyState(c, icons["connections"], a.t("No active connections", "暂无活动连接"), a.t("Connections will appear here as traffic passes through sing-box.", "网络流量经过 sing-box 时，连接信息将在这里显示。"))
					})
					return
				}
				cols := []ui.TableColumn{{Title: a.t("Destination", "目标地址"), MinWidth: 170}, {Title: a.t("Process", "进程"), Width: 110}, {Title: a.t("Proxy chain", "代理链"), Width: 130}, {Title: a.t("Upload", "上传"), Width: 80}, {Title: a.t("Download", "下载"), Width: 80}, {Title: "", Width: 64}}
				a.connectionsTable.Key = func(row int) any { return connections[row]["id"] }
				ui.Table(c, &a.connectionsTable, cols, len(connections), func(row, col int) {
					conn := connections[row]
					meta := object(conn["metadata"])
					id := text(conn["id"])
					switch col {
					case 0:
						host := text(meta["host"])
						if host == "" {
							host = text(meta["destinationIP"])
						}
						ui.Column(c).Gap(2).Children(func() {
							ui.Text(c, host).FontSize(13).SingleLine()
							muted(c, text(meta["network"])+" · "+text(meta["destinationPort"]))
						})
					case 1:
						ui.Text(c, text(meta["process"])).FontSize(12).SingleLine()
					case 2:
						chains := []string{}
						for _, v := range array(conn["chains"]) {
							chains = append(chains, text(v))
						}
						ui.Text(c, strings.Join(chains, " → ")).FontSize(12).SingleLine()
					case 3:
						muted(c, bytesLabel(conn["upload"]))
					case 4:
						muted(c, bytesLabel(conn["download"]))
					case 5:
						ui.Button(c, a.t("Close", "关闭")).FontSize(11).Padding(4, 8).OnClick(func() { a.run(func() (string, error) { return "", a.manager.CloseConnection(a.ctx, id) }, nil) })
					}
				}).Fill().Label("Connections table")
			})
		})
	})
}
