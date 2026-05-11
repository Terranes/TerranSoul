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
}
