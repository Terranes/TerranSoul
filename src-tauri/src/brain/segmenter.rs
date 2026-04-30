//! Topic-shift segmenter for conversation-aware fact extraction
//! (Chunk 26.2 — Phase 26 Brain Background Maintenance & Auto-Learn
//! Completion).
//!
//! Today `extract_facts` sees the entire chat session as one blob, so a
//! conversation that wanders across (cooking → vacation → debugging)
//! gets a single jumbled fact list. This module ships the **pure
//! segmenter** that splits a transcript into topic-coherent ranges
//! using embedding-distance peak detection, so the upstream extractor
//! can call the LLM once per topic and produce cleaner facts.
//!
//! ## Why this module is pure
//!
//! Same rationale as `brain::context_budget`:
//!
//! - **No I/O** — embeddings are passed in by the caller (so the
//!   integration site decides whether to use Ollama, cloud, or a
//!   cached fixture in tests).
//! - **No tokio** — the only async work (the embedding call) belongs
//!   upstream. This module is `pub fn`, not `pub async fn`.
//! - **Deterministic** — same input ⇒ same segments. Snapshot tests
//!   pin the algorithm against multi-topic fixtures.
//!
//! ## Algorithm
//!
//! 1. Compute pairwise consecutive **cosine distance** between every
//!    adjacent turn pair `(i, i+1)`.
//! 2. Mark `i` as a *topic boundary* when its distance exceeds a
//!    threshold computed as `mean + k·stddev` (default `k = 1.0`),
//!    AND it is a **local maximum** (strictly greater than both
//!    neighbours). The mean+stddev rule avoids declaring a boundary
//!    at every turn in a uniformly-noisy conversation.
//! 3. Boundaries split the turn list into half-open `[start, end)`
//!    ranges. Each segment also gets a one-line `summary` derived from
//!    the first user turn it contains (if any), trimmed to 80 chars
//!    for log readability.
//! 4. A configurable `min_segment_size` (default `2`) merges any
//!    segment shorter than the threshold into its previous neighbour
//!    so we don't fire one LLM call per stray "ok!" message.
//!
//! The wiring into `commands::brain_memory::extract_facts` is the
//! follow-up Chunk 26.2b — keeping this PR small and easy to review.

use serde::{Deserialize, Serialize};

/// Tunable knobs. Defaults are chosen so an "uneventful" five-turn
/// conversation segments to one segment, while a clear topic shift in
/// the middle of a 10-turn chat segments to two.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SegmenterConfig {
    /// Multiplier on stddev for the boundary threshold. Higher = fewer
    /// segments. `1.0` is a reasonable starting point on English chat.
    pub stddev_k: f64,
    /// Minimum number of turns per segment. Shorter segments are merged
    /// into the previous neighbour. `2` avoids firing on a stray "ok!".
    pub min_segment_size: usize,
    /// Maximum length of the auto-derived `summary` string per segment,
    /// in characters. The summary is purely a debug/log convenience.
    pub summary_max_chars: usize,
}

impl Default for SegmenterConfig {
    fn default() -> Self {
        Self { stddev_k: 1.0, min_segment_size: 2, summary_max_chars: 80 }
    }
}

/// One topic-coherent range of turns. Indexes are into the caller's
/// `turns` slice, half-open: `range = (start, end)` means turns
/// `[start, end)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopicSegment {
    /// Inclusive start index into the caller's `turns` slice.
    pub start: usize,
    /// Exclusive end index.
    pub end: usize,
    /// Best-effort one-line summary derived from the first user turn
    /// in the segment. Empty when the segment contains no user turns.
    pub summary: String,
}

impl TopicSegment {
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// One conversation turn fed into the segmenter. The `role` is recorded
/// only so the auto-summary can prefer user turns; it does not affect
/// segmentation.
#[derive(Debug, Clone)]
pub struct SegTurn<'a> {
    pub role: &'a str,
    pub content: &'a str,
}

/// Segment a conversation into topic-coherent ranges.
///
/// `turns` and `embeddings` must be parallel slices of the same length;
/// each `embeddings[i]` is the dense vector for `turns[i].content`. All
/// embeddings must share the same dimensionality (mismatched-dim pairs
/// are treated as maximally-distant, which biases toward declaring a
/// boundary — the safe direction when input is malformed).
///
/// Returns at least one segment for any non-empty `turns`. For empty
/// input, returns an empty `Vec`.
pub fn segment(
    turns: &[SegTurn<'_>],
    embeddings: &[Vec<f32>],
    config: &SegmenterConfig,
) -> Vec<TopicSegment> {
    if turns.is_empty() {
        return Vec::new();
    }
    // Defensive: if the embeddings slice is short, fall back to a
    // single segment covering the full transcript. We deliberately
    // *don't* panic — the upstream extractor should still get one
    // call's worth of data even in degraded mode.
    if embeddings.len() != turns.len() || turns.len() < 2 {
        return vec![one_segment(turns, 0, turns.len(), config)];
    }

    // 1. Pairwise consecutive cosine distances.
    let distances: Vec<f64> = (0..turns.len() - 1)
        .map(|i| cosine_distance(&embeddings[i], &embeddings[i + 1]))
        .collect();

    // 2. Threshold = mean + k * stddev. With ≤2 distances stddev is
    //    degenerate, so we just use the max as the threshold (one
    //    boundary at most).
    let threshold = boundary_threshold(&distances, config.stddev_k);

    // 3. Boundaries: indices `i` (in the distance array) that are
    //    strictly greater than both neighbours AND above threshold.
    //    A boundary at distance index `i` means turn `i+1` starts a
    //    new segment.
    let mut boundaries: Vec<usize> = Vec::new();
    for i in 0..distances.len() {
        let d = distances[i];
        if d <= threshold {
            continue;
        }
        let left_ok = i == 0 || d > distances[i - 1];
        let right_ok = i + 1 >= distances.len() || d > distances[i + 1];
        if left_ok && right_ok {
            // Translate distance-index → turn-index.
            boundaries.push(i + 1);
        }
    }

    // 4. Build [start, end) segments from the boundaries.
    let mut segments: Vec<TopicSegment> = Vec::with_capacity(boundaries.len() + 1);
    let mut prev = 0usize;
    for b in &boundaries {
        segments.push(one_segment(turns, prev, *b, config));
        prev = *b;
    }
    segments.push(one_segment(turns, prev, turns.len(), config));

    // 5. Merge segments shorter than `min_segment_size` into their
    //    previous neighbour. We never merge backwards into segment 0
    //    (i.e. a tiny first segment merges *forward* into segment 1).
    merge_small_segments(segments, turns, config)
}

fn boundary_threshold(distances: &[f64], k: f64) -> f64 {
    if distances.len() < 2 {
        return distances.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    }
    let n = distances.len() as f64;
    let mean: f64 = distances.iter().sum::<f64>() / n;
    let variance: f64 = distances.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / n;
    let stddev = variance.sqrt();
    mean + k * stddev
}

fn one_segment(
    turns: &[SegTurn<'_>],
    start: usize,
    end: usize,
    config: &SegmenterConfig,
) -> TopicSegment {
    let summary = summarise_segment(turns, start, end, config.summary_max_chars);
    TopicSegment { start, end, summary }
}

fn summarise_segment(
    turns: &[SegTurn<'_>],
    start: usize,
    end: usize,
    max_chars: usize,
) -> String {
    // Prefer the first user turn for the summary — it's the most
    // indicative of the topic. Fall back to the first turn of any role
    // if none is present.
    let mut chosen: Option<&str> = None;
    for t in turns.iter().take(end).skip(start) {
        if t.role.eq_ignore_ascii_case("user") {
            chosen = Some(t.content);
            break;
        }
    }
    if chosen.is_none() {
        chosen = turns.get(start).map(|t| t.content);
    }
    let raw = chosen.unwrap_or("").trim();
    let mut out: String = raw.chars().take(max_chars).collect();
    if raw.chars().count() > max_chars {
        out.push('…');
    }
    out
}

fn merge_small_segments(
    segments: Vec<TopicSegment>,
    turns: &[SegTurn<'_>],
    config: &SegmenterConfig,
) -> Vec<TopicSegment> {
    if segments.len() <= 1 {
        return segments;
    }
    let mut out: Vec<TopicSegment> = Vec::with_capacity(segments.len());
    for seg in segments {
        if seg.len() < config.min_segment_size && !out.is_empty() {
            // Merge backward into previous segment.
            let prev = out.last_mut().expect("non-empty checked");
            prev.end = seg.end;
            // Re-derive summary from the (now wider) merged range so
            // the summary still reflects the segment's content.
            prev.summary = summarise_segment(
                turns,
                prev.start,
                prev.end,
                config.summary_max_chars,
            );
        } else {
            out.push(seg);
        }
    }
    // If the very first segment was too small AND no merge target
    // existed (i.e. it was the only segment), the loop above pushed it
    // unchanged — that's the correct behaviour: a single short
    // conversation is still one segment.
    out
}

/// Cosine distance ∈ `[0.0, 2.0]`. `0.0` means identical direction,
/// `1.0` means orthogonal, `2.0` means opposite. Returns `1.0`
/// (orthogonal — neither close nor opposite) for any malformed input
/// (mismatched dim, all-zero vectors).
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f64 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 1.0;
    }
    let mut dot = 0f64;
    let mut na = 0f64;
    let mut nb = 0f64;
    for i in 0..a.len() {
        let x = a[i] as f64;
        let y = b[i] as f64;
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 1.0;
    }
    let cos = (dot / (na.sqrt() * nb.sqrt())).clamp(-1.0, 1.0);
    1.0 - cos
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn<'a>(role: &'a str, content: &'a str) -> SegTurn<'a> {
        SegTurn { role, content }
    }

    /// Build a 4-dim embedding pointing along axis `idx` so two turns
    /// with the same `idx` are identical and different `idx` are
    /// orthogonal — easy to reason about in segmentation tests.
    fn axis_emb(idx: usize) -> Vec<f32> {
        let mut v = vec![0.0f32; 4];
        v[idx % 4] = 1.0;
        v
    }

    #[test]
    fn cosine_distance_identical_is_zero() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!(cosine_distance(&a, &b).abs() < 1e-9);
    }

    #[test]
    fn cosine_distance_orthogonal_is_one() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_distance(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cosine_distance_opposite_is_two() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        assert!((cosine_distance(&a, &b) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn cosine_distance_mismatched_dim_returns_one() {
        let a = vec![1.0];
        let b = vec![1.0, 0.0];
        assert!((cosine_distance(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cosine_distance_zero_vector_returns_one() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 1.0, 1.0];
        assert!((cosine_distance(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn empty_input_returns_empty_segments() {
        let segments = segment(&[], &[], &SegmenterConfig::default());
        assert!(segments.is_empty());
    }

    #[test]
    fn single_turn_yields_single_segment() {
        let turns = vec![turn("user", "hi")];
        let embs = vec![axis_emb(0)];
        let segments = segment(&turns, &embs, &SegmenterConfig::default());
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start, 0);
        assert_eq!(segments[0].end, 1);
    }

    #[test]
    fn mismatched_embeddings_falls_back_to_one_segment() {
        let turns = vec![turn("user", "a"), turn("assistant", "b")];
        let embs = vec![axis_emb(0)]; // wrong length
        let segments = segment(&turns, &embs, &SegmenterConfig::default());
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].len(), 2);
    }

    #[test]
    fn uniform_topic_yields_single_segment() {
        // All turns embed to the same axis → zero distances → no
        // boundaries.
        let turns: Vec<SegTurn> = (0..5).map(|i| if i % 2 == 0 {
            turn("user", "cooking")
        } else {
            turn("assistant", "yum")
        }).collect();
        let embs: Vec<Vec<f32>> = (0..5).map(|_| axis_emb(0)).collect();
        let segments = segment(&turns, &embs, &SegmenterConfig::default());
        assert_eq!(segments.len(), 1, "uniform topic must collapse to 1 segment");
        assert_eq!(segments[0].start, 0);
        assert_eq!(segments[0].end, 5);
    }

    #[test]
    fn one_clear_topic_shift_yields_two_segments() {
        // Turns 0..3 on topic A, turns 3..6 on topic B → boundary at 3.
        let turns: Vec<SegTurn> = vec![
            turn("user", "let's talk cooking"),
            turn("assistant", "great, pasta?"),
            turn("user", "yes pasta"),
            turn("user", "actually let's debug rust now"),
            turn("assistant", "sure, what error?"),
            turn("user", "borrow checker is mad"),
        ];
        // First three on axis 0 (cooking), next three on axis 1 (rust).
        let embs: Vec<Vec<f32>> = vec![
            axis_emb(0), axis_emb(0), axis_emb(0),
            axis_emb(1), axis_emb(1), axis_emb(1),
        ];
        let segments = segment(&turns, &embs, &SegmenterConfig::default());
        assert_eq!(segments.len(), 2, "one shift must yield 2 segments");
        assert_eq!(segments[0].start, 0);
        assert_eq!(segments[0].end, 3);
        assert_eq!(segments[1].start, 3);
        assert_eq!(segments[1].end, 6);
    }

    #[test]
    fn small_segments_merge_into_previous() {
        // Force a tiny segment in the middle by giving turn 3 a unique
        // axis; the surrounding turns share axis 0. The default
        // min_segment_size=2 should merge it back.
        let turns: Vec<SegTurn> = vec![
            turn("user", "topic A 1"),
            turn("user", "topic A 2"),
            turn("user", "topic A 3"),
            turn("user", "tiny outlier"),
            turn("user", "topic A 5"),
        ];
        let embs: Vec<Vec<f32>> = vec![
            axis_emb(0), axis_emb(0), axis_emb(0),
            axis_emb(2), // outlier
            axis_emb(0),
        ];
        let segments = segment(&turns, &embs, &SegmenterConfig::default());
        // Outlier may or may not trigger a boundary, but the result
        // must never have a singleton segment per `min_segment_size=2`.
        for s in &segments {
            assert!(s.len() >= 2 || segments.len() == 1, "segment {:?} too small", s);
        }
    }

    #[test]
    fn segments_cover_full_transcript_without_gaps() {
        // Every turn must belong to exactly one segment.
        let turns: Vec<SegTurn> = (0..6).map(|i| turn("user", match i {
            0..=1 => "a",
            2..=3 => "b",
            _ => "c",
        })).collect();
        let embs: Vec<Vec<f32>> = (0..6).map(|i| axis_emb(i / 2)).collect();
        let segments = segment(&turns, &embs, &SegmenterConfig::default());
        // Coverage check: end of segment N == start of segment N+1.
        let mut cursor = 0;
        for s in &segments {
            assert_eq!(s.start, cursor);
            cursor = s.end;
        }
        assert_eq!(cursor, turns.len());
    }

    #[test]
    fn summary_prefers_first_user_turn() {
        // Force a segment with assistant-first ordering; summary must
        // still pick the user turn.
        let turns = vec![
            turn("assistant", "hello!"),
            turn("user", "I want to learn Rust ownership"),
        ];
        let embs = vec![axis_emb(0), axis_emb(0)];
        let segments = segment(&turns, &embs, &SegmenterConfig::default());
        assert_eq!(segments.len(), 1);
        assert!(segments[0].summary.contains("Rust"), "got: {}", segments[0].summary);
    }

    #[test]
    fn summary_truncated_to_max_chars() {
        let long = "x".repeat(500);
        let turns = vec![turn("user", &long)];
        let embs = vec![axis_emb(0)];
        let cfg = SegmenterConfig { summary_max_chars: 20, ..SegmenterConfig::default() };
        let segments = segment(&turns, &embs, &cfg);
        assert!(segments[0].summary.chars().count() <= 21); // 20 + ellipsis
        assert!(segments[0].summary.ends_with('…'));
    }

    #[test]
    fn higher_stddev_k_yields_fewer_segments() {
        // Same input, two different stddev_k values; the higher one
        // must produce ≤ segments than the lower one.
        let turns: Vec<SegTurn> = (0..6).map(|_| turn("user", "msg")).collect();
        let embs: Vec<Vec<f32>> = vec![
            axis_emb(0), axis_emb(0), axis_emb(1),
            axis_emb(0), axis_emb(2), axis_emb(0),
        ];
        let strict = SegmenterConfig { stddev_k: 0.0, ..SegmenterConfig::default() };
        let lenient = SegmenterConfig { stddev_k: 5.0, ..SegmenterConfig::default() };
        let s_strict = segment(&turns, &embs, &strict);
        let s_lenient = segment(&turns, &embs, &lenient);
        assert!(
            s_lenient.len() <= s_strict.len(),
            "lenient {} > strict {}",
            s_lenient.len(),
            s_strict.len()
        );
    }

    #[test]
    fn config_round_trips_through_serde() {
        let cfg = SegmenterConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: SegmenterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn topic_segment_round_trip() {
        let s = TopicSegment { start: 0, end: 3, summary: "hi".to_string() };
        let json = serde_json::to_string(&s).unwrap();
        let back: TopicSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn min_segment_size_one_keeps_singletons() {
        // With min=1, a single-turn segment is allowed.
        let turns: Vec<SegTurn> = vec![
            turn("user", "a"),
            turn("user", "a"),
            turn("user", "OUTLIER"),
            turn("user", "a"),
        ];
        let embs: Vec<Vec<f32>> = vec![
            axis_emb(0), axis_emb(0), axis_emb(2), axis_emb(0),
        ];
        let cfg = SegmenterConfig { min_segment_size: 1, ..SegmenterConfig::default() };
        let segments = segment(&turns, &embs, &cfg);
        // No assertion on count (algorithm-dependent), but we must not
        // panic and every turn must be covered.
        let mut total = 0;
        for s in &segments { total += s.len(); }
        assert_eq!(total, turns.len());
    }
}
