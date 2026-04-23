//! Schema-first migration system for TerranSoul's SQLite database.
//!
//! Each migration has a version number, a description, an `up` SQL block
//! (applied on upgrade), and a `down` SQL block (applied on downgrade).
//!
//! The current schema version is tracked in a `schema_version` table.
//! On startup, migrations are applied incrementally — only the delta
//! between the stored version and the target version is executed.
//!
//! **CI-safe**: run `cargo test` to verify every migration round-trips
//! (up then down) without losing data structure integrity.

use rusqlite::{Connection, Result as SqlResult};

/// A single versioned migration step.
struct Migration {
    version: i64,
    description: &'static str,
    up: &'static str,
    down: &'static str,
}

/// All migrations in order.  **Append-only** — never edit an existing
/// migration after it ships.  Add a new one at the end instead.
const MIGRATIONS: &[Migration] = &[
    // ── V1: Initial schema ─────────────────────────────────────────────
    Migration {
        version: 1,
        description: "initial memories table",
        up: r#"
CREATE TABLE IF NOT EXISTS memories (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    content       TEXT    NOT NULL,
    tags          TEXT    NOT NULL DEFAULT '',
    importance    INTEGER NOT NULL DEFAULT 3,
    memory_type   TEXT    NOT NULL DEFAULT 'fact',
    created_at    INTEGER NOT NULL,
    last_accessed INTEGER,
    access_count  INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance DESC);
CREATE INDEX IF NOT EXISTS idx_memories_created    ON memories(created_at DESC);
"#,
        down: r#"
DROP INDEX IF EXISTS idx_memories_created;
DROP INDEX IF EXISTS idx_memories_importance;
DROP TABLE IF EXISTS memories;
"#,
    },
    // ── V2: Vector embeddings for fast RAG ─────────────────────────────
    Migration {
        version: 2,
        description: "add embedding column for vector search",
        up: "ALTER TABLE memories ADD COLUMN embedding BLOB;",
        down: r#"
-- SQLite cannot DROP COLUMN before 3.35.0.  Recreate the table.
CREATE TABLE memories_backup AS
    SELECT id, content, tags, importance, memory_type,
           created_at, last_accessed, access_count
    FROM memories;
DROP TABLE memories;
ALTER TABLE memories_backup RENAME TO memories;
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance DESC);
CREATE INDEX IF NOT EXISTS idx_memories_created    ON memories(created_at DESC);
"#,
    },
    // ── V3: Source tracking for document/URL ingestion ──────────────────
    Migration {
        version: 3,
        description: "add source metadata columns for document ingestion",
        up: r#"
ALTER TABLE memories ADD COLUMN source_url  TEXT;
ALTER TABLE memories ADD COLUMN source_hash TEXT;
ALTER TABLE memories ADD COLUMN expires_at  INTEGER;
CREATE INDEX IF NOT EXISTS idx_memories_source_hash ON memories(source_hash);
"#,
        down: r#"
DROP INDEX IF EXISTS idx_memories_source_hash;
CREATE TABLE memories_backup AS
    SELECT id, content, tags, importance, memory_type,
           created_at, last_accessed, access_count, embedding
    FROM memories;
DROP TABLE memories;
ALTER TABLE memories_backup RENAME TO memories;
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance DESC);
CREATE INDEX IF NOT EXISTS idx_memories_created    ON memories(created_at DESC);
"#,
    },
    // ── V4: Tiered memory system (short-term / working / long-term) ────
    Migration {
        version: 4,
        description: "add memory tier, decay score, and session tracking",
        up: r#"
ALTER TABLE memories ADD COLUMN tier TEXT NOT NULL DEFAULT 'long';
ALTER TABLE memories ADD COLUMN decay_score REAL NOT NULL DEFAULT 1.0;
ALTER TABLE memories ADD COLUMN session_id  TEXT;
ALTER TABLE memories ADD COLUMN parent_id   INTEGER REFERENCES memories(id);
ALTER TABLE memories ADD COLUMN token_count INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_memories_tier ON memories(tier);
CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id);
CREATE INDEX IF NOT EXISTS idx_memories_decay ON memories(decay_score DESC);
"#,
        down: r#"
DROP INDEX IF EXISTS idx_memories_decay;
DROP INDEX IF EXISTS idx_memories_session;
DROP INDEX IF EXISTS idx_memories_tier;
CREATE TABLE memories_backup AS
    SELECT id, content, tags, importance, memory_type,
           created_at, last_accessed, access_count, embedding,
           source_url, source_hash, expires_at
    FROM memories;
DROP TABLE memories;
ALTER TABLE memories_backup RENAME TO memories;
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance DESC);
CREATE INDEX IF NOT EXISTS idx_memories_created    ON memories(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_memories_source_hash ON memories(source_hash);
"#,
    },
];

/// The latest version that the codebase targets.
pub const TARGET_VERSION: i64 = MIGRATIONS[MIGRATIONS.len() - 1].version;

// ── Public API ─────────────────────────────────────────────────────────────────

/// Ensure the `schema_version` table exists, then apply all pending
/// migrations up to `TARGET_VERSION`.  Safe to call on every startup.
pub fn migrate_to_latest(conn: &Connection) -> SqlResult<()> {
    ensure_version_table(conn)?;
    let current = get_version(conn)?;
    upgrade(conn, current, TARGET_VERSION)
}

/// Downgrade from the current version to `target`.
/// **Use with caution** — destructive by design (drops columns / tables).
pub fn downgrade_to(conn: &Connection, target: i64) -> SqlResult<()> {
    ensure_version_table(conn)?;
    let current = get_version(conn)?;
    if target >= current {
        return Ok(());
    }
    // Apply down migrations in reverse order.
    for m in MIGRATIONS.iter().rev() {
        if m.version <= target || m.version > current {
            continue;
        }
        conn.execute_batch(m.down)?;
        // Remove this version's record so get_version() returns correctly.
        conn.execute(
            "DELETE FROM schema_version WHERE version = ?1",
            rusqlite::params![m.version],
        )?;
    }
    Ok(())
}

/// Return the current schema version (0 if uninitialized).
pub fn get_version(conn: &Connection) -> SqlResult<i64> {
    ensure_version_table(conn)?;
    conn.query_row(
        "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .or(Ok(0))
}

/// Return a human-readable description of all migrations and their status.
pub fn migration_status(conn: &Connection) -> SqlResult<Vec<(i64, &'static str, bool)>> {
    let current = get_version(conn)?;
    Ok(MIGRATIONS
        .iter()
        .map(|m| (m.version, m.description, m.version <= current))
        .collect())
}

// ── Internal ───────────────────────────────────────────────────────────────────

fn ensure_version_table(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version     INTEGER PRIMARY KEY,
            applied_at  INTEGER NOT NULL,
            description TEXT    NOT NULL DEFAULT ''
        );",
    )
}

fn set_version(conn: &Connection, version: i64) -> SqlResult<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let desc = MIGRATIONS
        .iter()
        .find(|m| m.version == version)
        .map(|m| m.description)
        .unwrap_or("");

    conn.execute(
        "INSERT OR REPLACE INTO schema_version (version, applied_at, description)
         VALUES (?1, ?2, ?3)",
        rusqlite::params![version, now, desc],
    )?;
    Ok(())
}

fn upgrade(conn: &Connection, from: i64, to: i64) -> SqlResult<()> {
    for m in MIGRATIONS {
        if m.version <= from || m.version > to {
            continue;
        }
        // Check if this migration has already been partially applied
        // (e.g. column already exists from the old ad-hoc ALTER TABLE).
        if let Err(e) = conn.execute_batch(m.up) {
            let msg = e.to_string();
            // "duplicate column name" means the old ad-hoc migration
            // already ran — safe to skip and just record the version.
            if msg.contains("duplicate column") || msg.contains("already exists") {
                set_version(conn, m.version)?;
                continue;
            }
            return Err(e);
        }
        set_version(conn, m.version)?;
    }
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    #[test]
    fn migrate_from_zero_to_latest() {
        let conn = fresh_conn();
        migrate_to_latest(&conn).unwrap();
        assert_eq!(get_version(&conn).unwrap(), TARGET_VERSION);
    }

    #[test]
    fn migrate_is_idempotent() {
        let conn = fresh_conn();
        migrate_to_latest(&conn).unwrap();
        // Running again should be a no-op.
        migrate_to_latest(&conn).unwrap();
        assert_eq!(get_version(&conn).unwrap(), TARGET_VERSION);
    }

    #[test]
    fn upgrade_preserves_data() {
        let conn = fresh_conn();
        // Apply V1 only.
        ensure_version_table(&conn).unwrap();
        upgrade(&conn, 0, 1).unwrap();

        // Insert a memory at V1 schema.
        conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count)
             VALUES ('test fact', 'test', 3, 'fact', 1000, 0)",
            [],
        )
        .unwrap();

        // Upgrade to latest — data must survive.
        migrate_to_latest(&conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // New columns should exist and default to NULL.
        let emb: Option<Vec<u8>> = conn
            .query_row(
                "SELECT embedding FROM memories WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(emb.is_none());

        let src: Option<String> = conn
            .query_row(
                "SELECT source_url FROM memories WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(src.is_none());
    }

    #[test]
    fn downgrade_removes_columns() {
        let conn = fresh_conn();
        migrate_to_latest(&conn).unwrap();

        // Insert data.
        conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count)
             VALUES ('important', 'keep', 5, 'fact', 2000, 0)",
            [],
        )
        .unwrap();

        // Downgrade to V1 — embedding and source columns removed.
        downgrade_to(&conn, 1).unwrap();
        assert_eq!(get_version(&conn).unwrap(), 1);

        // Data still there.
        let content: String = conn
            .query_row(
                "SELECT content FROM memories WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(content, "important");

        // embedding column should not exist.
        let has_embedding = conn
            .query_row(
                "SELECT embedding FROM memories WHERE id = 1",
                [],
                |r| r.get::<_, Option<Vec<u8>>>(0),
            )
            .is_ok();
        assert!(!has_embedding);
    }

    #[test]
    fn full_roundtrip_up_down_up() {
        let conn = fresh_conn();

        // Up to latest.
        migrate_to_latest(&conn).unwrap();
        assert_eq!(get_version(&conn).unwrap(), TARGET_VERSION);

        // Insert data at latest.
        conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count, source_url)
             VALUES ('roundtrip', 'test', 4, 'fact', 3000, 0, 'https://example.com')",
            [],
        )
        .unwrap();

        // Down to 0.
        downgrade_to(&conn, 0).unwrap();
        assert_eq!(get_version(&conn).unwrap(), 0);

        // Up again — fresh schema.
        migrate_to_latest(&conn).unwrap();
        assert_eq!(get_version(&conn).unwrap(), TARGET_VERSION);
    }

    #[test]
    fn migration_status_reports_correctly() {
        let conn = fresh_conn();
        ensure_version_table(&conn).unwrap();
        upgrade(&conn, 0, 1).unwrap();

        let status = migration_status(&conn).unwrap();
        assert_eq!(status.len(), MIGRATIONS.len());
        assert!(status[0].2);  // V1 applied
        assert!(!status[1].2); // V2 not yet
    }

    #[test]
    fn tolerates_preexisting_ad_hoc_embedding_column() {
        // Simulate old code that ran ALTER TABLE directly.
        let conn = fresh_conn();
        ensure_version_table(&conn).unwrap();
        upgrade(&conn, 0, 1).unwrap();
        conn.execute_batch("ALTER TABLE memories ADD COLUMN embedding BLOB")
            .unwrap();

        // Now migrate_to_latest should NOT fail on duplicate column.
        migrate_to_latest(&conn).unwrap();
        assert_eq!(get_version(&conn).unwrap(), TARGET_VERSION);
    }

    #[test]
    fn version_zero_for_fresh_database() {
        let conn = fresh_conn();
        assert_eq!(get_version(&conn).unwrap(), 0);
    }

    #[test]
    fn migrations_are_sequential() {
        for (i, m) in MIGRATIONS.iter().enumerate() {
            assert_eq!(m.version, (i + 1) as i64, "migration versions must be sequential");
        }
    }
}
