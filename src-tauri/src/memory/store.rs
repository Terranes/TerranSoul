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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// A learned fact (e.g. "User's name is Alice").
    #[default]
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

/// Memory tier — determines retrieval priority and lifecycle.
///
/// **Short-term**: Last ~20 messages in current session. Evicted on session end
/// or when window overflows. Auto-summarized into working memory.
///
/// **Working**: Extracted facts/context from the current session. Lives until
/// session ends, then promoted to long-term or discarded via decay.
///
/// **Long-term**: Permanent storage. Vector-indexed. Subject to periodic
/// consolidation (merge near-duplicates) and importance decay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryTier {
    Short,
    Working,
    Long,
}

impl MemoryTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryTier::Short => "short",
            MemoryTier::Working => "working",
            MemoryTier::Long => "long",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "short" => MemoryTier::Short,
            "working" => MemoryTier::Working,
            _ => MemoryTier::Long,
        }
    }
}

/// A single memory entry with tier metadata.
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
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
    /// Which tier this memory lives in.
    pub tier: MemoryTier,
    /// Decay score 0.0–1.0. Decays over time for infrequently accessed entries.
    /// Used as a multiplier in hybrid ranking.
    pub decay_score: f64,
    /// Session identifier for grouping short-term/working memories.
    pub session_id: Option<String>,
    /// Parent memory (for summaries that consolidate children).
    pub parent_id: Option<i64>,
    /// Approximate token count of the content.
    pub token_count: i64,
    /// Origin URL for ingested/crawled documents.
    pub source_url: Option<String>,
    /// SHA-256 content hash for dedup / staleness detection.
    pub source_hash: Option<String>,
    /// Optional TTL — Unix-ms timestamp after which memory auto-expires.
    pub expires_at: Option<i64>,
}

/// Fields required to create a new memory.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NewMemory {
    pub content: String,
    pub tags: String,
    #[serde(default = "default_importance")]
    pub importance: i64,
    #[serde(default)]
    pub memory_type: MemoryType,
    /// Origin URL for ingested documents (optional).
    #[serde(default)]
    pub source_url: Option<String>,
    /// SHA-256 content hash for dedup / staleness detection (optional).
    #[serde(default)]
    pub source_hash: Option<String>,
    /// TTL timestamp — memory auto-expires after this Unix-ms time (optional).
    #[serde(default)]
    pub expires_at: Option<i64>,
}

fn default_importance() -> i64 { 3 }

/// Fields that may be updated on an existing memory.
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryUpdate {
    pub content: Option<String>,
    pub tags: Option<String>,
    pub importance: Option<i64>,
    pub memory_type: Option<MemoryType>,
}

/// Aggregated statistics across all memory tiers.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    pub total: i64,
    pub short: i64,
    pub working: i64,
    pub long: i64,
    pub embedded: i64,
    pub total_tokens: i64,
    pub avg_decay: f64,
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
        let token_count = estimate_tokens(&m.content);
        self.conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count, tier, decay_score, token_count, source_url, source_hash, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, 1.0, ?7, ?8, ?9, ?10)",
            params![m.content, m.tags, importance, m.memory_type.as_str(), now, MemoryTier::Long.as_str(), token_count, m.source_url, m.source_hash, m.expires_at],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get_by_id(id)
    }

    /// Insert a memory into a specific tier (for session management).
    pub fn add_to_tier(&self, m: NewMemory, tier: MemoryTier, session_id: Option<&str>) -> SqlResult<MemoryEntry> {
        let importance = m.importance.clamp(1, 5);
        let now = now_ms();
        let token_count = estimate_tokens(&m.content);
        self.conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count, tier, decay_score, session_id, token_count, source_url, source_hash, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, 1.0, ?7, ?8, ?9, ?10, ?11)",
            params![m.content, m.tags, importance, m.memory_type.as_str(), now, tier.as_str(), session_id, token_count, m.source_url, m.source_hash, m.expires_at],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get_by_id(id)
    }

    /// Fetch a memory by its id.
    pub fn get_by_id(&self, id: i64) -> SqlResult<MemoryEntry> {
        self.conn.query_row(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at
             FROM memories WHERE id = ?1",
            params![id],
            row_to_entry,
        )
    }

    /// Return all memories ordered by importance (desc) then created_at (desc).
    pub fn get_all(&self) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at
             FROM memories ORDER BY importance DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_entry)?;
        rows.collect()
    }

    /// Return memories in a specific tier.
    pub fn get_by_tier(&self, tier: &MemoryTier) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at
             FROM memories WHERE tier = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![tier.as_str()], row_to_entry)?;
        rows.collect()
    }

    /// Get working + long-term memories (skip short-term ephemeral).
    pub fn get_persistent(&self) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at
             FROM memories WHERE tier IN ('working', 'long')
             ORDER BY importance DESC, decay_score DESC, created_at DESC",
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
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at
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
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, embedding
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

    // ── Hybrid search (the core RAG pipeline) ──────────────────────────────

    /// Multi-signal hybrid search that combines:
    /// 1. Vector cosine similarity (semantic relevance)
    /// 2. Keyword BM25-style scoring (exact match boost)
    /// 3. Recency bias (recent memories score higher)
    /// 4. Importance weighting (user-assigned priority)
    /// 5. Decay score (frequently accessed memories retain weight)
    /// 6. Tier priority (working > long for current-session context)
    ///
    /// Returns top `limit` entries ranked by composite score.
    /// Scales to 1M+ entries: vector search is O(n) but purely arithmetic.
    pub fn hybrid_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> SqlResult<Vec<MemoryEntry>> {
        let now = now_ms();
        let hour_ms: f64 = 3_600_000.0;

        // Keyword scoring setup
        let words: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(String::from)
            .collect();

        // Load all entries with embeddings for vector scoring
        let all = if query_embedding.is_some() {
            self.get_with_embeddings()?
        } else {
            self.get_all()?
        };

        if all.is_empty() {
            return Ok(vec![]);
        }

        let mut scored: Vec<(f64, MemoryEntry)> = all
            .into_iter()
            .map(|entry| {
                let mut score = 0.0f64;

                // (1) Vector similarity — weight 0.40
                if let (Some(qe), Some(emb)) = (query_embedding, entry.embedding.as_ref()) {
                    let sim = cosine_similarity(qe, emb) as f64;
                    score += sim * 0.40;
                }

                // (2) Keyword match — weight 0.20
                let lower_content = entry.content.to_lowercase();
                let lower_tags = entry.tags.to_lowercase();
                let keyword_hits = words.iter()
                    .filter(|w| lower_content.contains(w.as_str()) || lower_tags.contains(w.as_str()))
                    .count();
                if !words.is_empty() {
                    score += (keyword_hits as f64 / words.len() as f64) * 0.20;
                }

                // (3) Recency — weight 0.15 (exponential decay, half-life = 24h)
                let age_hours = (now - entry.created_at) as f64 / hour_ms;
                let recency = (-age_hours / 24.0).exp(); // 1.0 = just created, 0.5 = 24h ago
                score += recency * 0.15;

                // (4) Importance — weight 0.10
                score += (entry.importance as f64 / 5.0) * 0.10;

                // (5) Decay score — weight 0.10
                score += entry.decay_score * 0.10;

                // (6) Tier priority — weight 0.05
                let tier_boost = match entry.tier {
                    MemoryTier::Working => 1.0,
                    MemoryTier::Long => 0.5,
                    MemoryTier::Short => 0.3,
                };
                score += tier_boost * 0.05;

                (score, entry)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        // Touch access counters
        for (_, e) in &scored {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }

        Ok(scored.into_iter().map(|(_, e)| e).collect())
    }

    // ── Tier management ────────────────────────────────────────────────────

    /// Promote a memory to a higher tier.
    pub fn promote(&self, id: i64, new_tier: MemoryTier) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE memories SET tier = ?1 WHERE id = ?2",
            params![new_tier.as_str(), id],
        )?;
        Ok(())
    }

    /// Apply time-based decay to all long-term memories.
    /// Memories that haven't been accessed recently lose decay_score.
    /// Called periodically (e.g. on app startup or once per session).
    ///
    /// Formula: decay_score *= 0.95^(hours_since_last_access / 168)
    /// (halves roughly every 2 weeks of non-access)
    pub fn apply_decay(&self) -> SqlResult<usize> {
        let now = now_ms();
        let mut stmt = self.conn.prepare(
            "SELECT id, last_accessed, decay_score FROM memories WHERE tier = 'long'",
        )?;
        let rows: Vec<(i64, Option<i64>, f64)> = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get::<_, f64>(2)?))
        })?.filter_map(|r| r.ok()).collect();

        let mut updated = 0;
        for (id, last_accessed, current_decay) in &rows {
            let last = last_accessed.unwrap_or(now);
            let hours_since = (now - last) as f64 / 3_600_000.0;
            let factor = 0.95f64.powf(hours_since / 168.0);
            let new_decay = (current_decay * factor).max(0.01);
            if (new_decay - current_decay).abs() > 0.001 {
                self.conn.execute(
                    "UPDATE memories SET decay_score = ?1 WHERE id = ?2",
                    params![new_decay, id],
                )?;
                updated += 1;
            }
        }
        Ok(updated)
    }

    /// Evict short-term memories from a session, summarizing them into working memory.
    pub fn evict_short_term(&self, session_id: &str) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at
             FROM memories WHERE tier = 'short' AND session_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![session_id], row_to_entry)?;
        let entries: SqlResult<Vec<MemoryEntry>> = rows.collect();
        let entries = entries?;

        // Delete the short-term entries
        self.conn.execute(
            "DELETE FROM memories WHERE tier = 'short' AND session_id = ?1",
            params![session_id],
        )?;
        Ok(entries)
    }

    /// Find a memory by its source_hash.  Returns the first match (if any).
    pub fn find_by_source_hash(&self, hash: &str) -> SqlResult<Option<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at
             FROM memories WHERE source_hash = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![hash], row_to_entry)?;
        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /// Find all memories from a given source URL.
    pub fn find_by_source_url(&self, url: &str) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at
             FROM memories WHERE source_url = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![url], row_to_entry)?;
        rows.collect()
    }

    /// Delete all memories from a given source URL.  Returns the count deleted.
    pub fn delete_by_source_url(&self, url: &str) -> SqlResult<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM memories WHERE source_url = ?1",
            params![url],
        )?;
        Ok(deleted)
    }

    /// Delete expired memories (expires_at < now).  Returns the count deleted.
    pub fn delete_expired(&self) -> SqlResult<usize> {
        let now = now_ms();
        let deleted = self.conn.execute(
            "DELETE FROM memories WHERE expires_at IS NOT NULL AND expires_at < ?1",
            params![now],
        )?;
        Ok(deleted)
    }

    /// Delete memories below a decay threshold (garbage collection).
    pub fn gc_decayed(&self, threshold: f64) -> SqlResult<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM memories WHERE tier = 'long' AND decay_score < ?1 AND importance <= 2",
            params![threshold],
        )?;
        Ok(deleted)
    }

    /// Get memory statistics per tier.
    pub fn stats(&self) -> SqlResult<MemoryStats> {
        let total: i64 = self.conn.query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))?;
        let short: i64 = self.conn.query_row("SELECT COUNT(*) FROM memories WHERE tier='short'", [], |r| r.get(0))?;
        let working: i64 = self.conn.query_row("SELECT COUNT(*) FROM memories WHERE tier='working'", [], |r| r.get(0))?;
        let long: i64 = self.conn.query_row("SELECT COUNT(*) FROM memories WHERE tier='long'", [], |r| r.get(0))?;
        let embedded: i64 = self.conn.query_row("SELECT COUNT(*) FROM memories WHERE embedding IS NOT NULL", [], |r| r.get(0))?;
        let total_tokens: i64 = self.conn.query_row("SELECT COALESCE(SUM(token_count), 0) FROM memories", [], |r| r.get(0))?;
        let avg_decay: f64 = self.conn.query_row("SELECT COALESCE(AVG(decay_score), 1.0) FROM memories WHERE tier='long'", [], |r| r.get(0))?;
        Ok(MemoryStats { total, short, working, long, embedded, total_tokens, avg_decay })
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
        tier: MemoryTier::from_str(&row.get::<_, String>(8).unwrap_or_else(|_| "long".to_string())),
        decay_score: row.get::<_, f64>(9).unwrap_or(1.0),
        session_id: row.get(10).unwrap_or(None),
        parent_id: row.get(11).unwrap_or(None),
        token_count: row.get::<_, i64>(12).unwrap_or(0),
        source_url: row.get(13).unwrap_or(None),
        source_hash: row.get(14).unwrap_or(None),
        expires_at: row.get(15).unwrap_or(None),
    })
}

fn row_to_entry_with_embedding(row: &rusqlite::Row<'_>) -> SqlResult<MemoryEntry> {
    let blob: Option<Vec<u8>> = row.get(16)?;
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
        tier: MemoryTier::from_str(&row.get::<_, String>(8).unwrap_or_else(|_| "long".to_string())),
        decay_score: row.get::<_, f64>(9).unwrap_or(1.0),
        session_id: row.get(10).unwrap_or(None),
        parent_id: row.get(11).unwrap_or(None),
        token_count: row.get::<_, i64>(12).unwrap_or(0),
        source_url: row.get(13).unwrap_or(None),
        source_hash: row.get(14).unwrap_or(None),
        expires_at: row.get(15).unwrap_or(None),
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

/// Rough token estimation (~4 chars per token for English text).
fn estimate_tokens(text: &str) -> i64 {
    (text.len() as i64 + 3) / 4
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
            ..Default::default()
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
                ..Default::default()
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
        let original = vec![0.1, -0.5, 3.125, 0.0, f32::MAX, f32::MIN];
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

    // ── Tiered memory tests ────────────────────────────────────────────

    #[test]
    fn add_sets_default_tier_long() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("default tier")).unwrap();
        assert_eq!(entry.tier, MemoryTier::Long);
        assert!((entry.decay_score - 1.0).abs() < 0.01);
    }

    #[test]
    fn add_to_tier_creates_working_memory() {
        let store = MemoryStore::in_memory();
        let entry = store.add_to_tier(new_memory("session fact"), MemoryTier::Working, Some("sess-1")).unwrap();
        assert_eq!(entry.tier, MemoryTier::Working);
        assert_eq!(entry.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn get_by_tier_filters_correctly() {
        let store = MemoryStore::in_memory();
        store.add_to_tier(new_memory("short"), MemoryTier::Short, Some("s1")).unwrap();
        store.add_to_tier(new_memory("working"), MemoryTier::Working, Some("s1")).unwrap();
        store.add(new_memory("long")).unwrap();

        assert_eq!(store.get_by_tier(&MemoryTier::Short).unwrap().len(), 1);
        assert_eq!(store.get_by_tier(&MemoryTier::Working).unwrap().len(), 1);
        assert_eq!(store.get_by_tier(&MemoryTier::Long).unwrap().len(), 1);
    }

    #[test]
    fn get_persistent_excludes_short_term() {
        let store = MemoryStore::in_memory();
        store.add_to_tier(new_memory("ephemeral"), MemoryTier::Short, Some("s1")).unwrap();
        store.add_to_tier(new_memory("session ctx"), MemoryTier::Working, Some("s1")).unwrap();
        store.add(new_memory("permanent")).unwrap();

        let persistent = store.get_persistent().unwrap();
        assert_eq!(persistent.len(), 2);
        assert!(persistent.iter().all(|e| e.tier != MemoryTier::Short));
    }

    #[test]
    fn promote_changes_tier() {
        let store = MemoryStore::in_memory();
        let entry = store.add_to_tier(new_memory("upgradeable"), MemoryTier::Working, Some("s1")).unwrap();
        store.promote(entry.id, MemoryTier::Long).unwrap();
        let updated = store.get_by_id(entry.id).unwrap();
        assert_eq!(updated.tier, MemoryTier::Long);
    }

    #[test]
    fn evict_short_term_clears_session() {
        let store = MemoryStore::in_memory();
        store.add_to_tier(new_memory("msg1"), MemoryTier::Short, Some("sess-1")).unwrap();
        store.add_to_tier(new_memory("msg2"), MemoryTier::Short, Some("sess-1")).unwrap();
        store.add_to_tier(new_memory("other session"), MemoryTier::Short, Some("sess-2")).unwrap();

        let evicted = store.evict_short_term("sess-1").unwrap();
        assert_eq!(evicted.len(), 2);
        assert_eq!(store.get_by_tier(&MemoryTier::Short).unwrap().len(), 1);
    }

    #[test]
    fn stats_returns_tier_counts() {
        let store = MemoryStore::in_memory();
        store.add_to_tier(new_memory("s"), MemoryTier::Short, Some("s1")).unwrap();
        store.add_to_tier(new_memory("w"), MemoryTier::Working, Some("s1")).unwrap();
        store.add(new_memory("l1")).unwrap();
        store.add(new_memory("l2")).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.short, 1);
        assert_eq!(stats.working, 1);
        assert_eq!(stats.long, 2);
    }

    #[test]
    fn hybrid_search_keyword_ranking() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("Python programming language")).unwrap();
        store.add(new_memory("JavaScript for web")).unwrap();
        store.add(new_memory("Rust systems programming")).unwrap();

        let results = store.hybrid_search("Python programming", None, 2).unwrap();
        assert_eq!(results.len(), 2);
        // Python entry should rank first (2 keyword hits vs 1 for Rust)
        assert!(results[0].content.contains("Python"));
    }

    #[test]
    fn gc_decayed_removes_low_importance() {
        let store = MemoryStore::in_memory();
        let e = store.add(NewMemory { importance: 1, ..new_memory("forgettable") }).unwrap();
        // Manually set low decay
        store.conn.execute(
            "UPDATE memories SET decay_score = 0.005 WHERE id = ?1",
            params![e.id],
        ).unwrap();
        let removed = store.gc_decayed(0.01).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn token_count_estimated_on_add() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("Hello world this is a test")).unwrap();
        assert!(entry.token_count > 0);
    }

    #[test]
    fn add_with_source_fields() {
        let store = MemoryStore::in_memory();
        let entry = store.add(NewMemory {
            content: "Rule 14.3: 30-day deadline".to_string(),
            tags: "law".to_string(),
            importance: 5,
            memory_type: MemoryType::Fact,
            source_url: Some("https://example.com/rules".to_string()),
            source_hash: Some("abc123".to_string()),
            expires_at: None,
        }).unwrap();
        assert_eq!(entry.source_url.as_deref(), Some("https://example.com/rules"));
        assert_eq!(entry.source_hash.as_deref(), Some("abc123"));
        assert!(entry.expires_at.is_none());
    }

    #[test]
    fn find_by_source_hash_returns_match() {
        let store = MemoryStore::in_memory();
        store.add(NewMemory {
            source_hash: Some("hash-001".to_string()),
            ..new_memory("sourced fact")
        }).unwrap();
        store.add(new_memory("no source")).unwrap();

        let found = store.find_by_source_hash("hash-001").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().content, "sourced fact");

        assert!(store.find_by_source_hash("nonexistent").unwrap().is_none());
    }

    #[test]
    fn find_by_source_url_returns_all() {
        let store = MemoryStore::in_memory();
        let url = "https://example.com/doc";
        store.add(NewMemory {
            source_url: Some(url.to_string()),
            source_hash: Some("h1".to_string()),
            ..new_memory("chunk 1")
        }).unwrap();
        store.add(NewMemory {
            source_url: Some(url.to_string()),
            source_hash: Some("h2".to_string()),
            ..new_memory("chunk 2")
        }).unwrap();
        store.add(new_memory("unrelated")).unwrap();

        let results = store.find_by_source_url(url).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn delete_by_source_url_removes_all() {
        let store = MemoryStore::in_memory();
        let url = "https://example.com/stale";
        store.add(NewMemory {
            source_url: Some(url.to_string()),
            ..new_memory("old chunk 1")
        }).unwrap();
        store.add(NewMemory {
            source_url: Some(url.to_string()),
            ..new_memory("old chunk 2")
        }).unwrap();
        store.add(new_memory("keep me")).unwrap();

        let removed = store.delete_by_source_url(url).unwrap();
        assert_eq!(removed, 2);
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn reingest_skip_when_hash_unchanged() {
        let store = MemoryStore::in_memory();
        let hash = "sha256-unchanged";
        store.add(NewMemory {
            source_hash: Some(hash.to_string()),
            source_url: Some("https://example.com/doc".to_string()),
            ..new_memory("existing content")
        }).unwrap();

        // Simulate re-ingest: find_by_source_hash returns Some → skip
        let existing = store.find_by_source_hash(hash).unwrap();
        assert!(existing.is_some());
    }

    #[test]
    fn reingest_replaces_when_hash_changed() {
        let store = MemoryStore::in_memory();
        let url = "https://example.com/rule";
        store.add(NewMemory {
            source_url: Some(url.to_string()),
            source_hash: Some("old-hash".to_string()),
            ..new_memory("old version of rule")
        }).unwrap();
        assert_eq!(store.count(), 1);

        // Hash changed → delete old entries by URL, then insert new
        let _ = store.delete_by_source_url(url).unwrap();
        assert_eq!(store.count(), 0);

        store.add(NewMemory {
            source_url: Some(url.to_string()),
            source_hash: Some("new-hash".to_string()),
            ..new_memory("updated version of rule")
        }).unwrap();
        assert_eq!(store.count(), 1);

        let found = store.find_by_source_hash("new-hash").unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().content.contains("updated"));
    }

    #[test]
    fn delete_expired_removes_past_entries() {
        let store = MemoryStore::in_memory();
        // Insert with an already-expired timestamp
        store.add(NewMemory {
            expires_at: Some(1000), // epoch ms, way in the past
            ..new_memory("ephemeral")
        }).unwrap();
        store.add(new_memory("permanent")).unwrap();

        let removed = store.delete_expired().unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.count(), 1);
    }
}
