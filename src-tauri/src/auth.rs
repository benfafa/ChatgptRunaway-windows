//! Codex auth.json model.
//!
//! Mirrors the on-disk shape the official Codex CLI uses. The two fields the
//! Windows app actually needs are `tokens.access_token` and
//! `tokens.account_id`; everything else is preserved on round-trip.

use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodexAuth {
    #[serde(rename = "auth_mode", skip_serializing_if = "Option::is_none")]
    pub auth_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<Tokens>,
    #[serde(rename = "last_refresh", skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<String>,
    #[serde(rename = "plan_type", skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
    #[serde(rename = "auth_file_plan_type", skip_serializing_if = "Option::is_none")]
    pub auth_file_plan_type: Option<String>,
    #[serde(rename = "OPENAI_API_KEY", skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Tokens {
    #[serde(rename = "id_token", skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(rename = "access_token", default)]
    pub access_token: String,
    #[serde(rename = "refresh_token", default)]
    pub refresh_token: String,
    #[serde(rename = "account_id", skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}

impl Tokens {
    pub fn has_oauth_tokens(&self) -> bool {
        !self.access_token.is_empty() || !self.refresh_token.is_empty()
    }
}

impl CodexAuth {
    /// `true` when this credential can be installed as the official
    /// `~/.codex/auth.json` for the Codex CLI to read.
    pub fn login_usability(&self) -> LoginUsability {
        if self.is_api_key() {
            return self
                .openai_api_key
                .as_deref()
                .filter(|k| k.len() >= 8)
                .map(|_| LoginUsability::Usable)
                .unwrap_or(LoginUsability::Invalid);
        }
        let tokens = match &self.tokens {
            Some(t) => t,
            None => return LoginUsability::Invalid,
        };
        if tokens.access_token.len() < 40 {
            return LoginUsability::Invalid;
        }
        if !tokens.refresh_token.is_empty() {
            if tokens.refresh_token.len() < 20 {
                return LoginUsability::Invalid;
            }
            return LoginUsability::Usable;
        }
        // Session-style: access only. Allow switch while JWT is still valid.
        if is_jwt_expired(&tokens.access_token) {
            LoginUsability::ExpiredAccess
        } else {
            LoginUsability::Usable
        }
    }

    pub fn is_api_key(&self) -> bool {
        if let Some(key) = &self.openai_api_key {
            if !key.is_empty() && !self.tokens.as_ref().is_some_and(|t| t.has_oauth_tokens()) {
                return true;
            }
        }
        matches!(
            self.auth_mode.as_deref().map(str::to_ascii_lowercase),
            Some(ref m) if m == "apikey" || m == "api_key" || m == "api-key"
        )
    }

    pub fn is_oauth(&self) -> bool {
        !self.is_api_key() && self.tokens.as_ref().is_some_and(|t| t.has_oauth_tokens())
    }

    pub fn account_id(&self) -> Option<String> {
        self.tokens.as_ref().and_then(|t| t.account_id.clone())
    }

    pub fn access_token(&self) -> Option<String> {
        self.tokens.as_ref().map(|t| t.access_token.clone())
    }

    pub fn can_refresh_oauth(&self) -> bool {
        self.tokens
            .as_ref()
            .map(|t| !t.refresh_token.is_empty())
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginUsability {
    Usable,
    /// Access token is expired (or missing) and there is no refresh_token to
    /// renew it. The account can be kept in the library but not switched to.
    ExpiredAccess,
    /// Placeholder / truncated junk. Never written back to official
    /// `auth.json`.
    Invalid,
}

/// Best-effort JWT expiry check. Returns `true` if the token is expired or
/// we cannot parse it. Codex access tokens are short-lived JWTs.
pub fn is_jwt_expired(jwt: &str) -> bool {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() < 2 {
        return true;
    }
    let payload = match base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
    {
        Ok(b) => b,
        Err(_) => return true,
    };
    #[derive(Deserialize)]
    struct Claims {
        exp: Option<i64>,
    }
    let claims: Claims = match serde_json::from_slice(&payload) {
        Ok(c) => c,
        Err(_) => return true,
    };
    let exp = match claims.exp {
        Some(e) => e,
        None => return false,
    };
    let now = chrono::Utc::now().timestamp();
    // 60s skew to match the upstream macOS app.
    exp <= now + 60
}

/// Encode the auth back to a JSON object suitable for writing to
/// `auth.json`. We deliberately do NOT emit our `plan_type` /
/// `auth_file_plan_type` into the official file, because Codex file
/// watchers may reject them.
pub fn official_auth_object(auth: &CodexAuth) -> serde_json::Value {
    let mut v = serde_json::json!({});
    let mode = auth
        .auth_mode
        .clone()
        .unwrap_or_else(|| if auth.is_api_key() { "apikey".to_string() } else { "chatgpt".to_string() });
    v["auth_mode"] = serde_json::Value::String(mode);
    if auth.is_api_key() {
        if let Some(k) = &auth.openai_api_key {
            v["OPENAI_API_KEY"] = serde_json::Value::String(k.clone());
        }
        return v;
    }
    if let Some(t) = &auth.tokens {
        let mut tokens = serde_json::Map::new();
        if let Some(id) = &t.id_token {
            if !id.is_empty() {
                tokens.insert("id_token".to_string(), serde_json::Value::String(id.clone()));
            }
        }
        tokens.insert("access_token".to_string(), serde_json::Value::String(t.access_token.clone()));
        // Codex expects refresh_token to always be present (may be empty).
        tokens.insert("refresh_token".to_string(), serde_json::Value::String(t.refresh_token.clone()));
        if let Some(acc) = &t.account_id {
            if !acc.is_empty() {
                tokens.insert("account_id".to_string(), serde_json::Value::String(acc.clone()));
            }
        }
        v["tokens"] = serde_json::Value::Object(tokens);
        if let Some(lr) = &auth.last_refresh {
            v["last_refresh"] = serde_json::Value::String(lr.clone());
        }
        if let Some(k) = &auth.openai_api_key {
            if !k.is_empty() {
                v["OPENAI_API_KEY"] = serde_json::Value::String(k.clone());
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oauth_auth(access: &str, refresh: &str, account: Option<&str>) -> CodexAuth {
        CodexAuth {
            auth_mode: Some("chatgpt".to_string()),
            tokens: Some(Tokens {
                access_token: access.to_string(),
                refresh_token: refresh.to_string(),
                account_id: account.map(String::from),
                id_token: None,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn short_access_token_is_invalid() {
        let a = oauth_auth("short", "long_refresh_token_value", None);
        assert_eq!(a.login_usability(), LoginUsability::Invalid);
    }

    #[test]
    fn long_access_with_refresh_is_usable() {
        let a = oauth_auth(&"a".repeat(50), &"r".repeat(40), Some("acc-1"));
        assert_eq!(a.login_usability(), LoginUsability::Usable);
    }

    #[test]
    fn short_refresh_is_invalid() {
        let a = oauth_auth(&"a".repeat(50), "rt", Some("acc-1"));
        assert_eq!(a.login_usability(), LoginUsability::Invalid);
    }

    #[test]
    fn api_key_auth_is_usable() {
        let a = CodexAuth {
            auth_mode: Some("apikey".to_string()),
            openai_api_key: Some("sk-12345678".to_string()),
            ..Default::default()
        };
        assert!(a.is_api_key());
        assert_eq!(a.login_usability(), LoginUsability::Usable);
    }

    #[test]
    fn official_object_strips_plan_fields() {
        let mut a = oauth_auth(&"a".repeat(50), &"r".repeat(40), Some("acc-1"));
        a.plan_type = Some("pro".to_string());
        let v = official_auth_object(&a);
        assert!(v.get("plan_type").is_none());
        assert!(v.get("tokens").is_some());
    }
}
