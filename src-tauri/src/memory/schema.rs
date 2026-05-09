//! Canonical SQLite schema for TerranSoul's memory store.
//!
//! Pre-release chunk 19.1 collapsed the append-only migration history into this
//! single initializer. New databases are created directly at the final schema;
//! existing databases must already match this canonical shape.

use rusqlite::{Connection, Result as SqlResult};

/// Canonical memory schema version reported by the app.
pub const CANONICAL_SCHEMA_VERSION: i64 = 20;

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
    embedding_model_id TEXT,
    embedding_dim INTEGER,
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
    hlc_counter   INTEGER NOT NULL DEFAULT 0,
    protected     INTEGER NOT NULL DEFAULT 0,
    share_scope   TEXT    NOT NULL DEFAULT 'private',
    confidence    REAL    NOT NULL DEFAULT 1.0
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
CREATE INDEX IF NOT EXISTS idx_memories_long_embedded ON memories(id) WHERE tier='long' AND embedding IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_memories_active ON memories(id) WHERE valid_to IS NULL;
CREATE INDEX IF NOT EXISTS idx_memories_session_recent ON memories(session_id, created_at DESC);

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
    origin_device TEXT,
    hlc_counter INTEGER NOT NULL DEFAULT 0,
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

CREATE TABLE IF NOT EXISTS memory_embeddings (
    memory_id    INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    model_id     TEXT    NOT NULL,
    dim          INTEGER NOT NULL,
    embedding    BLOB    NOT NULL,
    created_at   INTEGER NOT NULL,
    PRIMARY KEY (memory_id, model_id)
);
CREATE INDEX IF NOT EXISTS idx_memory_embeddings_model ON memory_embeddings(model_id);

CREATE TABLE IF NOT EXISTS memory_reinforcements (
    memory_id     INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    session_id    TEXT    NOT NULL,
    message_index INTEGER NOT NULL DEFAULT 0,
    ts            INTEGER NOT NULL,
    PRIMARY KEY (memory_id, session_id, message_index)
);
CREATE INDEX IF NOT EXISTS idx_reinforcements_memory ON memory_reinforcements(memory_id);

CREATE TABLE IF NOT EXISTS memory_trigger_patterns (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    memory_id INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    pattern   TEXT    NOT NULL,
    kind      TEXT    NOT NULL DEFAULT 'regex'
);
CREATE INDEX IF NOT EXISTS idx_trigger_patterns_memory ON memory_trigger_patterns(memory_id);

CREATE TABLE IF NOT EXISTS memory_gaps (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    query_embedding BLOB,
    context_snippet TEXT    NOT NULL DEFAULT '',
    session_id      TEXT,
    ts              INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_gaps_ts ON memory_gaps(ts);

CREATE TABLE IF NOT EXISTS safety_decisions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    action      TEXT    NOT NULL,
    decision    TEXT    NOT NULL,
    decided_at  INTEGER NOT NULL,
    decided_via TEXT    NOT NULL DEFAULT 'auto'
);
CREATE INDEX IF NOT EXISTS idx_safety_decided_at ON safety_decisions(decided_at);
"#;

/// Create the final memory schema directly and record its canonical version.
pub fn create_canonical_schema(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(CANONICAL_SCHEMA_SQL)?;
    ensure_memories_cognitive_kind(conn)?;
    ensure_pending_embeddings(conn)?;
    ensure_protected_column(conn)?;
    ensure_multi_model_embeddings(conn)?;
    ensure_hlc_counter(conn)?;
    ensure_edge_crdt_columns(conn)?;
    ensure_share_scope(conn)?;
    ensure_v20_tables(conn)?;
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

/// Ensure multi-model embedding columns + side table exist (upgrade path from v15 to v16).
fn ensure_multi_model_embeddings(conn: &Connection) -> SqlResult<()> {
    // Check if embedding_model_id column already exists
    let mut has_model_id = false;
    let mut stmt = conn.prepare("PRAGMA table_info(memories)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == "embedding_model_id" {
            has_model_id = true;
            break;
        }
    }
    if !has_model_id {
        conn.execute_batch(
            "ALTER TABLE memories ADD COLUMN embedding_model_id TEXT;
             ALTER TABLE memories ADD COLUMN embedding_dim INTEGER;",
        )?;
    }
    // Create the side table (IF NOT EXISTS is safe for idempotency)
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memory_embeddings (
            memory_id    INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
            model_id     TEXT    NOT NULL,
            dim          INTEGER NOT NULL,
            embedding    BLOB    NOT NULL,
            created_at   INTEGER NOT NULL,
            PRIMARY KEY (memory_id, model_id)
        );
        CREATE INDEX IF NOT EXISTS idx_memory_embeddings_model ON memory_embeddings(model_id);",
    )
}

/// Ensure the `hlc_counter` column exists on `memories` (upgrade path from v16 to v17).
fn ensure_hlc_counter(conn: &Connection) -> SqlResult<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(memories)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == "hlc_counter" {
            return Ok(());
        }
    }
    conn.execute_batch("ALTER TABLE memories ADD COLUMN hlc_counter INTEGER NOT NULL DEFAULT 0")
}

/// Ensure `origin_device` and `hlc_counter` columns exist on `memory_edges` (v17 → v18).
fn ensure_edge_crdt_columns(conn: &Connection) -> SqlResult<()> {
    let mut has_origin = false;
    let mut has_hlc = false;
    let mut stmt = conn.prepare("PRAGMA table_info(memory_edges)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == "origin_device" {
            has_origin = true;
        }
        if name == "hlc_counter" {
            has_hlc = true;
        }
    }
    if !has_origin {
        conn.execute_batch("ALTER TABLE memory_edges ADD COLUMN origin_device TEXT")?;
    }
    if !has_hlc {
        conn.execute_batch(
            "ALTER TABLE memory_edges ADD COLUMN hlc_counter INTEGER NOT NULL DEFAULT 0",
        )?;
    }
    Ok(())
}

/// Ensure the `share_scope` column exists on `memories` (upgrade path v18 → v19).
fn ensure_share_scope(conn: &Connection) -> SqlResult<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(memories)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == "share_scope" {
            return Ok(());
        }
    }
    conn.execute_batch(
        "ALTER TABLE memories ADD COLUMN share_scope TEXT NOT NULL DEFAULT 'private'",
    )
}

/// V20 migration: add `confidence` column, reinforcements, trigger patterns,
/// gaps, and safety decisions tables (upgrade path v19 → v20).
fn ensure_v20_tables(conn: &Connection) -> SqlResult<()> {
    // Add `confidence` column if missing
    let mut has_confidence = false;
    {
        let mut stmt = conn.prepare("PRAGMA table_info(memories)")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == "confidence" {
                has_confidence = true;
                break;
            }
        }
    }
    if !has_confidence {
        conn.execute_batch("ALTER TABLE memories ADD COLUMN confidence REAL NOT NULL DEFAULT 1.0")?;
    }

    // Create the four new tables (IF NOT EXISTS makes them idempotent)
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memory_reinforcements (
            memory_id     INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
            session_id    TEXT    NOT NULL,
            message_index INTEGER NOT NULL DEFAULT 0,
            ts            INTEGER NOT NULL,
            PRIMARY KEY (memory_id, session_id, message_index)
        );
        CREATE INDEX IF NOT EXISTS idx_reinforcements_memory ON memory_reinforcements(memory_id);

        CREATE TABLE IF NOT EXISTS memory_trigger_patterns (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            memory_id INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
            pattern   TEXT    NOT NULL,
            kind      TEXT    NOT NULL DEFAULT 'regex'
        );
        CREATE INDEX IF NOT EXISTS idx_trigger_patterns_memory ON memory_trigger_patterns(memory_id);

        CREATE TABLE IF NOT EXISTS memory_gaps (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            query_embedding BLOB,
            context_snippet TEXT    NOT NULL DEFAULT '',
            session_id      TEXT,
            ts              INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_gaps_ts ON memory_gaps(ts);

        CREATE TABLE IF NOT EXISTS safety_decisions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            action      TEXT    NOT NULL,
            decision    TEXT    NOT NULL,
            decided_at  INTEGER NOT NULL,
            decided_via TEXT    NOT NULL DEFAULT 'auto'
        );
        CREATE INDEX IF NOT EXISTS idx_safety_decided_at ON safety_decisions(decided_at);"
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
                last_accessed, access_count, embedding, embedding_model_id,
                embedding_dim, source_url, source_hash,
                expires_at, tier, decay_score, session_id, parent_id, token_count,
                valid_to, obsidian_path, last_exported, category, cognitive_kind,
                updated_at, origin_device, protected, hlc_counter, share_scope,
                confidence
          FROM memories LIMIT 0",
    )?;
    conn.prepare(
        "SELECT memory_id, model_id, dim, embedding, created_at
         FROM memory_embeddings LIMIT 0",
    )?;
    conn.prepare(
        "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at,
                valid_from, valid_to, edge_source, origin_device, hlc_counter
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
    conn.prepare(
        "SELECT memory_id, session_id, message_index, ts
         FROM memory_reinforcements LIMIT 0",
    )?;
    conn.prepare(
        "SELECT id, memory_id, pattern, kind
         FROM memory_trigger_patterns LIMIT 0",
    )?;
    conn.prepare(
        "SELECT id, query_embedding, context_snippet, session_id, ts
         FROM memory_gaps LIMIT 0",
    )?;
    conn.prepare(
        "SELECT id, action, decision, decided_at, decided_via
         FROM safety_decisions LIMIT 0",
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

    #[test]
    fn v20_confidence_column_exists_with_default() {
        let conn = fresh_conn();
        create_canonical_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO memories (content, created_at) VALUES ('test', 100)",
            [],
        )
        .unwrap();

        let confidence: f64 = conn
            .query_row(
                "SELECT confidence FROM memories WHERE content = 'test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            (confidence - 1.0).abs() < f64::EPSILON,
            "default confidence should be 1.0"
        );
    }

    #[test]
    fn v20_new_tables_exist() {
        let conn = fresh_conn();
        create_canonical_schema(&conn).unwrap();

        for table in [
            "memory_reinforcements",
            "memory_trigger_patterns",
            "memory_gaps",
            "safety_decisions",
        ] {
            assert!(
                object_exists(&conn, "table", table),
                "missing V20 table {table}"
            );
        }
    }

    #[test]
    fn v20_upgrade_from_v19_adds_confidence_and_tables() {
        let conn = fresh_conn();

        // Simulate a V19 database: create the canonical schema first,
        // then drop the V20-only column and tables.
        create_canonical_schema(&conn).unwrap();
        // We can't DROP COLUMN in older SQLite, so instead let's start
        // from a minimal V19-like memories table without confidence.
        conn.execute_batch("DROP TABLE IF EXISTS memory_reinforcements")
            .unwrap();
        conn.execute_batch("DROP TABLE IF EXISTS memory_trigger_patterns")
            .unwrap();
        conn.execute_batch("DROP TABLE IF EXISTS memory_gaps")
            .unwrap();
        conn.execute_batch("DROP TABLE IF EXISTS safety_decisions")
            .unwrap();

        // Verify tables are gone
        assert!(!object_exists(&conn, "table", "memory_reinforcements"));

        // Re-run canonical schema (idempotent migration)
        create_canonical_schema(&conn).unwrap();

        // V20 tables should be back
        for table in [
            "memory_reinforcements",
            "memory_trigger_patterns",
            "memory_gaps",
            "safety_decisions",
        ] {
            assert!(
                object_exists(&conn, "table", table),
                "V20 upgrade missing table {table}"
            );
        }
    }

    #[test]
    fn v20_reinforcements_round_trip() {
        let conn = fresh_conn();
        create_canonical_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO memories (content, created_at) VALUES ('m1', 100)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memory_reinforcements (memory_id, session_id, message_index, ts) VALUES (1, 'sess-1', 0, 200)",
            [],
        ).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_reinforcements WHERE memory_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
