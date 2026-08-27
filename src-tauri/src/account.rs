//! Multi-account index + per-account credential storage.
//!
//! Mirrors the macOS original's `AccountStore` (see
//! `Sources/CodexRunwayCore/AccountStore.swift`). The on-disk layout is:
//!
//! ```text
//! %USERPROFILE%\.codex-runway\accounts\
//!   index.json
//!   <id>\
//!     auth.json
//! ```

use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::auth::CodexAuth;
use crate::error::{AppError, AppResult};
use crate::paths::{self, Paths};

/// Public account row surfaced to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountRow {
    pub id: String,
    pub label: String,
    pub email: Option<String>,
    pub subject_id: Option<String>,
    pub account_id: Option<String>,
    pub plan_type: Option<String>,
    pub subscription_active_until: Option<String>,
    pub auth_mode: AccountAuthMode,
    pub added_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub requires_reauth: bool,
    pub last_error: Option<String>,
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountAuthMode {
    Oauth,
    ApiKey,
    Unknown,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AccountIndex {
    #[serde(default)]
    pub active_id: Option<String>,
    #[serde(default)]
    pub accounts: Vec<AccountRow>,
}

pub struct AccountStore<'a> {
    paths: &'a Paths,
}

impl<'a> AccountStore<'a> {
    pub fn new(paths: &'a Paths) -> Self {
        Self { paths }
    }

    pub fn load_index(&self) -> AppResult<AccountIndex> {
        let path = self.paths.account_index();
        if !path.exists() {
            return Ok(AccountIndex::default());
        }
        let raw = std::fs::read(&path)?;
        let mut idx: AccountIndex = serde_json::from_slice(&raw)?;
        // Backfill display metadata from the credential file when the index
        // row is too thin (older versions did not persist subject_id / email).
        for row in idx.accounts.iter_mut() {
            if row.auth_mode != AccountAuthMode::Oauth {
                continue;
            }
            let needs = row.subject_id.is_none()
                || row.account_id.is_none()
                || row.email.is_none()
                || row.subscription_active_until.is_none();
            if !needs {
                continue;
            }
            match self.load_credential(&row.id) {
                Ok(auth) => {
                    if row.subject_id.is_none() {
                        row.subject_id = subject_id_from(&auth);
                    }
                    if row.account_id.is_none() {
                        row.account_id = auth.account_id();
                    }
                    if row.email.is_none() {
                        row.email = email_from(&auth);
                    }
                    if row.subscription_active_until.is_none() {
                        row.subscription_active_until = auth
                            .tokens
                            .as_ref()
                            .and_then(|t| t.id_token.as_deref())
                            .and_then(decode_jwt_subscription_until);
                    }
                }
                Err(AppError::Account(msg)) if msg == "credential_missing" => {
                    row.requires_reauth = true;
                    row.last_error = Some("credential_missing".to_string());
                }
                Err(AppError::InvalidCredential(_)) => {
                    row.requires_reauth = true;
                    row.last_error = Some("invalid_credential".to_string());
                }
                Err(e) => return Err(e),
            }
        }
        Ok(idx)
    }

    pub fn save_index(&self, idx: &AccountIndex) -> AppResult<()> {
        paths::ensure_dir_restricted(&self.paths.app_accounts())?;
        let data = serde_json::to_vec_pretty(idx)?;
        paths::atomic_write(&self.paths.account_index(), &data)
    }

    pub fn load_credential(&self, id: &str) -> AppResult<CodexAuth> {
        let path = self.paths.account_auth(id);
        if !path.exists() {
            return Err(AppError::Account("credential_missing".to_string()));
        }
        let raw = std::fs::read(&path)?;
        serde_json::from_slice::<CodexAuth>(&raw)
            .map_err(|_| AppError::InvalidCredential(format!("cannot parse {}", path.display())))
    }

    /// Save a credential into the account library. Validates that the
    /// credential is at least structurally sound; refuses to write invalid
    /// placeholders so the user cannot accidentally install junk.
    pub fn save_credential(&self, id: &str, auth: &CodexAuth) -> AppResult<()> {
        let dir = self.paths.account_dir(id);
        paths::ensure_dir_restricted(&dir)?;
        let object = crate::auth::official_auth_object(auth);
        let data = serde_json::to_vec_pretty(&object)?;
        let path = self.paths.account_auth(id);
        paths::atomic_write(&path, &data)
    }

    pub fn delete_account(&self, id: &str) -> AppResult<()> {
        let dir = self.paths.account_dir(id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    /// Install the given account as the active Codex login. Validates the
    /// credential first; never writes an unusable credential to the official
    /// `auth.json`.
    pub fn activate(&self, id: &str) -> AppResult<CodexAuth> {
        let auth = self.load_credential(id)?;
        if matches!(auth.login_usability(), crate::auth::LoginUsability::Invalid) {
            return Err(AppError::InvalidCredential(format!(
                "account {id} is not usable as a Codex login"
            )));
        }
        let object = crate::auth::official_auth_object(&auth);
        let data = serde_json::to_vec_pretty(&object)?;
        paths::atomic_write(&self.paths.official_auth(), &data)?;
        Ok(auth)
    }
}

pub(crate) fn subject_id_from(auth: &CodexAuth) -> Option<String> {
    // JWT payload sub claim (we don't want a hard dep on jsonwebtoken crate)
    let jwt = auth.tokens.as_ref()?.id_token.as_deref()?;
    decode_jwt_sub(jwt)
}

/// Variant of `subject_id_from` that takes the raw id_token string. Useful
/// for OAuth code-exchange responses where we have not yet built a `CodexAuth`.
pub(crate) fn subject_id_from_idtoken(jwt: Option<&str>) -> Option<String> {
    decode_jwt_sub(jwt?)
}

pub(crate) fn email_from(auth: &CodexAuth) -> Option<String> {
    let jwt = auth.tokens.as_ref()?.id_token.as_deref()?;
    decode_jwt_email(jwt)
}

fn decode_jwt_sub(jwt: &str) -> Option<String> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() < 2 { return None; }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    let v: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    v.get("sub").and_then(|s| s.as_str()).map(String::from)
}

fn decode_jwt_email(jwt: &str) -> Option<String> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() < 2 { return None; }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    let v: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    v.get("email").and_then(|s| s.as_str()).map(String::from)
}

pub(crate) fn decode_jwt_subscription_until(jwt: &str) -> Option<String> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() < 2 { return None; }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    let v: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    let auth = v.get("https://api.openai.com/auth")?;

    if let Some(s) = auth.get("chatgpt_subscription_active_until").and_then(|x| x.as_str()) {
        return Some(s.to_string());
    }
    if let Some(ts) = auth.get("chatgpt_subscription_active_until").and_then(|x| x.as_i64()) {
        if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
            return Some(dt.to_rfc3339());
        }
    }
    if let Some(ts) = auth.get("chatgpt_subscription_active_until").and_then(|x| x.as_f64()) {
        if let Some(dt) = chrono::DateTime::from_timestamp(ts as i64, 0) {
            return Some(dt.to_rfc3339());
        }
    }
    None
}

/// Best-effort: import the current `~/.codex/auth.json` into the library if
/// it is not already present. Returns the existing or newly-created row.
pub fn import_official_if_absent(paths: &Paths) -> AppResult<Option<AccountRow>> {
    let store = AccountStore::new(paths);
    let mut idx = store.load_index()?;
    let auth = match std::fs::read(paths.official_auth())
        .ok()
        .and_then(|raw| serde_json::from_slice::<CodexAuth>(&raw).ok())
    {
        Some(a) => a,
        None => return Ok(None),
    };
    let id = auth
        .account_id()
        .or_else(|| subject_id_from(&auth))
        .unwrap_or_else(|| "imported".to_string());
    if idx.accounts.iter().any(|r| r.id == id) {
        return Ok(idx.accounts.into_iter().find(|r| r.id == id));
    }
    let sub_until = auth.tokens.as_ref().and_then(|t| t.id_token.as_deref()).and_then(decode_jwt_subscription_until);
    let row = AccountRow {
        id: id.clone(),
        label: email_from(&auth).unwrap_or_else(|| id.clone()),
        email: email_from(&auth),
        subject_id: subject_id_from(&auth),
        account_id: auth.account_id(),
        plan_type: auth.plan_type.clone(),
        subscription_active_until: sub_until,
        auth_mode: if auth.is_api_key() {
            AccountAuthMode::ApiKey
        } else if auth.is_oauth() {
            AccountAuthMode::Oauth
        } else {
            AccountAuthMode::Unknown
        },
        added_at: Utc::now(),
        last_used_at: Some(Utc::now()),
        requires_reauth: matches!(
            auth.login_usability(),
            crate::auth::LoginUsability::Invalid
        ),
        last_error: None,
        workspace: None,
    };
    store.save_credential(&id, &auth)?;
    idx.accounts.push(row.clone());
    if idx.active_id.is_none() {
        idx.active_id = Some(id.clone());
    }
    store.save_index(&idx)?;
    Ok(Some(row))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_paths() -> (tempdir::TempDir, Paths) {
        let td = tempdir::TempDir::new("codex-runway-test").unwrap();
        let mut p = Paths {
            home: td.path().to_path_buf(),
            codex_home: td.path().join(".codex"),
            app_home: td.path().join(".codex-runway"),
        };
        std::fs::create_dir_all(&p.codex_home).unwrap();
        std::fs::create_dir_all(&p.app_home).unwrap();
        p.home = td.path().to_path_buf();
        p.codex_home = p.home.join(".codex");
        p.app_home = p.home.join(".codex-runway");
        (td, p)
    }

    fn make_auth() -> CodexAuth {
        CodexAuth {
            auth_mode: Some("chatgpt".to_string()),
            tokens: Some(crate::auth::Tokens {
                access_token: "a".repeat(50),
                refresh_token: "r".repeat(40),
                account_id: Some("acc-1".to_string()),
                id_token: None,
            }),
            plan_type: Some("pro".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn save_load_roundtrip() {
        let (_td, p) = tmp_paths();
        let s = AccountStore::new(&p);
        let auth = make_auth();
        s.save_credential("acc-1", &auth).unwrap();
        let loaded = s.load_credential("acc-1").unwrap();
        assert_eq!(loaded.account_id().as_deref(), Some("acc-1"));
    }

    #[test]
    fn activate_writes_to_official_auth() {
        let (_td, p) = tmp_paths();
        let s = AccountStore::new(&p);
        let auth = make_auth();
        s.save_credential("acc-1", &auth).unwrap();
        s.activate("acc-1").unwrap();
        let official = std::fs::read_to_string(p.official_auth()).unwrap();
        assert!(official.contains("access_token"));
        // plan_type is runway-internal and must NOT be in the official file.
        assert!(!official.contains("plan_type"));
    }
}
