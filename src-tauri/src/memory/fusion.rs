//! Reciprocal Rank Fusion (RRF) — rank-based fusion of multiple ranked
//! candidate lists.
//!
//! RRF is the standard fusion algorithm used in modern hybrid retrieval
//! pipelines (vector + keyword + graph + reranker). It is robust to
//! score-scale mismatch between retrievers because it operates purely on
//! *rank position*, not on raw scores.
//!
//! Formula (per Cormack, Clarke & Büttcher, SIGIR 2009):
//!
//! ```text
//! RRF(d) = Σ  1 / (k + rank_i(d))
//!         i∈R
//! ```
//!
//! where `R` is the set of input rankings, `rank_i(d)` is the 1-based rank
//! of document `d` in ranking `i` (or `∞` if absent), and `k` is a constant
//! dampening factor. The original paper recommends `k = 60` and that value
//! is the de-facto standard across LangChain, LlamaIndex, Weaviate, etc.
//!
//! See `docs/brain-advanced-design.md` § 19 (April 2026 Research Survey,
//! row 2) for how this fits into TerranSoul's modern-RAG roadmap.

/// The constant `k` recommended by the original RRF paper. Higher values
/// flatten the influence of top ranks; lower values reward top-of-list hits
/// more aggressively. `60` is the universal default in production systems.
pub const DEFAULT_RRF_K: usize = 60;

/// Fuse multiple ranked lists of identifiers into a single ranking using
/// Reciprocal Rank Fusion.
///
/// * `rankings` — slice of input rankings; each inner slice is ordered from
///   most-relevant (index 0) to least-relevant (last index). Identifiers
///   may appear in any subset of the input rankings — items missing from a
///   given ranking simply contribute nothing from that ranking.
/// * `k` — RRF dampening constant (use [`DEFAULT_RRF_K`] for the standard
///   value of 60).
///
/// Returns the fused ranking as a `Vec<(id, score)>` sorted by descending
/// fused score. Ordering of items with identical scores is stable: ties are
/// broken by first appearance across the input rankings (then by id), so
/// the result is deterministic across runs.
///
/// # Example
///
/// ```
/// use terransoul_lib::memory::fusion::{reciprocal_rank_fuse, DEFAULT_RRF_K};
///
/// // Vector retriever ranks: A, B, C
/// let vector_rank = vec![1_i64, 2, 3];
/// // Keyword retriever ranks: B, D, A
/// let keyword_rank = vec![2_i64, 4, 1];
///
/// let fused = reciprocal_rank_fuse(&[&vector_rank, &keyword_rank], DEFAULT_RRF_K);
/// // B appears at rank 2 + 1 → highest fused score
/// assert_eq!(fused[0].0, 2);
/// ```
pub fn reciprocal_rank_fuse<T>(rankings: &[&[T]], k: usize) -> Vec<(T, f64)>
where
    T: Copy + Eq + std::hash::Hash + Ord,
{
    use std::collections::HashMap;

    // Track score and first-appearance index for stable tie-breaking.
    let mut scores: HashMap<T, f64> = HashMap::new();
    let mut first_seen: HashMap<T, usize> = HashMap::new();
    let mut order_counter: usize = 0;
    let k_f = k as f64;

    for ranking in rankings {
        for (rank_zero_based, id) in ranking.iter().enumerate() {
            let rank = (rank_zero_based + 1) as f64; // 1-based rank
            *scores.entry(*id).or_insert(0.0) += 1.0 / (k_f + rank);
            first_seen.entry(*id).or_insert_with(|| {
                let idx = order_counter;
                order_counter += 1;
                idx
            });
        }
    }

    let mut fused: Vec<(T, f64)> = scores.into_iter().collect();
    fused.sort_by(|a, b| {
        // Primary: descending score
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            // Secondary (tie-break): first appearance order
            .then_with(|| {
                first_seen
                    .get(&a.0)
                    .copied()
                    .unwrap_or(usize::MAX)
                    .cmp(&first_seen.get(&b.0).copied().unwrap_or(usize::MAX))
            })
            // Tertiary (deterministic): id order
            .then_with(|| a.0.cmp(&b.0))
    });
    fused
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty() {
        let fused: Vec<(i64, f64)> = reciprocal_rank_fuse(&[], DEFAULT_RRF_K);
        assert!(fused.is_empty());
    }

    #[test]
    fn single_ranking_passthrough_preserves_order() {
        let single: &[i64] = &[10, 20, 30, 40];
        let fused = reciprocal_rank_fuse(&[single], DEFAULT_RRF_K);
        assert_eq!(fused.len(), 4);
        let ids: Vec<i64> = fused.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![10, 20, 30, 40]);
        // Scores must be strictly descending for a single passthrough.
        for w in fused.windows(2) {
            assert!(
                w[0].1 > w[1].1,
                "expected descending scores, got {:?}",
                fused
            );
        }
    }

    #[test]
    fn fuses_two_rankings_with_overlap() {
        let vector_rank: &[i64] = &[1, 2, 3]; // A, B, C
        let keyword_rank: &[i64] = &[2, 4, 1]; // B, D, A
        let fused = reciprocal_rank_fuse(&[vector_rank, keyword_rank], DEFAULT_RRF_K);

        // Expected fused scores (k = 60):
        //   id 1 (A): 1/(60+1) + 1/(60+3) = 0.016393 + 0.015873 = 0.032266
        //   id 2 (B): 1/(60+2) + 1/(60+1) = 0.016129 + 0.016393 = 0.032522
        //   id 3 (C): 1/(60+3)            = 0.015873
        //   id 4 (D): 1/(60+2)            = 0.016129
        // Order: B > A > D > C
        let ids: Vec<i64> = fused.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![2, 1, 4, 3]);
    }

    #[test]
    fn missing_from_some_rankings_still_scores() {
        // Item only in ranking 1, item only in ranking 2.
        let r1: &[i64] = &[7];
        let r2: &[i64] = &[8];
        let fused = reciprocal_rank_fuse(&[r1, r2], DEFAULT_RRF_K);
        assert_eq!(fused.len(), 2);
        // Both at rank 1 → identical score → stable order by first appearance (7 then 8).
        assert!((fused[0].1 - fused[1].1).abs() < 1e-12);
        assert_eq!(fused[0].0, 7);
        assert_eq!(fused[1].0, 8);
    }

    #[test]
    fn ties_break_by_first_appearance_then_id() {
        // Two items appearing only at rank 1 in independent rankings → tie.
        let r1: &[i64] = &[100];
        let r2: &[i64] = &[50];
        let fused = reciprocal_rank_fuse(&[r1, r2], DEFAULT_RRF_K);
        // 100 was seen first.
        assert_eq!(fused[0].0, 100);
        assert_eq!(fused[1].0, 50);
    }

    #[test]
    fn three_rankings_compose_correctly() {
        // A appears at rank 1 in all three rankings → must clearly win.
        let r1: &[i64] = &[1, 2, 3];
        let r2: &[i64] = &[1, 3, 4];
        let r3: &[i64] = &[1, 5];
        let fused = reciprocal_rank_fuse(&[r1, r2, r3], DEFAULT_RRF_K);
        assert_eq!(fused[0].0, 1);
        // Score for id 1 = 3 / 61
        let expected = 3.0 / 61.0;
        assert!((fused[0].1 - expected).abs() < 1e-12);
    }

    #[test]
    fn k_value_changes_score_magnitude_not_order() {
        // Item 1 appears at rank 1 in both rankings → unambiguous winner.
        // Item 2 appears at rank 2 in both rankings → second.
        // Item 3 appears at rank 3 in both rankings → third.
        let r1: &[i64] = &[1, 2, 3];
        let r2: &[i64] = &[1, 2, 3];
        let fused_default = reciprocal_rank_fuse(&[r1, r2], DEFAULT_RRF_K);
        let fused_small_k = reciprocal_rank_fuse(&[r1, r2], 1);
        let order_default: Vec<i64> = fused_default.iter().map(|(id, _)| *id).collect();
        let order_small_k: Vec<i64> = fused_small_k.iter().map(|(id, _)| *id).collect();
        assert_eq!(order_default, vec![1, 2, 3]);
        assert_eq!(order_small_k, vec![1, 2, 3]);
        // Smaller k amplifies the score gap between top and bottom.
        let gap_default = fused_default[0].1 - fused_default[2].1;
        let gap_small_k = fused_small_k[0].1 - fused_small_k[2].1;
        assert!(
            gap_small_k > gap_default,
            "smaller k should widen the top-vs-bottom score gap (default={gap_default}, k=1={gap_small_k})"
        );
    }

    #[test]
    fn deterministic_across_runs() {
        let r1: &[i64] = &[1, 2, 3, 4, 5];
        let r2: &[i64] = &[5, 4, 3, 2, 1];
        let a = reciprocal_rank_fuse(&[r1, r2], DEFAULT_RRF_K);
        let b = reciprocal_rank_fuse(&[r1, r2], DEFAULT_RRF_K);
        let c = reciprocal_rank_fuse(&[r1, r2], DEFAULT_RRF_K);
        assert_eq!(a, b);
        assert_eq!(b, c);
    }
}
