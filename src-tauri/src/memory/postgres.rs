//! PostgreSQL storage backend — for distributed/multi-device deployments.
//!
//! Uses `sqlx` with the `postgres` feature. Requires a PostgreSQL 14+ server
//! with the `pgvector` extension for native vector similarity search.
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
//! Mirrors the SQLite V4 schema with PostgreSQL-native types:
//! - `SERIAL` instead of `AUTOINCREMENT`
//! - `BYTEA` for embedding blobs
//! - `BIGINT` for timestamps
//! - Optional `vector(768)` column via pgvector for native ANN search

#![cfg(feature = "postgres")]

use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

use super::backend::{StorageBackend, StorageError, StorageResult};
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
        let v2_applied: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM schema_version WHERE version = 2 LIMIT 1",
        )
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
        let v3_applied: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM schema_version WHERE version = 3 LIMIT 1",
        )
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
        let v4_applied: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM schema_version WHERE version = 4 LIMIT 1",
        )
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
                    COALESCE(AVG(decay_score), 1.0) AS avg_decay
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
        })
    }

    fn search(&self, query: &str) -> StorageResult<Vec<MemoryEntry>> {
        let pattern = format!("%{query}%");
        let now = Self::now_ms();
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
            .map_err(|_| StorageError::Other("limit is too large for PostgreSQL BIGINT".into()))?;

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
        self.block_on(
            sqlx::query("UPDATE memories SET embedding = $1 WHERE id = $2")
                .bind(&bytes)
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
        // Load all embeddings and do in-process cosine similarity
        // (pgvector native search can be added as an optimization)
        let all = self.get_with_embeddings()?;
        let mut scored: Vec<(f64, MemoryEntry)> = all
            .into_iter()
            .filter_map(|entry| {
                entry.embedding.as_ref().map(|emb| {
                    let score = super::store::cosine_similarity(query_embedding, emb) as f64;
                    (score, entry.clone())
                })
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored.into_iter().map(|(_, e)| e).collect())
    }

    fn find_duplicate(
        &self,
        query_embedding: &[f32],
        threshold: f32,
    ) -> StorageResult<Option<i64>> {
        let all = self.get_with_embeddings()?;
        let mut best: Option<(f32, i64)> = None;
        for entry in &all {
            if let Some(emb) = &entry.embedding {
                let sim = super::store::cosine_similarity(query_embedding, emb);
                if sim >= threshold && best.is_none_or(|(s, _)| sim > s) {
                    best = Some((sim, entry.id));
                }
            }
        }
        Ok(best.map(|(_, id)| id))
    }

    fn hybrid_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        // Load all entries with embeddings for scoring
        let all = self.get_with_embeddings()?;
        let query_words: Vec<&str> = query.split_whitespace().collect();
        let now = Self::now_ms();

        let mut scored: Vec<(f64, MemoryEntry)> = all
            .into_iter()
            .map(|entry| {
                let vector_score = query_embedding
                    .and_then(|qe| {
                        entry
                            .embedding
                            .as_ref()
                            .map(|ee| super::store::cosine_similarity(qe, ee) as f64)
                    })
                    .unwrap_or(0.0);

                let keyword_hits = query_words
                    .iter()
                    .filter(|w| {
                        entry.content.to_lowercase().contains(&w.to_lowercase())
                            || entry.tags.to_lowercase().contains(&w.to_lowercase())
                    })
                    .count() as f64;
                let keyword_score = if query_words.is_empty() {
                    0.0
                } else {
                    keyword_hits / query_words.len() as f64
                };

                let age_hours = (now - entry.created_at) as f64 / 3_600_000.0;
                let recency_score = (-age_hours / 24.0_f64).exp();

                let importance_score = entry.importance as f64 / 5.0;
                let decay_component = entry.decay_score;
                let tier_boost = match entry.tier {
                    MemoryTier::Working => 1.0,
                    MemoryTier::Long => 0.5,
                    MemoryTier::Short => 0.3,
                };

                let score = 0.40 * vector_score
                    + 0.20 * keyword_score
                    + 0.15 * recency_score
                    + 0.10 * importance_score
                    + 0.10 * decay_component
                    + 0.05 * tier_boost;

                (score, entry)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        // Update access counters for returned entries
        for (_, entry) in &scored {
            let _ = self.block_on(
                sqlx::query(
                    "UPDATE memories SET last_accessed = $1, access_count = access_count + 1
                     WHERE id = $2",
                )
                .bind(now)
                .bind(entry.id)
                .execute(&self.pool),
            );
        }

        Ok(scored.into_iter().map(|(_, e)| e).collect())
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
        false
    } // pgvector upgrade path exists
}
