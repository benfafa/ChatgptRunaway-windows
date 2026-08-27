//! App-level types and Tauri command surface.
//!
//! Commands are intentionally thin: each one maps to a single service in
//! `auth`, `account`, `quota`, or `session`. The frontend can compose them.

pub mod account;
pub mod auth;
pub mod cost;
pub mod crashlog;
pub mod error;
pub mod oauth;
pub mod paths;
pub mod pricing;
pub mod quota;
pub mod session;
pub mod tray_icon;

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::image::Image;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::account::{AccountAuthMode, AccountIndex, AccountRow, AccountStore};
use crate::auth::CodexAuth;
use crate::error::{ApiError, AppError, AppResult};
use crate::paths::Paths;
use crate::quota::{QuotaClient, QuotaSnapshot, ResetCreditsSnapshot};
use crate::session::{SessionScanner, UsageSummary};

/// Shared state for the main window commands. The single-instance plugin
/// guarantees at most one of these exists per user.
pub struct AppState {
    pub paths: Arc<Paths>,
    pub quota: Arc<Mutex<QuotaClient>>,
}

impl AppState {
    pub fn init() -> AppResult<Self> {
        let paths = Arc::new(Paths::discover()?);
        let quota = Arc::new(Mutex::new(QuotaClient::new()?));
        Ok(Self { paths, quota })
    }
}

#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub home_dir: String,
    pub codex_home: String,
    pub app_home: String,
    pub official_auth_exists: bool,
}

#[tauri::command]
fn app_info(state: State<'_, AppState>) -> AppResult<AppInfo> {
    let p = &state.paths;
    Ok(AppInfo {
        home_dir: p.home.display().to_string(),
        codex_home: p.codex_home.display().to_string(),
        app_home: p.app_home.display().to_string(),
        official_auth_exists: p.official_auth().exists(),
    })
}

#[tauri::command]
fn load_official_auth(state: State<'_, AppState>) -> AppResult<Option<RedactedAuth>> {
    let path = state.paths.official_auth();
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read(&path)?;
    let auth: CodexAuth = serde_json::from_slice(&raw)
        .map_err(|e| AppError::InvalidCredential(e.to_string()))?;
    Ok(Some(RedactedAuth::from(&auth)))
}

#[tauri::command]
fn list_accounts(state: State<'_, AppState>) -> AppResult<AccountIndex> {
    let store = AccountStore::new(&state.paths);
    let mut idx = store.load_index()?;
    if idx.accounts.is_empty() {
        // First run: import the official `~/.codex/auth.json` if any.
        if let Some(row) = crate::account::import_official_if_absent(&state.paths)? {
            idx.accounts.push(row);
        }
        if !idx.accounts.is_empty() && idx.active_id.is_none() {
            idx.active_id = idx.accounts.first().map(|r| r.id.clone());
            store.save_index(&idx)?;
        }
    }
    Ok(idx)
}

#[tauri::command]
fn upsert_account(
    state: State<'_, AppState>,
    id: String,
    label: Option<String>,
    auth: serde_json::Value,
) -> AppResult<AccountRow> {
    let auth: CodexAuth = serde_json::from_value(auth)
        .map_err(|e| AppError::InvalidCredential(format!("auth payload: {e}")))?;
    if matches!(auth.login_usability(), crate::auth::LoginUsability::Invalid) {
        return Err(AppError::InvalidCredential(
            "refusing to store an unusable credential".to_string(),
        ));
    }
    let store = AccountStore::new(&state.paths);
    store.save_credential(&id, &auth)?;
    let mut idx = store.load_index()?;
    let now = chrono::Utc::now();
    let email = crate::account::email_from(&auth);
    let subject = crate::account::subject_id_from(&auth);
    let row = AccountRow {
        id: id.clone(),
        label: label
            .or_else(|| email.clone())
            .unwrap_or_else(|| id.clone()),
        email,
        subject_id: subject,
        account_id: auth.account_id(),
        plan_type: auth.plan_type.clone(),
        auth_mode: if auth.is_api_key() {
            AccountAuthMode::ApiKey
        } else if auth.is_oauth() {
            AccountAuthMode::Oauth
        } else {
            AccountAuthMode::Unknown
        },
        added_at: now,
        last_used_at: Some(now),
        requires_reauth: false,
        last_error: None,
        workspace: None,
    };
    if let Some(existing) = idx.accounts.iter_mut().find(|r| r.id == id) {
        *existing = row.clone();
    } else {
        idx.accounts.push(row.clone());
    }
    if idx.active_id.is_none() {
        idx.active_id = Some(id.clone());
    }
    store.save_index(&idx)?;
    Ok(row)
}

#[tauri::command]
fn delete_account(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let store = AccountStore::new(&state.paths);
    store.delete_account(&id)?;
    let mut idx = store.load_index()?;
    idx.accounts.retain(|r| r.id != id);
    if idx.active_id.as_deref() == Some(&id) {
        idx.active_id = idx.accounts.first().map(|r| r.id.clone());
    }
    store.save_index(&idx)?;
    Ok(())
}

#[tauri::command]
fn activate_account(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<RedactedAuth> {
    let store = AccountStore::new(&state.paths);
    let auth = store.activate(&id)?;
    let mut idx = store.load_index()?;
    if let Some(row) = idx.accounts.iter_mut().find(|r| r.id == id) {
        row.last_used_at = Some(chrono::Utc::now());
    }
    idx.active_id = Some(id.clone());
    store.save_index(&idx)?;
    Ok(RedactedAuth::from(&auth))
}

#[tauri::command]
fn set_active_account(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let store = AccountStore::new(&state.paths);
    let mut idx = store.load_index()?;
    if !idx.accounts.iter().any(|r| r.id == id) {
        return Err(AppError::Account(format!("unknown account id {id}")));
    }
    idx.active_id = Some(id);
    store.save_index(&idx)?;
    Ok(())
}

#[tauri::command]
async fn fetch_quota(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<QuotaSnapshot> {
    let store = AccountStore::new(&state.paths);
    let auth = store.load_credential(&id)?;
    let snapshot = {
        let client = state.quota.lock().await;
        client.fetch_quota(&auth).await?
    };
    // Best-effort: refresh the tray icon with the new primary percent.
    // We deliberately swallow tray errors so a transient tray failure
    // never blocks the quota fetch from returning to the UI.
    let pct = snapshot.primary.used_percent as f32;
    if let Err(e) = apply_tray_icon(&app, pct) {
        log::warn!("tray icon update failed: {e}");
    }
    Ok(snapshot)
}

#[tauri::command]
async fn fetch_reset_credits(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<ResetCreditsSnapshot> {
    let store = AccountStore::new(&state.paths);
    let auth = store.load_credential(&id)?;
    let client = state.quota.lock().await;
    client.fetch_reset_credits(&auth).await
}

/// Render a fresh tray icon for the given usage percent and push it to the
/// tray. Exposed as a Tauri command so the frontend can also drive the icon
/// directly (e.g. when a refresh fails and we want to flash a warning state).
#[tauri::command]
fn apply_tray_icon_for(app: AppHandle, used_percent: f32) -> AppResult<()> {
    apply_tray_icon(&app, used_percent)
}

fn apply_tray_icon(app: &AppHandle, used_percent: f32) -> AppResult<()> {
    let bytes = tray_icon::render_png(used_percent);
    let img = Image::from_bytes(&bytes).map_err(|e| AppError::Config(format!("tray image: {e}")))?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_icon(Some(img))
            .map_err(|e| AppError::Config(format!("tray set_icon: {e}")))?;
        tray.set_tooltip(Some(format!(
            "Codex Runway · {:.0}% used",
            used_percent.clamp(0.0, 100.0)
        )))
        .ok();
    }
    Ok(())
}

#[tauri::command]
fn scan_local_sessions(state: State<'_, AppState>) -> AppResult<UsageSummary> {
    let scanner = SessionScanner::new(&state.paths);
    scanner.scan()
}

/// Optional ISO-8601 lower bound. `None` = all time.
#[tauri::command]
fn compute_api_cost(
    state: State<'_, AppState>,
    since: Option<String>,
) -> AppResult<cost::CostSummary> {
    let engine = match since {
        Some(s) => {
            let dt = chrono::DateTime::parse_from_rfc3339(&s)
                .map_err(|e| AppError::Auth(format!("invalid `since`: {e}")))?
                .with_timezone(&chrono::Utc);
            cost::CostEngine::new(&state.paths).since(dt)
        }
        None => cost::CostEngine::new(&state.paths),
    };
    engine.compute()
}

// ---------------------------------------------------------------------------
// OAuth login
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct OAuthSessionInfo {
    pub login_id: String,
    pub auth_url: String,
    pub port: u16,
    pub expires_at_unix: i64,
    /// The PKCE verifier. Returned to the frontend so it can be passed
    /// back to `oauth_finish` when exchanging the code. **Never** embedded
    /// in the auth URL.
    pub code_verifier: String,
    /// The CSRF state. Same handling as `code_verifier`.
    pub state: String,
}

#[tauri::command]
fn oauth_start() -> AppResult<OAuthSessionInfo> {
    // Bind the callback server NOW so we can return the actual port to the
    // frontend. The listener stays alive in the background; we hand the
    // raw fd ownership to a worker thread inside `oauth_finish`.
    //
    // We don't reuse a fixed port (e.g. 1455) because repeated OAuth flows
    // in the same minute would otherwise hit `address-in-use` from
    // TIME_WAIT. Letting the OS pick avoids that entirely.
    let server = oauth::CallbackServer::bind()?;
    let port = server.port();
    // Stash the listener in a global so `oauth_finish` can recover it
    // by matching on the port. (Tauri's State is a typed map; using a
    // dedicated Mutex<HashMap<u16, CallbackServer>> is overkill for the
    // one-in-flight-at-a-time pattern we have here.)
    PENDING_OAUTH.lock().unwrap().replace(server);
    let session = oauth::OAuthLogin::start_session(port, 600)?;
    Ok(OAuthSessionInfo {
        login_id: session.login_id,
        auth_url: session.auth_url,
        port,
        expires_at_unix: session.expires_at_unix,
        code_verifier: session.code_verifier,
        state: session.state,
    })
}

/// One in-flight OAuth session at a time. Replaced on every `oauth_start`;
/// taken out by `oauth_finish`.
static PENDING_OAUTH: std::sync::Mutex<Option<oauth::CallbackServer>> =
    std::sync::Mutex::new(None);

#[tauri::command]
async fn oauth_finish(
    state: State<'_, AppState>,
    port: u16,
    login_id: String,
    code_verifier: String,
    state_param: String,
) -> AppResult<crate::auth::CodexAuth> {
    // 1. Take ownership of the listener we created in `oauth_start`.
    let server = {
        let mut guard = PENDING_OAUTH.lock().unwrap();
        let pending = guard.take();
        match pending {
            Some(s) if s.port() == port => s,
            Some(other) => {
                // Port mismatch: someone else started a new session while
                // we were waiting. Drop ours; use the new one (best effort).
                let _ = other;
                return Err(AppError::Auth(
                    "OAuth session was superseded; please retry".to_string(),
                ));
            }
            None => {
                return Err(AppError::Auth(
                    "no pending OAuth session; please start again".to_string(),
                ));
            }
        }
    };
    let callback_url = tokio::task::spawn_blocking(move || {
        server.wait_for_callback(std::time::Duration::from_secs(600))
    })
    .await
    .map_err(|e| AppError::Auth(format!("join error: {e}")))??;

    // 2. Verify state + extract code.
    let code = oauth::OAuthLogin::authorization_code(&callback_url, &state_param)?;

    // 3. Reconstruct the session shell and exchange the code for tokens.
    let session = oauth::OAuthSession {
        login_id,
        auth_url: String::new(),
        redirect_uri: format!("http://localhost:{port}/auth/callback"),
        code_verifier,
        state: state_param,
        port,
        expires_at_unix: chrono::Utc::now().timestamp() + 600,
    };
    let auth = oauth::OAuthLogin::exchange_code(&session, &code).await?;

    // 4. Persist into the multi-account library so the user can pick it
    //    from the popover. The account id is the JWT `sub` claim, falling
    //    back to a stable hash of the refresh token.
    let id = auth
        .account_id()
        .or_else(|| crate::account::subject_id_from(&auth))
        .unwrap_or_else(|| "imported".to_string());
    let label = crate::account::email_from(&auth).unwrap_or_else(|| id.clone());
    let store = AccountStore::new(&state.paths);
    store.save_credential(&id, &auth)?;
    let mut idx = store.load_index()?;
    let now = chrono::Utc::now();
    let row = crate::account::AccountRow {
        id: id.clone(),
        label,
        email: crate::account::email_from(&auth),
        subject_id: crate::account::subject_id_from(&auth),
        account_id: auth.account_id(),
        plan_type: auth.plan_type.clone(),
        auth_mode: if auth.is_api_key() {
            AccountAuthMode::ApiKey
        } else {
            AccountAuthMode::Oauth
        },
        added_at: now,
        last_used_at: Some(now),
        requires_reauth: false,
        last_error: None,
        workspace: None,
    };
    if let Some(existing) = idx.accounts.iter_mut().find(|r| r.id == id) {
        *existing = row;
    } else {
        idx.accounts.push(row);
    }
    if idx.active_id.is_none() {
        idx.active_id = Some(id.clone());
    }
    store.save_index(&idx)?;
    Ok(auth)
}

#[tauri::command]
fn open_official_auth_in_explorer(state: State<'_, AppState>) -> AppResult<()> {
    let path = state.paths.official_auth();
    open_in_explorer(&path)
}

#[tauri::command]
fn open_app_home_in_explorer(state: State<'_, AppState>) -> AppResult<()> {
    open_in_explorer(&state.paths.app_home)
}

fn open_in_explorer(path: &std::path::Path) -> AppResult<()> {
    if !path.exists() {
        return Err(AppError::Config(format!(
            "path does not exist: {}",
            path.display()
        )));
    }
    #[cfg(windows)]
    {
        let arg = format!("/select,{}", path.display());
        std::process::Command::new("explorer")
            .arg(arg)
            .spawn()
            .map_err(AppError::from)?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(AppError::from)?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Open the parent directory if we can't select the file.
        let target = path.parent().unwrap_or(path);
        std::process::Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(AppError::from)?;
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedactedAuth {
    pub auth_mode: Option<String>,
    pub account_id: Option<String>,
    pub email: Option<String>,
    pub plan_type: Option<String>,
    pub has_access_token: bool,
    pub has_refresh_token: bool,
    pub usability: String,
}

impl From<&CodexAuth> for RedactedAuth {
    fn from(a: &CodexAuth) -> Self {
        let email = crate::account::email_from(a);
        Self {
            auth_mode: a.auth_mode.clone(),
            account_id: a.account_id(),
            email,
            plan_type: a.plan_type.clone(),
            has_access_token: a
                .tokens
                .as_ref()
                .map(|t| !t.access_token.is_empty())
                .unwrap_or(false),
            has_refresh_token: a
                .tokens
                .as_ref()
                .map(|t| !t.refresh_token.is_empty())
                .unwrap_or(false),
            usability: format!("{:?}", a.login_usability()),
        }
    }
}

/// Convert an `AppError` to a Tauri-friendly `InvokeError` for commands.
pub fn into_invoke_error(e: AppError) -> tauri::ipc::InvokeError {
    let api: ApiError = e.into();
    tauri::ipc::InvokeError::from(api.message)
}

// Re-exported path type for external helpers.
pub type CodePath = PathBuf;

// -----------------------------------------------------------------------------
// Tray + window plumbing (kept here so all windows share the same state)
// -----------------------------------------------------------------------------

/// Position the popover window near the tray icon, then show it.
pub fn show_popover(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if let Ok(pos) = tray_position(app) {
            let _ = win.set_position(pos);
        }
        let _ = win.show();
        let _ = win.set_focus();
    }
}

pub fn hide_popover(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

#[cfg(target_os = "windows")]
fn tray_position(app: &AppHandle) -> tauri::Result<tauri::PhysicalPosition<i32>> {
    // Position the popover above the taskbar, centered on the primary
    // monitor. We don't try to read the tray icon's exact rect here:
    // the Tauri 2 `tauri::Position` / `tauri::Size` types differ between
    // macOS and Windows (one exposes `x/y`, the other is logical), and
    // getting them wrong leaves the window off-screen. Instead we center
    // on the primary monitor and let the user move it if they want.
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(Some(monitor)) = window.current_monitor() {
            let size = monitor.size(); // PhysicalSize<u32>
            let pos = monitor.position(); // PhysicalPosition<i32>
            let pop_w = 380_i32;
            let pop_h = 560_i32;
            let x = pos.x + (size.width as i32 - pop_w) / 2;
            let y = pos.y + (size.height as i32 - pop_h - 40).max(0);
            return Ok(tauri::PhysicalPosition::new(x, y));
        }
    }
    Ok(tauri::PhysicalPosition::new(100, 100))
}

#[cfg(not(target_os = "windows"))]
fn tray_position(_app: &AppHandle) -> tauri::Result<tauri::PhysicalPosition<i32>> {
    Ok(tauri::PhysicalPosition::new(100, 100))
}

// -----------------------------------------------------------------------------
// `tauri::Builder::default()` glue
// -----------------------------------------------------------------------------

pub fn run() {
    // Install the panic hook *first* so any subsequent panic during
    // startup produces a log file (and a Windows MessageBox) instead of a
    // silent abort.
    crashlog::install();
    let _ = env_logger::try_init();
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("fatal: {e}");
            std::process::exit(1);
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_popover(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            app_info,
            load_official_auth,
            list_accounts,
            upsert_account,
            delete_account,
            activate_account,
            set_active_account,
            fetch_quota,
            fetch_reset_credits,
            apply_tray_icon_for,
            scan_local_sessions,
            compute_api_cost,
            oauth_start,
            oauth_finish,
            open_official_auth_in_explorer,
            open_app_home_in_explorer,
        ])
        .setup(|app| {
            // Tray: left click toggles the popover, right click opens a
            // context menu.
            let handle_for_event = app.handle().clone();
            let handle_for_menu = app.handle().clone();
            if let Some(tray) = app.tray_by_id("main") {
                tray.on_tray_icon_event(move |_tray, event| {
                    use tauri::tray::TrayIconEvent;
                    if let TrayIconEvent::Click { button, .. } = event {
                        if matches!(button, tauri::tray::MouseButton::Left) {
                            toggle_popover(&handle_for_event);
                        }
                    }
                });
                let menu = build_tray_menu(&handle_for_menu)?;
                let _ = tray.set_menu(Some(menu));
                let handle_for_menu_event = app.handle().clone();
                tray.on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => show_popover(app),
                        "refresh" => {
                            show_popover(app);
                            let _ = app.emit("refresh-requested", ());
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    }
                });
            }
            // We deliberately do NOT pre-render the tray icon here.
            // tauri.conf.json already wires a static iconPath. Calling
            // `tray.set_icon()` during setup races with that initial
            // render and can produce a no-icon / black-square tray entry
            // on Windows 10. The icon gets a fresh draw on the very first
            // `fetch_quota` (which is also when we have a real percent).
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Codex Runway");
}

fn toggle_popover(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        match win.is_visible() {
            Ok(true) => hide_popover(app),
            _ => show_popover(app),
        }
    }
}

fn build_tray_menu(app: &AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    let show = MenuItem::with_id(app, "show", "Open Codex Runway", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Refresh quota", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    Menu::with_items(app, &[&show, &refresh, &sep, &quit])
}
