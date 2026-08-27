# Codex Runway for Windows (中文说明)

> 🎨 **Codex Runway** 原生 Windows 托盘应用。完美复刻原版 macOS 版 [Licoy/codex-runway](https://github.com/Licoy/codex-runway) 的全部核心功能，深度适配 Windows 11 Fluent 磨砂亚克力玻璃拟态视觉体验。

---

## ✨ 核心特性

- 🖥️ **Windows 系统托盘原生集成**：常驻任务栏托盘，点击托盘图标即刻唤出高颜值 Fluent 亚克力磨砂玻璃弹窗，支持暗色/亮色自适应。
- 🟢 **剩余额度突出（Hero Quota）**：以 32px 翡翠绿发光大字与渐变能量槽优先突出展示**剩余可用额度 %**，辅以重置倒计时（精确到天、时、分）。
- 🔄 **账号订阅有效期与自动推算**：智能解析账号凭据，动态显示**订阅有效期至 YYYY/M/D · 剩余 X天Y小时**，支持自动按月推算下一续费周期。
- 🏥 **会话索引健康检测与一键修复**：实时监控本地 `%USERPROFILE%\.codex\session_index.jsonl`，比对缺失、孤立与重复项，提供一键全自动无损修复。
- 💬 **最近活跃会话与等价 API 费用折算**：智能提取各会话的自然中文标题、所属工作目录、Token 消耗，并右侧精准折算对应模型的等价美元 API 费用。
- 👥 **多账号一键切换与管理**：支持 ChatGPT 官方网页一键 OAuth 登录、支持粘贴/导入 `auth.json`。切换账号时原子化写入 `%USERPROFILE%\.codex\auth.json`，与 Codex CLI / IDE 实时同步。
- 📊 **可视化用量图表**：
  - 📅 **30 天活动热力图（GitHub 风格）**
  - 📈 **每日 Token 消耗折线趋势图**
  - 📊 **各模型消耗占比柱状图**
- 📦 **会话本地备份与还原**：支持一键将本地全量会话备份打包导出至任意指定目录，或从备份目录一键还原。
- 🔒 **隐私至上（Local-First）**：全本地运行，不会将任何 Token、凭证或会话内容上传至第三方服务器。

---

## 📥 安装包下载

最新构建的发布程序保存在 `dist_installers_latest/` 目录下：

| 安装包类型 | 说明 | 下载/运行路径 |
| :--- | :--- | :--- |
| **NSIS 极速安装程序 (推荐)** | 自动创建开始菜单与桌面快捷方式 | `dist_installers_latest/codex-runway-windows-nsis/Codex Runway_0.2.0_x64-setup.exe` |
| **MSI 原生安装包 (简体中文)** | 支持企业批量静默部署与安装管理 | `dist_installers_latest/codex-runway-windows-msi/Codex Runway_0.2.0_x64_zh-CN.msi` |
| **MSI 原生安装包 (英文)** | 英文界面标准 Windows Installer | `dist_installers_latest/codex-runway-windows-msi/Codex Runway_0.2.0_x64_en-US.msi` |
| **绿色便携单文件版** | 免安装解压即用 | `dist_installers_latest/codex-runway-windows-exe/codex-runway-windows.exe` |

---

## 🛠️ 本地开发与构建

### 系统要求

- Windows 10 (1809+) 或 Windows 11
- 已安装 WebView2 Runtime（Win11 自带）
- Node.js 20+，pnpm 9+，Rust 1.77+

### 开发调试

```bash
# 安装前端依赖
pnpm install

# 启动开发调试窗口（支持热重载）
pnpm tauri dev
```

### 构建正式安装程序

```bash
pnpm tauri build
```

---

## 📄 开源许可

本项目遵循 MIT 许可协议。
