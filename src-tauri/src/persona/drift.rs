//! Persona drift detection (Chunk 14.8).
//!
//! Periodically compares the user's active [`PersonaTraits`] against their
//! accumulated `personal:*` memories. When a meaningful shift is found the
//! brain returns a short natural-language summary so the frontend can
//! surface a suggestion like "Echo noticed you've shifted toward … —
//! update persona?"
//!
//! This module is deliberately I/O-free — pure prompt construction and
//! reply parsing — so it can be exhaustively unit-tested without an LLM
//! daemon. The thin Tauri command in [`crate::commands::persona`]
//! orchestrates the plumbing (read persona from disk, load memories,
//! call brain, return report to the frontend).
//!
//! See `docs/persona-design.md` § 15.1 row 143.

use serde::{Deserialize, Serialize};

/// Default: run a drift check after every 25 accumulated auto-learned
/// facts (not turns — we track the running total of facts saved by
/// `extract_memories_from_session`, so quiet sessions don't trigger
/// unnecessary checks).
pub const DEFAULT_DRIFT_FACT_THRESHOLD: u32 = 25;

/// Result of a persona drift check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriftReport {
    /// True when a meaningful shift was detected.
    pub drift_detected: bool,
    /// Short natural-language summary of the shift, e.g.
    /// "You've become more interested in Vietnamese law and formal
    /// academic topics." Empty when `drift_detected` is false.
    pub summary: String,
    /// Optional list of suggested trait updates the user can apply
    /// with one click. Each entry is a `(field, old, new)` triple,
    /// e.g. `("role", "playful imp", "studious librarian")`.
    #[serde(default)]
    pub suggested_changes: Vec<DriftSuggestion>,
}

/// A single suggested trait update.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriftSuggestion {
    /// Which PersonaTraits field changed: `role`, `tone`, `bio`, etc.
    pub field: String,
    /// Current value (stringified).
    pub current: String,
    /// Proposed replacement.
    pub proposed: String,
}

/// Build the (system, user) prompt pair for persona drift detection.
///
/// * `persona_json` — the active persona traits, pretty-printed JSON.
/// * `personal_memories` — the user's `personal:*` tagged memories,
///   each as `(content, tags)`.
///
/// The prompt is kept tight to fit inside a 4 K context window so even
/// small local models can handle it.
pub fn build_drift_prompt(
    persona_json: &str,
    personal_memories: &[(String, String)],
) -> (String, String) {
    let system = "You are a persona-alignment analyst. \
Compare a user's current AI companion persona against their recent \
personal memories. Determine if the persona still fits or if the \
user's interests / preferences have shifted. \
Reply with ONLY a JSON object — no prose, no markdown fences."
        .to_string();

    let mut memories_block = String::new();
    let mut budget = 6_000usize; // rough char budget for memory block
    for (i, (content, tags)) in personal_memories.iter().enumerate() {
        let entry = format!("{}. [{}] {}\n", i + 1, tags, content.trim());
        if entry.len() > budget {
            break;
        }
        budget -= entry.len();
        memories_block.push_str(&entry);
    }
    if memories_block.is_empty() {
        memories_block = "(No personal memories available.)".to_string();
    }

    let user = format!(
        "CURRENT PERSONA:\n{persona_json}\n\n\
        USER'S PERSONAL MEMORIES:\n{memories_block}\n\n\
        TASK: Compare the persona against the memories. Has the user's \
        personality, interests, communication style, or preferences shifted \
        away from what the persona describes?\n\n\
        OUTPUT FORMAT — reply with exactly one JSON object:\n\
        {{\n\
        \x20 \"drift_detected\": true/false,\n\
        \x20 \"summary\": \"<1-2 sentence description of the shift, or empty if none>\",\n\
        \x20 \"suggested_changes\": [\n\
        \x20   {{ \"field\": \"<role|bio|tone|quirks|avoid>\", \
        \"current\": \"<current value>\", \"proposed\": \"<suggested new value>\" }}\n\
        \x20 ]\n\
        }}\n\n\
        Rules:\n\
        - If no meaningful drift: set drift_detected=false, summary=\"\", suggested_changes=[].\n\
        - Only flag genuine shifts — not minor variations.\n\
        - suggested_changes: 0-3 entries max.\n\
        - For array fields (tone, quirks, avoid), stringify as comma-separated.\n\
        - Reply with ONLY the JSON object."
    );

    (system, user)
}

/// Parse a brain reply into a [`DriftReport`]. Tolerant of:
/// - leading / trailing prose ("Sure! Here's the JSON: { ... }"),
/// - markdown fenced blocks,
/// - missing optional fields (filled with defaults).
///
/// Returns `None` when no JSON object can be located or the required
/// `drift_detected` field is missing.
pub fn parse_drift_reply(raw: &str) -> Option<DriftReport> {
    let body = strip_fences(raw);
    // Try to find a JSON object in the response.
    let start = body.find('{')?;
    let end = body.rfind('}')? + 1;
    if start >= end {
        return None;
    }
    let json_str = &body[start..end];

    // Try strict parse first.
    if let Ok(report) = serde_json::from_str::<DriftReport>(json_str) {
        return Some(report);
    }

    // Fallback: parse as generic Value and extract fields manually.
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let drift_detected = v.get("drift_detected")?.as_bool()?;
    let summary = v
        .get("summary")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    let suggested_changes = v
        .get("suggested_changes")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    Some(DriftSuggestion {
                        field: item.get("field")?.as_str()?.to_string(),
                        current: item
                            .get("current")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string(),
                        proposed: item
                            .get("proposed")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(DriftReport {
        drift_detected,
        summary,
        suggested_changes,
    })
}

/// Strip markdown fences (`\`\`\`json ... \`\`\``) from a raw reply.
fn strip_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    // Handle ```json ... ``` or ``` ... ```
    if let Some(rest) = trimmed.strip_prefix("```") {
        let rest = rest.strip_prefix("json").unwrap_or(rest);
        let rest = rest.trim_start_matches('\n');
        if let Some(body) = rest.strip_suffix("```") {
            return body.trim().to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_drift_prompt_includes_persona_and_memories() {
        let persona = r#"{"name":"Soul","role":"playful imp"}"#;
        let memories = vec![
            ("Likes formal academic writing".into(), "personal:style".into()),
            ("Studies Vietnamese law".into(), "personal:interest".into()),
        ];
        let (system, user) = build_drift_prompt(persona, &memories);
        assert!(system.contains("persona-alignment"));
        assert!(user.contains("playful imp"));
        assert!(user.contains("Likes formal academic writing"));
        assert!(user.contains("Studies Vietnamese law"));
        assert!(user.contains("drift_detected"));
    }

    #[test]
    fn build_drift_prompt_empty_memories() {
        let persona = r#"{"name":"Soul"}"#;
        let (_, user) = build_drift_prompt(persona, &[]);
        assert!(user.contains("No personal memories available"));
    }

    #[test]
    fn build_drift_prompt_respects_char_budget() {
        let persona = r#"{"name":"Soul"}"#;
        // Create 100 large memories to blow the budget
        let memories: Vec<(String, String)> = (0..100)
            .map(|i| (format!("Memory content {i} {}", "x".repeat(200)), format!("personal:tag{i}")))
            .collect();
        let (_, user) = build_drift_prompt(persona, &memories);
        // Should include some but not all
        assert!(user.contains("Memory content 0"));
        assert!(!user.contains("Memory content 99"));
    }

    #[test]
    fn parse_drift_reply_clean_json() {
        let raw = r#"{"drift_detected":true,"summary":"You shifted toward law.","suggested_changes":[{"field":"role","current":"playful imp","proposed":"studious librarian"}]}"#;
        let report = parse_drift_reply(raw).unwrap();
        assert!(report.drift_detected);
        assert_eq!(report.summary, "You shifted toward law.");
        assert_eq!(report.suggested_changes.len(), 1);
        assert_eq!(report.suggested_changes[0].field, "role");
        assert_eq!(report.suggested_changes[0].current, "playful imp");
        assert_eq!(report.suggested_changes[0].proposed, "studious librarian");
    }

    #[test]
    fn parse_drift_reply_no_drift() {
        let raw = r#"{"drift_detected":false,"summary":"","suggested_changes":[]}"#;
        let report = parse_drift_reply(raw).unwrap();
        assert!(!report.drift_detected);
        assert!(report.summary.is_empty());
        assert!(report.suggested_changes.is_empty());
    }

    #[test]
    fn parse_drift_reply_with_fences() {
        let raw = "```json\n{\"drift_detected\":true,\"summary\":\"Shifted.\",\"suggested_changes\":[]}\n```";
        let report = parse_drift_reply(raw).unwrap();
        assert!(report.drift_detected);
        assert_eq!(report.summary, "Shifted.");
    }

    #[test]
    fn parse_drift_reply_with_leading_prose() {
        let raw = "Sure! Here's the analysis:\n{\"drift_detected\":false,\"summary\":\"\",\"suggested_changes\":[]}";
        let report = parse_drift_reply(raw).unwrap();
        assert!(!report.drift_detected);
    }

    #[test]
    fn parse_drift_reply_missing_optional_fields() {
        let raw = r#"{"drift_detected":true,"summary":"Changed."}"#;
        let report = parse_drift_reply(raw).unwrap();
        assert!(report.drift_detected);
        assert_eq!(report.summary, "Changed.");
        assert!(report.suggested_changes.is_empty());
    }

    #[test]
    fn parse_drift_reply_garbage_returns_none() {
        assert!(parse_drift_reply("not json at all").is_none());
        assert!(parse_drift_reply("").is_none());
    }

    #[test]
    fn parse_drift_reply_missing_drift_detected_returns_none() {
        let raw = r#"{"summary":"something"}"#;
        assert!(parse_drift_reply(raw).is_none());
    }

    #[test]
    fn strip_fences_removes_json_fences() {
        let input = "```json\n{\"a\":1}\n```";
        assert_eq!(strip_fences(input), r#"{"a":1}"#);
    }

    #[test]
    fn strip_fences_removes_plain_fences() {
        let input = "```\n{\"a\":1}\n```";
        assert_eq!(strip_fences(input), r#"{"a":1}"#);
    }

    #[test]
    fn strip_fences_passthrough_no_fences() {
        let input = r#"{"a":1}"#;
        assert_eq!(strip_fences(input), r#"{"a":1}"#);
    }

    #[test]
    fn drift_report_serde_round_trip() {
        let report = DriftReport {
            drift_detected: true,
            summary: "Shifted toward law.".into(),
            suggested_changes: vec![DriftSuggestion {
                field: "role".into(),
                current: "playful imp".into(),
                proposed: "studious librarian".into(),
            }],
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: DriftReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, back);
    }
}
