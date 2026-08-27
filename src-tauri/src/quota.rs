//! Codex quota API client.
//!
//! Mirrors the macOS original's `QuotaClient` (see
//! `Sources/CodexRunwayCore/QuotaClient.swift`). The backend is the same
//! `chatgpt.com/backend-api` host the official Codex CLI uses.

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::auth::CodexAuth;
use crate::error::{AppError, AppResult};

const DEFAULT_BASE: &str = "https://chatgpt.com/backend-api";
const USER_AGENT_STR: &str = "CodexRunway-Windows/0.1";

#[derive(Debug, Clone)]
pub struct QuotaClient {
    http: Client,
    base: String,
}

impl QuotaClient {
    pub fn new() -> AppResult<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(8))
            .user_agent(USER_AGENT_STR)
            .build()
            .map_err(AppError::Http)?;
        Ok(Self { http, base: DEFAULT_BASE.to_string() })
    }

    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        self.base = base.into();
        self
    }

    pub async fn fetch_quota(&self, auth: &CodexAuth) -> AppResult<QuotaSnapshot> {
        let data = self.get("wham/usage", auth).await?;
        let resp: QuotaResponse = serde_json::from_slice(&data)?;
        Ok(QuotaSnapshot::from_response(resp, Utc::now()))
    }

    pub async fn fetch_reset_credits(&self, auth: &CodexAuth) -> AppResult<ResetCreditsSnapshot> {
        let data = self.get("wham/rate-limit-reset-credits", auth).await?;
        let resp: ResetCreditsResponse = serde_json::from_slice(&data)?;
        Ok(ResetCreditsSnapshot::from_response(resp, Utc::now()))
    }

    pub async fn fetch_profile_token_usage(
        &self,
        auth: &CodexAuth,
    ) -> AppResult<CodexProfileTokenUsage> {
        let data = self.get("wham/profiles/me", auth).await?;
        serde_json::from_slice(&data).map_err(AppError::from)
    }

    async fn get(&self, path: &str, auth: &CodexAuth) -> AppResult<Vec<u8>> {
        let url = format!("{}/{}", self.base.trim_end_matches('/'), path);
        let mut headers = HeaderMap::new();
        let token = auth.access_token().ok_or_else(|| {
            AppError::Auth("missing access_token; cannot call Codex API".to_string())
        })?;
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| {
                AppError::Auth("access_token contains non-ASCII characters".to_string())
            })?,
        );
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STR));
        headers.insert(
            "Accept",
            HeaderValue::from_static("application/json"),
        );
        if let Some(acc) = auth.account_id() {
            if !acc.is_empty() {
                if let Ok(v) = HeaderValue::from_str(&acc) {
                    headers.insert("ChatGPT-Account-Id", v);
                }
            }
        }
        let resp = self.http.get(&url).headers(headers).send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if !status.is_success() {
            return Err(AppError::Quota(format!(
                "{} {}: {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                String::from_utf8_lossy(&bytes).chars().take(200).collect::<String>()
            )));
        }
        if status == StatusCode::NO_CONTENT {
            return Ok(Vec::new());
        }
        Ok(bytes.to_vec())
    }
}

// --- Response models ---------------------------------------------------------

#[derive(Debug, Deserialize)]
struct QuotaResponse {
    #[serde(rename = "plan_type")]
    plan_type: Option<String>,
    #[serde(default)]
    rate_limit: Option<RateLimit>,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    additional_rate_limits: Vec<NamedRateLimit>,
    #[serde(default)]
    credits: Option<CreditsBlock>,
}

fn deserialize_null_as_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let opt = Option::<Vec<T>>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

fn deserialize_flexible_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    match opt {
        Some(serde_json::Value::Number(n)) => Ok(n.as_f64()),
        Some(serde_json::Value::String(s)) => s.parse::<f64>().map(Some).map_err(serde::de::Error::custom),
        Some(serde_json::Value::Null) | None => Ok(None),
        _ => Ok(None),
    }
}

fn deserialize_flexible_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    match opt {
        Some(serde_json::Value::Number(n)) => Ok(n.as_i64()),
        Some(serde_json::Value::String(s)) => s.parse::<i64>().map(Some).map_err(serde::de::Error::custom),
        Some(serde_json::Value::Null) | None => Ok(None),
        _ => Ok(None),
    }
}

#[derive(Debug, Default, Deserialize)]
struct RateLimit {
    primary_window: Option<RateWindowRaw>,
    secondary_window: Option<RateWindowRaw>,
}

#[derive(Debug, Deserialize)]
struct RateWindowRaw {
    #[serde(default, rename = "used_percent", deserialize_with = "deserialize_flexible_f64")]
    used_percent: Option<f64>,
    #[serde(default, rename = "window_minutes", deserialize_with = "deserialize_flexible_i64")]
    window_minutes: Option<i64>,
    #[serde(default, rename = "reset_at", deserialize_with = "deserialize_flexible_i64")]
    reset_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct NamedRateLimit {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    rate_window: Option<RateWindowRaw>,
}

#[derive(Debug, Deserialize)]
struct CreditsBlock {
    #[serde(default, deserialize_with = "deserialize_flexible_f64")]
    balance: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateWindow {
    pub used_percent: f64,
    pub window_minutes: Option<i64>,
    pub resets_at: Option<DateTime<Utc>>,
}

impl RateWindow {
    fn from_raw(raw: RateWindowRaw) -> Self {
        let used = raw.used_percent.unwrap_or(0.0).clamp(0.0, 100.0);
        Self {
            used_percent: used,
            window_minutes: raw.window_minutes,
            resets_at: raw.reset_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NamedRateWindow {
    pub name: String,
    pub window: RateWindow,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuotaSnapshot {
    pub plan: Option<String>,
    pub primary: RateWindow,
    pub secondary: Option<RateWindow>,
    pub additional_windows: Vec<NamedRateWindow>,
    pub credits_balance: Option<f64>,
    pub updated_at: DateTime<Utc>,
}

impl QuotaSnapshot {
    fn from_response(r: QuotaResponse, now: DateTime<Utc>) -> Self {
        let rate_limit = r.rate_limit.unwrap_or_default();
        let primary = rate_limit
            .primary_window
            .map(RateWindow::from_raw)
            .unwrap_or(RateWindow {
                used_percent: 0.0,
                window_minutes: None,
                resets_at: None,
            });
        let secondary = rate_limit.secondary_window.map(RateWindow::from_raw);
        let additional_windows = r
            .additional_rate_limits
            .into_iter()
            .filter_map(|n| {
                Some(NamedRateWindow {
                    name: n.name?,
                    window: n.rate_window.map(RateWindow::from_raw)?,
                })
            })
            .collect();
        Self {
            plan: r.plan_type,
            primary,
            secondary,
            additional_windows,
            credits_balance: r.credits.and_then(|c| c.balance),
            updated_at: now,
        }
    }

    /// Convenience: the percentage used by the primary window, rounded.
    pub fn primary_used_percent(&self) -> i64 {
        self.primary.used_percent.round() as i64
    }

    /// Convenience: the time remaining on the primary window in seconds.
    pub fn primary_remaining_secs(&self, now: DateTime<Utc>) -> Option<i64> {
        self.primary
            .resets_at
            .map(|r| (r - now).num_seconds().max(0))
    }
}

#[derive(Debug, Deserialize)]
struct ResetCreditsResponse {
    #[serde(default, rename = "available_count", deserialize_with = "deserialize_flexible_i64")]
    available_count: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    credits: Vec<ResetCreditRaw>,
}

#[derive(Debug, Deserialize)]
struct ResetCreditRaw {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default, rename = "created_at", deserialize_with = "deserialize_flexible_date")]
    created_at: Option<DateTime<Utc>>,
    #[serde(default, rename = "expires_at", deserialize_with = "deserialize_flexible_date")]
    expires_at: Option<DateTime<Utc>>,
}

fn deserialize_flexible_date<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    match opt {
        Some(serde_json::Value::Number(n)) => {
            if let Some(ts) = n.as_i64() {
                Ok(DateTime::from_timestamp(ts, 0))
            } else if let Some(ts) = n.as_f64() {
                Ok(DateTime::from_timestamp(ts as i64, 0))
            } else {
                Ok(None)
            }
        }
        Some(serde_json::Value::String(s)) => {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                Ok(Some(dt.with_timezone(&Utc)))
            } else if let Ok(ts) = s.parse::<i64>() {
                Ok(DateTime::from_timestamp(ts, 0))
            } else {
                Ok(None)
            }
        }
        Some(serde_json::Value::Null) | None => Ok(None),
        _ => Ok(None),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResetCredit {
    pub id: Option<String>,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub remaining_seconds: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResetCreditsSnapshot {
    pub available_count: i64,
    pub credits: Vec<ResetCredit>,
    pub updated_at: DateTime<Utc>,
}

impl ResetCreditsSnapshot {
    fn from_response(r: ResetCreditsResponse, now: DateTime<Utc>) -> Self {
        let credits: Vec<ResetCredit> = r
            .credits
            .into_iter()
            .map(|c| {
                let expires = c.expires_at;
                let remaining = expires
                    .map(|e| (e - now).num_seconds().max(0))
                    .unwrap_or(0);
                ResetCredit {
                    id: c.id,
                    status: c.status.unwrap_or_else(|| "unknown".to_string()),
                    created_at: c.created_at,
                    expires_at: expires,
                    remaining_seconds: remaining,
                }
            })
            .collect();
        let available = r
            .available_count
            .unwrap_or_else(|| credits.iter().filter(|c| c.status == "available").count() as i64);
        Self {
            available_count: available,
            credits,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexProfileTokenUsage {
    // The backend can change shape; keep this permissive and surface the raw
    // JSON when in doubt. We just need a value to display in the popover.
    #[serde(flatten)]
    pub fields: BTreeMap<String, serde_json::Value>,
}
