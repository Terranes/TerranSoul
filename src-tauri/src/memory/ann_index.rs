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
//! **Quantization** (Chunk 41.9): The index supports `f32` (default),
//! `i8` (≈4× memory reduction, <1% recall loss), and `b1` (binary,
//! aggressive compression with documented recall trade-off). The setting
//! is persisted as `vectors.usearch.quant` next to the index file.
//!
//! See `docs/brain-advanced-design.md` section 16 Phase 4.

use std::cell::Cell;
#[cfg(not(feature = "native-ann"))]
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::Instant;
#[cfg(feature = "native-ann")]
use usearch::ffi::{IndexOptions, MetricKind, ScalarKind};
#[cfg(feature = "native-ann")]
use usearch::Index;

#[cfg(not(feature = "native-ann"))]
use super::mobile_ann::MobileAnnIndex;

/// File name for the persisted ANN index, stored alongside `memory.db`.
const INDEX_FILENAME: &str = "vectors.usearch";

/// Sidecar file that records which quantization the index was built with.
const QUANT_FILENAME: &str = "vectors.usearch.quant";

/// Number of `add`/`remove` operations that must accumulate before a flush
/// is considered due. At 20 000 ops the 1M-row bulk insert flushes ≤ 50 times.
const FLUSH_OPS_THRESHOLD: usize = 20_000;

/// Maximum seconds since the first unsaved mutation before a flush is due.
/// Ensures small bursts of writes are eventually persisted even when the
/// ops threshold is not reached.
const FLUSH_SECS_THRESHOLD: u64 = 30;

/// Embedding quantization mode for the ANN index.
///
/// Controls the `ScalarKind` passed to usearch `IndexOptions`.
/// Persisted as a sidecar file so reloads match the index format.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingQuantization {
    /// Full precision (4 bytes per dimension). Default.
    #[default]
    F32,
    /// Signed 8-bit integer quantization (1 byte per dimension).
    /// ≈4× memory reduction with <1% recall loss.
    I8,
    /// Binary quantization (1 bit per dimension).
    /// Aggressive compression; recall loss depends on dataset.
    B1,
}

impl EmbeddingQuantization {
    /// Convert to usearch ScalarKind (native-ann only).
    #[cfg(feature = "native-ann")]
    fn to_scalar_kind(self) -> ScalarKind {
        match self {
            Self::F32 => ScalarKind::F32,
            Self::I8 => ScalarKind::I8,
            Self::B1 => ScalarKind::B1,
        }
    }

    /// Parse from a string (for sidecar file reading).
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "i8" => Self::I8,
            "b1" => Self::B1,
            _ => Self::F32,
        }
    }

    /// Serialize to a short string for the sidecar file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::I8 => "i8",
            Self::B1 => "b1",
        }
    }
}

/// Wrapper around the selected ANN backend that tracks dimensions and persistence.
pub struct AnnIndex {
    #[cfg(feature = "native-ann")]
    index: Index,
    #[cfg(not(feature = "native-ann"))]
    mobile: RefCell<MobileAnnIndex>,
    path: Option<PathBuf>,
    dimensions: usize,
    /// Quantization mode used to build this index.
    quantization: EmbeddingQuantization,
    /// Counter of unsaved mutations (add / remove).
    dirty: Cell<usize>,
    /// Instant of the first unsaved mutation (reset on flush).
    first_dirty_at: Cell<Option<Instant>>,
    /// Number of `remove` calls since last compaction/rebuild.
    /// Used to compute fragmentation ratio for the compaction threshold.
    removed_since_compact: Cell<usize>,
}

/// Result of an ANN search: (memory_id, cosine_similarity).
pub type AnnMatch = (i64, f32);

impl AnnIndex {
    /// Create a new in-memory ANN index with the given dimensionality.
    pub fn new(dimensions: usize) -> Result<Self, String> {
        Self::new_quantized(dimensions, EmbeddingQuantization::default())
    }

    /// Create a new in-memory ANN index with specified quantization.
    pub fn new_quantized(
        dimensions: usize,
        quantization: EmbeddingQuantization,
    ) -> Result<Self, String> {
        #[cfg(feature = "native-ann")]
        {
            let options = IndexOptions {
                dimensions,
                metric: MetricKind::Cos,
                quantization: quantization.to_scalar_kind(),
                ..Default::default()
            };
            let index = Index::new(&options).map_err(|e| e.to_string())?;
            Ok(Self {
                index,
                path: None,
                dimensions,
                quantization,
                dirty: Cell::new(0),
                first_dirty_at: Cell::new(None),
                removed_since_compact: Cell::new(0),
            })
        }
        #[cfg(not(feature = "native-ann"))]
        {
            let _ = quantization; // mobile fallback handles quantization internally
            Ok(Self {
                mobile: RefCell::new(MobileAnnIndex::new(dimensions)),
                path: None,
                dimensions,
                quantization,
                dirty: Cell::new(0),
                first_dirty_at: Cell::new(None),
                removed_since_compact: Cell::new(0),
            })
        }
    }

    /// Create or load an ANN index backed by a file in `data_dir`.
    ///
    /// If `vectors.usearch` exists and has the expected dimensionality it is
    /// loaded.  Otherwise an empty index is returned (the caller should
    /// call [`rebuild`] to populate it from the database).
    pub fn open(data_dir: &Path, dimensions: usize) -> Result<Self, String> {
        // Read the persisted quantization sidecar, or default to f32.
        let quant = read_quant_sidecar(data_dir);
        Self::open_quantized(data_dir, dimensions, quant)
    }

    /// Open with explicit quantization (used by AnnRegistry and when
    /// the user changes the quantization setting).
    pub fn open_quantized(
        data_dir: &Path,
        dimensions: usize,
        quantization: EmbeddingQuantization,
    ) -> Result<Self, String> {
        let file = data_dir.join(INDEX_FILENAME);
        #[cfg(feature = "native-ann")]
        {
            let options = IndexOptions {
                dimensions,
                metric: MetricKind::Cos,
                quantization: quantization.to_scalar_kind(),
                ..Default::default()
            };
            let index = Index::new(&options).map_err(|e| e.to_string())?;

            // Try memory-mapping the persisted index (lower RSS than load).
            if file.exists() {
                match index.view(file.to_string_lossy().as_ref()) {
                    Ok(()) if index.dimensions() == dimensions => {
                        // Viewed successfully — mmap'd, low RSS.
                    }
                    _ => {
                        // Corrupt or dimension mismatch — start fresh.
                        let fresh = Index::new(&options).map_err(|e| e.to_string())?;
                        return Ok(Self {
                            index: fresh,
                            path: Some(file),
                            dimensions,
                            quantization,
                            dirty: Cell::new(0),
                            first_dirty_at: Cell::new(None),
                            removed_since_compact: Cell::new(0),
                        });
                    }
                }
            }

            Ok(Self {
                index,
                path: Some(file),
                dimensions,
                quantization,
                dirty: Cell::new(0),
                first_dirty_at: Cell::new(None),
                removed_since_compact: Cell::new(0),
            })
        }
        #[cfg(not(feature = "native-ann"))]
        {
            let _ = quantization;
            Ok(Self {
                mobile: RefCell::new(MobileAnnIndex::new(dimensions)),
                path: Some(file),
                dimensions,
                quantization,
                dirty: Cell::new(0),
                first_dirty_at: Cell::new(None),
                removed_since_compact: Cell::new(0),
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
            self.mobile.borrow().len()
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
            let mut mobile = self.mobile.borrow_mut();
            mobile.add(id, embedding);
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
            self.mobile.borrow_mut().reserve(n);
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
            self.mobile.borrow_mut().reserve(n);
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
                self.removed_since_compact
                    .set(self.removed_since_compact.get() + 1);
            }
        }
        #[cfg(not(feature = "native-ann"))]
        {
            let mut mobile = self.mobile.borrow_mut();
            if mobile.remove(id) {
                self.bump_dirty();
                self.removed_since_compact
                    .set(self.removed_since_compact.get() + 1);
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
            let mut mobile = self.mobile.borrow_mut();
            mobile.ensure_built();
            Ok(mobile.search(query, limit))
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
                // Persist the quantization sidecar next to the index.
                write_quant_sidecar(path, self.quantization);
            }
        }
        #[cfg(not(feature = "native-ann"))]
        {
            let _ = &self.path;
        }
        // Always reset dirty state (even for in-memory indices).
        self.dirty.set(0);
        self.first_dirty_at.set(None);
        Ok(())
    }

    /// The quantization mode this index was built with.
    pub fn quantization(&self) -> EmbeddingQuantization {
        self.quantization
    }

    /// Fragmentation ratio: proportion of removed entries vs total capacity.
    ///
    /// Returns a value in `[0.0, 1.0]`. A higher value means more tombstones
    /// are present and compaction would reclaim space / improve traversal.
    pub fn fragmentation_ratio(&self) -> f32 {
        let removed = self.removed_since_compact.get();
        let live = self.len();
        let total = live + removed;
        if total == 0 {
            0.0
        } else {
            removed as f32 / total as f32
        }
    }

    /// Reset the fragmentation counter (called after compaction).
    pub fn reset_fragmentation(&self) {
        self.removed_since_compact.set(0);
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
            let mut mobile = self.mobile.borrow_mut();
            // Clear and rebuild the mobile index from the provided entries.
            *mobile = MobileAnnIndex::new(self.dimensions);
            let mut count = 0usize;
            for (id, emb) in entries {
                if emb.len() != self.dimensions {
                    continue;
                }
                mobile.add(id, emb);
                count += 1;
            }
            mobile.build_ivf();
            drop(mobile);
            self.save()?;
            Ok(count)
        }
    }

    // ── Internal ───────────────────────────────────────────────────────

    /// Increment the dirty counter and return whether a flush is needed.
    ///
    /// Unlike the previous `SAVE_INTERVAL = 50` approach, this does NOT
    /// auto-save. The caller is responsible for calling `save()` when
    /// this returns `true` (or scheduling a debounced async flush).
    fn bump_dirty(&self) -> bool {
        let n = self.dirty.get() + 1;
        self.dirty.set(n);
        if self.first_dirty_at.get().is_none() {
            self.first_dirty_at.set(Some(Instant::now()));
        }
        self.needs_flush()
    }

    /// Check whether the index should be flushed based on ops count or time.
    pub fn needs_flush(&self) -> bool {
        let d = self.dirty.get();
        if d == 0 {
            return false;
        }
        if d >= FLUSH_OPS_THRESHOLD {
            return true;
        }
        if let Some(first) = self.first_dirty_at.get() {
            if first.elapsed().as_secs() >= FLUSH_SECS_THRESHOLD {
                return true;
            }
        }
        false
    }

    /// Flush the index to disk if there are pending mutations.
    /// Returns the number of dirty ops that were flushed (0 if clean).
    pub fn flush_if_needed(&self) -> Result<usize, String> {
        let d = self.dirty.get();
        if d > 0 {
            self.save()?;
            Ok(d)
        } else {
            Ok(0)
        }
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

/// Read the quantization sidecar from a data directory.
/// Returns `F32` if the file is missing or unreadable.
pub fn read_quant_sidecar(data_dir: &Path) -> EmbeddingQuantization {
    let path = data_dir.join(QUANT_FILENAME);
    std::fs::read_to_string(&path)
        .map(|s| EmbeddingQuantization::from_str_lossy(&s))
        .unwrap_or_default()
}

/// Write the quantization sidecar next to the index file.
#[cfg_attr(not(feature = "native-ann"), allow(dead_code))]
fn write_quant_sidecar(index_path: &Path, quant: EmbeddingQuantization) {
    // The sidecar lives alongside the index file: same directory, different name.
    if let Some(dir) = index_path.parent() {
        let sidecar = dir.join(QUANT_FILENAME);
        let _ = std::fs::write(sidecar, quant.as_str());
    }
}

// ── AnnRegistry (multi-model) ──────────────────────────────────────────────────

use std::collections::HashMap;

/// A registry of ANN indices keyed by `(model_id, dim)`.
///
/// Each model/dimension pair gets its own HNSW index persisted as
/// `vectors_<model_id>.usearch` in the data directory.  The primary
/// (legacy) index is stored under the default `vectors.usearch` name.
pub struct AnnRegistry {
    /// Primary index — the legacy single-model index.
    primary: std::cell::OnceCell<AnnIndex>,
    /// Additional per-model indices.
    models: std::cell::RefCell<HashMap<String, AnnIndex>>,
    data_dir: Option<PathBuf>,
}

impl AnnRegistry {
    /// Create an empty registry.
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        Self {
            primary: std::cell::OnceCell::new(),
            models: std::cell::RefCell::new(HashMap::new()),
            data_dir,
        }
    }

    /// Get or lazily initialize the primary (legacy) index.
    pub fn primary(&self, conn: &rusqlite::Connection) -> Option<&AnnIndex> {
        if let Some(idx) = self.primary.get() {
            return Some(idx);
        }
        let dims = detect_dimensions(conn)?;
        if dims == 0 {
            return None;
        }
        let idx = if let Some(dir) = &self.data_dir {
            AnnIndex::open(dir, dims).ok()?
        } else {
            AnnIndex::new(dims).ok()?
        };
        let _ = self.primary.set(idx);
        self.primary.get()
    }

    /// Get or create the primary index with a known dimension.
    pub fn primary_for_dim(&self, dim: usize) -> Option<&AnnIndex> {
        if let Some(idx) = self.primary.get() {
            if idx.dimensions() == dim {
                return Some(idx);
            }
            return None;
        }
        let idx = if let Some(dir) = &self.data_dir {
            AnnIndex::open(dir, dim).ok()?
        } else {
            AnnIndex::new(dim).ok()?
        };
        let _ = self.primary.set(idx);
        self.primary.get()
    }

    /// Get or create an index for a specific model_id + dimension.
    /// The index file is persisted as `vectors_<model_id>.usearch`.
    pub fn for_model(&self, model_id: &str, dim: usize) -> Option<&AnnIndex> {
        // SAFETY: We never hold a borrow across this function call and
        // MemoryStore is always behind a Mutex, so no concurrent access.
        let models = self.models.borrow();
        if models.contains_key(model_id) {
            drop(models);
            // Re-borrow to return a reference with the right lifetime.
            // This is safe because we never remove entries from the map.
            let models = unsafe { &*self.models.as_ptr() };
            return models.get(model_id);
        }
        drop(models);

        let idx = if let Some(dir) = &self.data_dir {
            let file = dir.join(format!("vectors_{model_id}.usearch"));
            // Use the same open logic but with a custom filename.
            ann_open_file(&file, dim).ok()?
        } else {
            AnnIndex::new(dim).ok()?
        };

        let mut models = self.models.borrow_mut();
        models.insert(model_id.to_string(), idx);
        drop(models);

        let models = unsafe { &*self.models.as_ptr() };
        models.get(model_id)
    }

    /// List all model IDs that have indices in this registry.
    pub fn model_ids(&self) -> Vec<String> {
        self.models.borrow().keys().cloned().collect()
    }

    /// Save all dirty indices to disk.
    pub fn save_all(&self) {
        if let Some(idx) = self.primary.get() {
            let _ = idx.save();
        }
        for idx in self.models.borrow().values() {
            let _ = idx.save();
        }
    }
}

/// Open or create an ANN index at a specific file path.
fn ann_open_file(file: &Path, dimensions: usize) -> Result<AnnIndex, String> {
    ann_open_file_quantized(file, dimensions, EmbeddingQuantization::default())
}

/// Open or create an ANN index at a specific file path with quantization.
fn ann_open_file_quantized(
    file: &Path,
    dimensions: usize,
    quantization: EmbeddingQuantization,
) -> Result<AnnIndex, String> {
    #[cfg(feature = "native-ann")]
    {
        let options = IndexOptions {
            dimensions,
            metric: MetricKind::Cos,
            quantization: quantization.to_scalar_kind(),
            ..Default::default()
        };
        let index = Index::new(&options).map_err(|e| e.to_string())?;
        if file.exists() {
            match index.view(file.to_string_lossy().as_ref()) {
                Ok(()) if index.dimensions() == dimensions => {}
                _ => {
                    let fresh = Index::new(&options).map_err(|e| e.to_string())?;
                    return Ok(AnnIndex {
                        index: fresh,
                        path: Some(file.to_path_buf()),
                        dimensions,
                        quantization,
                        dirty: Cell::new(0),
                        first_dirty_at: Cell::new(None),
                        removed_since_compact: Cell::new(0),
                    });
                }
            }
        }
        Ok(AnnIndex {
            index,
            path: Some(file.to_path_buf()),
            dimensions,
            quantization,
            dirty: Cell::new(0),
            first_dirty_at: Cell::new(None),
            removed_since_compact: Cell::new(0),
        })
    }
    #[cfg(not(feature = "native-ann"))]
    {
        let _ = quantization;
        Ok(AnnIndex {
            mobile: RefCell::new(MobileAnnIndex::new(dimensions)),
            path: Some(file.to_path_buf()),
            dimensions,
            quantization,
            dirty: Cell::new(0),
            first_dirty_at: Cell::new(None),
            removed_since_compact: Cell::new(0),
        })
    }
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

    // ── Chunk 41.9 — Quantization tests ──────────────────────────────────

    #[test]
    fn quantization_default_is_f32() {
        assert_eq!(EmbeddingQuantization::default(), EmbeddingQuantization::F32);
    }

    #[test]
    fn quantization_from_str_lossy() {
        assert_eq!(
            EmbeddingQuantization::from_str_lossy("i8"),
            EmbeddingQuantization::I8
        );
        assert_eq!(
            EmbeddingQuantization::from_str_lossy("I8"),
            EmbeddingQuantization::I8
        );
        assert_eq!(
            EmbeddingQuantization::from_str_lossy("b1"),
            EmbeddingQuantization::B1
        );
        assert_eq!(
            EmbeddingQuantization::from_str_lossy("f32"),
            EmbeddingQuantization::F32
        );
        assert_eq!(
            EmbeddingQuantization::from_str_lossy("garbage"),
            EmbeddingQuantization::F32
        );
        assert_eq!(
            EmbeddingQuantization::from_str_lossy("  i8\n"),
            EmbeddingQuantization::I8
        );
    }

    #[test]
    fn quantization_as_str_roundtrip() {
        for q in [
            EmbeddingQuantization::F32,
            EmbeddingQuantization::I8,
            EmbeddingQuantization::B1,
        ] {
            assert_eq!(EmbeddingQuantization::from_str_lossy(q.as_str()), q);
        }
    }

    #[test]
    fn new_quantized_creates_index_with_setting() {
        let idx = AnnIndex::new_quantized(4, EmbeddingQuantization::I8).unwrap();
        assert_eq!(idx.quantization(), EmbeddingQuantization::I8);
        assert_eq!(idx.dimensions(), 4);
        // Should still be functional: add and search.
        idx.add(1, &[1.0, 0.0, 0.0, 0.0]).unwrap();
        idx.add(2, &[0.0, 1.0, 0.0, 0.0]).unwrap();
        let results = idx.search(&[1.0, 0.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 1);
    }

    #[test]
    fn quant_sidecar_read_missing_is_f32() {
        let dir = std::env::temp_dir().join("ts_test_quant_missing");
        let _ = std::fs::create_dir_all(&dir);
        // No sidecar file — should default to f32.
        assert_eq!(read_quant_sidecar(&dir), EmbeddingQuantization::F32);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn quant_sidecar_write_and_read() {
        let dir = std::env::temp_dir().join("ts_test_quant_roundtrip");
        let _ = std::fs::create_dir_all(&dir);
        let index_path = dir.join(INDEX_FILENAME);
        write_quant_sidecar(&index_path, EmbeddingQuantization::I8);
        assert_eq!(read_quant_sidecar(&dir), EmbeddingQuantization::I8);
        write_quant_sidecar(&index_path, EmbeddingQuantization::B1);
        assert_eq!(read_quant_sidecar(&dir), EmbeddingQuantization::B1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recall_regression_i8_vs_f32() {
        // Insert 200 deterministic vectors, query with f32 vs i8 indices.
        // The i8 index should return the same top-1 result (exact match)
        // with recall loss < 10% on top-10 for this dataset.
        let dim = 32;
        let n = 200;

        let vectors: Vec<Vec<f32>> = (0..n)
            .map(|i| (0..dim).map(|j| ((i * j + 1) as f32 * 0.1).sin()).collect())
            .collect();

        let f32_idx = AnnIndex::new_quantized(dim, EmbeddingQuantization::F32).unwrap();
        let i8_idx = AnnIndex::new_quantized(dim, EmbeddingQuantization::I8).unwrap();

        for (i, v) in vectors.iter().enumerate() {
            f32_idx.add(i as i64 + 1, v).unwrap();
            i8_idx.add(i as i64 + 1, v).unwrap();
        }

        // Query: use vectors[0] as the query.
        let f32_results = f32_idx.search(&vectors[0], 10).unwrap();
        let i8_results = i8_idx.search(&vectors[0], 10).unwrap();

        // Top-1 must match (exact vector is in both indices).
        assert_eq!(f32_results[0].0, 1, "f32 top-1 should be exact match");
        assert_eq!(i8_results[0].0, 1, "i8 top-1 should be exact match");

        // Recall@10: count how many of f32 top-10 appear in i8 top-10.
        let f32_ids: std::collections::HashSet<i64> =
            f32_results.iter().map(|(id, _)| *id).collect();
        let i8_ids: std::collections::HashSet<i64> = i8_results.iter().map(|(id, _)| *id).collect();
        let overlap = f32_ids.intersection(&i8_ids).count();
        // Budget: at least 9 of 10 should match (90% recall).
        assert!(
            overlap >= 9,
            "i8 recall@10 too low: {overlap}/10 overlap with f32"
        );
    }

    #[test]
    fn needs_flush_respects_ops_threshold() {
        let idx = AnnIndex::new(3).unwrap();
        assert!(!idx.needs_flush());

        // Add vectors up to threshold - 1: should not trigger flush.
        for i in 0..(FLUSH_OPS_THRESHOLD - 1) {
            let v = vec![i as f32, 0.0, 1.0];
            idx.add(i as i64 + 1, &v).unwrap();
        }
        // One below threshold.
        assert!(!idx.needs_flush());

        // One more — hits the threshold.
        let v = vec![999.0, 0.0, 1.0];
        idx.add(FLUSH_OPS_THRESHOLD as i64, &v).unwrap();
        assert!(idx.needs_flush());
    }

    #[test]
    fn needs_flush_respects_time_threshold() {
        let idx = AnnIndex::new(3).unwrap();
        // Add one vector to make it dirty.
        idx.add(1, &[1.0, 0.0, 0.0]).unwrap();
        assert!(!idx.needs_flush()); // not enough ops, not enough time

        // Manually set first_dirty_at to a past time.
        idx.first_dirty_at.set(Some(
            Instant::now() - std::time::Duration::from_secs(FLUSH_SECS_THRESHOLD + 1),
        ));
        assert!(idx.needs_flush());
    }

    #[test]
    fn flush_if_needed_resets_dirty_state() {
        let idx = AnnIndex::new(3).unwrap();
        idx.add(1, &[1.0, 0.0, 0.0]).unwrap();
        assert!(idx.dirty.get() > 0);
        assert!(idx.first_dirty_at.get().is_some());

        idx.flush_if_needed().unwrap();
        assert_eq!(idx.dirty.get(), 0);
        assert!(idx.first_dirty_at.get().is_none());
        assert!(!idx.needs_flush());
    }

    #[test]
    fn bump_dirty_no_longer_auto_saves() {
        // Verify that adding many vectors does NOT auto-save (no SAVE_INTERVAL).
        let idx = AnnIndex::new(3).unwrap();
        for i in 0..100 {
            let v = vec![i as f32, 0.0, 1.0];
            idx.add(i + 1, &v).unwrap();
        }
        // dirty should be 100 (no auto-save occurred).
        assert_eq!(idx.dirty.get(), 100);
    }

    #[test]
    fn fragmentation_ratio_tracks_removes() {
        let idx = AnnIndex::new(3).unwrap();
        for i in 1..=10 {
            idx.add(i, &[i as f32, 0.0, 0.0]).unwrap();
        }
        assert_eq!(idx.fragmentation_ratio(), 0.0);
        // Remove 3 of 10 → ratio = 3/(10+3) won't work because len()
        // already decremented. Our counter is removed_since_compact.
        let _ = idx.remove(1);
        let _ = idx.remove(2);
        let _ = idx.remove(3);
        // removed_since_compact = 3, live = 7 → ratio = 3/(7+3) = 0.3
        let ratio = idx.fragmentation_ratio();
        assert!((ratio - 0.3).abs() < 0.01, "expected ~0.3, got {ratio}");
    }

    #[test]
    fn reset_fragmentation_zeroes_counter() {
        let idx = AnnIndex::new(3).unwrap();
        for i in 1..=5 {
            idx.add(i, &[i as f32, 0.0, 0.0]).unwrap();
        }
        let _ = idx.remove(1);
        let _ = idx.remove(2);
        assert!(idx.fragmentation_ratio() > 0.0);
        idx.reset_fragmentation();
        assert_eq!(idx.fragmentation_ratio(), 0.0);
    }
}
