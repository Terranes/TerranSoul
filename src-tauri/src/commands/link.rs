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
