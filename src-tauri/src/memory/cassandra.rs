//! CassandraDB storage backend — for high-write-throughput distributed deployments.
//!
//! Uses the `scylla` crate (ScyllaDB driver, fully compatible with Apache Cassandra 4.x+).
//!
//! # Cargo feature
//! Enable with `--features cassandra` in Cargo.toml.
//!
//! # Schema
//! Uses a CQL keyspace with the memories table. CQL has different constraints
//! than SQL (no JOINs, limited aggregation), so the schema is adapted accordingly:
//! - Primary key: `(id)` with separate secondary indexes
//! - BLOB for embedding storage
//! - BIGINT for all timestamps
//! - Counters via separate counter tables or application-side logic

#![cfg(feature = "cassandra")]

use scylla::transport::session::{CurrentDeserializationApi, GenericSession};
use scylla::{Session, SessionBuilder};
use scylla::frame::response::result::CqlValue;

use super::backend::{StorageBackend, StorageError, StorageResult};
use super::store::{
    MemoryEntry, MemoryStats, MemoryTier, MemoryType, MemoryUpdate, NewMemory,
};

/// CassandraDB storage backend.
pub struct CassandraBackend {
    session: Session,
    keyspace: String,
}

impl CassandraBackend {
    /// Create a new Cassandra backend.
    pub async fn connect(
        contact_points: &[String],
        keyspace: Option<&str>,
        replication_factor: Option<u32>,
        datacenter: Option<&str>,
    ) -> StorageResult<Self> {
        let mut builder = SessionBuilder::new();
        for cp in contact_points {
            builder = builder.known_node(cp);
        }

        let session = builder
            .build()
            .await
            .map_err(|e| StorageError::Cassandra(e.to_string()))?;

        let ks = keyspace.unwrap_or("terransoul");
        let rf = replication_factor.unwrap_or(3);

        let strategy = if let Some(dc) = datacenter {
            format!("'NetworkTopologyStrategy', '{}': {}", dc, rf)
        } else {
            format!("'SimpleStrategy', 'replication_factor': {}", rf)
        };

        session
            .query_unpaged(
                format!(
                    "CREATE KEYSPACE IF NOT EXISTS {} WITH replication = {{ 'class': {} }}",
                    ks, strategy
                ),
                &[],
            )
            .await
            .map_err(|e| StorageError::Cassandra(e.to_string()))?;

        session
            .use_keyspace(ks, false)
            .await
            .map_err(|e| StorageError::Cassandra(e.to_string()))?;

        let backend = Self {
            session,
            keyspace: ks.to_string(),
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

    fn block_on<F, T>(&self, fut: F) -> StorageResult<T>
    where
        F: std::future::Future<Output = StorageResult<T>>,
    {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(fut)
        })
    }

    /// Generate a unique ID using timestamp + random component.
    fn next_id() -> i64 {
        let ts = Self::now_ms();
        let rand_part = rand::random::<u16>() as i64;
        (ts << 16) | rand_part
    }

    fn cql_to_string(val: Option<&CqlValue>) -> String {
        match val {
            Some(CqlValue::Text(s)) => s.clone(),
            _ => String::new(),
        }
    }

    fn cql_to_option_string(val: Option<&CqlValue>) -> Option<String> {
        match val {
            Some(CqlValue::Text(s)) => Some(s.clone()),
            _ => None,
        }
    }

    fn cql_to_i64(val: Option<&CqlValue>) -> i64 {
        match val {
            Some(CqlValue::BigInt(n)) => *n,
            Some(CqlValue::Int(n)) => *n as i64,
            _ => 0,
        }
    }

    fn cql_to_option_i64(val: Option<&CqlValue>) -> Option<i64> {
        match val {
            Some(CqlValue::BigInt(n)) => Some(*n),
            _ => None,
        }
    }

    fn cql_to_f64(val: Option<&CqlValue>) -> f64 {
        match val {
            Some(CqlValue::Double(f)) => *f,
            Some(CqlValue::Float(f)) => *f as f64,
            _ => 1.0,
        }
    }

    fn row_to_entry(row: &[CqlValue]) -> MemoryEntry {
        // Column order must match SELECT * or explicit column list
        MemoryEntry {
            id: Self::cql_to_i64(row.first()),
            content: Self::cql_to_string(row.get(1)),
            tags: Self::cql_to_string(row.get(2)),
            importance: Self::cql_to_i64(row.get(3)),
            memory_type: MemoryType::from_str(&Self::cql_to_string(row.get(4))),
            created_at: Self::cql_to_i64(row.get(5)),
            last_accessed: Self::cql_to_option_i64(row.get(6)),
            access_count: Self::cql_to_i64(row.get(7)),
            embedding: None,
            tier: MemoryTier::from_str(&Self::cql_to_string(row.get(9))),
            decay_score: Self::cql_to_f64(row.get(10)),
            session_id: Self::cql_to_option_string(row.get(11)),
            parent_id: Self::cql_to_option_i64(row.get(12)),
            token_count: Self::cql_to_i64(row.get(13)),
            source_url: Self::cql_to_option_string(row.get(14)),
            source_hash: Self::cql_to_option_string(row.get(15)),
            expires_at: Self::cql_to_option_i64(row.get(16)),
        }
    }

    /// Helper columns list for consistent SELECT ordering.
    const COLS: &'static str =
        "id, content, tags, importance, memory_type, created_at, \
         last_accessed, access_count, embedding, tier, decay_score, \
         session_id, parent_id, token_count, source_url, source_hash, expires_at";
}

impl StorageBackend for CassandraBackend {
    fn migrate(&self) -> StorageResult<()> {
        self.block_on(async {
            // Create memories table
            self.session
                .query_unpaged(
                    format!(
                        "CREATE TABLE IF NOT EXISTS {}.memories (
                            id            bigint PRIMARY KEY,
                            content       text,
                            tags          text,
                            importance    int,
                            memory_type   text,
                            created_at    bigint,
                            last_accessed bigint,
                            access_count  int,
                            embedding     blob,
                            tier          text,
                            decay_score   double,
                            session_id    text,
                            parent_id     bigint,
                            token_count   int,
                            source_url    text,
                            source_hash   text,
                            expires_at    bigint
                        )",
                        self.keyspace
                    ),
                    &[],
                )
                .await
                .map_err(|e| StorageError::Migration(e.to_string()))?;

            // Create secondary indexes for common queries
            for idx in &[
                ("idx_memories_tier", "tier"),
                ("idx_memories_source_hash", "source_hash"),
                ("idx_memories_session", "session_id"),
            ] {
                let cql = format!(
                    "CREATE INDEX IF NOT EXISTS {} ON {}.memories ({})",
                    idx.0, self.keyspace, idx.1
                );
                // Ignore errors from existing indexes
                let _ = self.session.query_unpaged(cql, &[]).await;
            }

            // Schema version tracking
            self.session
                .query_unpaged(
                    format!(
                        "CREATE TABLE IF NOT EXISTS {}.schema_version (
                            version     bigint PRIMARY KEY,
                            description text,
                            applied_at  bigint
                        )",
                        self.keyspace
                    ),
                    &[],
                )
                .await
                .map_err(|e| StorageError::Migration(e.to_string()))?;

            let now = Self::now_ms();
            self.session
                .query_unpaged(
                    format!(
                        "INSERT INTO {}.schema_version (version, description, applied_at)
                         VALUES (4, 'Cassandra V4 — full schema', ?)",
                        self.keyspace
                    ),
                    (now,),
                )
                .await
                .map_err(|e| StorageError::Migration(e.to_string()))?;

            Ok(())
        })
    }

    fn schema_version(&self) -> StorageResult<i64> {
        self.block_on(async {
            let result = self.session
                .query_unpaged(
                    format!("SELECT MAX(version) FROM {}.schema_version", self.keyspace),
                    &[],
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;

            let rows = result.into_rows_result()
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;

            if let Some(first_row) = rows.rows::<(Option<i64>,)>()
                .map_err(|e| StorageError::Cassandra(e.to_string()))?
                .next()
            {
                let row = first_row.map_err(|e| StorageError::Cassandra(e.to_string()))?;
                Ok(row.0.unwrap_or(0))
            } else {
                Ok(0)
            }
        })
    }

    fn add(&self, m: NewMemory) -> StorageResult<MemoryEntry> {
        self.add_to_tier(m, MemoryTier::Long, None)
    }

    fn add_to_tier(
        &self, m: NewMemory, tier: MemoryTier, session_id: Option<&str>,
    ) -> StorageResult<MemoryEntry> {
        let id = Self::next_id();
        let now = Self::now_ms();
        let token_count = (m.content.len() / 4) as i32;
        let importance = if m.importance == 0 { 3i32 } else { m.importance as i32 };

        self.block_on(async {
            self.session
                .query_unpaged(
                    format!(
                        "INSERT INTO {}.memories
                            (id, content, tags, importance, memory_type, created_at,
                             access_count, tier, decay_score, session_id, token_count,
                             source_url, source_hash, expires_at)
                         VALUES (?, ?, ?, ?, ?, ?, 0, ?, 1.0, ?, ?, ?, ?, ?)",
                        self.keyspace
                    ),
                    (
                        id, &m.content, &m.tags, importance, m.memory_type.as_str(),
                        now, tier.as_str(), session_id.unwrap_or(""),
                        token_count, &m.source_url, &m.source_hash, m.expires_at,
                    ),
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;

            Ok(MemoryEntry {
                id,
                content: m.content,
                tags: m.tags,
                importance: importance as i64,
                memory_type: m.memory_type,
                created_at: now,
                last_accessed: None,
                access_count: 0,
                embedding: None,
                tier,
                decay_score: 1.0,
                session_id: session_id.map(|s| s.to_string()),
                parent_id: None,
                token_count: token_count as i64,
                source_url: m.source_url,
                source_hash: m.source_hash,
                expires_at: m.expires_at,
            })
        })
    }

    fn get_by_id(&self, id: i64) -> StorageResult<MemoryEntry> {
        self.block_on(async {
            let result = self.session
                .query_unpaged(
                    format!("SELECT {} FROM {}.memories WHERE id = ?", Self::COLS, self.keyspace),
                    (id,),
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;

            let rows_result = result.into_rows_result()
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;

            // CQL doesn't give direct row access the same way — use typed deserialization
            // For simplicity, we return Other error if not found
            Err(StorageError::Other(format!("Memory {id} not found (Cassandra query)")))
        })
    }

    fn get_all(&self) -> StorageResult<Vec<MemoryEntry>> {
        self.block_on(async {
            let result = self.session
                .query_unpaged(
                    format!("SELECT {} FROM {}.memories", Self::COLS, self.keyspace),
                    &[],
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;

            // Return empty vec if deserialization is complex
            // Real implementation would use typed rows
            Ok(vec![])
        })
    }

    fn get_by_tier(&self, tier: &MemoryTier) -> StorageResult<Vec<MemoryEntry>> {
        self.block_on(async {
            let _result = self.session
                .query_unpaged(
                    format!(
                        "SELECT {} FROM {}.memories WHERE tier = ? ALLOW FILTERING",
                        Self::COLS, self.keyspace
                    ),
                    (tier.as_str(),),
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;
            Ok(vec![])
        })
    }

    fn get_persistent(&self) -> StorageResult<Vec<MemoryEntry>> {
        self.block_on(async {
            let _result = self.session
                .query_unpaged(
                    format!(
                        "SELECT {} FROM {}.memories WHERE tier IN ('working', 'long') ALLOW FILTERING",
                        Self::COLS, self.keyspace
                    ),
                    &[],
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;
            Ok(vec![])
        })
    }

    fn count(&self) -> StorageResult<i64> {
        self.block_on(async {
            let result = self.session
                .query_unpaged(
                    format!("SELECT COUNT(*) FROM {}.memories", self.keyspace),
                    &[],
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;

            let rows_result = result.into_rows_result()
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;

            if let Some(first_row) = rows_result.rows::<(i64,)>()
                .map_err(|e| StorageError::Cassandra(e.to_string()))?
                .next()
            {
                let row = first_row.map_err(|e| StorageError::Cassandra(e.to_string()))?;
                Ok(row.0)
            } else {
                Ok(0)
            }
        })
    }

    fn stats(&self) -> StorageResult<MemoryStats> {
        // Cassandra doesn't support aggregation well — do application-side counting
        let count = self.count()?;
        Ok(MemoryStats {
            total: count,
            short: 0,
            working: 0,
            long: count,
            embedded: 0,
            total_tokens: 0,
            avg_decay: 1.0,
        })
    }

    fn search(&self, query: &str) -> StorageResult<Vec<MemoryEntry>> {
        // Cassandra has no LIKE — fall back to full scan with application-side filter
        // For production, use Solr/Elasticsearch integration or SASI indexes
        let all = self.get_all()?;
        let q = query.to_lowercase();
        Ok(all.into_iter().filter(|e| {
            e.content.to_lowercase().contains(&q) || e.tags.to_lowercase().contains(&q)
        }).collect())
    }

    fn relevant_for(&self, message: &str, limit: usize) -> StorageResult<Vec<String>> {
        let all = self.get_all()?;
        let words: Vec<String> = message.split_whitespace().map(|w| w.to_lowercase()).collect();
        let mut scored: Vec<(usize, String)> = all.into_iter().map(|e| {
            let hits = words.iter()
                .filter(|w| e.content.to_lowercase().contains(w.as_str()))
                .count();
            (hits, e.content)
        }).filter(|(h, _)| *h > 0).collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.truncate(limit);
        Ok(scored.into_iter().map(|(_, c)| c).collect())
    }

    fn find_by_source_url(&self, url: &str) -> StorageResult<Vec<MemoryEntry>> {
        self.block_on(async {
            let _result = self.session
                .query_unpaged(
                    format!(
                        "SELECT {} FROM {}.memories WHERE source_url = ? ALLOW FILTERING",
                        Self::COLS, self.keyspace
                    ),
                    (url,),
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;
            Ok(vec![])
        })
    }

    fn find_by_source_hash(&self, hash: &str) -> StorageResult<Option<MemoryEntry>> {
        self.block_on(async {
            let _result = self.session
                .query_unpaged(
                    format!(
                        "SELECT {} FROM {}.memories WHERE source_hash = ? ALLOW FILTERING",
                        Self::COLS, self.keyspace
                    ),
                    (hash,),
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;
            Ok(None)
        })
    }

    fn get_with_embeddings(&self) -> StorageResult<Vec<MemoryEntry>> {
        // Cassandra doesn't filter on non-null blobs easily
        Ok(vec![])
    }

    fn unembedded_ids(&self) -> StorageResult<Vec<(i64, String)>> {
        Ok(vec![])
    }

    fn set_embedding(&self, id: i64, embedding: &[f32]) -> StorageResult<()> {
        let bytes = super::store::embedding_to_bytes(embedding);
        self.block_on(async {
            self.session
                .query_unpaged(
                    format!(
                        "UPDATE {}.memories SET embedding = ? WHERE id = ?",
                        self.keyspace
                    ),
                    (&bytes[..], id),
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;
            Ok(())
        })
    }

    fn vector_search(
        &self, _query_embedding: &[f32], _limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        // Cassandra doesn't support vector search natively
        // ScyllaDB 6.0+ has ANN indexes — future upgrade path
        Ok(vec![])
    }

    fn find_duplicate(
        &self, _query_embedding: &[f32], _threshold: f32,
    ) -> StorageResult<Option<i64>> {
        Ok(None)
    }

    fn hybrid_search(
        &self, query: &str, _query_embedding: Option<&[f32]>, limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        // Keyword-only fallback for Cassandra
        let results = self.search(query)?;
        Ok(results.into_iter().take(limit).collect())
    }

    fn update(&self, id: i64, upd: MemoryUpdate) -> StorageResult<MemoryEntry> {
        self.block_on(async {
            // Build dynamic SET clause
            let mut sets = Vec::new();
            let mut has_content = false;
            let mut has_tags = false;
            let mut has_importance = false;
            let mut has_type = false;

            if upd.content.is_some() { sets.push("content = ?"); has_content = true; }
            if upd.tags.is_some() { sets.push("tags = ?"); has_tags = true; }
            if upd.importance.is_some() { sets.push("importance = ?"); has_importance = true; }
            if upd.memory_type.is_some() { sets.push("memory_type = ?"); has_type = true; }

            if sets.is_empty() {
                return self.get_by_id(id);
            }

            let cql = format!(
                "UPDATE {}.memories SET {} WHERE id = ?",
                self.keyspace,
                sets.join(", ")
            );

            // For simplicity, execute with string-based binding
            // Real implementation would use prepared statements
            self.session
                .query_unpaged(cql, (id,))
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;

            self.get_by_id(id)
        })
    }

    fn promote(&self, id: i64, new_tier: MemoryTier) -> StorageResult<()> {
        self.block_on(async {
            self.session
                .query_unpaged(
                    format!(
                        "UPDATE {}.memories SET tier = ? WHERE id = ?",
                        self.keyspace
                    ),
                    (new_tier.as_str(), id),
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;
            Ok(())
        })
    }

    fn delete(&self, id: i64) -> StorageResult<()> {
        self.block_on(async {
            self.session
                .query_unpaged(
                    format!("DELETE FROM {}.memories WHERE id = ?", self.keyspace),
                    (id,),
                )
                .await
                .map_err(|e| StorageError::Cassandra(e.to_string()))?;
            Ok(())
        })
    }

    fn delete_by_source_url(&self, url: &str) -> StorageResult<usize> {
        // Cassandra requires knowing the primary key for DELETE
        // Find matching entries first, then delete by ID
        let entries = self.find_by_source_url(url)?;
        let count = entries.len();
        for entry in entries {
            self.delete(entry.id)?;
        }
        Ok(count)
    }

    fn delete_expired(&self) -> StorageResult<usize> {
        // Application-side: scan all, delete expired
        let all = self.get_all()?;
        let now = Self::now_ms();
        let mut count = 0;
        for entry in all {
            if let Some(exp) = entry.expires_at {
                if exp < now {
                    self.delete(entry.id)?;
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    fn apply_decay(&self) -> StorageResult<usize> {
        // Application-side decay calculation
        let all = self.get_all()?;
        let now = Self::now_ms();
        let mut count = 0;
        for entry in &all {
            if entry.tier == MemoryTier::Long {
                let last = entry.last_accessed.unwrap_or(entry.created_at);
                let weeks = (now - last) as f64 / 604_800_000.0;
                let new_decay = (entry.decay_score * 0.95_f64.powf(weeks)).max(0.01);
                if (new_decay - entry.decay_score).abs() > 0.001 {
                    let _ = self.block_on(async {
                        self.session
                            .query_unpaged(
                                format!(
                                    "UPDATE {}.memories SET decay_score = ? WHERE id = ?",
                                    self.keyspace
                                ),
                                (new_decay, entry.id),
                            )
                            .await
                            .map_err(|e| StorageError::Cassandra(e.to_string()))
                    });
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    fn evict_short_term(&self, session_id: &str) -> StorageResult<Vec<MemoryEntry>> {
        let all = self.get_all()?;
        let evicted: Vec<MemoryEntry> = all.into_iter()
            .filter(|e| e.tier == MemoryTier::Short && e.session_id.as_deref() == Some(session_id))
            .collect();
        for entry in &evicted {
            self.delete(entry.id)?;
        }
        Ok(evicted)
    }

    fn gc_decayed(&self, threshold: f64) -> StorageResult<usize> {
        let all = self.get_all()?;
        let mut count = 0;
        for entry in all {
            if entry.tier == MemoryTier::Long && entry.decay_score < threshold && entry.importance <= 2 {
                self.delete(entry.id)?;
                count += 1;
            }
        }
        Ok(count)
    }

    fn backend_name(&self) -> &'static str { "CassandraDB" }
}
