# Codex Runway — Windows port

This is the Windows port of [Licoy/codex-runway](https://github.com/Licoy/codex-runway),
focused on the Codex panel only (no Grok, no desktop widgets). It is a fresh
codebase that shares data shapes and the privacy contract with the macOS
original, not the Swift sources.

## Stack

- Tauri 2.11 (Rust + WebView2 on Windows)
- React 18 + Vite 5 + TypeScript
- `reqwest` for the official Codex backend API
- `walkdir` for scanning local Codex session logs

## Where things live

- `src-tauri/src/paths.rs` — `~/.codex` / `~/.codex-runway` discovery,
  atomic file writes, owner-only ACLs on Windows.
- `src-tauri/src/auth.rs` — `CodexAuth` model + `login_usability()`.
- `src-tauri/src/account.rs` — multi-account index + per-account
  credential storage.
- `src-tauri/src/quota.rs` — official `chatgpt.com/backend-api` client
  (wham/usage, wham/rate-limit-reset-credits, wham/profiles/me).
- `src-tauri/src/session.rs` — local Codex session JSONL scanner.
- `src-tauri/src/lib.rs` — Tauri command surface + tray + window glue.
- `web/src/App.tsx` — popover root.
- `web/src/components/` — `AccountsCard`, `QuotaCard`, `ResetCreditsCard`,
  `UsageCard`, `AddAccountDialog`.

## Commands

- `pnpm tauri dev` — run with hot reload (uses the `*.dev` identifier).
- `pnpm tauri build` — produce MSI + NSIS installers.
- `cargo test --manifest-path src-tauri/Cargo.toml` — run the Rust tests.
- `pnpm exec tsc -b --noEmit` — typecheck the web.

## Iconography

The repository ships placeholder PNG/ICO icons. Replace them with real
artwork before a public release. The Tauri CLI can generate the full set
from a 1024×1024 source:

```bash
pnpm tauri icon ./path/to/source.png
```

## Privacy invariants

These mirror the macOS original and should not be relaxed without an
explicit user confirmation:

1. Tokens are read from `%USERPROFILE%\.codex\auth.json`. Multi-account
   credentials are stored only under
   `%USERPROFILE%\.codex-runway\accounts\<id>\auth.json` (owner-only
   permissions on Windows; `0700`/`0600` on Unix).
2. The account index file `index.json` never contains tokens.
3. `official_auth_object()` strips our `plan_type` / `auth_file_plan_type`
   keys before writing to the official `auth.json`, so the Codex CLI does
   not reject it.
4. Invalid or mock credentials are never written back to the official
   `auth.json`.
5. The session scanner is read-only: it never writes to
   `%USERPROFILE%\.codex\`.

## Test status

- 9 Rust unit tests pass (`auth`, `paths`, `account`, `session`).
- TypeScript `tsc -b --noEmit` is clean.
- `vite build` produces a 152KB / 48KB-gzipped bundle.

## What is intentionally out of scope

- Grok quota, billing, CLI, sessions.
- Desktop widgets (Tauri does not support Win32 widget hosts).
- macOS-only affordances (NSStatusItem, sandbox widgets, keychain,
  app groups).
- A native OAuth browser sign-in flow. The MVP takes pasted `auth.json`
  or imports the official file. The `grok login --oauth`-style flow is a
  follow-up.
