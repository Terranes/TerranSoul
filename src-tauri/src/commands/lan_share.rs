//! Tauri commands for LAN brain sharing and discovery.
//!
//! Allows TerranSoul instances to share knowledge on a local network.
//! One instance hosts its MCP brain, others discover and connect.

use std::sync::Arc;

use tauri::State;

use crate::network::lan_share::{
    DiscoveredBrain, LanShareAdvertiser, LanShareBrowser, RemoteBrainClient, RemoteBrainConnection,
    RemoteBrainHealth, RemoteSearchResult,
};
use crate::AppState;

/// Status of the LAN sharing system.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LanShareStatus {
    /// Whether this instance is advertising its brain on LAN.
    pub hosting: bool,
    /// Display name of the shared brain (when hosting).
    pub brain_name: Option<String>,
    /// Port the MCP server is bound to (when hosting).
    pub port: Option<u16>,
    /// Bearer token for clients to connect (when hosting).
    pub token: Option<String>,
    /// Selected LAN auth mode for the host.
    pub auth_mode: String,
    /// Whether remote clients must provide a token.
    pub token_required: bool,
    /// Number of connected remote brains.
    pub connected_brains: u32,
    /// Connected remote brain info.
    pub connections: Vec<RemoteBrainConnection>,
}

/// Start advertising this TerranSoul brain on the LAN via UDP broadcast.
///
/// Requires `lan_enabled = true` in settings and an active MCP server.
/// The brain becomes discoverable by other TerranSoul instances on the
/// same network. Clients need the bearer token (displayed in UI) to query.
#[tauri::command]
pub async fn lan_share_start(
    brain_name: String,
    state: State<'_, AppState>,
) -> Result<LanShareStatus, String> {
    let lan_auth_mode = {
        let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        if !settings.lan_enabled {
            return Err("LAN mode not enabled — enable lan_enabled in settings first".to_string());
        }
        settings.lan_auth_mode
    };
    let token_required = matches!(lan_auth_mode, crate::settings::LanAuthMode::TokenRequired);

    // Verify MCP server is running.
    let (port, token) = {
        let mcp_guard = state.mcp_server.lock().await;
        match mcp_guard.as_ref() {
            Some(handle) => (
                handle.port,
                if token_required {
                    Some(handle.token.clone())
                } else {
                    None
                },
            ),
            None => {
                return Err(
                    "MCP server must be running before sharing on LAN. Start it first.".to_string(),
                )
            }
        }
    };

    // Get memory count for TXT record.
    let memory_count = state
        .memory_store
        .lock()
        .ok()
        .and_then(|store| store.stats().ok())
        .map(|s| s.total as u32)
        .unwrap_or(0);

    // Get brain provider info.
    let provider = state
        .brain_mode
        .lock()
        .ok()
        .and_then(|mode| {
            mode.as_ref().map(|m| match m {
                crate::brain::BrainMode::FreeApi { provider_id, .. } => {
                    format!("free:{provider_id}")
                }
                crate::brain::BrainMode::PaidApi { provider, .. } => {
                    format!("paid:{provider}")
                }
                crate::brain::BrainMode::LocalOllama { model, .. } => {
                    format!("ollama:{model}")
                }
                _ => "local".to_string(),
            })
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Start UDP broadcast advertisement.
    let advertiser = LanShareAdvertiser::start(
        &brain_name,
        port,
        &provider,
        memory_count,
        true,
        token_required,
    )
    .await?;

    // Store in LAN share state.
    {
        let mut lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
        lan_share.advertiser = Some(advertiser);
    }

    Ok(LanShareStatus {
        hosting: true,
        brain_name: Some(brain_name),
        port: Some(port),
        token,
        auth_mode: match lan_auth_mode {
            crate::settings::LanAuthMode::TokenRequired => "token_required",
            crate::settings::LanAuthMode::PublicReadOnly => "public_read_only",
        }
        .to_string(),
        token_required,
        connected_brains: 0,
        connections: Vec::new(),
    })
}

/// Stop advertising this brain on the LAN.
#[tauri::command]
pub async fn lan_share_stop(state: State<'_, AppState>) -> Result<(), String> {
    let mut lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
    if let Some(advertiser) = lan_share.advertiser.take() {
        advertiser.stop();
    }
    Ok(())
}

/// Discover TerranSoul brains available on the LAN.
///
/// Starts or reuses a UDP broadcast listener, waits briefly for responses,
/// then returns all discovered brains.
#[tauri::command]
pub async fn lan_share_discover(
    state: State<'_, AppState>,
) -> Result<Vec<DiscoveredBrain>, String> {
    // Start browser if not already running (scope the lock before await).
    let needs_start = {
        let lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
        lan_share.browser.is_none()
    };

    if needs_start {
        let browser = LanShareBrowser::start().await?;
        let mut lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
        lan_share.browser = Some(browser);
    }

    // Give UDP listener time to collect broadcast responses.
    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

    let lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
    let discovered = if let Some(ref browser) = lan_share.browser {
        browser.collect_discovered()
    } else {
        Vec::new()
    };

    Ok(discovered)
}

/// Stop the discovery browser.
#[tauri::command]
pub async fn lan_share_stop_discovery(state: State<'_, AppState>) -> Result<(), String> {
    let mut lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
    if let Some(browser) = lan_share.browser.take() {
        browser.stop();
    }
    Ok(())
}

/// Connect to a remote TerranSoul brain.
///
/// The token must be obtained out-of-band (displayed on host's UI,
/// shared via QR code, email, etc.).
#[tauri::command]
pub async fn lan_share_connect(
    host: String,
    port: u16,
    token: Option<String>,
    token_required: Option<bool>,
    brain_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<RemoteBrainConnection, String> {
    // Validate connection by checking remote health.
    let client = RemoteBrainClient::new(&host, port, token.as_deref());
    let health = client.health().await?;

    let display_name = brain_name.unwrap_or_else(|| {
        health
            .brain_provider
            .clone()
            .unwrap_or_else(|| "Remote Brain".to_string())
    });

    let connection_id = uuid::Uuid::new_v4().to_string();
    let connection = RemoteBrainConnection {
        id: connection_id.clone(),
        host: host.clone(),
        port,
        token: token.clone().filter(|value| !value.trim().is_empty()),
        token_required: token_required.unwrap_or_else(|| token.as_ref().is_some_and(|value| !value.trim().is_empty())),
        brain_name: display_name,
        connected: true,
    };

    // Store the client and connection info.
    {
        let mut lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
        lan_share
            .connections
            .insert(connection_id.clone(), Arc::new(client));
        lan_share
            .connection_info
            .insert(connection_id, connection.clone());
    }

    Ok(connection)
}

/// Disconnect from a remote brain.
#[tauri::command]
pub async fn lan_share_disconnect(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
    lan_share.connections.remove(&id);
    lan_share.connection_info.remove(&id);
    Ok(())
}

/// Search a connected remote brain.
#[tauri::command]
pub async fn lan_share_search(
    connection_id: String,
    query: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<RemoteSearchResult>, String> {
    let client = {
        let lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
        lan_share
            .connections
            .get(&connection_id)
            .cloned()
            .ok_or_else(|| format!("No connection with id '{connection_id}'"))?
    };

    client.search(&query, limit.unwrap_or(10)).await
}

/// Search ALL connected remote brains simultaneously.
///
/// Returns results tagged with which brain they came from. Useful for
/// the "ask all shared brains" workflow.
#[tauri::command]
pub async fn lan_share_search_all(
    query: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<TaggedRemoteResult>, String> {
    let clients: Vec<(String, String, Arc<RemoteBrainClient>)> = {
        let lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
        lan_share
            .connections
            .iter()
            .map(|(id, client)| {
                let name = lan_share
                    .connection_info
                    .get(id)
                    .map(|c| c.brain_name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                (id.clone(), name, client.clone())
            })
            .collect()
    };

    let limit = limit.unwrap_or(5);
    let mut all_results = Vec::new();

    for (conn_id, brain_name, client) in clients {
        match client.search(&query, limit).await {
            Ok(results) => {
                for result in results {
                    all_results.push(TaggedRemoteResult {
                        connection_id: conn_id.clone(),
                        brain_name: brain_name.clone(),
                        result,
                    });
                }
            }
            Err(e) => {
                eprintln!("[lan-share] search failed for {brain_name}: {e}");
            }
        }
    }

    // Sort by score descending.
    all_results.sort_by(|a, b| {
        b.result
            .score
            .partial_cmp(&a.result.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(all_results)
}

/// A search result tagged with which remote brain it came from.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaggedRemoteResult {
    pub connection_id: String,
    pub brain_name: String,
    pub result: RemoteSearchResult,
}

/// Get the remote brain's health/status.
#[tauri::command]
pub async fn lan_share_remote_health(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<RemoteBrainHealth, String> {
    let client = {
        let lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
        lan_share
            .connections
            .get(&connection_id)
            .cloned()
            .ok_or_else(|| format!("No connection with id '{connection_id}'"))?
    };

    client.health().await
}

/// Get the current LAN sharing status.
#[tauri::command]
pub async fn lan_share_status(state: State<'_, AppState>) -> Result<LanShareStatus, String> {
    let hosting = {
        let lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
        lan_share.advertiser.is_some()
    };

    let (port, token) = if hosting {
        let mcp_guard = state.mcp_server.lock().await;
        let token_required = state
            .app_settings
            .lock()
            .map_err(|e| e.to_string())?
            .lan_auth_mode
            == crate::settings::LanAuthMode::TokenRequired;
        let p = mcp_guard.as_ref().map(|h| h.port);
        let t = if token_required {
            mcp_guard.as_ref().map(|h| h.token.clone())
        } else {
            None
        };
        (p, t)
    } else {
        (None, None)
    };

    let lan_auth_mode = state
        .app_settings
        .lock()
        .map_err(|e| e.to_string())?
        .lan_auth_mode;
    let token_required = lan_auth_mode == crate::settings::LanAuthMode::TokenRequired;

    let lan_share = state.lan_share.lock().map_err(|e| e.to_string())?;
    let connections: Vec<RemoteBrainConnection> =
        lan_share.connection_info.values().cloned().collect();

    Ok(LanShareStatus {
        hosting,
        brain_name: None,
        port,
        token,
        auth_mode: match lan_auth_mode {
            crate::settings::LanAuthMode::TokenRequired => "token_required",
            crate::settings::LanAuthMode::PublicReadOnly => "public_read_only",
        }
        .to_string(),
        token_required,
        connected_brains: connections.len() as u32,
        connections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lan_share_status_serializes() {
        let status = LanShareStatus {
            hosting: true,
            brain_name: Some("HR Company Rules".to_string()),
            port: Some(7421),
            token: Some("test-token-123".to_string()),
            auth_mode: "token_required".to_string(),
            token_required: true,
            connected_brains: 2,
            connections: vec![],
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("HR Company Rules"));
        assert!(json.contains("7421"));
    }

    #[test]
    fn tagged_result_serializes() {
        let result = TaggedRemoteResult {
            connection_id: "abc-123".to_string(),
            brain_name: "Legal Team".to_string(),
            result: RemoteSearchResult {
                id: 1,
                content: "Test content".to_string(),
                tags: Some("law".to_string()),
                importance: 5,
                score: 0.85,
                source_url: None,
                tier: "long".to_string(),
            },
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Legal Team"));
        assert!(json.contains("Test content"));
    }
}
