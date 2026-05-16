//! IVF-PQ (Inverted File with Product Quantization) index implementation.
//!
//! Phase BENCH-SCALE-3: Disk-backed ANN for billion-scale retrieval.
//! Implements the standard IVF-PQ algorithm (Jégou et al. 2011):
//!
//! - Coarse quantizer: k-means on full corpus → `nlist` IVF centroids
//! - PQ encoding: divide each residual vector into `pq_m` subspaces,
//!   quantize each subspace to the nearest of 256 centroids (8 bits)
//! - Search: ADC (Asymmetric Distance Computation) with nprobe cells
//!
//! File format: custom binary for fast memory-mapped access.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use super::disk_backed_ann::IvfPqParams;

// ─── Constants ───────────────────────────────────────────────────────────────

/// Magic bytes for IVF-PQ index files.
const IVFPQ_MAGIC: &[u8; 8] = b"TSIVFPQ\x01";

/// File format version.
const IVFPQ_FORMAT_VERSION: u32 = 1;

/// Default number of IVF cells to probe during search.
pub const DEFAULT_NPROBE: usize = 32;

/// Maximum k-means iterations for codebook and centroid training.
const KMEANS_MAX_ITERATIONS: usize = 20;

/// K-means convergence threshold (relative change in inertia).
const KMEANS_CONVERGENCE_THRESHOLD: f64 = 1e-4;

/// Maximum sample size for PQ codebook training.
const PQ_TRAINING_SAMPLE_SIZE: usize = 100_000;

/// File extension for IVF-PQ index files.
pub const IVFPQ_INDEX_SUFFIX: &str = ".ivfpq.bin";

// ─── Types ───────────────────────────────────────────────────────────────────

/// A single entry in an inverted list: memory ID + PQ code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IvfPqEntry {
    pub id: i64,
    /// PQ code: `pq_m` bytes, each byte encodes the nearest centroid index (0–255)
    /// for that subquantizer.
    pub pq_code: Vec<u8>,
}

/// The full IVF-PQ index, holding coarse centroids, PQ codebooks, and inverted lists.
#[derive(Debug, Clone)]
pub struct IvfPqIndex {
    /// Embedding dimensionality.
    pub dim: usize,
    /// IVF-PQ parameters (nlist, pq_m, pq_nbits).
    pub params: IvfPqParams,
    /// Coarse quantizer centroids: `[nlist][dim]` flattened.
    pub coarse_centroids: Vec<f32>,
    /// PQ codebooks: `[pq_m][256][subspace_dim]` flattened per subquantizer.
    /// Each subquantizer's codebook is `256 * subspace_dim` f32s.
    pub pq_codebooks: Vec<Vec<f32>>,
    /// Inverted lists: `[nlist]` lists of (id, pq_code) entries.
    pub inverted_lists: Vec<Vec<IvfPqEntry>>,
    /// Total number of indexed vectors.
    pub total_entries: usize,
}

/// Result of an IVF-PQ search: ID + approximate distance.
#[derive(Debug, Clone)]
pub struct IvfPqSearchResult {
    pub id: i64,
    pub distance: f32,
}

/// Training statistics returned after building an IVF-PQ index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IvfPqBuildStats {
    pub dim: usize,
    pub nlist: usize,
    pub pq_m: usize,
    pub pq_nbits: usize,
    pub total_vectors: usize,
    pub training_sample_size: usize,
    pub coarse_kmeans_iterations: usize,
    pub pq_kmeans_iterations: usize,
    pub build_time_ms: u64,
}

// ─── IVF-PQ Index Implementation ────────────────────────────────────────────

impl IvfPqIndex {
    /// Build an IVF-PQ index from a set of vectors.
    ///
    /// `vectors` is an iterator of `(id, embedding)` pairs.
    /// This performs:
    /// 1. Coarse k-means to create `nlist` IVF centroids
    /// 2. PQ codebook training on residual vectors
    /// 3. Assignment of all vectors to IVF cells + PQ encoding
    pub fn build(
        params: &IvfPqParams,
        dim: usize,
        vectors: Vec<(i64, Vec<f32>)>,
    ) -> Result<(Self, IvfPqBuildStats), String> {
        let start = std::time::Instant::now();

        if vectors.is_empty() {
            return Err("Cannot build IVF-PQ index from empty vector set".into());
        }
        if dim == 0 {
            return Err("Dimension must be > 0".into());
        }
        if !dim.is_multiple_of(params.pq_m) {
            return Err(format!(
                "Dimension {} must be divisible by pq_m {}",
                dim, params.pq_m
            ));
        }

        let nlist = params.nlist;
        let pq_m = params.pq_m;
        let subspace_dim = dim / pq_m;
        let total_vectors = vectors.len();

        // Determine training sample size (cap for efficiency)
        let training_sample = Self::select_training_sample(&vectors, PQ_TRAINING_SAMPLE_SIZE);
        let training_sample_size = training_sample.len();

        // Step 1: Train coarse quantizer (k-means on full embeddings)
        let coarse_train_vecs: Vec<&[f32]> =
            training_sample.iter().map(|(_, v)| v.as_slice()).collect();
        // Cap nlist to available training vectors
        let effective_nlist = nlist.min(coarse_train_vecs.len());
        let (coarse_centroids, coarse_iters) =
            kmeans_train(&coarse_train_vecs, effective_nlist, dim)?;

        // Step 2: Compute residuals for PQ training
        // For each training vector, find nearest coarse centroid and compute residual
        let residuals: Vec<Vec<f32>> = training_sample
            .iter()
            .map(|(_, v)| {
                let nearest =
                    find_nearest_centroid(v, &coarse_centroids, effective_nlist, dim);
                compute_residual(v, &coarse_centroids, nearest, dim)
            })
            .collect();

        // Step 3: Train PQ codebooks on residuals (one codebook per subspace)
        let mut pq_codebooks: Vec<Vec<f32>> = Vec::with_capacity(pq_m);
        let mut pq_iters = 0usize;
        for m in 0..pq_m {
            let start_dim = m * subspace_dim;
            let end_dim = start_dim + subspace_dim;

            // Extract subspace vectors from residuals
            let subspace_vecs: Vec<Vec<f32>> = residuals
                .iter()
                .map(|r| r[start_dim..end_dim].to_vec())
                .collect();
            let subspace_refs: Vec<&[f32]> = subspace_vecs.iter().map(|v| v.as_slice()).collect();

            let (mut codebook, iters) = kmeans_train(&subspace_refs, 256, subspace_dim)?;
            pq_iters = pq_iters.max(iters);

            // Ensure codebook has exactly 256 centroids (pad if fewer training vectors)
            let actual_k = codebook.len() / subspace_dim;
            if actual_k < 256 && actual_k > 0 {
                // Pad by cycling existing centroids
                codebook.reserve((256 - actual_k) * subspace_dim);
                for i in actual_k..256 {
                    let src_offset = (i % actual_k) * subspace_dim;
                    let slice: Vec<f32> =
                        codebook[src_offset..src_offset + subspace_dim].to_vec();
                    codebook.extend_from_slice(&slice);
                }
            }

            pq_codebooks.push(codebook);
        }

        // Step 4: Assign all vectors to IVF cells and encode with PQ
        let mut inverted_lists: Vec<Vec<IvfPqEntry>> = vec![Vec::new(); effective_nlist];

        for (id, vec) in &vectors {
            // Find nearest coarse centroid
            let cell = find_nearest_centroid(vec, &coarse_centroids, effective_nlist, dim);

            // Compute residual
            let residual = compute_residual(vec, &coarse_centroids, cell, dim);

            // PQ encode the residual
            let pq_code = pq_encode(&residual, &pq_codebooks, pq_m, subspace_dim);

            inverted_lists[cell].push(IvfPqEntry { id: *id, pq_code });
        }

        let total_entries = vectors.len();
        let build_time_ms = start.elapsed().as_millis() as u64;

        let stats = IvfPqBuildStats {
            dim,
            nlist: effective_nlist,
            pq_m,
            pq_nbits: params.pq_nbits,
            total_vectors,
            training_sample_size,
            coarse_kmeans_iterations: coarse_iters,
            pq_kmeans_iterations: pq_iters,
            build_time_ms,
        };

        let index = Self {
            dim,
            params: IvfPqParams {
                nlist: effective_nlist,
                pq_m: params.pq_m,
                pq_nbits: params.pq_nbits,
            },
            coarse_centroids,
            pq_codebooks,
            inverted_lists,
            total_entries,
        };

        Ok((index, stats))
    }

    /// Search the IVF-PQ index for the `k` nearest neighbors of `query`.
    ///
    /// Uses Asymmetric Distance Computation (ADC):
    /// - Probe `nprobe` IVF cells closest to query
    /// - Precompute distance tables for each subquantizer
    /// - Score each entry in probed cells using table lookups
    pub fn search(&self, query: &[f32], k: usize, nprobe: usize) -> Vec<IvfPqSearchResult> {
        if query.len() != self.dim {
            return Vec::new();
        }

        let nprobe = nprobe.min(self.params.nlist);
        let pq_m = self.params.pq_m;
        let subspace_dim = self.dim / pq_m;

        // Step 1: Find the nprobe closest IVF cells
        let probe_cells = find_nearest_k_centroids(
            query,
            &self.coarse_centroids,
            self.params.nlist,
            self.dim,
            nprobe,
        );

        // Step 2: Precompute ADC distance lookup tables
        // For each probed cell, compute residual of query w.r.t. that cell's centroid,
        // then build distance table from that residual to all PQ centroids.
        // Optimization: we use the query-to-PQ-centroid distance tables directly
        // (standard ADC: dist(q, x) ≈ dist(q - c_i, pq_decode(code)))
        // For simplicity and correctness, we compute per-cell tables.
        let mut results: Vec<IvfPqSearchResult> = Vec::new();

        for &cell_idx in &probe_cells {
            let list = &self.inverted_lists[cell_idx];
            if list.is_empty() {
                continue;
            }

            // Compute query residual for this cell
            let centroid_offset = cell_idx * self.dim;
            let query_residual: Vec<f32> = query
                .iter()
                .enumerate()
                .map(|(i, &q)| q - self.coarse_centroids[centroid_offset + i])
                .collect();

            // Build distance lookup table: [pq_m][256]
            let dist_table = build_adc_table(&query_residual, &self.pq_codebooks, pq_m, subspace_dim);

            // Score each entry in this cell
            for entry in list {
                let distance = adc_distance(&entry.pq_code, &dist_table, pq_m);
                results.push(IvfPqSearchResult {
                    id: entry.id,
                    distance,
                });
            }
        }

        // Step 3: Partial sort to find top-k
        if results.len() <= k {
            results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
            return results;
        }

        // Use partial sort for efficiency
        results.select_nth_unstable_by(k, |a, b| {
            a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(k);
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Save the IVF-PQ index to a binary file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let mut buf: Vec<u8> = Vec::new();

        // Header
        buf.extend_from_slice(IVFPQ_MAGIC);
        buf.extend_from_slice(&IVFPQ_FORMAT_VERSION.to_le_bytes());
        buf.extend_from_slice(&(self.dim as u32).to_le_bytes());
        buf.extend_from_slice(&(self.params.nlist as u32).to_le_bytes());
        buf.extend_from_slice(&(self.params.pq_m as u32).to_le_bytes());
        buf.extend_from_slice(&(self.params.pq_nbits as u32).to_le_bytes());
        buf.extend_from_slice(&(self.total_entries as u64).to_le_bytes());

        // Coarse centroids: nlist * dim f32s
        for &val in &self.coarse_centroids {
            buf.extend_from_slice(&val.to_le_bytes());
        }

        // PQ codebooks: pq_m codebooks, each 256 * subspace_dim f32s
        for codebook in &self.pq_codebooks {
            for &val in codebook {
                buf.extend_from_slice(&val.to_le_bytes());
            }
        }

        // Inverted lists
        for list in &self.inverted_lists {
            buf.extend_from_slice(&(list.len() as u32).to_le_bytes());
            for entry in list {
                buf.extend_from_slice(&entry.id.to_le_bytes());
                buf.extend_from_slice(&entry.pq_code);
            }
        }

        // Write atomically: write to temp then rename
        let tmp_path = path.with_extension("ivfpq.tmp");
        fs::write(&tmp_path, &buf)
            .map_err(|e| format!("Failed to write IVF-PQ index to {}: {e}", tmp_path.display()))?;
        fs::rename(&tmp_path, path)
            .map_err(|e| format!("Failed to rename IVF-PQ index to {}: {e}", path.display()))?;

        Ok(())
    }

    /// Load an IVF-PQ index from a binary file.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let data = fs::read(path)
            .map_err(|e| format!("Failed to read IVF-PQ index from {}: {e}", path.display()))?;

        let mut cursor = io::Cursor::new(data.as_slice());

        // Read header
        let mut magic = [0u8; 8];
        cursor.read_exact(&mut magic).map_err(|e| format!("Read magic: {e}"))?;
        if &magic != IVFPQ_MAGIC {
            return Err("Invalid IVF-PQ magic bytes".into());
        }

        let version = read_u32(&mut cursor)?;
        if version != IVFPQ_FORMAT_VERSION {
            return Err(format!("Unsupported IVF-PQ format version: {version}"));
        }

        let dim = read_u32(&mut cursor)? as usize;
        let nlist = read_u32(&mut cursor)? as usize;
        let pq_m = read_u32(&mut cursor)? as usize;
        let pq_nbits = read_u32(&mut cursor)? as usize;
        let total_entries = read_u64(&mut cursor)? as usize;

        if dim == 0 || nlist == 0 || pq_m == 0 {
            return Err("Invalid IVF-PQ index parameters".into());
        }
        let subspace_dim = dim / pq_m;

        // Read coarse centroids
        let centroid_count = nlist * dim;
        let mut coarse_centroids = vec![0.0f32; centroid_count];
        for val in &mut coarse_centroids {
            *val = read_f32(&mut cursor)?;
        }

        // Read PQ codebooks
        let mut pq_codebooks: Vec<Vec<f32>> = Vec::with_capacity(pq_m);
        for _ in 0..pq_m {
            let codebook_size = 256 * subspace_dim;
            let mut codebook = vec![0.0f32; codebook_size];
            for val in &mut codebook {
                *val = read_f32(&mut cursor)?;
            }
            pq_codebooks.push(codebook);
        }

        // Read inverted lists
        let mut inverted_lists: Vec<Vec<IvfPqEntry>> = Vec::with_capacity(nlist);
        for _ in 0..nlist {
            let list_len = read_u32(&mut cursor)? as usize;
            let mut list = Vec::with_capacity(list_len);
            for _ in 0..list_len {
                let id = read_i64(&mut cursor)?;
                let mut pq_code = vec![0u8; pq_m];
                cursor.read_exact(&mut pq_code).map_err(|e| format!("Read PQ code: {e}"))?;
                list.push(IvfPqEntry { id, pq_code });
            }
            inverted_lists.push(list);
        }

        let params = IvfPqParams {
            nlist,
            pq_m,
            pq_nbits,
        };

        Ok(Self {
            dim,
            params,
            coarse_centroids,
            pq_codebooks,
            inverted_lists,
            total_entries,
        })
    }

    /// Get the file path for an IVF-PQ index for a given shard.
    pub fn index_path(vectors_dir: &Path, shard: &str) -> PathBuf {
        vectors_dir.join(format!("{shard}{IVFPQ_INDEX_SUFFIX}"))
    }

    /// Check if an IVF-PQ index file exists for a shard.
    pub fn exists(vectors_dir: &Path, shard: &str) -> bool {
        Self::index_path(vectors_dir, shard).exists()
    }

    /// Number of entries in the index.
    pub fn len(&self) -> usize {
        self.total_entries
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.total_entries == 0
    }

    /// Select a training sample from the vector set (uniformly spaced).
    fn select_training_sample(
        vectors: &[(i64, Vec<f32>)],
        max_sample: usize,
    ) -> Vec<(i64, Vec<f32>)> {
        if vectors.len() <= max_sample {
            return vectors.to_vec();
        }
        let step = vectors.len() / max_sample;
        vectors
            .iter()
            .step_by(step)
            .take(max_sample)
            .cloned()
            .collect()
    }
}

// ─── K-Means ─────────────────────────────────────────────────────────────────

/// Public entry point for k-means training (used by `ann_index::build_pq_codebooks`).
pub fn kmeans_train_pub(vectors: &[&[f32]], k: usize, dim: usize) -> Result<(Vec<f32>, usize), String> {
    kmeans_train(vectors, k, dim)
}

/// Train k-means with Lloyd's algorithm.
///
/// Returns `(centroids_flat, iterations_run)`.
/// `centroids_flat` is `[k * dim]` contiguous f32 values.
fn kmeans_train(vectors: &[&[f32]], k: usize, dim: usize) -> Result<(Vec<f32>, usize), String> {
    if vectors.is_empty() {
        return Ok((vec![0.0; k * dim], 0));
    }
    if k == 0 {
        return Err("k must be > 0 for k-means".into());
    }

    // Cap k to available vectors
    let k = k.min(vectors.len());

    // Initialize centroids with k-means++ style initialization
    let mut centroids = kmeans_pp_init(vectors, k, dim);
    let mut assignments = vec![0usize; vectors.len()];
    let mut prev_inertia = f64::MAX;

    let mut iterations = 0;
    for _ in 0..KMEANS_MAX_ITERATIONS {
        iterations += 1;

        // Assignment step: assign each vector to nearest centroid
        let mut inertia = 0.0f64;
        for (i, vec) in vectors.iter().enumerate() {
            let (nearest, dist) = nearest_centroid_with_dist(vec, &centroids, k, dim);
            assignments[i] = nearest;
            inertia += dist as f64;
        }

        // Check convergence
        let relative_change = if prev_inertia > 0.0 {
            (prev_inertia - inertia).abs() / prev_inertia
        } else {
            1.0
        };
        if relative_change < KMEANS_CONVERGENCE_THRESHOLD && iterations > 1 {
            break;
        }
        prev_inertia = inertia;

        // Update step: recompute centroids as mean of assigned vectors
        let mut new_centroids = vec![0.0f32; k * dim];
        let mut counts = vec![0usize; k];

        for (i, vec) in vectors.iter().enumerate() {
            let c = assignments[i];
            counts[c] += 1;
            let offset = c * dim;
            for (d, &val) in vec.iter().enumerate() {
                new_centroids[offset + d] += val;
            }
        }

        // Divide by count; handle empty clusters by keeping old centroid
        for (c, &count) in counts.iter().enumerate() {
            if count > 0 {
                let offset = c * dim;
                let count_f = count as f32;
                for d in 0..dim {
                    new_centroids[offset + d] /= count_f;
                }
            } else {
                // Keep previous centroid for empty clusters
                let offset = c * dim;
                new_centroids[offset..offset + dim]
                    .copy_from_slice(&centroids[offset..offset + dim]);
            }
        }

        centroids = new_centroids;
    }

    Ok((centroids, iterations))
}

/// K-means++ initialization: select initial centroids to be spread out.
fn kmeans_pp_init(vectors: &[&[f32]], k: usize, dim: usize) -> Vec<f32> {
    let mut centroids = vec![0.0f32; k * dim];

    // First centroid: pick the middle vector (deterministic for reproducibility)
    let first_idx = vectors.len() / 2;
    centroids[..dim].copy_from_slice(vectors[first_idx]);

    if k == 1 {
        return centroids;
    }

    // Subsequent centroids: pick the vector with max min-distance to existing centroids
    let mut min_dists = vec![f32::MAX; vectors.len()];

    for c in 1..k {
        // Update min distances with the last added centroid
        let prev_offset = (c - 1) * dim;
        for (i, vec) in vectors.iter().enumerate() {
            let dist = l2_squared(vec, &centroids[prev_offset..prev_offset + dim]);
            if dist < min_dists[i] {
                min_dists[i] = dist;
            }
        }

        // Pick the vector with maximum min-distance (farthest point heuristic)
        let mut best_idx = 0;
        let mut best_dist = -1.0f32;
        for (i, &d) in min_dists.iter().enumerate() {
            if d > best_dist {
                best_dist = d;
                best_idx = i;
            }
        }

        let offset = c * dim;
        centroids[offset..offset + dim].copy_from_slice(vectors[best_idx]);
    }

    centroids
}

// ─── PQ Encoding ─────────────────────────────────────────────────────────────

/// Encode a vector using PQ codebooks.
/// Returns a `pq_m`-byte code where each byte is the nearest centroid index.
fn pq_encode(vector: &[f32], codebooks: &[Vec<f32>], pq_m: usize, subspace_dim: usize) -> Vec<u8> {
    let mut code = vec![0u8; pq_m];
    for m in 0..pq_m {
        let start = m * subspace_dim;
        let end = start + subspace_dim;
        let subvec = &vector[start..end];

        // Find nearest centroid in this subspace's codebook
        let codebook = &codebooks[m];
        let mut best_idx = 0u8;
        let mut best_dist = f32::MAX;

        for c in 0..256usize {
            let centroid_start = c * subspace_dim;
            let centroid = &codebook[centroid_start..centroid_start + subspace_dim];
            let dist = l2_squared(subvec, centroid);
            if dist < best_dist {
                best_dist = dist;
                best_idx = c as u8;
            }
        }

        code[m] = best_idx;
    }
    code
}

// ─── ADC (Asymmetric Distance Computation) ───────────────────────────────────

/// Build the ADC distance lookup table for a query residual.
/// Returns `[pq_m][256]` distances (L2 squared from query subvector to each centroid).
fn build_adc_table(
    query_residual: &[f32],
    codebooks: &[Vec<f32>],
    pq_m: usize,
    subspace_dim: usize,
) -> Vec<Vec<f32>> {
    let mut table = Vec::with_capacity(pq_m);
    for (m, codebook) in codebooks.iter().enumerate().take(pq_m) {
        let start = m * subspace_dim;
        let end = start + subspace_dim;
        let query_sub = &query_residual[start..end];

        let mut dists = vec![0.0f32; 256];
        for (c, dist) in dists.iter_mut().enumerate() {
            let centroid_start = c * subspace_dim;
            let centroid = &codebook[centroid_start..centroid_start + subspace_dim];
            *dist = l2_squared(query_sub, centroid);
        }
        table.push(dists);
    }
    table
}

/// Compute approximate distance from a PQ code using precomputed ADC table.
#[inline]
fn adc_distance(pq_code: &[u8], dist_table: &[Vec<f32>], pq_m: usize) -> f32 {
    let mut dist = 0.0f32;
    for m in 0..pq_m {
        dist += dist_table[m][pq_code[m] as usize];
    }
    dist
}

// ─── Vector Utilities ────────────────────────────────────────────────────────

/// Squared L2 distance between two vectors.
#[inline]
fn l2_squared(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| {
            let d = x - y;
            d * d
        })
        .sum()
}

/// Find the nearest centroid to a vector. Returns the centroid index.
fn find_nearest_centroid(vector: &[f32], centroids: &[f32], k: usize, dim: usize) -> usize {
    let mut best = 0;
    let mut best_dist = f32::MAX;
    for i in 0..k {
        let offset = i * dim;
        let dist = l2_squared(vector, &centroids[offset..offset + dim]);
        if dist < best_dist {
            best_dist = dist;
            best = i;
        }
    }
    best
}

/// Find the nearest centroid and return (index, distance).
fn nearest_centroid_with_dist(vector: &[f32], centroids: &[f32], k: usize, dim: usize) -> (usize, f32) {
    let mut best = 0;
    let mut best_dist = f32::MAX;
    for i in 0..k {
        let offset = i * dim;
        let dist = l2_squared(vector, &centroids[offset..offset + dim]);
        if dist < best_dist {
            best_dist = dist;
            best = i;
        }
    }
    (best, best_dist)
}

/// Find the k nearest centroids. Returns sorted indices (nearest first).
fn find_nearest_k_centroids(
    vector: &[f32],
    centroids: &[f32],
    nlist: usize,
    dim: usize,
    k: usize,
) -> Vec<usize> {
    let mut dists: Vec<(usize, f32)> = (0..nlist)
        .map(|i| {
            let offset = i * dim;
            (i, l2_squared(vector, &centroids[offset..offset + dim]))
        })
        .collect();

    if dists.len() <= k {
        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        return dists.iter().map(|&(idx, _)| idx).collect();
    }

    dists.select_nth_unstable_by(k, |a, b| {
        a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    dists.truncate(k);
    dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    dists.iter().map(|&(idx, _)| idx).collect()
}

/// Compute the residual vector: `vector - centroid[cell_idx]`.
fn compute_residual(vector: &[f32], centroids: &[f32], cell_idx: usize, dim: usize) -> Vec<f32> {
    let offset = cell_idx * dim;
    vector
        .iter()
        .enumerate()
        .map(|(i, &v)| v - centroids[offset + i])
        .collect()
}

// ─── Binary I/O Helpers ──────────────────────────────────────────────────────

fn read_u32(cursor: &mut io::Cursor<&[u8]>) -> Result<u32, String> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf).map_err(|e| format!("Read u32: {e}"))?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(cursor: &mut io::Cursor<&[u8]>) -> Result<u64, String> {
    let mut buf = [0u8; 8];
    cursor.read_exact(&mut buf).map_err(|e| format!("Read u64: {e}"))?;
    Ok(u64::from_le_bytes(buf))
}

fn read_i64(cursor: &mut io::Cursor<&[u8]>) -> Result<i64, String> {
    let mut buf = [0u8; 8];
    cursor.read_exact(&mut buf).map_err(|e| format!("Read i64: {e}"))?;
    Ok(i64::from_le_bytes(buf))
}

fn read_f32(cursor: &mut io::Cursor<&[u8]>) -> Result<f32, String> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf).map_err(|e| format!("Read f32: {e}"))?;
    Ok(f32::from_le_bytes(buf))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_random_vectors(n: usize, dim: usize, seed: u64) -> Vec<(i64, Vec<f32>)> {
        // Simple deterministic pseudo-random (xorshift64)
        let mut state = seed;
        let mut vectors = Vec::with_capacity(n);
        for i in 0..n {
            let mut vec = Vec::with_capacity(dim);
            for _ in 0..dim {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                // Map to [-1, 1]
                let val = (state as i64 as f32) / (i64::MAX as f32);
                vec.push(val);
            }
            vectors.push((i as i64, vec));
        }
        vectors
    }

    #[test]
    fn test_kmeans_basic() {
        // 2D vectors in two clusters
        let vecs: Vec<Vec<f32>> = vec![
            vec![0.0, 0.0],
            vec![0.1, 0.1],
            vec![0.2, 0.0],
            vec![10.0, 10.0],
            vec![10.1, 10.1],
            vec![10.2, 10.0],
        ];
        let refs: Vec<&[f32]> = vecs.iter().map(|v| v.as_slice()).collect();
        let (centroids, iters) = kmeans_train(&refs, 2, 2).unwrap();
        assert!(iters <= KMEANS_MAX_ITERATIONS);

        // Two centroids should be near (0.1, 0.03) and (10.1, 10.03) approximately
        let c0 = (centroids[0], centroids[1]);
        let c1 = (centroids[2], centroids[3]);
        let (low, high) = if c0.0 < c1.0 { (c0, c1) } else { (c1, c0) };
        assert!(low.0 < 1.0, "Low cluster centroid x should be near 0");
        assert!(high.0 > 9.0, "High cluster centroid x should be near 10");
    }

    #[test]
    fn test_pq_encode_decode_roundtrip() {
        let dim = 8;
        let pq_m = 4;
        let subspace_dim = dim / pq_m;

        // Simple codebooks: identity-like (centroid i = [i, i] for each subspace)
        let mut codebooks: Vec<Vec<f32>> = Vec::new();
        for _ in 0..pq_m {
            let mut codebook = vec![0.0f32; 256 * subspace_dim];
            for c in 0..256 {
                for d in 0..subspace_dim {
                    codebook[c * subspace_dim + d] = c as f32;
                }
            }
            codebooks.push(codebook);
        }

        // Vector [5.1, 5.1, 10.9, 10.9, 0.0, 0.0, 255.0, 255.0]
        let vector = vec![5.1, 5.1, 10.9, 10.9, 0.0, 0.0, 255.0, 255.0];
        let code = pq_encode(&vector, &codebooks, pq_m, subspace_dim);
        assert_eq!(code.len(), pq_m);
        assert_eq!(code[0], 5);  // Nearest to [5, 5]
        assert_eq!(code[1], 11); // Nearest to [11, 11]
        assert_eq!(code[2], 0);  // Nearest to [0, 0]
        assert_eq!(code[3], 255); // Nearest to [255, 255]
    }

    #[test]
    fn test_ivfpq_build_and_search() {
        let dim = 16;
        let n = 500;
        let params = IvfPqParams {
            nlist: 8,
            pq_m: 4,
            pq_nbits: 8,
        };

        let vectors = make_random_vectors(n, dim, 42);
        let (index, stats) = IvfPqIndex::build(&params, dim, vectors.clone()).unwrap();

        assert_eq!(index.total_entries, n);
        assert_eq!(stats.dim, dim);
        assert_eq!(stats.nlist, 8);
        assert!(stats.build_time_ms < 60_000); // Should finish quickly

        // Search for a vector that exists in the index
        let query = &vectors[0].1;
        let results = index.search(query, 5, 4);
        assert!(!results.is_empty());
        // The query vector itself should be among top results
        assert!(
            results.iter().any(|r| r.id == 0),
            "Query vector (id=0) should be found in results"
        );
    }

    #[test]
    fn test_ivfpq_save_load_roundtrip() {
        let dim = 16;
        let n = 100;
        let params = IvfPqParams {
            nlist: 4,
            pq_m: 4,
            pq_nbits: 8,
        };

        let vectors = make_random_vectors(n, dim, 123);
        let (index, _) = IvfPqIndex::build(&params, dim, vectors).unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.ivfpq.bin");

        index.save_to_file(&path).unwrap();
        let loaded = IvfPqIndex::load_from_file(&path).unwrap();

        assert_eq!(loaded.dim, index.dim);
        assert_eq!(loaded.params, index.params);
        assert_eq!(loaded.total_entries, index.total_entries);
        assert_eq!(loaded.coarse_centroids.len(), index.coarse_centroids.len());
        assert_eq!(loaded.pq_codebooks.len(), index.pq_codebooks.len());
        assert_eq!(loaded.inverted_lists.len(), index.inverted_lists.len());

        // Verify search produces same results
        let query = vec![0.5f32; dim];
        let r1 = index.search(&query, 5, 4);
        let r2 = loaded.search(&query, 5, 4);
        assert_eq!(r1.len(), r2.len());
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.id, b.id);
            assert!((a.distance - b.distance).abs() < 1e-6);
        }
    }

    #[test]
    fn test_ivfpq_empty_vectors_error() {
        let params = IvfPqParams {
            nlist: 4,
            pq_m: 4,
            pq_nbits: 8,
        };
        let result = IvfPqIndex::build(&params, 16, vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_ivfpq_dimension_mismatch() {
        let params = IvfPqParams {
            nlist: 4,
            pq_m: 5, // 16 not divisible by 5
            pq_nbits: 8,
        };
        let vectors = make_random_vectors(10, 16, 99);
        let result = IvfPqIndex::build(&params, 16, vectors);
        assert!(result.is_err());
    }

    #[test]
    fn test_adc_distance_correctness() {
        let pq_m = 4;
        let subspace_dim = 2;

        // Codebooks where centroid c = [c, c] for each subquantizer
        let mut codebooks: Vec<Vec<f32>> = Vec::new();
        for _ in 0..pq_m {
            let mut cb = vec![0.0f32; 256 * subspace_dim];
            for c in 0..256 {
                for d in 0..subspace_dim {
                    cb[c * subspace_dim + d] = c as f32;
                }
            }
            codebooks.push(cb);
        }

        // Query residual = [1, 1, 2, 2, 3, 3, 4, 4]
        let query_residual = vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0];
        let table = build_adc_table(&query_residual, &codebooks, pq_m, subspace_dim);

        // PQ code = [1, 2, 3, 4] → each subquantizer's centroid is [code, code]
        // Distance for sub 0: L2²([1,1], [1,1]) = 0
        // Distance for sub 1: L2²([2,2], [2,2]) = 0
        // Distance for sub 2: L2²([3,3], [3,3]) = 0
        // Distance for sub 3: L2²([4,4], [4,4]) = 0
        let code = vec![1u8, 2, 3, 4];
        let dist = adc_distance(&code, &table, pq_m);
        assert!((dist - 0.0).abs() < 1e-6, "Exact match should give distance 0");

        // PQ code = [0, 0, 0, 0] → centroids are [0,0] for each
        // Distance for sub 0: L2²([1,1], [0,0]) = 2
        // Distance for sub 1: L2²([2,2], [0,0]) = 8
        // Distance for sub 2: L2²([3,3], [0,0]) = 18
        // Distance for sub 3: L2²([4,4], [0,0]) = 32
        // Total = 60
        let code_zero = vec![0u8, 0, 0, 0];
        let dist_zero = adc_distance(&code_zero, &table, pq_m);
        assert!((dist_zero - 60.0).abs() < 1e-4, "Expected 60, got {dist_zero}");
    }

    #[test]
    fn test_recall_at_10_reasonable() {
        // Build index with structured data (10 clusters of 50 vectors each)
        let dim = 32;
        let params = IvfPqParams {
            nlist: 10,
            pq_m: 8,
            pq_nbits: 8,
        };

        let mut vectors = Vec::new();
        let mut id = 0i64;
        for cluster in 0..10 {
            let center = cluster as f32 * 10.0;
            for _ in 0..50 {
                let mut v = vec![center; dim];
                // Add small perturbation using deterministic pattern
                for (d, slot) in v.iter_mut().enumerate() {
                    *slot += ((id as f32 * 0.1 + d as f32 * 0.01).sin()) * 0.5;
                }
                vectors.push((id, v));
                id += 1;
            }
        }

        let (index, _) = IvfPqIndex::build(&params, dim, vectors.clone()).unwrap();

        // Test recall: for each cluster center query, top-10 results should be
        // mostly from that cluster
        let mut total_correct = 0;
        let total_queries = 10;
        for cluster in 0..total_queries {
            let query = vec![cluster as f32 * 10.0; dim];
            let results = index.search(&query, 10, 4);
            let expected_range = (cluster * 50) as i64..((cluster + 1) * 50) as i64;
            let correct = results
                .iter()
                .filter(|r| expected_range.contains(&r.id))
                .count();
            total_correct += correct;
        }

        let recall = total_correct as f32 / (total_queries * 10) as f32;
        assert!(
            recall > 0.5,
            "Recall@10 should be > 50% for well-separated clusters, got {recall:.2}"
        );
    }
}
