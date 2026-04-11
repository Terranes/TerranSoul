use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS memories (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    content      TEXT    NOT NULL,
    tags         TEXT    NOT NULL DEFAULT '',
    importance   INTEGER NOT NULL DEFAULT 3,
    memory_type  TEXT    NOT NULL DEFAULT 'fact',
    created_at   INTEGER NOT NULL,
    last_accessed INTEGER,
    access_count INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance DESC);
CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at DESC);
"#;

/// The category/purpose of a memory entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// A learned fact (e.g. "User's name is Alice").
    Fact,
    /// A user preference (e.g. "User prefers Python").
    Preference,
    /// Ongoing context (e.g. "User is working on a neural network").
    Context,
    /// A summary of a past conversation.
    Summary,
}

impl MemoryType {
    fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Fact => "fact",
            MemoryType::Preference => "preference",
            MemoryType::Context => "context",
            MemoryType::Summary => "summary",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "preference" => MemoryType::Preference,
            "context" => MemoryType::Context,
            "summary" => MemoryType::Summary,
            _ => MemoryType::Fact,
        }
    }
}

/// A single long-term memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: i64,
    pub content: String,
    /// Comma-separated tags (e.g. "python,work,preferences").
    pub tags: String,
    /// Importance score 1–5 (5 = most important).
    pub importance: i64,
    pub memory_type: MemoryType,
    pub created_at: i64,
    pub last_accessed: Option<i64>,
    pub access_count: i64,
}

/// Fields required to create a new memory.
#[derive(Debug, Clone, Deserialize)]
pub struct NewMemory {
    pub content: String,
    pub tags: String,
    pub importance: i64,
    pub memory_type: MemoryType,
}

/// Fields that may be updated on an existing memory.
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryUpdate {
    pub content: Option<String>,
    pub tags: Option<String>,
    pub importance: Option<i64>,
    pub memory_type: Option<MemoryType>,
}

/// SQLite-backed persistent memory store.
pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    /// Open (or create) the memory database at `data_dir/memory.db`.
    /// Falls back to an in-memory database if the file cannot be opened.
    pub fn new(data_dir: &Path) -> Self {
        let conn = Connection::open(data_dir.join("memory.db"))
            .unwrap_or_else(|_| {
                Connection::open_in_memory()
                    .expect("Failed to create in-memory SQLite fallback database")
            });
        conn.execute_batch(SCHEMA)
            .expect("memory schema init failed");
        MemoryStore { conn }
    }

    /// Create an in-memory store (for tests).
    pub fn in_memory() -> Self {
        let conn = Connection::open_in_memory()
            .expect("Failed to create in-memory SQLite database");
        conn.execute_batch(SCHEMA).expect("memory schema init");
        MemoryStore { conn }
    }

    /// Insert a new memory entry and return it with its assigned id.
    pub fn add(&self, m: NewMemory) -> SqlResult<MemoryEntry> {
        let importance = m.importance.clamp(1, 5);
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![m.content, m.tags, importance, m.memory_type.as_str(), now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get_by_id(id)
    }

    /// Fetch a memory by its id.
    pub fn get_by_id(&self, id: i64) -> SqlResult<MemoryEntry> {
        self.conn.query_row(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count
             FROM memories WHERE id = ?1",
            params![id],
            row_to_entry,
        )
    }

    /// Return all memories ordered by importance (desc) then created_at (desc).
    pub fn get_all(&self) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count
             FROM memories ORDER BY importance DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_entry)?;
        rows.collect()
    }

    /// Full-text keyword search across content and tags.
    /// Updates `last_accessed` and `access_count` on matched entries.
    pub fn search(&self, query: &str) -> SqlResult<Vec<MemoryEntry>> {
        if query.trim().is_empty() {
            return self.get_all();
        }
        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count
             FROM memories
             WHERE lower(content) LIKE ?1 OR lower(tags) LIKE ?1
             ORDER BY importance DESC, access_count DESC, created_at DESC",
        )?;
        let rows = stmt.query_map(params![pattern], row_to_entry)?;
        let entries: SqlResult<Vec<MemoryEntry>> = rows.collect();
        let entries = entries?;

        // Update last_accessed and access_count for matched entries.
        let now = now_ms();
        for e in &entries {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }
        Ok(entries)
    }

    /// Update a memory entry. Only provided fields are changed.
    pub fn update(&self, id: i64, upd: MemoryUpdate) -> SqlResult<MemoryEntry> {
        if let Some(content) = upd.content {
            self.conn.execute(
                "UPDATE memories SET content = ?1 WHERE id = ?2",
                params![content, id],
            )?;
        }
        if let Some(tags) = upd.tags {
            self.conn.execute(
                "UPDATE memories SET tags = ?1 WHERE id = ?2",
                params![tags, id],
            )?;
        }
        if let Some(importance) = upd.importance {
            self.conn.execute(
                "UPDATE memories SET importance = ?1 WHERE id = ?2",
                params![importance.clamp(1, 5), id],
            )?;
        }
        if let Some(mt) = upd.memory_type {
            self.conn.execute(
                "UPDATE memories SET memory_type = ?1 WHERE id = ?2",
                params![mt.as_str(), id],
            )?;
        }
        self.get_by_id(id)
    }

    /// Delete a memory entry by id.
    pub fn delete(&self, id: i64) -> SqlResult<()> {
        self.conn
            .execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Return the N most relevant memories for a message (keyword match + importance).
    /// Used to inject long-term context into the brain's system prompt.
    pub fn relevant_for(&self, message: &str, limit: usize) -> Vec<String> {
        let words: Vec<String> = message
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(String::from)
            .collect();

        let Ok(all) = self.get_all() else {
            return vec![];
        };

        let mut scored: Vec<(usize, &MemoryEntry)> = all
            .iter()
            .filter_map(|e| {
                let lower = e.content.to_lowercase();
                let tag_lower = e.tags.to_lowercase();
                let hits = words
                    .iter()
                    .filter(|w| lower.contains(w.as_str()) || tag_lower.contains(w.as_str()))
                    .count();
                if hits > 0 {
                    Some((e.importance as usize * (hits + 1), e))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored
            .iter()
            .take(limit)
            .map(|(_, e)| e.content.clone())
            .collect()
    }

    /// Return the total number of stored memories.
    pub fn count(&self) -> i64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
            .unwrap_or(0)
    }
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> SqlResult<MemoryEntry> {
    Ok(MemoryEntry {
        id: row.get(0)?,
        content: row.get(1)?,
        tags: row.get(2)?,
        importance: row.get(3)?,
        memory_type: MemoryType::from_str(&row.get::<_, String>(4)?),
        created_at: row.get(5)?,
        last_accessed: row.get(6)?,
        access_count: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_memory(content: &str) -> NewMemory {
        NewMemory {
            content: content.to_string(),
            tags: "test".to_string(),
            importance: 3,
            memory_type: MemoryType::Fact,
        }
    }

    #[test]
    fn add_and_get_roundtrip() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("User prefers Python")).unwrap();
        assert_eq!(entry.content, "User prefers Python");
        assert_eq!(entry.importance, 3);
        assert_eq!(entry.access_count, 0);
    }

    #[test]
    fn importance_clamped_to_1_5() {
        let store = MemoryStore::in_memory();
        let e1 = store
            .add(NewMemory {
                importance: 10,
                ..new_memory("high")
            })
            .unwrap();
        let e2 = store
            .add(NewMemory {
                importance: 0,
                ..new_memory("low")
            })
            .unwrap();
        assert_eq!(e1.importance, 5);
        assert_eq!(e2.importance, 1);
    }

    #[test]
    fn get_all_ordered_by_importance() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                importance: 1,
                ..new_memory("low importance")
            })
            .unwrap();
        store
            .add(NewMemory {
                importance: 5,
                ..new_memory("high importance")
            })
            .unwrap();
        let all = store.get_all().unwrap();
        assert_eq!(all[0].importance, 5);
        assert_eq!(all[1].importance, 1);
    }

    #[test]
    fn search_finds_by_content_keyword() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("User loves Python programming")).unwrap();
        store.add(new_memory("User's favourite colour is blue")).unwrap();
        let results = store.search("Python").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Python"));
    }

    #[test]
    fn search_finds_by_tags() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                content: "Likes dark mode".to_string(),
                tags: "ui,preferences".to_string(),
                importance: 2,
                memory_type: MemoryType::Preference,
            })
            .unwrap();
        let results = store.search("preferences").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_empty_query_returns_all() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("A")).unwrap();
        store.add(new_memory("B")).unwrap();
        assert_eq!(store.search("").unwrap().len(), 2);
    }

    #[test]
    fn update_fields() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("Original content")).unwrap();
        let updated = store
            .update(
                entry.id,
                MemoryUpdate {
                    content: Some("Updated content".to_string()),
                    tags: Some("new-tag".to_string()),
                    importance: Some(5),
                    memory_type: None,
                },
            )
            .unwrap();
        assert_eq!(updated.content, "Updated content");
        assert_eq!(updated.tags, "new-tag");
        assert_eq!(updated.importance, 5);
    }

    #[test]
    fn delete_removes_entry() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("To be deleted")).unwrap();
        store.delete(entry.id).unwrap();
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn count_tracks_entries() {
        let store = MemoryStore::in_memory();
        assert_eq!(store.count(), 0);
        store.add(new_memory("One")).unwrap();
        store.add(new_memory("Two")).unwrap();
        assert_eq!(store.count(), 2);
        let entry = store.get_all().unwrap()[0].clone();
        store.delete(entry.id).unwrap();
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn memory_type_roundtrip() {
        for mt in [
            MemoryType::Fact,
            MemoryType::Preference,
            MemoryType::Context,
            MemoryType::Summary,
        ] {
            assert_eq!(MemoryType::from_str(mt.as_str()), mt);
        }
    }
}
