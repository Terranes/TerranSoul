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
pub async fn list_agents() -> Result<Vec<Agent>, String> {
    Ok(vec![Agent {
        id: "stub".to_string(),
        name: "TerranSoul Stub".to_string(),
        description: "A built-in stub agent for local testing.".to_string(),
        status: "running".to_string(),
        capabilities: vec!["chat".to_string(), "sentiment".to_string()],
    }])
}

#[tauri::command]
pub async fn get_agent_status(id: String) -> Result<Option<Agent>, String> {
    if id == "stub" {
        Ok(Some(Agent {
            id: "stub".to_string(),
            name: "TerranSoul Stub".to_string(),
            description: "A built-in stub agent for local testing.".to_string(),
            status: "running".to_string(),
            capabilities: vec!["chat".to_string(), "sentiment".to_string()],
        }))
    } else {
        Ok(None)
    }
}
