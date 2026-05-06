//! Canonical SQLite schema for TerranSoul's memory store.
//!
//! Pre-release chunk 19.1 collapsed the append-only migration history into this
//! single initializer. New databases are created directly at the final schema;
//! existing databases must already match this canonical shape.

use rusqlite::{Connection, Result as SqlResult};

/// Canonical memory schema version reported by the app.
pub const CANONICAL_SCHEMA_VERSION: i64 = 15;

const CANONICAL_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version     INTEGER PRIMARY KEY,
    applied_at  INTEGER NOT NULL,
    description TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS memories (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    content       TEXT    NOT NULL,
    tags          TEXT    NOT NULL DEFAULT '',
    importance    INTEGER NOT NULL DEFAULT 3,
    memory_type   TEXT    NOT NULL DEFAULT 'fact',
    created_at    INTEGER NOT NULL,
    last_accessed INTEGER,
    access_count  INTEGER NOT NULL DEFAULT 0,
    embedding     BLOB,
    source_url    TEXT,
    source_hash   TEXT,
    expires_at    INTEGER,
    tier          TEXT    NOT NULL DEFAULT 'long',
    decay_score   REAL    NOT NULL DEFAULT 1.0,
    session_id    TEXT,
    parent_id     INTEGER REFERENCES memories(id),
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
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance DESC);
CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_memories_source_hash ON memories(source_hash);
CREATE INDEX IF NOT EXISTS idx_memories_tier ON memories(tier);
CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id);
CREATE INDEX IF NOT EXISTS idx_memories_decay ON memories(decay_score DESC);
CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
CREATE INDEX IF NOT EXISTS idx_memories_updated_at ON memories(updated_at);
CREATE INDEX IF NOT EXISTS idx_memories_eviction ON memories(tier, importance, decay_score);

CREATE TABLE IF NOT EXISTS memory_edges (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    src_id      INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    dst_id      INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    rel_type    TEXT    NOT NULL,
    confidence  REAL    NOT NULL DEFAULT 1.0,
    source      TEXT    NOT NULL DEFAULT 'user',
    created_at  INTEGER NOT NULL,
    valid_from  INTEGER,
    valid_to    INTEGER,
    edge_source TEXT,
    UNIQUE(src_id, dst_id, rel_type)
);
CREATE INDEX IF NOT EXISTS idx_edges_src ON memory_edges(src_id);
CREATE INDEX IF NOT EXISTS idx_edges_dst ON memory_edges(dst_id);
CREATE INDEX IF NOT EXISTS idx_edges_type ON memory_edges(rel_type);
CREATE INDEX IF NOT EXISTS idx_edges_valid_to ON memory_edges(valid_to);
CREATE INDEX IF NOT EXISTS idx_edges_edge_source ON memory_edges(edge_source);

CREATE TABLE IF NOT EXISTS memory_versions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    memory_id   INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    version_num INTEGER NOT NULL,
    content     TEXT    NOT NULL,
    tags        TEXT    NOT NULL DEFAULT '',
    importance  INTEGER NOT NULL DEFAULT 3,
    memory_type TEXT    NOT NULL DEFAULT 'fact',
    created_at  INTEGER NOT NULL,
    UNIQUE(memory_id, version_num)
);
CREATE INDEX IF NOT EXISTS idx_versions_memory ON memory_versions(memory_id);

CREATE TABLE IF NOT EXISTS memory_conflicts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_a_id  INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    entry_b_id  INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    status      TEXT    NOT NULL DEFAULT 'open',
    winner_id   INTEGER,
    created_at  INTEGER NOT NULL,
    resolved_at INTEGER,
    reason      TEXT    NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_conflicts_status ON memory_conflicts(status);

CREATE TABLE IF NOT EXISTS paired_devices (
    device_id        TEXT PRIMARY KEY,
    display_name     TEXT NOT NULL,
    cert_fingerprint TEXT NOT NULL,
    capabilities     TEXT NOT NULL DEFAULT '[]',
    paired_at        INTEGER NOT NULL,
    last_seen_at     INTEGER
);

CREATE TABLE IF NOT EXISTS sync_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    peer_device TEXT    NOT NULL,
    direction   TEXT    NOT NULL,
    entry_count INTEGER NOT NULL,
    timestamp   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sync_log_peer ON sync_log(peer_device);

CREATE TABLE IF NOT EXISTS pending_embeddings (
    memory_id    INTEGER PRIMARY KEY REFERENCES memories(id) ON DELETE CASCADE,
    attempts     INTEGER NOT NULL DEFAULT 0,
    last_error   TEXT,
    next_retry_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_pending_embeddings_next ON pending_embeddings(next_retry_at);
"#;

/// Create the final memory schema directly and record its canonical version.
pub fn create_canonical_schema(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(CANONICAL_SCHEMA_SQL)?;
    ensure_memories_cognitive_kind(conn)?;
    ensure_pending_embeddings(conn)?;
    ensure_protected_column(conn)?;
    validate_canonical_schema(conn)?;
    record_schema_version(conn)
}

fn ensure_memories_cognitive_kind(conn: &Connection) -> SqlResult<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(memories)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == "cognitive_kind" {
            return Ok(());
        }
    }
    conn.execute_batch("ALTER TABLE memories ADD COLUMN cognitive_kind TEXT")
}

/// Ensure the `pending_embeddings` table exists (upgrade path from v13).
fn ensure_pending_embeddings(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS pending_embeddings (
            memory_id    INTEGER PRIMARY KEY REFERENCES memories(id) ON DELETE CASCADE,
            attempts     INTEGER NOT NULL DEFAULT 0,
            last_error   TEXT,
            next_retry_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_pending_embeddings_next ON pending_embeddings(next_retry_at);",
    )
}

/// Ensure the `protected` column exists on `memories` (upgrade path from v14).
fn ensure_protected_column(conn: &Connection) -> SqlResult<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(memories)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == "protected" {
            // Already exists — just ensure the eviction index.
            conn.execute_batch(
                "CREATE INDEX IF NOT EXISTS idx_memories_eviction ON memories(tier, importance, decay_score);"
            )?;
            return Ok(());
        }
    }
    conn.execute_batch(
        "ALTER TABLE memories ADD COLUMN protected INTEGER NOT NULL DEFAULT 0;
         CREATE INDEX IF NOT EXISTS idx_memories_eviction ON memories(tier, importance, decay_score);"
    )
}

/// Return the recorded canonical schema version, or 0 before initialization.
pub fn schema_version(conn: &Connection) -> SqlResult<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
    .or(Ok(0))
}

fn record_schema_version(conn: &Connection) -> SqlResult<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    conn.execute(
        "INSERT OR REPLACE INTO schema_version (version, applied_at, description)
         VALUES (?1, ?2, 'canonical memory schema')",
        rusqlite::params![CANONICAL_SCHEMA_VERSION, now],
    )?;
    Ok(())
}

fn validate_canonical_schema(conn: &Connection) -> SqlResult<()> {
    conn.prepare(
        "SELECT id, content, tags, importance, memory_type, created_at,
                last_accessed, access_count, embedding, source_url, source_hash,
                expires_at, tier, decay_score, session_id, parent_id, token_count,
                valid_to, obsidian_path, last_exported, category, cognitive_kind,
                updated_at, origin_device, protected
          FROM memories LIMIT 0",
    )?;
    conn.prepare(
        "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at,
                valid_from, valid_to, edge_source
         FROM memory_edges LIMIT 0",
    )?;
    conn.prepare(
        "SELECT id, memory_id, version_num, content, tags, importance,
                memory_type, created_at
         FROM memory_versions LIMIT 0",
    )?;
    conn.prepare(
        "SELECT id, entry_a_id, entry_b_id, status, winner_id, created_at,
                resolved_at, reason
         FROM memory_conflicts LIMIT 0",
    )?;
    conn.prepare(
        "SELECT device_id, display_name, cert_fingerprint, capabilities,
                paired_at, last_seen_at
         FROM paired_devices LIMIT 0",
    )?;
    conn.prepare(
        "SELECT id, peer_device, direction, entry_count, timestamp
         FROM sync_log LIMIT 0",
    )?;
    conn.prepare(
        "SELECT memory_id, attempts, last_error, next_retry_at
         FROM pending_embeddings LIMIT 0",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn table_columns(conn: &Connection, table: &str) -> Vec<String> {
        let mut statement = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    fn object_exists(conn: &Connection, object_type: &str, name: &str) -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type = ?1 AND name = ?2",
            rusqlite::params![object_type, name],
            |_| Ok(()),
        )
        .is_ok()
    }

    #[test]
    fn creates_canonical_schema_and_records_version() {
        let conn = fresh_conn();
        create_canonical_schema(&conn).unwrap();

        assert_eq!(schema_version(&conn).unwrap(), CANONICAL_SCHEMA_VERSION);
        for table in [
            "schema_version",
            "memories",
            "memory_edges",
            "memory_versions",
            "memory_conflicts",
            "paired_devices",
            "sync_log",
        ] {
            assert!(
                object_exists(&conn, "table", table),
                "missing table {table}"
            );
        }
    }

    #[test]
    fn creates_final_memory_columns() {
        let conn = fresh_conn();
        create_canonical_schema(&conn).unwrap();
        let columns = table_columns(&conn, "memories");

        for column in [
            "embedding",
            "source_url",
            "source_hash",
            "expires_at",
            "tier",
            "decay_score",
            "valid_to",
            "obsidian_path",
            "last_exported",
            "category",
            "cognitive_kind",
            "updated_at",
            "origin_device",
        ] {
            assert!(
                columns.iter().any(|name| name == column),
                "missing column {column}"
            );
        }
    }

    #[test]
    fn creates_final_indexes() {
        let conn = fresh_conn();
        create_canonical_schema(&conn).unwrap();

        for index in [
            "idx_memories_source_hash",
            "idx_memories_category",
            "idx_memories_updated_at",
            "idx_edges_valid_to",
            "idx_edges_edge_source",
            "idx_versions_memory",
            "idx_conflicts_status",
            "idx_sync_log_peer",
        ] {
            assert!(
                object_exists(&conn, "index", index),
                "missing index {index}"
            );
        }
    }

    #[test]
    fn create_canonical_schema_is_idempotent() {
        let conn = fresh_conn();
        create_canonical_schema(&conn).unwrap();
        create_canonical_schema(&conn).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), CANONICAL_SCHEMA_VERSION);
    }

    #[test]
    fn canonical_schema_accepts_final_columns() {
        let conn = fresh_conn();
        create_canonical_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO memories
                (content, created_at, source_url, valid_to, obsidian_path,
                 category, cognitive_kind, updated_at, origin_device)
             VALUES ('canonical', 1000, 'https://example.com', NULL,
                     'notes/canonical.md', 'project', 'semantic', 1001, 'device-a')",
            [],
        )
        .unwrap();

        let (category, cognitive_kind, origin_device): (
            Option<String>,
            Option<String>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT category, cognitive_kind, origin_device FROM memories WHERE content = 'canonical'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(category.as_deref(), Some("project"));
        assert_eq!(cognitive_kind.as_deref(), Some("semantic"));
        assert_eq!(origin_device.as_deref(), Some("device-a"));
    }

    #[test]
    fn canonical_schema_adds_cognitive_kind_to_legacy_memories_table() {
        let conn = fresh_conn();
        conn.execute_batch(
            "CREATE TABLE memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '',
                importance INTEGER NOT NULL DEFAULT 3,
                memory_type TEXT NOT NULL DEFAULT 'fact',
                created_at INTEGER NOT NULL,
                last_accessed INTEGER,
                access_count INTEGER NOT NULL DEFAULT 0,
                embedding BLOB,
                source_url TEXT,
                source_hash TEXT,
                expires_at INTEGER,
                tier TEXT NOT NULL DEFAULT 'long',
                decay_score REAL NOT NULL DEFAULT 1.0,
                session_id TEXT,
                parent_id INTEGER,
                token_count INTEGER NOT NULL DEFAULT 0,
                valid_to INTEGER,
                obsidian_path TEXT,
                last_exported INTEGER,
                category TEXT,
                updated_at INTEGER,
                origin_device TEXT
            );",
        )
        .unwrap();

        create_canonical_schema(&conn).unwrap();

        let columns = table_columns(&conn, "memories");
        assert!(columns.iter().any(|name| name == "cognitive_kind"));
    }

    #[test]
    fn edge_foreign_keys_cascade() {
        let conn = fresh_conn();
        create_canonical_schema(&conn).unwrap();
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();

        conn.execute_batch(
            "INSERT INTO memories (content, created_at) VALUES ('a', 1), ('b', 2);
             INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
             VALUES (1, 2, 'related_to', 1.0, 'user', 3);",
        )
        .unwrap();
        conn.execute("DELETE FROM memories WHERE id = 1", [])
            .unwrap();

        let edge_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_edges", [], |row| row.get(0))
            .unwrap();
        assert_eq!(edge_count, 0);
    }
}
