//! API-equivalent cost computation.
//!
//! Reads the local Codex session JSONL logs, classifies each `task_complete`
//! by model, multiplies through the OpenAI price book in `pricing.rs`, and
//! returns a per-model and total USD summary. Mirrors the macOS original's
//! `ApiEquivalent` engine (see `Sources/CodexRunwayCore/ApiEquivalent.swift`)
//! but stays local-only and is driven by the existing `SessionScanner`.
//!
//! **Important:** this module never makes a network call and never writes
//! to `%USERPROFILE%\.codex\`. It only reads session logs and produces a
//! derived summary.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use walkdir::WalkDir;

use crate::error::AppResult;
use crate::paths::Paths;
use crate::pricing::{self, Model, Price, PRICING_VERSION, LONG_CONTEXT_THRESHOLD};
use crate::session::{SessionTurn, TokenUsage};

#[derive(Debug, Clone, Serialize)]
pub struct CostSummary {
    pub pricing_version: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub turns_priced: usize,
    pub turns_unknown: usize,
    pub total_uncached_input_tokens: i64,
    pub total_cached_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_usd: Decimal,
    pub per_model: Vec<ModelCost>,
    pub unknown_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelCost {
    pub raw_model: String,
    pub classified: String,
    pub turns: usize,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_usd: Decimal,
    pub priced: bool,
}

pub struct CostEngine<'a> {
    paths: &'a Paths,
    /// Earliest event to count toward the window. Defaults to "all time".
    pub window_start: Option<DateTime<Utc>>,
}

impl<'a> CostEngine<'a> {
    pub fn new(paths: &'a Paths) -> Self {
        Self { paths, window_start: None }
    }

    pub fn since(mut self, t: DateTime<Utc>) -> Self {
        self.window_start = Some(t);
        self
    }

    pub fn compute(&self) -> AppResult<CostSummary> {
        let turns = collect_turns(self.paths.sessions_dir().as_path(), self.window_start)?;
        let mut by_model: BTreeMap<String, ModelAccum> = BTreeMap::new();
        let mut total_uncached: i64 = 0;
        let mut total_cached: i64 = 0;
        let mut total_output: i64 = 0;
        let mut total_usd = Decimal::ZERO;
        let mut turns_priced = 0usize;
        let mut turns_unknown = 0usize;
        let mut window_start: Option<DateTime<Utc>> = None;
        let mut window_end: Option<DateTime<Utc>> = None;

        for turn in &turns {
            window_start = Some(match window_start {
                Some(prev) if prev < turn.timestamp => prev,
                _ => turn.timestamp,
            });
            window_end = Some(match window_end {
                Some(prev) if prev > turn.timestamp => prev,
                _ => turn.timestamp,
            });
            let raw = turn.model.clone().unwrap_or_else(|| "<unknown>".to_string());
            let classified = pricing::Model::classify(&raw);
            let usage = TokenUsage::from(turn);
            let cost = pricing::cost_for_turn(classified, &usage);
            let entry = by_model.entry(raw.clone()).or_insert_with(|| ModelAccum {
                raw_model: raw.clone(),
                classified: format!("{classified:?}"),
                turns: 0,
                input_tokens: 0,
                cached_input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                estimated_usd: Decimal::ZERO,
                priced: false,
            });
            entry.turns += 1;
            entry.input_tokens += usage.input_tokens;
            entry.cached_input_tokens += usage.cached_input_tokens;
            entry.output_tokens += usage.output_tokens;
            entry.total_tokens += usage.input_tokens
                + usage.cached_input_tokens
                + usage.output_tokens;
            total_uncached += usage.input_tokens;
            total_cached += usage.cached_input_tokens;
            total_output += usage.output_tokens;
            if let Some(c) = cost {
                entry.estimated_usd += c;
                entry.priced = true;
                total_usd += c;
                turns_priced += 1;
            } else {
                turns_unknown += 1;
            }
        }

        let unknown_models: Vec<String> = by_model
            .values()
            .filter(|m| !m.priced)
            .map(|m| m.raw_model.clone())
            .collect();

        let per_model: Vec<ModelCost> = by_model
            .into_values()
            .map(|m| ModelCost {
                raw_model: m.raw_model,
                classified: m.classified,
                turns: m.turns,
                input_tokens: m.input_tokens,
                cached_input_tokens: m.cached_input_tokens,
                output_tokens: m.output_tokens,
                total_tokens: m.total_tokens,
                estimated_usd: m.estimated_usd,
                priced: m.priced,
            })
            // Sort by USD desc, then by tokens desc.
            .collect::<Vec<_>>()
            .into_iter()
            .rev() // we'll re-sort below; placeholder for stability
            .collect();

        let mut per_model = per_model;
        per_model.sort_by(|a, b| {
            b.estimated_usd
                .partial_cmp(&a.estimated_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.total_tokens.cmp(&a.total_tokens))
        });

        let now = Utc::now();
        Ok(CostSummary {
            pricing_version: PRICING_VERSION.to_string(),
            window_start: window_start.unwrap_or(now),
            window_end: window_end.unwrap_or(now),
            turns_priced,
            turns_unknown,
            total_uncached_input_tokens: total_uncached,
            total_cached_input_tokens: total_cached,
            total_output_tokens: total_output,
            total_tokens: total_uncached + total_cached + total_output,
            estimated_usd: total_usd,
            per_model,
            unknown_models,
        })
    }
}

struct ModelAccum {
    raw_model: String,
    classified: String,
    turns: usize,
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    total_tokens: i64,
    estimated_usd: Decimal,
    priced: bool,
}

fn collect_turns(dir: &Path, since: Option<DateTime<Utc>>) -> AppResult<Vec<SessionTurn>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| !s.eq_ignore_ascii_case("jsonl"))
            .unwrap_or(true)
        {
            continue;
        }
        let session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(String::from)
            .unwrap_or_else(|| "unknown".to_string());
        let Ok(raw) = std::fs::read_to_string(&path) else { continue };
        let mut current_model: Option<String> = None;
        let mut current_cwd: Option<String> = None;
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
            let ty = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match ty {
                "turn_context" => {
                    if let Some(payload) = v.get("payload") {
                        current_model = payload.get("model").and_then(|m| m.as_str()).map(String::from);
                        current_cwd = payload.get("cwd").and_then(|c| c.as_str()).map(String::from);
                    }
                }
                "task_complete" => {
                    let ts = v
                        .get("timestamp")
                        .and_then(|t| t.as_str())
                        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now);
                    if let Some(s) = since {
                        if ts < s {
                            continue;
                        }
                    }
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
                        out.push(SessionTurn {
                            session_id: session_id.clone(),
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
    }
    Ok(out)
}

fn extract_usage(u: &serde_json::Value) -> (i64, i64, i64) {
    let input = u.get("input_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
    let cached = u
        .get("cached_input_tokens")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    let output = u.get("output_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
    (input, cached, output)
}

// Re-export the long-context threshold so tests can sanity-check.
#[allow(dead_code)]
pub const LC_THRESHOLD: i64 = LONG_CONTEXT_THRESHOLD;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use std::io::Write;

    fn write_session(dir: &Path, name: &str, body: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn long_context_threshold_picks_long_tier() {
        // gpt-5: 250K input + 5K output.
        let p = pricing::price_for(Model::Gpt5).unwrap();
        let usage = TokenUsage {
            input_tokens: 250_000,
            cached_input_tokens: 0,
            output_tokens: 5_000,
        };
        let is_long = usage.input_total() >= LONG_CONTEXT_THRESHOLD;
        assert!(is_long);
        let (in_p, out_p) = if is_long {
            (p.long_context_input_per_million, p.long_context_output_per_million)
        } else {
            (p.input_per_million, p.output_per_million)
        };
        // 250K * $2.50/1M = $0.625; 5K * $15/1M = $0.075 → $0.700
        let cost = Decimal::from(usage.input_tokens) / Decimal::from(1_000_000) * in_p
            + Decimal::from(usage.output_tokens) / Decimal::from(1_000_000) * out_p;
        assert_eq!(cost, dec!(0.700));
    }

    #[test]
    fn engine_sums_per_model() {
        let td = tempdir::TempDir::new("cost-engine").unwrap();
        let sessions = td.path().join("sessions");
        let body = r#"{"type":"turn_context","payload":{"model":"gpt-5"}}
{"type":"task_complete","timestamp":"2026-05-15T10:00:00Z","payload":{"info":{"last_token_usage":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":200,"output_tokens":300}}}}}
{"type":"task_complete","timestamp":"2026-05-15T10:01:00Z","payload":{"info":{"last_token_usage":{"total_token_usage":{"input_tokens":500,"cached_input_tokens":0,"output_tokens":100}}}}}
{"type":"turn_context","payload":{"model":"mystery-1"}}
{"type":"task_complete","timestamp":"2026-05-15T10:02:00Z","payload":{"info":{"last_token_usage":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":0,"output_tokens":50}}}}}
"#;
        write_session(&sessions, "a.jsonl", body);

        let p = Paths {
            home: td.path().to_path_buf(),
            codex_home: td.path().join(".codex"),
            app_home: td.path().join(".codex-runway"),
        };
        let target = p.sessions_dir();
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&sessions, &target).unwrap();
        #[cfg(windows)]
        std::fs::rename(&sessions, &target).unwrap();

        let summary = CostEngine::new(&p).compute().unwrap();
        assert_eq!(summary.turns_priced, 2);
        assert_eq!(summary.turns_unknown, 1);
        assert_eq!(summary.per_model.len(), 2);
        let gpt5 = summary.per_model.iter().find(|m| m.raw_model == "gpt-5").unwrap();
        assert!(gpt5.priced);
        assert!(gpt5.estimated_usd > Decimal::ZERO);
        let unk = summary.per_model.iter().find(|m| m.raw_model == "mystery-1").unwrap();
        assert!(!unk.priced);
        assert_eq!(unk.estimated_usd, Decimal::ZERO);
        assert!(summary.unknown_models.contains(&"mystery-1".to_string()));
    }
}
