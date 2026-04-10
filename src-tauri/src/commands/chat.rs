use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use uuid::Uuid;

use crate::agent::stub_agent::StubAgent;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub role: String,
    pub content: String,
    pub agent_name: Option<String>,
    pub timestamp: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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
        timestamp: now_ms(),
    };

    {
        let mut conv = app_state.conversation.lock().map_err(|e| e.to_string())?;
        conv.push(user_msg);
    }

    let agent = StubAgent::new(agent_id.unwrap_or("stub"));
    let (content, _sentiment) = agent.respond(message).await;

    let response = Message {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content,
        agent_name: Some(agent.name().to_string()),
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
    use std::sync::Mutex;

    fn make_state() -> AppState {
        AppState {
            conversation: Mutex::new(Vec::new()),
            vrm_path: Mutex::new(None),
        }
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
    async fn message_timestamps_are_ordered() {
        let state = make_state();
        let _ = process_message("hello", None, &state).await;
        let conv = fetch_conversation(&state);
        assert!(conv[0].timestamp <= conv[1].timestamp);
    }
}
