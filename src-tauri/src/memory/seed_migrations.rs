//! Versioned seed-data migration runner for MCP brain databases.
//!
//! Instead of a monolithic `memory-seed.sql` that only runs on a fresh
//! DB, this module tracks which seed migrations have been applied via a
//! `seed_migrations` table and runs only the new ones on each startup.
//!
//! ## Directory layout
//!
//! ```text
//! mcp-data/shared/migrations/
//!   001_initial_seed.sql
//!   002_obsidian_export_knowledge.sql
//!   003_preflight_enforcement.sql
//!   ...
//! ```
//!
//! Each file is a plain SQL script. Filenames must start with a
//! zero-padded 3-digit version number followed by `_`. The runner
//! sorts by version number and applies them in order.
//!
//! ## Guarantees
//!
//! - **Idempotent**: each migration runs inside a transaction;
//!   `INSERT OR IGNORE` / `CREATE TABLE IF NOT EXISTS` are recommended.
//! - **Append-only**: never edit an already-shipped migration. Add a
//!   new one instead. Old migrations are never re-run.
//! - **Offline-safe**: no network calls. Pure SQLite operations.
//! - **Backward-compatible**: the `seed_migrations` table is created
//!   automatically if missing (bootstraps existing DBs that predate
//!   this system).

use rusqlite::Connection;
use std::path::Path;

const INIT_SNAPSHOT_FILE: &str = "memory-seed.sql";

/// Ensure the tracking table exists. Safe to call on every startup.
pub fn ensure_migration_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS seed_migrations (
            version     INTEGER PRIMARY KEY,
            name        TEXT    NOT NULL,
            applied_at  INTEGER NOT NULL,
            checksum    TEXT    NOT NULL
        );",
    )
    .map_err(|e| format!("create seed_migrations table: {e}"))
}

/// Return the highest version number that has been applied, or 0 if
/// the table is empty (fresh DB or pre-migration DB).
pub fn current_version(conn: &Connection) -> Result<u32, String> {
    ensure_migration_table(conn)?;
    let v: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM seed_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("query seed version: {e}"))?;
    Ok(v)
}

/// A single parsed migration file.
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: u32,
    pub name: String,
    pub sql: String,
    pub checksum: String,
}

/// Simple FNV-1a hash of the SQL content, hex-encoded. Good enough for
/// tamper detection without pulling in a crypto crate.
fn fnv1a_hex(data: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Parse a migration filename like `001_initial_seed.sql` into
/// `(version=1, name="initial_seed")`.
fn parse_filename(name: &str) -> Option<(u32, String)> {
    let stem = name.strip_suffix(".sql")?;
    let underscore = stem.find('_')?;
    let version: u32 = stem[..underscore].parse().ok()?;
    let label = stem[underscore + 1..].to_string();
    Some((version, label))
}

/// Discover migration files from a directory on disk (the committed
/// `mcp-data/shared/migrations/` folder). Returns them sorted by
/// version number.
pub fn discover_migrations(dir: &Path) -> Vec<Migration> {
    let mut migrations = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return migrations,
    };
    for entry in entries.flatten() {
        let fname = entry.file_name().to_string_lossy().to_string();
        if !fname.ends_with(".sql") {
            continue;
        }
        if let Some((version, name)) = parse_filename(&fname) {
            if let Ok(sql) = std::fs::read_to_string(entry.path()) {
                let checksum = fnv1a_hex(sql.as_bytes());
                migrations.push(Migration {
                    version,
                    name,
                    sql,
                    checksum,
                });
            }
        }
    }
    migrations.sort_by_key(|m| m.version);
    migrations
}

/// Compiled-in fallback migrations for when the `mcp-data/shared/migrations/`
/// directory is not on disk (e.g. release builds, CI, fresh clones before
/// the directory is created). Each tuple is `(version, name, sql)`.
///
/// Keep this list in sync with the files on disk. The runner prefers
/// disk files when available so contributors can iterate without
/// recompiling.
pub fn compiled_migrations() -> Vec<Migration> {
    let entries: &[(&str, &str)] = &[
        (
            "001_initial_seed",
            include_str!("../../../mcp-data/shared/migrations/001_initial_seed.sql"),
        ),
        (
            "002_refresh_seed_facts",
            include_str!("../../../mcp-data/shared/migrations/002_refresh_seed_facts.sql"),
        ),
        (
            "003_health_response_descriptions",
            include_str!(
                "../../../mcp-data/shared/migrations/003_health_response_descriptions.sql"
            ),
        ),
        (
            "004_million_knowledge_crud_audit",
            include_str!(
                "../../../mcp-data/shared/migrations/004_million_knowledge_crud_audit.sql"
            ),
        ),
        (
            "005_db_strategy_audit",
            include_str!("../../../mcp-data/shared/migrations/005_db_strategy_audit.sql"),
        ),
        (
            "006_phase_41_result",
            include_str!("../../../mcp-data/shared/migrations/006_phase_41_result.sql"),
        ),
        (
            "007_phase_41_2_3_result",
            include_str!(
                "../../../mcp-data/shared/migrations/007_phase_41_2_3_result.sql"
            ),
        ),
        (
            "008_phase_41_audit_refresh",
            include_str!(
                "../../../mcp-data/shared/migrations/008_phase_41_audit_refresh.sql"
            ),
        ),
        (
            "009_benchmark_doc_refresh",
            include_str!(
                "../../../mcp-data/shared/migrations/009_benchmark_doc_refresh.sql"
            ),
        ),
        (
            "010_phase_41_plan_status_refresh",
            include_str!(
                "../../../mcp-data/shared/migrations/010_phase_41_plan_status_refresh.sql"
            ),
        ),
        (
            "011_phase_41_5_cursor_reads",
            include_str!(
                "../../../mcp-data/shared/migrations/011_phase_41_5_cursor_reads.sql"
            ),
        ),
        (
            "012_phase_41_6_reembed_on_update",
            include_str!(
                "../../../mcp-data/shared/migrations/012_phase_41_6_reembed_on_update.sql"
            ),
        ),
    ];
    entries
        .iter()
        .filter_map(|(name, sql)| {
            let (version, label) = parse_filename(&format!("{name}.sql"))?;
            let checksum = fnv1a_hex(sql.as_bytes());
            Some(Migration {
                version,
                name: label,
                sql: sql.to_string(),
                checksum,
            })
        })
        .collect()
}

/// Run all pending migrations against the given connection. Returns
/// the number of migrations applied.
///
/// Each migration runs in its own transaction. If a migration fails,
/// its transaction is rolled back and the error is returned — earlier
/// migrations that succeeded remain committed.
pub fn apply_pending(conn: &Connection, migrations: &[Migration]) -> Result<usize, String> {
    ensure_migration_table(conn)?;
    let current = current_version(conn)?;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let mut applied = 0usize;
    for m in migrations {
        if m.version <= current {
            continue;
        }
        // Run in a savepoint so a single bad migration doesn't nuke
        // everything. `execute_batch` doesn't support params, but
        // seed SQL is static trusted content.
        conn.execute_batch("SAVEPOINT seed_migration;")
            .map_err(|e| format!("savepoint v{}: {e}", m.version))?;

        match conn.execute_batch(&m.sql) {
            Ok(()) => {
                conn.execute(
                    "INSERT OR REPLACE INTO seed_migrations (version, name, applied_at, checksum)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![m.version, m.name, now_ms, m.checksum],
                )
                .map_err(|e| format!("record migration v{}: {e}", m.version))?;
                conn.execute_batch("RELEASE seed_migration;")
                    .map_err(|e| format!("release v{}: {e}", m.version))?;
                eprintln!(
                    "[seed-migrations] applied v{:03}_{} ({})",
                    m.version, m.name, m.checksum
                );
                applied += 1;
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK TO seed_migration;");
                let _ = conn.execute_batch("RELEASE seed_migration;");
                return Err(format!(
                    "migration v{:03}_{} failed: {e}",
                    m.version, m.name
                ));
            }
        }
    }
    Ok(applied)
}

fn load_init_snapshot_sql(shared_dir: &Path) -> String {
    let disk = shared_dir.join(INIT_SNAPSHOT_FILE);
    std::fs::read_to_string(disk)
        .unwrap_or_else(|_| include_str!("../../../mcp-data/shared/memory-seed.sql").to_string())
}

fn init_snapshot_disabled() -> bool {
    std::env::var("TERRANSOUL_MCP_DISABLE_INIT_SNAPSHOT")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Fresh database fast-path: apply the consolidated shared seed snapshot once,
/// then mark shipped migrations as already applied so only future deltas run.
///
/// This preserves append-only migration history while avoiding replaying every
/// historical migration script on first boot.
fn apply_init_snapshot_if_fresh(
    conn: &Connection,
    shared_dir: &Path,
    migrations: &[Migration],
) -> Result<usize, String> {
    if init_snapshot_disabled() || migrations.is_empty() {
        return Ok(0);
    }
    if current_version(conn)? != 0 {
        return Ok(0);
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let snapshot_sql = load_init_snapshot_sql(shared_dir);

    conn.execute_batch("SAVEPOINT seed_init_snapshot;")
        .map_err(|e| format!("savepoint init snapshot: {e}"))?;

    match conn.execute_batch(&snapshot_sql) {
        Ok(()) => {
            for m in migrations {
                conn.execute(
                    "INSERT OR REPLACE INTO seed_migrations (version, name, applied_at, checksum)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![m.version, m.name, now_ms, m.checksum],
                )
                .map_err(|e| format!("record snapshot migration v{}: {e}", m.version))?;
            }

            conn.execute_batch("RELEASE seed_init_snapshot;")
                .map_err(|e| format!("release init snapshot: {e}"))?;
            eprintln!(
                "[seed-migrations] init snapshot applied from {} (tracked through v{:03})",
                shared_dir.join(INIT_SNAPSHOT_FILE).display(),
                migrations.last().map(|m| m.version).unwrap_or(0)
            );
            Ok(migrations.len())
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK TO seed_init_snapshot;");
            let _ = conn.execute_batch("RELEASE seed_init_snapshot;");
            Err(format!("init snapshot failed: {e}"))
        }
    }
}

/// Convenience: discover from disk (prefer) or compiled fallback, then
/// apply all pending. Returns `(applied_count, current_version)`.
pub fn run_all(conn: &Connection, shared_dir: &Path) -> Result<(usize, u32), String> {
    let migrations_dir = shared_dir.join("migrations");
    let mut migrations = discover_migrations(&migrations_dir);
    if migrations.is_empty() {
        migrations = compiled_migrations();
    }

    let mut applied = 0usize;
    if current_version(conn)? == 0 {
        match apply_init_snapshot_if_fresh(conn, shared_dir, &migrations) {
            Ok(n) => applied += n,
            Err(e) => {
                // Snapshot is an optimization path. Fall back to replaying
                // numbered migrations to preserve boot reliability.
                eprintln!("[seed-migrations] warning: {e}; falling back to numbered migrations");
            }
        }
    }

    applied += apply_pending(conn, &migrations)?;
    let version = current_version(conn)?;
    Ok((applied, version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        // Need the memories + memory_edges tables for the seed SQL
        crate::memory::schema::create_canonical_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn parse_filename_works() {
        assert_eq!(
            parse_filename("001_initial_seed.sql"),
            Some((1, "initial_seed".to_string()))
        );
        assert_eq!(
            parse_filename("042_fix_edges.sql"),
            Some((42, "fix_edges".to_string()))
        );
        assert_eq!(parse_filename("not_a_migration.sql"), None);
        assert_eq!(parse_filename("001_test.txt"), None);
    }

    #[test]
    fn fnv1a_is_deterministic() {
        let a = fnv1a_hex(b"hello world");
        let b = fnv1a_hex(b"hello world");
        assert_eq!(a, b);
        assert_ne!(a, fnv1a_hex(b"hello world!"));
    }

    #[test]
    fn fresh_db_has_version_zero() {
        let conn = mem_conn();
        assert_eq!(current_version(&conn).unwrap(), 0);
    }

    #[test]
    fn apply_increments_version() {
        let conn = mem_conn();
        let migrations = vec![
            Migration {
                version: 1,
                name: "first".to_string(),
                sql: "INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count) VALUES ('test1', 'test', 5, 'fact', 0, 'long', 1.0, 5);".to_string(),
                checksum: "aaa".to_string(),
            },
            Migration {
                version: 2,
                name: "second".to_string(),
                sql: "INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count) VALUES ('test2', 'test', 5, 'fact', 0, 'long', 1.0, 5);".to_string(),
                checksum: "bbb".to_string(),
            },
        ];
        let applied = apply_pending(&conn, &migrations).unwrap();
        assert_eq!(applied, 2);
        assert_eq!(current_version(&conn).unwrap(), 2);

        // Re-run: nothing new
        let applied2 = apply_pending(&conn, &migrations).unwrap();
        assert_eq!(applied2, 0);
    }

    #[test]
    fn bad_migration_rolls_back_but_keeps_earlier() {
        let conn = mem_conn();
        let migrations = vec![
            Migration {
                version: 1,
                name: "good".to_string(),
                sql: "INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count) VALUES ('good', 'test', 5, 'fact', 0, 'long', 1.0, 5);".to_string(),
                checksum: "aaa".to_string(),
            },
            Migration {
                version: 2,
                name: "bad".to_string(),
                sql: "INSERT INTO nonexistent_table VALUES (1);".to_string(),
                checksum: "bbb".to_string(),
            },
        ];
        let result = apply_pending(&conn, &migrations);
        assert!(result.is_err());
        // v1 committed, v2 rolled back
        assert_eq!(current_version(&conn).unwrap(), 1);
    }

    #[test]
    fn compiled_migrations_parse() {
        let compiled = compiled_migrations();
        assert!(!compiled.is_empty(), "must have at least migration 001");
        assert_eq!(compiled[0].version, 1);
        assert_eq!(compiled[0].name, "initial_seed");
    }

    #[test]
    fn all_compiled_migrations_apply_against_canonical_schema() {
        // Smoke test: every migration shipped in the binary must execute
        // cleanly against a fresh canonical schema. Catches SQL syntax
        // errors, missing columns, and broken edge inserts at test time
        // instead of at first-run on a user's machine.
        let conn = mem_conn();
        let migrations = compiled_migrations();
        let applied = apply_pending(&conn, &migrations).expect("all migrations apply");
        assert_eq!(applied, migrations.len());
        assert_eq!(current_version(&conn).unwrap(), migrations.len() as u32);

        // Re-running is a no-op.
        let again = apply_pending(&conn, &migrations).unwrap();
        assert_eq!(again, 0);
    }

    #[test]
    fn init_snapshot_marks_all_migrations_on_fresh_db() {
        let conn = mem_conn();
        let migrations = compiled_migrations();
        let tmp = tempfile::tempdir().unwrap();

        let marked = apply_init_snapshot_if_fresh(&conn, tmp.path(), &migrations).unwrap();
        assert_eq!(marked, migrations.len());
        assert_eq!(current_version(&conn).unwrap(), migrations.len() as u32);

        // Re-running on non-fresh DB is a no-op.
        let again = apply_init_snapshot_if_fresh(&conn, tmp.path(), &migrations).unwrap();
        assert_eq!(again, 0);
    }
}
