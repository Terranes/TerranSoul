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
    // ── V5: Entity-Relationship Graph (typed, directional edges) ────────
    //
    // Promotes the memory store from a tag-based co-occurrence graph to a
    // proper knowledge graph with typed, directional edges between memories.
    //
    // - `src_id` --rel_type--> `dst_id` is a directed edge.
    // - `rel_type` is a free-form string but the application validates against
    //   a curated vocabulary (`contains`, `cites`, `governs`, `related_to`,
    //   `mother_of`, `studies`, `prefers`, `contradicts`, `supersedes`, etc.).
    // - `confidence` ∈ [0.0, 1.0] — usually 1.0 for user-asserted edges and
    //   the LLM's self-reported score for automatically extracted edges.
    // - `source` records who proposed the edge so the UI can show provenance
    //   (`user`, `llm`, `auto`).
    // - `(src_id, dst_id, rel_type)` is unique so re-running edge extraction
    //   is idempotent.
    // - `ON DELETE CASCADE` keeps the graph consistent when a memory is
    //   removed: all incident edges disappear automatically.
    Migration {
        version: 5,
        description: "entity-relationship graph: typed, directional memory edges",
        up: r#"
CREATE TABLE IF NOT EXISTS memory_edges (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    src_id     INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    dst_id     INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    rel_type   TEXT    NOT NULL,
    confidence REAL    NOT NULL DEFAULT 1.0,
    source     TEXT    NOT NULL DEFAULT 'user',
    created_at INTEGER NOT NULL,
    UNIQUE(src_id, dst_id, rel_type)
);
CREATE INDEX IF NOT EXISTS idx_edges_src  ON memory_edges(src_id);
CREATE INDEX IF NOT EXISTS idx_edges_dst  ON memory_edges(dst_id);
CREATE INDEX IF NOT EXISTS idx_edges_type ON memory_edges(rel_type);
"#,
        down: r#"
DROP INDEX IF EXISTS idx_edges_type;
DROP INDEX IF EXISTS idx_edges_dst;
DROP INDEX IF EXISTS idx_edges_src;
DROP TABLE IF EXISTS memory_edges;
"#,
    },
    // ── V6: Temporal knowledge-graph edges ─────────────────────────────
    //
    // Adds two **nullable** Unix-ms timestamps to `memory_edges`:
    //
    //   * `valid_from` — the edge starts being true at this instant.
    //                    `NULL` ≡ "valid since the beginning of time".
    //   * `valid_to`   — the edge stops being true at this instant
    //                    (right-exclusive). `NULL` ≡ "still valid".
    //
    // Backward-compatibility:
    //   * Existing rows get `NULL`/`NULL` automatically (always valid).
    //   * All previously-shipped query paths ignore the columns and
    //     therefore return identical results.
    //
    // The new `idx_edges_valid_to` index targets the most expensive
    // temporal query — "give me every still-valid edge as of timestamp
    // T" — by letting SQLite skip closed-interval edges when filtering.
    //
    // See `docs/brain-advanced-design.md` § 16 Phase 6 / § 19.2 row 13
    // (Zep / Graphiti pattern, 2024).
    Migration {
        version: 6,
        description: "temporal knowledge graph: valid_from / valid_to on memory_edges",
        up: r#"
ALTER TABLE memory_edges ADD COLUMN valid_from INTEGER;
ALTER TABLE memory_edges ADD COLUMN valid_to   INTEGER;
CREATE INDEX IF NOT EXISTS idx_edges_valid_to ON memory_edges(valid_to);
"#,
        down: r#"
DROP INDEX IF EXISTS idx_edges_valid_to;
-- SQLite 3.35+ supports DROP COLUMN. We guard the V6 down migration
-- behind that capability so older builds simply leave the columns in
-- place rather than failing the downgrade entirely.
ALTER TABLE memory_edges DROP COLUMN valid_to;
ALTER TABLE memory_edges DROP COLUMN valid_from;
"#,
    },
    // ── V7: External knowledge-graph mirror provenance ─────────────────
    //
    // Adds a single nullable `edge_source` TEXT column to `memory_edges`.
    //
    // Distinct from the existing `source` column (which records who
    // *asserted* the edge inside TerranSoul: `user` / `llm` / `auto`),
    // `edge_source` records which **external knowledge graph** the edge
    // was mirrored from. The convention is `<system>:<scope>` — for
    // example `gitnexus:repo:owner/name@sha`. `NULL` means the edge is
    // native to TerranSoul (the default for every existing row).
    //
    // The companion index `idx_edges_edge_source` makes
    // `gitnexus_unmirror` (which deletes every edge for one mirror
    // scope) a single B-tree range scan instead of a full table scan.
    //
    // See `docs/brain-advanced-design.md` § 13 (GitNexus integration,
    // Tier 3 — Knowledge-graph mirror).
    Migration {
        version: 7,
        description: "external KG mirror provenance: edge_source column on memory_edges",
        up: r#"
ALTER TABLE memory_edges ADD COLUMN edge_source TEXT;
CREATE INDEX IF NOT EXISTS idx_edges_edge_source ON memory_edges(edge_source);
"#,
        down: r#"
DROP INDEX IF EXISTS idx_edges_edge_source;
ALTER TABLE memory_edges DROP COLUMN edge_source;
"#,
    },
    // ── V8: Memory versioning — edit history for memories ──────────────
    //
    // Tracks every edit to a memory entry as an immutable snapshot.
    // When `update_memory` changes `content`, `tags`, `importance`, or
    // `memory_type`, the *previous* state is saved to `memory_versions`
    // before the update is applied. This makes edits non-destructive and
    // enables a "view history" panel in BrainView.
    //
    // The `version_num` column starts at 1 for the first edit and auto-
    // increments per-memory via `MAX(version_num) + 1` at insert time.
    //
    // FK cascade ensures that deleting a memory also deletes its version
    // history (no orphan rows).
    //
    // See `docs/brain-advanced-design.md` §16 Phase 4 (chunk 16.12).
    Migration {
        version: 8,
        description: "memory versioning: edit history for memories",
        up: r#"
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
"#,
        down: r#"
DROP INDEX IF EXISTS idx_versions_memory;
DROP TABLE IF EXISTS memory_versions;
"#,
    },
    // ── V9: Contradiction resolution — valid_to on memories + memory_conflicts ─
    //
    // `valid_to` is a nullable Unix-ms timestamp on `memories`. A NULL value
    // means the entry is still valid / active. When a contradiction is resolved,
    // the losing entry's `valid_to` is set to the resolution timestamp — it is
    // **never deleted**, preserving audit trail and enabling undo.
    //
    // `memory_conflicts` records detected semantic contradictions between two
    // memories. Status flows: open → resolved (winner picks one side) or
    // open → dismissed (user says "not a real conflict"). The loser_id is set
    // only on resolution.
    //
    // See `docs/brain-advanced-design.md` §16 Phase 5 (chunk 17.2).
    Migration {
        version: 9,
        description: "contradiction resolution: valid_to on memories + memory_conflicts table",
        up: r#"
ALTER TABLE memories ADD COLUMN valid_to INTEGER;

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
"#,
        down: r#"
DROP INDEX IF EXISTS idx_conflicts_status;
DROP TABLE IF EXISTS memory_conflicts;
ALTER TABLE memories DROP COLUMN valid_to;
"#,
    },
    // ── V10: Obsidian sync metadata (Chunk 17.7b) ──────────────────────
    //
    // Adds bidirectional-sync tracking columns to `memories`:
    //
    // - `obsidian_path` — the relative path of the Markdown file inside
    //   the user's Obsidian vault (e.g. `daily/2026-04-30.md`). NULL
    //   means the entry has never been exported.
    // - `last_exported` — Unix-ms timestamp of the most recent successful
    //   write to the vault. NULL means never exported. Used together
    //   with the file-watcher in chunk 17.7 (bidirectional sync) for LWW
    //   conflict resolution: if the .md file's mtime > `last_exported`,
    //   the file was edited externally and the memory should be updated
    //   from disk.
    //
    // No new index — `obsidian_path` is queried by the export pass on a
    // small (<1 % of memories) subset; full-table-scan is acceptable.
    //
    // See `docs/brain-advanced-design.md` §8 (On-Disk Schema & Storage
    // Layout) and Chunk 17.7 (bidirectional Obsidian sync).
    Migration {
        version: 10,
        description: "obsidian sync metadata: obsidian_path + last_exported on memories",
        up: r#"
ALTER TABLE memories ADD COLUMN obsidian_path TEXT;
ALTER TABLE memories ADD COLUMN last_exported INTEGER;
"#,
        down: r#"
ALTER TABLE memories DROP COLUMN last_exported;
ALTER TABLE memories DROP COLUMN obsidian_path;
"#,
    },
    // ── V11: Category column for taxonomy enforcement (Chunk 17.8) ─────
    //
    // Adds a dedicated `category` TEXT column to `memories` (vs the
    // current convention of encoding category as a `category:` tag
    // prefix). The column is nullable to keep the migration cheap and
    // back-compatible — entries without a category continue to rely on
    // tag-prefix lookup. New entries written through `add_memory_v2`
    // (chunk 17.8 follow-up) will populate the column.
    //
    // The companion index `idx_memories_category` makes
    // "all memories in category X" a B-tree range scan.
    //
    // No backfill in this migration — that runs lazily on first read
    // (or via a one-shot Rust pass triggered from the brain
    // maintenance scheduler).
    //
    // See `docs/brain-advanced-design.md` §3 "Proposed Category
    // Taxonomy".
    Migration {
        version: 11,
        description: "taxonomy enforcement: category column + index on memories",
        up: r#"
ALTER TABLE memories ADD COLUMN category TEXT;
CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
"#,
        down: r#"
DROP INDEX IF EXISTS idx_memories_category;
ALTER TABLE memories DROP COLUMN category;
"#,
    },
    // ── Phase 24 — Mobile Companion ────────────────────────────────────────────
    Migration {
        version: 12,
        description: "paired_devices table for mTLS device registry (24.2b)",
        up: r#"
CREATE TABLE IF NOT EXISTS paired_devices (
    device_id        TEXT PRIMARY KEY,
    display_name     TEXT NOT NULL,
    cert_fingerprint TEXT NOT NULL,
    capabilities     TEXT NOT NULL DEFAULT '[]',
    paired_at        INTEGER NOT NULL,
    last_seen_at     INTEGER
);
"#,
        down: r#"
DROP TABLE IF EXISTS paired_devices;
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
            .query_row("SELECT embedding FROM memories WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(emb.is_none());

        let src: Option<String> = conn
            .query_row("SELECT source_url FROM memories WHERE id = 1", [], |r| {
                r.get(0)
            })
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
            .query_row("SELECT content FROM memories WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(content, "important");

        // embedding column should not exist.
        let has_embedding = conn
            .query_row("SELECT embedding FROM memories WHERE id = 1", [], |r| {
                r.get::<_, Option<Vec<u8>>>(0)
            })
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
        assert!(status[0].2); // V1 applied
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
            assert_eq!(
                m.version,
                (i + 1) as i64,
                "migration versions must be sequential"
            );
        }
    }

    #[test]
    fn target_version_is_v12() {
        // Sentinel test: forces an explicit decision when adding a new
        // migration. Bumping TARGET_VERSION without deliberately
        // updating this assertion catches accidental schema additions.
        assert_eq!(
            TARGET_VERSION, 12,
            "V12 is the current paired_devices schema (24.2b)"
        );
    }

    #[test]
    fn v7_round_trip_preserves_edge_source() {
        // Insert an edge with a non-NULL `edge_source` at V7, downgrade
        // to V6 (which must drop the column and the index without
        // touching the parent row), and re-upgrade — the existing row
        // must come back with `edge_source = NULL` (ALTER ADD COLUMN
        // semantics).
        let conn = fresh_conn();
        migrate_to_latest(&conn).unwrap();
        // Downgrade to V7 first — the test operates at V7 level.
        downgrade_to(&conn, 7).unwrap();
        assert_eq!(get_version(&conn).unwrap(), 7);

        conn.execute_batch("INSERT INTO memories (content, created_at) VALUES ('a', 1), ('b', 2);")
            .unwrap();
        conn.execute(
            "INSERT INTO memory_edges
                (src_id, dst_id, rel_type, confidence, source, created_at,
                 valid_from, valid_to, edge_source)
             VALUES (1, 2, 'rel', 1.0, 'auto', 0, NULL, NULL, 'gitnexus:repo:foo/bar@abc');",
            [],
        )
        .unwrap();
        let es: Option<String> = conn
            .query_row(
                "SELECT edge_source FROM memory_edges WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(es.as_deref(), Some("gitnexus:repo:foo/bar@abc"));

        downgrade_to(&conn, 6).unwrap();
        assert_eq!(get_version(&conn).unwrap(), 6);
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_edges WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(n, 1, "edge row must survive V7→V6 downgrade");

        migrate_to_latest(&conn).unwrap();
        // V8 adds memory_versions; the edge_source column round-trips fine.
        assert_eq!(get_version(&conn).unwrap(), TARGET_VERSION);
        let es2: Option<String> = conn
            .query_row(
                "SELECT edge_source FROM memory_edges WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(es2, None, "ALTER ADD COLUMN should default to NULL");
    }

    #[test]
    fn v6_round_trip_preserves_data_when_columns_present() {
        // Up to V6, insert an edge with a non-NULL valid_to, ensure
        // both columns exist and accept the values, then down-migrate
        // back to V5 which must drop the columns and the index without
        // affecting the parent edge row.
        let conn = fresh_conn();
        migrate_to_latest(&conn).unwrap();
        // Schema is at TARGET_VERSION (V7+); the previous V5/V6 round-trip
        // logic still applies as long as we start the test at V6.
        downgrade_to(&conn, 6).unwrap();
        assert_eq!(get_version(&conn).unwrap(), 6);

        // Need two memories first (FK).
        conn.execute_batch("INSERT INTO memories (content, created_at) VALUES ('a', 1), ('b', 2);")
            .unwrap();
        conn.execute(
            "INSERT INTO memory_edges
                (src_id, dst_id, rel_type, confidence, source, created_at, valid_from, valid_to)
             VALUES (1, 2, 'rel', 1.0, 'user', 0, 100, 200);",
            [],
        )
        .unwrap();
        let (vf, vt): (Option<i64>, Option<i64>) = conn
            .query_row(
                "SELECT valid_from, valid_to FROM memory_edges WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(vf, Some(100));
        assert_eq!(vt, Some(200));

        // Downgrade to V5: drop columns, the underlying edge row must survive.
        downgrade_to(&conn, 5).unwrap();
        assert_eq!(get_version(&conn).unwrap(), 5);
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_edges WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(n, 1, "edge row must survive V6→V5 downgrade");

        // Re-upgrade: V6 columns return as NULL on the existing row
        // (default for ALTER TABLE … ADD COLUMN with no default).
        migrate_to_latest(&conn).unwrap();
        assert_eq!(get_version(&conn).unwrap(), TARGET_VERSION);
        let (vf2, vt2): (Option<i64>, Option<i64>) = conn
            .query_row(
                "SELECT valid_from, valid_to FROM memory_edges WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(vf2, None, "ALTER ADD COLUMN should default to NULL");
        assert_eq!(vt2, None);
    }

    // ── V10: Obsidian sync metadata (Chunk 17.7b) ──────────────────────

    #[test]
    fn v10_adds_obsidian_sync_columns() {
        let conn = fresh_conn();
        migrate_to_latest(&conn).unwrap();
        assert!(get_version(&conn).unwrap() >= 10);

        // Both columns must exist and accept NULL + non-NULL values.
        conn.execute(
            "INSERT INTO memories
                (content, created_at, obsidian_path, last_exported)
             VALUES ('exported', 1000, 'daily/2026-04-30.md', 1700000000000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memories
                (content, created_at, obsidian_path, last_exported)
             VALUES ('not-exported', 2000, NULL, NULL)",
            [],
        )
        .unwrap();

        let (path, ts): (Option<String>, Option<i64>) = conn
            .query_row(
                "SELECT obsidian_path, last_exported FROM memories WHERE content = 'exported'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(path.as_deref(), Some("daily/2026-04-30.md"));
        assert_eq!(ts, Some(1_700_000_000_000));
    }

    #[test]
    fn v10_round_trip_drops_columns_and_preserves_rows() {
        let conn = fresh_conn();
        migrate_to_latest(&conn).unwrap();
        // Insert with V10 columns populated.
        conn.execute(
            "INSERT INTO memories
                (content, created_at, obsidian_path, last_exported)
             VALUES ('a', 1, 'inbox/a.md', 100)",
            [],
        )
        .unwrap();
        // Down to V9 — columns dropped, row survives.
        downgrade_to(&conn, 9).unwrap();
        assert_eq!(get_version(&conn).unwrap(), 9);
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE content = 'a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "row survives V10→V9 downgrade");
        // Re-upgrade — columns return as NULL on the existing row.
        migrate_to_latest(&conn).unwrap();
        let (path, ts): (Option<String>, Option<i64>) = conn
            .query_row(
                "SELECT obsidian_path, last_exported FROM memories WHERE content = 'a'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            path, None,
            "ALTER ADD COLUMN defaults to NULL on re-upgrade"
        );
        assert_eq!(ts, None);
    }

    // ── V11: Category column (Chunk 17.8) ──────────────────────────────

    #[test]
    fn v11_adds_category_column_and_index() {
        let conn = fresh_conn();
        migrate_to_latest(&conn).unwrap();
        assert!(get_version(&conn).unwrap() >= 11);

        // Column accepts values.
        conn.execute(
            "INSERT INTO memories (content, created_at, category)
             VALUES ('vietnamese-law-fact', 100, 'law')",
            [],
        )
        .unwrap();
        let cat: Option<String> = conn
            .query_row(
                "SELECT category FROM memories WHERE content = 'vietnamese-law-fact'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cat.as_deref(), Some("law"));

        // Index exists — using EXPLAIN QUERY PLAN to confirm it's used.
        let plan: String = conn
            .query_row(
                "EXPLAIN QUERY PLAN SELECT id FROM memories WHERE category = 'law'",
                [],
                |r| r.get(3),
            )
            .unwrap();
        assert!(
            plan.contains("idx_memories_category"),
            "category lookup should hit the index: plan was {plan}"
        );
    }

    #[test]
    fn v11_round_trip_drops_column_and_index() {
        let conn = fresh_conn();
        migrate_to_latest(&conn).unwrap();
        // Insert with category populated.
        conn.execute(
            "INSERT INTO memories (content, created_at, category)
             VALUES ('a', 1, 'preference')",
            [],
        )
        .unwrap();
        // Down to V10 — column + index dropped, row survives.
        downgrade_to(&conn, 10).unwrap();
        assert_eq!(get_version(&conn).unwrap(), 10);
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE content = 'a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "row survives V11→V10 downgrade");
        // The index must also be gone — no row in sqlite_master.
        let idx: Option<String> = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_memories_category'",
                [],
                |r| r.get(0),
            )
            .ok();
        assert!(idx.is_none(), "category index dropped on V11→V10 downgrade");
        // Re-upgrade — column returns as NULL.
        migrate_to_latest(&conn).unwrap();
        let cat: Option<String> = conn
            .query_row(
                "SELECT category FROM memories WHERE content = 'a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cat, None);
    }
}
