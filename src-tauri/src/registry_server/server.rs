use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Json, Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::package_manager::AgentManifest;
use super::catalog;

#[derive(Clone)]
struct ServerState {
    agents: Arc<HashMap<String, AgentManifest>>,
}

#[derive(serde::Deserialize)]
struct SearchQuery {
    q: String,
}

async fn list_agents(State(state): State<ServerState>) -> Json<Vec<AgentManifest>> {
    let agents: Vec<AgentManifest> = state.agents.values().cloned().collect();
    Json(agents)
}

async fn get_agent(
    State(state): State<ServerState>,
    Path(name): Path<String>,
) -> Result<Json<AgentManifest>, StatusCode> {
    state
        .agents
        .get(&name)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn download_agent(
    State(state): State<ServerState>,
    Path(name): Path<String>,
) -> Result<Response, StatusCode> {
    let Some(manifest) = state.agents.get(&name) else {
        return Err(StatusCode::NOT_FOUND);
    };
    // Chunk 1.7 hosting model: the registry **does not** stream-proxy
    // third-party binaries — it would consume bandwidth and turn the
    // registry into a single point of failure. Instead, downloadable
    // install methods (`Binary { url }` / `Wasm { url }`) get a
    // `307 Temporary Redirect` to the upstream host (GitHub Releases /
    // S3 / R2). HTTP clients (`reqwest` included) follow this redirect
    // automatically, so the installer's download path is unchanged.
    //
    // Built-in agents (`InstallMethod::BuiltIn`) and bundled `Sidecar`
    // entries have no remote URL — return an empty
    // `application/octet-stream` body so the contract "GET /download
    // never 404s for known agents" holds.
    match &manifest.install_method {
        crate::package_manager::InstallMethod::Binary { url }
        | crate::package_manager::InstallMethod::Wasm { url } => {
            Ok(Redirect::temporary(url).into_response())
        }
        crate::package_manager::InstallMethod::Sidecar { .. }
        | crate::package_manager::InstallMethod::BuiltIn => Ok((
            [(header::CONTENT_TYPE, "application/octet-stream")],
            Vec::<u8>::new(),
        )
            .into_response()),
    }
}

async fn search_agents(
    State(state): State<ServerState>,
    Query(params): Query<SearchQuery>,
) -> Json<Vec<AgentManifest>> {
    let q = params.q.to_lowercase();
    let results: Vec<AgentManifest> = state
        .agents
        .values()
        .filter(|m| {
            m.name.to_lowercase().contains(&q) || m.description.to_lowercase().contains(&q)
        })
        .cloned()
        .collect();
    Json(results)
}

pub async fn start() -> Result<(u16, tokio::task::JoinHandle<()>), String> {
    let entries = catalog::all_entries();
    let mut map = HashMap::new();
    for entry in entries {
        map.insert(entry.name.clone(), entry);
    }
    let state = ServerState {
        agents: Arc::new(map),
    };

    let app = Router::new()
        .route("/agents", get(list_agents))
        .route("/agents/{name}", get(get_agent))
        .route("/agents/{name}/download", get(download_agent))
        .route("/search", get(search_agents))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok((port, handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_server::http_registry::HttpRegistry;
    use crate::package_manager::RegistrySource;

    #[tokio::test]
    async fn test_catalog_has_three_agents() {
        let entries = catalog::all_entries();
        assert_eq!(entries.len(), 3);
    }

    #[tokio::test]
    async fn test_server_list_agents() {
        let (port, _handle) = start().await.unwrap();
        let url = format!("http://127.0.0.1:{port}/agents");
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 200);
        let agents: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(agents.len(), 3);
    }

    #[tokio::test]
    async fn test_server_get_agent_by_name() {
        let (port, _handle) = start().await.unwrap();
        let url = format!("http://127.0.0.1:{port}/agents/stub-agent");
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 200);
        let manifest: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(manifest["name"], "stub-agent");
    }

    #[tokio::test]
    async fn test_server_agent_not_found() {
        let (port, _handle) = start().await.unwrap();
        let url = format!("http://127.0.0.1:{port}/agents/nonexistent-agent");
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_server_search() {
        let (port, _handle) = start().await.unwrap();
        let url = format!("http://127.0.0.1:{port}/search?q=stub");
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 200);
        let agents: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0]["name"], "stub-agent");
    }

    #[tokio::test]
    async fn test_http_registry_fetch_manifest() {
        let (port, _handle) = start().await.unwrap();
        let registry = HttpRegistry::new(port);
        let manifest = registry.fetch_manifest("stub-agent").await.unwrap();
        assert_eq!(manifest.name, "stub-agent");
    }

    #[tokio::test]
    async fn test_http_registry_not_found() {
        let (port, _handle) = start().await.unwrap();
        let registry = HttpRegistry::new(port);
        let result = registry.fetch_manifest("nonexistent").await;
        assert!(matches!(
            result,
            Err(crate::package_manager::RegistryError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn test_http_registry_search() {
        let (port, _handle) = start().await.unwrap();
        let registry = HttpRegistry::new(port);
        let results = registry.search("claude").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "claude-cowork");
    }
}
