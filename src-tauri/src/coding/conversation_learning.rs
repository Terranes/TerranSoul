//! Learn improvement ideas from daily-life chat conversation.
//!
//! When self-improve is enabled, every user chat message is offered to
//! the configured Coding LLM via [`detect_improvement`]. If the LLM
//! decides the message describes a feature request, bug, or improvement
//! idea, [`append_chunk_to_milestones`] atomically appends a new
//! `not-started` row to `rules/milestones.md` so the autonomous loop can
//! pick it up on its next cycle.
//!
//! Resilience contract:
//! - **No data loss**: milestone writes are atomic (write to `.tmp` +
//!   rename) and conflict-free (we never overwrite the file mid-edit).
//! - **No duplicates**: an append-only JSONL audit log
//!   (`learned_chunks.jsonl`) records every chunk we created so a normalised
//!   title cannot be re-added within the dedup window.
//! - **Never blocks chat**: callers fire-and-forget the detection task;
//!   any failure surfaces as a `None` return + an `eprintln`, never a
//!   panic or chat-pipeline error.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::brain::openai_client::OpenAiMessage;

use super::client::client_from;
use super::CodingLlmConfig;

const LEARNED_LOG_FILE: &str = "learned_chunks.jsonl";
const DEDUP_WINDOW_SECS: u64 = 60 * 60 * 24 * 30; // 30 days
const LEARNED_PHASE_HEADER: &str = "### Phase L — Learned from daily conversations";

/// One chunk appended to `rules/milestones.md` from chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LearnedChunk {
    pub id: String,
    pub title: String,
    pub category: String,
    pub source_message: String,
    pub detected_at_ms: u64,
}

/// Strict-JSON shape the detection LLM must return.
#[derive(Debug, Clone, Deserialize)]
pub struct DetectionReply {
    pub is_improvement: bool,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub category: String,
    /// When "lesson", route to brain_ingest_lesson instead of milestones.md.
    /// Recognised values: "feature", "bugfix", "improvement", "lesson", "none".
    /// Defaults to "improvement" if not recognised.
    #[serde(default)]
    pub reply_type: String,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn now_secs() -> u64 {
    now_ms() / 1000
}

/// Build the prompt that asks the Coding LLM to classify a user message.
/// Returns ONLY a single JSON object so we can parse with `serde_json`.
fn detection_prompt(message: &str) -> Vec<OpenAiMessage> {
    let system = "You analyse user chat messages to a personal AI assistant \
                  and decide whether the message describes a concrete \
                  improvement idea (a missing feature, a bug to fix, a \
                  desired enhancement, or a needed change in the assistant \
                  itself). Reply with EXACTLY ONE JSON object and nothing \
                  else, in this shape: \
                  {\"is_improvement\": bool, \"title\": string, \"category\": \
                  \"feature\"|\"bugfix\"|\"improvement\"|\"none\"}. \
                  The title MUST be a short imperative sentence (≤ 80 chars) \
                  describing the change. When the message is small talk, \
                  questions about facts, or anything not actionable, set \
                  is_improvement=false and category=\"none\".";
    let user = format!(
        "User message: \"\"\"\n{}\n\"\"\"\n\nReturn the JSON object now.",
        message.chars().take(4000).collect::<String>()
    );
    vec![
        OpenAiMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        OpenAiMessage {
            role: "user".to_string(),
            content: user,
        },
    ]
}

/// Pull a JSON object out of an LLM reply that may be wrapped in prose,
/// code fences, or extra whitespace. Returns the substring spanning the
/// first `{` to the matching `}` at the same nesting depth.
fn extract_json_object(reply: &str) -> Option<&str> {
    let start = reply.find('{')?;
    let bytes = reply.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        let c = b as char;
        if in_str {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&reply[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Ask the Coding LLM whether `message` describes an improvement; returns
/// `None` for chit-chat / questions / non-actionable inputs and on any
/// transport / parsing failure (the chat pipeline must never break).
pub async fn detect_improvement(message: &str, cfg: &CodingLlmConfig) -> Option<LearnedChunk> {
    let trimmed = message.trim();
    if trimmed.len() < 5 {
        return None;
    }
    let client = client_from(cfg);
    let reply = client.chat(detection_prompt(trimmed)).await.ok()?;
    let json_str = extract_json_object(&reply)?;
    let parsed: DetectionReply = serde_json::from_str(json_str).ok()?;
    if !parsed.is_improvement {
        return None;
    }
    let title = parsed.title.trim();
    if title.is_empty() {
        return None;
    }
    let category = match parsed.category.to_ascii_lowercase().as_str() {
        "feature" | "bugfix" | "improvement" => parsed.category.to_ascii_lowercase(),
        _ => "improvement".to_string(),
    };
    let secs = now_secs();
    Some(LearnedChunk {
        id: format!("L-{secs}"),
        title: title.chars().take(80).collect(),
        category,
        source_message: trimmed.chars().take(500).collect(),
        detected_at_ms: now_ms(),
    })
}

/// Audit-log path for learned chunks.
pub fn learned_log_path(data_dir: &Path) -> PathBuf {
    data_dir.join(LEARNED_LOG_FILE)
}

fn normalise_title(t: &str) -> String {
    t.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// True when an equivalent title was already learned within the dedup window.
pub fn is_duplicate(data_dir: &Path, title: &str) -> bool {
    let path = learned_log_path(data_dir);
    let Ok(contents) = fs::read_to_string(&path) else {
        return false;
    };
    let cutoff_ms = now_ms().saturating_sub(DEDUP_WINDOW_SECS * 1000);
    let target = normalise_title(title);
    for line in contents.lines() {
        if let Ok(c) = serde_json::from_str::<LearnedChunk>(line) {
            if c.detected_at_ms >= cutoff_ms && normalise_title(&c.title) == target {
                return true;
            }
        }
    }
    false
}

/// Append the chunk record to the audit log (one JSON object per line).
pub fn record_learned(data_dir: &Path, chunk: &LearnedChunk) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("create dir: {e}"))?;
    let path = learned_log_path(data_dir);
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open learned log: {e}"))?;
    let line = serde_json::to_string(chunk).map_err(|e| format!("serialize chunk: {e}"))?;
    writeln!(f, "{line}").map_err(|e| format!("write learned log: {e}"))
}

/// Atomically append a `not-started` row to `rules/milestones.md` under
/// the "Learned from daily conversations" phase. Creates the phase
/// section if it does not exist.
pub fn append_chunk_to_milestones(repo_root: &Path, chunk: &LearnedChunk) -> Result<(), String> {
    let path = repo_root.join("rules").join("milestones.md");
    let original = fs::read_to_string(&path).map_err(|e| format!("read milestones: {e}"))?;
    let row = format!(
        "| {} | **{}** _(learned: {})_ | not-started | Auto-detected from chat. |",
        chunk.id,
        chunk.title.replace('|', "/"),
        chunk.category,
    );
    let next = if original.contains(LEARNED_PHASE_HEADER) {
        // Append the row at the end of the document (the section is the
        // last one in the file by convention; this avoids tricky
        // table-end detection in arbitrary positions).
        let mut s = original.clone();
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(&row);
        s.push('\n');
        s
    } else {
        let mut s = original.clone();
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s.push('\n');
        s.push_str(LEARNED_PHASE_HEADER);
        s.push_str("\n\n");
        s.push_str(
            "> Auto-populated by the self-improve conversation-learning hook.\n\
             > Each row is a chunk the autonomous loop will plan on its next cycle.\n\n",
        );
        s.push_str("| # | Chunk | Status | Notes |\n");
        s.push_str("|---|---|---|---|\n");
        s.push_str(&row);
        s.push('\n');
        s
    };
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, next).map_err(|e| format!("write milestones tmp: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("rename milestones tmp: {e}"))
}

/// Convenience: detect, dedup, append + record. Returns the chunk on
/// success, `None` when nothing actionable was found or the chunk was a
/// duplicate. All I/O failures are swallowed (logged) so callers never
/// have to handle them in the chat hot path.
pub async fn learn_from_message(
    message: &str,
    cfg: &CodingLlmConfig,
    data_dir: &Path,
    repo_root: &Path,
) -> Option<LearnedChunk> {
    let chunk = detect_improvement(message, cfg).await?;
    if is_duplicate(data_dir, &chunk.title) {
        return None;
    }
    if let Err(e) = append_chunk_to_milestones(repo_root, &chunk) {
        eprintln!("[self-improve] append milestone failed: {e}");
        return None;
    }
    if let Err(e) = record_learned(data_dir, &chunk) {
        eprintln!("[self-improve] record learned failed: {e}");
    }
    Some(chunk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn extract_json_handles_prose_wrappers_and_fences() {
        let r = "Here you go:\n```json\n{\"is_improvement\":true,\"title\":\"x\",\"category\":\"feature\"}\n```";
        let inner = extract_json_object(r).unwrap();
        let parsed: DetectionReply = serde_json::from_str(inner).unwrap();
        assert!(parsed.is_improvement);
        assert_eq!(parsed.title, "x");
    }

    #[test]
    fn extract_json_returns_none_on_no_braces() {
        assert!(extract_json_object("plain text").is_none());
    }

    #[test]
    fn extract_json_respects_string_escapes() {
        let r = r#"prefix {"k":"v with } inside","x":1} suffix"#;
        let inner = extract_json_object(r).unwrap();
        let v: serde_json::Value = serde_json::from_str(inner).unwrap();
        assert_eq!(v["k"], "v with } inside");
    }

    #[test]
    fn append_creates_phase_section_first_time_then_reuses_it() {
        let dir = tempdir().unwrap();
        let rules = dir.path().join("rules");
        fs::create_dir_all(&rules).unwrap();
        fs::write(rules.join("milestones.md"), "# Milestones\n\n## Phase 1\n").unwrap();

        let chunk = LearnedChunk {
            id: "L-1".into(),
            title: "Add dark theme".into(),
            category: "feature".into(),
            source_message: "I want a dark theme".into(),
            detected_at_ms: 1000,
        };
        append_chunk_to_milestones(dir.path(), &chunk).unwrap();
        let after_one = fs::read_to_string(rules.join("milestones.md")).unwrap();
        assert!(after_one.contains(LEARNED_PHASE_HEADER));
        assert!(after_one.contains("Add dark theme"));
        // No leftover .tmp file.
        assert!(!rules.join("milestones.md.tmp").exists());

        let chunk2 = LearnedChunk {
            id: "L-2".into(),
            title: "Mute background music".into(),
            category: "feature".into(),
            source_message: "music too loud".into(),
            detected_at_ms: 2000,
        };
        append_chunk_to_milestones(dir.path(), &chunk2).unwrap();
        let after_two = fs::read_to_string(rules.join("milestones.md")).unwrap();
        // Section header appears exactly once.
        assert_eq!(after_two.matches(LEARNED_PHASE_HEADER).count(), 1);
        assert!(after_two.contains("Mute background music"));
    }

    #[test]
    fn record_learned_appends_jsonl_and_dedup_blocks_repeats() {
        let dir = tempdir().unwrap();
        let chunk = LearnedChunk {
            id: "L-1".into(),
            title: "Improve onboarding".into(),
            category: "improvement".into(),
            source_message: "msg".into(),
            detected_at_ms: now_ms(),
        };
        record_learned(dir.path(), &chunk).unwrap();
        record_learned(dir.path(), &chunk).unwrap(); // append-only

        let raw = fs::read_to_string(learned_log_path(dir.path())).unwrap();
        assert_eq!(raw.lines().count(), 2);

        // Title-equivalence dedup.
        assert!(is_duplicate(dir.path(), "Improve  ONBOARDING"));
        assert!(!is_duplicate(dir.path(), "Something completely different"));
    }

    #[test]
    fn detection_reply_parses_minimal_negative_case() {
        let json = r#"{"is_improvement": false, "category": "none"}"#;
        let parsed: DetectionReply = serde_json::from_str(json).unwrap();
        assert!(!parsed.is_improvement);
        assert_eq!(parsed.category, "none");
        assert_eq!(parsed.title, "");
    }
}
