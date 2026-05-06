//! Background-maintenance runtime wrapper (Chunk 26.1b).
//!
//! Closes the loop on Chunk 26.1: that chunk shipped the *pure*
//! decision module (`maintenance_scheduler`) which decides which jobs
//! are due. This module owns the *runtime concerns* — disk
//! persistence of the per-job last-run timestamps, the
//! `tokio::time::interval` tick loop, and the actual dispatch into the
//! brain's four maintenance jobs.
//!
//! ## Why this is its own module
//!
//! The pure scheduler is a `no_std`-friendly piece of math. The runtime
//! wrapper has to talk to the file system, the `AppState` mutexes, and
//! `tokio::spawn` — keeping them apart means the math can be unit-tested
//! exhaustively without mocking out the whole world, and the wrapper
//! can be re-targeted (e.g. moved into a separate process) without
//! touching the decision logic.
//!
//! ## Persistence shape
//!
//! `MaintenanceState` is serialised as compact JSON to
//! `<data_dir>/maintenance_state.json`. We deliberately do **not** put
//! it in SQLite — adding a new table for four `INTEGER` fields would
//! mean a schema migration, while a JSON file is dead-simple and
//! mirrors how the self-improve metrics log is persisted today.
//!
//! Crash safety is "best effort": we write whenever a job finishes, but
//! a crash mid-write would just lose the most recent timestamp, which
//! at worst causes one extra job run on the next tick. There is no
//! corruption risk because each job's timestamp is independent.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex as TokioMutex;

use super::maintenance_scheduler::{jobs_due, MaintenanceConfig, MaintenanceJob, MaintenanceState};

/// Filename inside `data_dir` where the per-job timestamps are
/// persisted. Versioned via the JSON shape itself (serde additive
/// compatibility) — bump only if the field set ever shrinks.
const STATE_FILENAME: &str = "maintenance_state.json";

/// Default tick interval between scheduler checks. Picked at 1h so the
/// 23h cool-downs in [`MaintenanceConfig`] always fire on the next
/// daily tick even with mild clock drift, while keeping wakeup cost
/// trivial (<1ms per tick).
pub const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// In-process handle to the maintenance runtime. Holds the persisted
/// state behind a tokio mutex (so the tick task and any future
/// "force-fire-now" Tauri command can both touch it safely) plus the
/// path it was loaded from.
#[derive(Debug, Clone)]
pub struct MaintenanceRuntime {
    state: Arc<TokioMutex<MaintenanceState>>,
    config: MaintenanceConfig,
    state_path: PathBuf,
}

impl MaintenanceRuntime {
    /// Load the persisted [`MaintenanceState`] from `<data_dir>/maintenance_state.json`,
    /// falling back to `MaintenanceState::default()` (all jobs "never
    /// run") on missing or corrupt file. Never panics.
    pub fn load(data_dir: &std::path::Path, config: MaintenanceConfig) -> Self {
        let state_path = data_dir.join(STATE_FILENAME);
        let state = std::fs::read_to_string(&state_path)
            .ok()
            .and_then(|s| serde_json::from_str::<MaintenanceState>(&s).ok())
            .unwrap_or_default();
        Self {
            state: Arc::new(TokioMutex::new(state)),
            config,
            state_path,
        }
    }

    /// Persist the current state to disk. Writes the file atomically
    /// via a temp-file + rename so a crash mid-write can't corrupt the
    /// previous state.
    async fn persist(&self) -> std::io::Result<()> {
        let snapshot = { self.state.lock().await.clone() };
        let json = serde_json::to_string_pretty(&snapshot).map_err(std::io::Error::other)?;
        if let Some(parent) = self.state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp_path = self.state_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, json)?;
        std::fs::rename(&tmp_path, &self.state_path)?;
        Ok(())
    }

    /// Returns the list of jobs that should fire right now, given the
    /// persisted state and the current `now_ms`. Pure — does not
    /// dispatch.
    pub async fn jobs_due_now(&self, now_ms: u64) -> Vec<MaintenanceJob> {
        let state = self.state.lock().await;
        jobs_due(&state, &self.config, now_ms)
    }

    /// Same as [`jobs_due_now`] but evaluates against a caller-supplied
    /// override config — used by the tick loop so live changes to
    /// `AppSettings.maintenance_interval_hours` take effect without
    /// restart.
    pub async fn jobs_due_with(
        &self,
        config: &MaintenanceConfig,
        now_ms: u64,
    ) -> Vec<MaintenanceJob> {
        let state = self.state.lock().await;
        jobs_due(&state, config, now_ms)
    }

    /// Mark a job as "just finished" and persist. Called by the tick
    /// loop after each successful or failed dispatch — both outcomes
    /// reset the cool-down so a permanently-broken job doesn't burn
    /// CPU on every tick.
    pub async fn record_finished(&self, job: MaintenanceJob, now_ms: u64) {
        {
            let mut state = self.state.lock().await;
            state.record_finished(job, now_ms);
        }
        // Persistence failures are logged but do not propagate — a
        // missing timestamp at most causes one extra run next tick.
        if let Err(e) = self.persist().await {
            eprintln!(
                "[maintenance] failed to persist state to {}: {e}",
                self.state_path.display()
            );
        }
    }

    /// Read-only snapshot of the persisted state (for tests and a
    /// future Brain-panel "last run" UI).
    pub async fn snapshot(&self) -> MaintenanceState {
        self.state.lock().await.clone()
    }

    /// Path the runtime is persisting to. Useful for diagnostics.
    pub fn state_path(&self) -> &std::path::Path {
        &self.state_path
    }
}

/// Current epoch milliseconds, saturating to `0` on a broken clock.
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Spawn a `tokio::time::interval` background task that wakes up every
/// `tick_interval`, asks the scheduler which jobs are due, dispatches
/// each one in canonical order, and persists the timestamps.
///
/// Returns the runtime handle (already loaded + cloned into the task).
/// Callers should keep the handle alive for the lifetime of the app —
/// dropping it does *not* abort the spawned task (the task holds its
/// own clone), but the handle is required to query state from
/// elsewhere (e.g. a "force run now" Tauri command).
///
/// Dispatch implementations live in [`dispatch_job`] below — currently
/// each job operates directly on the `AppState` mutex tree to avoid a
/// runtime dependency on the Tauri command surface (the commands
/// require `State<AppState>` which can only be constructed inside a
/// Tauri-managed call). Using the underlying primitives directly keeps
/// the maintenance loop self-contained.
pub fn spawn(
    state_arc: crate::AppState,
    config: MaintenanceConfig,
    tick_interval: Duration,
) -> MaintenanceRuntime {
    let runtime = MaintenanceRuntime::load(&state_arc.data_dir, config);
    let runtime_clone = runtime.clone();
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(tick_interval);
        // First tick fires immediately by default — use the natural
        // first-tick to evaluate any jobs that are due right at boot
        // (e.g. first run on a fresh install fires everything at once).
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;

            // Chunk 26.1 final — honour live AppSettings on every
            // tick: if the user disabled background maintenance, or
            // we're inside the idle-minimum guard window, skip
            // dispatch entirely (we still ran the tick so live
            // re-enables take effect on the next interval).
            let (enabled, cooldown_ms, idle_min_minutes) = {
                let guard = state_arc.app_settings.lock().ok();
                match guard {
                    Some(s) => (
                        s.background_maintenance_enabled,
                        s.maintenance_cooldown_ms(),
                        s.maintenance_idle_minimum_minutes,
                    ),
                    None => (true, 23 * 60 * 60 * 1000, 0),
                }
            };
            if !enabled {
                continue;
            }
            if idle_min_minutes > 0 {
                let idle_threshold_ms = (idle_min_minutes as i64).saturating_mul(60_000);
                if !state_arc.activity_tracker.is_idle(idle_threshold_ms) {
                    // User is actively chatting — defer to next tick.
                    continue;
                }
            }

            // Re-derive due-list using the live cooldown so users can
            // tighten or relax the schedule without restart.
            let now = now_ms();
            let live_config = MaintenanceConfig {
                decay_cooldown_ms: cooldown_ms,
                garbage_collect_cooldown_ms: cooldown_ms,
                promote_tier_cooldown_ms: cooldown_ms,
                edge_extract_cooldown_ms: cooldown_ms,
                obsidian_export_cooldown_ms: cooldown_ms,
            };
            let due = runtime_clone.jobs_due_with(&live_config, now).await;

            for job in due {
                let result = dispatch_job(job, &state_arc).await;
                if let Err(e) = result {
                    eprintln!("[maintenance] job {} failed: {e}", job.as_str());
                }
                runtime_clone.record_finished(job, now_ms()).await;
            }
        }
    });
    runtime
}

/// Run the maintenance loop in the foreground until cancellation
/// (Chunk 33B.10). Unlike [`spawn`] which uses `tauri::async_runtime`,
/// this runs on the current tokio runtime — suitable for a standalone
/// `terransoul-scheduler` binary in headless/server environments.
///
/// The `cancel` token is polled on each tick; when it resolves the
/// loop exits cleanly.
pub async fn run_foreground(
    state_arc: crate::AppState,
    config: MaintenanceConfig,
    tick_interval: Duration,
    cancel: tokio_util::sync::CancellationToken,
) {
    let runtime = MaintenanceRuntime::load(&state_arc.data_dir, config);
    let mut ticker = tokio::time::interval(tick_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                eprintln!("[scheduler] shutdown signal received — exiting cleanly");
                break;
            }
            _ = ticker.tick() => {}
        }

        // Live settings check — same logic as the spawned variant.
        let (enabled, cooldown_ms, idle_min_minutes) = {
            let guard = state_arc.app_settings.lock().ok();
            match guard {
                Some(s) => (
                    s.background_maintenance_enabled,
                    s.maintenance_cooldown_ms(),
                    s.maintenance_idle_minimum_minutes,
                ),
                None => (true, 23 * 60 * 60 * 1000, 0),
            }
        };
        if !enabled {
            continue;
        }
        if idle_min_minutes > 0 {
            let idle_threshold_ms = (idle_min_minutes as i64).saturating_mul(60_000);
            if !state_arc.activity_tracker.is_idle(idle_threshold_ms) {
                continue;
            }
        }

        let now = now_ms();
        let live_config = MaintenanceConfig {
            decay_cooldown_ms: cooldown_ms,
            garbage_collect_cooldown_ms: cooldown_ms,
            promote_tier_cooldown_ms: cooldown_ms,
            edge_extract_cooldown_ms: cooldown_ms,
            obsidian_export_cooldown_ms: cooldown_ms,
        };
        let due = runtime.jobs_due_with(&live_config, now).await;

        for job in due {
            let result = dispatch_job(job, &state_arc).await;
            match &result {
                Ok(msg) => eprintln!("[scheduler] {msg}"),
                Err(e) => eprintln!("[scheduler] job {} failed: {e}", job.as_str()),
            }
            runtime.record_finished(job, now_ms()).await;
        }
    }
}

/// Run a single maintenance job. Returns `Ok(human_readable_summary)`
/// or `Err(error_string)`. Outcomes are surfaced via stderr (logging
/// pipeline TBD) and via the persisted timestamp.
pub async fn dispatch_job(job: MaintenanceJob, state: &crate::AppState) -> Result<String, String> {
    match job {
        MaintenanceJob::Decay => {
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            let n = store.apply_decay().map_err(|e| e.to_string())?;
            Ok(format!("decay: updated {n} entries"))
        }
        MaintenanceJob::GarbageCollect => {
            let threshold = 0.05_f64;
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            let decayed = store.gc_decayed(threshold).map_err(|e| e.to_string())?;
            let max_bytes = state
                .app_settings
                .lock()
                .map(|s| s.max_memory_bytes())
                .unwrap_or_else(|_| crate::settings::AppSettings::default().max_memory_bytes());
            let capped = store
                .enforce_size_limit(max_bytes)
                .map_err(|e| e.to_string())?;

            // Capacity-based eviction (Chunk 38.4): enforce hard cap on long-tier entries.
            let max_long = state
                .app_settings
                .lock()
                .map(|s| s.max_long_term_entries)
                .unwrap_or(crate::memory::eviction::DEFAULT_MAX_LONG_TERM);
            let eviction = crate::memory::eviction::enforce_capacity(
                store.conn(),
                max_long,
                crate::memory::eviction::DEFAULT_TARGET_RATIO,
                &state.data_dir,
            )
            .map_err(|e| e.to_string())?;
            let evicted = eviction.map(|r| r.dropped).unwrap_or(0) as usize;

            Ok(format!(
                "garbage_collect: removed {} entries (decay={decayed}, size_limit={}, eviction={evicted})",
                decayed + capped.deleted + evicted,
                capped.deleted
            ))
        }
        MaintenanceJob::PromoteTier => {
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            let promoted = store
                .auto_promote_to_long(5, 7)
                .map_err(|e| e.to_string())?;
            Ok(format!("promote_tier: promoted {} entries", promoted.len()))
        }
        MaintenanceJob::EdgeExtract => {
            // Edge extraction needs an LLM — skip silently when no
            // active brain is configured rather than failing. The
            // scheduler will retry on the next tick once a brain is
            // configured.
            let model = state.active_brain.lock().ok().and_then(|g| g.clone());
            let Some(model) = model else {
                return Ok("edge_extract: skipped (no active brain)".to_string());
            };
            // We can't pass a `&MemoryStore` across `.await` (the
            // `MutexGuard<MemoryStore>` isn't `Send` on the
            // `tauri::async_runtime` thread pool), so re-implement the
            // extraction loop with a lock-drop-await-relock pattern:
            // snapshot under a short lock, run the LLM with no lock
            // held, then re-lock per batch to insert.
            let entries: Vec<crate::memory::MemoryEntry> = {
                let store = state.memory_store.lock().map_err(|e| e.to_string())?;
                store.get_all().map_err(|e| e.to_string())?
            };
            if entries.len() < 2 {
                return Ok("edge_extract: skipped (<2 memories)".to_string());
            }
            let known_ids: std::collections::HashSet<i64> = entries.iter().map(|e| e.id).collect();
            let agent = crate::brain::OllamaAgent::new(&model);
            let chunk = 25usize;
            let mut total_inserted = 0usize;
            for window in entries.chunks(chunk) {
                let block = crate::memory::format_memories_for_extraction(window);
                let reply = agent.propose_edges(&block).await;
                if reply.trim().eq_ignore_ascii_case("NONE") {
                    continue;
                }
                let new_edges = crate::memory::parse_llm_edges(&reply, &known_ids);
                if new_edges.is_empty() {
                    continue;
                }
                let store = state.memory_store.lock().map_err(|e| e.to_string())?;
                if let Ok(n) = store.add_edges_batch(&new_edges) {
                    total_inserted += n;
                }
            }
            Ok(format!("edge_extract: inserted {total_inserted} edges"))
        }
        MaintenanceJob::ObsidianExport => {
            let vault_dir = state.data_dir.join("wiki");
            let entries: Vec<crate::memory::MemoryEntry> = {
                let store = state.memory_store.lock().map_err(|e| e.to_string())?;
                store.get_all().map_err(|e| e.to_string())?
            };
            if entries.is_empty() {
                return Ok("obsidian_export: skipped (no memories)".to_string());
            }
            let layout = state
                .app_settings
                .lock()
                .map_err(|e| e.to_string())?
                .obsidian_layout;
            let report = crate::memory::obsidian_export::export_to_vault_with_layout(
                &vault_dir, &entries, layout,
            )?;
            Ok(format!(
                "obsidian_export: wrote {}, skipped {}, total {} → {}",
                report.written, report.skipped, report.total, report.output_dir
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn load_with_no_file_returns_default_state() {
        let dir = tempdir().unwrap();
        let runtime = MaintenanceRuntime::load(dir.path(), MaintenanceConfig::default());
        let state = runtime.snapshot().await;
        assert_eq!(state.last_decay_ms, 0);
        assert_eq!(state.last_garbage_collect_ms, 0);
        assert_eq!(state.last_promote_tier_ms, 0);
        assert_eq!(state.last_edge_extract_ms, 0);
    }

    #[tokio::test]
    async fn record_then_reload_round_trips() {
        let dir = tempdir().unwrap();
        let runtime = MaintenanceRuntime::load(dir.path(), MaintenanceConfig::default());
        runtime
            .record_finished(MaintenanceJob::Decay, 1_700_000_000_000)
            .await;
        runtime
            .record_finished(MaintenanceJob::GarbageCollect, 1_700_000_001_000)
            .await;

        // Reload from disk into a fresh runtime — the timestamps must
        // survive the process boundary.
        let reloaded = MaintenanceRuntime::load(dir.path(), MaintenanceConfig::default());
        let state = reloaded.snapshot().await;
        assert_eq!(state.last_decay_ms, 1_700_000_000_000);
        assert_eq!(state.last_garbage_collect_ms, 1_700_000_001_000);
        assert_eq!(state.last_promote_tier_ms, 0);
        assert_eq!(state.last_edge_extract_ms, 0);
    }

    #[tokio::test]
    async fn corrupt_state_file_falls_back_to_default() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join(STATE_FILENAME), "not valid json {").unwrap();
        let runtime = MaintenanceRuntime::load(dir.path(), MaintenanceConfig::default());
        // Corrupt input must not panic and must give us a clean slate.
        let state = runtime.snapshot().await;
        assert_eq!(state.last_decay_ms, 0);
    }

    #[tokio::test]
    async fn jobs_due_now_returns_all_jobs_when_state_is_fresh() {
        let dir = tempdir().unwrap();
        let runtime = MaintenanceRuntime::load(dir.path(), MaintenanceConfig::default());
        // Default state = all `last_*_ms = 0` = everything always due.
        let due = runtime.jobs_due_now(123_456_789).await;
        assert_eq!(due.len(), 5);
        // Canonical order from the scheduler module.
        assert_eq!(due[0], MaintenanceJob::Decay);
        assert_eq!(due[3], MaintenanceJob::EdgeExtract);
    }

    #[tokio::test]
    async fn record_finished_persists_atomically() {
        let dir = tempdir().unwrap();
        let runtime = MaintenanceRuntime::load(dir.path(), MaintenanceConfig::default());
        runtime
            .record_finished(MaintenanceJob::PromoteTier, 42_000)
            .await;

        // The temp file must NOT be left behind after a successful write.
        let tmp = dir.path().join("maintenance_state.json.tmp");
        assert!(
            !tmp.exists(),
            "atomic-write temp file should be renamed away"
        );
        // The real file must exist and contain the timestamp.
        let actual = dir.path().join(STATE_FILENAME);
        assert!(actual.exists(), "state file should be written");
        let json = std::fs::read_to_string(&actual).unwrap();
        assert!(json.contains("42000"));
    }

    #[tokio::test]
    async fn state_path_returns_the_load_path() {
        let dir = tempdir().unwrap();
        let runtime = MaintenanceRuntime::load(dir.path(), MaintenanceConfig::default());
        assert_eq!(runtime.state_path(), dir.path().join(STATE_FILENAME));
    }
}
