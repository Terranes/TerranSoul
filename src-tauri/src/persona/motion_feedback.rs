//! Motion-generation feedback log (Chunk 14.16e — Self-improve loop).
//!
//! When the user accepts or rejects an LLM-generated motion clip in the
//! Persona panel, we record a small feedback entry so the next prompt
//! cycle can:
//!
//! - Promote descriptions whose generations the user keeps accepting
//!   (those triggers earn a "trusted" badge in the prompt).
//! - Avoid re-suggesting identical phrasings the user repeatedly
//!   discards.
//! - Surface a tiny "you've taught me N motions" stat in the UI so the
//!   user sees the loop closing.
//!
//! Storage is a single newline-delimited JSON file under the persona
//! data root (`<app_data_dir>/persona/motion_feedback.jsonl`). One line
//! per event; durable, append-only, and crash-safe (each line is
//! self-describing). Reading is streaming so a 100k-event log never
//! materialises a giant `Vec<String>`.
//!
//! This module is **deliberately tiny + I/O-bounded** — the interesting
//! product surface is in `commands::persona::record_motion_feedback`,
//! which calls the helpers here. Tests cover round-trip + stats over a
//! tempdir.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Verdict the user gave on a generated motion clip.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackVerdict {
    /// User clicked "Accept & save" — the clip is now in their library.
    Accepted,
    /// User clicked "Discard" or regenerated without saving.
    Rejected,
}

/// One feedback event. Pure data — no behaviour.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MotionFeedbackEntry {
    /// Free-form description the user typed (≤500 chars; same cap as
    /// the generator command). Used to detect repeat-rejected phrasings.
    pub description: String,
    /// Slug-trigger the parser produced (`learned-<slug>`).
    pub trigger: String,
    /// Whether the user accepted or rejected.
    pub verdict: FeedbackVerdict,
    /// Generation duration in seconds (the requested duration_s).
    #[serde(default)]
    pub duration_s: f32,
    /// Generation FPS.
    #[serde(default)]
    pub fps: u32,
    /// Frames the parser had to drop (cleanup signal).
    #[serde(default)]
    pub dropped_frames: usize,
    /// Bone components clamped during parse.
    #[serde(default)]
    pub clamped_components: usize,
    /// ms epoch.
    pub at: i64,
}

/// Aggregated stats over the feedback log. Used by the UI to show the
/// "you've taught me N motions" hint and to promote trusted triggers.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MotionFeedbackStats {
    pub total: usize,
    pub accepted: usize,
    pub rejected: usize,
    /// Triggers with at least one accepted entry, sorted by accept count
    /// descending then alphabetically. Capped at 50 entries to keep the
    /// stats payload small.
    pub trusted_triggers: Vec<TrustedTrigger>,
    /// Descriptions with ≥3 rejections and zero accepts — caller may
    /// want to nudge the user to phrase them differently. Capped at 20.
    pub discouraged_descriptions: Vec<String>,
}

/// One row of the trusted-trigger leaderboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustedTrigger {
    pub trigger: String,
    pub accepted: usize,
    pub rejected: usize,
}

/// Errors that can happen while writing or reading the feedback log.
#[derive(Debug, Error)]
pub enum MotionFeedbackError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialise failed: {0}")]
    Serialise(serde_json::Error),
}

/// Append a feedback entry to the JSONL log at `path`. Creates the
/// file (and parents) on first call. **Atomic at line granularity**:
/// each write is a single `\n`-terminated JSON line, so a torn write
/// only ever loses the trailing partial line on crash, never an
/// already-committed event.
pub fn append_entry(path: &Path, entry: &MotionFeedbackEntry) -> Result<(), MotionFeedbackError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let line = serde_json::to_string(entry).map_err(MotionFeedbackError::Serialise)?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    f.flush()?;
    Ok(())
}

/// Read every entry from the log. Corrupt / partial lines are skipped
/// silently so a torn write never blocks the stats endpoint. Missing
/// file → empty Vec (not an error).
pub fn load_entries(path: &Path) -> Result<Vec<MotionFeedbackEntry>, MotionFeedbackError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let f = File::open(path)?;
    let reader = BufReader::new(f);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<MotionFeedbackEntry>(trimmed) {
            out.push(entry);
        }
    }
    Ok(out)
}

/// Compute aggregate stats over an entry list. Pure / I/O-free so unit
/// tests can drive the aggregation directly.
pub fn aggregate_stats(entries: &[MotionFeedbackEntry]) -> MotionFeedbackStats {
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut by_trigger: std::collections::BTreeMap<String, (usize, usize)> =
        std::collections::BTreeMap::new();
    let mut by_description: std::collections::BTreeMap<String, (usize, usize)> =
        std::collections::BTreeMap::new();
    for e in entries {
        match e.verdict {
            FeedbackVerdict::Accepted => accepted += 1,
            FeedbackVerdict::Rejected => rejected += 1,
        }
        let bucket = by_trigger.entry(e.trigger.clone()).or_default();
        match e.verdict {
            FeedbackVerdict::Accepted => bucket.0 += 1,
            FeedbackVerdict::Rejected => bucket.1 += 1,
        }
        let dbucket = by_description.entry(e.description.clone()).or_default();
        match e.verdict {
            FeedbackVerdict::Accepted => dbucket.0 += 1,
            FeedbackVerdict::Rejected => dbucket.1 += 1,
        }
    }

    let mut trusted: Vec<TrustedTrigger> = by_trigger
        .into_iter()
        .filter(|(_, (a, _))| *a > 0)
        .map(|(trigger, (a, r))| TrustedTrigger {
            trigger,
            accepted: a,
            rejected: r,
        })
        .collect();
    trusted.sort_by(|a, b| {
        b.accepted
            .cmp(&a.accepted)
            .then_with(|| a.trigger.cmp(&b.trigger))
    });
    trusted.truncate(50);

    let mut discouraged: Vec<(String, usize)> = by_description
        .into_iter()
        .filter(|(_, (a, r))| *a == 0 && *r >= 3)
        .map(|(d, (_, r))| (d, r))
        .collect();
    discouraged.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let discouraged_descriptions: Vec<String> = discouraged
        .into_iter()
        .take(20)
        .map(|(d, _)| d)
        .collect();

    MotionFeedbackStats {
        total: accepted + rejected,
        accepted,
        rejected,
        trusted_triggers: trusted,
        discouraged_descriptions,
    }
}

/// Render a short hint sentence the prompt builder can splice into the
/// motion-clip system prompt so the brain knows which triggers the user
/// has historically accepted. Empty string when there's no signal yet.
///
/// Format:
///   `User-trusted motion triggers (use a fresh slug, just for context): a, b, c.`
pub fn render_prompt_hint(stats: &MotionFeedbackStats) -> String {
    if stats.trusted_triggers.is_empty() {
        return String::new();
    }
    let triggers: Vec<&str> = stats
        .trusted_triggers
        .iter()
        .take(8)
        .map(|t| t.trigger.as_str())
        .collect();
    format!(
        "User-trusted motion triggers (for tone reference, mint a fresh slug for the new clip): {}.",
        triggers.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn entry(desc: &str, trig: &str, verdict: FeedbackVerdict) -> MotionFeedbackEntry {
        MotionFeedbackEntry {
            description: desc.to_string(),
            trigger: trig.to_string(),
            verdict,
            duration_s: 3.0,
            fps: 24,
            dropped_frames: 0,
            clamped_components: 0,
            at: 0,
        }
    }

    #[test]
    fn append_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("motion_feedback.jsonl");
        let e = entry("wave hello", "learned-wave-hello", FeedbackVerdict::Accepted);
        append_entry(&path, &e).unwrap();
        let loaded = load_entries(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], e);
    }

    #[test]
    fn missing_log_is_empty_vec_not_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nothing.jsonl");
        let loaded = load_entries(&path).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn corrupt_line_is_skipped() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("motion_feedback.jsonl");
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(
            &path,
            "{not valid json\n{\"description\":\"x\",\"trigger\":\"learned-x\",\"verdict\":\"accepted\",\"at\":0}\n",
        )
        .unwrap();
        let loaded = load_entries(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].description, "x");
    }

    #[test]
    fn aggregate_counts_total_accepted_rejected() {
        let entries = vec![
            entry("a", "learned-a", FeedbackVerdict::Accepted),
            entry("a", "learned-a", FeedbackVerdict::Accepted),
            entry("b", "learned-b", FeedbackVerdict::Rejected),
        ];
        let s = aggregate_stats(&entries);
        assert_eq!(s.total, 3);
        assert_eq!(s.accepted, 2);
        assert_eq!(s.rejected, 1);
    }

    #[test]
    fn trusted_triggers_sorted_by_accept_count_desc_then_name() {
        let entries = vec![
            entry("a", "learned-a", FeedbackVerdict::Accepted),
            entry("b", "learned-b", FeedbackVerdict::Accepted),
            entry("b", "learned-b", FeedbackVerdict::Accepted),
            entry("c", "learned-c", FeedbackVerdict::Rejected),
        ];
        let s = aggregate_stats(&entries);
        assert_eq!(s.trusted_triggers.len(), 2);
        assert_eq!(s.trusted_triggers[0].trigger, "learned-b");
        assert_eq!(s.trusted_triggers[0].accepted, 2);
        assert_eq!(s.trusted_triggers[1].trigger, "learned-a");
    }

    #[test]
    fn discouraged_descriptions_threshold_is_three_rejections_no_accepts() {
        let entries = vec![
            entry("zoom blast", "learned-zoom", FeedbackVerdict::Rejected),
            entry("zoom blast", "learned-zoom", FeedbackVerdict::Rejected),
            entry("zoom blast", "learned-zoom", FeedbackVerdict::Rejected),
            entry("twirl", "learned-twirl", FeedbackVerdict::Rejected),
            entry("twirl", "learned-twirl", FeedbackVerdict::Rejected),
        ];
        let s = aggregate_stats(&entries);
        assert_eq!(s.discouraged_descriptions, vec!["zoom blast".to_string()]);
    }

    #[test]
    fn description_with_one_accept_is_never_discouraged() {
        let entries = vec![
            entry("dance", "learned-dance", FeedbackVerdict::Rejected),
            entry("dance", "learned-dance", FeedbackVerdict::Rejected),
            entry("dance", "learned-dance", FeedbackVerdict::Rejected),
            entry("dance", "learned-dance", FeedbackVerdict::Accepted),
        ];
        let s = aggregate_stats(&entries);
        assert!(s.discouraged_descriptions.is_empty());
    }

    #[test]
    fn render_prompt_hint_empty_when_no_trusted() {
        assert_eq!(render_prompt_hint(&MotionFeedbackStats::default()), "");
    }

    #[test]
    fn render_prompt_hint_lists_top_triggers() {
        let stats = MotionFeedbackStats {
            total: 2,
            accepted: 2,
            rejected: 0,
            trusted_triggers: vec![
                TrustedTrigger {
                    trigger: "learned-wave".into(),
                    accepted: 3,
                    rejected: 0,
                },
                TrustedTrigger {
                    trigger: "learned-bow".into(),
                    accepted: 1,
                    rejected: 0,
                },
            ],
            discouraged_descriptions: vec![],
        };
        let hint = render_prompt_hint(&stats);
        assert!(hint.contains("learned-wave"));
        assert!(hint.contains("learned-bow"));
    }

    #[test]
    fn append_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested/deep/log.jsonl");
        let e = entry("a", "learned-a", FeedbackVerdict::Accepted);
        append_entry(&path, &e).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn aggregate_caps_trusted_at_50() {
        let mut entries = Vec::new();
        for i in 0..70usize {
            entries.push(entry(&format!("d{i}"), &format!("learned-{i:03}"), FeedbackVerdict::Accepted));
        }
        let s = aggregate_stats(&entries);
        assert_eq!(s.trusted_triggers.len(), 50);
    }
}
