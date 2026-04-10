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

#[tauri::command]
pub async fn send_message(
    message: String,
    agent_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Message, String> {
    let user_msg = Message {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        content: message.clone(),
        agent_name: None,
        timestamp: now_ms(),
    };

    {
        let mut conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.push(user_msg);
    }

    let agent = StubAgent::new(agent_id.as_deref().unwrap_or("stub"));
    let (content, _sentiment) = agent.respond(&message).await;

    let response = Message {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content,
        agent_name: Some(agent.name().to_string()),
        timestamp: now_ms(),
    };

    {
        let mut conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.push(response.clone());
    }

    Ok(response)
}

#[tauri::command]
pub fn get_conversation(state: State<'_, AppState>) -> Vec<Message> {
    state
        .conversation
        .lock()
        .map(|c| c.clone())
        .unwrap_or_default()
}
