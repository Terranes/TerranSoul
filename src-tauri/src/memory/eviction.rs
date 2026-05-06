//! Capacity-based memory eviction (Chunk 38.4).
//!
//! Enforces a hard cap on long-tier memories (`MAX_LONG_TERM_ENTRIES`). When
//! the count exceeds the cap, the lowest-value rows are evicted until the
//! count falls to `cap × target_ratio` (default 0.95).
//!
//! **Important data is never evicted:**
//! - Rows with `importance >= 4` (on 1–5 scale, i.e. high importance).
//! - Rows with `protected = 1` (user/agent-pinned).
//! - Tier `working` and `short` are exempt (they have their own lifecycle).
//!
//! Eviction is batched (1 000 rows per pass) and logged to an append-only
//! JSONL audit file.

use rusqlite::{params, Connection, Result as SqlResult};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Default maximum number of long-tier memory entries.
pub const DEFAULT_MAX_LONG_TERM: u64 = 1_000_000;

/// After eviction, reduce to this fraction of the cap.
pub const DEFAULT_TARGET_RATIO: f64 = 0.95;

/// Eviction audit log filename (appended in data_dir).
const EVICTION_LOG_FILE: &str = "eviction_log.jsonl";

/// Max audit log size before rotation (10 MB).
const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;

/// Report of an eviction pass.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvictionReport {
    pub timestamp_ms: u64,
    pub long_count_before: u64,
    pub dropped: u64,
    pub kept_protected: u64,
    pub kept_important: u64,
    pub long_count_after: u64,
}

/// Enforce the capacity cap on long-tier memories.
///
/// Returns `None` if no eviction was needed, or `Some(report)` if rows were evicted.
pub fn enforce_capacity(
    conn: &Connection,
    cap: u64,
    target_ratio: f64,
    data_dir: &Path,
) -> SqlResult<Option<EvictionReport>> {
    let long_count: u64 = conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE tier = 'long'",
        [],
        |r| r.get(0),
    )?;

    if long_count <= cap {
        return Ok(None);
    }

    let target = (cap as f64 * target_ratio) as u64;
    let to_evict = long_count.saturating_sub(target);

    // Count protected rows (to report, not to delete).
    let kept_protected: u64 = conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE tier = 'long' AND protected = 1",
        [],
        |r| r.get(0),
    )?;
    let kept_important: u64 = conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE tier = 'long' AND importance >= 4 AND protected = 0",
        [],
        |r| r.get(0),
    )?;

    // Evict in a single statement: select bottom-N by eviction score and delete.
    // Single pass through the eligible set instead of N batches × full sort.
    // Eviction score = importance * decay_score * recency_factor (lowest first).
    // Protected (protected=1) and high-importance (importance>=4) rows are excluded.
    let total_dropped = conn.execute(
        "DELETE FROM memories WHERE id IN (
            SELECT id FROM memories
            WHERE tier = 'long'
              AND protected = 0
              AND importance < 4
            ORDER BY
              (importance * decay_score * (1.0 / (1.0 + (CAST((?1 - COALESCE(last_accessed, created_at)) AS REAL) / 2592000000.0)))) ASC
            LIMIT ?2
        )",
        params![now_epoch_ms() as i64, to_evict as i64],
    )? as u64;

    let long_count_after: u64 = conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE tier = 'long'",
        [],
        |r| r.get(0),
    )?;

    let report = EvictionReport {
        timestamp_ms: now_epoch_ms(),
        long_count_before: long_count,
        dropped: total_dropped,
        kept_protected,
        kept_important,
        long_count_after,
    };

    // Write audit log.
    if let Err(e) = append_eviction_log(data_dir, &report) {
        eprintln!("[eviction] failed to write audit log: {e}");
    }

    Ok(Some(report))
}

/// Read the most recent eviction log entries (newest first).
pub fn read_eviction_log(data_dir: &Path, limit: usize) -> Vec<EvictionReport> {
    let log_path = data_dir.join(EVICTION_LOG_FILE);
    let Ok(content) = std::fs::read_to_string(&log_path) else {
        return vec![];
    };
    let mut entries: Vec<EvictionReport> = content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    entries.reverse(); // newest first
    entries.truncate(limit);
    entries
}

/// Path to the eviction log file.
pub fn eviction_log_path(data_dir: &Path) -> PathBuf {
    data_dir.join(EVICTION_LOG_FILE)
}

// ── Internal ───────────────────────────────────────────────────────────────────

fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn append_eviction_log(data_dir: &Path, report: &EvictionReport) -> std::io::Result<()> {
    let log_path = data_dir.join(EVICTION_LOG_FILE);

    // Rotate if over size limit.
    if let Ok(meta) = std::fs::metadata(&log_path) {
        if meta.len() >= MAX_LOG_BYTES {
            let archive = data_dir.join("eviction_log.jsonl.1");
            let _ = std::fs::rename(&log_path, &archive);
        }
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let line = serde_json::to_string(report).map_err(std::io::Error::other)?;
    writeln!(file, "{line}")?;
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::schema::create_canonical_schema;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_canonical_schema(&conn).unwrap();
        conn
    }

    fn insert_memory(conn: &Connection, id: i64, importance: i64, protected: i64) {
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count, protected)
             VALUES (?1, ?2, 'test', ?3, 'fact', 1000000000, 'long', 0.5, 10, ?4)",
            params![id, format!("memory {id}"), importance, protected],
        )
        .unwrap();
    }

    #[test]
    fn no_eviction_when_under_cap() {
        let conn = test_db();
        for i in 1..=100 {
            insert_memory(&conn, i, 3, 0);
        }
        let dir = tempfile::tempdir().unwrap();
        let result = enforce_capacity(&conn, 200, 0.95, dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn evicts_to_target_ratio() {
        let conn = test_db();
        // Insert 1050 rows with low importance.
        for i in 1..=1050 {
            insert_memory(&conn, i, 2, 0);
        }
        let dir = tempfile::tempdir().unwrap();
        let result = enforce_capacity(&conn, 1000, 0.95, dir.path()).unwrap();
        let report = result.unwrap();
        assert_eq!(report.long_count_before, 1050);
        assert!(report.dropped >= 100, "should drop at least 100 rows");
        assert!(
            report.long_count_after <= 950,
            "after eviction: {}",
            report.long_count_after
        );
    }

    #[test]
    fn preserves_important_and_protected() {
        let conn = test_db();
        // Insert 1050 rows: 100 with importance>=4, 100 protected, 850 evictable.
        for i in 1..=100 {
            insert_memory(&conn, i, 5, 0); // high importance
        }
        for i in 101..=200 {
            insert_memory(&conn, i, 2, 1); // protected
        }
        for i in 201..=1050 {
            insert_memory(&conn, i, 2, 0); // evictable
        }
        let dir = tempfile::tempdir().unwrap();
        let result = enforce_capacity(&conn, 1000, 0.95, dir.path()).unwrap();
        let report = result.unwrap();

        // All 200 preserved rows should survive.
        let important_count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE tier='long' AND importance >= 4",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let protected_count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE tier='long' AND protected = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(important_count, 100);
        assert_eq!(protected_count, 100);
        assert!(report.dropped > 0);
    }

    #[test]
    fn audit_log_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let report = EvictionReport {
            timestamp_ms: 1234567890,
            long_count_before: 1050,
            dropped: 100,
            kept_protected: 50,
            kept_important: 30,
            long_count_after: 950,
        };
        append_eviction_log(dir.path(), &report).unwrap();
        let entries = read_eviction_log(dir.path(), 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].dropped, 100);
        assert_eq!(entries[0].long_count_before, 1050);
    }

    #[test]
    fn stops_when_all_remaining_are_protected() {
        let conn = test_db();
        // Insert 1050 rows where 200 are evictable and 850 are protected.
        for i in 1..=200 {
            insert_memory(&conn, i, 2, 0); // evictable
        }
        for i in 201..=1050 {
            insert_memory(&conn, i, 2, 1); // protected — can't evict
        }
        let dir = tempfile::tempdir().unwrap();
        let result = enforce_capacity(&conn, 800, 0.95, dir.path()).unwrap();
        let report = result.unwrap();
        // Need to drop 1050 - 760 = 290, but only 200 are evictable.
        // It drops all 200 and stops.
        assert_eq!(report.dropped, 200);
        // Count should be 850 (all protected survived).
        let count: u64 = conn
            .query_row("SELECT COUNT(*) FROM memories WHERE tier='long'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 850);
    }
}
