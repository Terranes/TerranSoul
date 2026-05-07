//! Bounded KG traversal with LRU cache (Chunk 41.13).
//!
//! Provides a multi-hop BFS over the memory knowledge graph, bounded by
//! maximum depth and per-hop fan-out. Results are cached in an LRU map
//! keyed by `(seed_id, depth, direction)` and invalidated when edges
//! involving cached node IDs are mutated.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

use super::edges::EdgeDirection;
use super::MemoryEdge;

/// Maximum allowed traversal depth (hard cap regardless of request).
pub const MAX_DEPTH: u8 = 3;

/// Maximum neighbours expanded per hop (fan-out cap).
pub const MAX_FAN_OUT: usize = 50;

/// Default LRU cache capacity (number of unique traversal results cached).
pub const DEFAULT_CACHE_CAPACITY: usize = 64;

/// Cache key for a KG traversal result.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KgCacheKey {
    pub seed_id: i64,
    pub depth: u8,
    pub direction: EdgeDirection,
}

/// A single hop result: the edges discovered at that depth level.
#[derive(Debug, Clone)]
pub struct HopResult {
    pub depth: u8,
    pub edges: Vec<MemoryEdge>,
}

/// Full traversal result returned by `bounded_bfs`.
#[derive(Debug, Clone)]
pub struct TraversalResult {
    /// All hops in order (depth 1, depth 2, ...).
    pub hops: Vec<HopResult>,
    /// All unique node IDs touched during traversal (for cache invalidation).
    pub touched_ids: HashSet<i64>,
    /// Whether the traversal was truncated (depth exceeded MAX_DEPTH or
    /// fan-out capped).
    pub truncated: bool,
}

/// LRU cache for KG traversal results.
///
/// Thread-safe via internal `Mutex`. The LRU eviction is approximate:
/// on insert, if capacity is exceeded, the oldest entry is removed.
/// This is intentionally simple — a full LRU linked list is overkill
/// for 256 entries with sub-microsecond key hashing.
pub struct KgCache {
    inner: Mutex<KgCacheInner>,
}

struct KgCacheInner {
    /// Ordered map: most recently used at the end.
    entries: Vec<(KgCacheKey, TraversalResult)>,
    capacity: usize,
    /// Reverse index: node_id → set of cache keys that include it.
    /// Used for O(1) invalidation lookups.
    node_to_keys: HashMap<i64, HashSet<KgCacheKey>>,
}

impl KgCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(KgCacheInner {
                entries: Vec::with_capacity(capacity),
                capacity,
                node_to_keys: HashMap::new(),
            }),
        }
    }

    /// Look up a cached traversal result. Returns `None` on miss.
    /// Moves the entry to the most-recently-used position on hit.
    pub fn get(&self, key: &KgCacheKey) -> Option<TraversalResult> {
        let mut inner = self.inner.lock().ok()?;
        let pos = inner.entries.iter().position(|(k, _)| k == key)?;
        let entry = inner.entries.remove(pos);
        let result = entry.1.clone();
        inner.entries.push(entry);
        Some(result)
    }

    /// Insert a traversal result into the cache. Evicts the LRU entry
    /// if capacity is exceeded.
    pub fn insert(&self, key: KgCacheKey, result: TraversalResult) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        // Remove existing entry for this key if present.
        if let Some(pos) = inner.entries.iter().position(|(k, _)| k == &key) {
            let (old_key, old_result) = inner.entries.remove(pos);
            // Clean up reverse index.
            for id in &old_result.touched_ids {
                if let Some(set) = inner.node_to_keys.get_mut(id) {
                    set.remove(&old_key);
                }
            }
        }
        // Evict LRU if at capacity.
        while inner.entries.len() >= inner.capacity {
            if let Some((evicted_key, evicted_result)) = inner.entries.first().cloned() {
                inner.entries.remove(0);
                for id in &evicted_result.touched_ids {
                    if let Some(set) = inner.node_to_keys.get_mut(id) {
                        set.remove(&evicted_key);
                    }
                }
            } else {
                break;
            }
        }
        // Build reverse index entries.
        for id in &result.touched_ids {
            inner
                .node_to_keys
                .entry(*id)
                .or_default()
                .insert(key.clone());
        }
        inner.entries.push((key, result));
    }

    /// Invalidate all cached traversals that touch the given node IDs.
    /// Called when edges are added/removed for these endpoints.
    pub fn invalidate(&self, node_ids: &[i64]) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let mut keys_to_remove: HashSet<KgCacheKey> = HashSet::new();
        for id in node_ids {
            if let Some(keys) = inner.node_to_keys.remove(id) {
                keys_to_remove.extend(keys);
            }
        }
        if keys_to_remove.is_empty() {
            return;
        }
        // Collect touched_ids from entries being removed for reverse-index cleanup.
        let mut ids_to_cleanup: Vec<(KgCacheKey, HashSet<i64>)> = Vec::new();
        inner.entries.retain(|(k, result)| {
            if keys_to_remove.contains(k) {
                ids_to_cleanup.push((k.clone(), result.touched_ids.clone()));
                false
            } else {
                true
            }
        });
        // Clean up reverse index for remaining touched IDs.
        for (removed_key, touched) in &ids_to_cleanup {
            for id in touched {
                if let Some(set) = inner.node_to_keys.get_mut(id) {
                    set.remove(removed_key);
                }
            }
        }
    }

    /// Current number of cached entries.
    pub fn len(&self) -> usize {
        self.inner.lock().map(|i| i.entries.len()).unwrap_or(0)
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Perform a bounded BFS traversal over the knowledge graph.
///
/// - `seed_id`: starting node
/// - `depth`: number of hops (capped at `MAX_DEPTH`)
/// - `direction`: edge direction filter
/// - `get_edges`: closure that fetches edges for a given node
///
/// Returns a `TraversalResult` with all hops and touched node IDs.
pub fn bounded_bfs<F>(
    seed_id: i64,
    depth: u8,
    direction: EdgeDirection,
    get_edges: F,
) -> TraversalResult
where
    F: Fn(i64, EdgeDirection) -> Vec<MemoryEdge>,
{
    let effective_depth = depth.min(MAX_DEPTH);
    let mut visited: HashSet<i64> = HashSet::new();
    visited.insert(seed_id);
    let mut frontier: VecDeque<i64> = VecDeque::new();
    frontier.push_back(seed_id);
    let mut hops: Vec<HopResult> = Vec::new();
    let mut truncated = depth > MAX_DEPTH;

    for current_depth in 1..=effective_depth {
        let frontier_size = frontier.len();
        let mut next_frontier: VecDeque<i64> = VecDeque::new();
        let mut hop_edges: Vec<MemoryEdge> = Vec::new();

        for _ in 0..frontier_size {
            let Some(node_id) = frontier.pop_front() else {
                break;
            };
            let edges = get_edges(node_id, direction);
            let edges_capped: Vec<MemoryEdge> = edges
                .into_iter()
                .take(MAX_FAN_OUT)
                .collect();

            if edges_capped.len() >= MAX_FAN_OUT {
                truncated = true;
            }

            for edge in &edges_capped {
                let other_id = if edge.src_id == node_id {
                    edge.dst_id
                } else {
                    edge.src_id
                };
                if visited.insert(other_id) {
                    next_frontier.push_back(other_id);
                }
            }
            hop_edges.extend(edges_capped);
        }

        if !hop_edges.is_empty() {
            hops.push(HopResult {
                depth: current_depth,
                edges: hop_edges,
            });
        }
        frontier = next_frontier;
        if frontier.is_empty() {
            break;
        }
    }

    TraversalResult {
        hops,
        touched_ids: visited,
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::edges::EdgeSource;

    fn make_edge(id: i64, src: i64, dst: i64, rel: &str) -> MemoryEdge {
        MemoryEdge {
            id,
            src_id: src,
            dst_id: dst,
            rel_type: rel.to_string(),
            confidence: 1.0,
            source: EdgeSource::User,
            created_at: 1000,
            valid_from: None,
            valid_to: None,
            edge_source: None,
        }
    }

    #[test]
    fn bounded_bfs_single_hop() {
        // A -> B, A -> C
        let edges = [make_edge(1, 10, 20, "related_to"), make_edge(2, 10, 30, "related_to")];
        let result = bounded_bfs(10, 1, EdgeDirection::Both, |id, _dir| {
            edges
                .iter()
                .filter(|e| e.src_id == id || e.dst_id == id)
                .cloned()
                .collect()
        });
        assert_eq!(result.hops.len(), 1);
        assert_eq!(result.hops[0].edges.len(), 2);
        assert!(!result.truncated);
        assert!(result.touched_ids.contains(&10));
        assert!(result.touched_ids.contains(&20));
        assert!(result.touched_ids.contains(&30));
    }

    #[test]
    fn bounded_bfs_multi_hop() {
        // A -> B -> C (chain)
        let edges = [make_edge(1, 10, 20, "related_to"), make_edge(2, 20, 30, "cites")];
        let result = bounded_bfs(10, 2, EdgeDirection::Both, |id, _dir| {
            edges
                .iter()
                .filter(|e| e.src_id == id || e.dst_id == id)
                .cloned()
                .collect()
        });
        assert_eq!(result.hops.len(), 2);
        assert_eq!(result.hops[0].edges.len(), 1); // A->B
        // Hop 2 expands node 20: returns all incident edges (back to 10 + forward to 30)
        assert_eq!(result.hops[1].edges.len(), 2);
        assert!(result.touched_ids.contains(&30));
    }

    #[test]
    fn bounded_bfs_respects_max_depth() {
        // Chain of 5 nodes but depth requested = 10 (capped to MAX_DEPTH=3)
        let edges: Vec<MemoryEdge> = (0..5)
            .map(|i| make_edge(i + 1, i * 10, (i + 1) * 10, "chain"))
            .collect();
        let result = bounded_bfs(0, 10, EdgeDirection::Both, |id, _dir| {
            edges
                .iter()
                .filter(|e| e.src_id == id || e.dst_id == id)
                .cloned()
                .collect()
        });
        // Should only traverse up to MAX_DEPTH hops.
        assert!(result.hops.len() <= MAX_DEPTH as usize);
        assert!(result.truncated);
    }

    #[test]
    fn bounded_bfs_respects_fan_out() {
        // Node with MAX_FAN_OUT + 10 edges — should be capped.
        let edges: Vec<MemoryEdge> = (0..(MAX_FAN_OUT + 10))
            .map(|i| make_edge(i as i64, 1, (i + 100) as i64, "many"))
            .collect();
        let result = bounded_bfs(1, 1, EdgeDirection::Both, |id, _dir| {
            edges
                .iter()
                .filter(|e| e.src_id == id || e.dst_id == id)
                .cloned()
                .collect()
        });
        assert_eq!(result.hops[0].edges.len(), MAX_FAN_OUT);
        assert!(result.truncated);
    }

    #[test]
    fn cache_hit_and_miss() {
        let cache = KgCache::new(4);
        let key = KgCacheKey {
            seed_id: 1,
            depth: 1,
            direction: EdgeDirection::Both,
        };
        assert!(cache.get(&key).is_none());

        let result = TraversalResult {
            hops: vec![],
            touched_ids: [1, 2, 3].into_iter().collect(),
            truncated: false,
        };
        cache.insert(key.clone(), result.clone());
        let hit = cache.get(&key).unwrap();
        assert_eq!(hit.touched_ids, result.touched_ids);
    }

    #[test]
    fn cache_invalidation() {
        let cache = KgCache::new(4);
        let key = KgCacheKey {
            seed_id: 1,
            depth: 1,
            direction: EdgeDirection::Both,
        };
        let result = TraversalResult {
            hops: vec![],
            touched_ids: [1, 2, 3].into_iter().collect(),
            truncated: false,
        };
        cache.insert(key.clone(), result);
        assert_eq!(cache.len(), 1);

        // Invalidate by a touched node.
        cache.invalidate(&[2]);
        assert_eq!(cache.len(), 0);
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn cache_evicts_lru() {
        let cache = KgCache::new(2);
        let result = TraversalResult {
            hops: vec![],
            touched_ids: HashSet::new(),
            truncated: false,
        };
        let k1 = KgCacheKey { seed_id: 1, depth: 1, direction: EdgeDirection::Both };
        let k2 = KgCacheKey { seed_id: 2, depth: 1, direction: EdgeDirection::Both };
        let k3 = KgCacheKey { seed_id: 3, depth: 1, direction: EdgeDirection::Both };

        cache.insert(k1.clone(), result.clone());
        cache.insert(k2.clone(), result.clone());
        assert_eq!(cache.len(), 2);

        // Insert a third — should evict k1 (LRU).
        cache.insert(k3.clone(), result);
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&k1).is_none());
        assert!(cache.get(&k2).is_some());
        assert!(cache.get(&k3).is_some());
    }

    #[test]
    fn invalidation_does_not_affect_unrelated_entries() {
        let cache = KgCache::new(4);
        let k1 = KgCacheKey { seed_id: 1, depth: 1, direction: EdgeDirection::Both };
        let k2 = KgCacheKey { seed_id: 2, depth: 1, direction: EdgeDirection::Both };
        let r1 = TraversalResult {
            hops: vec![],
            touched_ids: [1, 10].into_iter().collect(),
            truncated: false,
        };
        let r2 = TraversalResult {
            hops: vec![],
            touched_ids: [2, 20].into_iter().collect(),
            truncated: false,
        };
        cache.insert(k1.clone(), r1);
        cache.insert(k2.clone(), r2);

        // Invalidate node 10 — should only remove k1.
        cache.invalidate(&[10]);
        assert!(cache.get(&k1).is_none());
        assert!(cache.get(&k2).is_some());
    }
}
