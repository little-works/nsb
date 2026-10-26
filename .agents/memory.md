# NSB 项目记忆

## 架构与优先级

- 桌面端使用 Go / MyGo native desktop，MyGo 锁定 v0.6.1。
- 项目按 `gui/core/server` 分层：`gui/app` 实现原生页面，`gui/desktop` 管理内核与桌面服务，`core` 负责订阅解析、组合及存储；`server` 是独立 HTTP 服务入口，目前提供健康检查骨架，职责定位为云端组合订阅计算。
- Desktop 直接调用本地服务，通过 HTTP client 访问 sing-box Clash API。
- 当前处于复刻阶段，聚焦 Windows、页面还原和主要功能。优先 MyGo 自带组件，保持清晰、精简的代码结构；验证范围与改动匹配。
- Windows 程序和 setup 均通过 MyGo build 产出。

## 界面约定

- 左侧 Sidebar；常规内容区居中，最大宽度 1320px，左右内边距 28px。搜索框限制宽度。
- 页面滚动条贴应用内容区右边缘：外层 Scroll 全宽，限宽和 padding 放在它内部的内容容器；嵌套表格、代码编辑器等内部滚动除外。
- 代理页 header 限宽居中，下面分组/节点列表随窗口宽度增加列数。所有 proxy card 等高等宽，固定高度 76px，内容紧凑排列。
- 代理分组顺序：PROXY、当前订阅对应的来源组、其他组、GLOBAL。
- 代理初始化“准备连接网络”空态容器也要左右留白、居中限宽；底部消息提示栏左右 28px、底部 16px 留白。
- Segmented 控件按内容定宽。
- 订阅使用以名称和信息为主的紧凑卡片/列表。
- 日志及连接页面内容限宽。日志整行按 level 着色，字号 14px，最新记录在前；level 使用 Checkbox 多选 TRACE/DEBUG/INFO/WARN/ERROR/FATAL，默认全选。
- 设置支持导入内核；左下角提供普通次级样式的“重启内核”按钮。

## 验证与打包

- 使用稳定的 MyGo keys，Element 生命周期限定于当前 build，持久状态保存数据、控件状态或 Handle；后台结果通过 Window.Update 发布。
- 按改动运行针对性检查，原生 UI 检查包括相关 GUI 测试及 `go tool mygo vet .`。
- Windows 打包：`go tool mygo build -platform windows/amd64 -o dist/<名称> .`。
- 产物位于 `dist/<名称>/windows-amd64/`，包含 `NSB.exe` 和 `NSB Setup <版本> amd64.exe`。每次试用打包使用新目录，保持用户当前实例运行；切换单实例程序前由用户从托盘退出旧实例。
