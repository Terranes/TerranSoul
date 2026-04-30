//! Persistent observability for the self-improve engine.
//!
//! Records every planning attempt to an append-only JSONL log
//! (`self_improve_runs.jsonl` in the app data dir). Designed for crash
//! safety: each run is one line, written as a single `write_all` after
//! `serde_json::to_string`, so a SIGKILL mid-write loses *at most* the
//! current run rather than corrupting prior history.
//!
//! The engine drives this through three calls per cycle:
//! 1. [`MetricsLog::record_start`] — emits a `running` entry with start time.
//! 2. [`MetricsLog::record_outcome`] — emits a `success` or `failure` entry.
//! 3. [`MetricsLog::summary`] — used by the UI to compute success/fail rates.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_FILE: &str = "self_improve_runs.jsonl";
/// Hard cap on the number of run records kept in memory / returned to UI.
/// The on-disk JSONL grows monotonically; readers truncate to the most
/// recent N. Set high enough for forensics, low enough to bound RAM.
pub const MAX_RECENT_RUNS: usize = 500;

/// One persisted run record. `outcome = "running"` means a plan started
/// but no terminal record has been written yet — the engine will append a
/// `success` or `failure` row when the plan resolves.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunRecord {
    /// Unix-epoch milliseconds.
    pub started_at_ms: u64,
    /// Unix-epoch milliseconds; equals `started_at_ms` for `running` rows.
    pub finished_at_ms: u64,
    pub chunk_id: String,
    pub chunk_title: String,
    /// `"running" | "success" | "failure"`.
    pub outcome: String,
    /// Latency milliseconds (0 for `running`).
    pub duration_ms: u64,
    /// Provider snapshot (audit trail). Never includes the API key.
    pub provider: String,
    pub model: String,
    /// Length of the plan text on success; 0 on failure.
    pub plan_chars: usize,
    /// Error message on failure; `None` otherwise. Truncated to 1 KB
    /// before persisting to keep log lines bounded.
    pub error: Option<String>,
    /// Prompt-side token count reported by the LLM provider, if any.
    /// `None` for older log lines (pre-Chunk 28.5) and for providers
    /// that don't return usage metadata.
    #[serde(default)]
    pub prompt_tokens: Option<u64>,
    /// Completion-side token count reported by the LLM provider, if any.
    #[serde(default)]
    pub completion_tokens: Option<u64>,
    /// Estimated USD cost of the run, computed from token counts and
    /// the per-provider price catalogue in [`crate::coding::cost`].
    /// `None` when token counts are unavailable; `Some(0.0)` for
    /// local Ollama (free) so the UI can distinguish "free" from
    /// "unknown".
    #[serde(default)]
    pub cost_usd: Option<f64>,
}

/// Aggregate stats computed from a list of [`RunRecord`]s. All counts
/// exclude `running` (in-flight) records.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricsSummary {
    pub total_runs: usize,
    pub successes: usize,
    pub failures: usize,
    /// 0.0–1.0; `0.0` when `total_runs == 0`.
    pub success_rate: f64,
    /// 0.0–1.0; `0.0` when `total_runs == 0`.
    pub failure_rate: f64,
    /// Mean duration of completed runs (success + failure), in milliseconds.
    pub avg_duration_ms: u64,
    /// Most recent failure's error string, if any.
    pub last_error: Option<String>,
    /// Most recent failure's chunk id, paired with `last_error`.
    pub last_error_chunk: Option<String>,
    /// Unix-epoch ms of the most recent failure.
    pub last_error_at_ms: u64,
    /// Sum of all `prompt_tokens` across completed runs (Chunk 28.5).
    /// `0` when no run reported usage. `successes + failures` are both
    /// counted because token spend is incurred regardless of outcome.
    pub total_prompt_tokens: u64,
    /// Sum of all `completion_tokens` across completed runs.
    pub total_completion_tokens: u64,
    /// Sum of all `cost_usd` across completed runs.
    pub total_cost_usd: f64,
    /// Same totals as above, but limited to runs that finished within
    /// the last 7 days (relative to whichever epoch the caller passes
    /// to [`summarise_with_now`]). Lets the UI show a rolling spend
    /// gauge that auto-decays as old runs age out of the window.
    pub rolling_7d_runs: usize,
    pub rolling_7d_prompt_tokens: u64,
    pub rolling_7d_completion_tokens: u64,
    pub rolling_7d_cost_usd: f64,
    /// Per-provider cost breakdown over the full window. Keys are the
    /// `provider` field of [`RunRecord`] (e.g. `"anthropic"`, `"custom"`).
    /// `BTreeMap` keeps the JSON deterministic for snapshot tests.
    pub cost_by_provider: BTreeMap<String, f64>,
}

/// Append-only log handle. Cheap to clone (just a path).
#[derive(Debug, Clone)]
pub struct MetricsLog {
    path: PathBuf,
}

impl MetricsLog {
    pub fn new(data_dir: &Path) -> Self {
        Self { path: data_dir.join(LOG_FILE) }
    }

    /// Append a `running` row when a plan cycle begins. Returns the
    /// timestamp the engine should pass back into [`record_outcome`] so
    /// duration math is consistent across both rows.
    pub fn record_start(
        &self,
        chunk_id: &str,
        chunk_title: &str,
        provider: &str,
        model: &str,
    ) -> u64 {
        let now = now_ms();
        let rec = RunRecord {
            started_at_ms: now,
            finished_at_ms: now,
            chunk_id: chunk_id.to_string(),
            chunk_title: chunk_title.to_string(),
            outcome: "running".to_string(),
            duration_ms: 0,
            provider: provider.to_string(),
            model: model.to_string(),
            plan_chars: 0,
            error: None,
            prompt_tokens: None,
            completion_tokens: None,
            cost_usd: None,
        };
        let _ = self.append(&rec);
        now
    }

    /// Append a terminal row (success or failure).
    ///
    /// `usage` carries token counts from the LLM provider when
    /// available — pass `TokenUsage::default()` for providers that
    /// don't expose usage metadata. Cost is computed automatically
    /// from `(provider, model, usage)` via [`crate::coding::cost`].
    #[allow(clippy::too_many_arguments)]
    pub fn record_outcome(
        &self,
        started_at_ms: u64,
        chunk_id: &str,
        chunk_title: &str,
        provider: &str,
        model: &str,
        success: bool,
        plan_chars: usize,
        error: Option<&str>,
        usage: TokenUsage,
    ) {
        let finished = now_ms();
        let cost_usd = usage.cost_usd_for(provider, model);
        let rec = RunRecord {
            started_at_ms,
            finished_at_ms: finished,
            chunk_id: chunk_id.to_string(),
            chunk_title: chunk_title.to_string(),
            outcome: if success { "success".to_string() } else { "failure".to_string() },
            duration_ms: finished.saturating_sub(started_at_ms),
            provider: provider.to_string(),
            model: model.to_string(),
            plan_chars,
            error: error.map(truncate_error),
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            cost_usd,
        };
        let _ = self.append(&rec);
    }

    fn append(&self, rec: &RunRecord) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string(rec)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        f.write_all(json.as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
    }

    /// Read the most recent `limit` records (newest first). Bad lines are
    /// skipped silently — partial-write tolerance.
    pub fn recent(&self, limit: usize) -> Vec<RunRecord> {
        let f = match std::fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let mut out: Vec<RunRecord> = BufReader::new(f)
            .lines()
            .map_while(|l| l.ok())
            .filter_map(|l| serde_json::from_str::<RunRecord>(&l).ok())
            .collect();
        // Newest first.
        out.reverse();
        if out.len() > limit {
            out.truncate(limit);
        }
        out
    }

    /// Compute the rolled-up summary from the most recent runs (capped
    /// at [`MAX_RECENT_RUNS`] to bound work).
    pub fn summary(&self) -> MetricsSummary {
        let recs = self.recent(MAX_RECENT_RUNS);
        summarise_with_now(&recs, now_ms())
    }

    /// Wipe the log. Used by the UI's "Clear log" button. Returns ok even
    /// when the file was already absent.
    pub fn clear(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// Pure summariser — kept out of the I/O struct so tests can verify the
/// math against synthetic record vectors without touching the disk.
///
/// Uses [`now_ms`] for the rolling-window cutoff. For deterministic
/// tests, prefer [`summarise_with_now`].
pub fn summarise(records: &[RunRecord]) -> MetricsSummary {
    summarise_with_now(records, now_ms())
}

/// Same as [`summarise`] but takes an explicit "now" so tests can pin
/// the 7-day rolling window deterministically.
pub fn summarise_with_now(records: &[RunRecord], now_epoch_ms: u64) -> MetricsSummary {
    let mut s = MetricsSummary::default();
    let mut total_duration: u64 = 0;
    // 7 days in ms — the rolling window is hard-coded because that's
    // what every observability dashboard the engine ships uses.
    const WINDOW_MS: u64 = 7 * 24 * 60 * 60 * 1000;
    let cutoff = now_epoch_ms.saturating_sub(WINDOW_MS);
    for r in records {
        let is_terminal = matches!(r.outcome.as_str(), "success" | "failure");
        if !is_terminal {
            continue;
        }
        s.total_runs += 1;
        match r.outcome.as_str() {
            "success" => s.successes += 1,
            "failure" => {
                s.failures += 1;
                // `records` is newest-first, so the first failure we
                // encounter is the most recent one.
                if s.last_error.is_none() {
                    s.last_error = r.error.clone();
                    s.last_error_chunk = Some(r.chunk_id.clone());
                    s.last_error_at_ms = r.finished_at_ms;
                }
            }
            _ => {}
        }
        total_duration += r.duration_ms;
        if let Some(p) = r.prompt_tokens {
            s.total_prompt_tokens = s.total_prompt_tokens.saturating_add(p);
        }
        if let Some(c) = r.completion_tokens {
            s.total_completion_tokens = s.total_completion_tokens.saturating_add(c);
        }
        if let Some(c) = r.cost_usd {
            s.total_cost_usd += c;
            *s.cost_by_provider.entry(r.provider.clone()).or_insert(0.0) += c;
        }
        if r.finished_at_ms >= cutoff {
            s.rolling_7d_runs += 1;
            if let Some(p) = r.prompt_tokens {
                s.rolling_7d_prompt_tokens =
                    s.rolling_7d_prompt_tokens.saturating_add(p);
            }
            if let Some(c) = r.completion_tokens {
                s.rolling_7d_completion_tokens =
                    s.rolling_7d_completion_tokens.saturating_add(c);
            }
            if let Some(c) = r.cost_usd {
                s.rolling_7d_cost_usd += c;
            }
        }
    }
    if s.total_runs > 0 {
        s.success_rate = s.successes as f64 / s.total_runs as f64;
        s.failure_rate = s.failures as f64 / s.total_runs as f64;
        s.avg_duration_ms = total_duration / s.total_runs as u64;
    }
    // Round summed cost values to 8 dp so the UI doesn't show
    // floating-point drift like `0.30000000000000004`.
    s.total_cost_usd = round8(s.total_cost_usd);
    s.rolling_7d_cost_usd = round8(s.rolling_7d_cost_usd);
    for v in s.cost_by_provider.values_mut() {
        *v = round8(*v);
    }
    s
}

fn round8(x: f64) -> f64 {
    (x * 1e8).round() / 1e8
}

/// Token usage reported by the LLM provider for a single run.
///
/// All fields default to `None`/zero so callers that don't have usage
/// data can simply pass `TokenUsage::default()`. The `cost_usd_for`
/// method computes the per-run dollar cost using the [`cost`] catalogue.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
}

impl TokenUsage {
    pub fn new(prompt: u64, completion: u64) -> Self {
        Self {
            prompt_tokens: Some(prompt),
            completion_tokens: Some(completion),
        }
    }

    /// Returns the estimated USD cost, or `None` when both token counts
    /// are missing. Provider strings are matched case-insensitively
    /// against [`crate::coding::CodingLlmProvider`] variants.
    pub fn cost_usd_for(&self, provider: &str, model: &str) -> Option<f64> {
        let prompt = self.prompt_tokens?;
        let completion = self.completion_tokens.unwrap_or(0);
        let provider_enum = parse_provider(provider)?;
        Some(crate::coding::cost::estimate_cost_usd(
            &provider_enum,
            model,
            prompt,
            completion,
        ))
    }
}

fn parse_provider(s: &str) -> Option<crate::coding::CodingLlmProvider> {
    use crate::coding::CodingLlmProvider as P;
    match s.to_ascii_lowercase().as_str() {
        "anthropic" => Some(P::Anthropic),
        "openai" => Some(P::Openai),
        "deepseek" => Some(P::Deepseek),
        // Treat ollama / local / unknown as Custom (free).
        "custom" | "ollama" | "local" => Some(P::Custom),
        _ => Some(P::Custom),
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn truncate_error(e: &str) -> String {
    const MAX: usize = 1024;
    if e.len() <= MAX {
        e.to_string()
    } else {
        let mut out = e.chars().take(MAX).collect::<String>();
        out.push_str("…[truncated]");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn rec(outcome: &str, chunk: &str, err: Option<&str>, dur: u64) -> RunRecord {
        RunRecord {
            started_at_ms: 1000,
            finished_at_ms: 1000 + dur,
            chunk_id: chunk.to_string(),
            chunk_title: format!("Chunk {chunk}"),
            outcome: outcome.to_string(),
            duration_ms: dur,
            provider: "ollama".to_string(),
            model: "gemma3:4b".to_string(),
            plan_chars: if outcome == "success" { 200 } else { 0 },
            error: err.map(|s| s.to_string()),
            prompt_tokens: None,
            completion_tokens: None,
            cost_usd: None,
        }
    }

    /// Variant of [`rec`] that tags a run with token usage and a fixed
    /// timestamp — used by the cost / rolling-window tests below.
    fn rec_with_usage(
        outcome: &str,
        chunk: &str,
        provider: &str,
        model: &str,
        prompt: u64,
        completion: u64,
        cost: f64,
        finished_at_ms: u64,
    ) -> RunRecord {
        RunRecord {
            started_at_ms: finished_at_ms.saturating_sub(100),
            finished_at_ms,
            chunk_id: chunk.to_string(),
            chunk_title: format!("Chunk {chunk}"),
            outcome: outcome.to_string(),
            duration_ms: 100,
            provider: provider.to_string(),
            model: model.to_string(),
            plan_chars: 100,
            error: None,
            prompt_tokens: Some(prompt),
            completion_tokens: Some(completion),
            cost_usd: Some(cost),
        }
    }

    #[test]
    fn summary_zero_runs_yields_default() {
        let s = summarise(&[]);
        assert_eq!(s.total_runs, 0);
        assert_eq!(s.success_rate, 0.0);
        assert_eq!(s.failure_rate, 0.0);
        assert!(s.last_error.is_none());
    }

    #[test]
    fn summary_success_and_failure_rates() {
        // newest-first: 2 successes, 1 failure
        let recs = vec![
            rec("success", "1.1", None, 1000),
            rec("failure", "1.2", Some("network down"), 500),
            rec("success", "1.3", None, 2000),
        ];
        let s = summarise(&recs);
        assert_eq!(s.total_runs, 3);
        assert_eq!(s.successes, 2);
        assert_eq!(s.failures, 1);
        assert!((s.success_rate - 2.0 / 3.0).abs() < 1e-9);
        assert!((s.failure_rate - 1.0 / 3.0).abs() < 1e-9);
        assert_eq!(s.avg_duration_ms, (1000 + 500 + 2000) / 3);
        assert_eq!(s.last_error.as_deref(), Some("network down"));
        assert_eq!(s.last_error_chunk.as_deref(), Some("1.2"));
    }

    #[test]
    fn summary_ignores_running_rows() {
        let recs = vec![
            rec("running", "x", None, 0),
            rec("success", "x", None, 100),
        ];
        let s = summarise(&recs);
        assert_eq!(s.total_runs, 1);
    }

    #[test]
    fn metrics_log_round_trip_persists_and_reads_back() {
        let dir = tempdir().unwrap();
        let log = MetricsLog::new(dir.path());
        let started = log.record_start("25.4", "Engine MVP", "ollama", "gemma3:4b");
        log.record_outcome(
            started,
            "25.4",
            "Engine MVP",
            "ollama",
            "gemma3:4b",
            true,
            512,
            None,
            TokenUsage::default(),
        );
        let recs = log.recent(50);
        // newest first → success row at index 0, running row at index 1
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].outcome, "success");
        assert_eq!(recs[1].outcome, "running");
        let s = log.summary();
        assert_eq!(s.successes, 1);
        assert_eq!(s.failures, 0);
    }

    #[test]
    fn metrics_log_clear_removes_history() {
        let dir = tempdir().unwrap();
        let log = MetricsLog::new(dir.path());
        log.record_start("a", "t", "p", "m");
        assert!(!log.recent(10).is_empty());
        log.clear().unwrap();
        assert!(log.recent(10).is_empty());
        // Idempotent.
        log.clear().unwrap();
    }

    #[test]
    fn truncate_error_caps_long_messages() {
        let huge = "x".repeat(5000);
        let t = truncate_error(&huge);
        assert!(t.len() <= 1024 + "…[truncated]".len());
        assert!(t.ends_with("[truncated]"));
    }

    #[test]
    fn recent_skips_corrupt_lines() {
        let dir = tempdir().unwrap();
        let log = MetricsLog::new(dir.path());
        log.record_start("x", "t", "p", "m");
        // Inject a bad line.
        let mut f = OpenOptions::new().append(true).open(&log.path).unwrap();
        f.write_all(b"{ this is not json\n").unwrap();
        log.record_start("y", "t", "p", "m");
        let recs = log.recent(10);
        // Two valid rows survive, the bad line is skipped.
        assert_eq!(recs.len(), 2);
    }

    #[test]
    fn recent_caps_at_limit() {
        let dir = tempdir().unwrap();
        let log = MetricsLog::new(dir.path());
        for i in 0..10 {
            log.record_start(&format!("c{i}"), "t", "p", "m");
        }
        let recs = log.recent(3);
        assert_eq!(recs.len(), 3);
        // Newest first.
        assert_eq!(recs[0].chunk_id, "c9");
    }

    // ---- Chunk 28.5 — token / cost telemetry ----

    #[test]
    fn token_usage_cost_zero_for_local_custom() {
        let usage = TokenUsage::new(1_000, 500);
        let cost = usage.cost_usd_for("custom", "gemma3:4b").unwrap();
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn token_usage_cost_anthropic_sonnet() {
        let usage = TokenUsage::new(1_000_000, 0);
        let cost = usage.cost_usd_for("anthropic", "claude-sonnet-4-5").unwrap();
        // 1M prompt @ $3.00/M = $3.00
        assert!((cost - 3.00).abs() < 1e-6);
    }

    #[test]
    fn token_usage_cost_returns_none_when_prompt_missing() {
        let usage = TokenUsage::default();
        assert!(usage.cost_usd_for("anthropic", "claude-sonnet-4-5").is_none());
    }

    #[test]
    fn summary_aggregates_total_tokens_and_cost() {
        let now = 10_000_000_000u64;
        let recs = vec![
            rec_with_usage("success", "a", "anthropic", "claude-sonnet-4-5", 1000, 500, 0.0105, now),
            rec_with_usage("success", "b", "anthropic", "claude-sonnet-4-5", 2000, 1000, 0.021, now - 1000),
            rec_with_usage("failure", "c", "openai", "gpt-4o-mini", 500, 100, 0.000135, now - 2000),
        ];
        let s = summarise_with_now(&recs, now);
        assert_eq!(s.total_runs, 3);
        assert_eq!(s.total_prompt_tokens, 3500);
        assert_eq!(s.total_completion_tokens, 1600);
        // Sum of costs, rounded to 8 dp.
        assert!((s.total_cost_usd - (0.0105 + 0.021 + 0.000135)).abs() < 1e-6);
        // Per-provider breakdown.
        let anth = s.cost_by_provider.get("anthropic").copied().unwrap_or(0.0);
        let oai = s.cost_by_provider.get("openai").copied().unwrap_or(0.0);
        assert!((anth - 0.0315).abs() < 1e-6);
        assert!((oai - 0.000135).abs() < 1e-6);
    }

    #[test]
    fn summary_rolling_7d_excludes_old_runs() {
        let now = 10_000_000_000u64;
        let day_ms = 24 * 60 * 60 * 1000;
        let recs = vec![
            // Inside window.
            rec_with_usage("success", "new", "anthropic", "claude-sonnet-4-5", 1000, 500, 0.01, now - day_ms),
            // Outside window (10 days old).
            rec_with_usage("success", "old", "anthropic", "claude-sonnet-4-5", 9999, 9999, 99.99, now - 10 * day_ms),
        ];
        let s = summarise_with_now(&recs, now);
        // Both runs counted in totals.
        assert_eq!(s.total_runs, 2);
        // Only the recent one in the rolling window.
        assert_eq!(s.rolling_7d_runs, 1);
        assert_eq!(s.rolling_7d_prompt_tokens, 1000);
        assert!((s.rolling_7d_cost_usd - 0.01).abs() < 1e-6);
    }

    #[test]
    fn summary_handles_records_without_usage() {
        // Mix old-format (no usage) and new-format records.
        let recs = vec![
            rec("success", "old1", None, 100),
            rec_with_usage("success", "new1", "anthropic", "claude-haiku", 100, 50, 0.0003, 5_000_000),
        ];
        let s = summarise_with_now(&recs, 5_000_000);
        assert_eq!(s.total_runs, 2);
        // Only the tagged record contributes tokens.
        assert_eq!(s.total_prompt_tokens, 100);
        assert_eq!(s.total_completion_tokens, 50);
        assert!((s.total_cost_usd - 0.0003).abs() < 1e-6);
    }

    #[test]
    fn record_outcome_persists_token_counts_and_cost() {
        let dir = tempdir().unwrap();
        let log = MetricsLog::new(dir.path());
        let started = log.record_start("x", "t", "anthropic", "claude-sonnet-4-5");
        log.record_outcome(
            started,
            "x",
            "t",
            "anthropic",
            "claude-sonnet-4-5",
            true,
            100,
            None,
            TokenUsage::new(1_000_000, 0),
        );
        let recs = log.recent(10);
        let success = recs.iter().find(|r| r.outcome == "success").unwrap();
        assert_eq!(success.prompt_tokens, Some(1_000_000));
        assert_eq!(success.completion_tokens, Some(0));
        // 1M prompt @ $3 = $3.00.
        assert!((success.cost_usd.unwrap() - 3.00).abs() < 1e-6);
    }

    #[test]
    fn run_record_serde_back_compat_old_log_lines() {
        // Older JSONL lines (pre-Chunk 28.5) didn't have the three
        // new optional fields. They must still deserialise cleanly.
        let old_json = r#"{
            "started_at_ms": 1,
            "finished_at_ms": 2,
            "chunk_id": "x",
            "chunk_title": "t",
            "outcome": "success",
            "duration_ms": 1,
            "provider": "ollama",
            "model": "gemma3:4b",
            "plan_chars": 0,
            "error": null
        }"#;
        let r: RunRecord = serde_json::from_str(old_json).expect("old log line must parse");
        assert!(r.prompt_tokens.is_none());
        assert!(r.completion_tokens.is_none());
        assert!(r.cost_usd.is_none());
    }
}
