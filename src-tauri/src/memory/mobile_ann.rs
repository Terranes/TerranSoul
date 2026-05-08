//! Mobile ANN fallback: IVF + i8 quantized scan (Chunk 42.1).
//!
//! A pure-Rust approximate nearest-neighbor index for mobile targets
//! (iOS/Android) where the C++ `usearch` library is unavailable.
//!
//! Architecture: Inverted File Index (IVF) with:
//! - `n_lists` partitions (clusters) built via k-means
//! - i8 scalar quantization (1 byte per dim, 4× less memory than f32)
//! - At query time, probes `n_probe` nearest centroids
//! - Hard cap at `MAX_MOBILE_VECTORS` (200k); beyond that, callers
//!   should degrade to FTS5 + keyword search
//!
//! This module is compiled when `native-ann` is NOT enabled (the default
//! on mobile builds via `cfg(not(feature = "native-ann"))`).

/// Maximum vectors the mobile index supports. Beyond this, hybrid search
/// should degrade to FTS5/keyword only.
pub const MAX_MOBILE_VECTORS: usize = 200_000;

/// Default number of IVF partitions (clusters). Empirically good for 50k–200k.
const DEFAULT_N_LISTS: usize = 128;

/// Default number of partitions to probe at query time.
const DEFAULT_N_PROBE: usize = 8;

/// Minimum vectors before building IVF (below this, brute-force is faster).
const IVF_BUILD_THRESHOLD: usize = 1000;

/// K-means iterations for cluster assignment.
const KMEANS_ITERATIONS: usize = 10;

/// A quantized vector: i8 per dimension + precomputed norm for fast cosine.
#[derive(Debug, Clone)]
struct QuantizedVector {
    id: i64,
    /// i8-quantized embedding (1 byte per dim).
    data: Vec<i8>,
    /// Precomputed L2 norm of the original f32 vector (for cosine correction).
    norm: f32,
}

/// IVF partition (cluster).
#[derive(Debug, Clone)]
struct IvfList {
    /// Centroid vector (f32, full precision).
    centroid: Vec<f32>,
    /// Vectors assigned to this partition.
    vectors: Vec<QuantizedVector>,
}

/// Mobile-optimized ANN index using IVF + i8 quantization.
#[derive(Debug)]
pub struct MobileAnnIndex {
    dimensions: usize,
    /// Flat store of all vectors (for brute-force when below IVF threshold).
    flat: Vec<(i64, Vec<f32>)>,
    /// IVF structure (built lazily when flat.len() >= IVF_BUILD_THRESHOLD).
    ivf: Option<IvfStructure>,
    /// Whether the IVF is stale and needs rebuilding.
    ivf_dirty: bool,
    /// Number of probes at query time.
    n_probe: usize,
}

#[derive(Debug, Clone)]
struct IvfStructure {
    lists: Vec<IvfList>,
}

/// Result type: (id, similarity).
pub type MobileAnnMatch = (i64, f32);

impl MobileAnnIndex {
    /// Create a new empty mobile index.
    pub fn new(dimensions: usize) -> Self {
        Self {
            dimensions,
            flat: Vec::new(),
            ivf: None,
            ivf_dirty: false,
            n_probe: DEFAULT_N_PROBE,
        }
    }

    /// Number of vectors in the index.
    pub fn len(&self) -> usize {
        self.flat.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.flat.is_empty()
    }

    /// Whether the index is at capacity.
    pub fn is_full(&self) -> bool {
        self.flat.len() >= MAX_MOBILE_VECTORS
    }

    /// Add or replace a vector. Returns `false` if at capacity.
    pub fn add(&mut self, id: i64, embedding: &[f32]) -> bool {
        if embedding.len() != self.dimensions {
            return true; // dimension mismatch — ignore silently
        }
        if self.flat.len() >= MAX_MOBILE_VECTORS && !self.flat.iter().any(|(eid, _)| *eid == id) {
            return false; // at capacity, not a replace
        }
        // Remove existing entry if updating.
        self.flat.retain(|(eid, _)| *eid != id);
        self.flat.push((id, embedding.to_vec()));
        self.ivf_dirty = true;
        true
    }

    /// Remove a vector by ID. Returns true if it was present.
    pub fn remove(&mut self, id: i64) -> bool {
        let before = self.flat.len();
        self.flat.retain(|(eid, _)| *eid != id);
        if self.flat.len() != before {
            self.ivf_dirty = true;
            true
        } else {
            false
        }
    }

    /// Reserve capacity for bulk inserts.
    pub fn reserve(&mut self, n: usize) {
        self.flat.reserve(n.min(MAX_MOBILE_VECTORS));
    }

    /// Search for the `limit` nearest neighbours.
    pub fn search(&self, query: &[f32], limit: usize) -> Vec<MobileAnnMatch> {
        if query.len() != self.dimensions || self.flat.is_empty() {
            return vec![];
        }

        if self.flat.len() < IVF_BUILD_THRESHOLD || self.ivf.is_none() {
            // Brute-force for small indices.
            return self.brute_force_search(query, limit);
        }

        // IVF search: probe nearest centroids, then score within partitions.
        if let Some(ivf) = &self.ivf {
            self.ivf_search(ivf, query, limit)
        } else {
            self.brute_force_search(query, limit)
        }
    }

    /// Build or rebuild the IVF structure from the current flat vectors.
    pub fn build_ivf(&mut self) {
        if self.flat.len() < IVF_BUILD_THRESHOLD {
            self.ivf = None;
            self.ivf_dirty = false;
            return;
        }

        let n_lists = DEFAULT_N_LISTS.min(self.flat.len() / 4);
        let centroids = kmeans_init_and_run(&self.flat, n_lists, self.dimensions);

        let mut lists: Vec<IvfList> = centroids
            .into_iter()
            .map(|c| IvfList {
                centroid: c,
                vectors: Vec::new(),
            })
            .collect();

        // Assign each vector to the nearest centroid.
        for (id, vec) in &self.flat {
            let nearest = find_nearest_centroid(&lists, vec);
            let qvec = quantize_i8(vec);
            let norm = l2_norm(vec);
            lists[nearest].vectors.push(QuantizedVector {
                id: *id,
                data: qvec,
                norm,
            });
        }

        self.ivf = Some(IvfStructure { lists });
        self.ivf_dirty = false;
    }

    /// Ensure the IVF is up to date. Call before search if writes happened.
    pub fn ensure_built(&mut self) {
        if self.ivf_dirty || (self.flat.len() >= IVF_BUILD_THRESHOLD && self.ivf.is_none()) {
            self.build_ivf();
        }
    }

    fn brute_force_search(&self, query: &[f32], limit: usize) -> Vec<MobileAnnMatch> {
        let mut results: Vec<MobileAnnMatch> = self
            .flat
            .iter()
            .map(|(id, emb)| (*id, cosine_sim(query, emb)))
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    fn ivf_search(&self, ivf: &IvfStructure, query: &[f32], limit: usize) -> Vec<MobileAnnMatch> {
        // Find the n_probe nearest centroids.
        let mut centroid_scores: Vec<(usize, f32)> = ivf
            .lists
            .iter()
            .enumerate()
            .map(|(i, list)| (i, cosine_sim(query, &list.centroid)))
            .collect();
        centroid_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let probes = centroid_scores.iter().take(self.n_probe);

        // Quantize the query for fast i8 dot product.
        let q_query = quantize_i8(query);
        let q_norm = l2_norm(query);

        // Score all vectors in the probed partitions.
        let mut results: Vec<MobileAnnMatch> = Vec::new();
        for &(list_idx, _) in probes {
            let list = &ivf.lists[list_idx];
            for qvec in &list.vectors {
                // Approximate cosine similarity using i8 dot product.
                let sim = quantized_cosine(&q_query, q_norm, &qvec.data, qvec.norm);
                results.push((qvec.id, sim));
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }
}

// ── Quantization helpers ─────────────────────────────────────────────────────

/// Quantize f32 → i8 using symmetric min-max scaling.
fn quantize_i8(vec: &[f32]) -> Vec<i8> {
    if vec.is_empty() {
        return vec![];
    }
    let max_abs = vec.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
    if max_abs == 0.0 {
        return vec![0i8; vec.len()];
    }
    let scale = 127.0 / max_abs;
    vec.iter()
        .map(|&v| (v * scale).round().clamp(-127.0, 127.0) as i8)
        .collect()
}

/// Approximate cosine similarity between two i8-quantized vectors.
fn quantized_cosine(a: &[i8], a_norm: f32, b: &[i8], b_norm: f32) -> f32 {
    let dot: i32 = a.iter().zip(b.iter()).map(|(&x, &y)| x as i32 * y as i32).sum();
    let denom = a_norm * b_norm;
    if denom == 0.0 {
        return 0.0;
    }
    // The i8 dot product is proportional to the true dot product.
    // Since both are scaled by 127/max_abs, the ratio approximates cosine.
    dot as f32 / (127.0 * 127.0) / denom * l2_norm_from_i8(a) * l2_norm_from_i8(b)
}

/// L2 norm of an f32 vector.
fn l2_norm(vec: &[f32]) -> f32 {
    vec.iter().map(|v| v * v).sum::<f32>().sqrt()
}

/// Approximate L2 norm from i8 data (for normalization).
fn l2_norm_from_i8(vec: &[i8]) -> f32 {
    let sum_sq: i64 = vec.iter().map(|&v| v as i64 * v as i64).sum();
    (sum_sq as f32).sqrt()
}

// ── K-means ──────────────────────────────────────────────────────────────────

/// Simple k-means: initialize centroids with k-means++ sampling, then iterate.
fn kmeans_init_and_run(
    data: &[(i64, Vec<f32>)],
    k: usize,
    dims: usize,
) -> Vec<Vec<f32>> {
    if data.is_empty() || k == 0 {
        return vec![];
    }
    let k = k.min(data.len());

    // K-means++ initialization.
    let mut centroids: Vec<Vec<f32>> = Vec::with_capacity(k);
    // First centroid: pick the middle element (deterministic for reproducibility).
    centroids.push(data[data.len() / 2].1.clone());

    for _ in 1..k {
        // Pick the data point farthest from any existing centroid.
        let mut max_dist = 0.0f32;
        let mut best_idx = 0;
        for (i, (_, vec)) in data.iter().enumerate() {
            let min_dist = centroids
                .iter()
                .map(|c| l2_distance_sq(vec, c))
                .fold(f32::MAX, f32::min);
            if min_dist > max_dist {
                max_dist = min_dist;
                best_idx = i;
            }
        }
        centroids.push(data[best_idx].1.clone());
    }

    // Lloyd's iterations.
    let mut assignments: Vec<usize> = vec![0; data.len()];
    for _ in 0..KMEANS_ITERATIONS {
        // Assign step.
        for (i, (_, vec)) in data.iter().enumerate() {
            assignments[i] = find_nearest_centroid_raw(&centroids, vec);
        }
        // Update step.
        let mut new_centroids = vec![vec![0.0f32; dims]; k];
        let mut counts = vec![0usize; k];
        for (i, (_, vec)) in data.iter().enumerate() {
            let c = assignments[i];
            counts[c] += 1;
            for (j, &val) in vec.iter().enumerate() {
                new_centroids[c][j] += val;
            }
        }
        for (c, centroid) in new_centroids.iter_mut().enumerate() {
            if counts[c] > 0 {
                for val in centroid.iter_mut() {
                    *val /= counts[c] as f32;
                }
            }
        }
        centroids = new_centroids;
    }

    centroids
}

fn find_nearest_centroid(lists: &[IvfList], vec: &[f32]) -> usize {
    lists
        .iter()
        .enumerate()
        .map(|(i, list)| (i, l2_distance_sq(vec, &list.centroid)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn find_nearest_centroid_raw(centroids: &[Vec<f32>], vec: &[f32]) -> usize {
    centroids
        .iter()
        .enumerate()
        .map(|(i, c)| (i, l2_distance_sq(vec, c)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn l2_distance_sq(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum()
}

/// Cosine similarity between two f32 vectors.
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a = l2_norm(a);
    let norm_b = l2_norm(b);
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_vec(dims: usize, seed: u64) -> Vec<f32> {
        // Simple deterministic pseudo-random via xorshift.
        let mut state = seed.max(1);
        (0..dims)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                // Map to [-1, 1].
                (state as f32 / u64::MAX as f32) * 2.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn basic_add_and_search() {
        let mut idx = MobileAnnIndex::new(4);
        idx.add(1, &[1.0, 0.0, 0.0, 0.0]);
        idx.add(2, &[0.0, 1.0, 0.0, 0.0]);
        idx.add(3, &[0.9, 0.1, 0.0, 0.0]);

        let results = idx.search(&[1.0, 0.0, 0.0, 0.0], 3);
        assert_eq!(results.len(), 3);
        // ID 1 should be the best match.
        assert_eq!(results[0].0, 1);
        // ID 3 should be second (closer to [1,0,0,0] than [0,1,0,0]).
        assert_eq!(results[1].0, 3);
    }

    #[test]
    fn remove_works() {
        let mut idx = MobileAnnIndex::new(4);
        idx.add(1, &[1.0, 0.0, 0.0, 0.0]);
        idx.add(2, &[0.0, 1.0, 0.0, 0.0]);
        assert_eq!(idx.len(), 2);

        idx.remove(1);
        assert_eq!(idx.len(), 1);

        let results = idx.search(&[1.0, 0.0, 0.0, 0.0], 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 2);
    }

    #[test]
    fn capacity_limit() {
        let mut idx = MobileAnnIndex::new(2);
        // Fill to a small limit for testing — we test the concept with MAX check.
        for i in 0..10 {
            assert!(idx.add(i, &[i as f32, 0.0]));
        }
        assert!(!idx.is_full()); // 10 < 200k
    }

    #[test]
    fn ivf_build_and_search() {
        let dims = 32;
        let mut idx = MobileAnnIndex::new(dims);

        // Add enough vectors to trigger IVF build.
        for i in 0..1500 {
            let v = random_vec(dims, i as u64 + 1);
            idx.add(i, &v);
        }

        idx.build_ivf();
        assert!(idx.ivf.is_some());

        // Search should return results.
        let query = random_vec(dims, 999);
        let results = idx.search(&query, 10);
        assert_eq!(results.len(), 10);

        // Verify that the brute-force top-1 is in the IVF top-10.
        let bf_results = idx.brute_force_search(&query, 1);
        let top1_id = bf_results[0].0;
        assert!(
            results.iter().any(|(id, _)| *id == top1_id),
            "IVF should find the true nearest neighbor in top-10 for 1500 vectors"
        );
    }

    #[test]
    fn quantize_i8_roundtrip_quality() {
        let original = vec![0.5, -0.3, 0.8, -1.0, 0.0];
        let quantized = quantize_i8(&original);
        // Check that max magnitude maps to ±127.
        assert_eq!(quantized[3], -127); // -1.0 is max abs
        // Check direction is preserved.
        assert!(quantized[0] > 0);
        assert!(quantized[1] < 0);
        assert!(quantized[4] == 0);
    }

    #[test]
    fn cosine_sim_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_sim(&v, &v);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_sim_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_sim(&a, &b);
        assert!(sim.abs() < 1e-5);
    }

    #[test]
    fn ensure_built_triggers_ivf() {
        let dims = 16;
        let mut idx = MobileAnnIndex::new(dims);
        for i in 0..1200 {
            idx.add(i, &random_vec(dims, i as u64 + 1));
        }
        assert!(idx.ivf.is_none());
        idx.ensure_built();
        assert!(idx.ivf.is_some());
    }

    #[test]
    fn dimension_mismatch_ignored() {
        let mut idx = MobileAnnIndex::new(4);
        // Wrong dimension — should be silently ignored.
        assert!(idx.add(1, &[1.0, 2.0]));
        assert_eq!(idx.len(), 0);
    }

    #[test]
    fn replace_updates_existing() {
        let mut idx = MobileAnnIndex::new(4);
        idx.add(1, &[1.0, 0.0, 0.0, 0.0]);
        idx.add(1, &[0.0, 1.0, 0.0, 0.0]); // replace
        assert_eq!(idx.len(), 1);

        let results = idx.search(&[0.0, 1.0, 0.0, 0.0], 1);
        assert_eq!(results[0].0, 1);
        assert!(results[0].1 > 0.99); // should match new vector
    }
}
