use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::metrics::{Timer, METRICS};
use super::search_cache::SEARCH_CACHE;
use super::schema;

pub(crate) fn now_ms() -> i64 {
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
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Fact => "fact",
            MemoryType::Preference => "preference",
            MemoryType::Context => "context",
            MemoryType::Summary => "summary",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
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
    /// Soft-close timestamp (Unix ms). Non-NULL means this memory was superseded
    /// by a contradiction resolution and is no longer active. Never deleted.
    pub valid_to: Option<i64>,
    /// Relative path within the Obsidian vault (e.g. `TerranSoul/42-hello.md`).
    pub obsidian_path: Option<String>,
    /// Unix-ms timestamp of last successful export to Obsidian vault.
    pub last_exported: Option<i64>,
    /// Unix-ms timestamp of last mutation (for CRDT LWW sync).
    pub updated_at: Option<i64>,
    /// UUID of the device that last wrote this entry (for CRDT tiebreaker).
    pub origin_device: Option<String>,
    /// HLC counter for causal CRDT ordering (Chunk 42.3).
    pub hlc_counter: Option<i64>,
    /// Confidence score 0.0–1.0 (V20). Decayed per-cognitive-kind and
    /// boosted by reinforcement. Multiplied into hybrid search scores.
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    1.0
}

/// A single reinforcement event for provenance tracking (Chunk 43.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinforcementRecord {
    pub memory_id: i64,
    pub session_id: String,
    pub message_index: i64,
    pub ts: i64,
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

fn default_importance() -> i64 {
    3
}

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
    #[serde(rename = "short_count")]
    pub short: i64,
    #[serde(rename = "working_count")]
    pub working: i64,
    #[serde(rename = "long_count")]
    pub long: i64,
    pub embedded: i64,
    pub total_tokens: i64,
    pub avg_decay: f64,
    pub storage_bytes: i64,
    pub cache_bytes: i64,
}

/// Result of pruning memory rows to satisfy the configured storage cap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryCleanupReport {
    pub before_bytes: i64,
    pub after_bytes: i64,
    pub max_bytes: i64,
    pub deleted: usize,
}

/// SQLite-backed persistent memory store.
/// Number of mutations between automatic ANALYZE runs.
const ANALYZE_EVERY: u64 = 10_000;

pub struct MemoryStore {
    pub(crate) conn: Connection,
    /// Optional ANN index for fast vector search (Chunk 16.10).
    /// Initialized lazily on first vector operation.
    ann: std::cell::OnceCell<super::ann_index::AnnIndex>,
    /// Data directory for persisting the ANN index file.
    /// `None` for in-memory stores (tests).
    data_dir: Option<std::path::PathBuf>,
    /// Handle for debounced ANN flush (Chunk 41.10).  Set after
    /// construction via [`set_flush_handle`].
    flush_handle: Option<super::ann_flush::AnnFlushHandle>,
    /// Cumulative mutation counter (add/update/delete). When it crosses
    /// an `ANALYZE_EVERY` boundary, we run `ANALYZE` to keep the query
    /// planner statistics fresh (Chunk 41.12R).
    mutations: AtomicU64,
}

impl MemoryStore {
    /// Open (or create) the memory database at `data_dir/memory.db`.
    /// Falls back to an in-memory database if the file cannot be opened.
    /// Enables WAL mode for crash durability and creates an auto-backup.
    /// Creates the canonical memory schema.
    pub fn new(data_dir: &Path) -> Self {
        Self::new_with_config(data_dir, None, None)
    }

    /// Open with user-configurable cache/mmap sizes (from AppSettings).
    /// Pass `None` for either to use the platform default.
    pub fn new_with_config(data_dir: &Path, cache_mb: Option<u32>, mmap_mb: Option<u32>) -> Self {
        auto_backup(data_dir);
        let conn = Connection::open(data_dir.join("memory.db")).unwrap_or_else(|_| {
            Connection::open_in_memory()
                .expect("Failed to create in-memory SQLite fallback database")
        });
        // Phase 41.1 — write-path tuning for million-memory CRUD.
        // WAL mode: crash-safe, concurrent reads, no data loss.
        // foreign_keys=ON is required for ON DELETE CASCADE on memory_edges (V5).
        // Platform-adaptive: desktop gets aggressive cache/mmap; mobile
        // reduces resource usage and tightens WAL autocheckpoint (42.2).
        if cache_mb.is_some() || mmap_mb.is_some() {
            let pragmas = super::platform::production_pragmas_custom(
                cache_mb.unwrap_or(crate::settings::DEFAULT_SQLITE_CACHE_MB),
                mmap_mb.unwrap_or(crate::settings::DEFAULT_SQLITE_MMAP_MB),
            );
            let _ = conn.execute_batch(&pragmas);
        } else {
            let _ = conn.execute_batch(super::platform::production_pragmas());
        }
        schema::create_canonical_schema(&conn).expect("memory schema initialization failed");
        // Phase 41.12R — let SQLite analyse table statistics on open.
        let _ = conn.execute_batch("PRAGMA optimize;");
        MemoryStore {
            conn,
            ann: std::cell::OnceCell::new(),
            data_dir: Some(data_dir.to_path_buf()),
            flush_handle: None,
            mutations: AtomicU64::new(0),
        }
    }

    /// Create an in-memory store (for tests).
    pub fn in_memory() -> Self {
        let conn =
            Connection::open_in_memory().expect("Failed to create in-memory SQLite database");
        // foreign_keys=ON keeps test parity with the on-disk store and
        // exercises the V5 memory_edges cascade behaviour.
        // temp_store=MEMORY + cache_size keep test perf representative of prod.
        let _ = conn.execute_batch(super::platform::test_pragmas());
        schema::create_canonical_schema(&conn).expect("memory schema initialization failed");
        MemoryStore {
            conn,
            ann: std::cell::OnceCell::new(),
            data_dir: None,
            flush_handle: None,
            mutations: AtomicU64::new(0),
        }
    }

    /// Return the current schema version.
    pub fn schema_version(&self) -> i64 {
        schema::schema_version(&self.conn).unwrap_or(0)
    }

    /// Bump the mutation counter and run `ANALYZE` when it crosses a 10k boundary.
    fn record_mutations(&self, n: u64) {
        let prev = self.mutations.fetch_add(n, Ordering::Relaxed);
        // When we cross an ANALYZE_EVERY boundary, refresh planner stats.
        if prev / ANALYZE_EVERY != (prev + n) / ANALYZE_EVERY {
            let _ = self.conn.execute_batch("ANALYZE;");
        }
    }

    /// Internal accessor to the underlying SQLite connection. `pub(crate)`
    /// so sibling modules in `crate::memory` (e.g. `edges`) can issue their
    /// own SQL without exposing `rusqlite::Connection` to the rest of the app.
    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Lazily initialize and return the ANN index.
    ///
    /// On first call, detects the embedding dimensionality from the DB,
    /// then either loads the persisted index from disk or rebuilds it.
    /// Returns `None` if no embeddings exist yet (dimension unknown).
    fn ann_index(&self) -> Option<&super::ann_index::AnnIndex> {
        // If already initialized, return it.
        if let Some(idx) = self.ann.get() {
            return Some(idx);
        }
        // Detect dimensions from existing embeddings.
        let dims = super::ann_index::detect_dimensions(&self.conn)?;
        if dims == 0 {
            return None;
        }
        // Try to initialize the index.
        let idx = if let Some(dir) = &self.data_dir {
            super::ann_index::AnnIndex::open(dir, dims).ok()?
        } else {
            super::ann_index::AnnIndex::new(dims).ok()?
        };
        // If the index is empty, rebuild from DB.
        if idx.is_empty() {
            if let Ok(entries) = self.get_with_embeddings() {
                let iter = entries.iter().filter_map(|e| {
                    let emb = e.embedding.as_ref()?;
                    Some((e.id, emb.as_slice()))
                });
                let _ = idx.rebuild(iter);
            }
        }
        // Store in OnceCell (may race if called concurrently, but
        // MemoryStore is behind a Mutex so that won't happen).
        let _ = self.ann.set(idx);
        self.ann.get()
    }

    /// Initialize the ANN index with a known dimensionality (e.g. after
    /// the first embedding is computed).  No-op if already initialized.
    fn ensure_ann_for_dim(&self, dim: usize) -> Option<&super::ann_index::AnnIndex> {
        if let Some(idx) = self.ann.get() {
            if idx.dimensions() == dim {
                return Some(idx);
            }
            // Dimension changed — can't reinitialize a OnceCell, so fall
            // back to brute-force until restart.
            return None;
        }
        let idx = if let Some(dir) = &self.data_dir {
            super::ann_index::AnnIndex::open(dir, dim).ok()?
        } else {
            super::ann_index::AnnIndex::new(dim).ok()?
        };
        let _ = self.ann.set(idx);
        self.ann.get()
    }

    /// Insert a new memory entry and return it with its assigned id.
    pub fn add(&self, m: NewMemory) -> SqlResult<MemoryEntry> {
        let _t = Timer::start(&METRICS.add);
        SEARCH_CACHE.invalidate();
        let importance = m.importance.clamp(1, 5);
        let now = now_ms();
        let token_count = estimate_tokens(&m.content);
        self.conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count, tier, decay_score, token_count, source_url, source_hash, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, 1.0, ?7, ?8, ?9, ?10)",
            params![m.content, m.tags, importance, m.memory_type.as_str(), now, MemoryTier::Long.as_str(), token_count, m.source_url, m.source_hash, m.expires_at],
        )?;
        let id = self.conn.last_insert_rowid();
        self.record_mutations(1);
        self.get_by_id(id)
    }

    /// Bulk-insert many memories in a single transaction (Phase 41.4).
    ///
    /// Returns the assigned row ids in the same order as the input.
    /// Skips the per-row `get_by_id` round-trip — callers that need the
    /// full `MemoryEntry` should call `get_by_id` afterwards. This is the
    /// path used by ingest pipelines that turn one document into thousands
    /// of chunks; it lifts insert throughput from ~600 rows/s (per-row
    /// auto-commit + per-row fsync) to >100k rows/s on commodity hardware.
    pub fn add_many(&mut self, mut items: Vec<NewMemory>) -> SqlResult<Vec<i64>> {
        let _t = Timer::start(&METRICS.add_many);
        SEARCH_CACHE.invalidate();
        if items.is_empty() {
            return Ok(Vec::new());
        }
        let now = now_ms();
        let mut ids = Vec::with_capacity(items.len());
        let tier_str = MemoryTier::Long.as_str();
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count, tier, decay_score, token_count, source_url, source_hash, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, 1.0, ?7, ?8, ?9, ?10)",
            )?;
            for m in items.drain(..) {
                let importance = m.importance.clamp(1, 5);
                let token_count = estimate_tokens(&m.content);
                stmt.execute(params![
                    m.content,
                    m.tags,
                    importance,
                    m.memory_type.as_str(),
                    now,
                    tier_str,
                    token_count,
                    m.source_url,
                    m.source_hash,
                    m.expires_at,
                ])?;
                ids.push(tx.last_insert_rowid());
            }
        }
        tx.commit()?;
        self.record_mutations(ids.len() as u64);
        Ok(ids)
    }

    /// Bulk content update inside a single transaction.
    ///
    /// Used by ingest pipelines that re-write large batches of rows
    /// (e.g. re-chunking) and by the million-memory benchmark. Skips
    /// version snapshots, importance clamping, and the per-row
    /// `get_by_id` round-trip — callers wanting full semantics should
    /// use [`MemoryStore::update`] one row at a time.
    pub fn update_content_many(&mut self, items: &[(i64, String)]) -> SqlResult<()> {
        if items.is_empty() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        {
            let mut stmt =
                tx.prepare_cached("UPDATE memories SET content = ?1 WHERE id = ?2")?;
            for (id, content) in items {
                stmt.execute(params![content, id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Bulk delete inside a single transaction. Also removes the rows
    /// from the ANN index on a best-effort basis.
    pub fn delete_many(&mut self, ids: &[i64]) -> SqlResult<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached("DELETE FROM memories WHERE id = ?1")?;
            for id in ids {
                stmt.execute(params![id])?;
            }
        }
        tx.commit()?;
        if let Some(idx) = self.ann.get() {
            for id in ids {
                let _ = idx.remove(*id);
            }
        }
        Ok(())
    }

    /// Insert a memory into a specific tier (for session management).
    pub fn add_to_tier(
        &self,
        m: NewMemory,
        tier: MemoryTier,
        session_id: Option<&str>,
    ) -> SqlResult<MemoryEntry> {
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
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE id = ?1",
            params![id],
            row_to_entry,
        )
    }

    /// Return all memories ordered by importance (desc) then created_at (desc).
    pub fn get_all(&self) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories ORDER BY importance DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_entry)?;
        rows.collect()
    }

    /// Return the broad memory list capped by estimated in-memory bytes.
    ///
    /// This bounds UI/cache memory without deleting any persisted rows.
    pub fn get_all_within_storage_bytes(&self, max_bytes: u64) -> SqlResult<Vec<MemoryEntry>> {
        let max_bytes = max_bytes.min(i64::MAX as u64) as i64;
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence,
                    length(content)
                    + length(tags)
                    + COALESCE(length(embedding), 0)
                    + COALESCE(length(source_url), 0)
                    + COALESCE(length(source_hash), 0)
                    + COALESCE(length(obsidian_path), 0)
                    + 128 AS row_bytes
             FROM memories ORDER BY importance DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| Ok((row_to_entry(row)?, row.get::<_, i64>(23)?)))?;

        let mut used = 0i64;
        let mut entries = Vec::new();
        for row in rows {
            let (entry, row_bytes) = row?;
            let row_bytes = row_bytes.max(0);
            if used > 0 && used.saturating_add(row_bytes) > max_bytes {
                break;
            }
            used = used.saturating_add(row_bytes);
            entries.push(entry);
        }
        Ok(entries)
    }

    /// Return estimated bytes represented by the current in-memory list cache cap.
    pub fn active_cache_bytes(&self, max_bytes: u64) -> SqlResult<i64> {
        let max_bytes = max_bytes.min(i64::MAX as u64) as i64;
        let mut stmt = self.conn.prepare(
            "SELECT length(content)
                    + length(tags)
                    + COALESCE(length(embedding), 0)
                    + COALESCE(length(source_url), 0)
                    + COALESCE(length(source_hash), 0)
                    + COALESCE(length(obsidian_path), 0)
                    + 128 AS row_bytes
             FROM memories ORDER BY importance DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;

        let mut used = 0i64;
        for row in rows {
            let row_bytes = row?.max(0);
            if used > 0 && used.saturating_add(row_bytes) > max_bytes {
                break;
            }
            used = used.saturating_add(row_bytes);
        }
        Ok(used)
    }

    /// Return memories in a specific tier.
    pub fn get_by_tier(&self, tier: &MemoryTier) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE tier = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![tier.as_str()], row_to_entry)?;
        rows.collect()
    }

    /// Get working + long-term memories (skip short-term ephemeral).
    pub fn get_persistent(&self) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
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
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
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
    ///
    /// Saves a version snapshot of the *previous* state before applying
    /// the update (V8 schema, chunk 16.12). The snapshot is best-effort;
    /// if the versioning table doesn't exist yet (pre-V8 schema), the
    /// update still proceeds.
    pub fn update(&self, id: i64, upd: MemoryUpdate) -> SqlResult<MemoryEntry> {
        let _t = Timer::start(&METRICS.update);
        SEARCH_CACHE.invalidate();
        // Snapshot the current state before editing (best-effort).
        let content_changed = upd.content.is_some();
        let has_changes = content_changed
            || upd.tags.is_some()
            || upd.importance.is_some()
            || upd.memory_type.is_some();
        if has_changes {
            let _ = super::versioning::save_version(&self.conn, id);
        }

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

        // When content changes, the old embedding is stale.
        // Clear it, remove from ANN, and enqueue for re-embedding (41.6R).
        if content_changed {
            self.conn.execute(
                "UPDATE memories SET embedding = NULL WHERE id = ?1",
                params![id],
            )?;
            // Remove stale vector from the ANN index (best-effort).
            if let Some(idx) = self.ann.get() {
                let _ = idx.remove(id);
            }
            // Enqueue for re-embedding (best-effort — table may not exist
            // on very old schemas).
            let _ = super::embedding_queue::enqueue(&self.conn, id);
        }

        self.record_mutations(1);
        self.get_by_id(id)
    }

    /// Delete a memory entry by id.
    pub fn delete(&self, id: i64) -> SqlResult<()> {
        let _t = Timer::start(&METRICS.delete);
        SEARCH_CACHE.invalidate();
        self.conn
            .execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        // Remove from ANN index (best-effort).
        if let Some(idx) = self.ann.get() {
            let _ = idx.remove(id);
        }
        self.record_mutations(1);
        Ok(())
    }

    /// Soft-close a memory by setting `valid_to` to the given timestamp.
    /// The entry is never deleted — this preserves the audit trail and
    /// allows undo. Used by contradiction resolution (Chunk 17.2).
    pub fn close_memory(&self, id: i64, valid_to_ms: i64) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE memories SET valid_to = ?1 WHERE id = ?2",
            params![valid_to_ms, id],
        )?;
        Ok(())
    }

    /// Return the N most relevant memories for a message (keyword match + importance).
    /// Used to inject long-term context into the brain's system prompt.
    /// Uses candidate-pool retrieval (41.5R) instead of loading every row.
    pub fn relevant_for(&self, message: &str, limit: usize) -> Vec<String> {
        let words: Vec<String> = message
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(String::from)
            .collect();

        // Use candidate pool instead of get_all() (41.5R).
        let Ok(candidates) = self.search_candidates(&words, None) else {
            return vec![];
        };

        let mut scored: Vec<(usize, &MemoryEntry)> = candidates
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
        let _t = Timer::start(&METRICS.set_embedding);
        let bytes = embedding_to_bytes(embedding);
        self.conn.execute(
            "UPDATE memories SET embedding = ?1 WHERE id = ?2",
            params![bytes, id],
        )?;
        // Keep the ANN index in sync (best-effort).
        if let Some(idx) = self.ensure_ann_for_dim(embedding.len()) {
            let _ = idx.add(id, embedding);
        }
        Ok(())
    }

    /// Record obsidian sync metadata after a successful export/import.
    pub fn set_obsidian_sync(&self, id: i64, path: &str, exported_at: i64) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE memories SET obsidian_path = ?1, last_exported = ?2 WHERE id = ?3",
            params![path, exported_at, id],
        )?;
        Ok(())
    }

    /// Return all memories that have an embedding stored.
    pub fn get_with_embeddings(&self) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, embedding, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE embedding IS NOT NULL",
        )?;
        let rows = stmt.query_map([], row_to_entry_with_embedding)?;
        rows.collect()
    }

    // ── Candidate-pool helpers (Chunk 41.5R) ────────────────────────────────
    //
    // Instead of loading the entire corpus via get_all()/get_with_embeddings()
    // on every search, we gather candidate IDs from three fast retrievers
    // (ANN, keyword SQL, freshness SQL), union them, then fetch only those
    // rows.  This keeps memory usage O(candidate_pool) instead of O(total).

    /// Maximum number of candidates fetched from each retriever.
    const CANDIDATE_POOL: usize = 500;

    /// Fetch the IDs of the `pool` freshest + most-important memories.
    fn freshness_candidate_ids(&self, pool: usize) -> SqlResult<Vec<i64>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id FROM memories ORDER BY created_at DESC, importance DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![pool as i64], |row| row.get::<_, i64>(0))?;
        rows.collect()
    }

    /// Fetch the IDs of memories whose content or tags contain any of
    /// the given `words` (case-insensitive via `INSTR` + `LOWER`).
    /// Returns at most `pool` IDs.
    fn keyword_candidate_ids(&self, words: &[String], pool: usize) -> SqlResult<Vec<i64>> {
        if words.is_empty() {
            return Ok(Vec::new());
        }
        // Build a WHERE clause: LOWER(content) LIKE '%word1%' OR LOWER(tags) LIKE '%word1%' OR ...
        // Using INSTR is slightly faster than LIKE for substring matching.
        let conditions: Vec<String> = words
            .iter()
            .enumerate()
            .map(|(i, _)| {
                format!(
                    "(INSTR(LOWER(content), ?{p}) > 0 OR INSTR(LOWER(tags), ?{p}) > 0)",
                    p = i + 1
                )
            })
            .collect();
        let sql = format!(
            "SELECT id FROM memories WHERE {} LIMIT ?{}",
            conditions.join(" OR "),
            words.len() + 1
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let n_params = words.len() + 1;
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = words
            .iter()
            .map(|w| Box::new(w.to_lowercase()) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        param_values.push(Box::new(pool as i64));
        let refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(&*refs, |row| row.get::<_, i64>(0))?;
        let _ = n_params; // suppress unused warning
        rows.collect()
    }

    /// Fetch full entries (with embeddings) for a set of IDs.
    fn get_entries_by_ids_with_embeddings(&self, ids: &[i64]) -> SqlResult<Vec<MemoryEntry>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: String = (0..ids.len()).map(|i| format!("?{}", i + 1)).collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, embedding, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE id IN ({})",
            placeholders
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<Box<dyn rusqlite::types::ToSql>> = ids.iter().map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>).collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(&*refs, row_to_entry_with_embedding)?;
        rows.collect()
    }

    /// Fetch full entries (without embeddings) for a set of IDs.
    fn get_entries_by_ids(&self, ids: &[i64]) -> SqlResult<Vec<MemoryEntry>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: String = (0..ids.len()).map(|i| format!("?{}", i + 1)).collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE id IN ({})",
            placeholders
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<Box<dyn rusqlite::types::ToSql>> = ids.iter().map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>).collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(&*refs, row_to_entry)?;
        rows.collect()
    }

    /// Gather candidate IDs from ANN + keyword + freshness retrievers, then
    /// return the deduplicated union of entries (with embeddings if available).
    fn search_candidates(
        &self,
        query_words: &[String],
        query_embedding: Option<&[f32]>,
    ) -> SqlResult<Vec<MemoryEntry>> {
        use std::collections::HashSet;

        let pool = Self::CANDIDATE_POOL;
        let mut id_set: HashSet<i64> = HashSet::with_capacity(pool * 3);

        // (1) ANN vector candidates
        if let Some(qe) = query_embedding {
            if let Some(idx) = self.ann_index() {
                if let Ok(matches) = idx.search(qe, pool) {
                    id_set.extend(matches.iter().map(|(id, _)| *id));
                }
            }
        }

        // (2) Keyword candidates via SQL
        if let Ok(kw_ids) = self.keyword_candidate_ids(query_words, pool) {
            id_set.extend(kw_ids);
        }

        // (3) Freshness candidates
        if let Ok(fresh_ids) = self.freshness_candidate_ids(pool) {
            id_set.extend(fresh_ids);
        }

        // Fetch the full entries for the candidate set
        let ids: Vec<i64> = id_set.into_iter().collect();
        if query_embedding.is_some() {
            self.get_entries_by_ids_with_embeddings(&ids)
        } else {
            self.get_entries_by_ids(&ids)
        }
    }

    /// Return the IDs of entries that have no embedding yet (need processing).
    pub fn unembedded_ids(&self) -> SqlResult<Vec<(i64, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, content FROM memories WHERE embedding IS NULL")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect()
    }

    /// Count memories that have embeddings.
    pub fn embedded_count(&self) -> Result<usize, String> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE embedding IS NOT NULL",
                [],
                |row| row.get::<_, usize>(0),
            )
            .map_err(|e| e.to_string())
    }

    /// Clear all embeddings (set to NULL) for re-embedding with a new model.
    pub fn clear_all_embeddings(&self) -> Result<usize, String> {
        self.conn
            .execute("UPDATE memories SET embedding = NULL", [])
            .map_err(|e| e.to_string())
    }

    /// Fast cosine-similarity vector search.  Returns the top `limit`
    /// memory entries ranked by similarity to `query_embedding`.
    ///
    /// Uses the HNSW ANN index (Chunk 16.10) when available for O(log n)
    /// lookup; falls back to brute-force O(n) scan when the index is
    /// missing, empty, or has a dimension mismatch.
    pub fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> SqlResult<Vec<MemoryEntry>> {
        // ── Fast path: ANN index ──────────────────────────────────────────
        if let Some(idx) = self.ann_index() {
            if let Ok(matches) = idx.search(query_embedding, limit) {
                if !matches.is_empty() {
                    let now = now_ms();
                    let mut results = Vec::with_capacity(matches.len());
                    for (id, _sim) in &matches {
                        if let Ok(entry) = self.get_by_id(*id) {
                            let _ = self.conn.execute(
                                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                                params![now, entry.id],
                            );
                            results.push(entry);
                        }
                    }
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }

        // ── Fallback: brute-force scan ────────────────────────────────────
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
    ///
    /// Uses ANN index when available; falls back to brute-force scan.
    pub fn find_duplicate(
        &self,
        query_embedding: &[f32],
        threshold: f32,
    ) -> SqlResult<Option<i64>> {
        // ── Fast path: ANN index ──────────────────────────────────────────
        if let Some(idx) = self.ann_index() {
            if let Ok(matches) = idx.search(query_embedding, 1) {
                if let Some(&(id, sim)) = matches.first() {
                    if sim >= threshold {
                        return Ok(Some(id));
                    }
                }
                return Ok(None);
            }
        }

        // ── Fallback: brute-force scan ────────────────────────────────────
        let all = self.get_with_embeddings()?;
        let best = all
            .iter()
            .filter_map(|e| {
                let emb = e.embedding.as_ref()?;
                let sim = cosine_similarity(query_embedding, emb);
                if sim >= threshold {
                    Some((sim, e.id))
                } else {
                    None
                }
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
    /// Like [`Self::hybrid_search`], but also filters out entries whose
    /// final hybrid score is below `min_score` (a value in `[0.0, 1.0]`,
    /// since the per-component weights sum to ≤ 1.0 by construction —
    /// see the inline weights above).
    ///
    /// Implements the "relevance threshold" item from
    /// `docs/brain-advanced-design.md` § 16 Phase 4 (Chunk 16.1). Pass
    /// `min_score = 0.0` to get the legacy behaviour (no filtering).
    ///
    /// Returns the surviving entries in descending score order, capped
    /// at `limit`. Touches `last_accessed` / `access_count` for survivors
    /// only — entries below the threshold do **not** count as accesses,
    /// which preserves the decay signal for genuinely irrelevant rows.
    pub fn hybrid_search_with_threshold(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
        min_score: f64,
    ) -> SqlResult<Vec<MemoryEntry>> {
        let scored = self.hybrid_search_scored(query, query_embedding)?;
        let now = now_ms();

        let kept: Vec<MemoryEntry> = scored
            .into_iter()
            .filter(|(s, _)| *s >= min_score)
            .take(limit)
            .map(|(_, e)| e)
            .collect();

        // Touch access counters for survivors only. Below-threshold rows
        // are intentionally NOT counted as accesses — the decay signal
        // should keep them ageing out of relevance.
        for e in &kept {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }

        Ok(kept)
    }

    /// Internal helper that returns every entry with its hybrid score,
    /// already sorted descending. Pure read — does not touch
    /// `access_count`. Shared between [`Self::hybrid_search`] and
    /// [`Self::hybrid_search_with_threshold`] so the two stay in lockstep.
    fn hybrid_search_scored(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
    ) -> SqlResult<Vec<(f64, MemoryEntry)>> {
        let now = now_ms();
        let hour_ms: f64 = 3_600_000.0;

        let words: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(String::from)
            .collect();

        // Use candidate-pool retrieval (41.5R) instead of loading entire corpus.
        let all = self.search_candidates(&words, query_embedding)?;

        if all.is_empty() {
            return Ok(vec![]);
        }

        let mut scored: Vec<(f64, MemoryEntry)> = all
            .into_iter()
            .map(|entry| {
                let mut score = 0.0f64;

                if let (Some(qe), Some(emb)) = (query_embedding, entry.embedding.as_ref()) {
                    let sim = cosine_similarity(qe, emb) as f64;
                    score += sim * 0.40;
                }

                let lower_content = entry.content.to_lowercase();
                let lower_tags = entry.tags.to_lowercase();
                let keyword_hits = words
                    .iter()
                    .filter(|w| {
                        lower_content.contains(w.as_str()) || lower_tags.contains(w.as_str())
                    })
                    .count();
                if !words.is_empty() {
                    score += (keyword_hits as f64 / words.len() as f64) * 0.20;
                }

                let age_hours = (now - entry.created_at) as f64 / hour_ms;
                let recency = (-age_hours / 24.0).exp();
                score += recency * 0.15;

                score += (entry.importance as f64 / 5.0) * 0.10;
                score += entry.decay_score * 0.10;

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
        Ok(scored)
    }

    pub fn hybrid_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> SqlResult<Vec<MemoryEntry>> {
        let _t = Timer::start(&METRICS.hybrid_search);
        let now = now_ms();
        let hour_ms: f64 = 3_600_000.0;

        // Keyword scoring setup
        let words: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(String::from)
            .collect();

        // Use candidate-pool retrieval (41.5R) instead of loading entire corpus.
        let all = self.search_candidates(&words, query_embedding)?;

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
                let keyword_hits = words
                    .iter()
                    .filter(|w| {
                        lower_content.contains(w.as_str()) || lower_tags.contains(w.as_str())
                    })
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

    /// Hybrid search using **Reciprocal Rank Fusion** (RRF) over three
    /// independent retrievers:
    ///
    /// 1. **Vector** ranking — cosine similarity vs `query_embedding`
    ///    (skipped if no embedding is provided).
    /// 2. **Keyword** ranking — count of distinct query tokens that appear
    ///    in the memory's content or tags (case-insensitive, words shorter
    ///    than 3 chars are ignored, BM25-style).
    /// 3. **Freshness** ranking — composite of recency (24 h half-life),
    ///    importance (1–5), `decay_score`, and tier weight (Working >
    ///    Long > Short).
    ///
    /// The three rankings are fused with [`crate::memory::fusion::reciprocal_rank_fuse`]
    /// using the standard `k = 60` constant (see Cormack et al., SIGIR 2009).
    ///
    /// RRF is preferred over the weighted-sum fusion in [`Self::hybrid_search`]
    /// when the underlying retrievers have **incomparable score scales**:
    /// raw cosine similarity (~0.0–1.0), keyword hit ratio (0.0–1.0), and
    /// freshness composites are all on different distributions, so summing
    /// them with hand-tuned weights is fragile. RRF operates purely on
    /// rank position, giving robust, parameter-light fusion.
    ///
    /// Implements §16 Phase 6 / §19.2 row 2 of `docs/brain-advanced-design.md`.
    /// Scales linearly in the number of memories with embeddings (the
    /// vector pass is the dominant cost; ~5 ms for 100 k entries).
    pub fn hybrid_search_rrf(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> SqlResult<Vec<MemoryEntry>> {
        let _t = Timer::start(&METRICS.hybrid_search_rrf);
        use crate::memory::cognitive_kind::classify as classify_kind;
        use crate::memory::confidence_decay::{confidence_factor, ConfidenceDecayConfig};
        use crate::memory::fusion::{reciprocal_rank_fuse, DEFAULT_RRF_K};
        use crate::memory::search_cache::{CachedHit, SearchCacheKey, SEARCH_CACHE};
        use std::collections::HashMap;

        if limit == 0 {
            return Ok(vec![]);
        }

        // ── Cache check ───────────────────────────────────────────────────
        let cache_key = SearchCacheKey {
            query: query.to_string(),
            mode: if query_embedding.is_some() { "rrf_vec".into() } else { "rrf".into() },
            limit,
        };
        if let Some(cached) = SEARCH_CACHE.get(&cache_key) {
            let _ch = Timer::start(&METRICS.rag_cache_hit);
            let ids: Vec<i64> = cached.iter().map(|h| h.memory_id).collect();
            // Fetch full entries preserving cached order.
            let mut by_id: HashMap<i64, MemoryEntry> = self
                .get_entries_by_ids(&ids)?
                .into_iter()
                .map(|e| (e.id, e))
                .collect();
            let results: Vec<MemoryEntry> = ids
                .into_iter()
                .filter_map(|id| by_id.remove(&id))
                .collect();
            return Ok(results);
        }

        let now = now_ms();
        let hour_ms: f64 = 3_600_000.0;

        // Keyword scoring setup (also used for candidate retrieval)
        let words: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(String::from)
            .collect();

        // Use candidate-pool retrieval (41.5R) instead of loading entire corpus.
        let all = {
            let _tc = Timer::start(&METRICS.rag_candidate_retrieval);
            self.search_candidates(&words, query_embedding)?
        };

        if all.is_empty() {
            return Ok(vec![]);
        }

        // Index entries by id once so we can rebuild MemoryEntry ordering
        // after fusion without cloning the vector twice.
        let by_id: HashMap<i64, MemoryEntry> = all.iter().map(|e| (e.id, e.clone())).collect();

        // ── (1) Vector ranking ────────────────────────────────────────────
        let mut vector_rank: Vec<i64> = Vec::new();
        if let Some(qe) = query_embedding {
            let mut scored: Vec<(f32, i64)> = all
                .iter()
                .filter_map(|e| {
                    let emb = e.embedding.as_ref()?;
                    Some((cosine_similarity(qe, emb), e.id))
                })
                .collect();
            // Descending by similarity. Tie-break by id for determinism.
            scored.sort_by(|a, b| {
                b.0.partial_cmp(&a.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.1.cmp(&b.1))
            });
            vector_rank = scored.into_iter().map(|(_, id)| id).collect();
        }

        let mut keyword_rank: Vec<i64> = Vec::new();
        if !words.is_empty() {
            let mut scored: Vec<(usize, i64)> = all
                .iter()
                .filter_map(|e| {
                    let lower_content = e.content.to_lowercase();
                    let lower_tags = e.tags.to_lowercase();
                    let hits = words
                        .iter()
                        .filter(|w| {
                            lower_content.contains(w.as_str()) || lower_tags.contains(w.as_str())
                        })
                        .count();
                    if hits > 0 {
                        Some((hits, e.id))
                    } else {
                        None
                    }
                })
                .collect();
            // Descending by hit count, deterministic id tie-break.
            scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
            keyword_rank = scored.into_iter().map(|(_, id)| id).collect();
        }

        // ── (3) Freshness composite ranking ───────────────────────────────
        let mut freshness_scored: Vec<(f64, i64)> = all
            .iter()
            .map(|e| {
                let age_hours = (now - e.created_at) as f64 / hour_ms;
                let recency = (-age_hours / 24.0).exp();
                let importance = e.importance as f64 / 5.0;
                let tier_boost = match e.tier {
                    MemoryTier::Working => 1.0,
                    MemoryTier::Long => 0.5,
                    MemoryTier::Short => 0.3,
                };
                // Equal-weighted composite — RRF only cares about ordering.
                let score = recency + importance + e.decay_score + tier_boost;
                (score, e.id)
            })
            .collect();
        freshness_scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
        });
        let freshness_rank: Vec<i64> = freshness_scored.into_iter().map(|(_, id)| id).collect();

        // ── Fuse with RRF (k = 60) ────────────────────────────────────────
        // Build the slice-of-slices input. Empty rankings (e.g. no embedding
        // or no usable query words) are simply skipped — RRF handles
        // missing-from-some-rankings gracefully.
        let _tf = Timer::start(&METRICS.rag_rrf_fusion);
        let mut rankings: Vec<&[i64]> = Vec::with_capacity(3);
        if !vector_rank.is_empty() {
            rankings.push(&vector_rank);
        }
        if !keyword_rank.is_empty() {
            rankings.push(&keyword_rank);
        }
        rankings.push(&freshness_rank);

        let fused = reciprocal_rank_fuse(&rankings, DEFAULT_RRF_K);

        // ── (4) Per-kind confidence decay (43.3) ──────────────────────────
        let decay_cfg = ConfidenceDecayConfig::default();
        let mut fused: Vec<(usize, i64, f64)> = fused
            .into_iter()
            .enumerate()
            .map(|(pos, (id, score))| {
                let adjusted = if let Some(entry) = by_id.get(&id) {
                    let kind = classify_kind(&entry.memory_type, &entry.tags, &entry.content);
                    let factor = confidence_factor(&decay_cfg, Some(kind), entry.confidence, now - entry.created_at);
                    score * factor
                } else {
                    score
                };
                (pos, id, adjusted)
            })
            .collect();
        // Re-sort: descending score, preserving RRF position order for ties.
        fused.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        // Store result in cache for future lookups.
        let cached_hits: Vec<CachedHit> = fused
            .iter()
            .take(limit)
            .map(|(_, id, score)| CachedHit { memory_id: *id, score: *score })
            .collect();
        SEARCH_CACHE.put(cache_key, cached_hits);

        // Materialize the top-`limit` MemoryEntry list, preserving fused order.
        let top: Vec<MemoryEntry> = fused
            .into_iter()
            .take(limit)
            .filter_map(|(_, id, _)| by_id.get(&id).cloned())
            .collect();

        // Touch access counters for the matched entries.
        for e in &top {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }

        Ok(top)
    }

    /// Query-intent–aware variant of [`hybrid_search_rrf`] (Chunk 16.6c).
    ///
    /// Runs the same RRF fusion as `hybrid_search_rrf`, then applies
    /// per-doc multiplicative score boosts derived from the user's
    /// **query intent** (procedural / episodic / factual / semantic /
    /// unknown). The boost for each doc is looked up from
    /// [`crate::memory::query_intent::IntentClassification::kind_boosts`]
    /// using the doc's classified [`CognitiveKind`].
    ///
    /// When the classifier returns `Unknown` (no signal) all boosts are
    /// 1.0, so this method becomes equivalent to `hybrid_search_rrf` —
    /// callers can use it unconditionally.
    ///
    /// Per `docs/brain-advanced-design.md` §3.5.6.
    pub fn hybrid_search_rrf_with_intent(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> SqlResult<Vec<MemoryEntry>> {
        use crate::memory::cognitive_kind::classify as classify_kind;
        use crate::memory::confidence_decay::{confidence_factor, ConfidenceDecayConfig};
        use crate::memory::fusion::{reciprocal_rank_fuse, DEFAULT_RRF_K};
        use crate::memory::query_intent::classify_query;
        use std::collections::HashMap;

        if limit == 0 {
            return Ok(vec![]);
        }

        // Classify intent up-front. If the classifier returns Unknown
        // with neutral boosts we can skip the rerank cleanly.
        let intent = classify_query(query);
        let needs_rerank = !matches!(
            intent.intent,
            crate::memory::query_intent::QueryIntent::Unknown
        );

        let now = now_ms();
        let hour_ms: f64 = 3_600_000.0;

        // Keyword words (also used for candidate-pool retrieval)
        let words: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(String::from)
            .collect();

        // Use candidate-pool retrieval (41.5R) instead of loading entire corpus.
        let all = self.search_candidates(&words, query_embedding)?;
        if all.is_empty() {
            return Ok(vec![]);
        }

        let by_id: HashMap<i64, MemoryEntry> = all.iter().map(|e| (e.id, e.clone())).collect();

        // ── (1) Vector ranking ────────────────────────────────────────
        let mut vector_rank: Vec<i64> = Vec::new();
        if let Some(qe) = query_embedding {
            let mut scored: Vec<(f32, i64)> = all
                .iter()
                .filter_map(|e| {
                    let emb = e.embedding.as_ref()?;
                    Some((cosine_similarity(qe, emb), e.id))
                })
                .collect();
            scored.sort_by(|a, b| {
                b.0.partial_cmp(&a.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.1.cmp(&b.1))
            });
            vector_rank = scored.into_iter().map(|(_, id)| id).collect();
        }

        // ── (2) Keyword ranking ───────────────────────────────────────

        let mut keyword_rank: Vec<i64> = Vec::new();
        if !words.is_empty() {
            let mut scored: Vec<(usize, i64)> = all
                .iter()
                .filter_map(|e| {
                    let lower_content = e.content.to_lowercase();
                    let lower_tags = e.tags.to_lowercase();
                    let hits = words
                        .iter()
                        .filter(|w| {
                            lower_content.contains(w.as_str()) || lower_tags.contains(w.as_str())
                        })
                        .count();
                    if hits > 0 {
                        Some((hits, e.id))
                    } else {
                        None
                    }
                })
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
            keyword_rank = scored.into_iter().map(|(_, id)| id).collect();
        }

        // ── (3) Freshness composite ───────────────────────────────────
        let mut freshness_scored: Vec<(f64, i64)> = all
            .iter()
            .map(|e| {
                let age_hours = (now - e.created_at) as f64 / hour_ms;
                let recency = (-age_hours / 24.0).exp();
                let importance = e.importance as f64 / 5.0;
                let tier_boost = match e.tier {
                    MemoryTier::Working => 1.0,
                    MemoryTier::Long => 0.5,
                    MemoryTier::Short => 0.3,
                };
                let score = recency + importance + e.decay_score + tier_boost;
                (score, e.id)
            })
            .collect();
        freshness_scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
        });
        let freshness_rank: Vec<i64> = freshness_scored.into_iter().map(|(_, id)| id).collect();

        // ── Fuse with RRF ─────────────────────────────────────────────
        let mut rankings: Vec<&[i64]> = Vec::with_capacity(3);
        if !vector_rank.is_empty() {
            rankings.push(&vector_rank);
        }
        if !keyword_rank.is_empty() {
            rankings.push(&keyword_rank);
        }
        rankings.push(&freshness_rank);

        let mut fused = reciprocal_rank_fuse(&rankings, DEFAULT_RRF_K);

        // ── (4a) Per-kind confidence decay (43.3) ─────────────────────
        let decay_cfg = ConfidenceDecayConfig::default();
        for (id, score) in fused.iter_mut() {
            if let Some(entry) = by_id.get(id) {
                let kind = classify_kind(&entry.memory_type, &entry.tags, &entry.content);
                let factor = confidence_factor(&decay_cfg, Some(kind), entry.confidence, now - entry.created_at);
                *score *= factor;
            }
        }

        // ── (4b) Intent-aware kind boosting ────────────────────────────
        if needs_rerank {
            // Multiply each fused score by the per-kind boost for the
            // doc's classified cognitive kind, then re-sort.
            for (id, score) in fused.iter_mut() {
                if let Some(entry) = by_id.get(id) {
                    let kind = classify_kind(&entry.memory_type, &entry.tags, &entry.content);
                    let boost = intent.kind_boosts.for_kind(kind) as f64;
                    *score *= boost;
                }
            }
            // Stable re-sort by boosted score (descending), id tie-break.
            fused.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });
        }

        let top: Vec<MemoryEntry> = fused
            .into_iter()
            .take(limit)
            .filter_map(|(id, _)| by_id.get(&id).cloned())
            .collect();

        for e in &top {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }

        Ok(top)
    }

    // ── Reinforcement provenance (43.4) ────────────────────────────────────

    /// Record that a memory was reinforced (confirmed useful) during a session.
    ///
    /// Uses `INSERT OR IGNORE` so repeated calls with the same
    /// `(memory_id, session_id, message_index)` PK are idempotent.
    pub fn record_reinforcement(
        &self,
        memory_id: i64,
        session_id: &str,
        message_index: i64,
    ) -> SqlResult<()> {
        let now = now_ms();
        self.conn.execute(
            "INSERT OR IGNORE INTO memory_reinforcements (memory_id, session_id, message_index, ts)
             VALUES (?1, ?2, ?3, ?4)",
            params![memory_id, session_id, message_index, now],
        )?;
        Ok(())
    }

    /// Retrieve the most recent reinforcements for a memory entry.
    pub fn get_reinforcements(
        &self,
        memory_id: i64,
        limit: usize,
    ) -> SqlResult<Vec<ReinforcementRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT memory_id, session_id, message_index, ts
             FROM memory_reinforcements
             WHERE memory_id = ?1
             ORDER BY ts DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![memory_id, limit as i64], |row| {
            Ok(ReinforcementRecord {
                memory_id: row.get(0)?,
                session_id: row.get(1)?,
                message_index: row.get(2)?,
                ts: row.get(3)?,
            })
        })?;
        rows.collect()
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
            "SELECT id, last_accessed, decay_score, tags FROM memories WHERE tier = 'long'",
        )?;
        let rows: Vec<(i64, Option<i64>, f64, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut updated = 0;
        for (id, last_accessed, current_decay, tags) in &rows {
            let last = last_accessed.unwrap_or(now);
            let hours_since = (now - last) as f64 / 3_600_000.0;
            // Chunk 18.2 — category-aware decay: per-prefix multiplier
            // pulled from the curated vocabulary. Lower multiplier = slower
            // decay (more durable). `personal:*` uses 0.5 → decays slower
            // than the 1.0 baseline; `tool:*` uses 1.5 → decays faster.
            // Default 1.0 for legacy / non-conforming tags.
            let multiplier = crate::memory::tag_vocabulary::category_decay_multiplier(tags);
            let factor = 0.95f64.powf((hours_since / 168.0) * multiplier);
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
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
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

    /// Promote working-tier entries to long-tier when they pass an
    /// access-pattern threshold. Pure SQL — no LLM, no embedding I/O.
    ///
    /// An entry is promoted when **both** conditions hold:
    /// 1. `access_count >= min_access_count` (default 5)
    /// 2. The most recent access (`last_accessed`) falls within the last
    ///    `window_days` days (default 7).
    ///
    /// Returns the IDs of promoted entries (in ascending id order). Designed
    /// to be called periodically (e.g. on app startup, alongside `apply_decay`).
    /// Idempotent — re-running on an already-long entry is a no-op because
    /// only `tier = 'working'` rows are considered.
    ///
    /// Maps to `docs/brain-advanced-design.md` § 16 Phase 5
    /// "Auto-promotion based on access patterns".
    pub fn auto_promote_to_long(
        &self,
        min_access_count: i64,
        window_days: i64,
    ) -> SqlResult<Vec<i64>> {
        // Defensive — non-positive window means "no recency requirement"; we
        // still treat 0 as "any time" to avoid silently promoting nothing.
        let cutoff_ms = if window_days <= 0 {
            0
        } else {
            now_ms().saturating_sub(window_days.saturating_mul(86_400_000))
        };
        let min_count = min_access_count.max(0);

        let mut stmt = self.conn.prepare(
            "SELECT id FROM memories
             WHERE tier = 'working'
               AND access_count >= ?1
               AND last_accessed IS NOT NULL
               AND last_accessed >= ?2
             ORDER BY id ASC",
        )?;
        let rows: Vec<i64> = stmt
            .query_map(params![min_count, cutoff_ms], |row| row.get::<_, i64>(0))?
            .filter_map(|r| r.ok())
            .collect();

        for id in &rows {
            self.conn.execute(
                "UPDATE memories SET tier = 'long' WHERE id = ?1",
                params![id],
            )?;
        }
        Ok(rows)
    }

    /// Nudge memory importance based on access patterns (Chunk 17.4).
    ///
    /// * Entries with `access_count >= hot_threshold` (default 10) gain +1
    ///   importance (capped at 5).
    /// * Entries with `access_count == 0` whose `last_accessed` is older
    ///   than `cold_days` (default 30) or NULL lose −1 importance (floored
    ///   at 1).
    ///
    /// Each adjustment is audited via `memory_versions` (V8 schema).
    /// Designed to be called periodically (daily or on app startup).
    /// Returns `(boosted, demoted)` counts.
    ///
    /// Maps to `docs/brain-advanced-design.md` §16 Phase 5
    /// "Memory importance auto-adjustment from access_count".
    pub fn adjust_importance_by_access(
        &self,
        hot_threshold: i64,
        cold_days: i64,
    ) -> SqlResult<(usize, usize)> {
        let hot = hot_threshold.max(1);
        let cold_cutoff = if cold_days <= 0 {
            now_ms() // everything is "cold" — edge case, still safe
        } else {
            now_ms().saturating_sub(cold_days.saturating_mul(86_400_000))
        };

        // ── Boost hot entries ──
        let mut boost_stmt = self.conn.prepare(
            "SELECT id, importance FROM memories
             WHERE access_count >= ?1
               AND importance < 5",
        )?;
        let hot_rows: Vec<(i64, i64)> = boost_stmt
            .query_map(params![hot], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let mut boosted = 0usize;
        for (id, current_imp) in &hot_rows {
            let new_imp = (*current_imp + 1).min(5);
            // Audit trail (best-effort; silently ignored on pre-V8 schemas)
            let _ = super::versioning::save_version(&self.conn, *id);
            self.conn.execute(
                "UPDATE memories SET importance = ?1 WHERE id = ?2",
                params![new_imp, id],
            )?;
            boosted += 1;
        }

        // ── Demote cold entries ──
        let mut cold_stmt = self.conn.prepare(
            "SELECT id, importance FROM memories
             WHERE access_count = 0
               AND (last_accessed IS NULL OR last_accessed < ?1)
               AND importance > 1",
        )?;
        let cold_rows: Vec<(i64, i64)> = cold_stmt
            .query_map(params![cold_cutoff], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let mut demoted = 0usize;
        for (id, current_imp) in &cold_rows {
            let new_imp = (*current_imp - 1).max(1);
            let _ = super::versioning::save_version(&self.conn, *id);
            self.conn.execute(
                "UPDATE memories SET importance = ?1 WHERE id = ?2",
                params![new_imp, id],
            )?;
            demoted += 1;
        }

        // Reset access_count for boosted entries so the next run doesn't
        // re-boost the same entries that already graduated.
        for (id, _) in &hot_rows {
            self.conn.execute(
                "UPDATE memories SET access_count = 0 WHERE id = ?1",
                params![id],
            )?;
        }

        Ok((boosted, demoted))
    }

    /// Find a memory by its source_hash.  Returns the first match (if any).
    pub fn find_by_source_hash(&self, hash: &str) -> SqlResult<Option<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
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
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE source_url = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![url], row_to_entry)?;
        rows.collect()
    }

    /// Delete all memories from a given source URL.  Returns the count deleted.
    pub fn delete_by_source_url(&self, url: &str) -> SqlResult<usize> {
        let deleted = self
            .conn
            .execute("DELETE FROM memories WHERE source_url = ?1", params![url])?;
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

    /// Estimate the active storage used by memory/RAG rows.
    pub fn active_storage_bytes(&self) -> SqlResult<i64> {
        self.conn.query_row(
            "SELECT COALESCE(SUM(
                length(content)
                + length(tags)
                + COALESCE(length(embedding), 0)
                + COALESCE(length(source_url), 0)
                + COALESCE(length(source_hash), 0)
                + COALESCE(length(obsidian_path), 0)
                + 128
            ), 0) FROM memories",
            [],
            |r| r.get(0),
        )
    }

    /// Prune the least-useful memories until estimated active storage is under `max_bytes`.
    pub fn enforce_size_limit(&self, max_bytes: u64) -> SqlResult<MemoryCleanupReport> {
        let max_bytes = max_bytes.min(i64::MAX as u64) as i64;
        let before = self.active_storage_bytes()?;
        if before <= max_bytes {
            return Ok(MemoryCleanupReport {
                before_bytes: before,
                after_bytes: before,
                max_bytes,
                deleted: 0,
            });
        }

        let mut stmt = self.conn.prepare(
            "SELECT id,
                    length(content)
                    + length(tags)
                    + COALESCE(length(embedding), 0)
                    + COALESCE(length(source_url), 0)
                    + COALESCE(length(source_hash), 0)
                    + COALESCE(length(obsidian_path), 0)
                    + 128 AS row_bytes
             FROM memories
             ORDER BY
                CASE tier WHEN 'short' THEN 0 WHEN 'working' THEN 1 ELSE 2 END ASC,
                importance ASC,
                decay_score ASC,
                COALESCE(last_accessed, 0) ASC,
                access_count ASC,
                created_at ASC",
        )?;
        let candidates: Vec<(i64, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<SqlResult<Vec<_>>>()?;
        drop(stmt);

        let mut current = before;
        let mut deleted = 0usize;
        for (id, row_bytes) in candidates {
            if current <= max_bytes {
                break;
            }
            self.delete(id)?;
            current = current.saturating_sub(row_bytes.max(0));
            deleted += 1;
        }

        if deleted > 0 && self.data_dir.is_some() {
            let _ = self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }

        Ok(MemoryCleanupReport {
            before_bytes: before,
            after_bytes: self.active_storage_bytes()?,
            max_bytes,
            deleted,
        })
    }

    /// Delete **all** memories, edges, and conflicts. Returns the count of
    /// deleted memory rows. The ANN index is rebuilt empty.
    ///
    /// This is an irreversible destructive operation — the frontend must
    /// confirm with the user before calling.
    pub fn delete_all(&self) -> SqlResult<usize> {
        // Edges and conflicts cascade via FK, but be explicit for backends
        // that may not enforce FK cascades.
        self.conn
            .execute_batch(
                "DELETE FROM memory_edges;
             DELETE FROM memory_conflicts;
             DELETE FROM memory_versions;",
            )
            .ok(); // tables may not exist on older schemas — ignore errors
        let deleted = self.conn.execute("DELETE FROM memories", [])?;
        // Rebuild ANN index empty.
        if let Some(idx) = self.ann.get() {
            let _ = idx.rebuild(std::iter::empty());
        }
        Ok(deleted)
    }

    /// Get memory statistics per tier.
    pub fn stats(&self) -> SqlResult<MemoryStats> {
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))?;
        let short: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE tier='short'",
            [],
            |r| r.get(0),
        )?;
        let working: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE tier='working'",
            [],
            |r| r.get(0),
        )?;
        let long: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM memories WHERE tier='long'", [], |r| {
                    r.get(0)
                })?;
        let embedded: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE embedding IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        let total_tokens: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(token_count), 0) FROM memories",
            [],
            |r| r.get(0),
        )?;
        let avg_decay: f64 = self.conn.query_row(
            "SELECT COALESCE(AVG(decay_score), 1.0) FROM memories WHERE tier='long'",
            [],
            |r| r.get(0),
        )?;
        let storage_bytes = self.active_storage_bytes()?;
        Ok(MemoryStats {
            total,
            short,
            working,
            long,
            embedded,
            total_tokens,
            avg_decay,
            storage_bytes,
            cache_bytes: storage_bytes,
        })
    }

    /// Count long-tier memories with vector embeddings, which is the
    /// health signal behind `rag_quality_pct`.
    pub fn embedded_long_count(&self) -> SqlResult<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE tier='long' AND embedding IS NOT NULL",
            [],
            |r| r.get(0),
        )
    }

    // ── Phase 41 ANN management methods ────────────────────────────────────────

    /// Store the ANN flush handle so insert/update paths can signal dirty state.
    pub fn set_flush_handle(&mut self, handle: super::ann_flush::AnnFlushHandle) {
        self.flush_handle = Some(handle);
    }

    /// Save all ANN indices to disk.  Returns `(flush_count, ops_flushed)`.
    /// Called by the background flush task.
    pub fn ann_save_all(&self) -> (u64, u64) {
        if let Some(idx) = self.ann.get() {
            let _ = idx.save();
        }
        // Return (1, 0) to signal a flush happened even if save was a no-op.
        (1, 0)
    }

    /// Returns `true` if the ANN index has fragmentation above the compaction
    /// threshold (20%).  Returns `false` if no index exists.
    pub fn ann_needs_compaction(&self) -> bool {
        const COMPACTION_THRESHOLD: f32 = 0.20;
        match self.ann.get() {
            Some(idx) => idx.fragmentation_ratio() > COMPACTION_THRESHOLD,
            None => false,
        }
    }

    /// Rebuild the ANN index from live long-tier entries, removing tombstones.
    /// Returns the number of vectors in the rebuilt index.
    pub fn compact_ann(&self) -> Result<usize, String> {
        let dim = match self.ann.get() {
            Some(idx) => idx.dimensions(),
            None => return Ok(0),
        };
        let entries = self.live_embeddings(dim)?;
        let ann = self.ann.get().ok_or("ANN index disappeared")?;
        let count = ann.rebuild(entries.iter().map(|(id, emb)| (*id, emb.as_slice())))?;
        ann.reset_fragmentation();
        Ok(count)
    }

    /// Backfill the `embedding_model_id` column for entries that have an
    /// embedding but no model tag.  Returns the number of rows updated.
    pub fn backfill_embedding_model(&self, model_id: &str) -> Result<usize, String> {
        self.conn
            .execute(
                "UPDATE memories SET embedding_model_id = ?1
                 WHERE embedding IS NOT NULL AND embedding_model_id IS NULL",
                rusqlite::params![model_id],
            )
            .map_err(|e| e.to_string())
    }

    /// Rebuild the ANN index with a new quantization mode. Returns vector count.
    pub fn rebuild_ann_quantized(
        &self,
        quant: super::ann_index::EmbeddingQuantization,
    ) -> Result<usize, String> {
        // Detect current dimension from existing index or DB.
        let dim = if let Some(idx) = self.ann.get() {
            idx.dimensions()
        } else {
            let d = super::ann_index::detect_dimensions(&self.conn).unwrap_or(0);
            if d == 0 {
                return Ok(0);
            }
            d
        };

        let entries = self.live_embeddings(dim)?;

        // Create a new index with the requested quantization.
        let new_idx = if let Some(dir) = &self.data_dir {
            super::ann_index::AnnIndex::open_quantized(dir, dim, quant)?
        } else {
            super::ann_index::AnnIndex::new_quantized(dim, quant)?
        };
        let count =
            new_idx.rebuild(entries.iter().map(|(id, emb)| (*id, emb.as_slice())))?;

        // Replace the primary index. Because OnceCell doesn't support
        // overwrite, we can only do this if it wasn't set. If it was, the
        // caller should restart. In practice the rebuild will have already
        // persisted to disk and a restart picks up the new quantization.
        let _ = self.ann.set(new_idx);
        Ok(count)
    }

    /// Collect `(id, embedding)` pairs from long-tier entries.
    fn live_embeddings(&self, dim: usize) -> Result<Vec<(i64, Vec<f32>)>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, embedding FROM memories
                 WHERE tier = 'long' AND embedding IS NOT NULL",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                Ok((id, blob))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            let (id, blob) = r.map_err(|e| e.to_string())?;
            let emb = bytes_to_embedding(&blob);
            if emb.len() == dim {
                out.push((id, emb));
            }
        }
        Ok(out)
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
        tier: MemoryTier::from_str(
            &row.get::<_, String>(8)
                .unwrap_or_else(|_| "long".to_string()),
        ),
        decay_score: row.get::<_, f64>(9).unwrap_or(1.0),
        session_id: row.get(10).unwrap_or(None),
        parent_id: row.get(11).unwrap_or(None),
        token_count: row.get::<_, i64>(12).unwrap_or(0),
        source_url: row.get(13).unwrap_or(None),
        source_hash: row.get(14).unwrap_or(None),
        expires_at: row.get(15).unwrap_or(None),
        valid_to: row.get(16).unwrap_or(None),
        obsidian_path: row.get(17).unwrap_or(None),
        last_exported: row.get(18).unwrap_or(None),
        updated_at: row.get(19).unwrap_or(None),
        origin_device: row.get(20).unwrap_or(None),
        hlc_counter: row.get(21).unwrap_or(None),
        confidence: row.get::<_, f64>(22).unwrap_or(1.0),
    })
}

fn row_to_entry_with_embedding(row: &rusqlite::Row<'_>) -> SqlResult<MemoryEntry> {
    let blob: Option<Vec<u8>> = row.get(17)?;
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
        tier: MemoryTier::from_str(
            &row.get::<_, String>(8)
                .unwrap_or_else(|_| "long".to_string()),
        ),
        decay_score: row.get::<_, f64>(9).unwrap_or(1.0),
        session_id: row.get(10).unwrap_or(None),
        parent_id: row.get(11).unwrap_or(None),
        token_count: row.get::<_, i64>(12).unwrap_or(0),
        source_url: row.get(13).unwrap_or(None),
        source_hash: row.get(14).unwrap_or(None),
        expires_at: row.get(15).unwrap_or(None),
        valid_to: row.get(16).unwrap_or(None),
        obsidian_path: row.get(18).unwrap_or(None),
        last_exported: row.get(19).unwrap_or(None),
        updated_at: row.get(20).unwrap_or(None),
        origin_device: row.get(21).unwrap_or(None),
        hlc_counter: row.get(22).unwrap_or(None),
        confidence: row.get::<_, f64>(23).unwrap_or(1.0),
    })
}

/// Convert an f32 slice to little-endian bytes for BLOB storage.
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Convert little-endian bytes back to an f32 vec.
pub fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
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
    if denom < 1e-12 {
        0.0
    } else {
        (dot / denom) as f32
    }
}

/// Rough token estimation (~4 chars per token for English text).
fn estimate_tokens(text: &str) -> i64 {
    (text.len() as i64 + 3) / 4
}

// ── StorageBackend impl for MemoryStore (SQLite) ─────────────────────────────

use super::backend::{StorageBackend, StorageResult};

impl StorageBackend for MemoryStore {
    fn migrate(&self) -> StorageResult<()> {
        // Schema initialization runs automatically in MemoryStore::new / in_memory
        Ok(())
    }

    fn schema_version(&self) -> StorageResult<i64> {
        Ok(self.schema_version())
    }

    fn add(&self, m: NewMemory) -> StorageResult<MemoryEntry> {
        Ok(self.add(m)?)
    }

    fn add_to_tier(
        &self,
        m: NewMemory,
        tier: MemoryTier,
        session_id: Option<&str>,
    ) -> StorageResult<MemoryEntry> {
        Ok(self.add_to_tier(m, tier, session_id)?)
    }

    fn get_by_id(&self, id: i64) -> StorageResult<MemoryEntry> {
        Ok(self.get_by_id(id)?)
    }

    fn get_all(&self) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.get_all()?)
    }

    fn get_by_tier(&self, tier: &MemoryTier) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.get_by_tier(tier)?)
    }

    fn get_persistent(&self) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.get_persistent()?)
    }

    fn count(&self) -> StorageResult<i64> {
        Ok(self.count())
    }

    fn stats(&self) -> StorageResult<MemoryStats> {
        Ok(self.stats()?)
    }

    fn search(&self, query: &str) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.search(query)?)
    }

    fn relevant_for(&self, message: &str, limit: usize) -> StorageResult<Vec<String>> {
        Ok(self.relevant_for(message, limit))
    }

    fn find_by_source_url(&self, url: &str) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.find_by_source_url(url)?)
    }

    fn find_by_source_hash(&self, hash: &str) -> StorageResult<Option<MemoryEntry>> {
        Ok(self.find_by_source_hash(hash)?)
    }

    fn get_with_embeddings(&self) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.get_with_embeddings()?)
    }

    fn unembedded_ids(&self) -> StorageResult<Vec<(i64, String)>> {
        Ok(self.unembedded_ids()?)
    }

    fn set_embedding(&self, id: i64, embedding: &[f32]) -> StorageResult<()> {
        Ok(self.set_embedding(id, embedding)?)
    }

    fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.vector_search(query_embedding, limit)?)
    }

    fn find_duplicate(
        &self,
        query_embedding: &[f32],
        threshold: f32,
    ) -> StorageResult<Option<i64>> {
        Ok(self.find_duplicate(query_embedding, threshold)?)
    }

    fn hybrid_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.hybrid_search(query, query_embedding, limit)?)
    }

    fn hybrid_search_rrf(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.hybrid_search_rrf(query, query_embedding, limit)?)
    }

    fn update(&self, id: i64, upd: MemoryUpdate) -> StorageResult<MemoryEntry> {
        Ok(self.update(id, upd)?)
    }

    fn promote(&self, id: i64, new_tier: MemoryTier) -> StorageResult<()> {
        Ok(self.promote(id, new_tier)?)
    }

    fn delete(&self, id: i64) -> StorageResult<()> {
        Ok(self.delete(id)?)
    }

    fn delete_by_source_url(&self, url: &str) -> StorageResult<usize> {
        Ok(self.delete_by_source_url(url)?)
    }

    fn delete_expired(&self) -> StorageResult<usize> {
        Ok(self.delete_expired()?)
    }

    fn delete_all(&self) -> StorageResult<usize> {
        Ok(self.delete_all()?)
    }

    fn apply_decay(&self) -> StorageResult<usize> {
        Ok(self.apply_decay()?)
    }

    fn evict_short_term(&self, session_id: &str) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.evict_short_term(session_id)?)
    }

    fn gc_decayed(&self, threshold: f64) -> StorageResult<usize> {
        Ok(self.gc_decayed(threshold)?)
    }

    fn backend_name(&self) -> &'static str {
        "SQLite"
    }
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
        store
            .add(new_memory("User loves Python programming"))
            .unwrap();
        store
            .add(new_memory("User's favourite colour is blue"))
            .unwrap();
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
        assert!(
            (sim - 1.0).abs() < 1e-5,
            "identical vectors should have sim ≈ 1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-5,
            "orthogonal vectors should have sim ≈ 0.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim + 1.0).abs() < 1e-5,
            "opposite vectors should have sim ≈ -1.0, got {sim}"
        );
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
        assert_eq!(
            store.schema_version(),
            super::schema::CANONICAL_SCHEMA_VERSION
        );
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
        let entry = store
            .add_to_tier(
                new_memory("session fact"),
                MemoryTier::Working,
                Some("sess-1"),
            )
            .unwrap();
        assert_eq!(entry.tier, MemoryTier::Working);
        assert_eq!(entry.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn get_by_tier_filters_correctly() {
        let store = MemoryStore::in_memory();
        store
            .add_to_tier(new_memory("short"), MemoryTier::Short, Some("s1"))
            .unwrap();
        store
            .add_to_tier(new_memory("working"), MemoryTier::Working, Some("s1"))
            .unwrap();
        store.add(new_memory("long")).unwrap();

        assert_eq!(store.get_by_tier(&MemoryTier::Short).unwrap().len(), 1);
        assert_eq!(store.get_by_tier(&MemoryTier::Working).unwrap().len(), 1);
        assert_eq!(store.get_by_tier(&MemoryTier::Long).unwrap().len(), 1);
    }

    #[test]
    fn get_persistent_excludes_short_term() {
        let store = MemoryStore::in_memory();
        store
            .add_to_tier(new_memory("ephemeral"), MemoryTier::Short, Some("s1"))
            .unwrap();
        store
            .add_to_tier(new_memory("session ctx"), MemoryTier::Working, Some("s1"))
            .unwrap();
        store.add(new_memory("permanent")).unwrap();

        let persistent = store.get_persistent().unwrap();
        assert_eq!(persistent.len(), 2);
        assert!(persistent.iter().all(|e| e.tier != MemoryTier::Short));
    }

    #[test]
    fn promote_changes_tier() {
        let store = MemoryStore::in_memory();
        let entry = store
            .add_to_tier(new_memory("upgradeable"), MemoryTier::Working, Some("s1"))
            .unwrap();
        store.promote(entry.id, MemoryTier::Long).unwrap();
        let updated = store.get_by_id(entry.id).unwrap();
        assert_eq!(updated.tier, MemoryTier::Long);
    }

    #[test]
    fn evict_short_term_clears_session() {
        let store = MemoryStore::in_memory();
        store
            .add_to_tier(new_memory("msg1"), MemoryTier::Short, Some("sess-1"))
            .unwrap();
        store
            .add_to_tier(new_memory("msg2"), MemoryTier::Short, Some("sess-1"))
            .unwrap();
        store
            .add_to_tier(
                new_memory("other session"),
                MemoryTier::Short,
                Some("sess-2"),
            )
            .unwrap();

        let evicted = store.evict_short_term("sess-1").unwrap();
        assert_eq!(evicted.len(), 2);
        assert_eq!(store.get_by_tier(&MemoryTier::Short).unwrap().len(), 1);
    }

    #[test]
    fn stats_returns_tier_counts() {
        let store = MemoryStore::in_memory();
        store
            .add_to_tier(new_memory("s"), MemoryTier::Short, Some("s1"))
            .unwrap();
        store
            .add_to_tier(new_memory("w"), MemoryTier::Working, Some("s1"))
            .unwrap();
        store.add(new_memory("l1")).unwrap();
        store.add(new_memory("l2")).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.short, 1);
        assert_eq!(stats.working, 1);
        assert_eq!(stats.long, 2);
        assert!(stats.storage_bytes > 0);
    }

    #[test]
    fn get_all_within_storage_bytes_limits_memory_cache_not_storage() {
        let store = MemoryStore::in_memory();
        let first = store
            .add(NewMemory {
                content: "important cached row".repeat(20),
                tags: "test".into(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        store
            .add(NewMemory {
                content: "second persisted row".repeat(20),
                tags: "test".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        let capped = store.get_all_within_storage_bytes(1).unwrap();

        assert_eq!(capped.len(), 1);
        assert_eq!(capped[0].id, first.id);
        assert_eq!(store.count(), 2);
    }

    #[test]
    fn enforce_size_limit_prunes_low_utility_memories_first() {
        let store = MemoryStore::in_memory();
        let old = store
            .add(NewMemory {
                content: "old low utility memory with enough text to consume space".into(),
                tags: "test".into(),
                importance: 1,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        let keep = store
            .add(NewMemory {
                content: "important recent memory with enough text to consume space".into(),
                tags: "test".into(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        let before = store.active_storage_bytes().unwrap();

        let report = store.enforce_size_limit((before - 1) as u64).unwrap();

        assert_eq!(report.deleted, 1);
        assert!(store.get_by_id(old.id).is_err());
        assert!(store.get_by_id(keep.id).is_ok());
        assert!(report.after_bytes <= report.max_bytes);
    }

    #[test]
    fn hybrid_search_keyword_ranking() {
        let store = MemoryStore::in_memory();
        store
            .add(new_memory("Python programming language"))
            .unwrap();
        store.add(new_memory("JavaScript for web")).unwrap();
        store.add(new_memory("Rust systems programming")).unwrap();

        let results = store.hybrid_search("Python programming", None, 2).unwrap();
        assert_eq!(results.len(), 2);
        // Python entry should rank first (2 keyword hits vs 1 for Rust)
        assert!(results[0].content.contains("Python"));
    }

    #[test]
    fn hybrid_search_rrf_keyword_ranking() {
        let store = MemoryStore::in_memory();
        store
            .add(new_memory("Python programming language"))
            .unwrap();
        store.add(new_memory("JavaScript for web")).unwrap();
        store.add(new_memory("Rust systems programming")).unwrap();

        let results = store
            .hybrid_search_rrf("Python programming", None, 2)
            .unwrap();
        assert_eq!(results.len(), 2);
        // RRF may vary top-1 depending on freshness tie-breaking, but Python
        // (2 keyword hits) must still survive into the top-k.
        assert!(results.iter().any(|r| r.content.contains("Python")));
    }

    #[test]
    fn hybrid_search_rrf_zero_limit_returns_empty() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("anything")).unwrap();
        let results = store.hybrid_search_rrf("anything", None, 0).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn hybrid_search_rrf_empty_store_returns_empty() {
        let store = MemoryStore::in_memory();
        let results = store.hybrid_search_rrf("anything", None, 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn hybrid_search_rrf_no_matching_keyword_still_returns_freshness_ranked() {
        // When the query has no keyword hits and no embedding, freshness
        // ranking alone must still produce results so RAG never returns
        // an empty top-k just because the query is unusual.
        let store = MemoryStore::in_memory();
        store.add(new_memory("alpha")).unwrap();
        store.add(new_memory("beta")).unwrap();
        store.add(new_memory("gamma")).unwrap();

        let results = store
            .hybrid_search_rrf("xyzzy-nonexistent-token", None, 2)
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn hybrid_search_rrf_uses_vector_when_embedding_provided() {
        // Clear the process-wide search cache to prevent stale hits from
        // other tests (each test uses its own in-memory store, but the
        // cache key only encodes query/mode/limit, not the store instance).
        SEARCH_CACHE.clear();

        let store = MemoryStore::in_memory();
        let a = store.add(new_memory("alpha content")).unwrap();
        let b = store.add(new_memory("beta content")).unwrap();
        let c = store.add(new_memory("gamma content")).unwrap();

        // Hand-crafted unit vectors: query is most similar to `b`.
        let qe = vec![0.0_f32, 1.0, 0.0];
        store.set_embedding(a.id, &[1.0, 0.0, 0.0]).unwrap();
        store.set_embedding(b.id, &[0.0, 1.0, 0.0]).unwrap();
        store.set_embedding(c.id, &[0.0, 0.0, 1.0]).unwrap();

        // No query keyword overlap → vector + freshness drive the order;
        // `b` is the unambiguous vector winner, so it must lead the results.
        let results = store.hybrid_search_rrf("zzz", Some(&qe), 3).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, b.id);
    }

    #[test]
    fn hybrid_search_rrf_deterministic_across_runs() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("alpha")).unwrap();
        store.add(new_memory("beta")).unwrap();
        store.add(new_memory("gamma")).unwrap();

        let r1 = store.hybrid_search_rrf("xyz", None, 3).unwrap();
        let r2 = store.hybrid_search_rrf("xyz", None, 3).unwrap();
        let r3 = store.hybrid_search_rrf("xyz", None, 3).unwrap();
        let ids = |v: &[MemoryEntry]| v.iter().map(|e| e.id).collect::<Vec<_>>();
        assert_eq!(ids(&r1), ids(&r2));
        assert_eq!(ids(&r2), ids(&r3));
    }

    // ── Chunk 16.6c: query-intent–aware RRF ────────────────────────────

    #[test]
    fn hybrid_search_rrf_with_intent_unknown_matches_plain_rrf() {
        // For a query with no detectable intent, the intent-aware
        // variant must produce identical ids to plain RRF.
        let store = MemoryStore::in_memory();
        store.add(new_memory("alpha")).unwrap();
        store.add(new_memory("beta")).unwrap();
        store.add(new_memory("gamma")).unwrap();

        let plain = store.hybrid_search_rrf("alpha", None, 3).unwrap();
        let intent = store
            .hybrid_search_rrf_with_intent("alpha", None, 3)
            .unwrap();
        let ids = |v: &[MemoryEntry]| v.iter().map(|e| e.id).collect::<Vec<_>>();
        assert_eq!(ids(&plain), ids(&intent));
    }

    #[test]
    fn hybrid_search_rrf_with_intent_zero_limit_returns_empty() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("anything")).unwrap();
        let r = store
            .hybrid_search_rrf_with_intent("How to install?", None, 0)
            .unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn hybrid_search_rrf_with_intent_empty_store_returns_empty() {
        let store = MemoryStore::in_memory();
        let r = store
            .hybrid_search_rrf_with_intent("How to install?", None, 5)
            .unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn hybrid_search_rrf_with_intent_boosts_procedural_kind() {
        // Insert a procedural how-to memory and a generic semantic
        // factoid that share a common keyword. Plain RRF will rank by
        // hit count + freshness; with a procedural query intent, the
        // procedural entry must move to the top via the kind boost.
        let store = MemoryStore::in_memory();

        // Generic factoid (no procedural cues) — semantic kind by default.
        store
            .add(NewMemory {
                content: "Coffee originated in Ethiopia in the 9th century.".to_string(),
                tags: "coffee,history".to_string(),
                ..Default::default()
            })
            .unwrap();

        // Procedural how-to entry (procedural verbs trigger
        // cognitive_kind::classify → Procedural).
        store
            .add(NewMemory {
                content: "How to brew coffee: Step 1 grind beans. Step 2 \
                          heat water. Step 3 pour over filter. Procedure \
                          for pour-over coffee."
                    .to_string(),
                tags: "coffee,how-to".to_string(),
                ..Default::default()
            })
            .unwrap();

        let results = store
            .hybrid_search_rrf_with_intent("How do I brew coffee step by step?", None, 2)
            .unwrap();
        assert_eq!(results.len(), 2);
        // Procedural entry must lead the results once kind-boost is applied.
        assert!(
            results[0].content.contains("How to brew"),
            "procedural intent should boost the how-to entry to top: got {:?}",
            results[0].content
        );
    }

    #[test]
    fn hybrid_search_rrf_with_intent_deterministic() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("how to install ollama")).unwrap();
        store.add(new_memory("step by step setup guide")).unwrap();
        store.add(new_memory("ollama is a thing")).unwrap();

        let q = "How to install ollama?";
        let r1 = store.hybrid_search_rrf_with_intent(q, None, 3).unwrap();
        let r2 = store.hybrid_search_rrf_with_intent(q, None, 3).unwrap();
        let ids = |v: &[MemoryEntry]| v.iter().map(|e| e.id).collect::<Vec<_>>();
        assert_eq!(ids(&r1), ids(&r2));
    }

    #[test]
    fn gc_decayed_removes_low_importance() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 1,
                ..new_memory("forgettable")
            })
            .unwrap();
        // Manually set low decay
        store
            .conn
            .execute(
                "UPDATE memories SET decay_score = 0.005 WHERE id = ?1",
                params![e.id],
            )
            .unwrap();
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
        let entry = store
            .add(NewMemory {
                content: "Rule 14.3: 30-day deadline".to_string(),
                tags: "law".to_string(),
                importance: 5,
                memory_type: MemoryType::Fact,
                source_url: Some("https://example.com/rules".to_string()),
                source_hash: Some("abc123".to_string()),
                expires_at: None,
            })
            .unwrap();
        assert_eq!(
            entry.source_url.as_deref(),
            Some("https://example.com/rules")
        );
        assert_eq!(entry.source_hash.as_deref(), Some("abc123"));
        assert!(entry.expires_at.is_none());
    }

    #[test]
    fn find_by_source_hash_returns_match() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                source_hash: Some("hash-001".to_string()),
                ..new_memory("sourced fact")
            })
            .unwrap();
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
        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                source_hash: Some("h1".to_string()),
                ..new_memory("chunk 1")
            })
            .unwrap();
        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                source_hash: Some("h2".to_string()),
                ..new_memory("chunk 2")
            })
            .unwrap();
        store.add(new_memory("unrelated")).unwrap();

        let results = store.find_by_source_url(url).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn delete_by_source_url_removes_all() {
        let store = MemoryStore::in_memory();
        let url = "https://example.com/stale";
        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                ..new_memory("old chunk 1")
            })
            .unwrap();
        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                ..new_memory("old chunk 2")
            })
            .unwrap();
        store.add(new_memory("keep me")).unwrap();

        let removed = store.delete_by_source_url(url).unwrap();
        assert_eq!(removed, 2);
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn reingest_skip_when_hash_unchanged() {
        let store = MemoryStore::in_memory();
        let hash = "sha256-unchanged";
        store
            .add(NewMemory {
                source_hash: Some(hash.to_string()),
                source_url: Some("https://example.com/doc".to_string()),
                ..new_memory("existing content")
            })
            .unwrap();

        // Simulate re-ingest: find_by_source_hash returns Some → skip
        let existing = store.find_by_source_hash(hash).unwrap();
        assert!(existing.is_some());
    }

    #[test]
    fn reingest_replaces_when_hash_changed() {
        let store = MemoryStore::in_memory();
        let url = "https://example.com/rule";
        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                source_hash: Some("old-hash".to_string()),
                ..new_memory("old version of rule")
            })
            .unwrap();
        assert_eq!(store.count(), 1);

        // Hash changed → delete old entries by URL, then insert new
        let _ = store.delete_by_source_url(url).unwrap();
        assert_eq!(store.count(), 0);

        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                source_hash: Some("new-hash".to_string()),
                ..new_memory("updated version of rule")
            })
            .unwrap();
        assert_eq!(store.count(), 1);

        let found = store.find_by_source_hash("new-hash").unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().content.contains("updated"));
    }

    #[test]
    fn delete_expired_removes_past_entries() {
        let store = MemoryStore::in_memory();
        // Insert with an already-expired timestamp
        store
            .add(NewMemory {
                expires_at: Some(1000), // epoch ms, way in the past
                ..new_memory("ephemeral")
            })
            .unwrap();
        store.add(new_memory("permanent")).unwrap();

        let removed = store.delete_expired().unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.count(), 1);
    }

    // ── StorageBackend trait tests ───────────────────────────────────────

    #[test]
    fn storage_backend_sqlite_round_trip() {
        let store = MemoryStore::in_memory();
        let backend: &dyn StorageBackend = &store;

        assert_eq!(backend.backend_name(), "SQLite");
        assert!(!backend.supports_native_vector_search());

        // Add via trait
        let entry = backend.add(new_memory("trait test")).unwrap();
        assert_eq!(entry.content, "trait test");

        // Read via trait
        let fetched = backend.get_by_id(entry.id).unwrap();
        assert_eq!(fetched.content, "trait test");

        // Count via trait
        assert_eq!(backend.count().unwrap(), 1);

        // Search via trait
        let results = backend.search("trait").unwrap();
        assert_eq!(results.len(), 1);

        // Delete via trait
        backend.delete(entry.id).unwrap();
        assert_eq!(backend.count().unwrap(), 0);
    }

    #[test]
    fn storage_backend_stats_via_trait() {
        let store = MemoryStore::in_memory();
        let backend: &dyn StorageBackend = &store;

        backend.add(new_memory("one")).unwrap();
        backend
            .add_to_tier(new_memory("two"), MemoryTier::Short, Some("sess"))
            .unwrap();

        let stats = backend.stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.short, 1);
        assert_eq!(stats.long, 1);
    }

    // ─── Chunk 16.1 — relevance threshold ─────────────────────────────

    #[test]
    fn hybrid_search_with_threshold_zero_matches_legacy_top_k() {
        // Threshold = 0.0 must reproduce the legacy hybrid_search top-k
        // (same ids, same order). Critical back-compat invariant — every
        // existing call site must keep working when the user hasn't
        // tuned the threshold.
        let store = MemoryStore::in_memory();
        store
            .add(new_memory("Python programming language"))
            .unwrap();
        store.add(new_memory("JavaScript for web")).unwrap();
        store.add(new_memory("Rust systems programming")).unwrap();

        let legacy = store.hybrid_search("Python programming", None, 2).unwrap();
        let with_t = store
            .hybrid_search_with_threshold("Python programming", None, 2, 0.0)
            .unwrap();
        assert_eq!(legacy.len(), with_t.len());
        for (a, b) in legacy.iter().zip(with_t.iter()) {
            assert_eq!(a.id, b.id);
        }
    }

    #[test]
    fn hybrid_search_with_threshold_filters_below_score() {
        // High threshold should drop weakly-matching rows. Both seeded
        // memories match nothing in the query "totally unrelated topic",
        // so all keyword scores collapse to 0 and only the freshness +
        // tier + importance + decay components remain — a small number.
        // 0.95 is well above any realistic combined score, so the result
        // must be empty.
        let store = MemoryStore::in_memory();
        store.add(new_memory("alpha")).unwrap();
        store.add(new_memory("beta")).unwrap();
        let r = store
            .hybrid_search_with_threshold("totally unrelated topic", None, 5, 0.95)
            .unwrap();
        assert!(r.is_empty(), "got {} hits", r.len());
    }

    #[test]
    fn hybrid_search_with_threshold_keeps_strong_matches() {
        // Strong keyword + freshness combo on a low threshold must keep
        // the matching row.
        let store = MemoryStore::in_memory();
        let e = store
            .add(new_memory("Python programming language"))
            .unwrap();
        let r = store
            .hybrid_search_with_threshold("Python programming", None, 5, 0.10)
            .unwrap();
        assert!(!r.is_empty());
        assert!(r.iter().any(|m| m.id == e.id));
    }

    #[test]
    fn hybrid_search_with_threshold_does_not_increment_access_for_filtered() {
        // Below-threshold rows must NOT count as accesses — keeps the
        // decay signal honest. We use a threshold above any realistic
        // score (the legacy hybrid score caps near 1.0; a query with no
        // keyword overlap hits ~0.3 on freshness alone) so every row is
        // filtered out, and assert no row's access_count was bumped.
        let store = MemoryStore::in_memory();
        let a = store.add(new_memory("alpha")).unwrap();
        let b = store.add(new_memory("beta")).unwrap();
        let r = store
            .hybrid_search_with_threshold("totally unrelated topic", None, 5, 0.95)
            .unwrap();
        assert!(r.is_empty(), "high threshold should filter all rows");

        let a_after = store.get_by_id(a.id).unwrap();
        let b_after = store.get_by_id(b.id).unwrap();
        assert_eq!(
            a_after.access_count, 0,
            "filtered row a must NOT be touched"
        );
        assert_eq!(
            b_after.access_count, 0,
            "filtered row b must NOT be touched"
        );
    }

    #[test]
    fn hybrid_search_with_threshold_respects_limit() {
        // Many strong matches + threshold = 0.0 — the `limit` cap still applies.
        let store = MemoryStore::in_memory();
        for i in 0..10 {
            store
                .add(new_memory(&format!("Python programming language {i}")))
                .unwrap();
        }
        let r = store
            .hybrid_search_with_threshold("Python programming", None, 3, 0.0)
            .unwrap();
        assert_eq!(r.len(), 3);
    }

    // ------------------------------------------------------------------
    // Chunk 17.1 — auto_promote_to_long
    // ------------------------------------------------------------------

    /// Helper: force a working-tier row's access_count + last_accessed
    /// to a known state. Tests only.
    fn force_access(store: &MemoryStore, id: i64, count: i64, last_accessed_ms: i64) {
        store
            .conn()
            .execute(
                "UPDATE memories SET access_count = ?1, last_accessed = ?2 WHERE id = ?3",
                params![count, last_accessed_ms, id],
            )
            .unwrap();
    }

    #[test]
    fn auto_promote_promotes_when_both_thresholds_met() {
        let store = MemoryStore::in_memory();
        let e = store
            .add_to_tier(new_memory("hot working entry"), MemoryTier::Working, None)
            .unwrap();
        force_access(&store, e.id, 5, now_ms());

        let promoted = store.auto_promote_to_long(5, 7).unwrap();
        assert_eq!(promoted, vec![e.id]);
        assert_eq!(store.get_by_id(e.id).unwrap().tier, MemoryTier::Long);
    }

    #[test]
    fn auto_promote_skips_when_access_count_below_threshold() {
        let store = MemoryStore::in_memory();
        let e = store
            .add_to_tier(new_memory("cold working entry"), MemoryTier::Working, None)
            .unwrap();
        force_access(&store, e.id, 4, now_ms()); // one short of the threshold

        let promoted = store.auto_promote_to_long(5, 7).unwrap();
        assert!(
            promoted.is_empty(),
            "below-threshold row must not be promoted"
        );
        assert_eq!(store.get_by_id(e.id).unwrap().tier, MemoryTier::Working);
    }

    #[test]
    fn auto_promote_skips_when_outside_recency_window() {
        let store = MemoryStore::in_memory();
        let e = store
            .add_to_tier(new_memory("stale working entry"), MemoryTier::Working, None)
            .unwrap();
        // last_accessed is well outside a 7-day window
        force_access(&store, e.id, 99, now_ms() - 30 * 86_400_000);

        let promoted = store.auto_promote_to_long(5, 7).unwrap();
        assert!(promoted.is_empty(), "stale row must not be promoted");
        assert_eq!(store.get_by_id(e.id).unwrap().tier, MemoryTier::Working);
    }

    #[test]
    fn auto_promote_ignores_long_and_short_tiers() {
        let store = MemoryStore::in_memory();
        let l = store
            .add_to_tier(new_memory("already long"), MemoryTier::Long, None)
            .unwrap();
        let s = store
            .add_to_tier(new_memory("short term"), MemoryTier::Short, Some("sess"))
            .unwrap();
        force_access(&store, l.id, 100, now_ms());
        force_access(&store, s.id, 100, now_ms());

        let promoted = store.auto_promote_to_long(5, 7).unwrap();
        assert!(
            promoted.is_empty(),
            "non-working tiers must not be promoted"
        );
    }

    #[test]
    fn auto_promote_is_idempotent() {
        let store = MemoryStore::in_memory();
        let e = store
            .add_to_tier(new_memory("hot"), MemoryTier::Working, None)
            .unwrap();
        force_access(&store, e.id, 10, now_ms());

        let first = store.auto_promote_to_long(5, 7).unwrap();
        let second = store.auto_promote_to_long(5, 7).unwrap();
        assert_eq!(first, vec![e.id]);
        assert!(
            second.is_empty(),
            "second run is a no-op once promoted to long"
        );
    }

    #[test]
    fn auto_promote_skips_rows_with_null_last_accessed() {
        // A working entry that was inserted but never accessed has
        // last_accessed = NULL — must not be promoted regardless of count.
        let store = MemoryStore::in_memory();
        let e = store
            .add_to_tier(new_memory("never accessed"), MemoryTier::Working, None)
            .unwrap();
        store
            .conn()
            .execute(
                "UPDATE memories SET access_count = 50, last_accessed = NULL WHERE id = ?1",
                params![e.id],
            )
            .unwrap();

        let promoted = store.auto_promote_to_long(5, 7).unwrap();
        assert!(
            promoted.is_empty(),
            "NULL last_accessed must be treated as not-recent"
        );
        assert_eq!(store.get_by_id(e.id).unwrap().tier, MemoryTier::Working);
    }

    // ------------------------------------------------------------------
    // Chunk 18.2 — category-aware decay (integration with apply_decay)
    // ------------------------------------------------------------------

    fn add_long_with_tags(store: &MemoryStore, content: &str, tags: &str) -> i64 {
        let m = NewMemory {
            content: content.to_string(),
            tags: tags.to_string(),
            importance: 3,
            memory_type: MemoryType::Fact,
            source_url: None,
            source_hash: None,
            expires_at: None,
        };
        let e = store.add(m).unwrap();
        // Force last_accessed to ~30 days ago so apply_decay actually moves the score.
        let thirty_days_ago = now_ms() - 30 * 86_400_000;
        store
            .conn()
            .execute(
                "UPDATE memories SET last_accessed = ?1, decay_score = 1.0 WHERE id = ?2",
                params![thirty_days_ago, e.id],
            )
            .unwrap();
        e.id
    }

    #[test]
    fn apply_decay_personal_decays_slower_than_tool() {
        let store = MemoryStore::in_memory();
        let personal = add_long_with_tags(&store, "user loves pho", "personal:loves_pho");
        let tool = add_long_with_tags(&store, "bun --hot flag", "tool:bun");
        store.apply_decay().unwrap();

        let p = store.get_by_id(personal).unwrap();
        let t = store.get_by_id(tool).unwrap();
        assert!(
            p.decay_score > t.decay_score,
            "personal:* (mult 0.5) must decay slower than tool:* (mult 1.5); got personal={}, tool={}",
            p.decay_score, t.decay_score
        );
    }

    #[test]
    fn apply_decay_baseline_for_legacy_or_non_conforming_tags() {
        // Two entries with default-multiplier-equivalent tags should land at
        // the same decay_score (within float tolerance).
        let store = MemoryStore::in_memory();
        let legacy = add_long_with_tags(&store, "legacy", "fact");
        let project = add_long_with_tags(&store, "project x", "project:x");
        store.apply_decay().unwrap();

        let l = store.get_by_id(legacy).unwrap();
        let p = store.get_by_id(project).unwrap();
        assert!(
            (l.decay_score - p.decay_score).abs() < 1e-6,
            "legacy tag and project:* (both mult 1.0) must decay identically; got legacy={}, project={}",
            l.decay_score, p.decay_score
        );
    }

    // ── Importance auto-adjustment (chunk 17.4) ────────────────────────

    /// Helper: simulate N accesses on a memory by directly bumping access_count.
    fn set_access_count(store: &MemoryStore, id: i64, count: i64) {
        store
            .conn
            .execute(
                "UPDATE memories SET access_count = ?1, last_accessed = ?2 WHERE id = ?3",
                params![count, now_ms(), id],
            )
            .unwrap();
    }

    #[test]
    fn adjust_boosts_hot_entries() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 3,
                ..new_memory("hot")
            })
            .unwrap();
        set_access_count(&store, e.id, 10);
        let (boosted, demoted) = store.adjust_importance_by_access(10, 30).unwrap();
        assert_eq!(boosted, 1);
        assert_eq!(demoted, 0);
        assert_eq!(store.get_by_id(e.id).unwrap().importance, 4);
    }

    #[test]
    fn adjust_caps_at_5() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 5,
                ..new_memory("maxed")
            })
            .unwrap();
        set_access_count(&store, e.id, 20);
        let (boosted, _) = store.adjust_importance_by_access(10, 30).unwrap();
        // Already at max → no boost
        assert_eq!(boosted, 0);
        assert_eq!(store.get_by_id(e.id).unwrap().importance, 5);
    }

    #[test]
    fn adjust_demotes_cold_entries() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 3,
                ..new_memory("cold")
            })
            .unwrap();
        // access_count stays 0, last_accessed is NULL → cold
        let (_, demoted) = store.adjust_importance_by_access(10, 30).unwrap();
        assert_eq!(demoted, 1);
        assert_eq!(store.get_by_id(e.id).unwrap().importance, 2);
    }

    #[test]
    fn adjust_floors_at_1() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 1,
                ..new_memory("min")
            })
            .unwrap();
        let (_, demoted) = store.adjust_importance_by_access(10, 30).unwrap();
        // Already at min → no demote
        assert_eq!(demoted, 0);
        assert_eq!(store.get_by_id(e.id).unwrap().importance, 1);
    }

    #[test]
    fn adjust_resets_access_count_after_boost() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("reset")).unwrap();
        set_access_count(&store, e.id, 15);
        store.adjust_importance_by_access(10, 30).unwrap();
        let updated = store.get_by_id(e.id).unwrap();
        assert_eq!(
            updated.access_count, 0,
            "access_count should reset after boost"
        );
    }

    #[test]
    fn adjust_leaves_middling_entries_alone() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("middling")).unwrap();
        // access_count = 5 (below hot_threshold 10), recently accessed → neither hot nor cold
        set_access_count(&store, e.id, 5);
        let (boosted, demoted) = store.adjust_importance_by_access(10, 30).unwrap();
        assert_eq!(boosted, 0);
        assert_eq!(demoted, 0);
        assert_eq!(store.get_by_id(e.id).unwrap().importance, 3);
    }

    #[test]
    fn adjust_mixed_hot_and_cold() {
        let store = MemoryStore::in_memory();
        let hot = store
            .add(NewMemory {
                importance: 2,
                ..new_memory("hot one")
            })
            .unwrap();
        let cold = store
            .add(NewMemory {
                importance: 4,
                ..new_memory("cold one")
            })
            .unwrap();
        set_access_count(&store, hot.id, 12);
        // cold stays at access_count=0, last_accessed=NULL

        let (boosted, demoted) = store.adjust_importance_by_access(10, 30).unwrap();
        assert_eq!(boosted, 1);
        assert_eq!(demoted, 1);
        assert_eq!(store.get_by_id(hot.id).unwrap().importance, 3);
        assert_eq!(store.get_by_id(cold.id).unwrap().importance, 3);
    }

    #[test]
    fn adjust_creates_version_audit_trail() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 3,
                ..new_memory("audited")
            })
            .unwrap();
        set_access_count(&store, e.id, 10);
        store.adjust_importance_by_access(10, 30).unwrap();

        let history = crate::memory::versioning::get_history(&store.conn, e.id).unwrap();
        assert_eq!(history.len(), 1, "boost should create one version snapshot");
        assert_eq!(
            history[0].importance, 3,
            "snapshot should capture pre-boost value"
        );
    }

    // ── Reinforcement provenance tests (43.4) ─────────────────────────────

    #[test]
    fn record_reinforcement_round_trip() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("reinforced fact")).unwrap();

        store
            .record_reinforcement(e.id, "sess-a", 0)
            .unwrap();

        let recs = store.get_reinforcements(e.id, 10).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].memory_id, e.id);
        assert_eq!(recs[0].session_id, "sess-a");
        assert_eq!(recs[0].message_index, 0);
    }

    #[test]
    fn record_reinforcement_idempotent_on_pk() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("idempotent test")).unwrap();

        // Same (memory_id, session_id, message_index) inserted twice
        store.record_reinforcement(e.id, "sess-b", 1).unwrap();
        store.record_reinforcement(e.id, "sess-b", 1).unwrap();

        let recs = store.get_reinforcements(e.id, 10).unwrap();
        assert_eq!(recs.len(), 1, "duplicate PK should be ignored");
    }

    #[test]
    fn get_reinforcements_respects_limit() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("limited")).unwrap();

        for i in 0..5 {
            store.record_reinforcement(e.id, &format!("s{i}"), 0).unwrap();
        }

        let recs = store.get_reinforcements(e.id, 3).unwrap();
        assert_eq!(recs.len(), 3);
    }

    #[test]
    fn get_reinforcements_empty_when_none() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("no reinforcements")).unwrap();

        let recs = store.get_reinforcements(e.id, 10).unwrap();
        assert!(recs.is_empty());
    }
}
