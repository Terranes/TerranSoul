//! Query result cache for RAG retrieval (Chunk 44.1).
//!
//! LRU cache of search results keyed by (query, mode, limit). Entries
//! expire on a configurable TTL and are globally invalidated on any
//! write to the memory store (add/update/delete/set_embedding).
//!
//! This avoids redundant embedding + candidate + RRF + scoring work
//! for repeated or near-identical queries within a short window (common
//! in MCP tool calls and streaming chat where the same RAG context is
//! fetched multiple times).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Default cache capacity (number of unique queries cached).
pub const DEFAULT_CAPACITY: usize = 64;

/// Default TTL in milliseconds (30 seconds).
pub const DEFAULT_TTL_MS: u64 = 30_000;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Cache key: combination of query text, search mode, and result limit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SearchCacheKey {
    pub query: String,
    pub mode: String, // "hybrid", "rrf", "hyde"
    pub limit: usize,
}

/// Cached entry with timestamp for TTL expiration.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Serialized result (memory IDs + scores).
    pub hits: Vec<CachedHit>,
    /// When this entry was stored.
    pub stored_at: Instant,
}

/// Minimal cached hit data (avoids storing full MemoryEntry).
#[derive(Debug, Clone)]
pub struct CachedHit {
    pub memory_id: i64,
    pub score: f64,
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// Thread-safe LRU search result cache.
pub struct SearchCache {
    entries: Mutex<HashMap<SearchCacheKey, CacheEntry>>,
    capacity: usize,
    ttl_ms: u64,
    /// Monotonic generation counter. Incremented on every store mutation.
    /// When a cached entry's generation is behind, it's stale.
    generation: AtomicU64,
    /// Generation at which each entry was stored.
    entry_generations: Mutex<HashMap<SearchCacheKey, u64>>,
    // Stats
    hits: AtomicU64,
    misses: AtomicU64,
}

impl SearchCache {
    /// Create a new cache with the given capacity and TTL.
    pub fn new(capacity: usize, ttl_ms: u64) -> Self {
        Self {
            entries: Mutex::new(HashMap::with_capacity(capacity)),
            capacity,
            ttl_ms,
            generation: AtomicU64::new(0),
            entry_generations: Mutex::new(HashMap::with_capacity(capacity)),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Look up a cached result. Returns `None` if not found, expired,
    /// or invalidated by a store mutation.
    pub fn get(&self, key: &SearchCacheKey) -> Option<Vec<CachedHit>> {
        let entries = self.entries.lock().ok()?;
        let entry = match entries.get(key) {
            Some(e) => e,
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };

        // Check TTL.
        if entry.stored_at.elapsed().as_millis() as u64 > self.ttl_ms {
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        // Check generation (invalidation).
        let gen = self.entry_generations.lock().ok()?;
        let entry_gen = gen.get(key).copied().unwrap_or(0);
        let current_gen = self.generation.load(Ordering::Relaxed);
        if entry_gen < current_gen {
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        self.hits.fetch_add(1, Ordering::Relaxed);
        Some(entry.hits.clone())
    }

    /// Store a search result in the cache.
    pub fn put(&self, key: SearchCacheKey, hits: Vec<CachedHit>) {
        let mut entries = match self.entries.lock() {
            Ok(e) => e,
            Err(_) => return,
        };

        // Evict oldest if at capacity.
        if entries.len() >= self.capacity && !entries.contains_key(&key) {
            Self::evict_oldest(&mut entries);
        }

        let current_gen = self.generation.load(Ordering::Relaxed);
        entries.insert(
            key.clone(),
            CacheEntry {
                hits,
                stored_at: Instant::now(),
            },
        );

        if let Ok(mut gens) = self.entry_generations.lock() {
            gens.insert(key, current_gen);
        }
    }

    /// Invalidate all cached entries by bumping the generation counter.
    /// Called on any store write operation.
    pub fn invalidate(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Cache statistics.
    pub fn stats(&self) -> CacheStats {
        let entries = self.entries.lock().map(|e| e.len()).unwrap_or(0);
        CacheStats {
            entries,
            capacity: self.capacity,
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            generation: self.generation.load(Ordering::Relaxed),
        }
    }

    /// Remove all entries.
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
        if let Ok(mut gens) = self.entry_generations.lock() {
            gens.clear();
        }
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Evict the oldest entry (by stored_at time).
    fn evict_oldest(entries: &mut HashMap<SearchCacheKey, CacheEntry>) {
        if let Some(oldest_key) = entries
            .iter()
            .min_by_key(|(_, v)| v.stored_at)
            .map(|(k, _)| k.clone())
        {
            entries.remove(&oldest_key);
        }
    }
}

impl Default for SearchCache {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY, DEFAULT_TTL_MS)
    }
}

// SAFETY: all interior mutability via Mutex + AtomicU64 — Send + Sync.
unsafe impl Send for SearchCache {}
unsafe impl Sync for SearchCache {}

/// Process-wide singleton search cache.
pub static SEARCH_CACHE: std::sync::LazyLock<SearchCache> =
    std::sync::LazyLock::new(SearchCache::default);

/// Cache statistics for observability.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    pub entries: usize,
    pub capacity: usize,
    pub hits: u64,
    pub misses: u64,
    pub generation: u64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn key(q: &str) -> SearchCacheKey {
        SearchCacheKey {
            query: q.to_string(),
            mode: "rrf".to_string(),
            limit: 10,
        }
    }

    fn hits(ids: &[i64]) -> Vec<CachedHit> {
        ids.iter()
            .enumerate()
            .map(|(i, &id)| CachedHit {
                memory_id: id,
                score: 1.0 - (i as f64 * 0.1),
            })
            .collect()
    }

    #[test]
    fn put_and_get() {
        let cache = SearchCache::default();
        let k = key("hello world");
        cache.put(k.clone(), hits(&[1, 2, 3]));
        let result = cache.get(&k).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].memory_id, 1);
    }

    #[test]
    fn miss_on_unknown_key() {
        let cache = SearchCache::default();
        assert!(cache.get(&key("not stored")).is_none());
    }

    #[test]
    fn invalidation_clears_on_write() {
        let cache = SearchCache::default();
        let k = key("test");
        cache.put(k.clone(), hits(&[10]));
        assert!(cache.get(&k).is_some());

        cache.invalidate();
        assert!(cache.get(&k).is_none());
    }

    #[test]
    fn ttl_expiration() {
        let cache = SearchCache::new(128, 10); // 10ms TTL
        let k = key("short-lived");
        cache.put(k.clone(), hits(&[5]));
        thread::sleep(Duration::from_millis(15));
        assert!(cache.get(&k).is_none());
    }

    #[test]
    fn eviction_at_capacity() {
        let cache = SearchCache::new(2, DEFAULT_TTL_MS);
        cache.put(key("a"), hits(&[1]));
        cache.put(key("b"), hits(&[2]));
        // At capacity, adding c should evict oldest.
        cache.put(key("c"), hits(&[3]));
        let stats = cache.stats();
        assert_eq!(stats.entries, 2);
    }

    #[test]
    fn stats_tracking() {
        let cache = SearchCache::default();
        let k = key("stats");
        cache.put(k.clone(), hits(&[1]));
        cache.get(&k); // hit
        cache.get(&key("miss")); // miss
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn clear_resets_everything() {
        let cache = SearchCache::default();
        cache.put(key("x"), hits(&[1]));
        cache.clear();
        assert!(cache.get(&key("x")).is_none());
        assert_eq!(cache.stats().entries, 0);
    }

    #[test]
    fn different_modes_are_separate_keys() {
        let cache = SearchCache::default();
        let k1 = SearchCacheKey {
            query: "same".into(),
            mode: "rrf".into(),
            limit: 10,
        };
        let k2 = SearchCacheKey {
            query: "same".into(),
            mode: "hybrid".into(),
            limit: 10,
        };
        cache.put(k1.clone(), hits(&[1]));
        cache.put(k2.clone(), hits(&[2]));
        assert_eq!(cache.get(&k1).unwrap()[0].memory_id, 1);
        assert_eq!(cache.get(&k2).unwrap()[0].memory_id, 2);
    }

    #[test]
    fn different_limits_are_separate_keys() {
        let cache = SearchCache::default();
        let k1 = SearchCacheKey {
            query: "same".into(),
            mode: "rrf".into(),
            limit: 5,
        };
        let k2 = SearchCacheKey {
            query: "same".into(),
            mode: "rrf".into(),
            limit: 10,
        };
        cache.put(k1.clone(), hits(&[1]));
        cache.put(k2.clone(), hits(&[1, 2]));
        assert_eq!(cache.get(&k1).unwrap().len(), 1);
        assert_eq!(cache.get(&k2).unwrap().len(), 2);
    }
}
