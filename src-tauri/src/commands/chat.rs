use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use uuid::Uuid;

use crate::agent::stub_agent::{Sentiment, StubAgent};
use crate::agent::AgentProvider;
use crate::brain::brain_config::BrainMode;
use crate::brain::openai_client::{OpenAiClient, OpenAiMessage};
use crate::brain::OllamaAgent;
use crate::AppState;

/// System prompt used by `send_message_stream` (streaming LLM).
pub const SYSTEM_PROMPT_FOR_STREAMING: &str = r#"You are TerranSoul, a friendly AI companion. Keep replies short — 1-3 sentences for casual chat, longer only when the user asks a detailed question.

Animation (use sparingly, only when emotion clearly fits):
<anim>{"emotion":"happy"}</anim>
<anim>{"emotion":"happy","motion":"wave"}</anim>
Emotions: happy, sad, angry, relaxed, surprised, neutral.
Motions: idle, walk, wave, clap, peace, spin, pose, angry, sad, thinking, surprised, relax, sleepy, jump, waiting, appearing, liked.

Quest system: When users ask "What should I do?" or "What's next?", suggest starting a quest.

Be concise, warm, and natural."#;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub role: String,
    pub content: String,
    pub agent_name: Option<String>,
    /// The agent profile ID that produced this message. `None` for
    /// messages created before per-agent threading was added.
    pub agent_id: Option<String>,
    pub sentiment: Option<String>,
    pub timestamp: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn sentiment_str(s: &Sentiment) -> &'static str {
    match s {
        Sentiment::Happy => "happy",
        Sentiment::Sad => "sad",
        Sentiment::Neutral => "neutral",
    }
}

/// Apply the Phase-27 [`crate::brain::context_budget`] budgeter to a
/// chat prompt: persona is kept verbatim, retrieved memories are
/// trimmed by score, and old history turns are dropped to stay within
/// the per-mode token budget. Returns the assembled system prompt
/// (persona + retrieved context pack) and the trimmed history.
///
/// `relevant` is assumed to be sorted best-first (as
/// [`crate::memory::MemoryStore::hybrid_search`] returns) — we
/// synthesise descending scores so the budgeter preserves that order
/// when it has to drop tail entries.
pub(super) fn build_budgeted_prompt(
    base_system: &str,
    history: &[(String, String)],
    relevant: &[crate::memory::MemoryEntry],
    judgment_block: &str,
    config: &crate::brain::context_budget::BudgetConfig,
) -> (String, Vec<(String, String)>) {
    use crate::brain::context_budget::{fit, BudgetInputs, HistoryTurn, RetrievedChunk};

    let total = relevant.len();
    let inputs = BudgetInputs {
        persona: base_system.to_string(),
        history: history
            .iter()
            .map(|(r, c)| HistoryTurn {
                role: r.clone(),
                content: c.clone(),
            })
            .collect(),
        retrieval: relevant
            .iter()
            .enumerate()
            .map(|(i, e)| RetrievedChunk {
                content: format!("- [{}] {}", e.tier.as_str(), e.content),
                // hybrid_search returns best-first; synthesise a
                // descending score so the budgeter keeps that order
                // when it has to drop tail entries.
                score: (total - i) as f64,
            })
            .collect(),
        tools: String::new(),
    };
    let result = fit(&inputs, config);

    let mut system = result.persona;
    if !result.retrieval.is_empty() {
        let mem_block: String = result
            .retrieval
            .iter()
            .map(|c| c.content.clone())
            .collect::<Vec<_>>()
            .join("\n");
        system.push_str(&crate::memory::format_retrieved_context_pack(&mem_block));
    }

    if !judgment_block.is_empty() {
        system.push_str(judgment_block);
    }

    let trimmed_history: Vec<(String, String)> = result
        .history
        .into_iter()
        .map(|t| (t.role, t.content))
        .collect();
    (system, trimmed_history)
}

/// Retrieve memories for the chat prompt RAG block, exercising the full
/// `docs/brain-advanced-design.md` retrieval pipeline:
/// embed → `hybrid_search_rrf` (6-signal hybrid + RRF fusion) → optional
/// 1–2 hop KG cascade expansion (`memory::cascade::cascade_expand`, gated
/// on `AppSettings.enable_kg_boost`, BENCH-KG-1) → optional
/// LLM-as-judge cross-encoder rerank with threshold (LCM-8) → reinforcement
/// telemetry.
///
/// Test coverage: `retrieve_prompt_memories_uses_rrf_pipeline_without_brain`
/// asserts the RRF stage runs and surfaces seeded memories when no brain is
/// configured. `retrieve_prompt_memories_kg_boost_promotes_neighbours`
/// asserts the cascade stage promotes a graph-adjacent memory when
/// `enable_kg_boost` is on. The reranker stage is exercised end-to-end by
/// the bench harness `scripts/locomo-mteb.mjs` system `rrf_rerank` (LCM-8)
/// — that is the canonical chat-system rerank verification because mocking
/// Ollama inside a unit test would not validate prompt-shape compatibility.
pub(crate) async fn retrieve_prompt_memories(
    app_state: &AppState,
    query: &str,
    brain_mode: Option<&BrainMode>,
    active_brain: Option<&str>,
    limit: usize,
) -> Vec<crate::memory::MemoryEntry> {
    let query_emb = crate::brain::embed_for_mode(query, brain_mode, active_brain).await;
    let (rerank_threshold, kg_boost) = app_state
        .app_settings
        .lock()
        .map(|settings| {
            (
                settings
                    .relevance_threshold
                    .max(crate::settings::DEFAULT_RERANK_THRESHOLD),
                settings.enable_kg_boost,
            )
        })
        .unwrap_or((crate::settings::DEFAULT_RERANK_THRESHOLD, false));
    let should_rerank = active_brain.is_some();
    let recall_limit = if should_rerank {
        limit.clamp(20, 50)
    } else {
        limit
    };

    let candidates: Vec<crate::memory::MemoryEntry> = {
        match app_state.memory_store.lock() {
            Ok(store) => {
                let seeds = store
                    .hybrid_search_rrf(query, query_emb.as_deref(), recall_limit)
                    .unwrap_or_default();
                if seeds.is_empty() || !kg_boost {
                    seeds
                } else {
                    // BENCH-KG-1: expand top-K via 1-2 hop BFS over
                    // `memory_edges` so graph-adjacent facts can be promoted
                    // ahead of weaker RRF tail entries. `cascade_expand`
                    // returns (id, score) sorted descending; we materialise
                    // the entries (skipping unknown ids gracefully) and cap
                    // at `recall_limit` so the reranker pool stays bounded.
                    expand_seeds_via_kg(&store, &seeds, recall_limit)
                }
            }
            Err(_) => Vec::new(),
        }
    };

    if candidates.is_empty() {
        return Vec::new();
    }

    let Some(model) = active_brain else {
        return candidates.into_iter().take(limit).collect();
    };

    let agent = OllamaAgent::new(model);
    let mut scores = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        scores.push(agent.rerank_score(query, &candidate.content).await);
    }

    let reranked = crate::memory::reranker::rerank_candidates_with_threshold(
        candidates,
        &scores,
        limit,
        rerank_threshold,
    );

    // Record reinforcement for entries that survived reranking (43.4).
    if let Ok(store) = app_state.memory_store.lock() {
        let session_id = format!("chat_{}", crate::memory::store::now_ms());
        for (idx, entry) in reranked.iter().enumerate() {
            let _ = store.record_reinforcement(entry.id, &session_id, idx as i64);
        }
    }

    reranked
}

/// BENCH-KG-1 helper: expand RRF seeds through 1-2 hop BFS over
/// `memory_edges`, then materialise the (id, cascade_score) result back
/// into a `Vec<MemoryEntry>` capped at `limit`. Seeds keep their original
/// RRF order at the top; KG-promoted neighbours follow, sorted by cascade
/// score descending. Unknown ids (edge target deleted between RRF and
/// materialisation) are skipped gracefully.
fn expand_seeds_via_kg(
    store: &crate::memory::MemoryStore,
    seeds: &[crate::memory::MemoryEntry],
    limit: usize,
) -> Vec<crate::memory::MemoryEntry> {
    use std::collections::HashSet;

    // Seed scores: linearly decreasing from 1.0 so cascade decay is
    // computed relative to RRF rank (top seed = 1.0, bottom seed = ~0.0).
    let total = seeds.len() as f64;
    let seed_scores: Vec<(i64, f64)> = seeds
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let score = if total > 0.0 {
                1.0 - (i as f64 / total)
            } else {
                0.0
            };
            (e.id, score)
        })
        .collect();

    let expanded = match crate::memory::cascade::cascade_expand(&store.conn, &seed_scores, None) {
        Ok(v) => v,
        Err(_) => return seeds.to_vec(),
    };
    if expanded.len() == seeds.len() {
        // No neighbours found — return seeds unchanged.
        return seeds.to_vec();
    }

    let seed_ids: HashSet<i64> = seeds.iter().map(|e| e.id).collect();
    let mut out: Vec<crate::memory::MemoryEntry> = Vec::with_capacity(limit);
    // 1. Keep all RRF seeds in their original order at the top.
    for entry in seeds {
        if out.len() >= limit {
            break;
        }
        out.push(entry.clone());
    }
    // 2. Append KG-promoted neighbours by cascade score.
    for (id, _score) in expanded.iter() {
        if out.len() >= limit {
            break;
        }
        if seed_ids.contains(id) {
            continue;
        }
        if let Ok(entry) = store.get_by_id(*id) {
            out.push(entry);
        }
    }
    out
}

fn retrieve_local_ollama_keyword_memories(
    app_state: &AppState,
    query: &str,
) -> Result<Vec<String>, String> {
    let threshold = app_state
        .app_settings
        .lock()
        .map(|settings| settings.relevance_threshold)
        .unwrap_or(crate::settings::DEFAULT_RELEVANCE_THRESHOLD);

    let store = app_state.memory_store.lock().map_err(|e| e.to_string())?;
    let memory_count = store.count();
    if crate::commands::streaming::should_skip_rag(query, memory_count) {
        return Ok(Vec::new());
    }

    Ok(store
        .hybrid_search_with_threshold(query, None, 5, threshold)
        .unwrap_or_default()
        .into_iter()
        .map(|entry| entry.content)
        .collect())
}

/// Retrieve relevant judgment rules for the current message and format
/// them as a prompt injection block.
fn retrieve_judgment_block(app_state: &AppState, query: &str) -> String {
    let judgments = match app_state.memory_store.lock() {
        Ok(store) => crate::memory::judgment::apply_judgments(&store, query, 5),
        Err(_) => Vec::new(),
    };
    crate::memory::judgment::format_judgment_block(&judgments)
}

pub async fn process_message(
    message: &str,
    agent_id: Option<&str>,
    app_state: &AppState,
) -> Result<Message, String> {
    if message.trim().is_empty() {
        return Err("Message cannot be empty".to_string());
    }

    app_state.mark_chat_activity_now();

    let user_msg = Message {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        content: message.to_string(),
        agent_name: None,
        agent_id: None,
        sentiment: None,
        timestamp: now_ms(),
    };

    {
        let mut conv = app_state.conversation.lock().map_err(|e| e.to_string())?;
        conv.push(user_msg);
    }

    // Clone model name before any await so the MutexGuard is not held across .await.
    let model_opt: Option<String> = {
        app_state
            .active_brain
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
    };

    // Read brain_mode for free/paid API routing.
    let brain_mode: Option<BrainMode> = {
        app_state
            .brain_mode
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
    };
    let brain_mode_for_retrieval = brain_mode.clone();

    // Build conversation history (needed for all LLM paths).
    let history: Vec<(String, String)> = {
        let conv = app_state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .rev()
            .take(20)
            .rev()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    };

    // Route through the configured brain mode, then legacy active_brain, then stub.
    let (agent_name, content, sentiment) = match brain_mode {
        Some(BrainMode::FreeApi {
            provider_id,
            api_key,
            model,
        }) => {
            // Use the free provider's OpenAI-compatible API (non-streaming).
            let effective_provider_id = {
                let mut rotator = app_state
                    .provider_rotator
                    .lock()
                    .map_err(|e| e.to_string())?;
                rotator
                    .next_healthy_provider()
                    .map(|p| p.id.clone())
                    .unwrap_or_else(|| provider_id.clone())
            };
            let provider = crate::brain::get_free_provider(&effective_provider_id)
                .ok_or_else(|| format!("Unknown free provider: {effective_provider_id}"))?;
            let chat_model = model
                .as_deref()
                .filter(|_| effective_provider_id == provider_id.as_str())
                .unwrap_or(&provider.model);
            let client = OpenAiClient::new(&provider.base_url, chat_model, api_key.as_deref());

            let relevant = retrieve_prompt_memories(
                app_state,
                message,
                brain_mode_for_retrieval.as_ref(),
                model_opt.as_deref(),
                5,
            )
            .await;
            let jblock = retrieve_judgment_block(app_state, message);
            let (system, history) = build_budgeted_prompt(
                SYSTEM_PROMPT_FOR_STREAMING,
                &history,
                &relevant,
                &jblock,
                &crate::brain::context_budget::BudgetConfig::for_free_mode(),
            );

            let mut msgs = vec![OpenAiMessage {
                role: "system".to_string(),
                content: system,
            }];
            for (role, c) in &history {
                msgs.push(OpenAiMessage {
                    role: role.clone(),
                    content: c.clone(),
                });
            }
            let text = client
                .chat(msgs)
                .await
                .map_err(|e| format!("Free API error: {e}"))?;
            ("TerranSoul".to_string(), text, Sentiment::Neutral)
        }
        Some(BrainMode::PaidApi {
            api_key,
            model,
            base_url,
            ..
        }) => {
            let client = OpenAiClient::new(&base_url, &model, Some(&api_key));

            let relevant = retrieve_prompt_memories(
                app_state,
                message,
                brain_mode_for_retrieval.as_ref(),
                model_opt.as_deref(),
                5,
            )
            .await;
            let jblock = retrieve_judgment_block(app_state, message);
            let (system, history) = build_budgeted_prompt(
                SYSTEM_PROMPT_FOR_STREAMING,
                &history,
                &relevant,
                &jblock,
                &crate::brain::context_budget::BudgetConfig::for_paid_mode(),
            );

            let mut msgs = vec![OpenAiMessage {
                role: "system".to_string(),
                content: system,
            }];
            for (role, c) in &history {
                msgs.push(OpenAiMessage {
                    role: role.clone(),
                    content: c.clone(),
                });
            }
            let text = client
                .chat(msgs)
                .await
                .map_err(|e| format!("Paid API error: {e}"))?;
            ("TerranSoul".to_string(), text, Sentiment::Neutral)
        }
        Some(BrainMode::LocalLmStudio {
            model,
            base_url,
            api_key,
            embedding_model: _,
        }) => {
            let client = OpenAiClient::new(&base_url, &model, api_key.as_deref());

            let relevant = retrieve_prompt_memories(
                app_state,
                message,
                brain_mode_for_retrieval.as_ref(),
                model_opt.as_deref(),
                5,
            )
            .await;
            let jblock = retrieve_judgment_block(app_state, message);
            let (system, history) = build_budgeted_prompt(
                SYSTEM_PROMPT_FOR_STREAMING,
                &history,
                &relevant,
                &jblock,
                &crate::brain::context_budget::BudgetConfig::for_local_mode(),
            );

            let mut msgs = vec![OpenAiMessage {
                role: "system".to_string(),
                content: system,
            }];
            for (role, c) in &history {
                msgs.push(OpenAiMessage {
                    role: role.clone(),
                    content: c.clone(),
                });
            }
            let text = client
                .chat(msgs)
                .await
                .map_err(|e| format!("LM Studio error: {e}"))?;
            (model.clone(), text, Sentiment::Neutral)
        }
        Some(BrainMode::LocalOllama { model }) => {
            let memories = retrieve_local_ollama_keyword_memories(app_state, message)?;
            let agent = OllamaAgent::new(&model);
            let (text, sent) = agent.respond_contextual(message, &history, &memories).await;
            (agent.name().to_string(), text, sent)
        }
        None => {
            // Legacy path: check active_brain for Ollama, otherwise stub.
            if let Some(ref model) = model_opt {
                let memories = retrieve_local_ollama_keyword_memories(app_state, message)?;
                let agent = OllamaAgent::new(model);
                let (text, sent) = agent.respond_contextual(message, &history, &memories).await;
                (agent.name().to_string(), text, sent)
            } else {
                let agent = StubAgent::new(agent_id.unwrap_or("stub"));
                let name = agent.name().to_string();
                let (text, sent) = agent.respond(message).await;
                (name, text, sent)
            }
        }
    };

    let response = Message {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content,
        agent_name: Some(agent_name),
        agent_id: None,
        sentiment: Some(sentiment_str(&sentiment).to_string()),
        timestamp: now_ms(),
    };

    {
        let mut conv = app_state.conversation.lock().map_err(|e| e.to_string())?;
        conv.push(response.clone());
    }

    Ok(response)
}

pub fn fetch_conversation(app_state: &AppState) -> Vec<Message> {
    app_state
        .conversation
        .lock()
        .map(|c| c.clone())
        .unwrap_or_default()
}

#[tauri::command(rename_all = "camelCase")]
pub async fn send_message(
    message: String,
    agent_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Message, String> {
    process_message(&message, agent_id.as_deref(), &state).await
}

#[tauri::command]
pub async fn get_conversation(state: State<'_, AppState>) -> Result<Vec<Message>, String> {
    Ok(fetch_conversation(&state))
}

/// Export the full conversation history as a pretty-printed JSON string.
///
/// Returns the serialised `Vec<Message>` for the frontend to save via a file
/// dialog or download link.
#[tauri::command]
pub async fn export_chat_log(state: State<'_, AppState>) -> Result<String, String> {
    let conversation = state.conversation.lock().map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&*conversation).map_err(|e| format!("Failed to serialize: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> AppState {
        AppState::for_test()
    }
    #[tokio::test]
    async fn send_message_success() {
        let state = make_state();
        let result = process_message("hello", None, &state).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.role, "assistant");
        assert!(response.agent_name.is_some());
        assert_eq!(response.agent_name.as_deref(), Some("TerranSoul"));
    }

    #[tokio::test]
    async fn send_message_empty_input_error() {
        let state = make_state();
        let result = process_message("", None, &state).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Message cannot be empty");
    }

    #[tokio::test]
    async fn send_message_whitespace_only_error() {
        let state = make_state();
        let result = process_message("   ", None, &state).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Message cannot be empty");
    }

    #[tokio::test]
    async fn send_message_adds_both_user_and_assistant_messages() {
        let state = make_state();
        let _ = process_message("hello", None, &state).await;
        let conv = fetch_conversation(&state);
        assert_eq!(conv.len(), 2);
        assert_eq!(conv[0].role, "user");
        assert_eq!(conv[0].content, "hello");
        assert_eq!(conv[1].role, "assistant");
    }

    #[test]
    fn get_conversation_empty() {
        let state = make_state();
        let conv = fetch_conversation(&state);
        assert!(conv.is_empty());
    }

    #[tokio::test]
    async fn get_conversation_ordering() {
        let state = make_state();
        let _ = process_message("first", None, &state).await;
        let _ = process_message("second", None, &state).await;
        let conv = fetch_conversation(&state);
        assert_eq!(conv.len(), 4); // 2 user + 2 assistant
        assert_eq!(conv[0].content, "first");
        assert_eq!(conv[0].role, "user");
        assert_eq!(conv[1].role, "assistant");
        assert_eq!(conv[2].content, "second");
        assert_eq!(conv[2].role, "user");
        assert_eq!(conv[3].role, "assistant");
    }

    #[tokio::test]
    async fn send_message_with_custom_agent_id() {
        let state = make_state();
        let result = process_message("hello", Some("custom"), &state).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.agent_name.as_deref(), Some("custom"));
    }

    #[tokio::test]
    async fn send_message_returns_happy_sentiment_for_hello() {
        let state = make_state();
        let result = process_message("hello", None, &state).await;
        let response = result.unwrap();
        assert_eq!(response.sentiment.as_deref(), Some("happy"));
    }

    #[tokio::test]
    async fn send_message_returns_sad_sentiment() {
        let state = make_state();
        let result = process_message("I am sad today", None, &state).await;
        let response = result.unwrap();
        assert_eq!(response.sentiment.as_deref(), Some("sad"));
    }

    #[tokio::test]
    async fn send_message_returns_neutral_sentiment() {
        let state = make_state();
        let result = process_message("tell me about weather", None, &state).await;
        let response = result.unwrap();
        assert_eq!(response.sentiment.as_deref(), Some("neutral"));
    }

    #[tokio::test]
    async fn user_message_has_no_sentiment() {
        let state = make_state();
        let _ = process_message("hello", None, &state).await;
        let conv = fetch_conversation(&state);
        assert!(conv[0].sentiment.is_none());
        assert!(conv[1].sentiment.is_some());
    }

    #[tokio::test]
    async fn message_timestamps_are_ordered() {
        let state = make_state();
        let _ = process_message("hello", None, &state).await;
        let conv = fetch_conversation(&state);
        assert!(conv[0].timestamp <= conv[1].timestamp);
    }

    // ── export_chat_log ─────────────────────────────────────────────────────

    #[test]
    fn export_chat_log_empty_returns_valid_json() {
        let state = make_state();
        let json = {
            let conv = state.conversation.lock().unwrap();
            serde_json::to_string_pretty(&*conv).unwrap()
        };
        let parsed: Vec<Message> = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_empty());
    }

    #[tokio::test]
    async fn export_chat_log_returns_all_messages() {
        let state = make_state();
        let _ = process_message("first", None, &state).await;
        let _ = process_message("second", None, &state).await;
        let json = {
            let conv = state.conversation.lock().unwrap();
            serde_json::to_string_pretty(&*conv).unwrap()
        };
        let parsed: Vec<Message> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 4); // 2 user + 2 assistant
        assert_eq!(parsed[0].content, "first");
        assert_eq!(parsed[0].role, "user");
        assert_eq!(parsed[2].content, "second");
    }

    #[tokio::test]
    async fn export_chat_log_preserves_sentiment() {
        let state = make_state();
        let _ = process_message("hello", None, &state).await;
        let json = {
            let conv = state.conversation.lock().unwrap();
            serde_json::to_string_pretty(&*conv).unwrap()
        };
        let parsed: Vec<Message> = serde_json::from_str(&json).unwrap();
        // User message has no sentiment, assistant does
        assert!(parsed[0].sentiment.is_none());
        assert!(parsed[1].sentiment.is_some());
    }

    /// Asserts the chat-prompt RAG path actually exercises the
    /// `docs/brain-advanced-design.md` retrieval pipeline (RRF stage).
    /// Without an active brain the function must return RRF top-k from
    /// `hybrid_search_rrf` rather than falling back to lexical-only
    /// `hybrid_search` or returning empty. This guards the chat surface
    /// from silently regressing past LCM-8.
    #[tokio::test]
    async fn retrieve_prompt_memories_uses_rrf_pipeline_without_brain() {
        let state = make_state();
        {
            let store = state.memory_store.lock().unwrap();
            store
                .add(crate::memory::NewMemory {
                    content: "User prefers Python coding examples over Rust for tutorials."
                        .to_string(),
                    tags: "preference,domain:programming".to_string(),
                    importance: 4,
                    ..Default::default()
                })
                .unwrap();
            store
                .add(crate::memory::NewMemory {
                    content: "User asked for help debugging async Tokio futures last week."
                        .to_string(),
                    tags: "context,domain:rust".to_string(),
                    importance: 3,
                    ..Default::default()
                })
                .unwrap();
        }

        let results =
            super::retrieve_prompt_memories(&state, "Show me a Python example", None, None, 5)
                .await;
        assert!(
            !results.is_empty(),
            "RRF stage must surface seeded memories on a clear lexical+semantic match"
        );
        assert!(
            results.iter().any(|m| m.content.contains("Python")),
            "Top-k should include the Python preference memory; got {:?}",
            results.iter().map(|m| &m.content).collect::<Vec<_>>()
        );
    }

    /// BENCH-KG-1: when `enable_kg_boost` is on, the cascade stage must
    /// promote a memory that is graph-adjacent to an RRF hit but does
    /// NOT itself match the query lexically. This is the design-doc
    /// promise of `memory_edges` traversal — a query about "Python" can
    /// surface a related "favourite editor" memory if the user explicitly
    /// linked them with a `related_to` edge, even though the editor
    /// memory has no Python tokens.
    #[tokio::test]
    async fn retrieve_prompt_memories_kg_boost_promotes_neighbours() {
        let state = make_state();
        // Flip the KG-boost flag on this test's settings — production
        // default is `false` so a chat turn with no edges configured
        // pays no cascade cost.
        state.app_settings.lock().unwrap().enable_kg_boost = true;

        let (seed_id, neighbour_id) = {
            let store = state.memory_store.lock().unwrap();
            let seed = store
                .add(crate::memory::NewMemory {
                    content: "User prefers Python coding examples over Rust for tutorials."
                        .to_string(),
                    tags: "preference,domain:programming".to_string(),
                    importance: 4,
                    ..Default::default()
                })
                .unwrap()
                .id;
            // Neighbour deliberately contains NO query tokens — it must
            // only be reachable via the graph edge.
            let neighbour = store
                .add(crate::memory::NewMemory {
                    content: "Favourite editor is Helix with the catppuccin theme."
                        .to_string(),
                    tags: "preference,domain:tooling".to_string(),
                    importance: 4,
                    ..Default::default()
                })
                .unwrap()
                .id;
            store
                .add_edge(crate::memory::NewMemoryEdge {
                    src_id: seed,
                    dst_id: neighbour,
                    rel_type: "related_to".to_string(),
                    confidence: 0.9,
                    source: crate::memory::EdgeSource::User,
                    valid_from: None,
                    valid_to: None,
                    edge_source: None,
                })
                .unwrap();
            (seed, neighbour)
        };

        let results =
            super::retrieve_prompt_memories(&state, "Show me a Python example", None, None, 5)
                .await;

        let ids: Vec<i64> = results.iter().map(|m| m.id).collect();
        assert!(
            ids.contains(&seed_id),
            "Seed (Python memory) must be retrieved by RRF; got {ids:?}"
        );
        assert!(
            ids.contains(&neighbour_id),
            "KG cascade must surface the related-edge neighbour even though it has no query tokens; got {ids:?}"
        );
    }

    /// BENCH-KG-1 negative: when `enable_kg_boost` is off (production
    /// default), an edge-only neighbour must NOT appear in the prompt.
    /// Guards against accidentally enabling cascade for all users.
    #[tokio::test]
    async fn retrieve_prompt_memories_kg_boost_disabled_by_default() {
        let state = make_state();
        assert!(
            !state.app_settings.lock().unwrap().enable_kg_boost,
            "production default for enable_kg_boost must be false"
        );

        let (seed_id, neighbour_id) = {
            let store = state.memory_store.lock().unwrap();
            let seed = store
                .add(crate::memory::NewMemory {
                    content: "User prefers Python coding examples over Rust for tutorials."
                        .to_string(),
                    tags: "preference,domain:programming".to_string(),
                    importance: 4,
                    ..Default::default()
                })
                .unwrap()
                .id;
            let neighbour = store
                .add(crate::memory::NewMemory {
                    content: "Favourite editor is Helix with the catppuccin theme."
                        .to_string(),
                    tags: "preference,domain:tooling".to_string(),
                    importance: 4,
                    ..Default::default()
                })
                .unwrap()
                .id;
            store
                .add_edge(crate::memory::NewMemoryEdge {
                    src_id: seed,
                    dst_id: neighbour,
                    rel_type: "related_to".to_string(),
                    confidence: 0.9,
                    source: crate::memory::EdgeSource::User,
                    valid_from: None,
                    valid_to: None,
                    edge_source: None,
                })
                .unwrap();
            (seed, neighbour)
        };

        let results =
            super::retrieve_prompt_memories(&state, "Show me a Python example", None, None, 5)
                .await;

        let ids: Vec<i64> = results.iter().map(|m| m.id).collect();
        assert!(
            ids.contains(&seed_id),
            "Seed must still be retrieved by RRF when KG boost is off"
        );
        assert!(
            !ids.contains(&neighbour_id),
            "Edge-only neighbour must NOT appear when enable_kg_boost is off; got {ids:?}"
        );
    }
}
