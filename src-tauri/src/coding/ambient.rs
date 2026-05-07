//! Background-maintenance ambient agent skeleton (Chunk 43.11).
//!
//! Wraps [`brain::maintenance_scheduler`] and the safety classifier to
//! provide a structured ambient agent cycle:
//!
//! 1. **garden** — run brain maintenance (decay, GC, promote, ANN compact).
//! 2. **extract_from_session** — pull learnings from recent sessions.
//! 3. **verify_fact** — spot-check a memory's confidence.
//! 4. **scout_recent_sessions** — scan session history for actionable items.
//! 5. **request_permission** — gate destructive work through the safety classifier.
//! 6. **schedule_next** — compute the next wake-up time.
//! 7. **end_cycle** (mandatory) — close the cycle, record feedback, trigger
//!    promotion checks.
//!
//! Default: `enabled = false`, `proactive_work = false` until 20
//! decision-history cycles exist.

use crate::brain::maintenance_scheduler::{
    self, MaintenanceConfig, MaintenanceJob, MaintenanceState as SchedulerState,
};
use crate::coding::safety::{self, Action, SafetyConfig};
use crate::memory::store::now_ms;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Ambient agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientConfig {
    /// Master switch — agent does nothing when `false`.
    #[serde(default)]
    pub enabled: bool,
    /// Whether the agent may do proactive work (extract, verify, scout).
    /// Requires ≥ 20 decision-history cycles before it can be turned on.
    #[serde(default)]
    pub proactive_work: bool,
    /// Minimum completed cycles before `proactive_work` may be enabled.
    #[serde(default = "default_maturity_threshold")]
    pub maturity_threshold: u32,
    /// Fraction of rate-limit budget reserved for user (0.0 – 1.0).
    #[serde(default = "default_user_headroom")]
    pub user_headroom: f64,
    /// Base delay between cycles in milliseconds.
    #[serde(default = "default_cycle_delay_ms")]
    pub cycle_delay_ms: u64,
    /// Safety config forwarded to the permission gate.
    #[serde(default)]
    pub safety: SafetyConfig,
    /// Maintenance scheduler config.
    #[serde(default)]
    pub maintenance: MaintenanceConfig,
}

fn default_maturity_threshold() -> u32 {
    20
}
fn default_user_headroom() -> f64 {
    0.20
}
fn default_cycle_delay_ms() -> u64 {
    60_000 // 1 minute
}

impl Default for AmbientConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            proactive_work: false,
            maturity_threshold: default_maturity_threshold(),
            user_headroom: default_user_headroom(),
            cycle_delay_ms: default_cycle_delay_ms(),
            safety: SafetyConfig::default(),
            maintenance: MaintenanceConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool enum — the seven ambient-agent actions
// ---------------------------------------------------------------------------

/// Discrete tools the ambient agent may invoke during a cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbientTool {
    /// Run brain maintenance (decay, GC, promote, edge-extract, ANN).
    Garden,
    /// Extract memories from a recent session.
    ExtractFromSession,
    /// Spot-check a memory's confidence / contradiction status.
    VerifyFact,
    /// Scan session history for actionable items.
    ScoutRecentSessions,
    /// Gate destructive work through the safety classifier.
    RequestPermission,
    /// Compute the next wake-up time.
    ScheduleNext,
    /// Close the cycle (mandatory).
    EndCycle,
}

impl AmbientTool {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Garden => "garden",
            Self::ExtractFromSession => "extract_from_session",
            Self::VerifyFact => "verify_fact",
            Self::ScoutRecentSessions => "scout_recent_sessions",
            Self::RequestPermission => "request_permission",
            Self::ScheduleNext => "schedule_next",
            Self::EndCycle => "end_cycle",
        }
    }

    /// Whether this tool requires `proactive_work` to be enabled.
    pub fn requires_proactive(&self) -> bool {
        matches!(
            self,
            Self::ExtractFromSession | Self::VerifyFact | Self::ScoutRecentSessions
        )
    }
}

// ---------------------------------------------------------------------------
// Cycle state
// ---------------------------------------------------------------------------

/// Tracks one ambient cycle's progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleState {
    /// Monotonic cycle counter.
    pub cycle_number: u64,
    /// Tools invoked so far in this cycle.
    pub tools_invoked: Vec<String>,
    /// Whether `end_cycle` has been called.
    pub ended: bool,
    /// Timestamp when the cycle started (Unix ms).
    pub started_at_ms: i64,
    /// Errors accumulated during this cycle.
    pub errors: Vec<String>,
}

impl CycleState {
    pub fn new(cycle_number: u64) -> Self {
        Self {
            cycle_number,
            tools_invoked: Vec::new(),
            ended: false,
            started_at_ms: now_ms(),
            errors: Vec::new(),
        }
    }

    pub fn record_tool(&mut self, tool: AmbientTool) {
        self.tools_invoked.push(tool.as_str().to_string());
    }

    pub fn record_error(&mut self, msg: String) {
        self.errors.push(msg);
    }
}

// ---------------------------------------------------------------------------
// PID guard — single-instance enforcement
// ---------------------------------------------------------------------------

/// Crash-safe PID file guard. Writes current PID to `<data_dir>/ambient.pid`.
/// Removal is best-effort on drop; stale files are detected by checking
/// whether the recorded PID is still alive.
#[derive(Debug)]
pub struct PidGuard {
    path: PathBuf,
}

impl PidGuard {
    /// Acquire the PID lock. Returns `None` if another instance holds it.
    pub fn acquire(data_dir: &Path) -> Option<Self> {
        let path = data_dir.join("ambient.pid");
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(pid) = contents.trim().parse::<u32>() {
                    if is_pid_alive(pid) {
                        return None; // another instance running
                    }
                }
            }
            // Stale PID file — remove it.
            let _ = fs::remove_file(&path);
        }
        let current_pid = std::process::id();
        if fs::write(&path, current_pid.to_string()).is_ok() {
            Some(Self { path })
        } else {
            None
        }
    }

    /// Path to the PID file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PidGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Best-effort process liveness check.
#[cfg(windows)]
fn is_pid_alive(pid: u32) -> bool {
    // Use tasklist to check if the PID exists. This avoids a dependency
    // on windows-sys.
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .map(|o| {
            let out = String::from_utf8_lossy(&o.stdout);
            // tasklist prints the process line if it exists, or "INFO: No tasks"
            !out.contains("No tasks") && out.contains(&pid.to_string())
        })
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_pid_alive(pid: u32) -> bool {
    // signal 0 checks existence without actually signalling.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

// ---------------------------------------------------------------------------
// Core logic — garden, permission gate, maturity check
// ---------------------------------------------------------------------------

/// Run the garden tool: check which maintenance jobs are due and return them.
pub fn garden(
    scheduler_state: &SchedulerState,
    config: &AmbientConfig,
) -> Vec<MaintenanceJob> {
    let now = now_ms() as u64;
    maintenance_scheduler::jobs_due(scheduler_state, &config.maintenance, now)
}

/// Check whether proactive work is allowed based on decision history.
pub fn is_mature(conn: &Connection, config: &AmbientConfig) -> bool {
    let total: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM safety_decisions",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    total >= config.maturity_threshold
}

/// Gate an action through the safety classifier, returning whether it
/// was approved.
pub fn gate_action(
    conn: &Connection,
    action: Action,
    config: &AmbientConfig,
    reason: &str,
) -> Result<bool, String> {
    safety::request_permission(conn, action, &config.safety, reason)
        .map_err(|e| e.to_string())
}

/// End a cycle: record feedback, check promotions for all actions.
pub fn end_cycle(
    conn: &Connection,
    cycle: &mut CycleState,
    config: &AmbientConfig,
) -> Result<Vec<Action>, String> {
    cycle.ended = true;
    // Check promotions for every action.
    let promotable: Vec<Action> = [
        Action::Read,
        Action::Write,
        Action::RunTests,
        Action::CreateBranch,
        Action::PushRemote,
        Action::OpenPr,
        Action::MergePr,
        Action::RunShell,
        Action::SendEmail,
        Action::InstallPackage,
        Action::DeleteFile,
        Action::DropTable,
    ]
    .iter()
    .filter(|&&a| {
        safety::check_promotion(conn, a, &config.safety)
            .unwrap_or(false)
    })
    .copied()
    .collect();
    Ok(promotable)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS safety_decisions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action TEXT NOT NULL,
                decision TEXT NOT NULL,
                decided_at INTEGER NOT NULL,
                decided_via TEXT NOT NULL DEFAULT 'auto'
            );
            CREATE INDEX IF NOT EXISTS idx_safety_decided_at
                ON safety_decisions(decided_at);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn default_config_disabled() {
        let cfg = AmbientConfig::default();
        assert!(!cfg.enabled);
        assert!(!cfg.proactive_work);
        assert_eq!(cfg.maturity_threshold, 20);
    }

    #[test]
    fn cycle_state_tracks_tools() {
        let mut cycle = CycleState::new(1);
        cycle.record_tool(AmbientTool::Garden);
        cycle.record_tool(AmbientTool::EndCycle);
        assert_eq!(cycle.tools_invoked, vec!["garden", "end_cycle"]);
        assert!(!cycle.ended);
    }

    #[test]
    fn proactive_tools_flagged() {
        assert!(AmbientTool::ExtractFromSession.requires_proactive());
        assert!(AmbientTool::VerifyFact.requires_proactive());
        assert!(AmbientTool::ScoutRecentSessions.requires_proactive());
        assert!(!AmbientTool::Garden.requires_proactive());
        assert!(!AmbientTool::EndCycle.requires_proactive());
    }

    #[test]
    fn maturity_check_below_threshold() {
        let conn = test_conn();
        let config = AmbientConfig::default();
        assert!(!is_mature(&conn, &config));
    }

    #[test]
    fn maturity_check_above_threshold() {
        let conn = test_conn();
        let config = AmbientConfig {
            maturity_threshold: 3,
            ..Default::default()
        };
        for i in 0..3 {
            conn.execute(
                "INSERT INTO safety_decisions (action, decision, decided_at, decided_via)
                 VALUES ('read', 'approved', ?1, 'test')",
                params![i],
            )
            .unwrap();
        }
        assert!(is_mature(&conn, &config));
    }

    #[test]
    fn gate_action_auto_approves_tier1() {
        let conn = test_conn();
        let config = AmbientConfig::default();
        let approved = gate_action(&conn, Action::Read, &config, "ambient_test").unwrap();
        assert!(approved);
    }

    #[test]
    fn gate_action_denies_tier2() {
        let conn = test_conn();
        let config = AmbientConfig::default();
        let approved = gate_action(&conn, Action::DropTable, &config, "ambient_test").unwrap();
        assert!(!approved);
    }

    #[test]
    fn end_cycle_marks_ended() {
        let conn = test_conn();
        let config = AmbientConfig::default();
        let mut cycle = CycleState::new(1);
        let promotable = end_cycle(&conn, &mut cycle, &config).unwrap();
        assert!(cycle.ended);
        // No approvals yet → nothing promotable.
        assert!(promotable.is_empty());
    }

    #[test]
    fn garden_delegates_to_scheduler() {
        let state = SchedulerState::default();
        let config = AmbientConfig::default();
        // Default scheduler state has all last-run = 0, so all jobs fire.
        let jobs = garden(&state, &config);
        assert!(!jobs.is_empty());
    }

    #[test]
    fn pid_guard_acquire_and_release() {
        let dir = std::env::temp_dir().join("ambient_test_pid");
        let _ = fs::create_dir_all(&dir);
        {
            let guard = PidGuard::acquire(&dir);
            assert!(guard.is_some());
            let path = guard.as_ref().unwrap().path().to_path_buf();
            assert!(path.exists());
            // Second acquire fails — same PID is alive.
            let guard2 = PidGuard::acquire(&dir);
            assert!(guard2.is_none());
            drop(guard);
            // After drop, PID file removed.
            assert!(!path.exists());
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
