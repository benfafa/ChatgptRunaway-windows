//! Local Codex session log parser.
//!
//! Mirrors the upstream `UsageCostLogParser` (see
//! `Sources/CodexRunwayCore/UsageCostLogParser.swift`). We scan
//! `%USERPROFILE%\.codex\sessions\**\*.jsonl` for `task_complete` events and
//! extract token usage + model from the surrounding `turn_context` payload.
//!
//! Important: this module is read-only. It never writes back to the
//! official Codex data dir.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use walkdir::WalkDir;

use crate::error::AppResult;
use crate::paths::Paths;

#[derive(Debug, Clone, Serialize)]
pub struct SessionTurn {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub cwd: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct UsageSummary {
    pub sessions_scanned: usize,
    pub turns_scanned: usize,
    pub total_input_tokens: i64,
    pub total_cached_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_tokens: i64,
    pub per_model: Vec<ModelUsage>,
    pub recent: Vec<SessionTurn>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelUsage {
    pub model: String,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
}

/// Plain token counts. The session scanner already produces these as
/// `SessionTurn.input_tokens` / `.cached_input_tokens` / `.output_tokens`,
/// but the cost engine also needs them, so we re-export a tiny struct
/// here to keep `pricing.rs` decoupled from the `SessionTurn` private
/// fields.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
}

impl TokenUsage {
    /// Total input tokens billed to the API, i.e. cached + uncached. Used
    /// to decide whether a long-context tier applies.
    pub fn input_total(&self) -> i64 {
        self.input_tokens
    }
}

impl From<&SessionTurn> for TokenUsage {
    fn from(t: &SessionTurn) -> Self {
        Self {
            input_tokens: t.input_tokens,
            cached_input_tokens: t.cached_input_tokens,
            output_tokens: t.output_tokens,
        }
    }
}

pub struct SessionScanner<'a> {
    paths: &'a Paths,
    /// Maximum number of `recent` rows to keep in memory.
    pub recent_limit: usize,
}

impl<'a> SessionScanner<'a> {
    pub fn new(paths: &'a Paths) -> Self {
        Self { paths, recent_limit: 50 }
    }

    pub fn scan(&self) -> AppResult<UsageSummary> {
        let dir = self.paths.sessions_dir();
        if !dir.exists() {
            return Ok(UsageSummary::default());
        }
        let mut summary = UsageSummary::default();
        summary.sessions_scanned = 0;

        let files: Vec<PathBuf> = WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("jsonl"))
                    .unwrap_or(false)
            })
            .map(|e| e.into_path())
            .collect();

        for file in files {
            summary.sessions_scanned += 1;
            let session_id = session_id_from_path(&file);
            let turns = match parse_session_file(&file, &session_id) {
                Ok(t) => t,
                Err(_) => continue, // skip corrupt files; never panic on user data
            };
            summary.turns_scanned += turns.len();
            for t in turns {
                summary.total_input_tokens += t.input_tokens;
                summary.total_cached_input_tokens += t.cached_input_tokens;
                summary.total_output_tokens += t.output_tokens;
                summary.total_tokens += t.total_tokens;
                upsert_model(&mut summary.per_model, &t);
                push_recent(&mut summary.recent, t, self.recent_limit);
            }
        }

        // Stable order: by total tokens desc.
        summary
            .per_model
            .sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));
        Ok(summary)
    }
}

fn session_id_from_path(p: &Path) -> String {
    // %USERPROFILE%\.codex\sessions\YYYY\MM\DD\<uuid>.jsonl
    // We surface the UUID stem; it's the only stable id we have.
    p.file_stem()
        .and_then(|s| s.to_str())
        .map(String::from)
        .unwrap_or_else(|| "unknown".to_string())
}

fn parse_session_file(path: &Path, session_id: &str) -> AppResult<Vec<SessionTurn>> {
    let raw = std::fs::read_to_string(path)?;
    let mut turns = Vec::new();
    let mut current_model: Option<String> = None;
    let mut current_cwd: Option<String> = None;

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let ty = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match ty {
            "turn_context" => {
                if let Some(payload) = v.get("payload") {
                    current_model = payload
                        .get("model")
                        .and_then(|m| m.as_str())
                        .map(String::from);
                    current_cwd = payload
                        .get("cwd")
                        .and_then(|c| c.as_str())
                        .map(String::from);
                }
            }
            "task_complete" => {
                let ts = parse_timestamp(&v);
                let info = v
                    .get("payload")
                    .and_then(|p| p.get("info"))
                    .and_then(|i| i.get("last_token_usage"));
                let usage = info
                    .and_then(|u| u.get("total_token_usage"))
                    .or_else(|| info);
                let (input, cached, output) = match usage {
                    Some(u) => extract_usage(u),
                    None => (0, 0, 0),
                };
                let model = v
                    .get("payload")
                    .and_then(|p| p.get("model"))
                    .and_then(|m| m.as_str())
                    .map(String::from)
                    .or_else(|| current_model.clone());
                let cwd = v
                    .get("payload")
                    .and_then(|p| p.get("cwd"))
                    .and_then(|c| c.as_str())
                    .map(String::from)
                    .or_else(|| current_cwd.clone());
                let total = input + cached + output;
                if total > 0 || model.is_some() {
                    turns.push(SessionTurn {
                        session_id: session_id.to_string(),
                        timestamp: ts,
                        model,
                        input_tokens: input,
                        cached_input_tokens: cached,
                        output_tokens: output,
                        total_tokens: total,
                        cwd,
                    });
                }
            }
            _ => {}
        }
    }
    Ok(turns)
}

fn parse_timestamp(v: &Value) -> DateTime<Utc> {
    v.get("timestamp")
        .and_then(|t| t.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

fn extract_usage(u: &Value) -> (i64, i64, i64) {
    let input = u.get("input_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
    let cached = u
        .get("cached_input_tokens")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    let output = u.get("output_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
    (input, cached, output)
}

fn upsert_model(per_model: &mut Vec<ModelUsage>, turn: &SessionTurn) {
    let Some(model) = turn.model.as_deref() else { return };
    if let Some(row) = per_model.iter_mut().find(|m| m.model == model) {
        row.input_tokens += turn.input_tokens;
        row.cached_input_tokens += turn.cached_input_tokens;
        row.output_tokens += turn.output_tokens;
        row.total_tokens += turn.total_tokens;
    } else {
        per_model.push(ModelUsage {
            model: model.to_string(),
            input_tokens: turn.input_tokens,
            cached_input_tokens: turn.cached_input_tokens,
            output_tokens: turn.output_tokens,
            total_tokens: turn.total_tokens,
        });
    }
}

fn push_recent(recent: &mut Vec<SessionTurn>, turn: SessionTurn, limit: usize) {
    recent.push(turn);
    // Keep newest `limit` rows by timestamp.
    if recent.len() > limit {
        recent.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        recent.truncate(limit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_session(dir: &Path, name: &str, content: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn scans_task_complete_tokens() {
        let td = tempdir::TempDir::new("codex-runway-session").unwrap();
        let sessions = td.path().join("sessions");
        let line = r#"{"type":"turn_context","payload":{"model":"gpt-5","cwd":"C:\\work"}}
{"type":"task_complete","timestamp":"2026-05-15T10:00:00Z","payload":{"info":{"last_token_usage":{"total_token_usage":{"input_tokens":1200,"cached_input_tokens":300,"output_tokens":450}}}}}
{"type":"task_complete","timestamp":"2026-05-15T10:05:00Z","payload":{"info":{"last_token_usage":{"total_token_usage":{"input_tokens":800,"cached_input_tokens":0,"output_tokens":200}}}}}"#;
        write_session(&sessions, "abc.jsonl", line);

        let p = Paths {
            home: td.path().to_path_buf(),
            codex_home: td.path().join(".codex"),
            app_home: td.path().join(".codex-runway"),
        };
        // Make the scanner find our test file: the scanner walks
        // `paths.sessions_dir()` which is `<home>/.codex/sessions`. Symlink
        // our test dir into the expected location.
        let target = p.sessions_dir();
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&sessions, &target).unwrap();
        #[cfg(windows)]
        std::fs::rename(&sessions, &target).unwrap();

        let scanner = SessionScanner::new(&p);
        let summary = scanner.scan().unwrap();
        assert_eq!(summary.turns_scanned, 2);
        assert_eq!(summary.total_input_tokens, 2000);
        assert_eq!(summary.total_output_tokens, 650);
        assert_eq!(summary.per_model.len(), 1);
        assert_eq!(summary.per_model[0].model, "gpt-5");
    }
}
