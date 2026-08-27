# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-08-27

### Added
- **OAuth browser sign-in**: a real `Sign in with ChatGPT` button in the
  popover. The app spawns a localhost callback server on port 1455, opens
  the user's default browser, captures the `code` + `state`, exchanges
  them with `https://auth.openai.com/oauth/token` using PKCE (S256), and
  writes the resulting `access_token` / `refresh_token` to the
  multi-account library. Account id is derived from the JWT `sub` claim,
  email + plan from the same JWT.
- **API-equivalent cost** card in the popover. Scans
  `%USERPROFILE%\.codex\sessions\**\*.jsonl`, classifies each
  `task_complete` by model, multiplies through a frozen OpenAI price book
  (gpt-5 / gpt-5-mini / gpt-5-nano / gpt-4.1 / gpt-4o / o1 / o3 / o3-mini
  / o4-mini / codex-mini-latest + the long-context tier for prompts
  ≥ 200K tokens), and surfaces a per-model and total USD estimate. Unknown
  models are flagged but never priced — we never invent numbers.
- **Dynamic tray icon** drawn in Rust: a 32x32 RGBA PNG with a circular
  progress ring + centered percent number, recolored by 70% / 90%
  thresholds. Tooltip mirrors the percent. Refreshed automatically after
  every `fetch_quota`.

### Changed
- `fetch_quota` now also updates the tray icon (and tooltip) before
  returning to the UI. Failures in the tray pipeline are logged but
  never propagated, so quota refreshes remain robust.
- The setup hook now pre-renders a neutral 0% tray icon so the user sees
  our icon from the moment the app launches, even before the first quota
  fetch.

### Fixed
- Classify helper: model ids with date suffixes (`gpt-5-2025-08-07`) and
  dot variants (`gpt-4.1`) are normalized to the canonical price table.

## [0.1.0] - 2026-08-27

### Added
- Initial Windows port of [Licoy/codex-runway](https://github.com/Licoy/codex-runway).
- Tauri 2.11 (Rust) + WebView2 + React 18 + Vite 5 + TypeScript.
- Multi-account library under `%USERPROFILE%\.codex-runway\accounts\`
  with per-account `auth.json`, an `index.json` that never holds tokens,
  and a `D:` (current user only) ACL on the accounts directory.
- Atomic file writes (`.tmp-uuid` + rename) for `auth.json` everywhere.
- `official_auth_object()` strips `plan_type` / `auth_file_plan_type`
  before writing the official `~/.codex/auth.json`, so the Codex CLI
  file watcher does not reject it.
- Invalid / placeholder credentials are never written back to the official
  `auth.json`.
- Quota API client: `wham/usage`, `wham/rate-limit-reset-credits`,
  `wham/profiles/me` against `https://chatgpt.com/backend-api`.
- Local Codex session log scanner: reads
  `%USERPROFILE%\.codex\sessions\**\*.jsonl` for `task_complete` events
  and aggregates per-model token usage.
- Popover UI with light + dark theme, keyboard-friendly buttons, and a
  react-style import dialog (paste `auth.json`).
- GitHub Actions CI (Ubuntu runner) and release (Windows runner, MSI +
  NSIS).
