//! LLM-powered auto-tagging for memory entries (Chunk 18.1).
//!
//! When `AppSettings.auto_tag` is `true`, every `add_memory` call runs a
//! fast LLM pass that classifies the content into ≤ 4 tags drawn from the
//! curated prefix vocabulary in [`super::tag_vocabulary::CURATED_PREFIXES`].
//!
//! The tagger is brain-mode agnostic: it dispatches to Ollama (local) or
//! OpenAI-compatible (free / paid) APIs depending on the active
//! [`BrainMode`].
//!
//! Maps to `docs/brain-advanced-design.md` §16 Phase 2 row
//! "Auto-categorise via LLM on insert" (chunk 18.1).

use crate::brain::{BrainMode, OllamaAgent, OpenAiClient};
use crate::brain::ollama_agent::ChatMessage;
use crate::brain::openai_client::OpenAiMessage;
use crate::memory::tag_vocabulary::{CURATED_PREFIXES, validate_csv, TagValidation};

/// Build the system prompt for the auto-tagger.
fn system_prompt() -> String {
    let prefixes = CURATED_PREFIXES.join(", ");
    format!(
        "You are a memory classification assistant. \
         Assign up to 4 tags to the given text. \
         Each tag MUST use the format `prefix:value` where prefix is one of: {prefixes}. \
         Reply with ONLY a comma-separated list of tags, nothing else. \
         Example: personal:likes-coffee, domain:rust-programming"
    )
}

/// Build the user prompt for the auto-tagger.
fn user_prompt(content: &str) -> String {
    format!("Classify this memory:\n\n{content}")
}

/// Parse the LLM's comma-separated tag response into validated tags.
///
/// Keeps only tags that pass [`validate_csv`] as `Curated`; silently
/// drops malformed / non-conforming suggestions. Caps at 4 tags.
pub fn parse_tag_response(raw: &str) -> Vec<String> {
    let cleaned = raw.trim().trim_matches('"').trim();
    if cleaned.is_empty() || cleaned.eq_ignore_ascii_case("none") {
        return vec![];
    }

    let verdicts = validate_csv(cleaned);
    let tags: Vec<String> = cleaned
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .zip(verdicts.iter())
        .filter(|(_, v)| matches!(v, TagValidation::Curated { .. }))
        .map(|(t, _)| t.to_string())
        .take(4)
        .collect();

    tags
}

/// Merge auto-generated tags with user-supplied tags (deduplication by
/// exact match). User tags take precedence — auto-tags are appended only
/// if the exact string is not already present.
pub fn merge_tags(user_tags: &str, auto_tags: &[String]) -> String {
    let existing: Vec<&str> = user_tags
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();

    let mut merged: Vec<String> = existing.iter().map(|t| t.to_string()).collect();
    for tag in auto_tags {
        if !existing.iter().any(|e| e.eq_ignore_ascii_case(tag)) {
            merged.push(tag.clone());
        }
    }
    merged.join(",")
}

/// Run the auto-tagger against the active brain mode.
///
/// Returns a list of curated-prefix tags (up to 4), or an empty vec when
/// the brain is unavailable or the LLM response is unparseable.
pub async fn auto_tag_content(content: &str, brain_mode: &BrainMode) -> Vec<String> {
    if content.trim().is_empty() {
        return vec![];
    }

    let sys = system_prompt();
    let user = user_prompt(content);

    let reply = match brain_mode {
        BrainMode::LocalOllama { model } => {
            let agent = OllamaAgent::new(model);
            let msgs = vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: sys,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user,
                },
            ];
            let (response, _) = agent.call(msgs).await;
            response
        }
        BrainMode::FreeApi { provider_id, api_key } => {
            let provider = match crate::brain::get_free_provider(provider_id) {
                Some(p) => p,
                None => return vec![],
            };
            let client = OpenAiClient::new(
                &provider.base_url,
                &provider.model,
                api_key.as_deref(),
            );
            let msgs = vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: sys,
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user,
                },
            ];
            match client.chat(msgs).await {
                Ok(r) => r,
                Err(_) => return vec![],
            }
        }
        BrainMode::PaidApi { provider: _, api_key, model, base_url } => {
            let client = OpenAiClient::new(base_url, model, Some(api_key));
            let msgs = vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: sys,
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user,
                },
            ];
            match client.chat(msgs).await {
                Ok(r) => r,
                Err(_) => return vec![],
            }
        }
        BrainMode::LocalLmStudio {
            model,
            base_url,
            api_key,
            ..
        } => {
            let client = OpenAiClient::new(base_url, model, api_key.as_deref());
            let msgs = vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: sys,
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user,
                },
            ];
            match client.chat(msgs).await {
                Ok(r) => r,
                Err(_) => return vec![],
            }
        }
    };

    parse_tag_response(&reply)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_curated_tags() {
        let raw = "personal:likes-coffee, domain:rust-programming, code:async-patterns";
        let tags = parse_tag_response(raw);
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0], "personal:likes-coffee");
        assert_eq!(tags[1], "domain:rust-programming");
        assert_eq!(tags[2], "code:async-patterns");
    }

    #[test]
    fn parse_drops_non_conforming_tags() {
        let raw = "personal:name, invalid-tag, domain:law, another-bad";
        let tags = parse_tag_response(raw);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0], "personal:name");
        assert_eq!(tags[1], "domain:law");
    }

    #[test]
    fn parse_caps_at_four_tags() {
        let raw = "personal:a, domain:b, code:c, tool:d, project:e";
        let tags = parse_tag_response(raw);
        assert_eq!(tags.len(), 4);
    }

    #[test]
    fn parse_empty_and_none() {
        assert!(parse_tag_response("").is_empty());
        assert!(parse_tag_response("NONE").is_empty());
        assert!(parse_tag_response("  none  ").is_empty());
    }

    #[test]
    fn parse_strips_surrounding_quotes() {
        let raw = "\"personal:likes-tea, domain:cooking\"";
        let tags = parse_tag_response(raw);
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn merge_deduplicates_by_case_insensitive() {
        let user = "personal:name";
        let auto = vec!["Personal:Name".to_string(), "domain:law".to_string()];
        let merged = merge_tags(user, &auto);
        assert_eq!(merged, "personal:name,domain:law");
    }

    #[test]
    fn merge_preserves_user_tags_first() {
        let user = "fact,preference";
        let auto = vec!["personal:goals".to_string()];
        let merged = merge_tags(user, &auto);
        assert_eq!(merged, "fact,preference,personal:goals");
    }

    #[test]
    fn merge_empty_user_tags() {
        let auto = vec!["domain:law".to_string(), "code:rust".to_string()];
        let merged = merge_tags("", &auto);
        assert_eq!(merged, "domain:law,code:rust");
    }

    #[test]
    fn system_prompt_includes_all_prefixes() {
        let prompt = system_prompt();
        for prefix in CURATED_PREFIXES {
            assert!(prompt.contains(prefix), "missing prefix: {prefix}");
        }
    }

    #[test]
    fn user_prompt_includes_content() {
        let prompt = user_prompt("I love Vietnamese pho");
        assert!(prompt.contains("I love Vietnamese pho"));
    }
}
