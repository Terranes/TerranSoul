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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
        };
        let _ = self.append(&rec);
        now
    }

    /// Append a terminal row (success or failure).
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
    ) {
        let finished = now_ms();
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
        summarise(&recs)
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
pub fn summarise(records: &[RunRecord]) -> MetricsSummary {
    let mut s = MetricsSummary::default();
    let mut total_duration: u64 = 0;
    for r in records {
        match r.outcome.as_str() {
            "success" => {
                s.total_runs += 1;
                s.successes += 1;
                total_duration += r.duration_ms;
            }
            "failure" => {
                s.total_runs += 1;
                s.failures += 1;
                total_duration += r.duration_ms;
                // `records` is newest-first, so the first failure we
                // encounter is the most recent one.
                if s.last_error.is_none() {
                    s.last_error = r.error.clone();
                    s.last_error_chunk = Some(r.chunk_id.clone());
                    s.last_error_at_ms = r.finished_at_ms;
                }
            }
            _ => {} // ignore "running"
        }
    }
    if s.total_runs > 0 {
        s.success_rate = s.successes as f64 / s.total_runs as f64;
        s.failure_rate = s.failures as f64 / s.total_runs as f64;
        s.avg_duration_ms = total_duration / s.total_runs as u64;
    }
    s
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
}
