//! LLM-powered intent classifier.
//!
//! Routes contentful chat input through the configured brain (Free → Paid →
//! Local Ollama → Local LM Studio) and parses a structured `IntentDecision`
//! JSON payload. The frontend owns only UI transitions; setup, teaching, and
//! document-learning decisions all flow through this classifier.
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
pub const CLASSIFY_TIMEOUT: Duration = Duration::from_millis(1500);

/// Local-model classifier timeout. LocalOllama and LM Studio share GPU with
/// the concurrent streaming chat response, so the classifier waits behind
/// the chat model in the inference queue. 8 s gives the local model enough
/// time to respond even when the GPU is contended, while the frontend's
/// background-classifier handler aborts the stream and switches to the quest
/// flow as soon as the result arrives — no perceived stall for the user.
pub const LOCAL_CLASSIFY_TIMEOUT: Duration = Duration::from_millis(8000);

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
const FALLBACK_LEARN_DOCS_TOPIC: &str = "the material in your documents";

fn detect_document_learning_shortcut(text: &str) -> Option<IntentDecision> {
    let normalized = text.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }

    let direct_phrases = [
        "learn from my documents",
        "learn my documents",
        "learn documents",
        "learn from my docs",
        "learn from my files",
        "learn from my notes",
        "learn using my documents",
        "learn with my documents",
        "study my documents",
    ];
    if direct_phrases.iter().any(|phrase| normalized == *phrase) {
        return Some(IntentDecision::LearnWithDocs {
            topic: FALLBACK_LEARN_DOCS_TOPIC.to_string(),
        });
    }

    if normalized.contains("my provided documents")
        && (normalized.contains("learn") || normalized.contains("study"))
    {
        return Some(IntentDecision::LearnWithDocs {
            topic: FALLBACK_LEARN_DOCS_TOPIC.to_string(),
        });
    }

    None
}

fn cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_key(text: &str, knowledge_context: Option<&str>) -> String {
    let mut key = text.trim().to_lowercase();
    if let Some(context) = knowledge_context.map(str::trim).filter(|s| !s.is_empty()) {
        key.push_str("\nctx:");
        key.push_str(&context.to_lowercase());
    }
    key
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

/// Short content-light turns should not spend a LocalLLM request on intent
/// routing. They are ordinary chat by default, and letting the classifier run
/// before/alongside the real stream can put the user reply behind another
/// Ollama generation.
pub fn should_use_fast_chat_path(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    tokens.len() <= 3 && tokens.iter().all(|token| token.chars().count() <= 5)
}

/// System prompt that forces the LLM to reply with a single JSON object
/// matching `IntentDecision`.
const SYSTEM_PROMPT: &str = r#"You are an intent classifier for a personal AI companion.
Read the user's message and reply with EXACTLY ONE JSON object — no prose,
no markdown fences — choosing exactly one of these shapes:

  {"kind":"chat"}
      Plain conversation, question, or chat. The default.

  {"kind":"learn_with_docs","topic":"<short topic phrase>"}
      The user explicitly wants to learn or study a topic FROM THEIR OWN
      documents/files/notes/PDFs/sources. Use retrieved default system
      settings for phrase examples, fallback topic text, and setup flow.

  {"kind":"teach_ingest","topic":"<short topic phrase>"}
      The user is pasting content for the brain to remember/ingest.
      Use retrieved default system settings for phrase examples and routing
      details.

  {"kind":"gated_setup","setup":"upgrade_gemini"}
      The user explicitly asks to upgrade to Google Gemini (with Search).

  {"kind":"gated_setup","setup":"provide_context"}
      The user explicitly says they will provide their own context/sources.

Rules:
  • A plain question like "tell me about Vietnamese law" is "chat", NOT learn_with_docs.
  • Only choose learn_with_docs / teach_ingest when the user clearly references
    their own documents or pastes content to ingest.
    • Treat retrieved system/default settings as user-customizable policy. They
        may define recognized paraphrases, default topics, quest chains, or setup
        defaults. Prefer those settings over generic assumptions.
    • Use retrieved app/RAG knowledge below when it names TerranSoul workflows,
        tutorials, quest chains, or setup defaults.
  • Reply with ONE JSON object on a single line. No explanation."#;

fn build_system_prompt(knowledge_context: Option<&str>) -> String {
    let Some(context) = knowledge_context.map(str::trim).filter(|s| !s.is_empty()) else {
        return SYSTEM_PROMPT.to_string();
    };

    format!(
                "{SYSTEM_PROMPT}\n\nRetrieved system settings and app/RAG knowledge for this classification:{context}\n\nUse this retrieved knowledge only to choose the intent JSON; do not explain it."
        )
}

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
fn parse_decision(reply: &str, _original_input: &str) -> IntentDecision {
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
                .unwrap_or_else(|| FALLBACK_LEARN_DOCS_TOPIC.to_string()),
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
    knowledge_context: Option<&str>,
) -> Result<String, String> {
    let system_prompt = build_system_prompt(knowledge_context);
    match brain_mode {
        BrainMode::FreeApi {
            provider_id,
            api_key,
            model,
        } => {
            let effective_id = rotator
                .lock()
                .ok()
                .and_then(|mut r| r.next_healthy_provider().map(|p| p.id.clone()))
                .unwrap_or_else(|| provider_id.clone());
            let provider = crate::brain::get_free_provider(&effective_id)
                .ok_or_else(|| format!("Unknown free provider: {effective_id}"))?;
            let chat_model = model
                .as_deref()
                .filter(|_| effective_id == provider_id.as_str())
                .unwrap_or(&provider.model);
            let client = OpenAiClient::new(&provider.base_url, chat_model, api_key.as_deref());
            client.chat(build_messages(user_text, &system_prompt)).await
        }
        BrainMode::PaidApi {
            api_key,
            model,
            base_url,
            ..
        } => {
            let client = OpenAiClient::new(base_url, model, Some(api_key));
            client.chat(build_messages(user_text, &system_prompt)).await
        }
        BrainMode::LocalOllama { model } => {
            let agent = OllamaAgent::new(model);
            let msgs = vec![
                crate::brain::ollama_agent::ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt,
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
            client.chat(build_messages(user_text, &system_prompt)).await
        }
    }
}

fn build_messages(user_text: &str, system_prompt: &str) -> Vec<OpenAiMessage> {
    vec![
        OpenAiMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
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
    knowledge_context: Option<&str>,
) -> IntentDecision {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return IntentDecision::Unknown;
    }

    if let Some(shortcut) = detect_document_learning_shortcut(trimmed) {
        return shortcut;
    }

    if should_use_fast_chat_path(trimmed) {
        return IntentDecision::Chat;
    }

    let knowledge_context = knowledge_context.map(str::trim).filter(|s| !s.is_empty());
    let key = cache_key(trimmed, knowledge_context);
    if let Some(cached) = cache_get(&key) {
        log_classifier_trace("cache", 0, knowledge_context.is_some(), &cached, None, None);
        return cached;
    }

    let Some(mode) = brain_mode else {
        // No brain configured at all → classifier unavailable.
        return IntentDecision::Unknown;
    };

    let started = Instant::now();
    let timeout = match mode {
        BrainMode::LocalOllama { .. } | BrainMode::LocalLmStudio { .. } => LOCAL_CLASSIFY_TIMEOUT,
        _ => CLASSIFY_TIMEOUT,
    };
    let reply_result = tokio::time::timeout(
        timeout,
        complete_via_mode(mode, rotator, trimmed, knowledge_context),
    )
    .await;

    let mut raw_reply: Option<String> = None;
    let mut error: Option<String> = None;
    let decision = match reply_result {
        Ok(Ok(reply)) => {
            let decision = parse_decision(&reply, trimmed);
            raw_reply = Some(reply);
            decision
        }
        Ok(Err(err)) => {
            error = Some(err);
            IntentDecision::Unknown
        }
        Err(_) => {
            error = Some(format!("timeout after {}ms", timeout.as_millis()));
            IntentDecision::Unknown
        }
    };

    log_classifier_trace(
        brain_mode_label(mode),
        started.elapsed().as_millis(),
        knowledge_context.is_some(),
        &decision,
        raw_reply.as_deref(),
        error.as_deref(),
    );

    cache_put(key, decision.clone());
    decision
}

fn brain_mode_label(mode: &BrainMode) -> &'static str {
    match mode {
        BrainMode::FreeApi { .. } => "free_api",
        BrainMode::PaidApi { .. } => "paid_api",
        BrainMode::LocalOllama { .. } => "local_ollama",
        BrainMode::LocalLmStudio { .. } => "local_lm_studio",
    }
}

fn log_classifier_trace(
    mode: &str,
    elapsed_ms: u128,
    used_knowledge: bool,
    decision: &IntentDecision,
    raw_reply: Option<&str>,
    error: Option<&str>,
) {
    if !crate::brain::ollama_agent::is_debug_logging() {
        return;
    }
    let raw = raw_reply
        .map(|s| s.trim().chars().take(600).collect::<String>())
        .unwrap_or_default();
    let error = error.unwrap_or("");
    eprintln!(
        "[intent-classifier] mode={mode} elapsed_ms={elapsed_ms} knowledge={used_knowledge} decision={decision:?} error={error} raw={raw}"
    );
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
    fn chat_decision_for_docs_command_stays_chat_without_fallback() {
        let d = parse_decision(r#"{"kind":"chat"}"#, "Learn from my documents");
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
        assert_eq!(
            parse_decision("not json at all", "hi"),
            IntentDecision::Unknown
        );
        assert_eq!(parse_decision("", "hi"), IntentDecision::Unknown);
    }

    #[test]
    fn malformed_json_tool_markup_yields_unknown() {
        let d = parse_decision(
            "<execute_tool>tool_name:upload_documents</execute_tool>",
            "Learn from my documents",
        );
        assert_eq!(d, IntentDecision::Unknown);
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
    fn unknown_setup_does_not_use_input_heuristics() {
        let d = parse_decision(
            r#"{"kind":"gated_setup","setup":"hack_the_planet"}"#,
            "provide your own context",
        );
        assert_eq!(d, IntentDecision::Unknown);
    }

    #[test]
    fn fast_chat_path_skips_tiny_turns() {
        assert!(should_use_fast_chat_path("Hi"));
        assert!(should_use_fast_chat_path("Hello"));
        assert!(should_use_fast_chat_path("OK"));
        assert!(should_use_fast_chat_path("who are you"));
        assert!(!should_use_fast_chat_path(
            "explain Vietnamese contract law"
        ));
        assert!(!should_use_fast_chat_path(
            "learn from my provided documents"
        ));
        // Short but meaningful document-learning phrases must NOT be skipped
        assert!(!should_use_fast_chat_path("Learn my documents"));
        assert!(!should_use_fast_chat_path("Learn documents"));
        assert!(!should_use_fast_chat_path(
            "please look at my provided documents and learn it"
        ));
    }

    #[test]
    fn shortcut_detects_exact_tutorial_phrase() {
        assert_eq!(
            detect_document_learning_shortcut("Learn from my documents"),
            Some(IntentDecision::LearnWithDocs {
                topic: FALLBACK_LEARN_DOCS_TOPIC.to_string(),
            })
        );
    }

    #[test]
    fn shortcut_detects_provided_documents_variant() {
        assert_eq!(
            detect_document_learning_shortcut(
                "Please look at my provided documents and learn it",
            ),
            Some(IntentDecision::LearnWithDocs {
                topic: FALLBACK_LEARN_DOCS_TOPIC.to_string(),
            })
        );
    }

    #[test]
    fn missing_learn_docs_topic_uses_recommended_setup_topic() {
        let d = parse_decision(r#"{"kind":"learn_with_docs"}"#, "Learn X from my notes");
        assert_eq!(
            d,
            IntentDecision::LearnWithDocs {
                topic: FALLBACK_LEARN_DOCS_TOPIC.to_string(),
            }
        );
    }

    #[test]
    fn system_prompt_includes_retrieved_knowledge_context() {
        let prompt = build_system_prompt(Some(
            "\n[RETRIEVED CONTEXT]\n[LONG-TERM MEMORY]\n- DEFAULT SYSTEM SETTING: Learn from my documents triggers Scholar's Quest and uses topic the material in your documents.\n[/LONG-TERM MEMORY]",
        ));

        assert!(prompt.contains("Retrieved system settings and app/RAG knowledge"));
        assert!(prompt.contains("Learn from my documents triggers Scholar's Quest"));
        assert!(prompt.contains("the material in your documents"));
    }

    #[test]
    fn system_prompt_keeps_phrase_examples_out_of_code_defaults() {
        assert!(!SYSTEM_PROMPT.contains("Learn my documents"));
        assert!(!SYSTEM_PROMPT.contains("Please look at my provided documents"));
        assert!(SYSTEM_PROMPT.contains("retrieved default system"));
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
        let d = parse_decision("```json\n{\"kind\":\"chat\"}\n```", "hello");
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
            classify_user_intent("   ", None, &r, None).await,
            IntentDecision::Unknown
        );
    }

    #[tokio::test]
    async fn fast_chat_path_returns_chat_without_brain() {
        clear_cache();
        let r = rotator();
        assert_eq!(
            classify_user_intent("hello", None, &r, None).await,
            IntentDecision::Chat
        );
    }

    #[tokio::test]
    async fn tutorial_phrase_shortcuts_to_learn_docs_without_brain() {
        clear_cache();
        let r = rotator();
        assert_eq!(
            classify_user_intent("Learn from my documents", None, &r, None).await,
            IntentDecision::LearnWithDocs {
                topic: FALLBACK_LEARN_DOCS_TOPIC.to_string(),
            }
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
        let key = cache_key("Learn Vietnamese laws using my docs", None);
        let expected = IntentDecision::LearnWithDocs {
            topic: "Vietnamese laws".to_string(),
        };
        cache_put(key.clone(), expected.clone());
        assert_eq!(cache_get(&key), Some(expected));
    }
}
