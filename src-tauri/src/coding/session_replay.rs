//! Cross-harness session replay (Chunk 44.4).
//!
//! Replays imported session transcripts (from `session_import`) through
//! the memory extraction pipeline to build context. This enables users
//! who migrate from other AI coding tools to bring their accumulated
//! knowledge into TerranSoul.
//!
//! The replay pipeline:
//! 1. Parse transcript → `Vec<ImportedTurn>`
//! 2. Group turns into segments (windows of N turns)
//! 3. Feed each segment through `extract_facts_any_mode` (or dry-run preview)
//! 4. Store extracted facts as new memories tagged `imported_from=<harness>`
//!
//! Pure segment/windowing logic lives here; I/O dispatch is in
//! `commands/coding.rs`.

use serde::{Deserialize, Serialize};

use crate::coding::session_import::{self, Harness, ImportResult, ImportedTurn};
use std::path::Path;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for a replay session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySessionConfig {
    /// Number of turns per extraction window.
    #[serde(default = "default_window_size")]
    pub window_size: usize,
    /// When true, parse and count but don't actually extract/store.
    #[serde(default)]
    pub dry_run: bool,
    /// Maximum total turns to replay (None = all).
    #[serde(default)]
    pub max_turns: Option<usize>,
    /// Maximum new memories per session (budget cap).
    #[serde(default = "default_max_memories_per_session")]
    pub max_memories_per_session: usize,
}

fn default_window_size() -> usize {
    20
}

fn default_max_memories_per_session() -> usize {
    50
}

impl Default for ReplaySessionConfig {
    fn default() -> Self {
        Self {
            window_size: default_window_size(),
            dry_run: false,
            max_turns: None,
            max_memories_per_session: default_max_memories_per_session(),
        }
    }
}

// ---------------------------------------------------------------------------
// Replay plan
// ---------------------------------------------------------------------------

/// A segment (window) of turns ready for extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySegment {
    /// Index of this segment (0-based).
    pub segment_index: usize,
    /// The (role, content) pairs for this window.
    pub history: Vec<(String, String)>,
    /// Source harness.
    pub harness: Harness,
    /// Session ID for tagging.
    pub session_id: String,
}

/// Plan for replaying one imported session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayPlan {
    pub harness: Harness,
    pub session_id: String,
    pub total_turns: usize,
    pub segments: Vec<ReplaySegment>,
}

/// Result of replaying one session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    pub harness: Harness,
    pub session_id: String,
    pub segments_processed: usize,
    pub facts_extracted: usize,
    pub facts_stored: usize,
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Planning logic (pure)
// ---------------------------------------------------------------------------

/// Build a replay plan from imported turns.
///
/// Splits the turns into sliding windows of `config.window_size` with
/// 50% overlap (so context at window boundaries isn't lost).
pub fn plan_replay(turns: &[ImportedTurn], config: &ReplaySessionConfig) -> ReplayPlan {
    let harness = turns.first().map(|t| t.harness).unwrap_or(Harness::Claude);
    let session_id = turns
        .first()
        .map(|t| t.session_id.clone())
        .unwrap_or_default();

    // Apply max_turns cap.
    let effective_turns = match config.max_turns {
        Some(cap) => &turns[..turns.len().min(cap)],
        None => turns,
    };

    let total_turns = effective_turns.len();
    let window = config.window_size.max(4); // minimum 4 turns per window
    let step = (window / 2).max(1); // 50% overlap

    let mut segments = Vec::new();
    let mut start = 0;
    let mut seg_idx = 0;

    while start < total_turns {
        let end = (start + window).min(total_turns);
        let history: Vec<(String, String)> = effective_turns[start..end]
            .iter()
            .map(|t| (t.role.clone(), t.content.clone()))
            .collect();

        segments.push(ReplaySegment {
            segment_index: seg_idx,
            history,
            harness,
            session_id: session_id.clone(),
        });

        start += step;
        seg_idx += 1;
    }

    ReplayPlan {
        harness,
        session_id,
        total_turns,
        segments,
    }
}

/// Build the tag string for memories extracted via replay.
pub fn replay_tag(harness: Harness, session_id: &str) -> String {
    format!(
        "imported_from={},replay_session={}",
        harness.tag(),
        session_id
    )
}

/// Parse and plan a single transcript file for replay.
pub fn plan_file_replay(
    harness: Harness,
    path: &Path,
    config: &ReplaySessionConfig,
) -> (ImportResult, ReplayPlan) {
    let import = session_import::parse_transcript(harness, path);
    let turns = session_import::parse_transcript_turns(harness, path);
    let plan = plan_replay(&turns, config);
    (import, plan)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_turns(count: usize) -> Vec<ImportedTurn> {
        (0..count)
            .map(|i| ImportedTurn {
                harness: Harness::Claude,
                session_id: "test-session".to_string(),
                role: if i % 2 == 0 {
                    "user".to_string()
                } else {
                    "assistant".to_string()
                },
                content: format!("Turn {i} content"),
                turn_index: i,
            })
            .collect()
    }

    #[test]
    fn plan_replay_empty_turns() {
        let plan = plan_replay(&[], &ReplaySessionConfig::default());
        assert_eq!(plan.total_turns, 0);
        assert!(plan.segments.is_empty());
    }

    #[test]
    fn plan_replay_few_turns_single_segment() {
        let turns = make_turns(5);
        let config = ReplaySessionConfig {
            window_size: 20,
            ..Default::default()
        };
        let plan = plan_replay(&turns, &config);
        assert_eq!(plan.total_turns, 5);
        assert_eq!(plan.segments.len(), 1);
        assert_eq!(plan.segments[0].history.len(), 5);
    }

    #[test]
    fn plan_replay_overlapping_windows() {
        let turns = make_turns(40);
        let config = ReplaySessionConfig {
            window_size: 20,
            ..Default::default()
        };
        let plan = plan_replay(&turns, &config);
        assert_eq!(plan.total_turns, 40);
        // window=20, step=10: starts at 0, 10, 20, 30 → 4 segments
        assert_eq!(plan.segments.len(), 4);
        // First segment has 20 turns.
        assert_eq!(plan.segments[0].history.len(), 20);
        // Last segment starts at 30, takes 10 turns.
        assert_eq!(plan.segments[3].history.len(), 10);
    }

    #[test]
    fn plan_replay_respects_max_turns() {
        let turns = make_turns(100);
        let config = ReplaySessionConfig {
            window_size: 20,
            max_turns: Some(30),
            ..Default::default()
        };
        let plan = plan_replay(&turns, &config);
        assert_eq!(plan.total_turns, 30);
    }

    #[test]
    fn plan_replay_minimum_window() {
        let turns = make_turns(10);
        let config = ReplaySessionConfig {
            window_size: 2, // below minimum, should be clamped to 4
            ..Default::default()
        };
        let plan = plan_replay(&turns, &config);
        // window=4, step=2: starts at 0,2,4,6,8 → 5 segments
        assert_eq!(plan.segments.len(), 5);
        assert_eq!(plan.segments[0].history.len(), 4);
    }

    #[test]
    fn replay_tag_format() {
        let tag = replay_tag(Harness::Codex, "session-123");
        assert_eq!(tag, "imported_from=codex,replay_session=session-123");
    }

    #[test]
    fn plan_preserves_harness_and_session_id() {
        let turns: Vec<ImportedTurn> = (0..5)
            .map(|i| ImportedTurn {
                harness: Harness::Cursor,
                session_id: "my-session".to_string(),
                role: "user".to_string(),
                content: format!("content {i}"),
                turn_index: i,
            })
            .collect();
        let plan = plan_replay(&turns, &ReplaySessionConfig::default());
        assert_eq!(plan.harness, Harness::Cursor);
        assert_eq!(plan.session_id, "my-session");
        assert_eq!(plan.segments[0].harness, Harness::Cursor);
    }

    #[test]
    fn plan_replay_large_session() {
        // 200 turns simulates a real coding session.
        let turns = make_turns(200);
        let config = ReplaySessionConfig::default(); // window=20, step=10
        let plan = plan_replay(&turns, &config);
        assert_eq!(plan.total_turns, 200);
        // 200 turns, step=10: starts at 0,10,20,...,190 → 20 segments
        assert_eq!(plan.segments.len(), 20);
        // First 19 segments have 20 turns each, last has 10.
        for seg in &plan.segments[..19] {
            assert_eq!(seg.history.len(), 20);
        }
        assert_eq!(plan.segments[19].history.len(), 10);
    }

    #[test]
    fn default_config_values() {
        let c = ReplaySessionConfig::default();
        assert_eq!(c.window_size, 20);
        assert!(!c.dry_run);
        assert!(c.max_turns.is_none());
        assert_eq!(c.max_memories_per_session, 50);
    }

    #[test]
    fn replay_result_structure() {
        let r = ReplayResult {
            harness: Harness::OpenCode,
            session_id: "sess".to_string(),
            segments_processed: 5,
            facts_extracted: 12,
            facts_stored: 10,
            errors: vec!["one error".to_string()],
        };
        assert_eq!(r.segments_processed, 5);
        assert_eq!(r.facts_extracted, 12);
        assert_eq!(r.facts_stored, 10);
        assert_eq!(r.errors.len(), 1);
    }

    #[test]
    fn plan_file_replay_with_json_file() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("session-abc.json");
        let mut f = std::fs::File::create(&file).unwrap();
        writeln!(
            f,
            r#"[
                {{"role":"user","content":"hello world"}},
                {{"role":"assistant","content":"hi there"}},
                {{"role":"user","content":"what is rust?"}},
                {{"role":"assistant","content":"a systems language"}}
            ]"#
        )
        .unwrap();

        let config = ReplaySessionConfig {
            window_size: 4,
            ..Default::default()
        };
        let (import, plan) = plan_file_replay(Harness::Claude, &file, &config);
        assert_eq!(import.turns_extracted, 4);
        assert_eq!(plan.total_turns, 4);
        assert_eq!(plan.session_id, "session-abc");
        assert_eq!(plan.segments.len(), 2); // window=4, step=2
    }

    #[test]
    fn plan_file_replay_missing_file() {
        let path = std::path::PathBuf::from("/tmp/nonexistent-session.json");
        let (import, plan) =
            plan_file_replay(Harness::Codex, &path, &ReplaySessionConfig::default());
        assert_eq!(import.turns_extracted, 0);
        assert_eq!(plan.total_turns, 0);
        assert!(plan.segments.is_empty());
    }
}
