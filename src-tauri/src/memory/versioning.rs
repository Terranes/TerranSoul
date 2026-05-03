//! Memory versioning — non-destructive edit history (Chunk 16.12).
//!
//! Every call to `MemoryStore::update` that changes `content`, `tags`,
//! `importance`, or `memory_type` first snapshots the *previous* state
//! into the `memory_versions` table (V8 schema). This makes edits fully
//! reversible and enables a "view history" panel in BrainView.
//!
//! Maps to `docs/brain-advanced-design.md` §16 Phase 4 (chunk 16.12).

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

/// A single version snapshot of a memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVersion {
    pub id: i64,
    pub memory_id: i64,
    pub version_num: i64,
    pub content: String,
    pub tags: String,
    pub importance: i64,
    pub memory_type: String,
    /// Unix-ms timestamp when this version was created (i.e. when the
    /// edit that superseded it occurred).
    pub created_at: i64,
}

/// Save the current state of a memory as a version snapshot.
///
/// Call this *before* applying the update so the snapshot captures the
/// pre-edit state. `version_num` is auto-assigned as `MAX + 1` for the
/// given `memory_id` (starting at 1).
pub fn save_version(conn: &Connection, memory_id: i64) -> SqlResult<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    // Get next version number for this memory.
    let next_version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version_num), 0) + 1 FROM memory_versions WHERE memory_id = ?1",
        params![memory_id],
        |row| row.get(0),
    )?;

    // Snapshot the current state directly from the memories table.
    conn.execute(
        "INSERT INTO memory_versions (memory_id, version_num, content, tags, importance, memory_type, created_at)
         SELECT ?1, ?2, content, tags, importance, memory_type, ?3
         FROM memories WHERE id = ?1",
        params![memory_id, next_version, now],
    )?;

    Ok(next_version)
}

/// Get the full version history for a memory, ordered oldest-first.
pub fn get_history(conn: &Connection, memory_id: i64) -> SqlResult<Vec<MemoryVersion>> {
    let mut stmt = conn.prepare(
        "SELECT id, memory_id, version_num, content, tags, importance, memory_type, created_at
         FROM memory_versions
         WHERE memory_id = ?1
         ORDER BY version_num ASC",
    )?;

    let rows = stmt.query_map(params![memory_id], |row| {
        Ok(MemoryVersion {
            id: row.get(0)?,
            memory_id: row.get(1)?,
            version_num: row.get(2)?,
            content: row.get(3)?,
            tags: row.get(4)?,
            importance: row.get(5)?,
            memory_type: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;

    rows.collect()
}

/// Get the number of versions for a memory.
pub fn version_count(conn: &Connection, memory_id: i64) -> SqlResult<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM memory_versions WHERE memory_id = ?1",
        params![memory_id],
        |row| row.get(0),
    )
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::schema::create_canonical_schema;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_canonical_schema(&conn).unwrap();
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        conn
    }

    fn insert_memory(conn: &Connection, content: &str, tags: &str) -> i64 {
        let now = 1_700_000_000_000i64;
        conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count, tier, decay_score, token_count)
             VALUES (?1, ?2, 3, 'fact', ?3, 0, 'long', 1.0, 0)",
            params![content, tags, now],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn save_version_creates_snapshot() {
        let conn = fresh_conn();
        let id = insert_memory(&conn, "Original content", "tag1");
        let ver = save_version(&conn, id).unwrap();
        assert_eq!(ver, 1);

        let history = get_history(&conn, id).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].version_num, 1);
        assert_eq!(history[0].content, "Original content");
        assert_eq!(history[0].tags, "tag1");
        assert_eq!(history[0].importance, 3);
        assert_eq!(history[0].memory_type, "fact");
    }

    #[test]
    fn save_version_increments_version_num() {
        let conn = fresh_conn();
        let id = insert_memory(&conn, "v1 content", "tags");

        let ver1 = save_version(&conn, id).unwrap();
        assert_eq!(ver1, 1);

        // Simulate an edit.
        conn.execute(
            "UPDATE memories SET content = 'v2 content' WHERE id = ?1",
            params![id],
        )
        .unwrap();

        let ver2 = save_version(&conn, id).unwrap();
        assert_eq!(ver2, 2);

        let history = get_history(&conn, id).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "v1 content");
        assert_eq!(history[1].content, "v2 content");
    }

    #[test]
    fn get_history_empty_for_unversioned_memory() {
        let conn = fresh_conn();
        let id = insert_memory(&conn, "Never edited", "tags");
        let history = get_history(&conn, id).unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn version_count_matches_history() {
        let conn = fresh_conn();
        let id = insert_memory(&conn, "content", "tags");
        assert_eq!(version_count(&conn, id).unwrap(), 0);

        save_version(&conn, id).unwrap();
        assert_eq!(version_count(&conn, id).unwrap(), 1);

        conn.execute(
            "UPDATE memories SET content = 'edited' WHERE id = ?1",
            params![id],
        )
        .unwrap();
        save_version(&conn, id).unwrap();
        assert_eq!(version_count(&conn, id).unwrap(), 2);
    }

    #[test]
    fn cascade_delete_removes_versions() {
        let conn = fresh_conn();
        let id = insert_memory(&conn, "to delete", "tags");
        save_version(&conn, id).unwrap();
        save_version(&conn, id).unwrap();
        assert_eq!(version_count(&conn, id).unwrap(), 2);

        conn.execute("DELETE FROM memories WHERE id = ?1", params![id])
            .unwrap();

        assert_eq!(version_count(&conn, id).unwrap(), 0);
    }

    #[test]
    fn independent_memories_have_separate_version_sequences() {
        let conn = fresh_conn();
        let id1 = insert_memory(&conn, "memory A", "a");
        let id2 = insert_memory(&conn, "memory B", "b");

        save_version(&conn, id1).unwrap();
        save_version(&conn, id1).unwrap();
        save_version(&conn, id2).unwrap();

        assert_eq!(version_count(&conn, id1).unwrap(), 2);
        assert_eq!(version_count(&conn, id2).unwrap(), 1);

        let hist1 = get_history(&conn, id1).unwrap();
        let hist2 = get_history(&conn, id2).unwrap();
        assert_eq!(hist1[0].version_num, 1);
        assert_eq!(hist1[1].version_num, 2);
        assert_eq!(hist2[0].version_num, 1);
    }

    #[test]
    fn version_captures_all_fields() {
        let conn = fresh_conn();
        let now = 1_700_000_000_000i64;
        conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count, tier, decay_score, token_count)
             VALUES ('special', 'domain:law,project:test', 5, 'reference', ?1, 0, 'long', 1.0, 0)",
            params![now],
        )
        .unwrap();
        let id = conn.last_insert_rowid();

        save_version(&conn, id).unwrap();
        let history = get_history(&conn, id).unwrap();
        assert_eq!(history[0].content, "special");
        assert_eq!(history[0].tags, "domain:law,project:test");
        assert_eq!(history[0].importance, 5);
        assert_eq!(history[0].memory_type, "reference");
    }
}
