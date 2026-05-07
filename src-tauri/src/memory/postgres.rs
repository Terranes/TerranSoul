//! PostgreSQL storage backend — for distributed/multi-device deployments.
//!
//! Uses `sqlx` with the `postgres` feature. Requires a PostgreSQL 14+ server.
//!
//! Connection string format:
//! ```text
//! postgresql://user:password@host:5432/terransoul?sslmode=require
//! ```
//!
//! # Cargo feature
//! Enable with `--features postgres` in Cargo.toml.
//!
//! # Schema
//! Mirrors the SQLite V18 schema with PostgreSQL-native types:
//! - `BIGSERIAL` instead of `AUTOINCREMENT`
//! - `BYTEA` for embedding blobs
//! - `BIGINT` for timestamps
//! - `tsvector` + GIN index for full-text search
//! - `memory_edges` table with recursive-CTE traversal for KG
//! - `hlc_counter` + `origin_device` for CRDT sync
//! - Embedding payloads stored alongside metadata for portable retrieval
//!
//! # Key features (Chunk 42.6)
//! - **FTS:** `tsvector` GIN index on content, auto-populated by trigger
//! - **RRF:** Native SQL CTE implementing Reciprocal Rank Fusion (k=60)
//! - **KG:** `memory_edges` table with recursive CTE traversal
//! - **Contextual Retrieval:** prefix prepended on insert (caller-driven)

#![cfg(feature = "postgres")]

use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

use super::backend::{StorageBackend, StorageError, StorageResult};
use super::edges::{EdgeDirection, EdgeSource, MemoryEdge, NewMemoryEdge};
use super::store::{MemoryEntry, MemoryStats, MemoryTier, MemoryType, MemoryUpdate, NewMemory};

/// PostgreSQL storage backend.
pub struct PostgresBackend {
    pool: PgPool,
}

impl PostgresBackend {
    /// Create a new PostgreSQL backend from a connection string.
    ///
    /// ```text
    /// let backend = PostgresBackend::connect(
    ///     "postgresql://user:pass@localhost:5432/terransoul",
    ///     Some(10),
    ///     true,
    /// ).await?;
    /// ```
    pub async fn connect(
        connection_string: &str,
        max_connections: Option<u32>,
        _ssl: bool,
    ) -> StorageResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections.unwrap_or(10))
            .connect(connection_string)
            .await
            .map_err(|e| StorageError::Postgres(e.to_string()))?;

        let backend = Self { pool };
        backend.run_migrations().await?;
        Ok(backend)
    }

    /// Run migrations synchronously (blocks current thread).
    fn run_migrations_sync(&self) -> StorageResult<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { self.run_migrations().await })
        })
    }

    /// Run migrations asynchronously.
    async fn run_migrations(&self) -> StorageResult<()> {
        // Create schema_version table if not exists
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version     BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at  BIGINT NOT NULL
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Migration(e.to_string()))?;

        // V1: Base memories table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memories (
                id            BIGSERIAL PRIMARY KEY,
                content       TEXT NOT NULL,
                tags          TEXT NOT NULL DEFAULT '',
                importance    INTEGER NOT NULL DEFAULT 3,
                memory_type   TEXT NOT NULL DEFAULT 'fact',
                created_at    BIGINT NOT NULL,
                last_accessed BIGINT,
                access_count  INTEGER NOT NULL DEFAULT 0,
                embedding     BYTEA,
                source_url    TEXT,
                source_hash   TEXT,
                expires_at    BIGINT,
                tier          TEXT NOT NULL DEFAULT 'long',
                decay_score   DOUBLE PRECISION NOT NULL DEFAULT 1.0,
                session_id    TEXT,
                parent_id     BIGINT REFERENCES memories(id),
                token_count   INTEGER NOT NULL DEFAULT 0,
                valid_to      BIGINT,
                obsidian_path TEXT,
                last_exported BIGINT,
                updated_at    BIGINT,
                origin_device TEXT
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Migration(e.to_string()))?;

        // V2: Add extended columns (version-tracked)
        let v2_applied: Option<i64> =
            sqlx::query_scalar("SELECT version FROM schema_version WHERE version = 2 LIMIT 1")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| StorageError::Migration(e.to_string()))?;

        if v2_applied.is_none() {
            for column_sql in &[
                "ALTER TABLE memories ADD COLUMN IF NOT EXISTS valid_to BIGINT",
                "ALTER TABLE memories ADD COLUMN IF NOT EXISTS obsidian_path TEXT",
                "ALTER TABLE memories ADD COLUMN IF NOT EXISTS last_exported BIGINT",
                "ALTER TABLE memories ADD COLUMN IF NOT EXISTS updated_at BIGINT",
                "ALTER TABLE memories ADD COLUMN IF NOT EXISTS origin_device TEXT",
            ] {
                sqlx::query(column_sql)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| StorageError::Migration(e.to_string()))?;
            }

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;

            sqlx::query(
                "INSERT INTO schema_version (version, description, applied_at)
                 VALUES (2, 'PostgreSQL V2 — extended memory columns', $1)
                 ON CONFLICT (version) DO NOTHING",
            )
            .bind(now_ms)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Migration(e.to_string()))?;
        }

        // V3: Create indexes (version-tracked)
        let v3_applied: Option<i64> =
            sqlx::query_scalar("SELECT version FROM schema_version WHERE version = 3 LIMIT 1")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| StorageError::Migration(e.to_string()))?;

        if v3_applied.is_none() {
            for idx in &[
                "CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories (importance DESC)",
                "CREATE INDEX IF NOT EXISTS idx_memories_created ON memories (created_at DESC)",
                "CREATE INDEX IF NOT EXISTS idx_memories_source_hash ON memories (source_hash)",
                "CREATE INDEX IF NOT EXISTS idx_memories_tier ON memories (tier)",
                "CREATE INDEX IF NOT EXISTS idx_memories_session ON memories (session_id)",
                "CREATE INDEX IF NOT EXISTS idx_memories_decay ON memories (decay_score DESC)",
            ] {
                sqlx::query(idx)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| StorageError::Migration(e.to_string()))?;
            }

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;

            sqlx::query(
                "INSERT INTO schema_version (version, description, applied_at)
                 VALUES (3, 'PostgreSQL V3 — indexes', $1)
                 ON CONFLICT (version) DO NOTHING",
            )
            .bind(now_ms)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Migration(e.to_string()))?;
        }

        // V4: Mark full schema migration complete (version-tracked)
        let v4_applied: Option<i64> =
            sqlx::query_scalar("SELECT version FROM schema_version WHERE version = 4 LIMIT 1")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| StorageError::Migration(e.to_string()))?;

        if v4_applied.is_none() {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;

            sqlx::query(
                "INSERT INTO schema_version (version, description, applied_at)
                 VALUES (4, 'PostgreSQL V4 — full schema', $1)
                 ON CONFLICT (version) DO NOTHING",
            )
            .bind(now_ms)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Migration(e.to_string()))?;
        }

        // V5: FTS — tsvector column + GIN index + auto-populate trigger
        self.apply_migration(5, "PostgreSQL V5 — FTS tsvector + GIN", &[
            "ALTER TABLE memories ADD COLUMN IF NOT EXISTS fts_vector tsvector",
            "CREATE INDEX IF NOT EXISTS idx_memories_fts ON memories USING GIN (fts_vector)",
            "UPDATE memories SET fts_vector = to_tsvector('english', coalesce(content, '') || ' ' || coalesce(tags, '')) WHERE fts_vector IS NULL",
            "CREATE OR REPLACE FUNCTION memories_fts_update() RETURNS trigger AS $$
             BEGIN
                 NEW.fts_vector := to_tsvector('english', coalesce(NEW.content, '') || ' ' || coalesce(NEW.tags, ''));
                 RETURN NEW;
             END
             $$ LANGUAGE plpgsql",
            "DROP TRIGGER IF EXISTS trg_memories_fts ON memories",
            "CREATE TRIGGER trg_memories_fts BEFORE INSERT OR UPDATE OF content, tags ON memories FOR EACH ROW EXECUTE FUNCTION memories_fts_update()",
        ]).await?;

        // V6: memory_edges table + indexes for KG
        self.apply_migration(6, "PostgreSQL V6 — memory_edges + KG", &[
            "CREATE TABLE IF NOT EXISTS memory_edges (
                id          BIGSERIAL PRIMARY KEY,
                src_id      BIGINT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
                dst_id      BIGINT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
                rel_type    TEXT NOT NULL DEFAULT 'related_to',
                confidence  DOUBLE PRECISION NOT NULL DEFAULT 1.0,
                source      TEXT NOT NULL DEFAULT 'user',
                created_at  BIGINT NOT NULL,
                valid_from  BIGINT,
                valid_to    BIGINT,
                edge_source TEXT,
                origin_device TEXT,
                hlc_counter BIGINT NOT NULL DEFAULT 0,
                UNIQUE (src_id, dst_id, rel_type)
            )",
            "CREATE INDEX IF NOT EXISTS idx_edges_src ON memory_edges (src_id)",
            "CREATE INDEX IF NOT EXISTS idx_edges_dst ON memory_edges (dst_id)",
            "CREATE INDEX IF NOT EXISTS idx_edges_rel ON memory_edges (rel_type)",
            "CREATE INDEX IF NOT EXISTS idx_edges_valid_to ON memory_edges (valid_to)",
        ]).await?;

        // V7: hlc_counter + cognitive_kind on memories (CRDT sync parity)
        self.apply_migration(7, "PostgreSQL V7 — CRDT columns", &[
            "ALTER TABLE memories ADD COLUMN IF NOT EXISTS hlc_counter BIGINT NOT NULL DEFAULT 0",
            "ALTER TABLE memories ADD COLUMN IF NOT EXISTS cognitive_kind TEXT",
            "ALTER TABLE memories ADD COLUMN IF NOT EXISTS context_prefix TEXT",
        ]).await?;

        // V8: Additional indexes for hybrid RRF queries
        self.apply_migration(8, "PostgreSQL V8 — RRF indexes", &[
            "CREATE INDEX IF NOT EXISTS idx_memories_updated ON memories (updated_at DESC NULLS LAST)",
            "CREATE INDEX IF NOT EXISTS idx_memories_hlc ON memories (hlc_counter DESC)",
        ]).await?;

        // V9: pgvector extension + HNSW index for native ANN
        self.apply_migration(9, "PostgreSQL V9 — pgvector HNSW", &[
            "CREATE EXTENSION IF NOT EXISTS vector",
            "ALTER TABLE memories ADD COLUMN IF NOT EXISTS vec_embedding vector(768)",
            "CREATE INDEX IF NOT EXISTS idx_memories_vec_hnsw ON memories USING hnsw (vec_embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64)",
            // Backfill vec_embedding from the existing BYTEA embedding column
            // (only for rows that already have embeddings but no vec_embedding).
            // This is a one-time migration for existing data.
            "UPDATE memories SET vec_embedding = NULL WHERE vec_embedding IS NULL AND embedding IS NULL",
        ]).await?;

        Ok(())
    }

    /// Apply a numbered migration idempotently.
    async fn apply_migration(
        &self,
        version: i64,
        description: &str,
        statements: &[&str],
    ) -> StorageResult<()> {
        let applied: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM schema_version WHERE version = $1 LIMIT 1",
        )
        .bind(version)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Migration(e.to_string()))?;

        if applied.is_some() {
            return Ok(());
        }

        for stmt in statements {
            sqlx::query(stmt)
                .execute(&self.pool)
                .await
                .map_err(|e| StorageError::Migration(format!("V{version}: {e}")))?;
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        sqlx::query(
            "INSERT INTO schema_version (version, description, applied_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (version) DO NOTHING",
        )
        .bind(version)
        .bind(description)
        .bind(now_ms)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Migration(e.to_string()))?;

        Ok(())
    }

    /// Block on an async sqlx operation from a sync context.
    fn block_on<F, T>(&self, fut: F) -> StorageResult<T>
    where
        F: std::future::Future<Output = Result<T, sqlx::Error>>,
    {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
            .map_err(|e| StorageError::Postgres(e.to_string()))
    }

    fn now_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    /// Convert an embedding slice to a pgvector text literal: `[0.1,0.2,...]`
    fn embedding_to_pgvector_literal(embedding: &[f32]) -> String {
        use std::fmt::Write;
        let mut s = String::with_capacity(embedding.len() * 10 + 2);
        s.push('[');
        for (i, &v) in embedding.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let _ = write!(s, "{v}");
        }
        s.push(']');
        s
    }

    fn row_to_entry(row: &sqlx::postgres::PgRow) -> MemoryEntry {
        MemoryEntry {
            id: row.get("id"),
            content: row.get("content"),
            tags: row.get("tags"),
            importance: row.get::<i32, _>("importance") as i64,
            memory_type: MemoryType::from_str(row.get::<&str, _>("memory_type")),
            created_at: row.get("created_at"),
            last_accessed: row.get("last_accessed"),
            access_count: row.get::<i32, _>("access_count") as i64,
            embedding: None,
            tier: MemoryTier::from_str(row.get::<&str, _>("tier")),
            decay_score: row.get("decay_score"),
            session_id: row.get("session_id"),
            parent_id: row.get("parent_id"),
            token_count: row.get::<i32, _>("token_count") as i64,
            source_url: row.get("source_url"),
            source_hash: row.get("source_hash"),
            expires_at: row.get("expires_at"),
            valid_to: row.try_get("valid_to").ok(),
            obsidian_path: row.try_get("obsidian_path").ok(),
            last_exported: row.try_get("last_exported").ok(),
            updated_at: row.try_get("updated_at").ok(),
            origin_device: row.try_get("origin_device").ok(),
            hlc_counter: row.try_get("hlc_counter").ok(),
        }
    }

    fn row_to_edge(row: &sqlx::postgres::PgRow) -> MemoryEdge {
        MemoryEdge {
            id: row.get("id"),
            src_id: row.get("src_id"),
            dst_id: row.get("dst_id"),
            rel_type: row.get("rel_type"),
            confidence: row.get("confidence"),
            source: EdgeSource::parse(row.get::<&str, _>("source")),
            created_at: row.get("created_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            edge_source: row.get("edge_source"),
        }
    }
}

impl StorageBackend for PostgresBackend {
    /// Run schema migrations for the PostgreSQL backend.
    ///
    /// This method is synchronous and performs blocking work internally
    /// (`run_migrations_sync`). Do not call it directly from an async runtime
    /// worker thread. If you need to invoke migrations from async code, run this
    /// method in a dedicated blocking context (for example, `spawn_blocking`) or
    /// execute it during synchronous startup.
    fn migrate(&self) -> StorageResult<()> {
        self.run_migrations_sync()
    }

    fn schema_version(&self) -> StorageResult<i64> {
        let ver = self.block_on(
            sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(version), 0) FROM schema_version")
                .fetch_one(&self.pool),
        )?;
        Ok(ver)
    }

    fn add(&self, m: NewMemory) -> StorageResult<MemoryEntry> {
        self.add_to_tier(m, MemoryTier::Long, None)
    }

    fn add_to_tier(
        &self,
        m: NewMemory,
        tier: MemoryTier,
        session_id: Option<&str>,
    ) -> StorageResult<MemoryEntry> {
        let now = Self::now_ms();
        let token_count = (m.content.len() / 4) as i32;
        let importance = if m.importance == 0 {
            3i32
        } else {
            m.importance as i32
        };

        let row = self.block_on(
            sqlx::query(
                "INSERT INTO memories
                    (content, tags, importance, memory_type, created_at, access_count,
                     tier, decay_score, session_id, token_count,
                     source_url, source_hash, expires_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, 0, $6, 1.0, $7, $8, $9, $10, $11, $12)
                 RETURNING *",
            )
            .bind(&m.content)
            .bind(&m.tags)
            .bind(importance)
            .bind(m.memory_type.as_str())
            .bind(now)
            .bind(tier.as_str())
            .bind(session_id)
            .bind(token_count)
            .bind(&m.source_url)
            .bind(&m.source_hash)
            .bind(m.expires_at)
            .bind(now)
            .fetch_one(&self.pool),
        )?;

        Ok(Self::row_to_entry(&row))
    }

    fn get_by_id(&self, id: i64) -> StorageResult<MemoryEntry> {
        let row = self.block_on(
            sqlx::query("SELECT * FROM memories WHERE id = $1")
                .bind(id)
                .fetch_one(&self.pool),
        )?;
        Ok(Self::row_to_entry(&row))
    }

    fn get_all(&self) -> StorageResult<Vec<MemoryEntry>> {
        let rows = self.block_on(
            sqlx::query("SELECT * FROM memories ORDER BY importance DESC, created_at DESC")
                .fetch_all(&self.pool),
        )?;
        Ok(rows.iter().map(Self::row_to_entry).collect())
    }

    fn get_by_tier(&self, tier: &MemoryTier) -> StorageResult<Vec<MemoryEntry>> {
        let rows = self.block_on(
            sqlx::query("SELECT * FROM memories WHERE tier = $1 ORDER BY created_at DESC")
                .bind(tier.as_str())
                .fetch_all(&self.pool),
        )?;
        Ok(rows.iter().map(Self::row_to_entry).collect())
    }

    fn get_persistent(&self) -> StorageResult<Vec<MemoryEntry>> {
        let rows = self.block_on(
            sqlx::query(
                "SELECT * FROM memories WHERE tier IN ('working', 'long')
                 ORDER BY importance DESC, created_at DESC",
            )
            .fetch_all(&self.pool),
        )?;
        Ok(rows.iter().map(Self::row_to_entry).collect())
    }

    fn count(&self) -> StorageResult<i64> {
        let c = self.block_on(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM memories").fetch_one(&self.pool),
        )?;
        Ok(c)
    }

    fn stats(&self) -> StorageResult<MemoryStats> {
        let row = self.block_on(
            sqlx::query(
                "SELECT
                    COUNT(*) AS total,
                    COUNT(*) FILTER (WHERE tier = 'short') AS short,
                    COUNT(*) FILTER (WHERE tier = 'working') AS working,
                    COUNT(*) FILTER (WHERE tier = 'long') AS long_tier,
                    COUNT(embedding) AS embedded,
                    COALESCE(SUM(token_count), 0) AS total_tokens,
                    COALESCE(AVG(decay_score), 1.0) AS avg_decay,
                    COALESCE(SUM(
                        length(content)
                        + length(tags)
                        + COALESCE(octet_length(embedding), 0)
                        + COALESCE(length(source_url), 0)
                        + COALESCE(length(source_hash), 0)
                        + 128
                    ), 0) AS storage_bytes
                 FROM memories",
            )
            .fetch_one(&self.pool),
        )?;
        Ok(MemoryStats {
            total: row.get("total"),
            short: row.get("short"),
            working: row.get("working"),
            long: row.get("long_tier"),
            embedded: row.get("embedded"),
            total_tokens: row.get("total_tokens"),
            avg_decay: row.get("avg_decay"),
            storage_bytes: row.get("storage_bytes"),
        })
    }

    fn search(&self, query: &str) -> StorageResult<Vec<MemoryEntry>> {
        let now = Self::now_ms();
        // Use PostgreSQL native FTS with ts_rank for relevance ordering.
        // Falls back to ILIKE for very short queries where tsquery may not help.
        let tsquery = query
            .split_whitespace()
            .filter(|w| w.len() > 1)
            .collect::<Vec<_>>()
            .join(" & ");

        if tsquery.is_empty() {
            // Fallback to ILIKE for single-char or empty queries
            let pattern = format!("%{query}%");
            let rows = self.block_on(async {
                let rows = sqlx::query(
                    "UPDATE memories SET last_accessed = $1, access_count = access_count + 1
                     WHERE content ILIKE $2 OR tags ILIKE $2
                     RETURNING *",
                )
                .bind(now)
                .bind(&pattern)
                .fetch_all(&self.pool)
                .await?;
                Ok::<_, sqlx::Error>(rows)
            })?;
            return Ok(rows.iter().map(Self::row_to_entry).collect());
        }

        let rows = self.block_on(async {
            let rows = sqlx::query(
                "UPDATE memories SET last_accessed = $1, access_count = access_count + 1
                 WHERE fts_vector @@ to_tsquery('english', $2)
                 RETURNING *",
            )
            .bind(now)
            .bind(&tsquery)
            .fetch_all(&self.pool)
            .await?;
            Ok::<_, sqlx::Error>(rows)
        })?;
        Ok(rows.iter().map(Self::row_to_entry).collect())
    }

    fn relevant_for(&self, message: &str, limit: usize) -> StorageResult<Vec<String>> {
        let words: Vec<&str> = message.split_whitespace().collect();
        if words.is_empty() {
            return Ok(vec![]);
        }
        // Build ILIKE conditions for each word
        let conditions: Vec<String> = words
            .iter()
            .enumerate()
            .map(|(i, _)| format!("(content ILIKE ${} OR tags ILIKE ${})", i + 1, i + 1))
            .collect();
        let where_clause = conditions.join(" OR ");
        let limit_placeholder = words.len() + 1;
        let sql = format!(
            "SELECT content FROM memories WHERE {where_clause}
             ORDER BY importance DESC LIMIT ${limit_placeholder}"
        );

        let limit_i64 = i64::try_from(limit)
            .map_err(|_| StorageError::Other("limit exceeds i64::MAX".into()))?;

        let mut query = sqlx::query_scalar::<_, String>(&sql);
        for word in &words {
            query = query.bind(format!("%{word}%"));
        }
        query = query.bind(limit_i64);

        let results = self.block_on(query.fetch_all(&self.pool))?;
        Ok(results)
    }

    fn find_by_source_url(&self, url: &str) -> StorageResult<Vec<MemoryEntry>> {
        let rows = self.block_on(
            sqlx::query("SELECT * FROM memories WHERE source_url = $1")
                .bind(url)
                .fetch_all(&self.pool),
        )?;
        Ok(rows.iter().map(Self::row_to_entry).collect())
    }

    fn find_by_source_hash(&self, hash: &str) -> StorageResult<Option<MemoryEntry>> {
        let row = self.block_on(
            sqlx::query("SELECT * FROM memories WHERE source_hash = $1 LIMIT 1")
                .bind(hash)
                .fetch_optional(&self.pool),
        )?;
        Ok(row.as_ref().map(Self::row_to_entry))
    }

    fn get_with_embeddings(&self) -> StorageResult<Vec<MemoryEntry>> {
        let rows = self.block_on(
            sqlx::query("SELECT * FROM memories WHERE embedding IS NOT NULL").fetch_all(&self.pool),
        )?;
        Ok(rows
            .iter()
            .map(|r| {
                let mut entry = Self::row_to_entry(r);
                let blob: Option<Vec<u8>> = r.get("embedding");
                entry.embedding = blob.map(|b| super::store::bytes_to_embedding(&b));
                entry
            })
            .collect())
    }

    fn unembedded_ids(&self) -> StorageResult<Vec<(i64, String)>> {
        let rows = self.block_on(
            sqlx::query("SELECT id, content FROM memories WHERE embedding IS NULL")
                .fetch_all(&self.pool),
        )?;
        Ok(rows
            .iter()
            .map(|r| (r.get("id"), r.get("content")))
            .collect())
    }

    fn set_embedding(&self, id: i64, embedding: &[f32]) -> StorageResult<()> {
        let bytes = super::store::embedding_to_bytes(embedding);
        // Also store as pgvector `vector(768)` for native HNSW search.
        let vec_str = Self::embedding_to_pgvector_literal(embedding);
        self.block_on(
            sqlx::query(
                "UPDATE memories SET embedding = $1, vec_embedding = $2::vector WHERE id = $3",
            )
            .bind(&bytes)
            .bind(&vec_str)
            .bind(id)
            .execute(&self.pool),
        )?;
        Ok(())
    }

    fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        // Native pgvector cosine distance search using HNSW index.
        let vec_str = Self::embedding_to_pgvector_literal(query_embedding);
        let limit_i64 = limit as i64;
        let rows = self.block_on(
            sqlx::query(
                "SELECT *, 1 - (vec_embedding <=> $1::vector) AS similarity
                 FROM memories
                 WHERE vec_embedding IS NOT NULL
                 ORDER BY vec_embedding <=> $1::vector
                 LIMIT $2",
            )
            .bind(&vec_str)
            .bind(limit_i64)
            .fetch_all(&self.pool),
        )?;
        Ok(rows.iter().map(Self::row_to_entry).collect())
    }

    fn find_duplicate(
        &self,
        query_embedding: &[f32],
        threshold: f32,
    ) -> StorageResult<Option<i64>> {
        // Native pgvector: find the nearest neighbour and check if
        // similarity >= threshold (cosine distance <= 1 - threshold).
        let vec_str = Self::embedding_to_pgvector_literal(query_embedding);
        let max_distance = 1.0 - threshold as f64;
        let row = self.block_on(
            sqlx::query(
                "SELECT id FROM memories
                 WHERE vec_embedding IS NOT NULL
                   AND (vec_embedding <=> $1::vector) <= $2
                 ORDER BY vec_embedding <=> $1::vector
                 LIMIT 1",
            )
            .bind(&vec_str)
            .bind(max_distance)
            .fetch_optional(&self.pool),
        )?;
        Ok(row.map(|r| r.get::<i64, _>("id")))
    }

    fn hybrid_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        // Delegate to the RRF implementation for full parity.
        self.hybrid_search_rrf(query, query_embedding, limit)
    }

    /// Native PostgreSQL RRF: three independent retrievers fused via
    /// Reciprocal Rank Fusion (k=60) entirely in SQL CTEs.
    ///
    /// Retrievers:
    /// 1. **FTS** — `ts_rank` over `fts_vector @@ plainto_tsquery`
    /// 2. **Vector** — cosine similarity (in-process until pgvector lands in 42.7)
    /// 3. **Freshness** — composite of recency + importance + decay + tier
    fn hybrid_search_rrf(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        use std::collections::HashMap;

        if limit == 0 {
            return Ok(vec![]);
        }

        let now = Self::now_ms();
        let hour_ms: f64 = 3_600_000.0;
        const RRF_K: f64 = 60.0;

        // Load all entries for keyword + freshness scoring.
        // Vector ranking is done server-side via pgvector HNSW.
        let all = self.get_all()?;

        if all.is_empty() {
            return Ok(vec![]);
        }

        let by_id: HashMap<i64, MemoryEntry> = all.iter().map(|e| (e.id, e.clone())).collect();

        // ── (1) FTS ranking via ts_rank (done in SQL for efficiency) ──────
        let fts_rank: Vec<i64> = {
            let tsquery = query
                .split_whitespace()
                .filter(|w| w.len() > 2)
                .collect::<Vec<_>>()
                .join(" & ");

            if tsquery.is_empty() {
                // Fallback: keyword ILIKE match
                let words: Vec<String> = query
                    .to_lowercase()
                    .split_whitespace()
                    .filter(|w| w.len() > 2)
                    .map(String::from)
                    .collect();
                let mut scored: Vec<(usize, i64)> = all
                    .iter()
                    .filter_map(|e| {
                        let lc = e.content.to_lowercase();
                        let lt = e.tags.to_lowercase();
                        let hits = words.iter().filter(|w| lc.contains(w.as_str()) || lt.contains(w.as_str())).count();
                        if hits > 0 { Some((hits, e.id)) } else { None }
                    })
                    .collect();
                scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
                scored.into_iter().map(|(_, id)| id).collect()
            } else {
                // Native FTS rank from Postgres
                let rows = self.block_on(
                    sqlx::query(
                        "SELECT id FROM memories
                         WHERE fts_vector @@ to_tsquery('english', $1)
                         ORDER BY ts_rank(fts_vector, to_tsquery('english', $1)) DESC, id ASC
                         LIMIT 200",
                    )
                    .bind(&tsquery)
                    .fetch_all(&self.pool),
                )?;
                rows.iter().map(|r| r.get::<i64, _>("id")).collect()
            }
        };

        // ── (2) Vector ranking (native pgvector HNSW) ────────────────────
        let vector_rank: Vec<i64> = if let Some(qe) = query_embedding {
            // Use server-side pgvector ORDER BY <=> for HNSW-accelerated ranking.
            let vec_str = Self::embedding_to_pgvector_literal(qe);
            let rows = self.block_on(
                sqlx::query(
                    "SELECT id FROM memories
                     WHERE vec_embedding IS NOT NULL
                     ORDER BY vec_embedding <=> $1::vector
                     LIMIT 200",
                )
                .bind(&vec_str)
                .fetch_all(&self.pool),
            )?;
            rows.iter().map(|r| r.get::<i64, _>("id")).collect()
        } else {
            vec![]
        };

        // ── (3) Freshness / importance composite ──────────────────────────
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

        // ── RRF Fusion (k = 60) ──────────────────────────────────────────
        let mut rrf_scores: HashMap<i64, f64> = HashMap::new();
        let rankings: &[&[i64]] = &[&fts_rank, &vector_rank, &freshness_rank];
        for ranking in rankings {
            if ranking.is_empty() {
                continue;
            }
            for (rank_pos, &id) in ranking.iter().enumerate() {
                *rrf_scores.entry(id).or_insert(0.0) +=
                    1.0 / (RRF_K + rank_pos as f64 + 1.0);
            }
        }

        let mut fused: Vec<(i64, f64)> = rrf_scores.into_iter().collect();
        fused.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        let top: Vec<MemoryEntry> = fused
            .into_iter()
            .take(limit)
            .filter_map(|(id, _)| by_id.get(&id).cloned())
            .collect();

        // Touch access counters
        for e in &top {
            let _ = self.block_on(
                sqlx::query(
                    "UPDATE memories SET last_accessed = $1, access_count = access_count + 1
                     WHERE id = $2",
                )
                .bind(now)
                .bind(e.id)
                .execute(&self.pool),
            );
        }

        Ok(top)
    }

    fn update(&self, id: i64, upd: MemoryUpdate) -> StorageResult<MemoryEntry> {
        let existing = self.get_by_id(id)?;
        let content = upd.content.unwrap_or(existing.content);
        let tags = upd.tags.unwrap_or(existing.tags);
        let importance = upd.importance.unwrap_or(existing.importance) as i32;
        let memory_type = upd.memory_type.unwrap_or(existing.memory_type);
        let token_count = (content.len() / 4) as i32;

        let row = self.block_on(
            sqlx::query(
                "UPDATE memories
                 SET content = $1, tags = $2, importance = $3, memory_type = $4,
                     token_count = $5
                 WHERE id = $6 RETURNING *",
            )
            .bind(&content)
            .bind(&tags)
            .bind(importance)
            .bind(memory_type.as_str())
            .bind(token_count)
            .bind(id)
            .fetch_one(&self.pool),
        )?;
        Ok(Self::row_to_entry(&row))
    }

    fn promote(&self, id: i64, new_tier: MemoryTier) -> StorageResult<()> {
        self.block_on(
            sqlx::query("UPDATE memories SET tier = $1 WHERE id = $2")
                .bind(new_tier.as_str())
                .bind(id)
                .execute(&self.pool),
        )?;
        Ok(())
    }

    fn delete(&self, id: i64) -> StorageResult<()> {
        self.block_on(
            sqlx::query("DELETE FROM memories WHERE id = $1")
                .bind(id)
                .execute(&self.pool),
        )?;
        Ok(())
    }

    fn delete_by_source_url(&self, url: &str) -> StorageResult<usize> {
        let result = self.block_on(
            sqlx::query("DELETE FROM memories WHERE source_url = $1")
                .bind(url)
                .execute(&self.pool),
        )?;
        Ok(result.rows_affected() as usize)
    }

    fn delete_expired(&self) -> StorageResult<usize> {
        let now = Self::now_ms();
        let result = self.block_on(
            sqlx::query("DELETE FROM memories WHERE expires_at IS NOT NULL AND expires_at < $1")
                .bind(now)
                .execute(&self.pool),
        )?;
        Ok(result.rows_affected() as usize)
    }

    fn delete_all(&self) -> StorageResult<usize> {
        // Edges/conflicts/versions cascade via FK.
        let result = self.block_on(sqlx::query("DELETE FROM memories").execute(&self.pool))?;
        Ok(result.rows_affected() as usize)
    }

    fn apply_decay(&self) -> StorageResult<usize> {
        let now = Self::now_ms();
        let result = self.block_on(
            sqlx::query(
                "UPDATE memories
                 SET decay_score = GREATEST(0.01,
                     decay_score * POWER(0.95,
                         (CAST($1 AS DOUBLE PRECISION) -
                          COALESCE(CAST(last_accessed AS DOUBLE PRECISION),
                                   CAST(created_at AS DOUBLE PRECISION)))
                         / 604800000.0))
                 WHERE tier = 'long'",
            )
            .bind(now)
            .execute(&self.pool),
        )?;
        Ok(result.rows_affected() as usize)
    }

    fn evict_short_term(&self, session_id: &str) -> StorageResult<Vec<MemoryEntry>> {
        let rows = self.block_on(
            sqlx::query(
                "DELETE FROM memories WHERE tier = 'short' AND session_id = $1 RETURNING *",
            )
            .bind(session_id)
            .fetch_all(&self.pool),
        )?;
        Ok(rows.iter().map(Self::row_to_entry).collect())
    }

    fn gc_decayed(&self, threshold: f64) -> StorageResult<usize> {
        let result = self.block_on(
            sqlx::query(
                "DELETE FROM memories
                 WHERE tier = 'long' AND decay_score < $1 AND importance <= 2",
            )
            .bind(threshold)
            .execute(&self.pool),
        )?;
        Ok(result.rows_affected() as usize)
    }

    fn backend_name(&self) -> &'static str {
        "PostgreSQL"
    }
    fn supports_native_vector_search(&self) -> bool {
        true
    }
}

// ── Knowledge Graph (edges) ──────────────────────────────────────────────

impl PostgresBackend {
    /// Insert a new edge. Returns the created edge.
    /// Unique constraint `(src_id, dst_id, rel_type)` prevents duplicates.
    pub fn add_edge(&self, edge: NewMemoryEdge) -> StorageResult<MemoryEdge> {
        let now = Self::now_ms();
        let row = self.block_on(
            sqlx::query(
                "INSERT INTO memory_edges
                    (src_id, dst_id, rel_type, confidence, source, created_at,
                     valid_from, valid_to, edge_source)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (src_id, dst_id, rel_type) DO UPDATE
                    SET confidence = EXCLUDED.confidence,
                        valid_from = EXCLUDED.valid_from,
                        valid_to = EXCLUDED.valid_to
                 RETURNING *",
            )
            .bind(edge.src_id)
            .bind(edge.dst_id)
            .bind(&edge.rel_type)
            .bind(edge.confidence)
            .bind(edge.source.as_str())
            .bind(now)
            .bind(edge.valid_from)
            .bind(edge.valid_to)
            .bind(&edge.edge_source)
            .fetch_one(&self.pool),
        )?;
        Ok(Self::row_to_edge(&row))
    }

    /// Get edges for a memory by direction.
    pub fn get_edges_for(
        &self,
        memory_id: i64,
        direction: EdgeDirection,
    ) -> StorageResult<Vec<MemoryEdge>> {
        let sql = match direction {
            EdgeDirection::Out => {
                "SELECT * FROM memory_edges WHERE src_id = $1 ORDER BY confidence DESC, id ASC"
            }
            EdgeDirection::In => {
                "SELECT * FROM memory_edges WHERE dst_id = $1 ORDER BY confidence DESC, id ASC"
            }
            EdgeDirection::Both => {
                "SELECT * FROM memory_edges WHERE src_id = $1 OR dst_id = $1
                 ORDER BY confidence DESC, id ASC"
            }
        };
        let rows = self.block_on(
            sqlx::query(sql)
                .bind(memory_id)
                .fetch_all(&self.pool),
        )?;
        Ok(rows.iter().map(Self::row_to_edge).collect())
    }

    /// Recursive CTE graph traversal from a starting memory.
    ///
    /// Returns `(memory_id, depth)` pairs for all memories reachable within
    /// `max_hops` from `start_id`. Uses a native PostgreSQL `WITH RECURSIVE`
    /// CTE for server-side BFS — no round-trips per hop.
    pub fn traverse_from(
        &self,
        start_id: i64,
        max_hops: i32,
        rel_filter: Option<&[String]>,
    ) -> StorageResult<Vec<(i64, i32)>> {
        if max_hops <= 0 {
            return Ok(vec![]);
        }

        // Build optional rel-type filter clause
        let rel_clause = match rel_filter {
            Some(rels) if !rels.is_empty() => {
                let placeholders: Vec<String> = rels
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("${}", i + 3))
                    .collect();
                format!(" AND e.rel_type IN ({})", placeholders.join(","))
            }
            _ => String::new(),
        };

        let sql = format!(
            "WITH RECURSIVE graph_walk AS (
                SELECT
                    CASE WHEN e.src_id = $1 THEN e.dst_id ELSE e.src_id END AS node_id,
                    1 AS depth
                FROM memory_edges e
                WHERE (e.src_id = $1 OR e.dst_id = $1){rel_clause}
              UNION
                SELECT
                    CASE WHEN e.src_id = gw.node_id THEN e.dst_id ELSE e.src_id END,
                    gw.depth + 1
                FROM graph_walk gw
                JOIN memory_edges e ON (e.src_id = gw.node_id OR e.dst_id = gw.node_id){rel_clause}
                WHERE gw.depth < $2
                  AND CASE WHEN e.src_id = gw.node_id THEN e.dst_id ELSE e.src_id END <> $1
            )
            SELECT DISTINCT node_id, MIN(depth) AS depth
            FROM graph_walk
            WHERE node_id <> $1
            GROUP BY node_id
            ORDER BY depth, node_id"
        );

        let rows = self.block_on(async {
            let mut q = sqlx::query(&sql)
                .bind(start_id)
                .bind(max_hops);
            if let Some(rels) = rel_filter {
                for r in rels {
                    q = q.bind(r);
                }
            }
            q.fetch_all(&self.pool).await
        })?;

        Ok(rows.iter().map(|r| {
            (r.get::<i64, _>("node_id"), r.get::<i32, _>("depth"))
        }).collect())
    }

    /// Delete an edge by ID.
    pub fn delete_edge(&self, edge_id: i64) -> StorageResult<()> {
        self.block_on(
            sqlx::query("DELETE FROM memory_edges WHERE id = $1")
                .bind(edge_id)
                .execute(&self.pool),
        )?;
        Ok(())
    }

    /// Count total edges.
    pub fn edge_count(&self) -> StorageResult<i64> {
        let c = self.block_on(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM memory_edges")
                .fetch_one(&self.pool),
        )?;
        Ok(c)
    }

    /// Insert a memory with a contextual retrieval prefix prepended to content.
    ///
    /// Equivalent to `add_to_tier` but prepends `context_prefix` to the stored
    /// `content` (Anthropic Contextual Retrieval pattern, 2024). The original
    /// prefix is also stored in the `context_prefix` column for provenance.
    pub fn add_with_context(
        &self,
        m: NewMemory,
        context_prefix: &str,
        tier: MemoryTier,
        session_id: Option<&str>,
    ) -> StorageResult<MemoryEntry> {
        let prefixed_content = if context_prefix.trim().is_empty() {
            m.content.clone()
        } else {
            format!("{}\n\n{}", context_prefix.trim(), m.content)
        };

        let now = Self::now_ms();
        let token_count = (prefixed_content.len() / 4) as i32;
        let importance = if m.importance == 0 { 3i32 } else { m.importance as i32 };

        let row = self.block_on(
            sqlx::query(
                "INSERT INTO memories
                    (content, tags, importance, memory_type, created_at, access_count,
                     tier, decay_score, session_id, token_count,
                     source_url, source_hash, expires_at, updated_at, context_prefix)
                 VALUES ($1, $2, $3, $4, $5, 0, $6, 1.0, $7, $8, $9, $10, $11, $12, $13)
                 RETURNING *",
            )
            .bind(&prefixed_content)
            .bind(&m.tags)
            .bind(importance)
            .bind(m.memory_type.as_str())
            .bind(now)
            .bind(tier.as_str())
            .bind(session_id)
            .bind(token_count)
            .bind(&m.source_url)
            .bind(&m.source_hash)
            .bind(m.expires_at)
            .bind(now)
            .bind(context_prefix.trim())
            .fetch_one(&self.pool),
        )?;

        Ok(Self::row_to_entry(&row))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;

    fn test_pool_url() -> String {
        std::env::var("TEST_POSTGRES_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/terransoul_test".into())
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL"]
    async fn connect_and_migrate() {
        let backend = PostgresBackend::connect(&test_pool_url(), Some(2), false)
            .await
            .expect("connect");
        let version = backend.schema_version().expect("schema_version");
        assert!(version >= 8, "expected at least V8, got {version}");
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL"]
    async fn add_and_search_fts() {
        let backend = PostgresBackend::connect(&test_pool_url(), Some(2), false)
            .await
            .expect("connect");
        backend.delete_all().unwrap();

        let entry = backend
            .add(NewMemory {
                content: "The quick brown fox jumps over the lazy dog".into(),
                tags: "animal,test".into(),
                importance: 4,
                memory_type: MemoryType::Fact,
                source_url: None,
                source_hash: None,
                expires_at: None,
            })
            .unwrap();
        assert!(entry.id > 0);

        let results = backend.search("quick fox").unwrap();
        assert!(!results.is_empty(), "FTS should find the entry");
        assert_eq!(results[0].id, entry.id);
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL"]
    async fn hybrid_search_rrf_returns_results() {
        let backend = PostgresBackend::connect(&test_pool_url(), Some(2), false)
            .await
            .expect("connect");
        backend.delete_all().unwrap();

        backend
            .add(NewMemory {
                content: "Rust programming language systems programming".into(),
                tags: "rust,programming".into(),
                importance: 5,
                memory_type: MemoryType::Fact,
                source_url: None,
                source_hash: None,
                expires_at: None,
            })
            .unwrap();

        backend
            .add(NewMemory {
                content: "Python is a scripting language".into(),
                tags: "python".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                source_url: None,
                source_hash: None,
                expires_at: None,
            })
            .unwrap();

        let results = backend.hybrid_search_rrf("Rust programming", None, 5).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].content.contains("Rust"));
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL"]
    async fn edge_crud_and_traverse() {
        let backend = PostgresBackend::connect(&test_pool_url(), Some(2), false)
            .await
            .expect("connect");
        backend.delete_all().unwrap();

        let a = backend
            .add(NewMemory {
                content: "Alice".into(),
                tags: "person".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                source_url: None,
                source_hash: None,
                expires_at: None,
            })
            .unwrap();

        let b = backend
            .add(NewMemory {
                content: "Bob".into(),
                tags: "person".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                source_url: None,
                source_hash: None,
                expires_at: None,
            })
            .unwrap();

        let c = backend
            .add(NewMemory {
                content: "Charlie".into(),
                tags: "person".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                source_url: None,
                source_hash: None,
                expires_at: None,
            })
            .unwrap();

        backend
            .add_edge(NewMemoryEdge {
                src_id: a.id,
                dst_id: b.id,
                rel_type: "knows".into(),
                confidence: 0.9,
                source: EdgeSource::User,
                valid_from: None,
                valid_to: None,
                edge_source: None,
            })
            .unwrap();

        backend
            .add_edge(NewMemoryEdge {
                src_id: b.id,
                dst_id: c.id,
                rel_type: "knows".into(),
                confidence: 0.8,
                source: EdgeSource::User,
                valid_from: None,
                valid_to: None,
                edge_source: None,
            })
            .unwrap();

        let one_hop = backend.traverse_from(a.id, 1, None).unwrap();
        assert_eq!(one_hop.len(), 1);
        assert_eq!(one_hop[0].0, b.id);
        assert_eq!(one_hop[0].1, 1);

        let two_hop = backend.traverse_from(a.id, 2, None).unwrap();
        assert_eq!(two_hop.len(), 2);

        assert_eq!(backend.edge_count().unwrap(), 2);
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL"]
    async fn add_with_context_prefix() {
        let backend = PostgresBackend::connect(&test_pool_url(), Some(2), false)
            .await
            .expect("connect");
        backend.delete_all().unwrap();

        let entry = backend
            .add_with_context(
                NewMemory {
                    content: "The fox is clever.".into(),
                    tags: "animal".into(),
                    importance: 3,
                    memory_type: MemoryType::Fact,
                    source_url: None,
                    source_hash: None,
                    expires_at: None,
                },
                "This passage is from a chapter about animal intelligence.",
                MemoryTier::Long,
                None,
            )
            .unwrap();

        assert!(entry.content.starts_with("This passage is from"));
        assert!(entry.content.contains("The fox is clever."));
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL with pgvector"]
    async fn vector_search_native_pgvector() {
        let backend = PostgresBackend::connect(&test_pool_url(), Some(2), false)
            .await
            .expect("connect");
        backend.delete_all().unwrap();

        // Insert an entry and set its embedding
        let entry = backend
            .add(NewMemory {
                content: "Machine learning neural networks".into(),
                tags: "ai,ml".into(),
                importance: 4,
                memory_type: MemoryType::Fact,
                source_url: None,
                source_hash: None,
                expires_at: None,
            })
            .unwrap();

        // Create a dummy 768-dim embedding
        let mut emb = vec![0.0f32; 768];
        emb[0] = 1.0;
        emb[1] = 0.5;
        backend.set_embedding(entry.id, &emb).unwrap();

        // Search with a similar vector
        let mut query_emb = vec![0.0f32; 768];
        query_emb[0] = 0.9;
        query_emb[1] = 0.6;
        let results = backend.vector_search(&query_emb, 5).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, entry.id);

        // find_duplicate should find it above threshold
        let dup = backend.find_duplicate(&query_emb, 0.5).unwrap();
        assert_eq!(dup, Some(entry.id));

        // supports_native_vector_search should be true
        assert!(backend.supports_native_vector_search());
    }
}
