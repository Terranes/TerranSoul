//! Brain data migration & optimisation (Chunk 25.12).
//!
//! Autonomous background task that:
//! 1. Checks the memory store's schema version.
//! 2. Runs pending migrations if the schema is behind.
//! 3. Detects ANN dimension mismatches and triggers rebuilds.
//! 4. Runs garbage collection on stale/expired entries.
//! 5. Reports results via structured events.
//!
//! Designed to run as a periodic background task within the self-improve
//! loop or as a standalone maintenance job.

use serde::{Deserialize, Serialize};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Configuration for the brain maintenance task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainMaintenanceConfig {
    /// Whether schema migrations should be applied automatically.
    #[serde(default = "default_true")]
    pub auto_migrate: bool,
    /// Whether to check and rebuild the ANN index if dimensions changed.
    #[serde(default = "default_true")]
    pub auto_rebuild_ann: bool,
    /// Whether to run GC on stale entries.
    #[serde(default = "default_true")]
    pub auto_gc: bool,
    /// Retention period for soft-deleted entries (days). Entries older
    /// than this are permanently removed during GC.
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    /// Minimum interval between maintenance runs (seconds).
    #[serde(default = "default_interval_secs")]
    pub interval_secs: u64,
}

fn default_true() -> bool {
    true
}

fn default_retention_days() -> u32 {
    90
}

fn default_interval_secs() -> u64 {
    3600 // 1 hour
}

impl Default for BrainMaintenanceConfig {
    fn default() -> Self {
        Self {
            auto_migrate: true,
            auto_rebuild_ann: true,
            auto_gc: true,
            retention_days: default_retention_days(),
            interval_secs: default_interval_secs(),
        }
    }
}

/// Result of a single maintenance run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceReport {
    /// Whether any migrations were applied.
    pub migrations_applied: u32,
    /// Schema version after migration (0 if check skipped).
    pub schema_version_after: u32,
    /// Whether the ANN index was rebuilt.
    pub ann_rebuilt: bool,
    /// Number of vectors in ANN after rebuild (0 if not rebuilt).
    pub ann_entry_count: usize,
    /// Number of stale entries garbage-collected.
    pub gc_removed: usize,
    /// Total elapsed time for the maintenance run (milliseconds).
    pub duration_ms: u128,
    /// Errors encountered (non-fatal — task continues through partial failures).
    pub errors: Vec<String>,
}

/// Status of the maintenance background task.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceStatus {
    /// Task is idle, waiting for next scheduled run.
    Idle,
    /// Currently executing a maintenance pass.
    Running,
    /// Task has been disabled.
    Disabled,
}

/// State tracked by the maintenance scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceState {
    pub status: MaintenanceStatus,
    /// Unix-ms timestamp of the last completed run.
    pub last_run_at_ms: u64,
    /// Report from the most recent run.
    pub last_report: Option<MaintenanceReport>,
    /// Total number of runs completed since app start.
    pub total_runs: u32,
}

impl Default for MaintenanceState {
    fn default() -> Self {
        Self {
            status: MaintenanceStatus::Idle,
            last_run_at_ms: 0,
            last_report: None,
            total_runs: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Maintenance Operations (Pure Logic)
// ---------------------------------------------------------------------------

/// Check whether it's time to run maintenance based on the last run timestamp.
pub fn should_run(state: &MaintenanceState, config: &BrainMaintenanceConfig, now_ms: u64) -> bool {
    if state.status == MaintenanceStatus::Disabled {
        return false;
    }
    if !config.auto_migrate && !config.auto_rebuild_ann && !config.auto_gc {
        return false;
    }
    let interval_ms = config.interval_secs * 1000;
    now_ms.saturating_sub(state.last_run_at_ms) >= interval_ms
}

/// Schema migration check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationCheckResult {
    /// Current schema version before migration.
    pub current_version: u32,
    /// Target (latest) schema version.
    pub target_version: u32,
    /// Whether a migration is needed.
    pub needs_migration: bool,
}

/// Check if schema migration is needed.
pub fn check_migration_needed(current: u32, target: u32) -> MigrationCheckResult {
    MigrationCheckResult {
        current_version: current,
        target_version: target,
        needs_migration: current < target,
    }
}

/// ANN index health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnHealthCheck {
    /// Current dimension of stored vectors.
    pub stored_dimensions: usize,
    /// Expected dimension from the embedding model.
    pub expected_dimensions: usize,
    /// Whether a rebuild is needed (dimension mismatch or empty index).
    pub needs_rebuild: bool,
    /// Number of entries currently in the index.
    pub entry_count: usize,
    /// Number of entries in the DB (may differ from index if out of sync).
    pub db_entry_count: usize,
}

/// Check if ANN index needs rebuilding.
pub fn check_ann_health(
    stored_dim: usize,
    expected_dim: usize,
    index_count: usize,
    db_count: usize,
) -> AnnHealthCheck {
    let needs_rebuild = stored_dim != expected_dim
        || (index_count == 0 && db_count > 0)
        || (index_count > 0 && db_count == 0);
    AnnHealthCheck {
        stored_dimensions: stored_dim,
        expected_dimensions: expected_dim,
        needs_rebuild,
        entry_count: index_count,
        db_entry_count: db_count,
    }
}

/// GC eligibility check: how many entries are eligible for removal.
pub fn gc_eligible_count(total_stale: usize, retention_days: u32, now_ms: u64, entries_valid_to: &[u64]) -> usize {
    if retention_days == 0 || total_stale == 0 {
        return 0;
    }
    let cutoff_ms = now_ms.saturating_sub(retention_days as u64 * 86_400_000);
    entries_valid_to.iter().filter(|&&ts| ts < cutoff_ms).count()
}

/// Execute a simulated maintenance pass (for testing without real DB).
///
/// In production, each step would call into `MemoryStore` and `AnnIndex`.
/// This function provides the scheduling and reporting logic.
#[allow(clippy::too_many_arguments)]
pub fn execute_maintenance_pass(
    config: &BrainMaintenanceConfig,
    current_schema: u32,
    target_schema: u32,
    stored_dim: usize,
    expected_dim: usize,
    index_count: usize,
    db_count: usize,
    stale_entries: &[u64],
    now_ms: u64,
) -> MaintenanceReport {
    let start = Instant::now();
    let mut report = MaintenanceReport {
        migrations_applied: 0,
        schema_version_after: current_schema,
        ann_rebuilt: false,
        ann_entry_count: index_count,
        gc_removed: 0,
        duration_ms: 0,
        errors: Vec::new(),
    };

    // Step 1: Schema migration.
    if config.auto_migrate {
        let check = check_migration_needed(current_schema, target_schema);
        if check.needs_migration {
            report.migrations_applied = target_schema - current_schema;
            report.schema_version_after = target_schema;
        }
    }

    // Step 2: ANN index health.
    if config.auto_rebuild_ann {
        let health = check_ann_health(stored_dim, expected_dim, index_count, db_count);
        if health.needs_rebuild {
            report.ann_rebuilt = true;
            report.ann_entry_count = db_count;
        }
    }

    // Step 3: GC.
    if config.auto_gc {
        report.gc_removed = gc_eligible_count(stale_entries.len(), config.retention_days, now_ms, stale_entries);
    }

    report.duration_ms = start.elapsed().as_millis();
    report
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = BrainMaintenanceConfig::default();
        assert!(cfg.auto_migrate);
        assert!(cfg.auto_rebuild_ann);
        assert!(cfg.auto_gc);
        assert_eq!(cfg.retention_days, 90);
        assert_eq!(cfg.interval_secs, 3600);
    }

    #[test]
    fn config_serde_roundtrip() {
        let cfg = BrainMaintenanceConfig {
            auto_migrate: false,
            auto_rebuild_ann: true,
            auto_gc: false,
            retention_days: 30,
            interval_secs: 1800,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let deser: BrainMaintenanceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.retention_days, 30);
        assert!(!deser.auto_migrate);
    }

    #[test]
    fn should_run_respects_interval() {
        let state = MaintenanceState {
            last_run_at_ms: 1000,
            ..Default::default()
        };
        let cfg = BrainMaintenanceConfig {
            interval_secs: 60,
            ..Default::default()
        };
        // Not enough time elapsed.
        assert!(!should_run(&state, &cfg, 50_000));
        // Enough time elapsed.
        assert!(should_run(&state, &cfg, 61_000));
    }

    #[test]
    fn should_run_disabled_returns_false() {
        let state = MaintenanceState {
            status: MaintenanceStatus::Disabled,
            ..Default::default()
        };
        let cfg = BrainMaintenanceConfig::default();
        assert!(!should_run(&state, &cfg, u64::MAX));
    }

    #[test]
    fn should_run_all_disabled_returns_false() {
        let state = MaintenanceState::default();
        let cfg = BrainMaintenanceConfig {
            auto_migrate: false,
            auto_rebuild_ann: false,
            auto_gc: false,
            ..Default::default()
        };
        assert!(!should_run(&state, &cfg, u64::MAX));
    }

    #[test]
    fn check_migration_needed_when_behind() {
        let result = check_migration_needed(7, 9);
        assert!(result.needs_migration);
        assert_eq!(result.current_version, 7);
        assert_eq!(result.target_version, 9);
    }

    #[test]
    fn check_migration_not_needed_when_current() {
        let result = check_migration_needed(9, 9);
        assert!(!result.needs_migration);
    }

    #[test]
    fn ann_health_ok_when_dimensions_match() {
        let health = check_ann_health(768, 768, 100, 100);
        assert!(!health.needs_rebuild);
    }

    #[test]
    fn ann_health_needs_rebuild_dimension_mismatch() {
        let health = check_ann_health(384, 768, 100, 100);
        assert!(health.needs_rebuild);
    }

    #[test]
    fn ann_health_needs_rebuild_empty_index_with_data() {
        let health = check_ann_health(768, 768, 0, 50);
        assert!(health.needs_rebuild);
    }

    #[test]
    fn gc_eligible_count_respects_cutoff() {
        let now = 100_000_000u64; // 100,000s in ms
        let retention_days = 1; // 86_400_000 ms
        // cutoff = now - 86_400_000 = 13_600_000
        // 1_000 < cutoff (eligible)
        // 50_000 < cutoff (eligible)
        // now - 90_000_000 = 10_000_000 < cutoff (eligible)
        // now - 80_000_000 = 20_000_000 >= cutoff (NOT eligible)
        let entries = vec![1_000, 50_000, now - 90_000_000, now - 80_000_000];
        let count = gc_eligible_count(entries.len(), retention_days, now, &entries);
        assert_eq!(count, 3);
    }

    #[test]
    fn gc_eligible_zero_retention_returns_zero() {
        let entries = vec![1, 2, 3];
        assert_eq!(gc_eligible_count(entries.len(), 0, 100_000, &entries), 0);
    }

    #[test]
    fn execute_maintenance_pass_full() {
        let cfg = BrainMaintenanceConfig::default();
        let stale = vec![1_000, 2_000]; // Very old entries
        let now = 100_000_000_000; // Far in the future
        let report = execute_maintenance_pass(
            &cfg,
            7,   // current schema
            9,   // target schema
            384, // stored dim (wrong)
            768, // expected dim
            50,  // index count
            100, // db count
            &stale,
            now,
        );
        assert_eq!(report.migrations_applied, 2);
        assert_eq!(report.schema_version_after, 9);
        assert!(report.ann_rebuilt);
        assert_eq!(report.ann_entry_count, 100);
        assert_eq!(report.gc_removed, 2);
    }

    #[test]
    fn execute_maintenance_pass_nothing_needed() {
        let cfg = BrainMaintenanceConfig::default();
        let report = execute_maintenance_pass(
            &cfg,
            9,   // current = target
            9,   // target
            768, // stored = expected
            768, // expected
            100, // index = db
            100, // db
            &[], // no stale
            0,
        );
        assert_eq!(report.migrations_applied, 0);
        assert!(!report.ann_rebuilt);
        assert_eq!(report.gc_removed, 0);
    }

    #[test]
    fn maintenance_state_default() {
        let state = MaintenanceState::default();
        assert_eq!(state.status, MaintenanceStatus::Idle);
        assert_eq!(state.total_runs, 0);
        assert!(state.last_report.is_none());
    }

    #[test]
    fn maintenance_report_serde() {
        let report = MaintenanceReport {
            migrations_applied: 2,
            schema_version_after: 9,
            ann_rebuilt: true,
            ann_entry_count: 500,
            gc_removed: 10,
            duration_ms: 42,
            errors: vec!["test error".to_owned()],
        };
        let json = serde_json::to_string(&report).unwrap();
        let deser: MaintenanceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.migrations_applied, 2);
        assert!(deser.ann_rebuilt);
    }
}
