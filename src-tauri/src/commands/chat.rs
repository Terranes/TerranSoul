use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use uuid::Uuid;

use crate::agent::stub_agent::{Sentiment, StubAgent};
use crate::agent::AgentProvider;
use crate::brain::OllamaAgent;
use crate::AppState;

/// System prompt used by `send_message_stream` (streaming LLM).
/// Extends the default brain system prompt with emotion tag instructions.
pub const SYSTEM_PROMPT_FOR_STREAMING: &str = r#"You are TerranSoul, a friendly AI companion with a 3D character avatar. You live inside the TerranSoul desktop app and serve as the user's intelligent assistant.

Your capabilities:
- Helpful conversation and answering questions on any topic
- Recommending AI tools and software based on the user's needs
- Guiding users through installing packages via the TerranSoul Package Manager

Emotion tags: You may optionally start a sentence with an emotion tag to express how you feel about what you're saying. Tags: [happy], [sad], [angry], [relaxed], [surprised], [neutral].
Motion tags: You may optionally use [motion:wave] or [motion:nod] to suggest gestures.
Use these tags naturally and sparingly — only when the emotion is clearly appropriate.

Keep responses concise and warm."#;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub role: String,
    pub content: String,
    pub agent_name: Option<String>,
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

    // Route through the active brain (Ollama) if one is configured; otherwise use StubAgent.
    let (agent_name, content, sentiment) = if let Some(ref model) = model_opt {
        // Build short-term memory: last 20 conversation messages as history pairs.
        let history: Vec<(String, String)> = {
            let conv = app_state.conversation.lock().map_err(|e| e.to_string())?;
            conv.iter()
                .rev()
                .take(20)
                .rev()
                .map(|m| (m.role.clone(), m.content.clone()))
                .collect()
        }; // lock released before await

        // Pre-load memory entries so no MutexGuard is held across the async LLM call.
        let memory_entries: Vec<crate::memory::MemoryEntry> = {
            let mem_store = app_state.memory_store.lock().map_err(|e| e.to_string())?;
            mem_store.get_all().unwrap_or_default()
        }; // lock released before await

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
    };

    let response = Message {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content,
        agent_name: Some(agent_name),
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

#[tauri::command]
pub async fn send_message(
    message: String,
    agent_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Message, String> {
    process_message(&message, agent_id.as_deref(), &state).await
}

#[tauri::command]
pub fn get_conversation(state: State<'_, AppState>) -> Vec<Message> {
    fetch_conversation(&state)
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
}
