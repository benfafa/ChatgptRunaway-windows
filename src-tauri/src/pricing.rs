//! OpenAI / Codex text API price book.
//!
//! This is a frozen snapshot of the public OpenAI pricing page relevant to
//! the models Codex actually uses. We do **not** scrape the live page at
//! runtime — pricing must be deterministic and reproducible, and the live
//! page changes shape too often for an MVP.
//!
//! When a model is not in this table the cost engine returns `None` and
//! the per-model row is flagged as `Unknown` — never invented.

use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    Gpt5,
    Gpt5Mini,
    Gpt5Nano,
    Gpt41,
    Gpt41Mini,
    Gpt41Nano,
    Gpt4o,
    Gpt4oMini,
    Gpt4Turbo,
    O1,
    O1Mini,
    O3,
    O3Mini,
    O4Mini,
    CodexMiniLatest,
    /// Anything starting with `codex-` that we don't model precisely.
    CodexOther,
    /// Any other model we don't have a price for.
    Unknown,
}

impl Model {
    pub fn classify(raw: &str) -> Model {
        let n = raw.trim().to_ascii_lowercase();
        if n.is_empty() {
            return Model::Unknown;
        }
        // Strip a trailing date suffix like "-2025-08-07" if present.
        let stem = strip_date_suffix(&n);
        // We try the most specific match first.
        let dotted = stem.replace('.', "-");
        let candidates = [stem.as_str(), dotted.as_str()];
        for c in candidates {
            match c {
                "gpt-5" => return Model::Gpt5,
                "gpt-5-mini" => return Model::Gpt5Mini,
                "gpt-5-nano" => return Model::Gpt5Nano,
                "gpt-4-1" | "gpt-4-1-mini" | "gpt-4-1-nano" => {
                    return if c == "gpt-4-1" {
                        Model::Gpt41
                    } else if c == "gpt-4-1-mini" {
                        Model::Gpt41Mini
                    } else {
                        Model::Gpt41Nano
                    };
                }
                "gpt-4o" => return Model::Gpt4o,
                "gpt-4o-mini" => return Model::Gpt4oMini,
                "gpt-4-turbo" | "gpt-4-turbo-preview" => return Model::Gpt4Turbo,
                "o1" => return Model::O1,
                "o1-mini" => return Model::O1Mini,
                "o3" => return Model::O3,
                "o3-mini" => return Model::O3Mini,
                "o4-mini" => return Model::O4Mini,
                "codex-mini" | "codex-mini-latest" => return Model::CodexMiniLatest,
                _ => {}
            }
        }
        if stem.starts_with("codex-") {
            Model::CodexOther
        } else {
            Model::Unknown
        }
    }
}

/// If the model id ends with `-YYYY-MM-DD`, drop the date.
fn strip_date_suffix(s: &str) -> String {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() >= 4 {
        let n = parts.len();
        let looks_like_date = parts[n - 3].len() == 4
            && parts[n - 3].chars().all(|c| c.is_ascii_digit())
            && parts[n - 2].len() == 2
            && parts[n - 2].chars().all(|c| c.is_ascii_digit())
            && parts[n - 1].len() == 2
            && parts[n - 1].chars().all(|c| c.is_ascii_digit());
        if looks_like_date {
            return parts[..n - 3].join("-");
        }
    }
    s.to_string()
}

#[derive(Debug, Clone, Copy)]
pub struct Price {
    /// Per 1M uncached input tokens.
    pub input_per_million: Decimal,
    /// Per 1M cached input tokens (defaults to input if unknown).
    pub cached_input_per_million: Decimal,
    /// Per 1M output tokens.
    pub output_per_million: Decimal,
    /// Long-context input price (≥ LONG_CONTEXT_THRESHOLD total prompt tokens).
    pub long_context_input_per_million: Decimal,
    /// Long-context cached input price.
    pub long_context_cached_per_million: Decimal,
    /// Long-context output price.
    pub long_context_output_per_million: Decimal,
}

/// Prompt size at which the long-context tier kicks in. OpenAI's published
/// threshold is 200K total prompt tokens for most current models.
pub const LONG_CONTEXT_THRESHOLD: i64 = 200_000;

pub const PRICING_VERSION: &str = "oai-text-2026-08-15";

pub fn price_for(model: Model) -> Option<Price> {
    let p = table();
    p.get(&model).copied()
}

fn table() -> HashMap<Model, Price> {
    // USD per 1M tokens. All values come from the OpenAI public pricing
    // page; long-context prices use the >200K tier when published.
    let mut t = HashMap::new();
    t.insert(
        Model::Gpt5,
        Price {
            input_per_million: dec("1.25"),
            cached_input_per_million: dec("0.125"),
            output_per_million: dec("10.00"),
            long_context_input_per_million: dec("2.50"),
            long_context_cached_per_million: dec("0.25"),
            long_context_output_per_million: dec("15.00"),
        },
    );
    t.insert(
        Model::Gpt5Mini,
        Price {
            input_per_million: dec("0.25"),
            cached_input_per_million: dec("0.025"),
            output_per_million: dec("2.00"),
            long_context_input_per_million: dec("0.45"),
            long_context_cached_per_million: dec("0.045"),
            long_context_output_per_million: dec("3.60"),
        },
    );
    t.insert(
        Model::Gpt5Nano,
        Price {
            input_per_million: dec("0.05"),
            cached_input_per_million: dec("0.005"),
            output_per_million: dec("0.40"),
            long_context_input_per_million: dec("0.10"),
            long_context_cached_per_million: dec("0.010"),
            long_context_output_per_million: dec("0.72"),
        },
    );
    t.insert(
        Model::Gpt41,
        Price {
            input_per_million: dec("2.50"),
            cached_input_per_million: dec("0.50"),
            output_per_million: dec("10.00"),
            long_context_input_per_million: dec("5.00"),
            long_context_cached_per_million: dec("1.00"),
            long_context_output_per_million: dec("15.00"),
        },
    );
    t.insert(
        Model::Gpt41Mini,
        Price {
            input_per_million: dec("0.40"),
            cached_input_per_million: dec("0.10"),
            output_per_million: dec("1.60"),
            long_context_input_per_million: dec("0.80"),
            long_context_cached_per_million: dec("0.20"),
            long_context_output_per_million: dec("3.20"),
        },
    );
    t.insert(
        Model::Gpt41Nano,
        Price {
            input_per_million: dec("0.10"),
            cached_input_per_million: dec("0.025"),
            output_per_million: dec("0.40"),
            long_context_input_per_million: dec("0.20"),
            long_context_cached_per_million: dec("0.05"),
            long_context_output_per_million: dec("0.80"),
        },
    );
    t.insert(
        Model::Gpt4o,
        Price {
            input_per_million: dec("2.50"),
            cached_input_per_million: dec("1.25"),
            output_per_million: dec("10.00"),
            // gpt-4o has no published long-context tier; reuse standard.
            long_context_input_per_million: dec("2.50"),
            long_context_cached_per_million: dec("1.25"),
            long_context_output_per_million: dec("10.00"),
        },
    );
    t.insert(
        Model::Gpt4oMini,
        Price {
            input_per_million: dec("0.15"),
            cached_input_per_million: dec("0.075"),
            output_per_million: dec("0.60"),
            long_context_input_per_million: dec("0.15"),
            long_context_cached_per_million: dec("0.075"),
            long_context_output_per_million: dec("0.60"),
        },
    );
    t.insert(
        Model::Gpt4Turbo,
        Price {
            input_per_million: dec("10.00"),
            cached_input_per_million: dec("10.00"),
            output_per_million: dec("30.00"),
            long_context_input_per_million: dec("10.00"),
            long_context_cached_per_million: dec("10.00"),
            long_context_output_per_million: dec("30.00"),
        },
    );
    t.insert(
        Model::O1,
        Price {
            input_per_million: dec("15.00"),
            cached_input_per_million: dec("7.50"),
            output_per_million: dec("60.00"),
            long_context_input_per_million: dec("30.00"),
            long_context_cached_per_million: dec("15.00"),
            long_context_output_per_million: dec("120.00"),
        },
    );
    t.insert(
        Model::O1Mini,
        Price {
            input_per_million: dec("3.00"),
            cached_input_per_million: dec("1.50"),
            output_per_million: dec("12.00"),
            long_context_input_per_million: dec("6.00"),
            long_context_cached_per_million: dec("3.00"),
            long_context_output_per_million: dec("24.00"),
        },
    );
    t.insert(
        Model::O3,
        Price {
            input_per_million: dec("10.00"),
            cached_input_per_million: dec("2.50"),
            output_per_million: dec("40.00"),
            long_context_input_per_million: dec("20.00"),
            long_context_cached_per_million: dec("5.00"),
            long_context_output_per_million: dec("80.00"),
        },
    );
    t.insert(
        Model::O3Mini,
        Price {
            input_per_million: dec("1.10"),
            cached_input_per_million: dec("0.55"),
            output_per_million: dec("4.40"),
            long_context_input_per_million: dec("2.20"),
            long_context_cached_per_million: dec("1.10"),
            long_context_output_per_million: dec("8.80"),
        },
    );
    t.insert(
        Model::O4Mini,
        Price {
            input_per_million: dec("1.10"),
            cached_input_per_million: dec("0.275"),
            output_per_million: dec("4.40"),
            long_context_input_per_million: dec("2.20"),
            long_context_cached_per_million: dec("0.55"),
            long_context_output_per_million: dec("8.80"),
        },
    );
    t.insert(
        Model::CodexMiniLatest,
        Price {
            // codex-mini-latest is the new cheap Codex model. Pricing
            // matches gpt-4.1-mini as of 2026-Q3.
            input_per_million: dec("0.40"),
            cached_input_per_million: dec("0.10"),
            output_per_million: dec("1.60"),
            long_context_input_per_million: dec("0.80"),
            long_context_cached_per_million: dec("0.20"),
            long_context_output_per_million: dec("3.20"),
        },
    );
    t
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("static decimal")
}

/// Compute the USD cost of a single turn given its token usage and the
/// observed model. Returns `None` for `Model::Unknown` so the caller can
/// surface it.
pub fn cost_for_turn(model: Model, usage: &super::session::TokenUsage) -> Option<Decimal> {
    let price = price_for(model)?;
    let total_prompt = usage.input_total();
    Some(compute_cost(price, usage, total_prompt))
}

fn compute_cost(price: Price, usage: &super::session::TokenUsage, total_prompt: i64) -> Decimal {
    let (in_p, cached_p, out_p) = if total_prompt >= LONG_CONTEXT_THRESHOLD {
        (
            price.long_context_input_per_million,
            price.long_context_cached_per_million,
            price.long_context_output_per_million,
        )
    } else {
        (
            price.input_per_million,
            price.cached_input_per_million,
            price.output_per_million,
        )
    };
    let per_million = Decimal::from(1_000_000);
    let uncached = Decimal::from(usage.input_tokens.max(0)) / per_million * in_p;
    let cached = Decimal::from(usage.cached_input_tokens.max(0)) / per_million * cached_p;
    let output = Decimal::from(usage.output_tokens.max(0)) / per_million * out_p;
    uncached + cached + output
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn classify_basic() {
        assert_eq!(Model::classify("gpt-5"), Model::Gpt5);
        assert_eq!(Model::classify("gpt-5-mini"), Model::Gpt5Mini);
        assert_eq!(Model::classify("o3-mini"), Model::O3Mini);
        assert_eq!(Model::classify("codex-mini-latest"), Model::CodexMiniLatest);
        assert_eq!(Model::classify("gpt-5-2025-08-07"), Model::Gpt5);
        assert_eq!(Model::classify("foo"), Model::Unknown);
    }

    #[test]
    fn known_models_have_prices() {
        for m in [
            Model::Gpt5,
            Model::Gpt5Mini,
            Model::Gpt4o,
            Model::O1,
            Model::O3Mini,
        ] {
            assert!(price_for(m).is_some(), "{m:?} missing price");
        }
        assert!(price_for(Model::Unknown).is_none());
    }

    #[test]
    fn cost_handles_long_context() {
        // gpt-5: 250K uncached input + 10K output
        // Should pick the long-context tier.
        let usage = super::super::session::TokenUsage {
            input_tokens: 250_000,
            cached_input_tokens: 0,
            output_tokens: 10_000,
        };
        let cost = cost_for_turn(Model::Gpt5, &usage).unwrap();
        // 250K * $2.50/1M = $0.625
        // 10K * $15/1M = $0.15
        // Total = $0.775
        assert_eq!(cost, dec!(0.775));
    }

    #[test]
    fn cost_handles_cached_input() {
        // input_tokens = uncached portion; cached_input_tokens = cache hits.
        // gpt-5: 100K uncached + 80K cached + 1K output.
        let usage = super::super::session::TokenUsage {
            input_tokens: 100_000,
            cached_input_tokens: 80_000,
            output_tokens: 1_000,
        };
        let cost = cost_for_turn(Model::Gpt5, &usage).unwrap();
        // 100K * $1.25/1M = $0.125
        //  80K * $0.125/1M = $0.010
        //   1K * $10.00/1M  = $0.010
        // Total = $0.145
        assert_eq!(cost, dec!(0.145));
    }

    #[test]
    fn unknown_model_returns_none() {
        let usage = super::super::session::TokenUsage {
            input_tokens: 100,
            cached_input_tokens: 0,
            output_tokens: 100,
        };
        assert!(cost_for_turn(Model::Unknown, &usage).is_none());
    }
}
