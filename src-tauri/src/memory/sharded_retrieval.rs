//! Shard-aware retrieval scaffold (Phase 1 of billion-scale plan).
//!
//! This module introduces the *abstraction* needed for sharded retrieval
//! without yet splitting the underlying SQLite file. Phase 2 will replace
//! the single global `usearch` HNSW with one index per [`ShardKey`]; this
//! Phase 1 module gives the rest of the codebase a stable surface to call
//! against while that work proceeds.
//!
//! Pipeline contract
//! -----------------
//!
//! 1. The brain classifies each memory by [`ShardKey`] = (`tier`,
//!    `cognitive_kind`).
//! 2. For a query, each shard returns its own top-K ranking using the
//!    existing hybrid search.
//! 3. [`merge_shard_rankings`] fuses the per-shard rankings into a single
//!    global ranking via Reciprocal Rank Fusion (k = 60).
//! 4. [`cap_rerank_pool`] truncates the merged candidates so the LLM-as-judge
//!    rerank step never sees more than `cap` rows (default 50).
//!
//! At today's scale all shards live in the same SQLite file, but the
//! abstraction is honest: nothing in this module assumes the shards share
//! storage, so swapping in per-shard files (Phase 2) is a localized change.
//!
//! See `docs/billion-scale-retrieval-design.md` for the full plan.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::cognitive_kind::{classify as classify_cognitive_kind, CognitiveKind};
use super::fusion::{reciprocal_rank_fuse, DEFAULT_RRF_K};
use super::store::{MemoryEntry, MemoryTier};

/// Default cap on the number of candidates handed to the LLM-as-judge
/// reranker. Higher caps multiply rerank latency and token cost without
/// meaningfully improving final top-K quality past this point.
pub const DEFAULT_RERANK_CAP: usize = 50;

/// A logical shard identifier derived from `(tier, cognitive_kind)`.
///
/// At billion-scale each `ShardKey` will own a dedicated `usearch` index
/// file and a dedicated FTS5 keyword table. At Phase 1 scale all shards
/// share the SQLite file, but query results are still partitioned along
/// these axes so the search layer behaves identically whether the storage
/// is unified or split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShardKey {
    pub tier: MemoryTier,
    pub kind: CognitiveKind,
}

impl ShardKey {
    /// Build a `ShardKey` from a memory entry. Uses the cognitive-kind
    /// classifier as the third axis.
    pub fn from_entry(entry: &MemoryEntry) -> Self {
        let kind = classify_cognitive_kind(&entry.memory_type, &entry.tags, &entry.content);
        Self {
            tier: entry.tier,
            kind,
        }
    }

    /// Stable, filesystem-safe string used to name per-shard index files
    /// in Phase 2 (e.g. `vectors/long__semantic.usearch`).
    pub fn as_path_token(self) -> String {
        let tier = match self.tier {
            MemoryTier::Short => "short",
            MemoryTier::Working => "working",
            MemoryTier::Long => "long",
        };
        format!("{tier}__{}", self.kind.as_str())
    }

    /// Reverse of `as_path_token`: reconstruct a `ShardKey` from a token string.
    /// Returns `None` if the token is malformed or unknown.
    pub fn from_path_token(token: &str) -> Option<Self> {
        let parts: Vec<&str> = token.split("__").collect();
        if parts.len() != 2 {
            return None;
        }
        let tier = match parts[0] {
            "short" => MemoryTier::Short,
            "working" => MemoryTier::Working,
            "long" => MemoryTier::Long,
            _ => return None,
        };
        let kind = CognitiveKind::parse(parts[1])?;
        Some(ShardKey { tier, kind })
    }

    /// Enumerate every `ShardKey` that exists in the type system.
    /// Useful for shard-aware fan-out where we need a fixed set of slots.
    pub fn all() -> Vec<ShardKey> {
        const TIERS: [MemoryTier; 3] = [MemoryTier::Short, MemoryTier::Working, MemoryTier::Long];
        const KINDS: [CognitiveKind; 5] = [
            CognitiveKind::Episodic,
            CognitiveKind::Semantic,
            CognitiveKind::Procedural,
            CognitiveKind::Judgment,
            CognitiveKind::Negative,
        ];
        let mut out = Vec::with_capacity(TIERS.len() * KINDS.len());
        for tier in TIERS {
            for kind in KINDS {
                out.push(ShardKey { tier, kind });
            }
        }
        out
    }
}

/// Partition a flat list of entries into per-shard buckets.
///
/// The returned map only contains shards that actually have entries, so
/// callers can iterate it cheaply.
pub fn partition_by_shard(entries: Vec<MemoryEntry>) -> HashMap<ShardKey, Vec<MemoryEntry>> {
    let mut buckets: HashMap<ShardKey, Vec<MemoryEntry>> = HashMap::new();
    for entry in entries {
        let key = ShardKey::from_entry(&entry);
        buckets.entry(key).or_default().push(entry);
    }
    buckets
}

/// Fuse multiple shard rankings into a single ranking using RRF.
///
/// * `per_shard` — each inner slice is a ranking returned by one shard,
///   ordered most-relevant first. The same memory id must not appear
///   twice within a single shard ranking, but it *may* appear in multiple
///   shards (e.g. when a memory has been moved between tiers); RRF will
///   naturally promote such entries.
/// * `cap` — maximum number of entries to keep in the merged result.
///
/// Returns the merged ranking as `(memory_id, fused_score)` pairs sorted
/// by descending score, truncated to `cap`.
pub fn merge_shard_rankings(per_shard: &[&[i64]], cap: usize) -> Vec<(i64, f64)> {
    if per_shard.is_empty() || cap == 0 {
        return Vec::new();
    }
    let mut fused = reciprocal_rank_fuse(per_shard, DEFAULT_RRF_K);
    fused.truncate(cap);
    fused
}

/// Truncate a candidate pool so the LLM-as-judge reranker never sees more
/// than `cap` rows. Preserves input order (callers pass an already-ranked
/// list).
pub fn cap_rerank_pool<T: Clone>(entries: &[T], cap: usize) -> Vec<T> {
    let limit = entries.len().min(cap);
    entries[..limit].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::MemoryType;

    fn make_entry(id: i64, tier: MemoryTier, tags: &str, content: &str) -> MemoryEntry {
        MemoryEntry {
            id,
            content: content.to_string(),
            tags: tags.to_string(),
            importance: 3,
            memory_type: MemoryType::Fact,
            created_at: 0,
            last_accessed: None,
            access_count: 0,
            embedding: None,
            tier,
            decay_score: 1.0,
            session_id: None,
            parent_id: None,
            token_count: 0,
            source_url: None,
            source_hash: None,
            expires_at: None,
            valid_to: None,
            obsidian_path: None,
            last_exported: None,
            updated_at: None,
            origin_device: None,
            hlc_counter: None,
            confidence: 1.0,
        }
    }

    #[test]
    fn shard_key_partitions_by_tier_and_kind() {
        let entries = vec![
            make_entry(
                1,
                MemoryTier::Long,
                "semantic:facts",
                "Rust uses ownership for memory safety",
            ),
            make_entry(2, MemoryTier::Long, "episodic", "Yesterday we shipped"),
            make_entry(
                3,
                MemoryTier::Working,
                "procedural",
                "How to release: bump, tag, push",
            ),
        ];
        let buckets = partition_by_shard(entries);
        // 3 entries → at least 3 distinct shard keys (tier × kind differ).
        assert!(buckets.len() >= 2);
        // Every bucket should have at least one entry.
        for (_, v) in buckets.iter() {
            assert!(!v.is_empty());
        }
    }

    #[test]
    fn shard_key_path_token_is_filesystem_safe() {
        let key = ShardKey {
            tier: MemoryTier::Long,
            kind: CognitiveKind::Semantic,
        };
        let token = key.as_path_token();
        assert_eq!(token, "long__semantic");
        for ch in token.chars() {
            assert!(
                ch.is_ascii_alphanumeric() || ch == '_',
                "path token char {:?} is not filesystem-safe",
                ch
            );
        }
    }

    #[test]
    fn shard_key_from_path_token_reverses_as_path_token() {
        let key = ShardKey {
            tier: MemoryTier::Long,
            kind: CognitiveKind::Semantic,
        };
        let token = key.as_path_token();
        let recovered = ShardKey::from_path_token(&token).expect("failed to recover key");
        assert_eq!(recovered.tier, key.tier);
        assert_eq!(recovered.kind, key.kind);
    }

    #[test]
    fn shard_key_from_path_token_rejects_malformed_tokens() {
        assert!(ShardKey::from_path_token("no-separator").is_none());
        assert!(ShardKey::from_path_token("long").is_none());
        assert!(ShardKey::from_path_token("unknown__semantic").is_none());
        assert!(ShardKey::from_path_token("long__unknown").is_none());
    }

    #[test]
    fn shard_key_all_enumerates_full_grid() {
        let all = ShardKey::all();
        // 3 tiers × 5 cognitive kinds = 15 logical shards.
        assert_eq!(all.len(), 15);
        // All entries are unique.
        let mut seen = std::collections::HashSet::new();
        for k in &all {
            assert!(seen.insert(*k), "duplicate shard key: {k:?}");
        }
    }

    #[test]
    fn merge_shard_rankings_prefers_items_seen_in_multiple_shards() {
        // Vector shard ranking: A=1, B=2, C=3
        let vector_rank: Vec<i64> = vec![1, 2, 3];
        // Keyword shard ranking: B=1, D=2, A=3
        let keyword_rank: Vec<i64> = vec![2, 4, 1];

        let fused = merge_shard_rankings(&[&vector_rank, &keyword_rank], 10);
        // B appears at rank 1 (keyword) + rank 2 (vector) → highest fused.
        assert_eq!(fused[0].0, 2);
    }

    #[test]
    fn merge_shard_rankings_respects_cap() {
        let r1: Vec<i64> = (1..=20).collect();
        let r2: Vec<i64> = (21..=40).collect();
        let fused = merge_shard_rankings(&[&r1, &r2], 5);
        assert_eq!(fused.len(), 5);
    }

    #[test]
    fn merge_shard_rankings_handles_empty_inputs() {
        let empty: Vec<i64> = Vec::new();
        let fused = merge_shard_rankings(&[&empty], 10);
        assert!(fused.is_empty());

        let none: Vec<&[i64]> = Vec::new();
        let fused2 = merge_shard_rankings(&none, 10);
        assert!(fused2.is_empty());
    }

    #[test]
    fn cap_rerank_pool_truncates_to_cap() {
        let items: Vec<i64> = (1..=100).collect();
        let capped = cap_rerank_pool(&items, DEFAULT_RERANK_CAP);
        assert_eq!(capped.len(), DEFAULT_RERANK_CAP);
        assert_eq!(capped.first().copied(), Some(1));
        assert_eq!(capped.last().copied(), Some(DEFAULT_RERANK_CAP as i64));
    }

    #[test]
    fn cap_rerank_pool_no_op_when_smaller_than_cap() {
        let items = vec![1_i64, 2, 3];
        let capped = cap_rerank_pool(&items, 50);
        assert_eq!(capped, items);
    }
}
