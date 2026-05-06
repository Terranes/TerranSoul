//! Vector nearest-neighbor index for memory search (Chunk 16.10).
//!
//! Uses the native [`usearch`] HNSW backend when the `native-ann` feature is
//! enabled. Default builds use a pure-Rust linear backend so Windows app/test
//! binaries do not depend on the MSVC C++ runtime at process load time.
//!
//! The index lives next to the SQLite file as `vectors.usearch`.  On
//! startup the store loads the index from disk; if the file is missing
//! or corrupt the index is rebuilt from the DB embeddings.  Each
//! `set_embedding` / `delete` call keeps the index in sync and
//! periodically persists to disk.
//!
//! **Fallback**: When the index is unavailable (dimension mismatch,
//! corrupt file, empty DB) `vector_search` silently falls back to
//! the brute-force path.
//!
//! See `docs/brain-advanced-design.md` section 16 Phase 4.

use std::cell::Cell;
#[cfg(not(feature = "native-ann"))]
use std::cell::RefCell;
use std::path::{Path, PathBuf};
#[cfg(feature = "native-ann")]
use usearch::ffi::{IndexOptions, MetricKind, ScalarKind};
#[cfg(feature = "native-ann")]
use usearch::Index;

/// File name for the persisted ANN index, stored alongside `memory.db`.
const INDEX_FILENAME: &str = "vectors.usearch";

/// Number of `add` operations between automatic saves to disk.
/// Keeps the on-disk copy reasonably fresh without flushing on every write.
const SAVE_INTERVAL: usize = 50;

/// Wrapper around the selected ANN backend that tracks dimensions and persistence.
pub struct AnnIndex {
    #[cfg(feature = "native-ann")]
    index: Index,
    #[cfg(not(feature = "native-ann"))]
    entries: RefCell<Vec<(i64, Vec<f32>)>>,
    path: Option<PathBuf>,
    dimensions: usize,
    /// Counter of unsaved mutations (add / remove).  When this reaches
    /// `SAVE_INTERVAL` the index is flushed to disk.
    dirty: Cell<usize>,
}

/// Result of an ANN search: (memory_id, cosine_similarity).
pub type AnnMatch = (i64, f32);

impl AnnIndex {
    /// Create a new in-memory ANN index with the given dimensionality.
    pub fn new(dimensions: usize) -> Result<Self, String> {
        #[cfg(feature = "native-ann")]
        {
            let options = IndexOptions {
                dimensions,
                metric: MetricKind::Cos,
                quantization: ScalarKind::F32,
                ..Default::default()
            };
            let index = Index::new(&options).map_err(|e| e.to_string())?;
            Ok(Self {
                index,
                path: None,
                dimensions,
                dirty: Cell::new(0),
            })
        }
        #[cfg(not(feature = "native-ann"))]
        {
            Ok(Self {
                entries: RefCell::new(Vec::new()),
                path: None,
                dimensions,
                dirty: Cell::new(0),
            })
        }
    }

    /// Create or load an ANN index backed by a file in `data_dir`.
    ///
    /// If `vectors.usearch` exists and has the expected dimensionality it is
    /// loaded.  Otherwise an empty index is returned (the caller should
    /// call [`rebuild`] to populate it from the database).
    pub fn open(data_dir: &Path, dimensions: usize) -> Result<Self, String> {
        let file = data_dir.join(INDEX_FILENAME);
        #[cfg(feature = "native-ann")]
        {
            let options = IndexOptions {
                dimensions,
                metric: MetricKind::Cos,
                quantization: ScalarKind::F32,
                ..Default::default()
            };
            let index = Index::new(&options).map_err(|e| e.to_string())?;

            // Try loading the persisted index.
            if file.exists() {
                match index.load(file.to_string_lossy().as_ref()) {
                    Ok(()) if index.dimensions() == dimensions => {
                        // Loaded successfully with matching dimensions.
                    }
                    _ => {
                        // Corrupt or dimension mismatch — start fresh.
                        let fresh = Index::new(&options).map_err(|e| e.to_string())?;
                        return Ok(Self {
                            index: fresh,
                            path: Some(file),
                            dimensions,
                            dirty: Cell::new(0),
                        });
                    }
                }
            }

            Ok(Self {
                index,
                path: Some(file),
                dimensions,
                dirty: Cell::new(0),
            })
        }
        #[cfg(not(feature = "native-ann"))]
        {
            Ok(Self {
                entries: RefCell::new(Vec::new()),
                path: Some(file),
                dimensions,
                dirty: Cell::new(0),
            })
        }
    }

    /// The dimensionality this index was created with.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Number of vectors currently in the index.
    pub fn len(&self) -> usize {
        #[cfg(feature = "native-ann")]
        {
            self.index.size()
        }
        #[cfg(not(feature = "native-ann"))]
        {
            self.entries.borrow().len()
        }
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Insert or replace a vector for the given memory ID.
    ///
    /// If the embedding dimensionality does not match, the call is silently
    /// ignored (caller should rebuild the index with the new dimensions).
    pub fn add(&self, id: i64, embedding: &[f32]) -> Result<(), String> {
        if embedding.len() != self.dimensions {
            return Ok(()); // dimension mismatch — ignore silently
        }
        #[cfg(feature = "native-ann")]
        {
            // usearch allows duplicate keys by default (multi=false means
            // latest-wins).  Remove first to ensure a clean replace.
            if self.index.contains(id as u64) {
                let _ = self.index.remove(id as u64);
            }
            // Grow capacity exponentially (doubling) instead of +1 per add,
            // amortising O(n²) bulk inserts down to O(n).
            let size = self.index.size();
            let cap = self.index.capacity();
            if size + 1 > cap {
                let new_cap = (cap.max(16)).saturating_mul(2).max(size + 1);
                self.index.reserve(new_cap).map_err(|e| e.to_string())?;
            }
            self.index
                .add(id as u64, embedding)
                .map_err(|e| e.to_string())?;
        }
        #[cfg(not(feature = "native-ann"))]
        {
            let mut entries = self.entries.borrow_mut();
            entries.retain(|(existing_id, _)| *existing_id != id);
            entries.push((id, embedding.to_vec()));
        }
        self.bump_dirty();
        Ok(())
    }

    /// Pre-reserve capacity for at least `n` vectors. Useful for bulk
    /// inserts (benchmarks, rebuilds) to avoid amortised allocation cost.
    pub fn reserve_capacity(&self, n: usize) -> Result<(), String> {
        #[cfg(feature = "native-ann")]
        {
            if self.index.capacity() < n {
                self.index.reserve(n).map_err(|e| e.to_string())?;
            }
        }
        #[cfg(not(feature = "native-ann"))]
        {
            self.entries.borrow_mut().reserve(n);
        }
        Ok(())
    }

    /// Pre-reserve capacity using the native backend's threaded reservation
    /// path when available. This is primarily useful for large benchmark and
    /// rebuild jobs where the full target size is known up front.
    pub fn reserve_capacity_with_threads(&self, n: usize, threads: usize) -> Result<(), String> {
        #[cfg(feature = "native-ann")]
        {
            if self.index.capacity() < n {
                self.index
                    .reserve_capacity_and_threads(n, threads.max(1))
                    .map_err(|e| e.to_string())?;
            }
        }
        #[cfg(not(feature = "native-ann"))]
        {
            let _ = threads;
            self.entries.borrow_mut().reserve(n);
        }
        Ok(())
    }

    /// Remove a vector by memory ID.  No-op if the ID is not in the index.
    pub fn remove(&self, id: i64) -> Result<(), String> {
        #[cfg(feature = "native-ann")]
        {
            if self.index.contains(id as u64) {
                self.index.remove(id as u64).map_err(|e| e.to_string())?;
                self.bump_dirty();
            }
        }
        #[cfg(not(feature = "native-ann"))]
        {
            let mut entries = self.entries.borrow_mut();
            let before = entries.len();
            entries.retain(|(existing_id, _)| *existing_id != id);
            if entries.len() != before {
                self.bump_dirty();
            }
        }
        Ok(())
    }

    /// Find the `limit` nearest neighbours to `query`.
    ///
    /// Returns `(memory_id, cosine_similarity)` pairs sorted descending
    /// by similarity.  `usearch` returns cosine *distance* (`1 - sim`)
    /// so we convert.
    pub fn search(&self, query: &[f32], limit: usize) -> Result<Vec<AnnMatch>, String> {
        if query.len() != self.dimensions || self.is_empty() {
            return Ok(vec![]);
        }
        #[cfg(feature = "native-ann")]
        {
            let matches = self.index.search(query, limit).map_err(|e| e.to_string())?;
            Ok(matches
                .keys
                .iter()
                .zip(matches.distances.iter())
                .map(|(&k, &d)| (k as i64, 1.0 - d))
                .collect())
        }
        #[cfg(not(feature = "native-ann"))]
        {
            let mut matches: Vec<AnnMatch> = self
                .entries
                .borrow()
                .iter()
                .map(|(id, embedding)| (*id, cosine_similarity(query, embedding)))
                .collect();
            matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            matches.truncate(limit);
            Ok(matches)
        }
    }

    /// Persist the index to disk (no-op for in-memory indices).
    pub fn save(&self) -> Result<(), String> {
        #[cfg(feature = "native-ann")]
        {
            if let Some(path) = &self.path {
                self.index
                    .save(path.to_string_lossy().as_ref())
                    .map_err(|e| e.to_string())?;
                self.dirty.set(0);
            }
        }
        #[cfg(not(feature = "native-ann"))]
        {
            let _ = &self.path;
            self.dirty.set(0);
        }
        Ok(())
    }

    /// Rebuild the index from an iterator of `(id, embedding)` pairs.
    ///
    /// This is used on first startup when the index file is missing or
    /// after a dimension change.
    pub fn rebuild<'a>(
        &self,
        entries: impl Iterator<Item = (i64, &'a [f32])>,
    ) -> Result<usize, String> {
        #[cfg(feature = "native-ann")]
        {
            // Reset the index.
            self.index.reset().map_err(|e| e.to_string())?;
            let mut count = 0usize;
            for (id, emb) in entries {
                if emb.len() != self.dimensions {
                    continue;
                }
                self.index
                    .reserve(self.index.size() + 1)
                    .map_err(|e| e.to_string())?;
                self.index.add(id as u64, emb).map_err(|e| e.to_string())?;
                count += 1;
            }
            self.save()?;
            Ok(count)
        }
        #[cfg(not(feature = "native-ann"))]
        {
            let mut stored = self.entries.borrow_mut();
            stored.clear();
            let mut count = 0usize;
            for (id, emb) in entries {
                if emb.len() != self.dimensions {
                    continue;
                }
                stored.push((id, emb.to_vec()));
                count += 1;
            }
            drop(stored);
            self.save()?;
            Ok(count)
        }
    }

    // ── Internal ───────────────────────────────────────────────────────

    /// Increment the dirty counter and auto-save when threshold is reached.
    fn bump_dirty(&self) {
        let n = self.dirty.get() + 1;
        if n >= SAVE_INTERVAL {
            let _ = self.save();
        } else {
            self.dirty.set(n);
        }
    }
}

#[cfg(not(feature = "native-ann"))]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}

/// Detect the embedding dimensionality from the first embedded entry in the DB.
///
/// Returns `None` if there are no embeddings yet.
pub fn detect_dimensions(conn: &rusqlite::Connection) -> Option<usize> {
    let blob: Vec<u8> = conn
        .query_row(
            "SELECT embedding FROM memories WHERE embedding IS NOT NULL LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok()?;
    // Each f32 is 4 bytes.
    Some(blob.len() / 4)
}

/// Derive the index file path from a data directory.
pub fn index_path(data_dir: &Path) -> PathBuf {
    data_dir.join(INDEX_FILENAME)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vec(dim: usize, val: f32) -> Vec<f32> {
        vec![val; dim]
    }

    #[test]
    fn create_and_search_basic() {
        let idx = AnnIndex::new(3).unwrap();
        idx.add(1, &[1.0, 0.0, 0.0]).unwrap();
        idx.add(2, &[0.0, 1.0, 0.0]).unwrap();
        idx.add(3, &[1.0, 0.1, 0.0]).unwrap();

        let results = idx.search(&[1.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(results.len(), 2);
        // The closest match to [1,0,0] should be id=1 (exact match).
        assert_eq!(results[0].0, 1);
        assert!(results[0].1 > 0.99, "exact match should have sim ~1.0");
    }

    #[test]
    fn dimension_mismatch_silently_ignored() {
        let idx = AnnIndex::new(3).unwrap();
        // Wrong dimension — should not error, just no-op.
        idx.add(1, &[1.0, 0.0]).unwrap();
        assert_eq!(idx.len(), 0);
    }

    #[test]
    fn remove_entry() {
        let idx = AnnIndex::new(3).unwrap();
        idx.add(1, &[1.0, 0.0, 0.0]).unwrap();
        assert_eq!(idx.len(), 1);
        idx.remove(1).unwrap();
        // usearch marks as deleted but may report size differently;
        // search should return empty.
        let results = idx.search(&[1.0, 0.0, 0.0], 5).unwrap();
        assert!(
            results.is_empty() || !results.iter().any(|(id, _)| *id == 1),
            "removed entry should not appear in search results"
        );
    }

    #[test]
    fn rebuild_from_entries() {
        let idx = AnnIndex::new(4).unwrap();
        let vecs: Vec<(i64, Vec<f32>)> = (0..10)
            .map(|i| (i, sample_vec(4, (i as f32 + 1.0) / 10.0)))
            .collect();
        let refs: Vec<(i64, &[f32])> = vecs.iter().map(|(id, v)| (*id, v.as_slice())).collect();
        let count = idx.rebuild(refs.into_iter()).unwrap();
        assert_eq!(count, 10);
        assert_eq!(idx.len(), 10);
    }

    #[test]
    fn search_empty_index() {
        let idx = AnnIndex::new(3).unwrap();
        let results = idx.search(&[1.0, 0.0, 0.0], 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn replace_existing_key() {
        let idx = AnnIndex::new(3).unwrap();
        idx.add(1, &[1.0, 0.0, 0.0]).unwrap();
        // Replace with a different vector.
        idx.add(1, &[0.0, 1.0, 0.0]).unwrap();
        let results = idx.search(&[0.0, 1.0, 0.0], 1).unwrap();
        assert_eq!(results[0].0, 1);
        assert!(results[0].1 > 0.99);
    }

    #[test]
    fn detect_dimensions_empty_db() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE memories (id INTEGER PRIMARY KEY, embedding BLOB);")
            .unwrap();
        assert_eq!(detect_dimensions(&conn), None);
    }

    #[test]
    fn detect_dimensions_from_db() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE memories (id INTEGER PRIMARY KEY, embedding BLOB);")
            .unwrap();
        // Insert a 4-dim embedding (16 bytes).
        let bytes: Vec<u8> = [1.0f32, 2.0, 3.0, 4.0]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        conn.execute(
            "INSERT INTO memories (embedding) VALUES (?1)",
            rusqlite::params![bytes],
        )
        .unwrap();
        assert_eq!(detect_dimensions(&conn), Some(4));
    }

    /// Parity test: with a deterministic seed, search results should
    /// be identical regardless of backend (linear or HNSW). Since we
    /// only compile one backend at a time, this test validates that
    /// the active backend returns correct cosine-ranked results for
    /// a known dataset (Chunk 38.3).
    #[test]
    fn ann_parity_deterministic_ranking() {
        // Use a deterministic "xoshiro-like" seed to generate vectors.
        let dim = 32;
        let n = 100;
        let idx = AnnIndex::new(dim).unwrap();

        // Generate deterministic vectors: vec[i][j] = sin(i * j + 1)
        let mut vectors: Vec<Vec<f32>> = Vec::with_capacity(n);
        for i in 0..n {
            let v: Vec<f32> = (0..dim).map(|j| ((i * j + 1) as f32 * 0.1).sin()).collect();
            vectors.push(v);
        }

        for (i, v) in vectors.iter().enumerate() {
            idx.add(i as i64 + 1, v).unwrap();
        }

        // Query with vectors[0]
        let results = idx.search(&vectors[0], 10).unwrap();
        assert_eq!(results.len(), 10, "should return k=10 results");
        // First result must be the exact match (id=1 ~ vectors[0])
        assert_eq!(results[0].0, 1);
        assert!(results[0].1 > 0.99, "exact match sim={}", results[0].1);
        // Results should be sorted descending by similarity.
        for w in results.windows(2) {
            assert!(
                w[0].1 >= w[1].1 - 1e-6,
                "results not sorted: {} >= {}",
                w[0].1,
                w[1].1
            );
        }

        // Verify all returned IDs are valid (1..=100)
        for (id, sim) in &results {
            assert!(*id >= 1 && *id <= n as i64);
            // Cosine similarity is theoretically in [-1, 1] but floating
            // point arithmetic can slightly exceed that range.
            assert!(
                *sim >= -1.1 && *sim <= 1.1,
                "similarity out of range: {sim}"
            );
        }
    }
}
