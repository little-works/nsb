# Not Sing-Box

Go + MyGo native 的 sing-box 桌面管理器，当前聚焦 Windows 复刻。项目按 `gui / core / server` 分层，桌面直接调用共享业务层，通过 HTTP 客户端访问 sing-box 的 Clash API。

## 开发

开发环境需要 Go 1.27.1 或更高版本。

```sh
go tool mygo dev .
go test ./...
go tool mygo vet .
```

`mise run dev` 执行 `go tool mygo dev .`，代码变化后自动重编译并重启应用。开发模式通过 `mygo.IsDev()` 判断，使用 `gui/dev-config` 和 `gui/dev-data`；生产构建读取 `~/.config/nsb/app-config.json` 和 `~/.nsb`。可通过 `--dev` 覆盖开发模式，或通过 `--config-dir`、`--data-dir` 指定隔离目录。

应用包含代理、订阅、模板、日志、连接、设置六个页面，支持内核下载与本地导入、测速、托盘、自启动、系统代理和定时更新。关闭窗口隐藏到托盘，通过托盘退出。代码编辑使用 MyGo 原生代码编辑器，支持语法高亮、行号和查找。系统代理配置支持 Windows。

## 代码结构

- `core`：独立的共享业务层，负责订阅模型、协议解析、字段保留与引用重写、组合生成、JavaScript hook、下载、调度、存储和数据归档。
- `gui/app`：原生页面、表单状态、交互协调与后台结果发布。
- `gui/desktop`：内核进程、Clash API、系统代理、内核安装和桌面业务协调。
- `gui/cmd/nsb`：桌面入口。
- `server`：独立 HTTP 服务和入口，面向云端组合订阅计算，目前提供健康检查骨架。

## 数据与配置

数据导入导出使用 portable data v1 格式，可在设置页面操作。首次改写已有配置前保存 `.pre-go.bak`，文件通过原子替换写入；读取损坏配置时显示错误并保留原文件。

导入按名称覆盖匹配条目并保留已有 ID；无效条目跳过并显示报告。订阅支持 Clash、sing-box JSONC 和 VLESS / Hysteria2 / AnyTLS / TUIC 分享链接。多来源按编辑器顺序组合，可上移、下移来源。

hook 使用 `onGenerate({singbox, remote/remotes})`、`onFinalize({singbox})` 的同步 JavaScript 约定，由纯 Go 引擎执行，单次执行最多 5 秒，可随操作取消。Cron 支持秒级六/七字段格式、Sunday=1、可选年份，以及日与星期同时匹配的语义。

## 构建

Windows 程序和 setup 安装程序由 MyGo 一次构建产出：

```powershell
go tool mygo build -platform windows/amd64 .
```

产物位于 `dist/windows-amd64/`，包括 `NSB.exe` 和 `NSB Setup <version> amd64.exe`。MyGo 自动准备打包工具。版本、图标和程序标识统一在 `mygo.json` 配置。

当前以 Windows 为验收目标，优先完善主要功能与页面体验。

## HTTP server

```sh
go run ./server/cmd/nsb-server --listen :8080
curl http://127.0.0.1:8080/healthz
# 云端独立构建示例
GOOS=linux GOARCH=amd64 CGO_ENABLED=0 go build -o dist/nsb-server ./server/cmd/nsb-server
```

server 作为独立进程运行，当前提供健康检查、超时和优雅退出。

## 开发检查

```sh
go test ./...
go tool mygo vet .
```

订阅核心使用输入输出对照测试，原生页面包含跨帧交互检查。验证范围与改动匹配，优先保障 Windows 主要功能和页面体验。
