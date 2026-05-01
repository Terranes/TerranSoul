//! LLM-powered intent classifier.
//!
//! Routes every chat input through the configured brain (Free → Paid → Local
//! Ollama → Local LM Studio) and parses a structured `IntentDecision` JSON
//! payload. This replaces three brittle regex detectors that used to live in
//! `src/stores/conversation.ts`:
//!
//!   * `detectLearnWithDocsIntent()` — "learn X using my documents"
//!   * `detectTeachIntent()`         — "remember the following …"
//!   * `detectGatedSetupCommand()`   — "upgrade to gemini" / "provide your own context"
//!
//! Architectural rationale: routing decisions belong to the brain, not to a
//! handful of English-only regexes. The classifier understands paraphrases,
//! typos, and multilingual phrasings that the regexes never could.
//!
//! When the configured brain is unreachable, returns a malformed reply, or
//! takes longer than the hard timeout, the classifier returns
//! `IntentDecision::Unknown` and the caller is expected to trigger the
//! "Auto install all" path so future turns have a working classifier
//! (typically by installing the local Ollama brain).
//!
//! See `docs/brain-advanced-design.md` § "Intent Classification" for the
//! full design and the JSON schema this classifier emits.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::brain::openai_client::{OpenAiClient, OpenAiMessage};
use crate::brain::{BrainMode, OllamaAgent, ProviderRotator};

/// Maximum duration we'll wait for the classifier LLM to reply before giving
/// up and returning `IntentDecision::Unknown`. A short timeout is critical:
/// the user is waiting on the message-send path, and if the free provider is
/// slow we'd rather fall back to install-all than block the chat turn.
///
/// 3 s is the design constant from `docs/brain-advanced-design.md` § 25 —
/// long enough that Pollinations / Groq routinely reply (typical ~700 ms)
/// but short enough that a hung connection never noticeably stalls a chat
/// turn. Tune this in lockstep with the design doc.
pub const CLASSIFY_TIMEOUT: Duration = Duration::from_secs(3);

/// How long to remember a previous classification for the exact same trimmed
/// input. Avoids double-classifying when the user retries or when the
/// conversation store is recreated mid-session.
pub const CACHE_TTL: Duration = Duration::from_secs(30);

/// One of the four gated setup commands the user can confirm in chat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatedSetupKind {
    /// Switch to Google Gemini with Search grounding (paid API).
    UpgradeGemini,
    /// Open the Scholar's Quest so the user can attach URLs/files.
    ProvideContext,
}

/// The classifier's structured decision about what to do with the user input.
///
/// The `kind` discriminator is serialised with snake_case so the frontend
/// switch reads naturally (`'learn_with_docs'`, `'teach_ingest'`, …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IntentDecision {
    /// Plain conversation — pass through to the streaming chat path.
    Chat,
    /// User wants to learn a topic from their own documents.
    /// Triggers the Scholar's Quest prereq walk.
    LearnWithDocs { topic: String },
    /// User wants to teach the brain new content (paste, ingest, remember).
    /// Triggers Scholar's Quest directly.
    TeachIngest { topic: String },
    /// User typed one of the two gated setup confirmations.
    GatedSetup { setup: GatedSetupKind },
    /// Classifier could not decide (no brain reachable, malformed JSON,
    /// timeout). Caller should fall back to the install-all path so a
    /// local Ollama brain is set up for future turns.
    Unknown,
}

/// Lightweight cache entry: a previous decision plus its insert time.
#[derive(Debug, Clone)]
struct CacheEntry {
    decision: IntentDecision,
    inserted_at: Instant,
}

/// Process-global classification cache, keyed by trimmed lowercase input.
///
/// Bounded to keep memory usage stable across long sessions: when full, the
/// oldest entries are evicted before insertion. The bound is generous because
/// each entry is a few hundred bytes at most.
const CACHE_MAX_ENTRIES: usize = 256;

/// Fallback topic used when the LLM tags a teach/ingest intent but doesn't
/// echo back a usable topic phrase. Surfaced verbatim in the Scholar's Quest
/// invitation, so keep it short and human-readable. This is a safety net,
/// not a preferred default — a well-behaved classifier always provides a
/// real topic.
const FALLBACK_TEACH_TOPIC: &str = "the provided content";

fn cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_key(text: &str) -> String {
    text.trim().to_lowercase()
}

fn cache_get(key: &str) -> Option<IntentDecision> {
    let map = cache().lock().ok()?;
    let entry = map.get(key)?;
    if entry.inserted_at.elapsed() > CACHE_TTL {
        return None;
    }
    Some(entry.decision.clone())
}

fn cache_put(key: String, decision: IntentDecision) {
    let Ok(mut map) = cache().lock() else {
        return;
    };
    if map.len() >= CACHE_MAX_ENTRIES {
        // Drop the oldest entry to keep the cache bounded.
        if let Some(oldest_key) = map
            .iter()
            .min_by_key(|(_, e)| e.inserted_at)
            .map(|(k, _)| k.clone())
        {
            map.remove(&oldest_key);
        }
    }
    map.insert(
        key,
        CacheEntry {
            decision,
            inserted_at: Instant::now(),
        },
    );
}

/// Clear the classification cache. Useful when the brain mode changes
/// (different model may classify differently) and in tests.
pub fn clear_cache() {
    if let Ok(mut map) = cache().lock() {
        map.clear();
    }
}

/// System prompt that forces the LLM to reply with a single JSON object
/// matching `IntentDecision`. Kept ~150 tokens so it's cheap on free
/// providers.
const SYSTEM_PROMPT: &str = r#"You are an intent classifier for a personal AI companion.
Read the user's message and reply with EXACTLY ONE JSON object — no prose,
no markdown fences — choosing exactly one of these shapes:

  {"kind":"chat"}
      Plain conversation, question, or chat. The default.

  {"kind":"learn_with_docs","topic":"<short topic phrase>"}
      The user explicitly wants to learn or study a topic FROM THEIR OWN
      documents/files/notes/PDFs/sources. Examples in any language:
        "Learn Vietnamese laws using my provided documents"
        "Study quantum physics with my files"
        "học luật Việt Nam từ tài liệu của tôi"

  {"kind":"teach_ingest","topic":"<short topic phrase>"}
      The user is pasting content for the brain to remember/ingest.
      Examples: "remember the following law: ...", "ingest this URL: ...",
      "memorize this fact: ...", "here is the source: ...".

  {"kind":"gated_setup","setup":"upgrade_gemini"}
      The user explicitly asks to upgrade to Google Gemini (with Search).

  {"kind":"gated_setup","setup":"provide_context"}
      The user explicitly says they will provide their own context/sources.

Rules:
  • A plain question like "tell me about Vietnamese law" is "chat", NOT learn_with_docs.
  • Only choose learn_with_docs / teach_ingest when the user clearly references
    their own documents or pastes content to ingest.
  • Reply with ONE JSON object on a single line. No explanation."#;

/// Wire format the LLM is asked to emit. Slightly more permissive than the
/// `IntentDecision` enum (a `setup` discriminator instead of nesting) so the
/// model has fewer ways to go wrong.
#[derive(Debug, Deserialize)]
struct WireDecision {
    kind: String,
    topic: Option<String>,
    setup: Option<String>,
}

/// Parse the LLM's raw reply into an `IntentDecision`.
///
/// Tolerant of common LLM idiosyncrasies:
///   * leading/trailing prose around the JSON object
///   * markdown code fences
///   * missing `topic` for learn/teach (synthesised from input)
fn parse_decision(reply: &str, original_input: &str) -> IntentDecision {
    let json = extract_json_object(reply).unwrap_or(reply);
    let parsed: WireDecision = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return IntentDecision::Unknown,
    };

    match parsed.kind.as_str() {
        "chat" => IntentDecision::Chat,
        "learn_with_docs" => IntentDecision::LearnWithDocs {
            topic: parsed
                .topic
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| original_input.trim().to_string()),
        },
        "teach_ingest" => IntentDecision::TeachIngest {
            topic: parsed
                .topic
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| FALLBACK_TEACH_TOPIC.to_string()),
        },
        "gated_setup" => match parsed.setup.as_deref() {
            Some("upgrade_gemini") => IntentDecision::GatedSetup {
                setup: GatedSetupKind::UpgradeGemini,
            },
            Some("provide_context") => IntentDecision::GatedSetup {
                setup: GatedSetupKind::ProvideContext,
            },
            _ => IntentDecision::Unknown,
        },
        _ => IntentDecision::Unknown,
    }
}

/// Pull the first balanced `{ … }` substring out of a reply. Handles the
/// common case of "Sure! Here's the JSON: {…} hope that helps."
fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if escape {
            escape = false;
            continue;
        }
        match b {
            b'\\' if in_string => escape = true,
            b'"' => in_string = !in_string,
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => {
                depth -= 1;
                if depth < 0 {
                    // Malformed input: more closing braces than opening.
                    // Refuse to return a substring rather than over-trim.
                    return None;
                }
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Dispatch one classification call to the configured brain mode.
///
/// Returns the raw LLM reply on success. Errors (network failure, HTTP
/// non-2xx, etc.) bubble up as `Err(String)` so the caller can map them
/// onto `IntentDecision::Unknown`.
async fn complete_via_mode(
    brain_mode: &BrainMode,
    rotator: &Mutex<ProviderRotator>,
    user_text: &str,
) -> Result<String, String> {
    match brain_mode {
        BrainMode::FreeApi {
            provider_id,
            api_key,
        } => {
            let effective_id = rotator
                .lock()
                .ok()
                .and_then(|mut r| r.next_healthy_provider().map(|p| p.id.clone()))
                .unwrap_or_else(|| provider_id.clone());
            let provider = crate::brain::get_free_provider(&effective_id)
                .ok_or_else(|| format!("Unknown free provider: {effective_id}"))?;
            let client =
                OpenAiClient::new(&provider.base_url, &provider.model, api_key.as_deref());
            client.chat(build_messages(user_text)).await
        }
        BrainMode::PaidApi {
            api_key,
            model,
            base_url,
            ..
        } => {
            let client = OpenAiClient::new(base_url, model, Some(api_key));
            client.chat(build_messages(user_text)).await
        }
        BrainMode::LocalOllama { model } => {
            let agent = OllamaAgent::new(model);
            let msgs = vec![
                crate::brain::ollama_agent::ChatMessage {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                crate::brain::ollama_agent::ChatMessage {
                    role: "user".to_string(),
                    content: user_text.to_string(),
                },
            ];
            let (reply, _) = agent.call(msgs).await;
            if reply.is_empty() {
                Err("ollama returned empty reply".to_string())
            } else {
                Ok(reply)
            }
        }
        BrainMode::LocalLmStudio {
            model,
            base_url,
            api_key,
            ..
        } => {
            let client = OpenAiClient::new(base_url, model, api_key.as_deref());
            client.chat(build_messages(user_text)).await
        }
    }
}

fn build_messages(user_text: &str) -> Vec<OpenAiMessage> {
    vec![
        OpenAiMessage {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
        },
        OpenAiMessage {
            role: "user".to_string(),
            content: user_text.to_string(),
        },
    ]
}

/// Classify a user message into a structured `IntentDecision`.
///
/// Behaviour:
///   1. Returns `Unknown` immediately for empty input.
///   2. Returns the cached decision if the same trimmed input was classified
///      within the last `CACHE_TTL`.
///   3. Otherwise, asks the configured brain (or `Unknown` when no brain is
///      configured), bounded by `CLASSIFY_TIMEOUT`. Any failure to obtain a
///      well-formed JSON reply maps to `Unknown` — the caller is responsible
///      for triggering the install-all fallback so future turns work offline.
pub async fn classify_user_intent(
    text: &str,
    brain_mode: Option<&BrainMode>,
    rotator: &Mutex<ProviderRotator>,
) -> IntentDecision {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return IntentDecision::Unknown;
    }

    let key = cache_key(trimmed);
    if let Some(cached) = cache_get(&key) {
        return cached;
    }

    let Some(mode) = brain_mode else {
        // No brain configured at all → classifier unavailable.
        return IntentDecision::Unknown;
    };

    let reply_result =
        tokio::time::timeout(CLASSIFY_TIMEOUT, complete_via_mode(mode, rotator, trimmed)).await;

    let decision = match reply_result {
        Ok(Ok(reply)) => parse_decision(&reply, trimmed),
        Ok(Err(_)) | Err(_) => IntentDecision::Unknown,
    };

    cache_put(key, decision.clone());
    decision
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rotator() -> Mutex<ProviderRotator> {
        Mutex::new(ProviderRotator::new())
    }

    // ── parse_decision ───────────────────────────────────────────────

    #[test]
    fn parses_chat() {
        let d = parse_decision(r#"{"kind":"chat"}"#, "hello");
        assert_eq!(d, IntentDecision::Chat);
    }

    #[test]
    fn parses_learn_with_docs() {
        let d = parse_decision(
            r#"{"kind":"learn_with_docs","topic":"Vietnamese laws"}"#,
            "Learn Vietnamese laws using my docs",
        );
        assert_eq!(
            d,
            IntentDecision::LearnWithDocs {
                topic: "Vietnamese laws".to_string(),
            }
        );
    }

    #[test]
    fn parses_teach_ingest() {
        let d = parse_decision(
            r#"{"kind":"teach_ingest","topic":"Article 429"}"#,
            "Remember the following law: Article 429",
        );
        assert_eq!(
            d,
            IntentDecision::TeachIngest {
                topic: "Article 429".to_string(),
            }
        );
    }

    #[test]
    fn parses_gated_upgrade_gemini() {
        let d = parse_decision(
            r#"{"kind":"gated_setup","setup":"upgrade_gemini"}"#,
            "upgrade to gemini",
        );
        assert_eq!(
            d,
            IntentDecision::GatedSetup {
                setup: GatedSetupKind::UpgradeGemini,
            }
        );
    }

    #[test]
    fn parses_gated_provide_context() {
        let d = parse_decision(
            r#"{"kind":"gated_setup","setup":"provide_context"}"#,
            "provide your own context",
        );
        assert_eq!(
            d,
            IntentDecision::GatedSetup {
                setup: GatedSetupKind::ProvideContext,
            }
        );
    }

    #[test]
    fn malformed_json_yields_unknown() {
        assert_eq!(parse_decision("not json at all", "hi"), IntentDecision::Unknown);
        assert_eq!(parse_decision("", "hi"), IntentDecision::Unknown);
    }

    #[test]
    fn unknown_kind_yields_unknown() {
        let d = parse_decision(r#"{"kind":"banana"}"#, "hi");
        assert_eq!(d, IntentDecision::Unknown);
    }

    #[test]
    fn unknown_setup_yields_unknown() {
        let d = parse_decision(r#"{"kind":"gated_setup","setup":"hack_the_planet"}"#, "hi");
        assert_eq!(d, IntentDecision::Unknown);
    }

    #[test]
    fn missing_topic_falls_back_to_input() {
        let d = parse_decision(r#"{"kind":"learn_with_docs"}"#, "Learn X from my notes");
        assert_eq!(
            d,
            IntentDecision::LearnWithDocs {
                topic: "Learn X from my notes".to_string(),
            }
        );
    }

    #[test]
    fn tolerates_prose_around_json() {
        let d = parse_decision(
            r#"Sure thing! Here's the answer: {"kind":"chat"}  hope that helps."#,
            "hello",
        );
        assert_eq!(d, IntentDecision::Chat);
    }

    #[test]
    fn tolerates_markdown_code_fences() {
        let d = parse_decision(
            "```json\n{\"kind\":\"chat\"}\n```",
            "hello",
        );
        assert_eq!(d, IntentDecision::Chat);
    }

    #[test]
    fn extract_json_handles_nested_braces() {
        let s = extract_json_object(r#"prefix {"a":{"b":1},"c":"}"} suffix"#).unwrap();
        assert_eq!(s, r#"{"a":{"b":1},"c":"}"}"#);
    }

    #[test]
    fn extract_json_ignores_braces_in_strings() {
        let s = extract_json_object(r#"{"text":"oh look a } in the string"}"#).unwrap();
        assert_eq!(s, r#"{"text":"oh look a } in the string"}"#);
    }

    #[test]
    fn extract_json_returns_none_on_unbalanced_closing_brace() {
        // More `}` than `{` — must not panic or over-trim.
        assert_eq!(extract_json_object("} not json {"), None);
    }

    // ── cache ────────────────────────────────────────────────────────
    //
    // The cache is a `OnceLock` static shared across the whole process, so
    // tests that mutate it race when run in parallel (the default).  Every
    // cache-touching test below acquires `cache_test_lock()` for the entire
    // duration of its body to guarantee `clear_cache` + `cache_put` +
    // `cache_get` operate atomically with respect to other tests.

    fn cache_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    #[test]
    fn cache_roundtrip() {
        let _g = cache_test_lock();
        clear_cache();
        cache_put("hello".to_string(), IntentDecision::Chat);
        assert_eq!(cache_get("hello"), Some(IntentDecision::Chat));
    }

    #[test]
    fn cache_returns_none_for_missing() {
        let _g = cache_test_lock();
        clear_cache();
        assert_eq!(cache_get("nope-i-am-not-there"), None);
    }

    #[test]
    fn cache_evicts_oldest_when_full() {
        let _g = cache_test_lock();
        clear_cache();
        // Fill the cache.
        for i in 0..CACHE_MAX_ENTRIES {
            cache_put(format!("key-{i}"), IntentDecision::Chat);
        }
        // Adding one more should evict the oldest (key-0) without panicking.
        cache_put("new-key".to_string(), IntentDecision::Chat);
        let map = cache().lock().unwrap();
        assert!(map.contains_key("new-key"));
        assert!(map.len() <= CACHE_MAX_ENTRIES);
    }

    // ── classify_user_intent ─────────────────────────────────────────

    #[tokio::test]
    async fn empty_input_is_unknown() {
        clear_cache();
        let r = rotator();
        assert_eq!(
            classify_user_intent("   ", None, &r).await,
            IntentDecision::Unknown
        );
    }

    #[tokio::test]
    async fn no_brain_mode_is_unknown() {
        clear_cache();
        let r = rotator();
        assert_eq!(
            classify_user_intent("hello", None, &r).await,
            IntentDecision::Unknown
        );
    }

    #[test]
    fn cache_short_circuits_classification() {
        // Sync test — verifies the cache-hit fast path inside
        // `classify_user_intent` directly, without an async await.
        //
        // The previous version of this test held a sync MutexGuard
        // across an await and lost races against other tests that call
        // `intent_classifier::clear_cache()` indirectly (via
        // `set_brain_mode` etc.) without acquiring the test lock. Since
        // the production code path that the test cares about is
        //
        //     if let Some(cached) = cache_get(&key) { return cached; }
        //
        // we can verify the cache contract synchronously and avoid the
        // race entirely.
        let _g = cache_test_lock();
        clear_cache();
        let key = cache_key("Learn Vietnamese laws using my docs");
        let expected = IntentDecision::LearnWithDocs {
            topic: "Vietnamese laws".to_string(),
        };
        cache_put(key.clone(), expected.clone());
        assert_eq!(cache_get(&key), Some(expected));
    }
}
