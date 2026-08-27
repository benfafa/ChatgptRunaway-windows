# Codex Runway for Windows

> Native Windows system tray app for Codex: quota monitoring, subscription renewal tracker, reset credits, token usage charts, multi-account management, session index repair, and local session inspection.
>
> 🇨🇳 **[中文说明文档 (Chinese README)](./README_zh.md)**

---

## ✨ Key Features

- 🖥️ **Native System Tray Integration**: Lives in the Windows taskbar tray. One-click popover with Windows 11 Fluent Acrylic / Mica glassmorphic UI, supporting dynamic Light/Dark modes.
- 🟢 **Hero Quota (Remaining Focus)**: Large 32px glowing Emerald typography and gradient progress gauge highlighting remaining quota % first, paired with precise countdowns (Days, Hours, Minutes).
- 🔄 **Subscription Expiry & Cycle Projection**: Parses official JWT credentials and displays dynamic badges like `🔄 订阅有效期至 2026/9/23 · 27天9小时`, with automatic monthly renewal cycle advancement.
- 🏥 **Session Index Health & Auto-Repair**: Real-time health monitor for `%USERPROFILE%\.codex\session_index.jsonl` (Missing, Orphan, Duplicate counts) with one-click lossless rebuilding.
- 💬 **Recent Sessions & API-Equivalent Cost**: Natural Chinese titles, workspace directories, total token counts, and right-aligned USD equivalent cost estimations ($).
- 👥 **Multi-Account Manager**: ChatGPT web OAuth login and `auth.json` paste/import. Atomic sync back to `%USERPROFILE%\.codex\auth.json` keeps the official Codex CLI and IDE in sync.
- 📊 **Visual Usage Analytics**:
  - 📅 **30-Day Activity Heatmap (GitHub style)**
  - 📈 **Daily Token Consumption Trend**
  - 📊 **Per-Model Token Breakdown Bar Chart**
- 📦 **Session Backup & Restore**: One-click folder export/import of all local `.jsonl` sessions.
- 🔒 **Privacy First (Local-Only)**: Zero tokens, credentials, or session content uploaded anywhere.

---

## 📥 Download Installers

All latest builds are located under `dist_installers_latest/`:

| Installer Type | Description | Path |
| :--- | :--- | :--- |
| **NSIS Setup (Recommended)** | Standard installer with desktop/start-menu shortcuts | `dist_installers_latest/codex-runway-windows-nsis/Codex Runway_0.2.0_x64-setup.exe` |
| **MSI Package (zh-CN)** | Native Windows Installer in Simplified Chinese | `dist_installers_latest/codex-runway-windows-msi/Codex Runway_0.2.0_x64_zh-CN.msi` |
| **MSI Package (en-US)** | Native Windows Installer in English | `dist_installers_latest/codex-runway-windows-msi/Codex Runway_0.2.0_x64_en-US.msi` |
| **Portable Binary (.exe)** | Standalone green single-file executable | `dist_installers_latest/codex-runway-windows-exe/codex-runway-windows.exe` |

---

## 🛠️ Local Development

### Requirements

- Windows 10 (1809+) or Windows 11
- WebView2 Runtime installed
- Node.js 20+, pnpm 9+, Rust 1.77+

```bash
# Install dependencies
pnpm install

# Start development popover with hot reload
pnpm tauri dev

# Build release bundle
pnpm tauri build
```

---

## 📄 License

MIT License
