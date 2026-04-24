//! Master-Echo persona-extraction helpers (Chunk 14.2).
//!
//! The brain reads recent conversation history + the user's long-term
//! "personal:*" memories and proposes a [`PersonaCandidate`] JSON the
//! frontend reviews. **Nothing is written to disk here** — application
//! happens only after the user clicks "Apply" in `PersonaPanel.vue`.
//!
//! This module is deliberately I/O-free so it can be exhaustively unit
//! tested without an Ollama daemon. The thin Tauri command in
//! [`crate::commands::persona`] orchestrates: pull conversation +
//! memories → call [`crate::brain::OllamaAgent::propose_persona`] →
//! [`parse_persona_reply`] → return JSON to the frontend.

use serde::{Deserialize, Serialize};

/// Maximum number of past chat turns that get folded into the prompt.
/// Keeps the prompt comfortably under the typical 8 K context window
/// even for verbose models.
pub const PERSONA_PROMPT_HISTORY_TURNS: usize = 30;

/// Maximum number of long-term memories that get folded into the prompt.
/// Picked to balance signal vs. context budget.
pub const PERSONA_PROMPT_MEMORY_LIMIT: usize = 20;

/// Hard cap on the rendered prompt size (chars, not tokens — we'd rather
/// over-truncate than overflow a small local model). Each turn / memory
/// is included only while the running total is below this.
pub const PERSONA_PROMPT_CHAR_BUDGET: usize = 12_000;

/// One snippet (a chat turn or a memory) that may be folded into the
/// persona-extraction prompt.
#[derive(Debug, Clone)]
pub struct PromptSnippet {
    /// Display label, e.g. `"USER"`, `"ASSISTANT"`, `"MEMORY"`.
    pub label: String,
    /// The textual body. Will be trimmed; empty bodies are dropped by
    /// [`render_snippets`].
    pub body: String,
}

/// JSON candidate returned by the brain. Mirrors the frontend
/// `PersonaTraits` shape minus the bookkeeping fields (`version`,
/// `active`, `updatedAt`) which the frontend fills in itself before
/// applying. Every list field is optional to make the brain's life easy:
/// missing → `Some(vec![])`, never an error.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonaCandidate {
    pub name: String,
    pub role: String,
    pub bio: String,
    #[serde(default)]
    pub tone: Vec<String>,
    #[serde(default)]
    pub quirks: Vec<String>,
    #[serde(default)]
    pub avoid: Vec<String>,
}

/// Build the (system, user) pair for the persona-extraction prompt.
///
/// Both strings are pure functions of the inputs — no time, no PRNG, no
/// network. The user-message contains the rendered transcript in plain
/// text and a short "OUTPUT FORMAT" block that asks the brain to reply
/// with **only** a JSON object (no prose, no fences). The reply is then
/// parsed by [`parse_persona_reply`] which is tolerant of common
/// deviations (markdown fences, leading prose).
pub fn build_persona_prompt(snippets: &[PromptSnippet]) -> (String, String) {
    build_persona_prompt_with_hints(snippets, None)
}

/// Same as [`build_persona_prompt`] but also folds an optional
/// prosody-hint block into the user message (Chunk 14.6). When `hints`
/// is `None` the output is byte-identical to [`build_persona_prompt`]
/// so existing tests stay green.
///
/// The hint block lives **between** the transcript and the OUTPUT
/// FORMAT instructions so a model that respects positional cues sees
/// the hints as supporting context, not as user content to echo back.
pub fn build_persona_prompt_with_hints(
    snippets: &[PromptSnippet],
    hints: Option<&str>,
) -> (String, String) {
    let system = "You are a thoughtful character-design assistant. \
Read the user's recent conversations and stored personal notes, \
and propose a single JSON object describing the AI companion persona \
that would best fit them. Reply with ONLY the JSON object — no prose, \
no markdown fences, no commentary."
        .to_string();

    let transcript = render_snippets(snippets);
    let body = if transcript.is_empty() {
        "(No conversation or personal memories available — propose a sensible default persona.)"
            .to_string()
    } else {
        transcript
    };

    // Optional prosody-hint block (Chunk 14.6). Rendered as a
    // single-line aside between transcript and OUTPUT FORMAT so the
    // model treats it as context, not content to echo.
    let hints_block = hints
        .map(|h| h.trim())
        .filter(|h| !h.is_empty())
        .map(|h| format!("\n{}\n", h))
        .unwrap_or_default();

    let user = format!(
        "Below is recent conversation history and personal notes about the user.\n\
        Use them to infer a persona that would converse comfortably with this user.\n\n\
        ---\n{body}\n---\n{hints_block}\n\
        OUTPUT FORMAT — reply with exactly one JSON object, no prose, no fences:\n\
        {{\n  \"name\": \"<short character name>\",\n  \"role\": \"<one-line archetype>\",\n  \"bio\": \"<2–3 sentence backstory tailored to the user>\",\n  \"tone\": [\"<adjective>\", \"<adjective>\", \"<adjective>\"],\n  \"quirks\": [\"<short habit or catchphrase>\", \"...\"],\n  \"avoid\": [\"<a hard 'don't' for this companion>\", \"...\"]\n}}\n\n\
        Rules:\n\
        - Tone: 2–4 adjectives total.\n\
        - Quirks: 0–4 short phrases.\n\
        - Avoid: 1–3 entries; always include 'unsolicited medical, legal, or financial advice' unless the user explicitly invited it.\n\
        - bio MUST be at most 500 characters.\n\
        - Do not invent personal data the snippets do not support.\n\
        - Reply with ONLY the JSON object."
    );

    (system, user)
}

/// Render the snippet list into a single transcript block, respecting
/// [`PERSONA_PROMPT_CHAR_BUDGET`]. Empty bodies are skipped.
fn render_snippets(snippets: &[PromptSnippet]) -> String {
    let mut out = String::new();
    for s in snippets {
        let body = s.body.trim();
        if body.is_empty() {
            continue;
        }
        let chunk = format!("{}: {}\n", s.label.trim().to_uppercase(), body);
        if out.len() + chunk.len() > PERSONA_PROMPT_CHAR_BUDGET {
            break;
        }
        out.push_str(&chunk);
    }
    out.trim_end().to_string()
}

/// Build the snippet list a Tauri command would feed into
/// [`build_persona_prompt`]. Caller passes raw `(role, content)` chat
/// turns and `(content, tags)` memory rows; this function trims them to
/// the configured caps and applies the `personal:*` tag filter the
/// design doc calls for (see `docs/persona-design.md` § 9.3).
pub fn assemble_snippets(
    history: &[(String, String)],
    memories: &[(String, String)],
) -> Vec<PromptSnippet> {
    let mut out = Vec::with_capacity(
        PERSONA_PROMPT_HISTORY_TURNS + PERSONA_PROMPT_MEMORY_LIMIT,
    );

    // History: take the last N turns so the most-recent context wins.
    let start = history.len().saturating_sub(PERSONA_PROMPT_HISTORY_TURNS);
    for (role, content) in &history[start..] {
        out.push(PromptSnippet {
            label: role.clone(),
            body: content.clone(),
        });
    }

    // Memories: prefer rows tagged `personal:*` (case-insensitive); fall
    // back to plain long-term memories if the personal tag space is
    // empty so the prompt is never devoid of background.
    let personal: Vec<&(String, String)> = memories
        .iter()
        .filter(|(_, tags)| tags.to_lowercase().contains("personal:"))
        .take(PERSONA_PROMPT_MEMORY_LIMIT)
        .collect();
    let chosen: Vec<&(String, String)> = if personal.is_empty() {
        memories
            .iter()
            .take(PERSONA_PROMPT_MEMORY_LIMIT)
            .collect()
    } else {
        personal
    };
    for (content, _) in chosen {
        out.push(PromptSnippet {
            label: "MEMORY".to_string(),
            body: content.clone(),
        });
    }

    out
}

/// Parse a brain reply into a [`PersonaCandidate`]. Tolerant of:
/// - leading / trailing prose ("Sure! Here's the JSON: { ... }"),
/// - markdown fenced blocks (```json ... ``` or ``` ... ```),
/// - missing optional list fields (filled with `vec![]`),
/// - non-string list entries (silently dropped).
///
/// Returns `None` when no JSON object can be located or the required
/// fields (`name` / `role` / `bio`) are missing or empty.
pub fn parse_persona_reply(raw: &str) -> Option<PersonaCandidate> {
    let body = strip_fences(raw);
    let json_slice = extract_json_object(&body)?;
    let mut value: serde_json::Value = serde_json::from_str(json_slice).ok()?;
    sanitize_value(&mut value);
    let candidate: PersonaCandidate = serde_json::from_value(value).ok()?;
    if candidate.name.trim().is_empty()
        || candidate.role.trim().is_empty()
        || candidate.bio.trim().is_empty()
    {
        return None;
    }
    Some(normalise_candidate(candidate))
}

/// Drop opening / closing markdown fences when the brain ignores the
/// "no fences" instruction. Idempotent on already-clean input.
fn strip_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    // Pattern: ```{lang}\n…\n```
    if let Some(stripped) = trimmed.strip_prefix("```") {
        // Drop optional language tag on the first line.
        let after_first_line = stripped.split_once('\n').map(|(_, rest)| rest).unwrap_or(stripped);
        let cleaned = after_first_line
            .trim_end()
            .trim_end_matches("```")
            .trim_end();
        return cleaned.to_string();
    }
    trimmed.to_string()
}

/// Slice out the first `{ ... }` JSON object from `text`, balancing
/// braces and ignoring braces inside string literals. Returns `None`
/// when no balanced object can be located.
fn extract_json_object(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let start = text.find('{')?;
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Coerce list-of-strings fields to drop non-string entries so a model
/// emitting `tone: ["warm", 7, null, "concise"]` still parses.
fn sanitize_value(value: &mut serde_json::Value) {
    if let Some(obj) = value.as_object_mut() {
        for key in ["tone", "quirks", "avoid"] {
            if let Some(field) = obj.get_mut(key) {
                if let Some(arr) = field.as_array_mut() {
                    arr.retain(|v| v.is_string());
                }
            }
        }
    }
}

/// Normalise the candidate after deserialisation: trim whitespace, cap
/// the bio at 500 chars, deduplicate list entries (case-insensitive),
/// cap each list at 6 entries.
fn normalise_candidate(mut c: PersonaCandidate) -> PersonaCandidate {
    c.name = c.name.trim().to_string();
    c.role = c.role.trim().to_string();
    c.bio = c.bio.trim().chars().take(500).collect();
    c.tone = dedupe_cap(c.tone, 6);
    c.quirks = dedupe_cap(c.quirks, 6);
    c.avoid = dedupe_cap(c.avoid, 6);
    c
}

fn dedupe_cap(items: Vec<String>, cap: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(items.len().min(cap));
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for raw in items {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            out.push(trimmed);
            if out.len() >= cap {
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_includes_format_block_and_transcript() {
        let snippets = vec![
            PromptSnippet { label: "user".into(), body: "I love haiku.".into() },
            PromptSnippet { label: "assistant".into(), body: "Noted.".into() },
        ];
        let (system, user) = build_persona_prompt(&snippets);
        assert!(system.contains("ONLY the JSON object"));
        assert!(user.contains("USER: I love haiku."));
        assert!(user.contains("ASSISTANT: Noted."));
        assert!(user.contains("OUTPUT FORMAT"));
        assert!(user.contains("\"name\""));
    }

    #[test]
    fn build_prompt_handles_empty_history() {
        let (_system, user) = build_persona_prompt(&[]);
        assert!(user.contains("No conversation or personal memories"));
        assert!(user.contains("OUTPUT FORMAT"));
    }

    #[test]
    fn build_prompt_with_hints_includes_hint_block() {
        let snippets = vec![PromptSnippet { label: "user".into(), body: "yes".into() }];
        let (_s, user) = build_persona_prompt_with_hints(
            &snippets,
            Some("Voice-derived hints: tone: concise · pacing: fast."),
        );
        assert!(user.contains("Voice-derived hints"));
        assert!(user.contains("OUTPUT FORMAT"));
    }

    #[test]
    fn build_prompt_with_none_hints_matches_legacy_output() {
        let snippets = vec![PromptSnippet { label: "user".into(), body: "yes".into() }];
        let (s_a, u_a) = build_persona_prompt(&snippets);
        let (s_b, u_b) = build_persona_prompt_with_hints(&snippets, None);
        assert_eq!(s_a, s_b);
        assert_eq!(u_a, u_b);
    }

    #[test]
    fn build_prompt_with_blank_hints_is_treated_as_none() {
        let snippets = vec![PromptSnippet { label: "user".into(), body: "yes".into() }];
        let (_s, u_a) = build_persona_prompt(&snippets);
        let (_s, u_b) = build_persona_prompt_with_hints(&snippets, Some("   \n\t  "));
        assert_eq!(u_a, u_b);
    }

    #[test]
    fn render_snippets_respects_char_budget() {
        let huge = "X".repeat(PERSONA_PROMPT_CHAR_BUDGET + 100);
        let snippets = vec![
            PromptSnippet { label: "user".into(), body: huge.clone() },
            PromptSnippet { label: "user".into(), body: "tail".into() },
        ];
        let rendered = render_snippets(&snippets);
        // Either the huge snippet was the first (over-budget on its own
        // it is still emitted because we check before pushing); or it
        // got truncated. Either way, we must NOT have the tail snippet
        // appended past the cap.
        assert!(!rendered.ends_with("tail"));
    }

    #[test]
    fn render_snippets_skips_empty_bodies() {
        let snippets = vec![
            PromptSnippet { label: "user".into(), body: "   ".into() },
            PromptSnippet { label: "user".into(), body: "real".into() },
        ];
        let rendered = render_snippets(&snippets);
        assert_eq!(rendered, "USER: real");
    }

    #[test]
    fn assemble_snippets_takes_last_n_turns() {
        let history: Vec<(String, String)> = (0..PERSONA_PROMPT_HISTORY_TURNS + 5)
            .map(|i| ("user".into(), format!("turn-{i}")))
            .collect();
        let snippets = assemble_snippets(&history, &[]);
        assert_eq!(snippets.len(), PERSONA_PROMPT_HISTORY_TURNS);
        // First retained snippet should be the (N+5 - N) = 5th turn.
        assert_eq!(snippets[0].body, "turn-5");
        assert_eq!(snippets.last().unwrap().body, format!("turn-{}", PERSONA_PROMPT_HISTORY_TURNS + 4));
    }

    #[test]
    fn assemble_snippets_prefers_personal_tagged_memories() {
        let memories = vec![
            ("loves coffee".to_string(), "personal:preference".to_string()),
            ("HTTP/2 was finalised in 2015".to_string(), "general".to_string()),
            ("speaks Vietnamese".to_string(), "personal:language".to_string()),
        ];
        let snippets = assemble_snippets(&[], &memories);
        let bodies: Vec<&str> = snippets.iter().map(|s| s.body.as_str()).collect();
        assert!(bodies.contains(&"loves coffee"));
        assert!(bodies.contains(&"speaks Vietnamese"));
        assert!(!bodies.contains(&"HTTP/2 was finalised in 2015"));
    }

    #[test]
    fn assemble_snippets_falls_back_when_no_personal_tag() {
        let memories = vec![
            ("plain note A".to_string(), "general".to_string()),
            ("plain note B".to_string(), "general".to_string()),
        ];
        let snippets = assemble_snippets(&[], &memories);
        assert_eq!(snippets.len(), 2);
        assert_eq!(snippets[0].body, "plain note A");
    }

    #[test]
    fn parse_reply_clean_json() {
        let raw = r#"{"name":"Lia","role":"librarian","bio":"Quiet bookworm.","tone":["warm","concise"],"quirks":["hums"],"avoid":["medical advice"]}"#;
        let parsed = parse_persona_reply(raw).unwrap();
        assert_eq!(parsed.name, "Lia");
        assert_eq!(parsed.tone, vec!["warm", "concise"]);
    }

    #[test]
    fn parse_reply_strips_markdown_fences() {
        let raw = "```json\n{\n  \"name\": \"Pip\",\n  \"role\": \"imp\",\n  \"bio\": \"Tiny troublemaker.\"\n}\n```";
        let parsed = parse_persona_reply(raw).unwrap();
        assert_eq!(parsed.name, "Pip");
        assert!(parsed.tone.is_empty(), "missing list defaults to empty");
    }

    #[test]
    fn parse_reply_handles_leading_prose() {
        let raw = "Sure! Here's the persona JSON:\n\n{\"name\":\"Echo\",\"role\":\"oracle\",\"bio\":\"Whispers of advice.\"}\n\nHope that helps!";
        let parsed = parse_persona_reply(raw).unwrap();
        assert_eq!(parsed.name, "Echo");
    }

    #[test]
    fn parse_reply_drops_non_string_list_entries() {
        let raw = r#"{"name":"X","role":"y","bio":"z","tone":["a", 7, null, "b"]}"#;
        let parsed = parse_persona_reply(raw).unwrap();
        assert_eq!(parsed.tone, vec!["a", "b"]);
    }

    #[test]
    fn parse_reply_caps_bio_at_500_chars() {
        let long_bio = "B".repeat(800);
        let raw = format!(r#"{{"name":"X","role":"y","bio":"{long_bio}"}}"#);
        let parsed = parse_persona_reply(&raw).unwrap();
        assert_eq!(parsed.bio.len(), 500);
    }

    #[test]
    fn parse_reply_dedupes_lists_case_insensitive() {
        let raw = r#"{"name":"X","role":"y","bio":"z","tone":["Warm","warm","Concise","WARM"]}"#;
        let parsed = parse_persona_reply(raw).unwrap();
        assert_eq!(parsed.tone, vec!["Warm", "Concise"]);
    }

    #[test]
    fn parse_reply_caps_lists_at_six_entries() {
        let raw = r#"{"name":"X","role":"y","bio":"z","quirks":["a","b","c","d","e","f","g","h"]}"#;
        let parsed = parse_persona_reply(raw).unwrap();
        assert_eq!(parsed.quirks.len(), 6);
    }

    #[test]
    fn parse_reply_rejects_missing_required_fields() {
        // Missing bio.
        let raw = r#"{"name":"X","role":"y"}"#;
        assert!(parse_persona_reply(raw).is_none());
        // Empty name.
        let raw = r#"{"name":"   ","role":"y","bio":"z"}"#;
        assert!(parse_persona_reply(raw).is_none());
    }

    #[test]
    fn parse_reply_rejects_garbage() {
        assert!(parse_persona_reply("not json at all").is_none());
        assert!(parse_persona_reply("").is_none());
        assert!(parse_persona_reply("{ unbalanced ").is_none());
    }

    #[test]
    fn extract_json_object_skips_braces_inside_strings() {
        // The opening `{` inside the string literal must NOT throw the
        // brace counter off. The valid object is the outer one.
        let text = r#"prefix { "name": "}}}", "x": 1 } trailing"#;
        let slice = extract_json_object(text).unwrap();
        assert!(slice.starts_with('{'));
        assert!(slice.ends_with('}'));
        // Should round-trip as JSON.
        let v: serde_json::Value = serde_json::from_str(slice).unwrap();
        assert_eq!(v["name"], "}}}");
    }

    #[test]
    fn strip_fences_handles_plain_input() {
        assert_eq!(strip_fences("just text"), "just text");
        assert_eq!(strip_fences("```\n{}\n```"), "{}");
        assert_eq!(strip_fences("```json\n{\"a\":1}\n```"), "{\"a\":1}");
    }
}
