use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("network: {0}")]
    Network(String),
    #[error("config: {0}")]
    Config(String),
    #[error("auth: {0}")]
    Auth(String),
    #[error("account: {0}")]
    Account(String),
    #[error("invalid credential: {0}")]
    InvalidCredential(String),
    #[error("quota: {0}")]
    Quota(String),
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

/// Tauri commands need a `Serialize` error variant. We collapse everything
/// to a string so the frontend can display it without re-deriving.
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub kind: String,
    pub message: String,
}

impl From<AppError> for ApiError {
    fn from(e: AppError) -> Self {
        let kind = match &e {
            AppError::Io(_) => "io",
            AppError::Json(_) => "json",
            AppError::Http(_) => "http",
            AppError::Network(_) => "network",
            AppError::Config(_) => "config",
            AppError::Auth(_) => "auth",
            AppError::Account(_) => "account",
            AppError::InvalidCredential(_) => "invalid_credential",
            AppError::Quota(_) => "quota",
            AppError::NotImplemented(_) => "not_implemented",
        };
        ApiError { kind: kind.to_string(), message: e.to_string() }
    }
}

impl From<AppError> for tauri::ipc::InvokeError {
    fn from(e: AppError) -> Self {
        let api: ApiError = e.into();
        tauri::ipc::InvokeError::from(api.message)
    }
}

pub type AppResult<T> = Result<T, AppError>;
