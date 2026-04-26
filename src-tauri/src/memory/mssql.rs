//! SQL Server storage backend — for enterprise/Azure environments.
//!
//! Uses `tiberius` with `tokio-util` for TDS connections. Requires SQL Server 2019+
//! or Azure SQL Database.
//!
//! Connection string format:
//! ```text
//! Server=tcp:host,1433;Database=terransoul;User Id=user;Password=pass;Encrypt=true;
//! ```
//!
//! # Cargo feature
//! Enable with `--features mssql` in Cargo.toml.
//!
//! # Schema
//! Mirrors the SQLite V4 schema with SQL Server types:
//! - `BIGINT IDENTITY(1,1)` instead of `AUTOINCREMENT`
//! - `VARBINARY(MAX)` for embedding blobs
//! - `NVARCHAR(MAX)` for text fields
//! - `FLOAT` for decay_score

#![cfg(feature = "mssql")]

use tiberius::{Client, Config, AuthMethod, Row};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

use super::backend::{StorageBackend, StorageError, StorageResult};
use super::store::{
    MemoryEntry, MemoryStats, MemoryTier, MemoryType, MemoryUpdate, NewMemory,
};

/// SQL Server storage backend.
pub struct MssqlBackend {
    client: tokio::sync::Mutex<Client<tokio_util::compat::Compat<TcpStream>>>,
}

impl MssqlBackend {
    /// Create a new SQL Server backend.
    pub async fn connect(
        connection_string: &str,
        _max_connections: Option<u32>,
    ) -> StorageResult<Self> {
        let config = Config::from_ado_string(connection_string)
            .map_err(|e| StorageError::Mssql(e.to_string()))?;

        let tcp = TcpStream::connect(config.get_addr())
            .await
            .map_err(|e| StorageError::Mssql(e.to_string()))?;

        tcp.set_nodelay(true)
            .map_err(|e| StorageError::Mssql(e.to_string()))?;

        let client = Client::connect(config, tcp.compat_write())
            .await
            .map_err(|e| StorageError::Mssql(e.to_string()))?;

        let backend = Self {
            client: tokio::sync::Mutex::new(client),
        };
        backend.migrate()?;
        Ok(backend)
    }

    fn now_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    /// Block on an async operation from a sync context.
    fn block_on<F, T>(&self, fut: F) -> StorageResult<T>
    where
        F: std::future::Future<Output = StorageResult<T>>,
    {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(fut)
        })
    }

    fn row_to_entry(row: &Row) -> MemoryEntry {
        MemoryEntry {
            id: row.get::<i64, _>("id").unwrap_or(0),
            content: row.get::<&str, _>("content").unwrap_or("").to_string(),
            tags: row.get::<&str, _>("tags").unwrap_or("").to_string(),
            importance: row.get::<i32, _>("importance").unwrap_or(3) as i64,
            memory_type: MemoryType::from_str(
                row.get::<&str, _>("memory_type").unwrap_or("fact"),
            ),
            created_at: row.get::<i64, _>("created_at").unwrap_or(0),
            last_accessed: row.get::<i64, _>("last_accessed"),
            access_count: row.get::<i32, _>("access_count").unwrap_or(0) as i64,
            embedding: None,
            tier: MemoryTier::from_str(
                row.get::<&str, _>("tier").unwrap_or("long"),
            ),
            decay_score: row.get::<f64, _>("decay_score").unwrap_or(1.0),
            session_id: row.get::<&str, _>("session_id").map(|s| s.to_string()),
            parent_id: row.get::<i64, _>("parent_id"),
            token_count: row.get::<i32, _>("token_count").unwrap_or(0) as i64,
            source_url: row.get::<&str, _>("source_url").map(|s| s.to_string()),
            source_hash: row.get::<&str, _>("source_hash").map(|s| s.to_string()),
            expires_at: row.get::<i64, _>("expires_at"),
            valid_to: None,
        }
    }
}

impl StorageBackend for MssqlBackend {
    fn migrate(&self) -> StorageResult<()> {
        self.block_on(async {
            let mut client = self.client.lock().await;

            // Create schema_version table
            client.execute(
                "IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = 'schema_version')
                 CREATE TABLE schema_version (
                     version     BIGINT PRIMARY KEY,
                     description NVARCHAR(MAX) NOT NULL,
                     applied_at  BIGINT NOT NULL
                 )",
                &[],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;

            // Create memories table
            client.execute(
                "IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = 'memories')
                 CREATE TABLE memories (
                     id            BIGINT IDENTITY(1,1) PRIMARY KEY,
                     content       NVARCHAR(MAX) NOT NULL,
                     tags          NVARCHAR(MAX) NOT NULL DEFAULT '',
                     importance    INT NOT NULL DEFAULT 3,
                     memory_type   NVARCHAR(50) NOT NULL DEFAULT 'fact',
                     created_at    BIGINT NOT NULL,
                     last_accessed BIGINT NULL,
                     access_count  INT NOT NULL DEFAULT 0,
                     embedding     VARBINARY(MAX) NULL,
                     source_url    NVARCHAR(MAX) NULL,
                     source_hash   NVARCHAR(128) NULL,
                     expires_at    BIGINT NULL,
                     tier          NVARCHAR(20) NOT NULL DEFAULT 'long',
                     decay_score   FLOAT NOT NULL DEFAULT 1.0,
                     session_id    NVARCHAR(255) NULL,
                     parent_id     BIGINT NULL REFERENCES memories(id),
                     token_count   INT NOT NULL DEFAULT 0
                 )",
                &[],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;

            // Create indexes
            for idx_sql in &[
                "IF NOT EXISTS (SELECT * FROM sys.indexes WHERE name = 'idx_memories_importance')
                 CREATE INDEX idx_memories_importance ON memories (importance DESC)",
                "IF NOT EXISTS (SELECT * FROM sys.indexes WHERE name = 'idx_memories_created')
                 CREATE INDEX idx_memories_created ON memories (created_at DESC)",
                "IF NOT EXISTS (SELECT * FROM sys.indexes WHERE name = 'idx_memories_source_hash')
                 CREATE INDEX idx_memories_source_hash ON memories (source_hash)",
                "IF NOT EXISTS (SELECT * FROM sys.indexes WHERE name = 'idx_memories_tier')
                 CREATE INDEX idx_memories_tier ON memories (tier)",
                "IF NOT EXISTS (SELECT * FROM sys.indexes WHERE name = 'idx_memories_decay')
                 CREATE INDEX idx_memories_decay ON memories (decay_score DESC)",
            ] {
                client.execute(*idx_sql, &[])
                    .await
                    .map_err(|e| StorageError::Mssql(e.to_string()))?;
            }

            Ok(())
        })
    }

    fn schema_version(&self) -> StorageResult<i64> {
        self.block_on(async {
            let mut client = self.client.lock().await;
            let row = client.query(
                "SELECT ISNULL(MAX(version), 0) AS ver FROM schema_version",
                &[],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?
             .into_row()
             .await
             .map_err(|e| StorageError::Mssql(e.to_string()))?;

            Ok(row.map(|r| r.get::<i64, _>("ver").unwrap_or(0)).unwrap_or(0))
        })
    }

    fn add(&self, m: NewMemory) -> StorageResult<MemoryEntry> {
        self.add_to_tier(m, MemoryTier::Long, None)
    }

    fn add_to_tier(
        &self, m: NewMemory, tier: MemoryTier, session_id: Option<&str>,
    ) -> StorageResult<MemoryEntry> {
        let now = Self::now_ms();
        let token_count = (m.content.len() / 4) as i32;
        let importance = if m.importance == 0 { 3i32 } else { m.importance as i32 };
        let content = m.content.clone();
        let tags = m.tags.clone();
        let mtype = m.memory_type.as_str().to_string();
        let tier_str = tier.as_str().to_string();
        let sess = session_id.map(|s| s.to_string());
        let src_url = m.source_url.clone();
        let src_hash = m.source_hash.clone();
        let expires = m.expires_at;

        self.block_on(async {
            let mut client = self.client.lock().await;
            let result = client.query(
                "INSERT INTO memories
                    (content, tags, importance, memory_type, created_at, access_count,
                     tier, decay_score, session_id, token_count,
                     source_url, source_hash, expires_at)
                 OUTPUT INSERTED.*
                 VALUES (@P1, @P2, @P3, @P4, @P5, 0, @P6, 1.0, @P7, @P8, @P9, @P10, @P11)",
                &[
                    &content, &tags, &importance, &mtype, &now,
                    &tier_str, &sess, &token_count,
                    &src_url, &src_hash, &expires,
                ],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;

            let row = result.into_row()
                .await
                .map_err(|e| StorageError::Mssql(e.to_string()))?
                .ok_or_else(|| StorageError::Mssql("No row returned from INSERT".into()))?;

            Ok(Self::row_to_entry(&row))
        })
    }

    fn get_by_id(&self, id: i64) -> StorageResult<MemoryEntry> {
        self.block_on(async {
            let mut client = self.client.lock().await;
            let row = client.query("SELECT * FROM memories WHERE id = @P1", &[&id])
                .await.map_err(|e| StorageError::Mssql(e.to_string()))?
                .into_row()
                .await
                .map_err(|e| StorageError::Mssql(e.to_string()))?
                .ok_or_else(|| StorageError::Mssql(format!("Memory {id} not found")))?;
            Ok(Self::row_to_entry(&row))
        })
    }

    fn get_all(&self) -> StorageResult<Vec<MemoryEntry>> {
        self.block_on(async {
            let mut client = self.client.lock().await;
            let stream = client.query(
                "SELECT * FROM memories ORDER BY importance DESC, created_at DESC",
                &[],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;

            let rows = stream.into_first_result()
                .await
                .map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(rows.iter().map(Self::row_to_entry).collect())
        })
    }

    fn get_by_tier(&self, tier: &MemoryTier) -> StorageResult<Vec<MemoryEntry>> {
        let tier_str = tier.as_str().to_string();
        self.block_on(async {
            let mut client = self.client.lock().await;
            let stream = client.query(
                "SELECT * FROM memories WHERE tier = @P1 ORDER BY created_at DESC",
                &[&tier_str],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            let rows = stream.into_first_result().await
                .map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(rows.iter().map(Self::row_to_entry).collect())
        })
    }

    fn get_persistent(&self) -> StorageResult<Vec<MemoryEntry>> {
        self.block_on(async {
            let mut client = self.client.lock().await;
            let stream = client.query(
                "SELECT * FROM memories WHERE tier IN ('working', 'long')
                 ORDER BY importance DESC, created_at DESC",
                &[],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            let rows = stream.into_first_result().await
                .map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(rows.iter().map(Self::row_to_entry).collect())
        })
    }

    fn count(&self) -> StorageResult<i64> {
        self.block_on(async {
            let mut client = self.client.lock().await;
            let row = client.query("SELECT COUNT(*) AS cnt FROM memories", &[])
                .await.map_err(|e| StorageError::Mssql(e.to_string()))?
                .into_row().await
                .map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(row.map(|r| r.get::<i64, _>("cnt").unwrap_or(0)).unwrap_or(0))
        })
    }

    fn stats(&self) -> StorageResult<MemoryStats> {
        self.block_on(async {
            let mut client = self.client.lock().await;
            let row = client.query(
                "SELECT
                    COUNT(*) AS total,
                    SUM(CASE WHEN tier = 'short' THEN 1 ELSE 0 END) AS short_cnt,
                    SUM(CASE WHEN tier = 'working' THEN 1 ELSE 0 END) AS working_cnt,
                    SUM(CASE WHEN tier = 'long' THEN 1 ELSE 0 END) AS long_cnt,
                    SUM(CASE WHEN embedding IS NOT NULL THEN 1 ELSE 0 END) AS embedded,
                    ISNULL(SUM(token_count), 0) AS total_tokens,
                    ISNULL(AVG(CAST(decay_score AS FLOAT)), 1.0) AS avg_decay
                 FROM memories",
                &[],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?
             .into_row().await
             .map_err(|e| StorageError::Mssql(e.to_string()))?
             .ok_or_else(|| StorageError::Mssql("No stats row".into()))?;

            Ok(MemoryStats {
                total: row.get::<i64, _>("total").unwrap_or(0),
                short: row.get::<i64, _>("short_cnt").unwrap_or(0),
                working: row.get::<i64, _>("working_cnt").unwrap_or(0),
                long: row.get::<i64, _>("long_cnt").unwrap_or(0),
                embedded: row.get::<i64, _>("embedded").unwrap_or(0),
                total_tokens: row.get::<i64, _>("total_tokens").unwrap_or(0),
                avg_decay: row.get::<f64, _>("avg_decay").unwrap_or(1.0),
            })
        })
    }

    fn search(&self, query: &str) -> StorageResult<Vec<MemoryEntry>> {
        let pattern = format!("%{query}%");
        self.block_on(async {
            let mut client = self.client.lock().await;
            let stream = client.query(
                "SELECT * FROM memories WHERE content LIKE @P1 OR tags LIKE @P1",
                &[&pattern],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            let rows = stream.into_first_result().await
                .map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(rows.iter().map(Self::row_to_entry).collect())
        })
    }

    fn relevant_for(&self, message: &str, limit: usize) -> StorageResult<Vec<String>> {
        let pattern = format!("%{}%", message.split_whitespace().next().unwrap_or(""));
        self.block_on(async {
            let mut client = self.client.lock().await;
            let stream = client.query(
                &format!(
                    "SELECT TOP {limit} content FROM memories
                     WHERE content LIKE @P1 OR tags LIKE @P1
                     ORDER BY importance DESC"
                ),
                &[&pattern],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            let rows = stream.into_first_result().await
                .map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(rows.iter().map(|r| r.get::<&str, _>("content").unwrap_or("").to_string()).collect())
        })
    }

    fn find_by_source_url(&self, url: &str) -> StorageResult<Vec<MemoryEntry>> {
        let url = url.to_string();
        self.block_on(async {
            let mut client = self.client.lock().await;
            let stream = client.query(
                "SELECT * FROM memories WHERE source_url = @P1", &[&url],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            let rows = stream.into_first_result().await
                .map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(rows.iter().map(Self::row_to_entry).collect())
        })
    }

    fn find_by_source_hash(&self, hash: &str) -> StorageResult<Option<MemoryEntry>> {
        let hash = hash.to_string();
        self.block_on(async {
            let mut client = self.client.lock().await;
            let row = client.query(
                "SELECT TOP 1 * FROM memories WHERE source_hash = @P1", &[&hash],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?
             .into_row().await
             .map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(row.as_ref().map(Self::row_to_entry))
        })
    }

    fn get_with_embeddings(&self) -> StorageResult<Vec<MemoryEntry>> {
        self.block_on(async {
            let mut client = self.client.lock().await;
            let stream = client.query(
                "SELECT * FROM memories WHERE embedding IS NOT NULL", &[],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            let rows = stream.into_first_result().await
                .map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(rows.iter().map(|r| {
                let mut entry = Self::row_to_entry(r);
                if let Some(blob) = r.get::<&[u8], _>("embedding") {
                    entry.embedding = Some(super::store::bytes_to_embedding(blob));
                }
                entry
            }).collect())
        })
    }

    fn unembedded_ids(&self) -> StorageResult<Vec<(i64, String)>> {
        self.block_on(async {
            let mut client = self.client.lock().await;
            let stream = client.query(
                "SELECT id, content FROM memories WHERE embedding IS NULL", &[],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            let rows = stream.into_first_result().await
                .map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(rows.iter().map(|r| (
                r.get::<i64, _>("id").unwrap_or(0),
                r.get::<&str, _>("content").unwrap_or("").to_string(),
            )).collect())
        })
    }

    fn set_embedding(&self, id: i64, embedding: &[f32]) -> StorageResult<()> {
        let bytes = super::store::embedding_to_bytes(embedding);
        self.block_on(async {
            let mut client = self.client.lock().await;
            client.execute(
                "UPDATE memories SET embedding = @P1 WHERE id = @P2",
                &[&bytes.as_slice(), &id],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(())
        })
    }

    fn vector_search(
        &self, query_embedding: &[f32], limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        // In-process cosine similarity (same as Postgres backend)
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
        &self, query_embedding: &[f32], threshold: f32,
    ) -> StorageResult<Option<i64>> {
        let all = self.get_with_embeddings()?;
        let mut best: Option<(f32, i64)> = None;
        for entry in &all {
            if let Some(emb) = &entry.embedding {
                let sim = super::store::cosine_similarity(query_embedding, emb);
                if sim >= threshold && best.map_or(true, |(s, _)| sim > s) {
                    best = Some((sim, entry.id));
                }
            }
        }
        Ok(best.map(|(_, id)| id))
    }

    fn hybrid_search(
        &self, query: &str, query_embedding: Option<&[f32]>, limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        // Same 6-signal scoring as other backends
        let all = self.get_with_embeddings()?;
        let query_words: Vec<&str> = query.split_whitespace().collect();
        let now = Self::now_ms();

        let mut scored: Vec<(f64, MemoryEntry)> = all
            .into_iter()
            .map(|entry| {
                let vector_score = query_embedding
                    .and_then(|qe| entry.embedding.as_ref().map(|ee| {
                        super::store::cosine_similarity(qe, ee) as f64
                    }))
                    .unwrap_or(0.0);
                let keyword_hits = query_words.iter()
                    .filter(|w| {
                        entry.content.to_lowercase().contains(&w.to_lowercase())
                            || entry.tags.to_lowercase().contains(&w.to_lowercase())
                    })
                    .count() as f64;
                let keyword_score = if query_words.is_empty() { 0.0 }
                    else { keyword_hits / query_words.len() as f64 };
                let age_hours = (now - entry.created_at) as f64 / 3_600_000.0;
                let recency_score = (-age_hours / 24.0_f64).exp();
                let importance_score = entry.importance as f64 / 5.0;
                let tier_boost = match entry.tier {
                    MemoryTier::Working => 1.0,
                    MemoryTier::Long => 0.5,
                    MemoryTier::Short => 0.3,
                };
                let score = 0.40 * vector_score
                    + 0.20 * keyword_score
                    + 0.15 * recency_score
                    + 0.10 * importance_score
                    + 0.10 * entry.decay_score
                    + 0.05 * tier_boost;
                (score, entry)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored.into_iter().map(|(_, e)| e).collect())
    }

    fn update(&self, id: i64, upd: MemoryUpdate) -> StorageResult<MemoryEntry> {
        let existing = self.get_by_id(id)?;
        let content = upd.content.unwrap_or(existing.content);
        let tags = upd.tags.unwrap_or(existing.tags);
        let importance = upd.importance.unwrap_or(existing.importance) as i32;
        let mtype = upd.memory_type.unwrap_or(existing.memory_type).as_str().to_string();
        let token_count = (content.len() / 4) as i32;

        self.block_on(async {
            let mut client = self.client.lock().await;
            client.execute(
                "UPDATE memories SET content = @P1, tags = @P2, importance = @P3,
                 memory_type = @P4, token_count = @P5 WHERE id = @P6",
                &[&content, &tags, &importance, &mtype, &token_count, &id],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            drop(client);
            self.get_by_id(id)
        })
    }

    fn promote(&self, id: i64, new_tier: MemoryTier) -> StorageResult<()> {
        let tier_str = new_tier.as_str().to_string();
        self.block_on(async {
            let mut client = self.client.lock().await;
            client.execute(
                "UPDATE memories SET tier = @P1 WHERE id = @P2",
                &[&tier_str, &id],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(())
        })
    }

    fn delete(&self, id: i64) -> StorageResult<()> {
        self.block_on(async {
            let mut client = self.client.lock().await;
            client.execute("DELETE FROM memories WHERE id = @P1", &[&id])
                .await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(())
        })
    }

    fn delete_by_source_url(&self, url: &str) -> StorageResult<usize> {
        let url = url.to_string();
        self.block_on(async {
            let mut client = self.client.lock().await;
            let result = client.execute(
                "DELETE FROM memories WHERE source_url = @P1", &[&url],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(result.rows_affected().iter().sum::<u64>() as usize)
        })
    }

    fn delete_expired(&self) -> StorageResult<usize> {
        let now = Self::now_ms();
        self.block_on(async {
            let mut client = self.client.lock().await;
            let result = client.execute(
                "DELETE FROM memories WHERE expires_at IS NOT NULL AND expires_at < @P1",
                &[&now],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(result.rows_affected().iter().sum::<u64>() as usize)
        })
    }

    fn apply_decay(&self) -> StorageResult<usize> {
        let now = Self::now_ms();
        self.block_on(async {
            let mut client = self.client.lock().await;
            let result = client.execute(
                "UPDATE memories
                 SET decay_score = CASE
                     WHEN POWER(0.95,
                         (CAST(@P1 AS FLOAT) - ISNULL(CAST(last_accessed AS FLOAT), CAST(created_at AS FLOAT)))
                         / 604800000.0) * decay_score < 0.01
                     THEN 0.01
                     ELSE POWER(0.95,
                         (CAST(@P1 AS FLOAT) - ISNULL(CAST(last_accessed AS FLOAT), CAST(created_at AS FLOAT)))
                         / 604800000.0) * decay_score
                 END
                 WHERE tier = 'long'",
                &[&now],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(result.rows_affected().iter().sum::<u64>() as usize)
        })
    }

    fn evict_short_term(&self, session_id: &str) -> StorageResult<Vec<MemoryEntry>> {
        let sess = session_id.to_string();
        self.block_on(async {
            let mut client = self.client.lock().await;
            // Fetch then delete (SQL Server doesn't have DELETE ... RETURNING)
            let stream = client.query(
                "SELECT * FROM memories WHERE tier = 'short' AND session_id = @P1",
                &[&sess],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            let rows = stream.into_first_result().await
                .map_err(|e| StorageError::Mssql(e.to_string()))?;
            let entries: Vec<MemoryEntry> = rows.iter().map(Self::row_to_entry).collect();

            client.execute(
                "DELETE FROM memories WHERE tier = 'short' AND session_id = @P1",
                &[&sess],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;

            Ok(entries)
        })
    }

    fn gc_decayed(&self, threshold: f64) -> StorageResult<usize> {
        self.block_on(async {
            let mut client = self.client.lock().await;
            let result = client.execute(
                "DELETE FROM memories WHERE tier = 'long' AND decay_score < @P1 AND importance <= 2",
                &[&threshold],
            ).await.map_err(|e| StorageError::Mssql(e.to_string()))?;
            Ok(result.rows_affected().iter().sum::<u64>() as usize)
        })
    }

    fn backend_name(&self) -> &'static str { "SQL Server" }
}
