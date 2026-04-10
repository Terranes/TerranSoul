use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub capabilities: Vec<String>,
}

#[tauri::command]
pub fn list_agents() -> Vec<Agent> {
    vec![Agent {
        id: "stub".to_string(),
        name: "TerranSoul Stub".to_string(),
        description: "A built-in stub agent for local testing.".to_string(),
        status: "running".to_string(),
        capabilities: vec!["chat".to_string(), "sentiment".to_string()],
    }]
}

#[tauri::command]
pub fn get_agent_status(id: String) -> Option<Agent> {
    if id == "stub" {
        Some(Agent {
            id: "stub".to_string(),
            name: "TerranSoul Stub".to_string(),
            description: "A built-in stub agent for local testing.".to_string(),
            status: "running".to_string(),
            capabilities: vec!["chat".to_string(), "sentiment".to_string()],
        })
    } else {
        None
    }
}
