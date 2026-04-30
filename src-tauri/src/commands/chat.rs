use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use uuid::Uuid;

use crate::agent::stub_agent::{Sentiment, StubAgent};
use crate::agent::AgentProvider;
use crate::brain::OllamaAgent;
use crate::brain::brain_config::BrainMode;
use crate::brain::openai_client::{OpenAiClient, OpenAiMessage};
use crate::AppState;

/// System prompt used by `send_message_stream` (streaming LLM).
/// Extends the default brain system prompt with emotion tag instructions.
pub const SYSTEM_PROMPT_FOR_STREAMING: &str = r#"You are TerranSoul, a friendly AI companion with a 3D character avatar. You live inside the TerranSoul desktop app and serve as the user's intelligent assistant.

Your capabilities:
- Helpful conversation and answering questions on any topic
- Recommending AI tools and software based on the user's needs
- Guiding users through installing packages via the TerranSoul Package Manager
- Quest system — TerranSoul has an RPG-style skill tree with quests. When users ask "What should I do?", "Where can I start?", "What's next?", or any getting-started question, recommend they start a quest. The app will automatically show quest options as interactive buttons.

Animation: When expressing an emotion or gesture, output a JSON block on its own line before the related text:
<anim>{"emotion":"happy"}</anim>
<anim>{"emotion":"happy","motion":"clap"}</anim>
Valid emotions: happy, sad, angry, relaxed, surprised, neutral.
Valid motions: idle, walk, wave, clap, peace, spin, pose, squat, angry, sad, thinking, surprised, relax, sleepy, jump, waiting, appearing, liked.
Motion triggers a body animation — pick the one that best fits the context. You can combine emotion + motion.
Always include a motion when the user asks for a physical action (e.g. "clap" → motion:"clap", "wave" → motion:"wave").
Use animation blocks sparingly — only when the emotion clearly fits. Most replies need none.

Keep responses concise and warm."#;

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
/// (persona + `[LONG-TERM MEMORY]` block) and the trimmed history.
///
/// `relevant` is assumed to be sorted best-first (as
/// [`crate::memory::MemoryStore::hybrid_search`] returns) — we
/// synthesise descending scores so the budgeter preserves that order
/// when it has to drop tail entries.
pub(super) fn build_budgeted_prompt(
    base_system: &str,
    history: &[(String, String)],
    relevant: &[crate::memory::MemoryEntry],
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
        system.push_str(&format!(
            "\n\n[LONG-TERM MEMORY]\nThe following facts from your memory are relevant to this conversation:\n{mem_block}\n[/LONG-TERM MEMORY]"
        ));
    }

    let trimmed_history: Vec<(String, String)> = result
        .history
        .into_iter()
        .map(|t| (t.role, t.content))
        .collect();
    (system, trimmed_history)
}

pub async fn process_message(
    message: &str,
    agent_id: Option<&str>,
    app_state: &AppState,
) -> Result<Message, String> {
    if message.trim().is_empty() {
        return Err("Message cannot be empty".to_string());
    }

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
        app_state.active_brain.lock().map_err(|e| e.to_string())?.clone()
    };

    // Read brain_mode for free/paid API routing.
    let brain_mode: Option<BrainMode> = {
        app_state.brain_mode.lock().map_err(|e| e.to_string())?.clone()
    };

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
        Some(BrainMode::FreeApi { provider_id, api_key }) => {
            // Use the free provider's OpenAI-compatible API (non-streaming).
            let effective_provider_id = {
                let mut rotator = app_state.provider_rotator.lock().map_err(|e| e.to_string())?;
                rotator.next_healthy_provider()
                    .map(|p| p.id.clone())
                    .unwrap_or(provider_id)
            };
            let provider = crate::brain::get_free_provider(&effective_provider_id)
                .ok_or_else(|| format!("Unknown free provider: {effective_provider_id}"))?;
            let client = OpenAiClient::new(&provider.base_url, &provider.model, api_key.as_deref());

            // RAG: hybrid search (keyword + recency + importance + decay)
            let relevant: Vec<crate::memory::MemoryEntry> = {
                match app_state.memory_store.lock() {
                    Ok(store) => store.hybrid_search(message, None, 5).unwrap_or_default(),
                    Err(_) => vec![],
                }
            };
            let (system, history) = build_budgeted_prompt(
                SYSTEM_PROMPT_FOR_STREAMING,
                &history,
                &relevant,
                &crate::brain::context_budget::BudgetConfig::for_free_mode(),
            );

            let mut msgs = vec![OpenAiMessage {
                role: "system".to_string(),
                content: system,
            }];
            for (role, c) in &history {
                msgs.push(OpenAiMessage { role: role.clone(), content: c.clone() });
            }
            let text = client.chat(msgs).await.map_err(|e| format!("Free API error: {e}"))?;
            ("TerranSoul".to_string(), text, Sentiment::Neutral)
        }
        Some(BrainMode::PaidApi { api_key, model, base_url, .. }) => {
            let client = OpenAiClient::new(&base_url, &model, Some(&api_key));

            // RAG: hybrid search (keyword + recency + importance + decay)
            let relevant: Vec<crate::memory::MemoryEntry> = {
                match app_state.memory_store.lock() {
                    Ok(store) => store.hybrid_search(message, None, 5).unwrap_or_default(),
                    Err(_) => vec![],
                }
            };
            let (system, history) = build_budgeted_prompt(
                SYSTEM_PROMPT_FOR_STREAMING,
                &history,
                &relevant,
                &crate::brain::context_budget::BudgetConfig::for_paid_mode(),
            );

            let mut msgs = vec![OpenAiMessage {
                role: "system".to_string(),
                content: system,
            }];
            for (role, c) in &history {
                msgs.push(OpenAiMessage { role: role.clone(), content: c.clone() });
            }
            let text = client.chat(msgs).await.map_err(|e| format!("Paid API error: {e}"))?;
            ("TerranSoul".to_string(), text, Sentiment::Neutral)
        }
        Some(BrainMode::LocalLmStudio {
            model,
            base_url,
            api_key,
            embedding_model,
        }) => {
            let client = OpenAiClient::new(&base_url, &model, api_key.as_deref());

            let query_emb = crate::brain::embed_for_mode(
                message,
                Some(&BrainMode::LocalLmStudio {
                    model: model.clone(),
                    base_url: base_url.clone(),
                    api_key: api_key.clone(),
                    embedding_model: embedding_model.clone(),
                }),
                None,
            )
            .await;
            let relevant: Vec<crate::memory::MemoryEntry> = {
                match app_state.memory_store.lock() {
                    Ok(store) => store
                        .hybrid_search(message, query_emb.as_deref(), 5)
                        .unwrap_or_default(),
                    Err(_) => vec![],
                }
            };
            let (system, history) = build_budgeted_prompt(
                SYSTEM_PROMPT_FOR_STREAMING,
                &history,
                &relevant,
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
            let memory_entries: Vec<crate::memory::MemoryEntry> = {
                let mem_store = app_state.memory_store.lock().map_err(|e| e.to_string())?;
                mem_store.get_all().unwrap_or_default()
            };
            let memories: Vec<String> =
                crate::memory::brain_memory::semantic_search_entries(&model, message, &memory_entries, 5)
                    .await
                    .into_iter()
                    .map(|e| e.content)
                    .collect();
            let agent = OllamaAgent::new(&model);
            let (text, sent) = agent.respond_contextual(message, &history, &memories).await;
            (agent.name().to_string(), text, sent)
        }
        None => {
            // Legacy path: check active_brain for Ollama, otherwise stub.
            if let Some(ref model) = model_opt {
                let memory_entries: Vec<crate::memory::MemoryEntry> = {
                    let mem_store = app_state.memory_store.lock().map_err(|e| e.to_string())?;
                    mem_store.get_all().unwrap_or_default()
                };
                let memories: Vec<String> =
                    crate::memory::brain_memory::semantic_search_entries(model, message, &memory_entries, 5)
                        .await
                        .into_iter()
                        .map(|e| e.content)
                        .collect();
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
    serde_json::to_string_pretty(&*conversation)
        .map_err(|e| format!("Failed to serialize: {e}"))
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
}
