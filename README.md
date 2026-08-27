# Codex Runway for Windows

> Native Windows tray-bar app for Codex: quota, reset-credits, daily usage,
> multi-account management, and local session inspection. The Windows port of
> [Licoy/codex-runway](https://github.com/Licoy/codex-runway), focused on the
> Codex panel (Grok and desktop widgets are intentionally out of scope).

## Why a separate project

The macOS original is a pure SwiftUI + AppKit app that depends on macOS-only
APIs (NSStatusItem, NSWorkspace, keychain, sandbox widgets, app groups, etc.).
A faithful Windows port is a different kind of beast: system tray, WebView2,
the Windows credential store, and the Win32 filesystem. The two codebases
share the same data shapes and the same privacy contract, not the same
implementation.

## Goals

- **Tray-first.** The app lives in the Windows system tray, not a window. A
  small popover appears when you click the tray icon.
- **Codex only.** `~/.codex/auth.json`, `~/.codex/sessions/`, and the official
  `chatgpt.com/backend-api` endpoints. Grok is not implemented.
- **Multi-account.** Manage, switch, and refresh multiple Codex accounts.
  Active account is written back to `~/.codex/auth.json` atomically, so the
  Codex CLI and IDE stay in sync.
- **Local-first.** API-equivalent cost is computed from local session JSONL
  logs. No session content leaves the machine.
- **Windows 10 / 11.** Built on Tauri 2 + WebView2. Single MSI/NSIS installer.

## Non-goals

- Grok quota, billing, CLI, or sessions.
- Desktop widgets (Tauri does not currently support Win32 widget hosts).
- macOS-only affordances such as the menubar extra or Notification Center.

## Privacy

This project inherits the privacy contract of the macOS original:

- `auth.json` is read from `%USERPROFILE%\.codex\auth.json`. Multi-account
  credentials are stored under `%USERPROFILE%\.codex-runway\accounts\<id>\auth.json`
  (directory `D`, file `D:` for the current user only).
- The account index file `index.json` never contains tokens.
- Invalid or mock credentials are never written back to the official
  `auth.json`.
- "Reset today?" (when implemented) only downloads the public status feed.
- API-equivalent cost is computed locally from session JSONL logs. Derived
  data lives under `%USERPROFILE%\.codex-runway\`. Nothing is uploaded.
- Update checks request only version information.

## Requirements

- Windows 10 (1809) or Windows 11 with WebView2 Runtime installed.
  Windows 11 has WebView2 preinstalled; Windows 10 typically does not. The
  installer will offer to fetch it on first launch if missing.
- Codex CLI / IDE installed and used on this machine is recommended.
- An existing `%USERPROFILE%\.codex\auth.json`, or you can add an account
  from inside the app (paste, import, browser sign-in).

## Run locally

> Requires Node 20+, pnpm 9+, Rust 1.77+, and the Tauri 2 prerequisites for
> Windows: `Microsoft Visual C++ Build Tools` and the `WebView2` SDK.

```bash
pnpm install
pnpm tauri dev
```

`pnpm tauri dev` builds and launches the app with hot reload. The dev
instance uses a separate identifier (`com.github.codex-runway-windows.dev`)
so it can run side by side with a release install.

## Build a release installer

```bash
pnpm tauri build
```

Output:

- `src-tauri/target/release/CodexRunway.exe` — single binary.
- `src-tauri/target/release/bundle/msi/CodexRunway_*.msi` — Windows
  installer.
- `src-tauri/target/release/bundle/nsis/CodexRunway_*.exe` — NSIS
  installer.

## Self-check

```bash
pnpm tauri dev -- --self-check
```

The self-check reads local state only and makes no network request. It
prints redacted Codex diagnostics plus credential status and account
identity. Tokens and API keys are never printed.

## Data sources

- **Quota / reset credits / official token usage**: signed-in requests use
  the local credential against the official `chatgpt.com/backend-api`
  endpoints (`wham/usage`, `wham/rate-limit-reset-credits`,
  `wham/profiles/me`).
- **API-equivalent cost / local sessions**: computed from local
  `%USERPROFILE%\.codex\sessions\*.jsonl` turn-complete usage using the
  official OpenAI Text API price book. Unknown models are not invented as
  exact costs.

## License

AGPL-3.0, matching the upstream project.
