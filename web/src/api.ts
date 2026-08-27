/**
 * Type-safe bindings for the Rust commands exposed by Tauri.
 *
 * Keep this file in sync with `src-tauri/src/lib.rs`. The shape returned
 * from each command is the `Serialize` shape declared on the Rust side.
 */

import { invoke } from "@tauri-apps/api/core";

export type AccountAuthMode = "oauth" | "api_key" | "unknown";

export interface AccountRow {
  id: string;
  label: string;
  email: string | null;
  subject_id: string | null;
  account_id: string | null;
  plan_type: string | null;
  auth_mode: AccountAuthMode;
  added_at: string;
  last_used_at: string | null;
  requires_reauth: boolean;
  last_error: string | null;
  workspace: string | null;
}

export interface AccountIndex {
  active_id: string | null;
  accounts: AccountRow[];
}

export interface RateWindow {
  used_percent: number;
  window_minutes: number | null;
  resets_at: string | null;
}

export interface NamedRateWindow {
  name: string;
  window: RateWindow;
}

export interface QuotaSnapshot {
  plan: string | null;
  primary: RateWindow;
  secondary: RateWindow | null;
  additional_windows: NamedRateWindow[];
  credits_balance: number | null;
  updated_at: string;
}

export interface ResetCredit {
  id: string | null;
  status: string;
  created_at: string | null;
  expires_at: string | null;
  remaining_seconds: number;
}

export interface ResetCreditsSnapshot {
  available_count: number;
  credits: ResetCredit[];
  updated_at: string;
}

export interface ModelUsage {
  model: string;
  input_tokens: number;
  cached_input_tokens: number;
  output_tokens: number;
  total_tokens: number;
}

export interface SessionTurn {
  session_id: string;
  timestamp: string;
  model: string | null;
  input_tokens: number;
  cached_input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  cwd: string | null;
}

export interface DailyUsage {
  date: string;
  input_tokens: number;
  cached_input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  turns_count: number;
}

export interface SessionItem {
  session_id: string;
  file_path: string;
  created_at: string;
  last_updated_at: string;
  turns_count: number;
  total_tokens: number;
  primary_model: string | null;
  cwd: string | null;
}

export interface UsageSummary {
  sessions_scanned: number;
  turns_scanned: number;
  total_input_tokens: number;
  total_cached_input_tokens: number;
  total_output_tokens: number;
  total_tokens: number;
  per_model: ModelUsage[];
  daily_usage?: DailyUsage[];
  sessions?: SessionItem[];
  recent: SessionTurn[];
}

export interface AppInfo {
  home_dir: string;
  codex_home: string;
  app_home: string;
  official_auth_exists: boolean;
}

export interface RedactedAuth {
  auth_mode: string | null;
  account_id: string | null;
  email: string | null;
  plan_type: string | null;
  has_access_token: boolean;
  has_refresh_token: boolean;
  usability: string;
}

export interface ApiCostSummary {
  pricing_version: string;
  window_start: string;
  window_end: string;
  turns_priced: number;
  turns_unknown: number;
  total_uncached_input_tokens: number;
  total_cached_input_tokens: number;
  total_output_tokens: number;
  total_tokens: number;
  estimated_usd: string; // Decimal serialized as string
  per_model: ApiModelCost[];
  unknown_models: string[];
}

export interface ApiModelCost {
  raw_model: string;
  classified: string;
  turns: number;
  input_tokens: number;
  cached_input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  estimated_usd: string;
  priced: boolean;
}

export interface OAuthSessionInfo {
  login_id: string;
  auth_url: string;
  port: number;
  expires_at_unix: number;
  code_verifier: string;
  state: string;
}

export interface BackupResult {
  files_copied: number;
  target_dir: string;
}

export interface RestoreResult {
  files_restored: number;
  dest_dir: string;
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  loadOfficialAuth: () => invoke<RedactedAuth | null>("load_official_auth"),

  listAccounts: () => invoke<AccountIndex>("list_accounts"),
  upsertAccount: (id: string, label: string | null, auth: unknown) =>
    invoke<AccountRow>("upsert_account", { id, label, auth }),
  deleteAccount: (id: string) => invoke<void>("delete_account", { id }),
  activateAccount: (id: string) =>
    invoke<RedactedAuth>("activate_account", { id }),
  setActiveAccount: (id: string) =>
    invoke<void>("set_active_account", { id }),

  fetchQuota: (id: string) => invoke<QuotaSnapshot>("fetch_quota", { id }),
  fetchResetCredits: (id: string) =>
    invoke<ResetCreditsSnapshot>("fetch_reset_credits", { id }),
  applyTrayIconFor: (usedPercent: number) =>
    invoke<void>("apply_tray_icon_for", { usedPercent }),
  scanLocalSessions: () => invoke<UsageSummary>("scan_local_sessions"),
  backupSessions: (targetPath: string) =>
    invoke<BackupResult>("backup_sessions", { targetPath }),
  restoreSessions: (sourcePath: string) =>
    invoke<RestoreResult>("restore_sessions", { sourcePath }),
  computeApiCost: (since?: string) =>
    invoke<ApiCostSummary>("compute_api_cost", { since: since ?? null }),

  oauthStart: () => invoke<OAuthSessionInfo>("oauth_start"),
  oauthFinish: (
    port: number,
    loginId: string,
    codeVerifier: string,
    stateParam: string,
  ) =>
    invoke<RedactedAuth>("oauth_finish", {
      port,
      loginId,
      codeVerifier,
      stateParam,
    }),

  openOfficialAuthInExplorer: () =>
    invoke<void>("open_official_auth_in_explorer"),
  openAppHomeInExplorer: () => invoke<void>("open_app_home_in_explorer"),
};
