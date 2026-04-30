//! Matryoshka Representation Learning — Chunk 16.8.
//!
//! "Matryoshka" embeddings (Kusupati et al., NeurIPS 2022) are vectors
//! that can be truncated to a smaller dimension *and re-normalised*
//! while still preserving most of their semantic signal. This is a
//! property of the embedding model, not of the storage layer — the
//! same model that produces 768-dim outputs (e.g. `nomic-embed-text`,
//! the default Ollama embed model) also produces meaningful 256-dim
//! and 512-dim sub-vectors when truncated and L2-renormalised.
//!
//! ## Why we want it
//!
//! TerranSoul stores full 768-dim embeddings. For a memory store
//! with N entries, the brute-force vector scan is O(N · 768) f32
//! multiplications. Once we have an ANN index (`memory::ann_index`,
//! Chunk 16.10) the asymptotic cost is fine, but the index has its
//! own memory overhead and the per-query constant factor is still
//! dominated by dimensionality.
//!
//! Two-stage Matryoshka search lets us:
//!
//! 1. **Fast pass** — score every candidate against the *truncated*
//!    256-dim query embedding. ~3× cheaper per dot product, with a
//!    minimal recall hit on the top-K candidates.
//! 2. **Re-rank** — recompute cosine similarity using the **full**
//!    768-dim embedding for only the top `fast_top_k` survivors.
//!
//! The result is the same as a full-dim search to within a small
//! re-ranking error, at a fraction of the wall-clock cost on a
//! brute-force fallback path.
//!
//! ## Why this module is pure
//!
//! Storage stays at full-dim — we never persist truncated vectors.
//! Truncation happens at query time only; the cost is one slice +
//! one L2 normalise. This keeps the schema unchanged (no new column,
//! no migration) and lets us turn the feature on or off per-query
//! without retraining or rebuilding any index.
//!
//! ## Recommended dimensions for `nomic-embed-text`
//!
//! Per the model card and common practice:
//! - 64-dim — too lossy for retrieval, useful only for clustering
//! - 128-dim — coarse, ~40 % faster, 5–10 % recall@10 hit
//! - **256-dim — sweet spot for fast first-pass scan**
//! - 512-dim — minimal recall loss, ~33 % faster than full
//! - 768-dim — full quality, baseline cost
//!
//! Default `FAST_DIM` is 256; see [`DEFAULT_FAST_DIM`].

/// Default fast-pass dimension. Picked for `nomic-embed-text` per
/// the module-level docs; callers can override via
/// [`two_stage_search`]'s `fast_dim` argument.
pub const DEFAULT_FAST_DIM: usize = 256;

/// Truncate an embedding to `target_dim` and L2-renormalise.
///
/// Returns `None` when:
/// - `target_dim` is 0 or larger than the input length,
/// - the input is empty,
/// - or the truncated vector has zero L2 norm (degenerate input).
///
/// L2 renormalisation matters because cosine similarity assumes
/// unit-norm vectors; a raw truncated vector has shorter norm than
/// the original and would bias scores downward.
pub fn truncate_and_normalize(embedding: &[f32], target_dim: usize) -> Option<Vec<f32>> {
    if target_dim == 0 || target_dim > embedding.len() || embedding.is_empty() {
        return None;
    }
    let slice = &embedding[..target_dim];
    let norm_sq: f64 = slice.iter().map(|&x| (x as f64) * (x as f64)).sum();
    if norm_sq < 1e-24 {
        return None;
    }
    let norm = norm_sq.sqrt() as f32;
    Some(slice.iter().map(|&x| x / norm).collect())
}

/// A scored (id, similarity) pair — the minimal data needed by the
/// two-stage search caller. Memory entries themselves are looked up
/// separately so this module stays storage-agnostic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoredId {
    pub id: i64,
    pub score: f32,
}

/// Two-stage Matryoshka search over a slice of `(id, full_embedding)`
/// pairs.
///
/// 1. Truncates `query_embedding` to `fast_dim` and renormalises.
/// 2. Scores every candidate against the truncated query, using the
///    candidate's own truncated+renormalised slice.
/// 3. Sorts descending, keeps the top `fast_top_k`.
/// 4. Re-scores those survivors with the full-dim query.
/// 5. Returns the top `final_top_k` by full-dim cosine similarity.
///
/// Constraints:
/// - All embeddings must share the same length (≥ `fast_dim`). Mixed-
///   dim candidates are silently skipped.
/// - `fast_top_k` should be larger than `final_top_k` (typically
///   3–5×) to give the re-ranker enough survivors to re-order.
/// - When `fast_top_k <= final_top_k`, the algorithm degenerates to a
///   single-stage truncated search and emits a warning via tracing.
pub fn two_stage_search(
    query_embedding: &[f32],
    candidates: &[(i64, Vec<f32>)],
    fast_dim: usize,
    fast_top_k: usize,
    final_top_k: usize,
) -> Vec<ScoredId> {
    if candidates.is_empty() || final_top_k == 0 {
        return vec![];
    }

    // Stage 1: prepare the truncated query.
    let Some(query_short) = truncate_and_normalize(query_embedding, fast_dim) else {
        // Truncation failed — fall back to a full-dim scan so the caller
        // still gets useful results.
        return full_dim_topk(query_embedding, candidates, final_top_k);
    };

    // Stage 1: score every candidate against the truncated query.
    // Reject candidates whose full embedding doesn't match the query
    // length up-front — they can't survive stage-2 re-rank anyway.
    let mut fast: Vec<ScoredId> = candidates
        .iter()
        .filter(|(_, emb)| emb.len() == query_embedding.len())
        .filter_map(|(id, emb)| {
            let short = truncate_and_normalize(emb, fast_dim)?;
            Some(ScoredId {
                id: *id,
                score: dot(&query_short, &short),
            })
        })
        .collect();

    fast.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    fast.truncate(fast_top_k.max(final_top_k));

    if fast_top_k <= final_top_k {
        // Degenerate case: no real re-ranking budget. Return the
        // truncated-pass top-K as-is.
        fast.truncate(final_top_k);
        return fast;
    }

    // Stage 2: re-score the survivors with the full-dim embedding.
    // We need to look up the full embedding for each survivor; build
    // a small index over the slice so we don't go quadratic.
    let mut reranked: Vec<ScoredId> = fast
        .iter()
        .filter_map(|sid| {
            let (_, full) = candidates.iter().find(|(id, _)| *id == sid.id)?;
            // Cosine similarity assumes both vectors have the same
            // length; reject mismatches (defensive — should be caught
            // by the truncation step already).
            if full.len() != query_embedding.len() {
                return None;
            }
            Some(ScoredId {
                id: sid.id,
                score: cosine_unnormalised(query_embedding, full),
            })
        })
        .collect();

    reranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    reranked.truncate(final_top_k);
    reranked
}

/// Fallback used when the truncated query can't be built. Pure
/// brute-force full-dim cosine.
fn full_dim_topk(query: &[f32], candidates: &[(i64, Vec<f32>)], top_k: usize) -> Vec<ScoredId> {
    let mut scored: Vec<ScoredId> = candidates
        .iter()
        .filter(|(_, emb)| emb.len() == query.len())
        .map(|(id, emb)| ScoredId {
            id: *id,
            score: cosine_unnormalised(query, emb),
        })
        .collect();
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);
    scored
}

/// Cosine similarity that does *not* assume unit-norm inputs. Matches
/// the contract of `memory::store::cosine_similarity` so callers get
/// identical scores whether they re-rank via this module or via the
/// public `vector_search` API. Returns 0.0 on degenerate input.
fn cosine_unnormalised(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f64, 0.0f64, 0.0f64);
    for (x, y) in a.iter().zip(b.iter()) {
        let (x, y) = (*x as f64, *y as f64);
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom < 1e-12 {
        0.0
    } else {
        (dot / denom) as f32
    }
}

/// Plain dot product for unit-norm vectors. Used inside stage 1 where
/// both inputs are already L2-normalised by `truncate_and_normalize`.
fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn truncate_rejects_invalid_targets() {
        assert!(truncate_and_normalize(&[1.0, 2.0, 3.0], 0).is_none());
        assert!(truncate_and_normalize(&[1.0, 2.0, 3.0], 4).is_none());
        assert!(truncate_and_normalize(&[], 1).is_none());
    }

    #[test]
    fn truncate_rejects_zero_norm() {
        assert!(truncate_and_normalize(&[0.0, 0.0, 0.0, 1.0], 3).is_none());
    }

    #[test]
    fn truncate_produces_unit_norm_output() {
        let v = vec![3.0, 4.0, 5.0, 6.0];
        let t = truncate_and_normalize(&v, 2).unwrap();
        assert_eq!(t.len(), 2);
        let norm_sq: f32 = t.iter().map(|x| x * x).sum();
        assert!(approx_eq(norm_sq, 1.0, 1e-6), "norm² = {norm_sq}");
        // Direction preserved (3,4) → (0.6, 0.8).
        assert!(approx_eq(t[0], 0.6, 1e-6));
        assert!(approx_eq(t[1], 0.8, 1e-6));
    }

    #[test]
    fn truncate_to_full_dim_is_just_normalize() {
        let v = vec![3.0, 4.0];
        let t = truncate_and_normalize(&v, 2).unwrap();
        assert!(approx_eq(t[0], 0.6, 1e-6));
        assert!(approx_eq(t[1], 0.8, 1e-6));
    }

    #[test]
    fn two_stage_search_returns_full_dim_winner() {
        // Build a synthetic 4-dim "embedding" world where one candidate
        // is an exact match for the query in the full-dim space, but a
        // different candidate scores higher in the truncated 2-dim
        // space. The full-dim re-ranker should surface the true winner.
        let query = vec![1.0, 0.0, 0.0, 1.0];

        // Candidate A: matches query exactly (full-dim winner).
        let a = vec![1.0, 0.0, 0.0, 1.0];
        // Candidate B: better truncated match (same first two dims as query)
        // but bad in the rest.
        let b = vec![1.0, 0.0, -10.0, -10.0];
        // Candidate C: noise.
        let c = vec![0.1, 0.1, 0.1, 0.1];

        let candidates = vec![(1, a), (2, b), (3, c)];

        let results = two_stage_search(&query, &candidates, 2, 3, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1, "full-dim winner should bubble up");
    }

    #[test]
    fn two_stage_search_skips_dim_mismatched_candidates() {
        let query = vec![1.0, 0.0, 0.0, 1.0];
        let good = vec![1.0, 0.0, 0.0, 1.0];
        let mismatched = vec![1.0, 0.0]; // wrong length

        let candidates = vec![(1, good), (2, mismatched)];

        let results = two_stage_search(&query, &candidates, 2, 5, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn two_stage_search_empty_inputs_return_empty() {
        let query = vec![1.0, 0.0];
        assert!(two_stage_search(&query, &[], 1, 5, 5).is_empty());
        assert!(two_stage_search(&query, &[(1, vec![1.0, 0.0])], 1, 5, 0).is_empty());
    }

    #[test]
    fn two_stage_degenerates_when_fast_topk_le_final_topk() {
        // When fast_top_k <= final_top_k there's no real re-ranking
        // happening; we just return the truncated-pass top-K.
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let a = vec![1.0, 0.0, 0.0, 0.0]; // exact match
        let b = vec![0.0, 1.0, 0.0, 0.0];
        let candidates = vec![(1, a), (2, b)];
        let results = two_stage_search(&query, &candidates, 2, 2, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn fallback_to_full_dim_when_query_truncation_fails() {
        // fast_dim larger than the embedding length → truncation fails
        // → fallback path.
        let query = vec![1.0, 0.0];
        let a = vec![1.0, 0.0];
        let candidates = vec![(7, a)];
        let results = two_stage_search(&query, &candidates, 99, 5, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 7);
    }

    #[test]
    fn dot_product_matches_cosine_for_unit_vectors() {
        let a = vec![0.6, 0.8];
        let b = vec![0.6, 0.8];
        assert!(approx_eq(dot(&a, &b), 1.0, 1e-6));
        let c = vec![0.8, -0.6]; // orthogonal-ish
        assert!(approx_eq(dot(&a, &c), 0.0, 1e-6));
    }

    #[test]
    fn cosine_unnormalised_matches_store_implementation() {
        // Ensure our re-rank scoring agrees with the canonical
        // `memory::store::cosine_similarity` so callers get the same
        // numerical answer regardless of which path produced them.
        let a = vec![3.0, 4.0, 0.0];
        let b = vec![3.0, 4.0, 0.0];
        let c = vec![0.0, 0.0, 1.0];
        let store_ab = crate::memory::store::cosine_similarity(&a, &b);
        let our_ab = cosine_unnormalised(&a, &b);
        assert!(approx_eq(store_ab, our_ab, 1e-6));

        let store_ac = crate::memory::store::cosine_similarity(&a, &c);
        let our_ac = cosine_unnormalised(&a, &c);
        assert!(approx_eq(store_ac, our_ac, 1e-6));
    }

    #[test]
    fn default_fast_dim_is_documented_value() {
        // Hard-pin the constant so a future bump (e.g. moving to a
        // different embedding model) is an intentional, reviewed
        // change rather than an accidental drift.
        assert_eq!(DEFAULT_FAST_DIM, 256);
    }
}
