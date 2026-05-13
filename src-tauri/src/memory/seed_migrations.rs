//! Single-shot shared seed bootstrap for MCP brain databases.
//!
//! TerranSoul now uses one init snapshot (`mcp-data/shared/memory-seed.sql`)
//! rather than replaying numbered seed migrations.

use rusqlite::Connection;
use std::path::Path;

const INIT_SNAPSHOT_FILE: &str = "memory-seed.sql";
const INIT_SEED_VERSION: u32 = 1;

fn fnv1a_hex(data: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn ensure_seed_state_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS seed_migrations (
            version     INTEGER PRIMARY KEY,
            name        TEXT    NOT NULL,
            applied_at  INTEGER NOT NULL,
            checksum    TEXT    NOT NULL
        );",
    )
    .map_err(|e| format!("create seed state table: {e}"))
}

fn current_seed_version(conn: &Connection) -> Result<u32, String> {
    ensure_seed_state_table(conn)?;
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM seed_migrations",
        [],
        |row| row.get(0),
    )
    .map_err(|e| format!("query seed version: {e}"))
}

fn existing_memory_rows(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(1) FROM memories", [], |row| row.get(0))
        .unwrap_or(0)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn load_init_snapshot_sql(shared_dir: &Path) -> String {
    let disk = shared_dir.join(INIT_SNAPSHOT_FILE);
    std::fs::read_to_string(disk)
        .unwrap_or_else(|_| include_str!("../../../mcp-data/shared/memory-seed.sql").to_string())
}

fn mark_seed_applied(conn: &Connection, name: &str, checksum: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO seed_migrations (version, name, applied_at, checksum)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![INIT_SEED_VERSION, name, now_ms(), checksum],
    )
    .map(|_| ())
    .map_err(|e| format!("record init seed state: {e}"))
}

/// Apply only the consolidated init snapshot seed once.
///
/// Returns `(applied_count, current_version)` where `applied_count` is either
/// `1` when the snapshot was executed in this run or `0` otherwise.
pub fn run_all(conn: &Connection, shared_dir: &Path) -> Result<(usize, u32), String> {
    let current = current_seed_version(conn)?;
    if current >= INIT_SEED_VERSION {
        return Ok((0, current));
    }

    if existing_memory_rows(conn) > 0 {
        // Existing DB without seed state metadata: do not replay the full seed.
        mark_seed_applied(conn, "init_seed_existing_db", "existing-db")?;
        return Ok((0, INIT_SEED_VERSION));
    }

    let snapshot_sql = load_init_snapshot_sql(shared_dir);
    let checksum = fnv1a_hex(snapshot_sql.as_bytes());

    conn.execute_batch("SAVEPOINT init_seed_snapshot;")
        .map_err(|e| format!("savepoint init seed: {e}"))?;

    match conn.execute_batch(&snapshot_sql) {
        Ok(()) => {
            if let Err(e) = mark_seed_applied(conn, "init_seed_snapshot", &checksum) {
                let _ = conn.execute_batch("ROLLBACK TO init_seed_snapshot;");
                let _ = conn.execute_batch("RELEASE init_seed_snapshot;");
                return Err(e);
            }

            conn.execute_batch("RELEASE init_seed_snapshot;")
                .map_err(|e| format!("release init seed: {e}"))?;
            eprintln!(
                "[seed-init] applied snapshot from {}",
                shared_dir.join(INIT_SNAPSHOT_FILE).display()
            );
            Ok((1, INIT_SEED_VERSION))
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK TO init_seed_snapshot;");
            let _ = conn.execute_batch("RELEASE init_seed_snapshot;");
            Err(format!("init seed failed: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::memory::schema::create_canonical_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn fresh_db_applies_seed_once() {
        let conn = mem_conn();
        let tmp = tempfile::tempdir().unwrap();

        let (applied, version) = run_all(&conn, tmp.path()).unwrap();
        assert_eq!(applied, 1);
        assert_eq!(version, INIT_SEED_VERSION);

        let memory_count: i64 = conn
            .query_row("SELECT COUNT(1) FROM memories", [], |row| row.get(0))
            .unwrap();
        assert!(memory_count > 0);

        let (again, version_again) = run_all(&conn, tmp.path()).unwrap();
        assert_eq!(again, 0);
        assert_eq!(version_again, INIT_SEED_VERSION);
    }

    #[test]
    fn existing_db_is_marked_without_reseed() {
        let conn = mem_conn();
        conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
             VALUES ('existing', 'test', 5, 'fact', 0, 'long', 1.0, 1)",
            [],
        )
        .unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let (applied, version) = run_all(&conn, tmp.path()).unwrap();

        assert_eq!(applied, 0);
        assert_eq!(version, INIT_SEED_VERSION);

        let memory_count: i64 = conn
            .query_row("SELECT COUNT(1) FROM memories", [], |row| row.get(0))
            .unwrap();
        assert_eq!(memory_count, 1);
    }

    /// Validate that every INSERT column in `memory-seed.sql` actually exists
    /// in the canonical schema.  This catches typos like `char_count` (should
    /// be `token_count`), `last_accessed_at` (should be `last_accessed`), and
    /// `source` on `memories` (only exists on `memory_edges`).
    #[test]
    fn seed_sql_columns_match_canonical_schema() {
        let conn = mem_conn();

        // Build a map of table → column set from the canonical schema.
        let mut table_columns: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        for table in &tables {
            let cols: std::collections::HashSet<String> = conn
                .prepare(&format!("PRAGMA table_info(\"{}\")", table))
                .unwrap()
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .collect::<Result<std::collections::HashSet<_>, _>>()
                .unwrap();
            table_columns.insert(table.clone(), cols);
        }

        // Parse every INSERT statement in the compiled-in seed SQL.
        // We process line-by-line and only match lines whose first
        // non-whitespace token is INSERT (skips comments and string content).
        let seed_sql = include_str!("../../../mcp-data/shared/memory-seed.sql");

        let mut errors = Vec::new();

        for line in seed_sql.lines() {
            let trimmed = line.trim();
            // Skip comments and blank lines
            if trimmed.is_empty() || trimmed.starts_with("--") {
                continue;
            }
            let lower = trimmed.to_lowercase();
            // Only match lines that begin with INSERT
            if !lower.starts_with("insert") {
                continue;
            }
            // Find "into <table> (<cols>)"
            let Some(into_pos) = lower.find("into ") else { continue };
            let after_into = into_pos + 5;
            let rest = &lower[after_into..];
            let table_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(rest.len());
            let table = &rest[..table_end];
            if table.is_empty() { continue; }

            let Some(paren_offset) = rest[table_end..].find('(') else { continue };
            let col_start = table_end + paren_offset + 1;
            let Some(paren_close) = rest[col_start..].find(')') else { continue };
            let col_list = &rest[col_start..col_start + paren_close];
            let columns: Vec<&str> = col_list.split(',').map(|c| c.trim()).collect();

            if let Some(schema_cols) = table_columns.get(table) {
                for col in &columns {
                    if !col.is_empty() && !schema_cols.contains(*col) {
                        errors.push(format!(
                            "INSERT INTO {table} references unknown column `{col}`"
                        ));
                    }
                }
            }
        }

        // De-duplicate: the same bad column may appear multiple times
        errors.sort();
        errors.dedup();

        assert!(
            errors.is_empty(),
            "memory-seed.sql has column mismatches against canonical schema:\n  {}",
            errors.join("\n  ")
        );
    }

    /// Verify the compiled-in seed SQL applies without error on a fresh
    /// canonical schema.  This is the compile-time guard against column
    /// mismatches that would otherwise surface only at MCP startup.
    #[test]
    fn compiled_seed_applies_to_canonical_schema() {
        let conn = mem_conn();
        let seed_sql = include_str!("../../../mcp-data/shared/memory-seed.sql");
        conn.execute_batch(seed_sql).unwrap_or_else(|e| {
            panic!(
                "memory-seed.sql failed on canonical schema — \
                 this means the MCP tray will fail at startup.\n\
                 SQLite error: {e}"
            );
        });

        let count: i64 = conn
            .query_row("SELECT COUNT(1) FROM memories", [], |row| row.get(0))
            .unwrap();
        assert!(
            count > 0,
            "seed should insert at least one memory row"
        );

        let edge_count: i64 = conn
            .query_row("SELECT COUNT(1) FROM memory_edges", [], |row| row.get(0))
            .unwrap();
        assert!(
            edge_count > 0,
            "seed should insert at least one edge row"
        );
    }
}
