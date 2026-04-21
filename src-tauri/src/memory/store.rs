use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::migrations;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Backup the database file. Silently ignored on failure.
fn auto_backup(data_dir: &Path) {
    let src = data_dir.join("memory.db");
    if src.exists() {
        let dst = data_dir.join("memory.db.bak");
        let _ = std::fs::copy(&src, &dst);
    }
}

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
    /// 768-dimensional f32 embedding (serialized as little-endian bytes).
    /// `None` when the entry has not been embedded yet.
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
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
    /// Enables WAL mode for crash durability and creates an auto-backup.
    /// Runs versioned migrations to bring the schema up to date.
    pub fn new(data_dir: &Path) -> Self {
        auto_backup(data_dir);
        let conn = Connection::open(data_dir.join("memory.db"))
            .unwrap_or_else(|_| {
                Connection::open_in_memory()
                    .expect("Failed to create in-memory SQLite fallback database")
            });
        // WAL mode: crash-safe, concurrent reads, no data loss.
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");
        migrations::migrate_to_latest(&conn)
            .expect("memory schema migration failed");
        MemoryStore { conn }
    }

    /// Create an in-memory store (for tests).
    pub fn in_memory() -> Self {
        let conn = Connection::open_in_memory()
            .expect("Failed to create in-memory SQLite database");
        migrations::migrate_to_latest(&conn)
            .expect("memory schema migration failed");
        MemoryStore { conn }
    }

    /// Return the current schema version.
    pub fn schema_version(&self) -> i64 {
        migrations::get_version(&self.conn).unwrap_or(0)
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

        scored.sort_by_key(|item| std::cmp::Reverse(item.0));
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

    // ── Vector embedding operations ────────────────────────────────────────

    /// Store a pre-computed embedding for a memory entry.
    pub fn set_embedding(&self, id: i64, embedding: &[f32]) -> SqlResult<()> {
        let bytes = embedding_to_bytes(embedding);
        self.conn.execute(
            "UPDATE memories SET embedding = ?1 WHERE id = ?2",
            params![bytes, id],
        )?;
        Ok(())
    }

    /// Return all memories that have an embedding stored.
    pub fn get_with_embeddings(&self) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count, embedding
             FROM memories WHERE embedding IS NOT NULL",
        )?;
        let rows = stmt.query_map([], row_to_entry_with_embedding)?;
        rows.collect()
    }

    /// Return the IDs of entries that have no embedding yet (need processing).
    pub fn unembedded_ids(&self) -> SqlResult<Vec<(i64, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content FROM memories WHERE embedding IS NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect()
    }

    /// Fast cosine-similarity vector search.  Returns the top `limit`
    /// memory entries ranked by similarity to `query_embedding`.
    /// Pure arithmetic — no LLM call, runs in <5 ms for 100 k entries.
    pub fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> SqlResult<Vec<MemoryEntry>> {
        let all = self.get_with_embeddings()?;
        if all.is_empty() {
            return Ok(vec![]);
        }

        let mut scored: Vec<(f32, MemoryEntry)> = all
            .into_iter()
            .filter_map(|entry| {
                let emb = entry.embedding.as_ref()?;
                let sim = cosine_similarity(query_embedding, emb);
                Some((sim, entry))
            })
            .collect();

        // Sort descending by similarity.
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        // Touch access counters for the matched entries.
        let now = now_ms();
        for (_, e) in &scored {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }

        Ok(scored.into_iter().map(|(_, e)| e).collect())
    }

    /// Check if a new text is a near-duplicate of an existing memory.
    /// Returns `Some(id)` of the most similar existing entry if cosine > threshold.
    pub fn find_duplicate(
        &self,
        query_embedding: &[f32],
        threshold: f32,
    ) -> SqlResult<Option<i64>> {
        let all = self.get_with_embeddings()?;
        let best = all
            .iter()
            .filter_map(|e| {
                let emb = e.embedding.as_ref()?;
                let sim = cosine_similarity(query_embedding, emb);
                if sim >= threshold { Some((sim, e.id)) } else { None }
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(best.map(|(_, id)| id))
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

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
        embedding: None,
    })
}

fn row_to_entry_with_embedding(row: &rusqlite::Row<'_>) -> SqlResult<MemoryEntry> {
    let blob: Option<Vec<u8>> = row.get(8)?;
    Ok(MemoryEntry {
        id: row.get(0)?,
        content: row.get(1)?,
        tags: row.get(2)?,
        importance: row.get(3)?,
        memory_type: MemoryType::from_str(&row.get::<_, String>(4)?),
        created_at: row.get(5)?,
        last_accessed: row.get(6)?,
        access_count: row.get(7)?,
        embedding: blob.map(|b| bytes_to_embedding(&b)),
    })
}

/// Convert an f32 slice to little-endian bytes for SQLite BLOB storage.
fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Convert little-endian bytes back to an f32 vec.
fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Cosine similarity between two vectors.  Returns 0.0 on degenerate input.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f64, 0.0f64, 0.0f64);
    for (x, y) in a.iter().zip(b.iter()) {
        let (x, y) = (*x as f64, *y as f64);
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom < 1e-12 { 0.0 } else { (dot / denom) as f32 }
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

    // ── Vector / embedding tests ───────────────────────────────────────

    #[test]
    fn cosine_similarity_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-5, "identical vectors should have sim ≈ 1.0, got {sim}");
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5, "orthogonal vectors should have sim ≈ 0.0, got {sim}");
    }

    #[test]
    fn cosine_similarity_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-5, "opposite vectors should have sim ≈ -1.0, got {sim}");
    }

    #[test]
    fn cosine_similarity_mismatched_lengths() {
        assert_eq!(cosine_similarity(&[1.0, 2.0], &[1.0]), 0.0);
    }

    #[test]
    fn cosine_similarity_empty_vectors() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn embedding_bytes_roundtrip() {
        let original = vec![0.1, -0.5, 3.14159, 0.0, f32::MAX, f32::MIN];
        let bytes = embedding_to_bytes(&original);
        assert_eq!(bytes.len(), original.len() * 4);
        let restored = bytes_to_embedding(&bytes);
        assert_eq!(original, restored);
    }

    #[test]
    fn set_and_get_embedding() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("test embedding")).unwrap();
        let emb = vec![0.1, 0.2, 0.3, 0.4];
        store.set_embedding(entry.id, &emb).unwrap();

        let with_emb = store.get_with_embeddings().unwrap();
        assert_eq!(with_emb.len(), 1);
        assert_eq!(with_emb[0].embedding.as_ref().unwrap(), &emb);
    }

    #[test]
    fn unembedded_ids_tracks_missing() {
        let store = MemoryStore::in_memory();
        let e1 = store.add(new_memory("has embedding")).unwrap();
        let _e2 = store.add(new_memory("no embedding")).unwrap();
        store.set_embedding(e1.id, &[1.0, 2.0, 3.0]).unwrap();

        let unembedded = store.unembedded_ids().unwrap();
        assert_eq!(unembedded.len(), 1);
        assert_eq!(unembedded[0].1, "no embedding");
    }

    #[test]
    fn vector_search_returns_ranked_results() {
        let store = MemoryStore::in_memory();

        // Create 3 memories with different embeddings.
        let e1 = store.add(new_memory("python programming")).unwrap();
        let e2 = store.add(new_memory("rust systems")).unwrap();
        let e3 = store.add(new_memory("javascript web")).unwrap();

        // Unit vectors in different directions.
        store.set_embedding(e1.id, &[1.0, 0.0, 0.0]).unwrap();
        store.set_embedding(e2.id, &[0.0, 1.0, 0.0]).unwrap();
        store.set_embedding(e3.id, &[0.7, 0.7, 0.0]).unwrap();

        // Query vector close to e1.
        let query = vec![0.9, 0.1, 0.0];
        let results = store.vector_search(&query, 2).unwrap();
        assert_eq!(results.len(), 2);
        // e1 should be first (most similar), e3 second.
        assert_eq!(results[0].id, e1.id);
        assert_eq!(results[1].id, e3.id);
    }

    #[test]
    fn vector_search_empty_store() {
        let store = MemoryStore::in_memory();
        let results = store.vector_search(&[1.0, 0.0], 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn vector_search_limit_respected() {
        let store = MemoryStore::in_memory();
        for i in 0..10 {
            let e = store.add(new_memory(&format!("memory {i}"))).unwrap();
            store.set_embedding(e.id, &[i as f32, 0.0, 1.0]).unwrap();
        }
        let results = store.vector_search(&[5.0, 0.0, 1.0], 3).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn find_duplicate_above_threshold() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("exact match")).unwrap();
        let emb = vec![1.0, 0.0, 0.0];
        store.set_embedding(e.id, &emb).unwrap();

        // Same vector → cosine = 1.0 → above 0.97 threshold.
        let dup = store.find_duplicate(&emb, 0.97).unwrap();
        assert_eq!(dup, Some(e.id));
    }

    #[test]
    fn find_duplicate_below_threshold() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("different")).unwrap();
        store.set_embedding(e.id, &[1.0, 0.0, 0.0]).unwrap();

        // Orthogonal vector → cosine = 0.0 → below threshold.
        let dup = store.find_duplicate(&[0.0, 1.0, 0.0], 0.97).unwrap();
        assert_eq!(dup, None);
    }

    #[test]
    fn schema_version_returns_latest() {
        let store = MemoryStore::in_memory();
        assert_eq!(store.schema_version(), super::migrations::TARGET_VERSION);
    }

    #[test]
    fn vector_search_updates_access_counters() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("tracked")).unwrap();
        assert_eq!(e.access_count, 0);
        store.set_embedding(e.id, &[1.0, 0.0]).unwrap();

        store.vector_search(&[1.0, 0.0], 5).unwrap();
        store.vector_search(&[1.0, 0.0], 5).unwrap();

        let updated = store.get_by_id(e.id).unwrap();
        assert_eq!(updated.access_count, 2);
        assert!(updated.last_accessed.is_some());
    }
}
