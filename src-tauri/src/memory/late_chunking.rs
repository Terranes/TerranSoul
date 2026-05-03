//! Late Chunking utility — Chunk 16.3a.
//!
//! "Late chunking" (Günther et al., 2024 — Jina AI) flips the
//! traditional embed pipeline:
//!
//! - **Naive chunking**: split document → embed each chunk
//!   independently → store. Each chunk's embedding only sees its own
//!   ~500 tokens of context.
//! - **Late chunking**: embed the **whole document** in one pass with
//!   a long-context model → split into chunks → **mean-pool** each
//!   chunk's per-token embeddings → store. Each chunk's embedding now
//!   carries the global document context (e.g. "this paragraph is
//!   about Alice, who was introduced two pages ago").
//!
//! The improvement on hard cross-reference queries is meaningful
//! (the Jina paper reports ~7 % nDCG@10 lifts on long-document
//! retrieval benchmarks). It costs nothing at retrieval time — the
//! storage shape is identical to naive chunking — and only changes
//! the *embedding* path.
//!
//! ## What this chunk ships (16.3a)
//!
//! The pure pooling utility:
//!
//! - [`mean_pool_token_embeddings`] — given per-token embeddings + a
//!   character span, return the L2-normalised mean embedding for that
//!   span. The character→token alignment is handled by
//!   [`TokenSpan`] which the caller supplies.
//! - [`pool_chunks`] — apply the above to every `(chunk_text,
//!   char_span)` pair in one call, returning embeddings aligned with
//!   the chunk list.
//!
//! ## What this chunk does *not* ship (deferred to 16.3b)
//!
//! - The actual long-context embedder. Ollama's current default
//!   (`nomic-embed-text`) returns a single pooled embedding per
//!   request — it does **not** expose per-token vectors. Wiring late
//!   chunking end-to-end requires either (a) an Ollama model that
//!   exposes per-token embeddings (e.g. `jina-embeddings-v3` once
//!   available locally), or (b) an HTTP embedding endpoint that
//!   returns the per-token tensor. Both are external to TerranSoul.
//! - The ingest-pipeline integration. `run_ingest_task` currently
//!   embeds each chunk independently; flipping it to late-chunking
//!   mode is a one-line change once the per-token embedder is
//!   available, gated behind an `AppSettings.late_chunking` flag
//!   following the same pattern as `contextual_retrieval`.
//!
//! Splitting this way keeps 16.3a a pure, fully-tested module that
//! the integration chunk can compose without re-engineering anything.

use serde::{Deserialize, Serialize};

/// Inclusive-start, exclusive-end text range for a token or chunk.
///
/// Offsets are UTF-8 byte offsets into the original document string.
/// They must be valid string boundaries when produced by Rust code, but
/// this module only compares ranges and does not slice by them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharSpan {
    pub start: usize,
    pub end: usize,
}

impl CharSpan {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    fn overlaps(&self, other: CharSpan) -> bool {
        !self.is_empty() && !other.is_empty() && self.start < other.end && other.start < self.end
    }
}

/// Inclusive-start, exclusive-end token range covering a chunk.
///
/// `start` and `end` are indices into the per-token embedding array
/// returned by the long-context embedder. Empty ranges (`start ==
/// end`) represent zero-token chunks and are skipped during pooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenSpan {
    pub start: usize,
    pub end: usize,
}

impl TokenSpan {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

/// Mean-pool the per-token embeddings covering `span`, then
/// L2-renormalise the result.
///
/// Returns `None` when:
/// - `span` is empty (`start >= end`),
/// - `span.end > token_embeddings.len()` (out-of-range),
/// - any token in the span has a different dimensionality than the
///   first one (corrupt input — embedders should never do this, but
///   we guard against it),
/// - or the resulting mean has zero L2 norm.
///
/// Why L2-renormalise? The downstream consumer is cosine similarity
/// (`memory::store::cosine_similarity` / `memory::matryoshka`).
/// Pooling raw embeddings produces a vector whose magnitude grows
/// roughly with √(span.len), which would bias scores. Unit-norm
/// outputs make the pooled chunk embedding numerically comparable to
/// any other unit-norm embedding in the store.
pub fn mean_pool_token_embeddings(
    token_embeddings: &[Vec<f32>],
    span: TokenSpan,
) -> Option<Vec<f32>> {
    if span.is_empty() {
        return None;
    }
    if span.end > token_embeddings.len() {
        return None;
    }

    let dim = token_embeddings[span.start].len();
    if dim == 0 {
        return None;
    }

    let mut sum = vec![0.0f64; dim];
    let mut count: usize = 0;
    for token in &token_embeddings[span.start..span.end] {
        if token.len() != dim {
            // Dimensionality drift mid-document — refuse rather than
            // silently truncate / pad.
            return None;
        }
        for (acc, &x) in sum.iter_mut().zip(token.iter()) {
            *acc += x as f64;
        }
        count += 1;
    }

    if count == 0 {
        return None;
    }

    let inv_count = 1.0 / count as f64;
    let mean: Vec<f32> = sum.iter().map(|&s| (s * inv_count) as f32).collect();

    // L2 renormalise.
    let norm_sq: f64 = mean.iter().map(|&x| (x as f64) * (x as f64)).sum();
    if norm_sq < 1e-24 {
        return None;
    }
    let norm = norm_sq.sqrt() as f32;
    Some(mean.iter().map(|&x| x / norm).collect())
}

/// Pool every chunk in one pass.
///
/// Returns a `Vec<Option<Vec<f32>>>` aligned with `spans` — `None`
/// entries correspond to chunks whose pooling failed (per the rules
/// in [`mean_pool_token_embeddings`]). Aligning the result with the
/// input means the caller can zip with the original chunk metadata
/// without bookkeeping.
pub fn pool_chunks(token_embeddings: &[Vec<f32>], spans: &[TokenSpan]) -> Vec<Option<Vec<f32>>> {
    spans
        .iter()
        .map(|&s| mean_pool_token_embeddings(token_embeddings, s))
        .collect()
}

/// Convenience: build a contiguous, gap-free `Vec<TokenSpan>` from a
/// slice of token counts. Useful when the caller already knows how
/// many tokens each chunk consumed.
///
/// `[3, 5, 2]` → `[(0,3), (3,8), (8,10)]`.
pub fn spans_from_token_counts(token_counts: &[usize]) -> Vec<TokenSpan> {
    let mut spans = Vec::with_capacity(token_counts.len());
    let mut cursor = 0;
    for &n in token_counts {
        spans.push(TokenSpan::new(cursor, cursor + n));
        cursor += n;
    }
    spans
}

/// Convert document-level chunk text spans into token-index spans.
///
/// A token is considered part of a chunk when its character span overlaps
/// the chunk span. Missing chunk spans map to an empty [`TokenSpan`], which
/// lets callers pass the result directly to [`pool_chunks`] and receive
/// `None` for unalignable chunks.
pub fn token_spans_for_char_spans(
    token_char_spans: &[CharSpan],
    chunk_char_spans: &[Option<CharSpan>],
) -> Vec<TokenSpan> {
    chunk_char_spans
        .iter()
        .map(|chunk_span| {
            let Some(chunk_span) = chunk_span else {
                return TokenSpan::new(0, 0);
            };
            if chunk_span.is_empty() {
                return TokenSpan::new(0, 0);
            }

            let first = token_char_spans
                .iter()
                .position(|token_span| token_span.overlaps(*chunk_span));
            let Some(first) = first else {
                return TokenSpan::new(0, 0);
            };
            let last = token_char_spans
                .iter()
                .rposition(|token_span| token_span.overlaps(*chunk_span))
                .unwrap_or(first);
            TokenSpan::new(first, last + 1)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    fn unit_norm_sq(v: &[f32]) -> f32 {
        v.iter().map(|x| x * x).sum()
    }

    #[test]
    fn token_span_basics() {
        let s = TokenSpan::new(2, 5);
        assert_eq!(s.start, 2);
        assert_eq!(s.end, 5);
        assert_eq!(s.len(), 3);
        assert!(!s.is_empty());

        let empty = TokenSpan::new(4, 4);
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let inverted = TokenSpan::new(5, 3);
        assert!(inverted.is_empty());
    }

    #[test]
    fn char_span_basics() {
        let s = CharSpan::new(10, 15);
        assert_eq!(s.start, 10);
        assert_eq!(s.end, 15);
        assert_eq!(s.len(), 5);
        assert!(!s.is_empty());
        assert!(CharSpan::new(4, 4).is_empty());
        assert!(s.overlaps(CharSpan::new(12, 20)));
        assert!(!s.overlaps(CharSpan::new(15, 20)));
    }

    #[test]
    fn pool_single_token_returns_normalized_input() {
        // Pooling a single token = renormalising that token. The
        // direction must be preserved.
        let tokens = vec![vec![3.0, 4.0]];
        let pooled = mean_pool_token_embeddings(&tokens, TokenSpan::new(0, 1)).unwrap();
        assert_eq!(pooled.len(), 2);
        assert!(approx_eq(pooled[0], 0.6, 1e-6));
        assert!(approx_eq(pooled[1], 0.8, 1e-6));
        assert!(approx_eq(unit_norm_sq(&pooled), 1.0, 1e-6));
    }

    #[test]
    fn pool_averages_then_normalises() {
        // Two tokens that average to (0.5, 0.0) → normalised → (1, 0).
        let tokens = vec![vec![1.0, 0.0], vec![0.0, 0.0]];
        let pooled = mean_pool_token_embeddings(&tokens, TokenSpan::new(0, 2)).unwrap();
        assert!(approx_eq(pooled[0], 1.0, 1e-6));
        assert!(approx_eq(pooled[1], 0.0, 1e-6));
        assert!(approx_eq(unit_norm_sq(&pooled), 1.0, 1e-6));
    }

    #[test]
    fn pool_orthogonal_tokens_yields_45_degree_vector() {
        // (1,0) + (0,1) → mean (0.5, 0.5) → normalised (1/√2, 1/√2).
        let tokens = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let pooled = mean_pool_token_embeddings(&tokens, TokenSpan::new(0, 2)).unwrap();
        let inv_sqrt2 = 1.0 / 2.0_f32.sqrt();
        assert!(approx_eq(pooled[0], inv_sqrt2, 1e-6));
        assert!(approx_eq(pooled[1], inv_sqrt2, 1e-6));
    }

    #[test]
    fn pool_rejects_empty_span() {
        let tokens = vec![vec![1.0, 0.0]];
        assert!(mean_pool_token_embeddings(&tokens, TokenSpan::new(0, 0)).is_none());
        assert!(mean_pool_token_embeddings(&tokens, TokenSpan::new(2, 2)).is_none());
    }

    #[test]
    fn pool_rejects_out_of_range_span() {
        let tokens = vec![vec![1.0, 0.0]];
        assert!(mean_pool_token_embeddings(&tokens, TokenSpan::new(0, 5)).is_none());
    }

    #[test]
    fn pool_rejects_dimension_mismatch() {
        let tokens = vec![vec![1.0, 0.0], vec![1.0, 0.0, 0.0]];
        // First token has dim 2; second has dim 3 → refuse rather
        // than pad/truncate.
        assert!(mean_pool_token_embeddings(&tokens, TokenSpan::new(0, 2)).is_none());
    }

    #[test]
    fn pool_rejects_zero_norm_mean() {
        // Two tokens that cancel to zero → no meaningful direction.
        let tokens = vec![vec![1.0, 0.0], vec![-1.0, 0.0]];
        assert!(mean_pool_token_embeddings(&tokens, TokenSpan::new(0, 2)).is_none());
    }

    #[test]
    fn pool_rejects_zero_dim_tokens() {
        let tokens = vec![vec![]; 3];
        assert!(mean_pool_token_embeddings(&tokens, TokenSpan::new(0, 3)).is_none());
    }

    #[test]
    fn pool_chunks_aligns_with_spans() {
        let tokens = vec![
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 0.0],
            vec![1.0, 0.0],
        ];
        let spans = vec![
            TokenSpan::new(0, 2),
            TokenSpan::new(2, 4),
            TokenSpan::new(0, 0), // empty → None
        ];
        let pooled = pool_chunks(&tokens, &spans);
        assert_eq!(pooled.len(), 3);
        assert!(pooled[0].is_some());
        assert!(pooled[1].is_some());
        assert!(pooled[2].is_none());

        // Chunk 1 is two identical (1,0) tokens → still (1,0).
        let p1 = pooled[1].as_ref().unwrap();
        assert!(approx_eq(p1[0], 1.0, 1e-6));
        assert!(approx_eq(p1[1], 0.0, 1e-6));
    }

    #[test]
    fn spans_from_counts_round_trip() {
        let spans = spans_from_token_counts(&[3, 5, 2]);
        assert_eq!(
            spans,
            vec![
                TokenSpan::new(0, 3),
                TokenSpan::new(3, 8),
                TokenSpan::new(8, 10),
            ]
        );
    }

    #[test]
    fn spans_from_counts_handles_empty_counts() {
        // A zero-token chunk in the middle should still produce an
        // empty span at the right cursor position.
        let spans = spans_from_token_counts(&[2, 0, 3]);
        assert_eq!(
            spans,
            vec![
                TokenSpan::new(0, 2),
                TokenSpan::new(2, 2),
                TokenSpan::new(2, 5),
            ]
        );
        assert!(spans[1].is_empty());
    }

    #[test]
    fn spans_from_counts_empty_input_empty_output() {
        assert!(spans_from_token_counts(&[]).is_empty());
    }

    #[test]
    fn token_spans_for_char_spans_maps_overlapping_tokens() {
        let token_chars = vec![
            CharSpan::new(0, 5),
            CharSpan::new(5, 6),
            CharSpan::new(6, 11),
            CharSpan::new(11, 12),
            CharSpan::new(12, 17),
        ];
        let chunks = vec![Some(CharSpan::new(0, 11)), Some(CharSpan::new(12, 17))];
        let spans = token_spans_for_char_spans(&token_chars, &chunks);
        assert_eq!(spans, vec![TokenSpan::new(0, 3), TokenSpan::new(4, 5)]);
    }

    #[test]
    fn token_spans_for_char_spans_returns_empty_for_unaligned_chunks() {
        let token_chars = vec![CharSpan::new(0, 5), CharSpan::new(6, 10)];
        let chunks = vec![
            None,
            Some(CharSpan::new(10, 10)),
            Some(CharSpan::new(20, 25)),
        ];
        let spans = token_spans_for_char_spans(&token_chars, &chunks);
        assert_eq!(
            spans,
            vec![
                TokenSpan::new(0, 0),
                TokenSpan::new(0, 0),
                TokenSpan::new(0, 0),
            ]
        );
    }

    #[test]
    fn pooled_output_is_unit_norm() {
        // No matter what the token magnitudes are, the pooled output
        // must be unit-norm so it's directly comparable via cosine.
        let tokens = vec![vec![100.0, 200.0], vec![3.0, 4.0], vec![0.5, 0.5]];
        let pooled = mean_pool_token_embeddings(&tokens, TokenSpan::new(0, 3)).unwrap();
        assert!(
            approx_eq(unit_norm_sq(&pooled), 1.0, 1e-6),
            "pooled vector not unit norm: {pooled:?}"
        );
    }

    #[test]
    fn pool_partial_span() {
        // Pool only tokens 1..3 of a 4-token sequence.
        let tokens = vec![
            vec![10.0, 0.0], // skipped
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![10.0, 10.0], // skipped
        ];
        let pooled = mean_pool_token_embeddings(&tokens, TokenSpan::new(1, 3)).unwrap();
        let inv_sqrt2 = 1.0 / 2.0_f32.sqrt();
        assert!(approx_eq(pooled[0], inv_sqrt2, 1e-6));
        assert!(approx_eq(pooled[1], inv_sqrt2, 1e-6));
    }
}
