use crate::AppState;
use serde::Serialize;
use tauri::State;

/// Frontend-facing agent message.
#[derive(Debug, Clone, Serialize)]
pub struct AgentMessageInfo {
    pub id: String,
    pub sender: String,
    pub topic: String,
    pub payload: serde_json::Value,
    pub timestamp: u64,
}

impl From<&crate::messaging::AgentMessage> for AgentMessageInfo {
    fn from(m: &crate::messaging::AgentMessage) -> Self {
        Self {
            id: m.id.clone(),
            sender: m.sender.clone(),
            topic: m.topic.clone(),
            payload: m.payload.clone(),
            timestamp: m.timestamp,
        }
    }
}

/// Frontend-facing subscription info.
#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionInfo {
    pub agent_name: String,
    pub topic: String,
}

/// Publish a message from one agent to a topic.
#[tauri::command]
pub async fn publish_agent_message(
    sender: String,
    topic: String,
    payload: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<AgentMessageInfo, String> {
    let mut bus = state.message_bus.lock().await;
    let msg = bus.publish(&sender, &topic, payload);
    Ok(AgentMessageInfo::from(&msg))
}

/// Subscribe an agent to a topic.
#[tauri::command(rename_all = "camelCase")]
pub async fn subscribe_agent_topic(
    agent_name: String,
    topic: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut bus = state.message_bus.lock().await;
    bus.subscribe(&agent_name, &topic);
    Ok(())
}

/// Unsubscribe an agent from a topic.
#[tauri::command(rename_all = "camelCase")]
pub async fn unsubscribe_agent_topic(
    agent_name: String,
    topic: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut bus = state.message_bus.lock().await;
    bus.unsubscribe(&agent_name, &topic);
    Ok(())
}

/// Get and drain all pending messages for an agent.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_agent_messages(
    agent_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<AgentMessageInfo>, String> {
    let mut bus = state.message_bus.lock().await;
    let msgs = bus.get_messages(&agent_name);
    Ok(msgs.iter().map(AgentMessageInfo::from).collect())
}

/// List all topics an agent is subscribed to.
#[tauri::command(rename_all = "camelCase")]
pub async fn list_agent_subscriptions(
    agent_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let bus = state.message_bus.lock().await;
    Ok(bus.subscriptions_for(&agent_name))
}
