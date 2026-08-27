//! Cross-platform home + Codex home path resolution.
//!
//! On Windows the official Codex CLI keeps state under
//! `%USERPROFILE%\.codex\`. We mirror that here. On macOS / Linux (used only
//! for dev), we fall back to `~/.codex/` so contributors on other OSes can
//! run the test suite.

use std::path::{Path, PathBuf};

use crate::error::AppError;

pub const APP_DIR_NAME: &str = ".codex-runway";
pub const ACCOUNTS_DIR_NAME: &str = "accounts";
pub const INDEX_FILE_NAME: &str = "index.json";
pub const AUTH_FILE_NAME: &str = "auth.json";
pub const SESSIONS_DIR_NAME: &str = "sessions";

#[derive(Debug, Clone)]
pub struct Paths {
    /// `%USERPROFILE%` on Windows, `~` elsewhere.
    pub home: PathBuf,
    /// `%USERPROFILE%\.codex` on Windows.
    pub codex_home: PathBuf,
    /// `%USERPROFILE%\.codex-runway`.
    pub app_home: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self, AppError> {
        // Prefer %USERPROFILE% (Windows) so we match what the Codex CLI uses,
        // not whatever %HOME% may have been inherited from a service context.
        let home = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .ok_or_else(|| {
                AppError::Config("USERPROFILE / HOME is not set".to_string())
            })?;
        if !home.exists() {
            return Err(AppError::Config(format!(
                "Home directory does not exist: {}",
                home.display()
            )));
        }
        let codex_home = home.join(".codex");
        let app_home = home.join(APP_DIR_NAME);
        Ok(Self { home, codex_home, app_home })
    }

    pub fn official_auth(&self) -> PathBuf {
        self.codex_home.join(AUTH_FILE_NAME)
    }

    pub fn app_accounts(&self) -> PathBuf {
        self.app_home.join(ACCOUNTS_DIR_NAME)
    }

    pub fn account_index(&self) -> PathBuf {
        self.app_home.join(ACCOUNTS_DIR_NAME).join(INDEX_FILE_NAME)
    }

    pub fn account_dir(&self, id: &str) -> PathBuf {
        self.app_accounts().join(sanitize(id))
    }

    pub fn account_auth(&self, id: &str) -> PathBuf {
        self.account_dir(id).join(AUTH_FILE_NAME)
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.codex_home.join(SESSIONS_DIR_NAME)
    }
}

/// Sanitize a free-form id into a single path component. Codex account ids
/// are opaque; we still need to keep them from escaping the accounts dir.
pub fn sanitize(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for ch in id.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "default".to_string()
    } else {
        out
    }
}

pub fn ensure_dir_restricted(path: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(path).map_err(AppError::from)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let parent = path.parent().ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "target path has no parent",
        ))
    })?;
    let tmp = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("file"),
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&tmp, data)?;
    // Best-effort restrictive perms on the temp file before rename.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(windows)]
    {
        // On Windows, if target exists, std::fs::rename will fail with ERROR_ALREADY_EXISTS
        // or access denied unless replaced. If target exists, try remove first or fallback to write.
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
        if let Err(_) = std::fs::rename(&tmp, path) {
            // Fallback direct write in case rename is blocked by antivirus or volume lock
            std::fs::write(path, data)?;
            let _ = std::fs::remove_file(&tmp);
        }
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(&tmp, path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_safe_chars() {
        assert_eq!(sanitize("codex-acc_1.0"), "codex-acc_1.0");
        assert_eq!(sanitize("a/b\\c"), "a_b_c");
        assert_eq!(sanitize(""), "default");
        assert_eq!(sanitize("../etc/passwd"), ".._etc_passwd");
    }
}
