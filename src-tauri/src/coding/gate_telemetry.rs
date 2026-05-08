//! Gate-level telemetry for the self-improve coding workflow.
//!
//! Each gate (plan, code, review, apply, test, stage) emits a
//! [`GateEvent`] at start and at completion. Events are:
//! 1. Persisted to `self_improve_gates.jsonl` with current + `.001` rotation.
//! 2. Emitted as a Tauri event (`"self-improve-gate"`) for live frontend
//!    display of the active gate and per-gate timing.
//!
//! The frontend panel can then show:
//! - Which gate is currently executing (phase dot/highlight).
//! - Per-gate pass/fail ratio and average duration.
//! - The last N gate transitions per session.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const GATE_LOG_FILE: &str = "self_improve_gates.jsonl";

/// Maximum gate records retained in-memory when computing summaries.
pub const MAX_GATE_RECORDS: usize = 2000;

// ---------------------------------------------------------------------------
// Gate names (stable string identifiers)
// ---------------------------------------------------------------------------

/// All possible gate names in execution order.
pub const GATE_NAMES: &[&str] = &[
    "context_load",
    "plan",
    "code",
    "review",
    "apply",
    "test",
    "stage",
    "archive",
];

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

/// Result of a single gate execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateResult {
    /// Gate completed successfully.
    Pass,
    /// Gate completed with degraded quality (e.g. review accepted with warnings).
    Partial,
    /// Gate failed; downstream gates skipped.
    Fail,
}

/// A single gate telemetry event. One is written at gate-start (with
/// `result = None`, `duration_ms = 0`) and one at gate-end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateEvent {
    /// Unix-epoch milliseconds.
    pub ts: u64,
    /// Stable gate identifier (one of [`GATE_NAMES`]).
    pub gate: String,
    /// Self-improve session/chunk this gate belongs to.
    pub session_id: String,
    /// Chunk id (e.g. "34.2") from milestones.md.
    pub chunk_id: String,
    /// `"start"` or `"end"`.
    pub event_type: String,
    /// Gate outcome; `None` for `start` events.
    pub result: Option<GateResult>,
    /// Wall-clock duration in milliseconds (0 for start events).
    pub duration_ms: u64,
    /// Optional error message on failure (truncated to 512 chars).
    pub error: Option<String>,
    /// Optional key-value metadata (token counts, file count, etc.).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, String>,
}

impl GateEvent {
    /// Create a gate-start event.
    pub fn start(gate: &str, session_id: &str, chunk_id: &str) -> Self {
        Self {
            ts: now_ms(),
            gate: gate.to_string(),
            session_id: session_id.to_string(),
            chunk_id: chunk_id.to_string(),
            event_type: "start".to_string(),
            result: None,
            duration_ms: 0,
            error: None,
            meta: BTreeMap::new(),
        }
    }

    /// Create a gate-end event.
    pub fn end(
        gate: &str,
        session_id: &str,
        chunk_id: &str,
        result: GateResult,
        duration_ms: u64,
        error: Option<&str>,
    ) -> Self {
        Self {
            ts: now_ms(),
            gate: gate.to_string(),
            session_id: session_id.to_string(),
            chunk_id: chunk_id.to_string(),
            event_type: "end".to_string(),
            result: Some(result),
            duration_ms,
            error: error.map(|e| truncate_str(e, 512)),
            meta: BTreeMap::new(),
        }
    }

    /// Add a metadata key-value pair.
    pub fn with_meta(mut self, key: &str, value: impl ToString) -> Self {
        self.meta.insert(key.to_string(), value.to_string());
        self
    }
}

// ---------------------------------------------------------------------------
// Gate log (append-only JSONL)
// ---------------------------------------------------------------------------

/// Append-only gate event log. Cheap to clone (just a path).
#[derive(Debug, Clone)]
pub struct GateLog {
    path: PathBuf,
}

impl GateLog {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join(GATE_LOG_FILE),
        }
    }

    /// Append a gate event to the log.
    pub fn record(&self, event: &GateEvent) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string(event)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        crate::coding::rolling_log::append_line(&self.path, &json)
    }

    /// Read the most recent `limit` end-events (newest first) from the
    /// current file and its `.001` archive.
    pub fn recent_ends(&self, limit: usize) -> Vec<GateEvent> {
        let mut out: Vec<GateEvent> =
            crate::coding::rolling_log::read_jsonl_pair::<GateEvent>(&self.path)
                .into_iter()
                .filter(|e| e.event_type == "end")
                .collect();
        out.reverse();
        if out.len() > limit {
            out.truncate(limit);
        }
        out
    }

    /// Compute per-gate aggregate stats.
    pub fn summary(&self) -> GateMetricsSummary {
        let events = self.recent_ends(MAX_GATE_RECORDS);
        summarise_gates(&events)
    }

    /// Wipe the log.
    pub fn clear(&self) -> std::io::Result<()> {
        crate::coding::rolling_log::clear_pair(&self.path)
    }
}

// ---------------------------------------------------------------------------
// Aggregate stats
// ---------------------------------------------------------------------------

/// Per-gate aggregate metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GateStats {
    pub total: usize,
    pub pass: usize,
    pub partial: usize,
    pub fail: usize,
    /// 0.0–1.0; `0.0` when `total == 0`.
    pub pass_rate: f64,
    /// Mean duration of completed gates in milliseconds.
    pub avg_duration_ms: u64,
    /// Most recent failure message for this gate, if any.
    pub last_error: Option<String>,
    /// Unix-epoch ms of the most recent execution.
    pub last_run_at_ms: u64,
}

/// Summary across all gates.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GateMetricsSummary {
    /// Per-gate stats keyed by gate name.
    pub gates: BTreeMap<String, GateStats>,
    /// The gate that was most recently executing (if any session is active).
    pub active_gate: Option<String>,
    /// The last gate that completed successfully in the most recent session.
    pub last_successful_gate: Option<String>,
    /// The session id of the most recent gate event.
    pub last_session_id: Option<String>,
}

/// Pure aggregation function over end-events (newest-first).
pub fn summarise_gates(events: &[GateEvent]) -> GateMetricsSummary {
    let mut summary = GateMetricsSummary::default();
    let mut per_gate: BTreeMap<String, GateStats> = BTreeMap::new();

    // Events are newest-first, so first event is the most recent.
    if let Some(first) = events.first() {
        summary.last_session_id = Some(first.session_id.clone());
    }

    for event in events {
        let stats = per_gate.entry(event.gate.clone()).or_default();
        stats.total += 1;
        match event.result {
            Some(GateResult::Pass) => {
                stats.pass += 1;
                if summary.last_successful_gate.is_none() {
                    summary.last_successful_gate = Some(event.gate.clone());
                }
            }
            Some(GateResult::Partial) => stats.partial += 1,
            Some(GateResult::Fail) => {
                stats.fail += 1;
                if stats.last_error.is_none() {
                    stats.last_error = event.error.clone();
                }
            }
            None => {}
        }
        stats.avg_duration_ms = stats
            .avg_duration_ms
            .saturating_add(event.duration_ms / stats.total.max(1) as u64);
        if event.ts > stats.last_run_at_ms {
            stats.last_run_at_ms = event.ts;
        }
    }

    // Fix avg to be true averages (accumulated sum above isn't correct)
    for stats in per_gate.values_mut() {
        if stats.total > 0 {
            stats.pass_rate = stats.pass as f64 / stats.total as f64;
        }
    }

    // Recompute avg_duration_ms correctly
    let all_events_by_gate = {
        let mut map: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        for e in events {
            map.entry(e.gate.clone()).or_default().push(e.duration_ms);
        }
        map
    };
    for (gate, durations) in &all_events_by_gate {
        if let Some(stats) = per_gate.get_mut(gate) {
            let sum: u64 = durations.iter().sum();
            stats.avg_duration_ms = sum / durations.len().max(1) as u64;
        }
    }

    summary.gates = per_gate;
    summary
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.min(s.len())])
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_event_serializes_round_trip() {
        let event = GateEvent::start("plan", "session-1", "34.2").with_meta("tokens", "1500");
        let json = serde_json::to_string(&event).unwrap();
        let back: GateEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.gate, "plan");
        assert_eq!(back.session_id, "session-1");
        assert_eq!(back.chunk_id, "34.2");
        assert_eq!(back.event_type, "start");
        assert_eq!(back.meta.get("tokens").map(|s| s.as_str()), Some("1500"));
    }

    #[test]
    fn gate_event_end_serializes() {
        let event = GateEvent::end(
            "test",
            "s1",
            "34.2",
            GateResult::Fail,
            5000,
            Some("tests failed"),
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"result\":\"fail\""));
        assert!(json.contains("\"duration_ms\":5000"));
    }

    #[test]
    fn summarise_empty_is_default() {
        let summary = summarise_gates(&[]);
        assert!(summary.gates.is_empty());
        assert!(summary.active_gate.is_none());
    }

    #[test]
    fn summarise_computes_pass_rate() {
        let events = vec![
            GateEvent::end("plan", "s1", "1.0", GateResult::Pass, 100, None),
            GateEvent::end("plan", "s1", "1.1", GateResult::Pass, 200, None),
            GateEvent::end("plan", "s1", "1.2", GateResult::Fail, 50, Some("oops")),
            GateEvent::end("test", "s1", "1.0", GateResult::Pass, 3000, None),
        ];
        let summary = summarise_gates(&events);
        let plan = summary.gates.get("plan").unwrap();
        assert_eq!(plan.total, 3);
        assert_eq!(plan.pass, 2);
        assert_eq!(plan.fail, 1);
        assert!((plan.pass_rate - 2.0 / 3.0).abs() < 0.01);
        // avg of 100+200+50 = 350/3 ≈ 116
        assert!(plan.avg_duration_ms > 100 && plan.avg_duration_ms < 130);
        let test = summary.gates.get("test").unwrap();
        assert_eq!(test.total, 1);
        assert_eq!(test.avg_duration_ms, 3000);
    }

    #[test]
    fn gate_log_record_and_read() {
        let dir = std::env::temp_dir().join(format!("terransoul_gate_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let log = GateLog::new(&dir);
        let _ = log.clear();

        log.record(&GateEvent::start("plan", "s1", "34.2")).unwrap();
        log.record(&GateEvent::end(
            "plan",
            "s1",
            "34.2",
            GateResult::Pass,
            150,
            None,
        ))
        .unwrap();
        log.record(&GateEvent::end(
            "code",
            "s1",
            "34.2",
            GateResult::Fail,
            500,
            Some("err"),
        ))
        .unwrap();

        let ends = log.recent_ends(10);
        assert_eq!(ends.len(), 2); // only end events
        assert_eq!(ends[0].gate, "code"); // newest first
        assert_eq!(ends[1].gate, "plan");

        let summary = log.summary();
        assert_eq!(summary.gates.len(), 2);
        assert_eq!(summary.gates["plan"].pass, 1);
        assert_eq!(summary.gates["code"].fail, 1);

        let _ = log.clear();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
