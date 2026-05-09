//! Time-bucketed shards for long-tier memories (Chunk 41.14).
//!
//! Behind the `time-shards` feature flag. Shards the `memories` table by
//! `created_at` quarter into separate SQLite databases joined via
//! `ATTACH DATABASE`. Reads stay transparent through a temp view union;
//! writes route on `created_at`.
//!
//! File naming: `memory_long_<YYYY>q<Q>.db` (e.g. `memory_long_2026q2.db`).

use rusqlite::{params, Connection, Result as SqlResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// A quarter identifier (e.g. "2026q2").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuarterKey {
    pub year: i32,
    pub quarter: u8, // 1–4
}

impl QuarterKey {
    /// Derive a quarter key from a millisecond epoch timestamp.
    pub fn from_epoch_ms(ms: i64) -> Self {
        // Convert ms to seconds, then to a naive date.
        let secs = ms / 1000;
        // Days since Unix epoch (1970-01-01).
        let days = (secs / 86400) as i32;
        // Approximate year/month using the civil calendar algorithm.
        let (y, m, _d) = civil_from_days(days);
        let quarter = ((m - 1) / 3 + 1) as u8;
        Self { year: y, quarter }
    }

    /// The schema name used in SQLite `ATTACH` (e.g. `shard_2026q2`).
    pub fn schema_name(&self) -> String {
        format!("shard_{}q{}", self.year, self.quarter)
    }

    /// The database filename (e.g. `memory_long_2026q2.db`).
    pub fn filename(&self) -> String {
        format!("memory_long_{}q{}.db", self.year, self.quarter)
    }
}

impl std::fmt::Display for QuarterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}q{}", self.year, self.quarter)
    }
}

/// Manages attached shard databases.
#[derive(Debug)]
pub struct ShardManager {
    /// Map of quarter → whether attached.
    attached: HashMap<QuarterKey, PathBuf>,
    /// The data directory containing shard files.
    data_dir: PathBuf,
}

impl ShardManager {
    /// Create a new shard manager for the given data directory.
    pub fn new(data_dir: &Path) -> Self {
        Self {
            attached: HashMap::new(),
            data_dir: data_dir.to_path_buf(),
        }
    }

    /// Discover existing shard files on disk and attach them all.
    pub fn attach_existing_shards(&mut self, conn: &Connection) -> SqlResult<()> {
        let pattern = "memory_long_";
        let entries = std::fs::read_dir(&self.data_dir)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(pattern) && name_str.ends_with(".db") {
                // Parse quarter from filename: memory_long_2026q2.db
                if let Some(key) = parse_quarter_from_filename(&name_str) {
                    self.attach_shard(conn, &key)?;
                }
            }
        }
        Ok(())
    }

    /// Attach a specific shard database. Creates the file if it doesn't exist.
    pub fn attach_shard(&mut self, conn: &Connection, key: &QuarterKey) -> SqlResult<()> {
        if self.attached.contains_key(key) {
            return Ok(());
        }
        let path = self.data_dir.join(key.filename());
        let schema = key.schema_name();
        conn.execute(
            &format!("ATTACH DATABASE ?1 AS {schema}"),
            params![path.to_string_lossy().as_ref()],
        )?;
        // Create the memories table in the shard if it doesn't exist.
        conn.execute_batch(&shard_memories_ddl(&schema))?;
        self.attached.insert(key.clone(), path);
        Ok(())
    }

    /// Detach a shard (for cleanup or vacuum).
    pub fn detach_shard(&mut self, conn: &Connection, key: &QuarterKey) -> SqlResult<()> {
        if !self.attached.contains_key(key) {
            return Ok(());
        }
        let schema = key.schema_name();
        conn.execute_batch(&format!("DETACH DATABASE {schema};"))?;
        self.attached.remove(key);
        Ok(())
    }

    /// Get the filesystem path for a shard (for per-shard VACUUM or backup).
    pub fn shard_path(&self, key: &QuarterKey) -> PathBuf {
        self.data_dir.join(key.filename())
    }

    /// All currently attached quarter keys.
    pub fn attached_shards(&self) -> Vec<QuarterKey> {
        self.attached.keys().cloned().collect()
    }

    /// Number of attached shards.
    pub fn shard_count(&self) -> usize {
        self.attached.len()
    }

    /// Rebuild the `all_memories` temp view that unions main + all shards.
    /// Must be called after attaching/detaching shards.
    pub fn rebuild_union_view(&self, conn: &Connection) -> SqlResult<()> {
        let mut parts: Vec<String> = vec!["SELECT * FROM main.memories".to_string()];
        for key in self.attached.keys() {
            let schema = key.schema_name();
            parts.push(format!("SELECT * FROM {schema}.memories"));
        }
        let union_sql = parts.join(" UNION ALL ");
        conn.execute_batch(&format!(
            "DROP VIEW IF EXISTS all_memories;\nCREATE TEMP VIEW all_memories AS {union_sql};"
        ))
    }

    /// Determine which shard (or main) a memory write should go to.
    /// Returns `None` for main (non-long tier or current quarter),
    /// `Some(key)` for a shard.
    pub fn route_write(&self, tier: &str, created_at_ms: i64) -> Option<QuarterKey> {
        if tier != "long" {
            return None;
        }
        let key = QuarterKey::from_epoch_ms(created_at_ms);
        let current = QuarterKey::from_epoch_ms(now_ms());
        // Current quarter stays in main for fast access.
        if key == current {
            return None;
        }
        Some(key)
    }

    /// Insert a memory row into the appropriate shard table.
    /// Caller must have already attached the shard.
    pub fn insert_into_shard(
        &self,
        conn: &Connection,
        key: &QuarterKey,
        row: &ShardInsert<'_>,
    ) -> SqlResult<i64> {
        let schema = key.schema_name();
        conn.execute(
            &format!(
                "INSERT INTO {schema}.memories
                 (content, tags, importance, memory_type, created_at,
                  access_count, tier, decay_score, token_count, source_url, source_hash, expires_at)
                 VALUES (?1,?2,?3,?4,?5, 0, ?6, 1.0, ?7, ?8, ?9, ?10)"
            ),
            params![
                row.content,
                row.tags,
                row.importance,
                row.memory_type,
                row.created_at,
                row.tier,
                row.token_count,
                row.source_url,
                row.source_hash,
                row.expires_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Vacuum a single shard independently (for maintenance).
    pub fn vacuum_shard(&self, conn: &Connection, key: &QuarterKey) -> SqlResult<()> {
        let path = self.shard_path(key);
        conn.execute(
            "VACUUM ?1 INTO ?2",
            params![key.schema_name(), path.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    /// Migrate existing long-tier memories older than the current quarter
    /// from main into their respective shards.
    pub fn migrate_existing(&mut self, conn: &Connection, batch_size: usize) -> SqlResult<usize> {
        let current = QuarterKey::from_epoch_ms(now_ms());
        let current_start_ms = quarter_start_ms(current.year, current.quarter);

        // Find old long-tier entries in main that should be sharded.
        let mut stmt = conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, tier,
                    token_count, source_url, source_hash, expires_at
             FROM main.memories
             WHERE tier = 'long' AND created_at < ?1
             LIMIT ?2",
        )?;
        let rows: Vec<ShardRow> = stmt
            .query_map(params![current_start_ms, batch_size as i64], |row| {
                Ok(ShardRow {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    tags: row.get(2)?,
                    importance: row.get(3)?,
                    memory_type: row.get(4)?,
                    created_at: row.get(5)?,
                    tier: row.get(6)?,
                    token_count: row.get(7)?,
                    source_url: row.get(8)?,
                    source_hash: row.get(9)?,
                    expires_at: row.get(10)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        if rows.is_empty() {
            return Ok(0);
        }

        let mut moved = 0;
        for row in &rows {
            let key = QuarterKey::from_epoch_ms(row.created_at);
            // Ensure shard is attached.
            self.attach_shard(conn, &key)?;
            self.insert_into_shard(
                conn,
                &key,
                &ShardInsert {
                    content: &row.content,
                    tags: &row.tags,
                    importance: row.importance,
                    memory_type: &row.memory_type,
                    created_at: row.created_at,
                    tier: &row.tier,
                    token_count: row.token_count,
                    source_url: row.source_url.as_deref(),
                    source_hash: row.source_hash.as_deref(),
                    expires_at: row.expires_at,
                },
            )?;
            // Delete from main (edges stay in main — no FK issue since CASCADE
            // only fires on main.memories; sharded rows are logically separate).
            conn.execute("DELETE FROM main.memories WHERE id = ?1", params![row.id])?;
            moved += 1;
        }
        // Rebuild view after potential new shards.
        self.rebuild_union_view(conn)?;
        Ok(moved)
    }
}

/// Parameters for inserting a row into a shard.
#[derive(Debug)]
pub struct ShardInsert<'a> {
    pub content: &'a str,
    pub tags: &'a str,
    pub importance: i32,
    pub memory_type: &'a str,
    pub created_at: i64,
    pub tier: &'a str,
    pub token_count: i64,
    pub source_url: Option<&'a str>,
    pub source_hash: Option<&'a str>,
    pub expires_at: Option<i64>,
}

/// Internal struct for migration rows.
#[derive(Debug)]
struct ShardRow {
    id: i64,
    content: String,
    tags: String,
    importance: i32,
    memory_type: String,
    created_at: i64,
    tier: String,
    token_count: i64,
    source_url: Option<String>,
    source_hash: Option<String>,
    expires_at: Option<i64>,
}

/// DDL for the shard memories table (same columns, no FK references).
fn shard_memories_ddl(schema: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {schema}.memories (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            content       TEXT    NOT NULL,
            tags          TEXT    NOT NULL DEFAULT '',
            importance    INTEGER NOT NULL DEFAULT 3,
            memory_type   TEXT    NOT NULL DEFAULT 'fact',
            created_at    INTEGER NOT NULL,
            last_accessed INTEGER,
            access_count  INTEGER NOT NULL DEFAULT 0,
            embedding     BLOB,
            embedding_model_id TEXT,
            embedding_dim INTEGER,
            source_url    TEXT,
            source_hash   TEXT,
            expires_at    INTEGER,
            tier          TEXT    NOT NULL DEFAULT 'long',
            decay_score   REAL    NOT NULL DEFAULT 1.0,
            session_id    TEXT,
            parent_id     INTEGER,
            token_count   INTEGER NOT NULL DEFAULT 0,
            valid_to      INTEGER,
            obsidian_path TEXT,
            last_exported INTEGER,
            category      TEXT,
            cognitive_kind TEXT,
            updated_at    INTEGER,
            origin_device TEXT,
            protected     INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS {schema}.idx_memories_created ON memories(created_at DESC);
        CREATE INDEX IF NOT EXISTS {schema}.idx_memories_tier ON memories(tier);
        CREATE INDEX IF NOT EXISTS {schema}.idx_memories_decay ON memories(decay_score DESC);"
    )
}

/// Parse a quarter key from a shard filename like `memory_long_2026q2.db`.
fn parse_quarter_from_filename(name: &str) -> Option<QuarterKey> {
    // Expected: memory_long_YYYYqQ.db
    let stem = name.strip_prefix("memory_long_")?.strip_suffix(".db")?;
    let (year_str, q_str) = stem.split_once('q')?;
    let year: i32 = year_str.parse().ok()?;
    let quarter: u8 = q_str.parse().ok()?;
    if !(1..=4).contains(&quarter) {
        return None;
    }
    Some(QuarterKey { year, quarter })
}

/// Convert days since Unix epoch to (year, month, day) using the civil calendar algorithm.
/// Reference: Howard Hinnant's chrono-compatible algorithm.
fn civil_from_days(days: i32) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

/// Calculate the start epoch (ms) of a given quarter.
fn quarter_start_ms(year: i32, quarter: u8) -> i64 {
    let month = (quarter - 1) * 3 + 1;
    // days from 1970-01-01 to year-month-01
    let days = days_from_civil(year, month as u32, 1);
    days as i64 * 86400 * 1000
}

/// Convert (year, month, day) to days since Unix epoch.
fn days_from_civil(y: i32, m: u32, d: u32) -> i32 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i32 - 719468
}

/// The shard-aware ANN key: combines shard name with model ID.
/// Used by AnnRegistry when `time-shards` is active.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShardAnnKey {
    pub shard: Option<QuarterKey>, // None = main
    pub model_id: String,
}

impl ShardAnnKey {
    /// Filesystem stem for the usearch index file.
    pub fn file_stem(&self) -> String {
        match &self.shard {
            Some(q) => format!("vectors_{}_{}q{}", self.model_id, q.year, q.quarter),
            None => format!("vectors_{}", self.model_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        crate::memory::schema::create_canonical_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn quarter_key_from_epoch() {
        // 2026-04-15 00:00:00 UTC = approx epoch ms 1776384000000
        let key = QuarterKey::from_epoch_ms(1776384000000);
        assert_eq!(key.year, 2026);
        assert_eq!(key.quarter, 2);
        assert_eq!(key.schema_name(), "shard_2026q2");
        assert_eq!(key.filename(), "memory_long_2026q2.db");
    }

    #[test]
    fn quarter_key_from_epoch_q1() {
        // 2025-01-15 = epoch ms ~ 1736899200000
        let key = QuarterKey::from_epoch_ms(1736899200000);
        assert_eq!(key.year, 2025);
        assert_eq!(key.quarter, 1);
    }

    #[test]
    fn quarter_key_from_epoch_q4() {
        // 2025-12-01 = epoch ms ~ 1764547200000
        let key = QuarterKey::from_epoch_ms(1764547200000);
        assert_eq!(key.year, 2025);
        assert_eq!(key.quarter, 4);
    }

    #[test]
    fn parse_filename_valid() {
        let key = parse_quarter_from_filename("memory_long_2026q2.db").unwrap();
        assert_eq!(key.year, 2026);
        assert_eq!(key.quarter, 2);
    }

    #[test]
    fn parse_filename_invalid() {
        assert!(parse_quarter_from_filename("memory.db").is_none());
        assert!(parse_quarter_from_filename("memory_long_2026q5.db").is_none());
        assert!(parse_quarter_from_filename("memory_long_2026q0.db").is_none());
    }

    #[test]
    fn shard_manager_attach_and_rebuild_view() {
        let conn = test_conn();
        // Use a temp dir for shard files.
        let tmp = std::env::temp_dir().join("ts_shard_test_attach");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);
        let mut mgr = ShardManager::new(&tmp);

        let key = QuarterKey {
            year: 2025,
            quarter: 3,
        };
        mgr.attach_shard(&conn, &key).unwrap();
        assert_eq!(mgr.shard_count(), 1);

        mgr.rebuild_union_view(&conn).unwrap();
        // The view should be queryable.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM all_memories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);

        // Insert into shard.
        mgr.insert_into_shard(
            &conn,
            &key,
            &ShardInsert {
                content: "test content",
                tags: "tag1",
                importance: 3,
                memory_type: "fact",
                created_at: 1690000000000,
                tier: "long",
                token_count: 5,
                source_url: None,
                source_hash: None,
                expires_at: None,
            },
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM all_memories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // Cleanup.
        mgr.detach_shard(&conn, &key).unwrap();
        assert_eq!(mgr.shard_count(), 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn shard_manager_route_write() {
        let tmp = std::env::temp_dir();
        let mgr = ShardManager::new(&tmp);
        let now = now_ms();

        // Non-long tier always routes to main.
        assert_eq!(mgr.route_write("session", now), None);
        assert_eq!(mgr.route_write("working", now), None);

        // Current quarter long-tier routes to main.
        assert_eq!(mgr.route_write("long", now), None);

        // Old long-tier routes to shard.
        let old_ms = 1577836800000; // 2020-01-01
        let route = mgr.route_write("long", old_ms);
        assert!(route.is_some());
        let key = route.unwrap();
        assert_eq!(key.year, 2020);
        assert_eq!(key.quarter, 1);
    }

    #[test]
    fn shard_manager_migrate_existing() {
        let conn = test_conn();
        let tmp = std::env::temp_dir().join("ts_shard_test_migrate");
        // Clean up any leftover shard files from prior runs.
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);
        let mut mgr = ShardManager::new(&tmp);

        // Insert some old long-tier memories into main.
        let old_ts = 1577836800000i64; // 2020-01-01
        for i in 0..5 {
            conn.execute(
                "INSERT INTO memories (content, tags, importance, memory_type, created_at,
                 access_count, tier, decay_score, token_count)
                 VALUES (?1, '', 3, 'fact', ?2, 0, 'long', 1.0, 10)",
                params![format!("old memory {i}"), old_ts + i * 1000],
            )
            .unwrap();
        }
        // Insert a current-quarter memory (should NOT be migrated).
        let now_ts = now_ms();
        conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at,
             access_count, tier, decay_score, token_count)
             VALUES ('recent', '', 3, 'fact', ?1, 0, 'long', 1.0, 10)",
            params![now_ts],
        )
        .unwrap();

        let moved = mgr.migrate_existing(&conn, 100).unwrap();
        assert_eq!(moved, 5);

        // Main should only have the recent one.
        let main_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM main.memories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(main_count, 1);

        // View should show all 6.
        let view_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM all_memories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(view_count, 6);

        // Shard should have 5.
        let key = QuarterKey::from_epoch_ms(old_ts);
        let shard_count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {}.memories", key.schema_name()),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(shard_count, 5);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn shard_ann_key_file_stem() {
        let key = ShardAnnKey {
            shard: Some(QuarterKey {
                year: 2026,
                quarter: 2,
            }),
            model_id: "nomic-embed-text".to_string(),
        };
        assert_eq!(key.file_stem(), "vectors_nomic-embed-text_2026q2");

        let main_key = ShardAnnKey {
            shard: None,
            model_id: "nomic-embed-text".to_string(),
        };
        assert_eq!(main_key.file_stem(), "vectors_nomic-embed-text");
    }

    #[test]
    fn quarter_boundary_rotation() {
        // Q4 2025 end → Q1 2026 start
        // 2025-12-31 23:59:59 UTC ≈ 1767225599000
        let q4_end = QuarterKey::from_epoch_ms(1767225599000);
        assert_eq!(q4_end.year, 2025);
        assert_eq!(q4_end.quarter, 4);

        // 2026-01-01 00:00:00 UTC ≈ 1767225600000
        let q1_start = QuarterKey::from_epoch_ms(1767225600000);
        assert_eq!(q1_start.year, 2026);
        assert_eq!(q1_start.quarter, 1);
    }

    #[test]
    fn discover_existing_shards() {
        let tmp = std::env::temp_dir().join("ts_shard_test_discover");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        // Create fake shard files.
        std::fs::write(tmp.join("memory_long_2025q1.db"), b"").unwrap();
        std::fs::write(tmp.join("memory_long_2025q3.db"), b"").unwrap();
        std::fs::write(tmp.join("unrelated.db"), b"").unwrap();

        let conn = test_conn();
        let mut mgr = ShardManager::new(&tmp);
        mgr.attach_existing_shards(&conn).unwrap();
        assert_eq!(mgr.shard_count(), 2);

        let keys = mgr.attached_shards();
        assert!(keys.iter().any(|k| k.year == 2025 && k.quarter == 1));
        assert!(keys.iter().any(|k| k.year == 2025 && k.quarter == 3));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
