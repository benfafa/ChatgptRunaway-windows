# Codex Runway for Windows

> Windows 原生托盘应用：Codex 额度、reset credits、本机会话、多账号管理。
> 这是 [Licoy/codex-runway](https://github.com/Licoy/codex-runway) 的
> Windows 移植版，聚焦 Codex 面板（不含 Grok 和桌面 widget）。

[English version](./README.md) · [更新日志](./CHANGELOG.md)

## 为什么单独搞一个仓库

macOS 原版是纯 SwiftUI + AppKit 应用，依赖 NSStatusItem、NSWorkspace、
keychain、沙盒 widget、App Group 等 macOS-only API。在 Windows 上做
1:1 移植等于换一套系统：托盘、WebView2、Windows 凭据存储、Win32
文件系统。两份代码共享数据形态和**隐私契约**，不共享实现。

## 功能

- **托盘优先**。应用常驻 Windows 系统托盘，不开主窗口。点击托盘图
  标弹出紧凑 popover。
- **仅 Codex**。`~/.codex\auth.json`、`~/.codex\sessions\`、官方
  `chatgpt.com/backend-api` 端点。Grok 不实现。
- **多账号**。增删、切换、刷新多个 Codex 账号。激活账号原子写回
  `~/.codex\auth.json`，Codex CLI / IDE 自动同步。
- **本地优先**。API 等价成本完全从本机会话 JSONL 日志算，不上传会话内容。
- **Windows 10/11**。Tauri 2 + WebView2，单 MSI/NSIS 安装包。

## 非目标

- Grok 额度、计费、CLI、session。
- 桌面 widget（Tauri 当前不支持 Win32 widget host）。
- macOS-only 特性：menubar extra、Notification Center、keychain、App
  Group。

## 隐私

延续 macOS 原版的隐私契约：

- `auth.json` 读取自 `%USERPROFILE%\.codex\auth.json`。多账号凭据只存
  在 `%USERPROFILE%\.codex-runway\accounts\<id>\auth.json`（Windows 上
  设 owner-only ACL；Unix 设 `0700`/`0600`）。
- 账号索引文件 `index.json` 绝不存 token。
- 失效或占位符凭据绝不回写到官方 `auth.json`。
- API 等价成本仅基于本地 JSONL 会话日志计算，推导数据存在
  `%USERPROFILE%\.codex-runway\`。无任何上传。
- 升级检查仅请求版本信息。

## 系统要求

- Windows 10 (1809) 或 Windows 11，**需 WebView2 Runtime**。
  Windows 11 自带，Windows 10 一般需要安装。安装器在首次启动时会
  提示下载。
- 建议先装好 Codex CLI / IDE 至少用过一次（有 `auth.json` 才能立即
  显示数据）。
- 已存在 `%USERPROFILE%\.codex\auth.json`，或直接在 app 里添加账号
  （粘贴、扫码登录）。

## 快速开始

```cmd
pnpm install
pnpm tauri dev
```

`pnpm tauri dev` 会构建并启动带热重载的应用。开发实例使用独立标识符
`com.github.codex-runway-windows.dev`，可与正式版共存。

## 打包正式安装器

```cmd
pnpm tauri build
```

输出：

- `src-tauri\target\release\CodexRunway.exe` — 单文件可执行
- `src-tauri\target\release\bundle\msi\CodexRunway_*.msi` — Windows
  Installer（适合企业部署）
- `src-tauri\target\release\bundle\nsis\CodexRunway_*.exe` — NSIS
  安装器（适合个人分发）

## 自检

```cmd
pnpm tauri dev -- --self-check
```

自检只读本地状态，不发网络请求。打印脱敏的 Codex 诊断信息 + 凭据
状态 + 账号身份。Token 和 API key 永远不打印。

## 数据来源

- **额度 / reset credits / 官方 token 用量**：登录后用本地凭据请求
  官方 `chatgpt.com/backend-api` 端点（`wham/usage`、
  `wham/rate-limit-reset-credits`、`wham/profiles/me`）。
- **API 等价成本 / 本地 session**：基于本地
  `%USERPROFILE%\.codex\sessions\*.jsonl` 中 `task_complete` 的 token
  用量，按 OpenAI Text API 价格表计算。未知模型不杜撰价格。

## 开发

- 后端：`src-tauri\src\`
  - `auth.rs` — `CodexAuth` 模型 + `login_usability()`
  - `account.rs` — 多账号库 + 索引
  - `quota.rs` — 官方后端 API 客户端
  - `session.rs` — 本地 Codex session JSONL 扫描
  - `oauth.rs` — Codex OAuth (PKCE) + 本地回调服务器
  - `pricing.rs` — OpenAI 文本 API 冻结价格表
  - `cost.rs` — API 等价成本计算引擎
  - `tray_icon.rs` — 动态托盘图标渲染
- 前端：`web\src\`
  - `App.tsx` — 根组件
  - `components\` — 各卡片
- 测试：`cargo test --manifest-path src-tauri\Cargo.toml`（29 个测试）

## 图标

仓库内的 `src-tauri\icons\` 是占位 PNG/ICO。发布前用真图替换：

```cmd
pnpm tauri icon .\path\to\source.png
```

## 协议

AGPL-3.0，继承上游 [Licoy/codex-runway](https://github.com/Licoy/codex-runway)。
