//! Shard backpressure + health surface (Chunk 48.7).
//!
//! Prevents ingests from degrading search quality by rejecting writes when a
//! shard exceeds its capacity threshold. Also provides per-shard health
//! reporting wired into `brain_health`.
//!
//! **Backpressure strategy:**
//! - Each logical shard (ShardKey = tier × cognitive_kind) has a soft cap
//!   (`SHARD_MAX_ENTRIES`). When a shard would exceed the cap, the ingest
//!   path returns an `Err` signalling the caller to split/rebalance.
//! - The cap is configurable per-store (default 2M entries per shard,
//!   giving ~30M total across 15 shards).
//!
//! **Health surface:**
//! - `ShardHealth` reports per-shard status: entry count, whether the FTS5
//!   index is present, whether the ANN index file exists, and whether the
//!   shard is over-capacity.
//! - `brain_health` includes the aggregate shard health so the search layer
//!   never silently returns partial results.

use rusqlite::{params, Result as SqlResult};
use serde::{Deserialize, Serialize};

use super::sharded_retrieval::ShardKey;
use super::store::MemoryStore;

/// Default maximum entries per logical shard before backpressure kicks in.
/// At 768-dim f32 vectors, 2M entries ≈ 6 GB of raw vectors per shard.
pub const DEFAULT_SHARD_MAX_ENTRIES: i64 = 2_000_000;

/// Per-shard health status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShardHealth {
    /// Shard identifier (e.g. "long__semantic").
    pub shard: String,
    /// Number of entries in this shard.
    pub entry_count: i64,
    /// Maximum allowed entries (backpressure threshold).
    pub max_entries: i64,
    /// Whether FTS5 keyword index is available for this shard.
    pub fts5_available: bool,
    /// Whether the vector ANN index file exists on disk.
    pub ann_index_exists: bool,
    /// Whether the shard is at or over capacity.
    pub over_capacity: bool,
    /// Human-readable status label.
    pub status: String,
}

/// Aggregate health across all shards.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ShardHealthSummary {
    /// Per-shard health entries.
    pub shards: Vec<ShardHealth>,
    /// Total number of shards that are over capacity.
    pub over_capacity_count: usize,
    /// Total number of shards with missing indexes.
    pub degraded_count: usize,
    /// Overall status: "healthy", "degraded", or "over_capacity".
    pub overall: String,
}

/// Backpressure rejection error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureError {
    pub shard: String,
    pub current_count: i64,
    pub max_entries: i64,
    pub message: String,
}

impl std::fmt::Display for BackpressureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl MemoryStore {
    /// Check whether the shard for the given ShardKey has capacity for more entries.
    /// Returns `Ok(())` if under the limit, or `Err(BackpressureError)` if at/over capacity.
    pub fn check_shard_capacity(
        &self,
        shard: &ShardKey,
        max_entries: i64,
    ) -> Result<(), BackpressureError> {
        let conn = self.conn();
        let tier_str = shard.tier.as_str();
        let kind_str = shard.kind.as_str();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories
                 WHERE tier = ?1 AND COALESCE(cognitive_kind, 'semantic') = ?2",
                params![tier_str, kind_str],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if count >= max_entries {
            Err(BackpressureError {
                shard: shard.as_path_token(),
                current_count: count,
                max_entries,
                message: format!(
                    "Shard {} is at capacity ({}/{} entries). Split or rebalance required.",
                    shard.as_path_token(),
                    count,
                    max_entries
                ),
            })
        } else {
            Ok(())
        }
    }

    /// Count entries in a specific shard.
    pub fn shard_entry_count(&self, shard: &ShardKey) -> SqlResult<i64> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM memories
             WHERE tier = ?1 AND COALESCE(cognitive_kind, 'semantic') = ?2",
            params![shard.tier.as_str(), shard.kind.as_str()],
            |row| row.get(0),
        )
    }

    /// Compute health status for all active shards (those with ≥ 1 entry).
    /// The `max_entries` parameter sets the per-shard capacity threshold.
    pub fn shard_health_summary(&self, max_entries: i64) -> SqlResult<ShardHealthSummary> {
        let conn = self.conn();
        let fts5_available = self.has_fts5();

        // Query actual shard populations.
        let mut stmt = conn.prepare(
            "SELECT tier, COALESCE(cognitive_kind, 'semantic') AS kind, COUNT(*) AS cnt
             FROM memories
             GROUP BY tier, kind
             ORDER BY cnt DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;

        let mut shards = Vec::new();
        let mut over_capacity_count = 0;
        let mut degraded_count = 0;

        for row in rows {
            let (tier, kind, count) = row?;
            let shard_token = format!("{tier}__{kind}");
            let over_cap = count >= max_entries;
            if over_cap {
                over_capacity_count += 1;
            }

            // Check if ANN index file exists (only for disk-based stores).
            let ann_exists = self.ann_index_exists_for_token(&shard_token);
            if !fts5_available || !ann_exists {
                degraded_count += 1;
            }

            let status = if over_cap {
                "over_capacity".to_string()
            } else if !fts5_available || !ann_exists {
                "degraded".to_string()
            } else {
                "healthy".to_string()
            };

            shards.push(ShardHealth {
                shard: shard_token,
                entry_count: count,
                max_entries,
                fts5_available,
                ann_index_exists: ann_exists,
                over_capacity: over_cap,
                status,
            });
        }

        let overall = if over_capacity_count > 0 {
            "over_capacity".to_string()
        } else if degraded_count > 0 {
            "degraded".to_string()
        } else {
            "healthy".to_string()
        };

        Ok(ShardHealthSummary {
            shards,
            over_capacity_count,
            degraded_count,
            overall,
        })
    }

    /// Pre-ingest capacity check for the entire `long` tier.
    ///
    /// Bulk ingest pipelines call this before `add_many()`. If the
    /// aggregate long-tier entry count plus the incoming batch size
    /// would exceed `max_entries * num_long_shards`, the ingest is
    /// rejected. This is a coarse gate — individual shard-level checks
    /// run in `shard_health_summary()` for diagnostics.
    pub fn check_ingest_capacity(&self, batch_size: usize) -> Result<(), BackpressureError> {
        let conn = self.conn();
        let long_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE tier = 'long'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // 5 cognitive kinds × 1 tier = 5 long shards.
        let total_long_capacity = DEFAULT_SHARD_MAX_ENTRIES * 5;
        let projected = long_count + batch_size as i64;

        if projected > total_long_capacity {
            Err(BackpressureError {
                shard: "long__*".to_string(),
                current_count: long_count,
                max_entries: total_long_capacity,
                message: format!(
                    "Ingest of {} entries would exceed long-tier capacity ({}/{} total). \
                     Split or rebalance required.",
                    batch_size, long_count, total_long_capacity
                ),
            })
        } else {
            Ok(())
        }
    }

    /// Check if an ANN index file exists for a given shard token.
    /// For in-memory stores (tests), always returns true.
    fn ann_index_exists_for_token(&self, shard_token: &str) -> bool {
        match self.data_dir() {
            Some(dir) => {
                let path = dir.join("vectors").join(format!("{shard_token}.usearch"));
                path.exists()
            }
            None => true, // In-memory stores always "have" their ANN
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::cognitive_kind::CognitiveKind;
    use crate::memory::store::{MemoryTier, MemoryType, NewMemory};

    #[test]
    fn check_shard_capacity_under_limit() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                content: "test entry".to_string(),
                tags: "test".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        let shard = ShardKey {
            tier: MemoryTier::Long,
            kind: CognitiveKind::Semantic,
        };
        assert!(store.check_shard_capacity(&shard, 1000).is_ok());
    }

    #[test]
    fn check_shard_capacity_at_limit_rejects() {
        let store = MemoryStore::in_memory();
        // Insert 5 entries (all default to Long tier, classified as Semantic for short content)
        for i in 0..5 {
            store
                .add(NewMemory {
                    content: format!("entry {i}"),
                    tags: "test".to_string(),
                    importance: 3,
                    memory_type: MemoryType::Fact,
                    ..Default::default()
                })
                .unwrap();
        }

        let shard = ShardKey {
            tier: MemoryTier::Long,
            kind: CognitiveKind::Semantic,
        };
        let result = store.check_shard_capacity(&shard, 5);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("at capacity"));
        assert_eq!(err.current_count, 5);
    }

    #[test]
    fn shard_health_summary_reports_healthy() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                content: "test".to_string(),
                tags: "t".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        let summary = store
            .shard_health_summary(DEFAULT_SHARD_MAX_ENTRIES)
            .unwrap();
        assert_eq!(summary.overall, "healthy");
        assert_eq!(summary.over_capacity_count, 0);
        assert!(!summary.shards.is_empty());
    }

    #[test]
    fn shard_health_summary_detects_over_capacity() {
        let store = MemoryStore::in_memory();
        for i in 0..10 {
            store
                .add(NewMemory {
                    content: format!("entry {i}"),
                    tags: "t".to_string(),
                    importance: 3,
                    memory_type: MemoryType::Fact,
                    ..Default::default()
                })
                .unwrap();
        }

        // Set a very low cap to trigger over_capacity.
        let summary = store.shard_health_summary(5).unwrap();
        assert_eq!(summary.overall, "over_capacity");
        assert!(summary.over_capacity_count >= 1);
    }

    #[test]
    fn shard_entry_count_returns_correct_count() {
        let store = MemoryStore::in_memory();
        for idx in 0..3 {
            store
                .add(NewMemory {
                    content: format!("test {idx}"),
                    tags: "t".to_string(),
                    importance: 3,
                    memory_type: MemoryType::Fact,
                    ..Default::default()
                })
                .unwrap();
        }

        let shard = ShardKey {
            tier: MemoryTier::Long,
            kind: CognitiveKind::Semantic,
        };
        let count = store.shard_entry_count(&shard).unwrap();
        // All entries go to Long tier, classified as semantic for short content
        assert!(count >= 3);
    }

    #[test]
    fn check_ingest_capacity_under_limit() {
        let store = MemoryStore::in_memory();
        // Empty store, batch of 10 should be fine
        assert!(store.check_ingest_capacity(10).is_ok());
    }

    #[test]
    fn check_ingest_capacity_rejects_when_full() {
        let store = MemoryStore::in_memory();
        // 5 long shards × 2M = 10M total capacity
        // A batch that would exceed this is rejected
        let total_cap = (DEFAULT_SHARD_MAX_ENTRIES * 5) as usize;
        let result = store.check_ingest_capacity(total_cap + 1);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("would exceed"));
    }
}
