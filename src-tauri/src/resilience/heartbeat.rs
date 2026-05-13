//! Heartbeat writer, startup classification, and crash-loop guard.
//!
//! The heartbeat writer appends a JSONL line every 30 s to
//! `<data_dir>/uptime/heartbeat.jsonl`. On startup, the classifier reads
//! the tail of this file to determine how the previous session ended
//! (`clean_exit`, `crash`, `power_loss`, `first_run`). If ≥ 3 crashes
//! occurred within the last 5 min, the crash-loop guard fires and puts
//! the app in safe mode.
//!
//! This module is intentionally self-contained with no dependency on
//! `AppState` — it takes a `PathBuf` for the data dir and an
//! `Arc<AtomicBool>` for the safe-mode flag. Background tasks own their
//! own cancellation via `watch::Receiver<bool>`.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

// ─── Types ───────────────────────────────────────────────────────────────────

/// One line in `heartbeat.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEntry {
    /// Unix epoch seconds.
    pub ts: u64,
    /// Process ID of the writer.
    pub pid: u32,
    /// App version string (from `env!("CARGO_PKG_VERSION")`).
    pub version: String,
    /// Present (and "clean") only on graceful shutdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<String>,
}

/// How the previous session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownClass {
    /// Graceful exit with a clean-exit marker.
    CleanExit,
    /// Heartbeats exist but no clean-exit marker — the process crashed.
    Crash,
    /// Heartbeats are too old (>5 min gap) — likely a power loss / OS crash.
    PowerLoss,
    /// No heartbeat file at all — first ever run.
    FirstRun,
}

/// Result of startup classification — includes the crash-loop guard decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupClassification {
    pub shutdown_class: ShutdownClass,
    /// True if ≥ 3 crashes detected within the last 5 minutes.
    pub safe_mode_required: bool,
    /// Number of recent crashes detected (within 5-min window).
    pub recent_crash_count: usize,
}

/// Handle to the background heartbeat writer task.
pub struct HeartbeatWriter {
    shutdown_tx: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl HeartbeatWriter {
    /// Spawn the heartbeat writer. Writes to `<data_dir>/uptime/heartbeat.jsonl`
    /// every `interval` (default 30 s). Stops when `shutdown_tx` fires.
    pub fn spawn(data_dir: PathBuf, interval: std::time::Duration) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let task = tokio::spawn(heartbeat_loop(data_dir, interval, shutdown_rx));
        Self { shutdown_tx, task }
    }

    /// Signal the writer to stop and wait for the task to finish.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(true);
        let _ = self.task.await;
    }
}

/// Crash-loop guard — wraps the safe-mode flag and auto-recovery timer.
pub struct CrashLoopGuard {
    safe_mode: Arc<AtomicBool>,
    /// If safe-mode is active, this handle runs the 10-min auto-recovery timer.
    recovery_task: Option<tokio::task::JoinHandle<()>>,
}

impl CrashLoopGuard {
    /// Create a new guard. If `classification.safe_mode_required`, the
    /// `safe_mode` flag is immediately set to `true` and a 10-min recovery
    /// timer is spawned.
    pub fn new(classification: &StartupClassification, safe_mode: Arc<AtomicBool>) -> Self {
        if classification.safe_mode_required {
            safe_mode.store(true, Ordering::SeqCst);
            let flag = safe_mode.clone();
            let recovery_task = Some(tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(600)).await;
                flag.store(false, Ordering::SeqCst);
            }));
            Self {
                safe_mode,
                recovery_task,
            }
        } else {
            Self {
                safe_mode,
                recovery_task: None,
            }
        }
    }

    /// Whether the app is currently in safe mode.
    pub fn is_safe_mode(&self) -> bool {
        self.safe_mode.load(Ordering::SeqCst)
    }

    /// Manually exit safe mode (user clicked "Exit safe mode").
    pub fn exit_safe_mode(&self) {
        self.safe_mode.store(false, Ordering::SeqCst);
    }
}

impl Drop for CrashLoopGuard {
    fn drop(&mut self) {
        if let Some(task) = self.recovery_task.take() {
            task.abort();
        }
    }
}

// ─── Startup Classification ──────────────────────────────────────────────────

/// Read the heartbeat file and classify the previous shutdown.
///
/// `data_dir` is the app's data directory (containing `uptime/heartbeat.jsonl`).
pub fn classify_previous_shutdown(data_dir: &Path) -> StartupClassification {
    let hb_path = data_dir.join("uptime").join("heartbeat.jsonl");

    if !hb_path.exists() {
        return StartupClassification {
            shutdown_class: ShutdownClass::FirstRun,
            safe_mode_required: false,
            recent_crash_count: 0,
        };
    }

    let entries = read_last_entries(&hb_path, 100);
    if entries.is_empty() {
        return StartupClassification {
            shutdown_class: ShutdownClass::FirstRun,
            safe_mode_required: false,
            recent_crash_count: 0,
        };
    }

    let now = epoch_secs();
    let last = &entries[entries.len() - 1];

    // Determine shutdown class from the last entry
    let shutdown_class = if last.exit.as_deref() == Some("clean") {
        ShutdownClass::CleanExit
    } else if now.saturating_sub(last.ts) > 300 {
        // Last heartbeat is more than 5 min old — power loss
        ShutdownClass::PowerLoss
    } else {
        ShutdownClass::Crash
    };

    // Count crashes in last 5 minutes: a "crash" is a session that has
    // heartbeats but ends without a clean-exit marker. We detect session
    // boundaries by looking for gaps >60s or clean-exit markers.
    let five_min_ago = now.saturating_sub(300);
    let recent_crash_count = count_recent_crashes(&entries, five_min_ago);
    let safe_mode_required = recent_crash_count >= 3;

    StartupClassification {
        shutdown_class,
        safe_mode_required,
        recent_crash_count,
    }
}

/// Write a clean-exit marker to the heartbeat file (called on graceful shutdown).
pub fn write_clean_exit_marker(data_dir: &Path) {
    let dir = data_dir.join("uptime");
    let _ = fs::create_dir_all(&dir);
    let hb_path = dir.join("heartbeat.jsonl");

    let entry = HeartbeatEntry {
        ts: epoch_secs(),
        pid: std::process::id(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        exit: Some("clean".to_string()),
    };

    if let Ok(line) = serde_json::to_string(&entry) {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&hb_path) {
            let _ = writeln!(f, "{}", line);
        }
    }
}

// ─── Internal Helpers ────────────────────────────────────────────────────────

async fn heartbeat_loop(
    data_dir: PathBuf,
    interval: std::time::Duration,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let dir = data_dir.join("uptime");
    let _ = fs::create_dir_all(&dir);
    let hb_path = dir.join("heartbeat.jsonl");

    // Rotate if > 1 MB
    if let Ok(meta) = fs::metadata(&hb_path) {
        if meta.len() > 1_048_576 {
            let prev = dir.join("heartbeat.prev.jsonl");
            let _ = fs::rename(&hb_path, prev);
        }
    }

    loop {
        // Write one heartbeat
        let entry = HeartbeatEntry {
            ts: epoch_secs(),
            pid: std::process::id(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            exit: None,
        };
        if let Ok(line) = serde_json::to_string(&entry) {
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&hb_path) {
                let _ = writeln!(f, "{}", line);
            }
        }

        // Wait for the next tick or shutdown
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    break;
                }
            }
        }
    }
}

/// Read up to the last `max_lines` entries from the heartbeat file.
fn read_last_entries(path: &Path, max_lines: usize) -> Vec<HeartbeatEntry> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(max_lines);

    lines[start..]
        .iter()
        .filter_map(|line| serde_json::from_str::<HeartbeatEntry>(line).ok())
        .collect()
}

/// Count the number of crash-like session ends within the time window.
/// A crash boundary is: a gap >60s between consecutive heartbeats where
/// the earlier entry does NOT have `exit: "clean"`.
fn count_recent_crashes(entries: &[HeartbeatEntry], since_ts: u64) -> usize {
    if entries.len() < 2 {
        return 0;
    }

    let mut crashes = 0;
    for window in entries.windows(2) {
        let prev = &window[0];
        let next = &window[1];

        // Only look at entries in our time window
        if next.ts < since_ts {
            continue;
        }

        // A gap >60s without a clean exit = crash boundary
        let gap = next.ts.saturating_sub(prev.ts);
        if gap > 60 && prev.exit.as_deref() != Some("clean") {
            crashes += 1;
        }
    }
    crashes
}

fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper: write heartbeat entries to a temp dir's heartbeat.jsonl.
    fn write_entries(dir: &Path, entries: &[HeartbeatEntry]) {
        let uptime_dir = dir.join("uptime");
        fs::create_dir_all(&uptime_dir).unwrap();
        let path = uptime_dir.join("heartbeat.jsonl");
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .unwrap();
        for entry in entries {
            writeln!(f, "{}", serde_json::to_string(entry).unwrap()).unwrap();
        }
    }

    #[test]
    fn first_run_when_no_file() {
        let tmp = TempDir::new().unwrap();
        let result = classify_previous_shutdown(tmp.path());
        assert_eq!(result.shutdown_class, ShutdownClass::FirstRun);
        assert!(!result.safe_mode_required);
    }

    #[test]
    fn clean_exit_detected() {
        let tmp = TempDir::new().unwrap();
        let now = epoch_secs();
        write_entries(
            tmp.path(),
            &[
                HeartbeatEntry {
                    ts: now - 60,
                    pid: 1,
                    version: "0.1.0".into(),
                    exit: None,
                },
                HeartbeatEntry {
                    ts: now - 30,
                    pid: 1,
                    version: "0.1.0".into(),
                    exit: Some("clean".into()),
                },
            ],
        );
        let result = classify_previous_shutdown(tmp.path());
        assert_eq!(result.shutdown_class, ShutdownClass::CleanExit);
        assert!(!result.safe_mode_required);
    }

    #[test]
    fn crash_detected_when_no_clean_marker() {
        let tmp = TempDir::new().unwrap();
        let now = epoch_secs();
        // Last heartbeat is recent but has no clean-exit marker
        write_entries(
            tmp.path(),
            &[HeartbeatEntry {
                ts: now - 10,
                pid: 1,
                version: "0.1.0".into(),
                exit: None,
            }],
        );
        let result = classify_previous_shutdown(tmp.path());
        assert_eq!(result.shutdown_class, ShutdownClass::Crash);
    }

    #[test]
    fn power_loss_when_heartbeat_too_old() {
        let tmp = TempDir::new().unwrap();
        let now = epoch_secs();
        write_entries(
            tmp.path(),
            &[HeartbeatEntry {
                ts: now - 600, // 10 min ago, no clean exit
                pid: 1,
                version: "0.1.0".into(),
                exit: None,
            }],
        );
        let result = classify_previous_shutdown(tmp.path());
        assert_eq!(result.shutdown_class, ShutdownClass::PowerLoss);
    }

    #[test]
    fn crash_loop_triggers_safe_mode() {
        let tmp = TempDir::new().unwrap();
        let now = epoch_secs();
        // Simulate 4 crash boundaries within 5 minutes: 4 sessions that
        // each wrote 1 heartbeat then died (gap > 60s, no clean marker).
        write_entries(
            tmp.path(),
            &[
                HeartbeatEntry {
                    ts: now - 240,
                    pid: 100,
                    version: "0.1.0".into(),
                    exit: None,
                },
                // 80s gap → crash #1
                HeartbeatEntry {
                    ts: now - 160,
                    pid: 101,
                    version: "0.1.0".into(),
                    exit: None,
                },
                // 80s gap → crash #2
                HeartbeatEntry {
                    ts: now - 80,
                    pid: 102,
                    version: "0.1.0".into(),
                    exit: None,
                },
                // 70s gap → crash #3
                HeartbeatEntry {
                    ts: now - 10,
                    pid: 103,
                    version: "0.1.0".into(),
                    exit: None,
                },
            ],
        );
        let result = classify_previous_shutdown(tmp.path());
        assert_eq!(result.shutdown_class, ShutdownClass::Crash);
        assert!(result.safe_mode_required);
        assert!(result.recent_crash_count >= 3);
    }

    #[test]
    fn no_safe_mode_when_crashes_are_old() {
        let tmp = TempDir::new().unwrap();
        let now = epoch_secs();
        // Crashes all happened > 5 min ago
        write_entries(
            tmp.path(),
            &[
                HeartbeatEntry {
                    ts: now - 600,
                    pid: 100,
                    version: "0.1.0".into(),
                    exit: None,
                },
                HeartbeatEntry {
                    ts: now - 500,
                    pid: 101,
                    version: "0.1.0".into(),
                    exit: None,
                },
                HeartbeatEntry {
                    ts: now - 400,
                    pid: 102,
                    version: "0.1.0".into(),
                    exit: None,
                },
                HeartbeatEntry {
                    ts: now - 310,
                    pid: 103,
                    version: "0.1.0".into(),
                    exit: None,
                },
            ],
        );
        let result = classify_previous_shutdown(tmp.path());
        // All crashes are > 5 min ago — should not trigger safe mode
        assert!(!result.safe_mode_required);
    }

    #[tokio::test]
    async fn crash_loop_guard_sets_and_clears_safe_mode() {
        let safe_mode = Arc::new(AtomicBool::new(false));
        let classification = StartupClassification {
            shutdown_class: ShutdownClass::Crash,
            safe_mode_required: true,
            recent_crash_count: 3,
        };
        let guard = CrashLoopGuard::new(&classification, safe_mode.clone());
        assert!(guard.is_safe_mode());

        // Manual exit
        guard.exit_safe_mode();
        assert!(!guard.is_safe_mode());
        drop(guard);
    }

    #[tokio::test]
    async fn heartbeat_writer_writes_and_stops() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().to_path_buf();
        let writer =
            HeartbeatWriter::spawn(data_dir.clone(), std::time::Duration::from_millis(50));

        // Wait for a few beats
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        writer.shutdown().await;

        let hb_path = data_dir.join("uptime").join("heartbeat.jsonl");
        assert!(hb_path.exists());
        let content = fs::read_to_string(&hb_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        // Should have written at least 2 heartbeats in 200ms with 50ms interval
        assert!(lines.len() >= 2, "Expected ≥2 lines, got {}", lines.len());

        // Each line should parse as HeartbeatEntry
        for line in &lines {
            let entry: HeartbeatEntry = serde_json::from_str(line).unwrap();
            assert_eq!(entry.exit, None);
            assert_eq!(entry.pid, std::process::id());
        }
    }

    #[test]
    fn write_clean_exit_marker_appends() {
        let tmp = TempDir::new().unwrap();
        write_clean_exit_marker(tmp.path());

        let hb_path = tmp.path().join("uptime").join("heartbeat.jsonl");
        let content = fs::read_to_string(&hb_path).unwrap();
        let entry: HeartbeatEntry = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(entry.exit.as_deref(), Some("clean"));
    }
}
