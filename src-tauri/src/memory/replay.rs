//! Replay-from-history backfill.
//!
//! **Chunk 26.4** — `rules/milestones.md` Phase 26 row 4. Earlier
//! versions of TerranSoul didn't auto-extract long-term facts from chat
//! sessions; only `MemoryType::Summary` paragraphs were saved. After
//! Chunk 26.2 (segmented extraction) those summaries are a goldmine of
//! never-extracted facts. This module ships the pure logic that decides
//! *which* summaries to replay; the Tauri command in
//! `commands::memory::replay_extract_history` does the I/O + LLM
//! dispatch + progress events.
//!
//! Keeping the selection pure means we can unit-test the filter
//! exhaustively without spinning up a `MemoryStore` or an LLM provider.

use serde::{Deserialize, Serialize};

use crate::memory::{MemoryEntry, MemoryType};

/// Configuration for one replay invocation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReplayConfig {
    /// If `Some(ts)`, only replay summaries with `created_at >= ts`
    /// (Unix milliseconds). `None` means all summaries.
    pub since_timestamp_ms: Option<i64>,
    /// When `true`, the command runs the LLM extraction but **does not**
    /// persist any extracted facts. Used by the UI to preview how many
    /// memories a real run would create.
    pub dry_run: bool,
    /// Optional hard cap on the number of summaries replayed in one call.
    /// Useful for the UI's "replay last 50 sessions" button. `None` =
    /// no cap.
    pub max_summaries: Option<usize>,
}

/// Snapshot of progress emitted on the `brain-replay-progress` Tauri event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayProgress {
    /// Number of summaries processed so far (including the current one
    /// once it finishes).
    pub processed: usize,
    /// Total summaries selected for this run.
    pub total: usize,
    /// Cumulative new memories saved (or, in `dry_run`, *would have been*
    /// saved) across all summaries processed so far.
    pub new_memories: usize,
    /// `created_at` of the most-recently processed summary, for UI
    /// "currently replaying chat from <date>" labels.
    pub current_summary_created_at: Option<i64>,
    /// `id` of the most-recently processed summary, for the same purpose.
    pub current_summary_id: Option<i64>,
    /// True once `processed == total`.
    pub done: bool,
}

/// Filter `all` down to the `MemoryType::Summary` entries that match the
/// config, sorted **oldest-first** so the UI's progress bar advances in
/// chronological order. Pure: no I/O, no allocation beyond the filtered
/// vector.
pub fn select_summaries(all: &[MemoryEntry], config: &ReplayConfig) -> Vec<MemoryEntry> {
    let mut selected: Vec<MemoryEntry> = all
        .iter()
        .filter(|m| matches!(m.memory_type, MemoryType::Summary))
        .filter(|m| match config.since_timestamp_ms {
            Some(ts) => m.created_at >= ts,
            None => true,
        })
        .cloned()
        .collect();
    selected.sort_by_key(|m| m.created_at);
    if let Some(cap) = config.max_summaries {
        selected.truncate(cap);
    }
    selected
}

/// Convenience: build a synthetic `(role, content)` history list from one
/// summary so it can be fed straight into
/// `extract_facts_segmented_any_mode`. We use one `("user", ...)` turn
/// because the segmenter only needs the raw text — speaker labels do not
/// affect topic-shift detection.
///
/// Long summaries (>2000 chars) are split on blank-line boundaries into
/// multiple synthetic turns so the segmenter has something to work with.
/// Short summaries become a single turn.
pub fn synthetic_history_from_summary(summary: &str) -> Vec<(String, String)> {
    const SHORT_THRESHOLD: usize = 2000;
    if summary.chars().count() <= SHORT_THRESHOLD {
        return vec![("user".to_string(), summary.to_string())];
    }
    let parts: Vec<String> = summary
        .split("\n\n")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() < 2 {
        // No paragraph breaks — fall back to a single turn rather than
        // arbitrary character-window splits.
        return vec![("user".to_string(), summary.to_string())];
    }
    parts.into_iter().map(|p| ("user".to_string(), p)).collect()
}

/// Compute the next [`ReplayProgress`] snapshot. Pure helper extracted so
/// the command path stays focused on I/O + emit logic.
pub fn next_progress(
    prev: &ReplayProgress,
    summary: &MemoryEntry,
    new_facts_this_round: usize,
    total: usize,
) -> ReplayProgress {
    let processed = prev.processed + 1;
    ReplayProgress {
        processed,
        total,
        new_memories: prev.new_memories + new_facts_this_round,
        current_summary_created_at: Some(summary.created_at),
        current_summary_id: Some(summary.id),
        done: processed >= total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryTier;

    fn summary(id: i64, created_at: i64, content: &str) -> MemoryEntry {
        MemoryEntry {
            id,
            content: content.to_string(),
            tags: String::new(),
            importance: 3,
            memory_type: MemoryType::Summary,
            created_at,
            last_accessed: None,
            access_count: 0,
            embedding: None,
            tier: MemoryTier::Long,
            decay_score: 1.0,
            session_id: None,
            parent_id: None,
            token_count: content.len() as i64 / 4,
            source_url: None,
            source_hash: None,
            expires_at: None,
            valid_to: None,
        }
    }

    fn fact(id: i64, created_at: i64) -> MemoryEntry {
        MemoryEntry {
            id,
            content: "User likes Python.".to_string(),
            tags: String::new(),
            importance: 3,
            memory_type: MemoryType::Fact,
            created_at,
            last_accessed: None,
            access_count: 0,
            embedding: None,
            tier: MemoryTier::Long,
            decay_score: 1.0,
            session_id: None,
            parent_id: None,
            token_count: 5,
            source_url: None,
            source_hash: None,
            expires_at: None,
            valid_to: None,
        }
    }

    #[test]
    fn select_summaries_keeps_only_summary_type() {
        let entries = vec![summary(1, 100, "s1"), fact(2, 150), summary(3, 200, "s2")];
        let selected = select_summaries(&entries, &ReplayConfig::default());
        assert_eq!(selected.len(), 2);
        assert!(selected
            .iter()
            .all(|m| matches!(m.memory_type, MemoryType::Summary)));
    }

    #[test]
    fn select_summaries_filters_by_since_timestamp() {
        let entries = vec![
            summary(1, 100, "old"),
            summary(2, 500, "recent"),
            summary(3, 1000, "newest"),
        ];
        let selected = select_summaries(
            &entries,
            &ReplayConfig {
                since_timestamp_ms: Some(500),
                ..Default::default()
            },
        );
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].id, 2);
        assert_eq!(selected[1].id, 3);
    }

    #[test]
    fn select_summaries_sorts_oldest_first() {
        let entries = vec![
            summary(3, 1000, "newest"),
            summary(1, 100, "oldest"),
            summary(2, 500, "middle"),
        ];
        let selected = select_summaries(&entries, &ReplayConfig::default());
        assert_eq!(
            selected.iter().map(|s| s.id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn select_summaries_respects_max_cap() {
        let entries: Vec<MemoryEntry> = (0..10)
            .map(|i| summary(i, i * 100, &format!("s{i}")))
            .collect();
        let selected = select_summaries(
            &entries,
            &ReplayConfig {
                max_summaries: Some(3),
                ..Default::default()
            },
        );
        assert_eq!(selected.len(), 3);
        // Cap applies AFTER sort, so we keep the oldest 3.
        assert_eq!(selected[0].id, 0);
        assert_eq!(selected[2].id, 2);
    }

    #[test]
    fn select_summaries_empty_input_returns_empty() {
        let selected = select_summaries(&[], &ReplayConfig::default());
        assert!(selected.is_empty());
    }

    #[test]
    fn synthetic_history_short_summary_is_single_turn() {
        let h = synthetic_history_from_summary("User likes Python and ML.");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].0, "user");
        assert_eq!(h[0].1, "User likes Python and ML.");
    }

    #[test]
    fn synthetic_history_long_summary_splits_on_blank_lines() {
        let long = format!(
            "{}\n\n{}\n\n{}",
            "A".repeat(800),
            "B".repeat(800),
            "C".repeat(800)
        );
        let h = synthetic_history_from_summary(&long);
        assert_eq!(h.len(), 3);
        assert!(h.iter().all(|(role, _)| role == "user"));
    }

    #[test]
    fn synthetic_history_long_summary_no_paragraphs_falls_back_to_single_turn() {
        let long = "x".repeat(3000);
        let h = synthetic_history_from_summary(&long);
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn next_progress_increments_and_carries_totals() {
        let prev = ReplayProgress {
            processed: 1,
            total: 5,
            new_memories: 3,
            current_summary_created_at: Some(50),
            current_summary_id: Some(7),
            done: false,
        };
        let s = summary(11, 200, "x");
        let next = next_progress(&prev, &s, 4, 5);
        assert_eq!(next.processed, 2);
        assert_eq!(next.new_memories, 7);
        assert_eq!(next.total, 5);
        assert_eq!(next.current_summary_id, Some(11));
        assert_eq!(next.current_summary_created_at, Some(200));
        assert!(!next.done);
    }

    #[test]
    fn next_progress_marks_done_on_final_step() {
        let prev = ReplayProgress {
            processed: 4,
            total: 5,
            new_memories: 10,
            current_summary_created_at: None,
            current_summary_id: None,
            done: false,
        };
        let s = summary(99, 999, "x");
        let next = next_progress(&prev, &s, 0, 5);
        assert_eq!(next.processed, 5);
        assert!(next.done);
    }

    #[test]
    fn replay_progress_serializes_to_json() {
        let p = ReplayProgress {
            processed: 2,
            total: 5,
            new_memories: 7,
            current_summary_created_at: Some(123),
            current_summary_id: Some(42),
            done: false,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"processed\":2"));
        assert!(json.contains("\"new_memories\":7"));
        assert!(json.contains("\"done\":false"));
    }
}
