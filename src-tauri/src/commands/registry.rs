use serde::Serialize;
use tauri::State;
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct AgentSearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub homepage: Option<String>,
    /// Kind of agent.
    ///
    /// - `"package"` — installable via `install_agent`.
    /// - `"local_llm"` — a local Ollama model installed via
    ///   `pull_ollama_model` + activated via `set_active_brain`.
    ///
    /// Defaults to `"package"` for backwards-compatibility.
    #[serde(default)]
    pub kind: String,
    /// Optional Ollama model tag for `kind = "local_llm"` agents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_tag: Option<String>,
    /// Optional minimum RAM (MB) for local-LLM agents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_ram_mb: Option<u64>,
    /// True if this is the top recommendation for this hardware tier.
    #[serde(default)]
    pub is_top_pick: bool,
    /// True if this is a cloud-routed model (Ollama Cloud).
    #[serde(default)]
    pub is_cloud: bool,
}

#[tauri::command]
pub async fn start_registry_server(state: State<'_, AppState>) -> Result<u16, String> {
    let mut handle_guard = state.registry_server_handle.lock().await;
    if let Some((port, _)) = handle_guard.as_ref() {
        return Ok(*port);
    }
    let (port, handle) = crate::registry_server::start_server()
        .await
        .map_err(|e| e.to_string())?;
    let http_registry = crate::registry_server::HttpRegistry::new(port);
    *state.package_registry.lock().await = Box::new(http_registry);
    *handle_guard = Some((port, handle));
    Ok(port)
}

#[tauri::command]
pub async fn stop_registry_server(state: State<'_, AppState>) -> Result<(), String> {
    let mut handle_guard = state.registry_server_handle.lock().await;
    if let Some((_, handle)) = handle_guard.take() {
        handle.abort();
    }
    // Restore the in-process catalog registry so the marketplace remains
    // populated after the HTTP server is stopped.
    *state.package_registry.lock().await =
        Box::new(crate::registry_server::CatalogRegistry::new());
    Ok(())
}

#[tauri::command]
pub async fn get_registry_server_port(state: State<'_, AppState>) -> Result<Option<u16>, String> {
    let handle_guard = state.registry_server_handle.lock().await;
    Ok(handle_guard.as_ref().map(|(port, _)| *port))
}

/// Convert a [`crate::brain::ModelRecommendation`] into a virtual marketplace
/// agent so the user can browse local-LLM models alongside packaged agents.
fn local_llm_to_search_result(rec: &crate::brain::ModelRecommendation) -> AgentSearchResult {
    let homepage = if rec.is_cloud {
        Some("https://ollama.com/cloud".to_string())
    } else {
        Some(format!(
            "https://ollama.com/library/{}",
            rec.model_tag.split(':').next().unwrap_or(&rec.model_tag)
        ))
    };
    let mut caps = vec!["chat".to_string(), "local_llm".to_string()];
    if rec.is_cloud {
        caps.push("network".to_string());
    }
    AgentSearchResult {
        name: format!("ollama:{}", rec.model_tag),
        version: "ollama".to_string(),
        description: rec.description.clone(),
        capabilities: caps,
        homepage,
        kind: "local_llm".to_string(),
        model_tag: Some(rec.model_tag.clone()),
        required_ram_mb: Some(rec.required_ram_mb),
        is_top_pick: rec.is_top_pick,
        is_cloud: rec.is_cloud,
    }
}

#[tauri::command]
pub async fn search_agents(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<AgentSearchResult>, String> {
    let registry = state.package_registry.lock().await;
    let manifests = registry.search(&query).await.map_err(|e| e.to_string())?;
    drop(registry);

    let mut results: Vec<AgentSearchResult> = manifests
        .into_iter()
        .map(|m| {
            let capabilities = m
                .capabilities
                .iter()
                .map(|c| {
                    serde_json::to_value(c)
                        .unwrap_or_default()
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string()
                })
                .collect();
            AgentSearchResult {
                name: m.name,
                version: m.version,
                description: m.description,
                capabilities,
                homepage: m.homepage,
                kind: "package".to_string(),
                model_tag: None,
                required_ram_mb: None,
                is_top_pick: false,
                is_cloud: false,
            }
        })
        .collect();

    // Local LLM models are also marketplace agents — surface the
    // hardware-appropriate Ollama recommendations alongside packaged agents.
    let info = crate::brain::collect_system_info();
    let recommendations = crate::brain::recommend(info.total_ram_mb);
    let q = query.to_lowercase();
    for rec in recommendations.iter() {
        let r = local_llm_to_search_result(rec);
        if q.is_empty()
            || r.name.to_lowercase().contains(&q)
            || r.description.to_lowercase().contains(&q)
            || r.capabilities.iter().any(|c| c.to_lowercase().contains(&q))
        {
            results.push(r);
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::ModelRecommendation;

    #[test]
    fn local_llm_to_search_result_marks_kind_and_model_tag() {
        let rec = ModelRecommendation {
            model_tag: "gemma3:4b".to_string(),
            display_name: "Gemma 3 4B".to_string(),
            description: "Test model".to_string(),
            required_ram_mb: 6_144,
            download_size_mb: 0,
            is_top_pick: true,
            is_cloud: false,
        };
        let r = local_llm_to_search_result(&rec);
        assert_eq!(r.name, "ollama:gemma3:4b");
        assert_eq!(r.kind, "local_llm");
        assert_eq!(r.model_tag.as_deref(), Some("gemma3:4b"));
        assert_eq!(r.required_ram_mb, Some(6_144));
        assert!(r.is_top_pick);
        assert!(!r.is_cloud);
        assert!(r.capabilities.contains(&"chat".to_string()));
        assert!(r.capabilities.contains(&"local_llm".to_string()));
        assert!(!r.capabilities.contains(&"network".to_string()));
        assert_eq!(
            r.homepage.as_deref(),
            Some("https://ollama.com/library/gemma3"),
        );
    }

    #[test]
    fn local_llm_cloud_model_advertises_network_capability_and_cloud_homepage() {
        let rec = ModelRecommendation {
            model_tag: "kimi-k2.6:cloud".to_string(),
            display_name: "Kimi K2.6".to_string(),
            description: "Cloud-routed".to_string(),
            required_ram_mb: 0,
            download_size_mb: 0,
            is_top_pick: false,
            is_cloud: true,
        };
        let r = local_llm_to_search_result(&rec);
        assert!(r.is_cloud);
        assert!(r.capabilities.contains(&"network".to_string()));
        assert_eq!(r.homepage.as_deref(), Some("https://ollama.com/cloud"));
    }
}
