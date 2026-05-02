use serde::Serialize;
use tauri::State;

use crate::link::{LinkPeer, LinkStatus};
use crate::AppState;

/// Summary of the current link state, exposed to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct LinkStatusResponse {
    pub status: LinkStatus,
    pub transport: String,
    pub peer: Option<LinkPeer>,
    pub server_port: Option<u16>,
}

/// Return the current link status.
#[tauri::command]
pub async fn get_link_status(state: State<'_, AppState>) -> Result<LinkStatusResponse, String> {
    let mgr = state.link_manager.lock().await;
    let peer = mgr.connected_peer().await;
    let transport = format!("{:?}", mgr.transport_kind());
    Ok(LinkStatusResponse {
        status: mgr.status(),
        transport,
        peer,
        server_port: *state.link_server_port.lock().await,
    })
}

/// Start the link server on the specified port (0 = auto-assign).
#[tauri::command]
pub async fn start_link_server(
    port: Option<u16>,
    state: State<'_, AppState>,
) -> Result<u16, String> {
    let mgr = state.link_manager.lock().await;
    let bound_port = mgr.start_server(port.unwrap_or(0)).await?;
    *state.link_server_port.lock().await = Some(bound_port);
    Ok(bound_port)
}

/// Connect to a peer at the given host:port.
#[tauri::command(rename_all = "camelCase")]
pub async fn connect_to_peer(
    host: String,
    port: u16,
    device_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let addr = crate::link::PeerAddr {
        host: host.clone(),
        port,
    };
    let peer = LinkPeer {
        device_id,
        name,
        addr: format!("{}:{}", host, port),
    };
    let mgr = state.link_manager.lock().await;
    mgr.connect(&addr, peer).await
}

/// Disconnect the current link.
#[tauri::command]
pub async fn disconnect_link(state: State<'_, AppState>) -> Result<(), String> {
    let mgr = state.link_manager.lock().await;
    mgr.disconnect().await?;
    *state.link_server_port.lock().await = None;
    Ok(())
}

// ── CRDT Memory Sync (Chunk 17.5) ────────────────────────────────────

/// Compute memory deltas for a peer device (since last sync).
///
/// Returns serialized deltas ready to send over Soul Link.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_memory_deltas(
    peer_device_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::memory::crdt_sync::SyncDelta>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let since = store
        .last_sync_time(&peer_device_id)
        .map_err(|e| e.to_string())?
        .unwrap_or(0);
    let device_id = state
        .device_identity
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|id| id.device_id.clone())
        .unwrap_or_else(|| "unknown".into());
    store
        .compute_sync_deltas(since, &device_id)
        .map_err(|e| e.to_string())
}

/// Apply inbound memory deltas from a peer device.
///
/// Uses LWW conflict resolution with device-id tiebreaker.
#[tauri::command(rename_all = "camelCase")]
pub async fn apply_memory_deltas(
    peer_device_id: String,
    deltas: Vec<crate::memory::crdt_sync::SyncDelta>,
    state: State<'_, AppState>,
) -> Result<crate::memory::crdt_sync::ApplyResult, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let device_id = state
        .device_identity
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|id| id.device_id.clone())
        .unwrap_or_else(|| "unknown".into());
    let result = store
        .apply_sync_deltas(&deltas, &device_id)
        .map_err(|e| e.to_string())?;
    let total = result.inserted + result.updated + result.soft_closed;
    store
        .log_sync(&peer_device_id, "inbound", total)
        .map_err(|e| e.to_string())?;
    Ok(result)
}
