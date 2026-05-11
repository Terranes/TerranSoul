//! Detect and extract lessons learned from interactive agent-coding sessions.
//!
//! When an external coding agent (Copilot, Claude Code, etc.) operating
//! in the main checkout learns a procedural rule (user correction, anti-pattern
//! discovery, screenshot QA insight), this module recognises the patterns and
//! extracts the lesson for durable storage in the brain via `brain_ingest_lesson`.

use serde::{Deserialize, Serialize};

/// A lesson chunk extracted from agent conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LessonChunk {
    /// The lesson content (usually a paragraph or list).
    pub content: String,
    /// Comma-separated tags (e.g., "frontend,css,theme,accessibility").
    pub tags: String,
    /// Importance 1–10; typically 8–10 for agent-discovered lessons.
    pub importance: i64,
    /// Category (e.g., "frontend", "coding-workflow", "security").
    pub category: String,
    /// Optional source URL or reference.
    #[serde(default)]
    pub source_url: Option<String>,
}

/// Represents the speaker in conversation context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Agent,
}

impl From<&str> for Role {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "user" => Role::User,
            "assistant" => Role::Agent,
            _ => Role::Agent,
        }
    }
}

/// Detect lesson patterns in a message.
/// Returns `Some(LessonChunk)` if a lesson pattern is recognised.
pub fn detect_lesson(
    message: &str,
    role: Role,
    _prior_messages: &[(&str, &str)],
) -> Option<LessonChunk> {
    let trimmed = message.trim();
    if trimmed.len() < 20 {
        return None;
    }

    // User-corrective patterns: "you should X instead of Y", "stop doing X"
    if role == Role::User {
        if let Some(lesson) = detect_user_corrective(trimmed) {
            return Some(lesson);
        }
    }

    // Agent-authored patterns: "I learned X", "lesson:"
    if role == Role::Agent {
        if let Some(lesson) = detect_agent_authored(trimmed) {
            return Some(lesson);
        }
    }

    None
}

/// Detect user-corrective patterns:
/// - "you should X instead of Y"
/// - "stop doing X"
/// - "don't use X, use Y instead"
/// - "instead of X, do Y manually"
fn detect_user_corrective(msg: &str) -> Option<LessonChunk> {
    let lower = msg.to_lowercase();

    // Pattern: "you should X instead of Y" or "I should X instead of Y"
    if let Some(pos) = lower.find("instead of") {
        let before = &msg[..pos].trim_end();
        let after = &msg[pos + 10..].trim_start();

        // Extract a summary from the immediate context.
        let (category, tag_hint) = categorise_context(before, after);
        let content = msg.chars().take(500).collect::<String>();

        return Some(LessonChunk {
            content,
            tags: format!("lesson,user-correction,{}", tag_hint),
            importance: 8,
            category,
            source_url: None,
        });
    }

    // Pattern: "stop doing X" or "don't do X"
    if lower.contains("stop doing") || lower.starts_with("don't") && lower.contains("do") {
        let content = msg.chars().take(500).collect::<String>();
        return Some(LessonChunk {
            content,
            tags: "lesson,user-correction,anti-pattern".to_string(),
            importance: 7,
            category: "coding-workflow".to_string(),
            source_url: None,
        });
    }

    None
}

/// Detect agent-authored patterns:
/// - "I learned X"
/// - "lesson:"
/// - "LESSON:" or "RULE:"
fn detect_agent_authored(msg: &str) -> Option<LessonChunk> {
    let lower = msg.to_lowercase();

    // Pattern: "I learned X"
    if lower.contains("i learned") || lower.contains("we learned") {
        let content = msg.chars().take(500).collect::<String>();
        return Some(LessonChunk {
            content,
            tags: "lesson,agent-discovered,procedural".to_string(),
            importance: 9,
            category: "coding-workflow".to_string(),
            source_url: None,
        });
    }

    // Pattern: "lesson:" or "LESSON:" or "RULE:"
    if lower.contains("lesson:") || lower.contains("rule:") {
        // Extract everything after the marker.
        let marker_pos = lower.find("lesson:").or_else(|| lower.find("rule:"))?;
        let after_marker = &msg[marker_pos..];
        let content = after_marker.chars().take(500).collect::<String>();

        return Some(LessonChunk {
            content,
            tags: "lesson,agent-authored,formalized".to_string(),
            importance: 9,
            category: "coding-workflow".to_string(),
            source_url: None,
        });
    }

    None
}

/// Guess category and tags from context clues.
fn categorise_context(before: &str, after: &str) -> (String, String) {
    let combined = format!("{} {}", before, after).to_lowercase();

    let (category, tag) = if combined.contains("test") {
        ("coding-workflow", "testing")
    } else if combined.contains("css") || combined.contains("theme") || combined.contains("style") {
        ("frontend", "css,theme")
    } else if combined.contains("screenshot") || combined.contains("qa") {
        ("frontend", "qa,screenshot")
    } else if combined.contains("shell") || combined.contains("script") || combined.contains("bash")
    {
        ("coding-workflow", "shell,scripting")
    } else if combined.contains("batch") || combined.contains("loop") || combined.contains("manual")
    {
        ("coding-workflow", "automation,manual-workflow")
    } else {
        ("coding-workflow", "general")
    };

    (category.to_string(), tag.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_user_corrective_instead_of() {
        let msg = "You should use textarea instead of input for multi-line text entry.";
        let lesson = detect_lesson(msg, Role::User, &[]);
        assert!(lesson.is_some());
        let l = lesson.unwrap();
        assert!(l.content.contains("textarea"));
        assert!(l.tags.contains("user-correction"));
    }

    #[test]
    fn test_detect_user_corrective_stop_doing() {
        let msg =
            "Stop doing batch scripts for screenshot QA; do it manually with visual verification.";
        let lesson = detect_lesson(msg, Role::User, &[]);
        assert!(lesson.is_some());
        let l = lesson.unwrap();
        assert_eq!(l.importance, 7);
    }

    #[test]
    fn test_detect_agent_authored_i_learned() {
        let msg =
            "I learned that CSS custom properties must match definitions in every theme block.";
        let lesson = detect_lesson(msg, Role::Agent, &[]);
        assert!(lesson.is_some());
        let l = lesson.unwrap();
        assert_eq!(l.importance, 9);
        assert!(l.tags.contains("agent-discovered"));
    }

    #[test]
    fn test_detect_agent_authored_lesson_marker() {
        let msg = "LESSON: When writing a Tauri command, always check if a resource is already locked before trying to acquire it.";
        let lesson = detect_lesson(msg, Role::Agent, &[]);
        assert!(lesson.is_some());
        let l = lesson.unwrap();
        assert_eq!(l.importance, 9);
        assert!(l.tags.contains("formalized"));
    }

    #[test]
    fn test_detect_agent_authored_rule_marker() {
        let msg = "RULE: Never call .unwrap() in library code; use ? and thiserror instead.";
        let lesson = detect_lesson(msg, Role::Agent, &[]);
        assert!(lesson.is_some());
    }

    #[test]
    fn test_no_lesson_in_short_message() {
        let msg = "ok";
        let lesson = detect_lesson(msg, Role::User, &[]);
        assert!(lesson.is_none());
    }

    #[test]
    fn test_no_lesson_in_generic_chat() {
        let msg = "What's the best way to learn Rust?";
        let lesson = detect_lesson(msg, Role::User, &[]);
        assert!(lesson.is_none());
    }
}
