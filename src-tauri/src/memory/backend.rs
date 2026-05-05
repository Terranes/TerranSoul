//! Storage backend trait — abstracts the database layer.
//!
//! TerranSoul supports four storage backends:
//! - **SQLite** (default, local mode) — single-file, zero-config, offline-first
//! - **PostgreSQL** (distributed mode) — multi-device sync, server deployment
//! - **SQL Server** (enterprise mode) — corporate/Azure integration
//! - **CassandraDB** (scale-out mode) — high-write throughput, eventual consistency
//!
//! The `StorageBackend` trait defines the contract that every backend must implement.
//! Backend selection happens at startup via `StorageConfig`.

use serde::{Deserialize, Serialize};

use super::store::{MemoryEntry, MemoryStats, MemoryTier, MemoryUpdate, NewMemory};

/// Errors from any storage backend.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[cfg(feature = "postgres")]
    #[error("PostgreSQL error: {0}")]
    Postgres(String),

    #[cfg(feature = "mssql")]
    #[error("SQL Server error: {0}")]
    Mssql(String),

    #[cfg(feature = "cassandra")]
    #[error("Cassandra error: {0}")]
    Cassandra(String),

    #[error("Backend not available: {0}")]
    Unavailable(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("{0}")]
    Other(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

/// Configuration for selecting and connecting to a storage backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum StorageConfig {
    /// Local SQLite database (default).
    Sqlite {
        /// Directory containing `memory.db`. If None, uses app data dir.
        data_dir: Option<String>,
    },

    /// PostgreSQL for distributed/multi-device deployments.
    #[cfg(feature = "postgres")]
    Postgres {
        /// Connection string: `postgresql://user:pass@host:5432/dbname`
        connection_string: String,
        /// Max connections in the pool (default: 10).
        max_connections: Option<u32>,
        /// Enable SSL (default: true).
        ssl: Option<bool>,
    },

    /// SQL Server for enterprise/Azure environments.
    #[cfg(feature = "mssql")]
    SqlServer {
        /// Connection string: `Server=host;Database=db;User Id=user;Password=pass;`
        connection_string: String,
        /// Max connections in the pool (default: 10).
        max_connections: Option<u32>,
    },

    /// CassandraDB for high-write-throughput distributed deployments.
    #[cfg(feature = "cassandra")]
    Cassandra {
        /// Contact points: `["host1:9042", "host2:9042"]`
        contact_points: Vec<String>,
        /// Keyspace name (default: `terransoul`).
        keyspace: Option<String>,
        /// Replication factor (default: 3).
        replication_factor: Option<u32>,
        /// Datacenter name for `NetworkTopologyStrategy`.
        datacenter: Option<String>,
    },
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig::Sqlite { data_dir: None }
    }
}

/// The core storage abstraction. Every database backend implements this trait.
///
/// Methods are synchronous by design — the caller is responsible for running
/// distributed backends in a blocking task or behind an async adapter.
/// This matches the existing `Mutex<MemoryStore>` pattern in `AppState`.
///
/// Only `Send` is required (not `Sync`) because the backend is always held
/// behind a `Mutex`, which provides `Sync` at the outer level.
pub trait StorageBackend: Send {
    // ── Schema ───────────────────────────────────────────────────────────
    /// Run migrations up to the latest version.
    fn migrate(&self) -> StorageResult<()>;
    /// Current schema version.
    fn schema_version(&self) -> StorageResult<i64>;

    // ── Create ───────────────────────────────────────────────────────────
    /// Insert a new memory into the long-term tier.
    fn add(&self, m: NewMemory) -> StorageResult<MemoryEntry>;
    /// Insert a new memory into a specific tier with an optional session ID.
    fn add_to_tier(
        &self,
        m: NewMemory,
        tier: MemoryTier,
        session_id: Option<&str>,
    ) -> StorageResult<MemoryEntry>;

    // ── Read ─────────────────────────────────────────────────────────────
    fn get_by_id(&self, id: i64) -> StorageResult<MemoryEntry>;
    fn get_all(&self) -> StorageResult<Vec<MemoryEntry>>;
    fn get_by_tier(&self, tier: &MemoryTier) -> StorageResult<Vec<MemoryEntry>>;
    fn get_persistent(&self) -> StorageResult<Vec<MemoryEntry>>;
    fn count(&self) -> StorageResult<i64>;
    fn stats(&self) -> StorageResult<MemoryStats>;

    // ── Search ───────────────────────────────────────────────────────────
    /// Keyword search (SQL LIKE on content + tags).
    fn search(&self, query: &str) -> StorageResult<Vec<MemoryEntry>>;
    /// Keyword-scored relevant content strings.
    fn relevant_for(&self, message: &str, limit: usize) -> StorageResult<Vec<String>>;
    /// Find by source URL.
    fn find_by_source_url(&self, url: &str) -> StorageResult<Vec<MemoryEntry>>;
    /// Find by source hash (SHA-256).
    fn find_by_source_hash(&self, hash: &str) -> StorageResult<Option<MemoryEntry>>;

    // ── Vector search ────────────────────────────────────────────────────
    /// Entries with non-null embeddings (includes embedding blob).
    fn get_with_embeddings(&self) -> StorageResult<Vec<MemoryEntry>>;
    /// IDs of entries missing embeddings.
    fn unembedded_ids(&self) -> StorageResult<Vec<(i64, String)>>;
    /// Store an embedding vector for a memory.
    fn set_embedding(&self, id: i64, embedding: &[f32]) -> StorageResult<()>;
    /// Pure cosine-similarity vector search.
    fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>>;
    /// Find near-duplicate by cosine threshold.
    fn find_duplicate(&self, query_embedding: &[f32], threshold: f32)
        -> StorageResult<Option<i64>>;
    /// 6-signal hybrid search (vector + keyword + recency + importance + decay + tier).
    fn hybrid_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>>;

    /// Hybrid search using **Reciprocal Rank Fusion** over independent
    /// vector / keyword / freshness retrievers. Robust to score-scale
    /// mismatch between retrievers (Cormack et al., SIGIR 2009).
    ///
    /// Default implementation falls back to [`Self::hybrid_search`] so
    /// non-default backends keep working until they implement RRF natively.
    /// See `docs/brain-advanced-design.md` §16 Phase 6 / §19.2 row 2.
    fn hybrid_search_rrf(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        self.hybrid_search(query, query_embedding, limit)
    }

    // ── Update ───────────────────────────────────────────────────────────
    fn update(&self, id: i64, upd: MemoryUpdate) -> StorageResult<MemoryEntry>;
    fn promote(&self, id: i64, new_tier: MemoryTier) -> StorageResult<()>;

    // ── Delete ───────────────────────────────────────────────────────────
    fn delete(&self, id: i64) -> StorageResult<()>;
    fn delete_by_source_url(&self, url: &str) -> StorageResult<usize>;
    fn delete_expired(&self) -> StorageResult<usize>;
    /// Delete **all** memories, edges, and conflicts. Returns deleted memory count.
    fn delete_all(&self) -> StorageResult<usize>;

    // ── Lifecycle ────────────────────────────────────────────────────────
    /// Apply time-based decay to long-term memories.
    fn apply_decay(&self) -> StorageResult<usize>;
    /// Evict short-term memories for a session, returning evicted entries.
    fn evict_short_term(&self, session_id: &str) -> StorageResult<Vec<MemoryEntry>>;
    /// Garbage-collect decayed, low-importance memories.
    fn gc_decayed(&self, threshold: f64) -> StorageResult<usize>;

    // ── Backend info ─────────────────────────────────────────────────────
    /// Human-readable backend name (e.g. "SQLite", "PostgreSQL").
    fn backend_name(&self) -> &'static str;
    /// Whether this backend supports server-side vector operations.
    /// If false, vector search is done in-process.
    fn supports_native_vector_search(&self) -> bool {
        false
    }
}
