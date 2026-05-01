//! LAN discovery and pairing commands — Phase 24.
//!
//! Tauri commands for the mobile companion pairing UI.

use tauri::State;

use crate::network::lan_addresses::LanAddress;
use crate::network::lan_probe::discover_lan_addresses;
use crate::network::pairing::PairedDevice;
use crate::network::vscode_log::CopilotLogSummary;
use crate::network::vscode_probe;
use crate::AppState;

/// List LAN-eligible addresses on this machine for the pairing UI.
///
/// Returns only private IPv4 addresses by default (conservative).
/// The frontend displays these in the pairing QR code / manual-entry
/// dialog so the iOS companion knows which IP to connect to.
#[tauri::command]
pub fn list_lan_addresses() -> Vec<LanAddress> {
    discover_lan_addresses()
}

/// Probe the local VS Code installation for the latest Copilot Chat
/// session and return a structured summary. Returns `null` if VS Code
/// or the Copilot extension is not found.
///
/// Maps to Chunk 24.5b.
#[tauri::command]
pub async fn get_copilot_session_status() -> Option<CopilotLogSummary> {
    vscode_probe::probe_copilot_session().await
}

/// Start a pairing session. Returns a `terransoul://pair?...` URI for QR display.
///
/// The session expires after 5 minutes. Only one active session at a time.
/// Lazily initializes the pairing CA on first call.
#[tauri::command]
pub async fn start_pairing(state: State<'_, AppState>) -> Result<String, String> {
    // Check lan_enabled first.
    {
        let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        if !settings.lan_enabled {
            return Err("LAN mode not enabled — enable lan_enabled in settings first".to_string());
        }
    }

    // Lazily init the pairing manager.
    {
        let mut guard = state.pairing_manager.lock().map_err(|e| e.to_string())?;
        if guard.is_none() {
            let mgr = crate::network::pairing::PairingManager::load_or_create(&state.data_dir)?;
            *guard = Some(mgr);
        }
    }

    // Use the first LAN address as the host for the QR payload.
    let addrs = discover_lan_addresses();
    let host = addrs
        .first()
        .map(|a| a.addr.to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    let mcp_port = state
        .mcp_server
        .lock()
        .await
        .as_ref()
        .map(|h| h.port)
        .unwrap_or(7421);

    // Now acquire again after the await.
    let pairing_mgr = state.pairing_manager.lock().map_err(|e| e.to_string())?;
    let mgr = pairing_mgr.as_ref().unwrap();
    mgr.start_pairing(&host, mcp_port)
}

/// Confirm a pending pairing. The phone presents the token it received from the QR code.
///
/// On success, returns the issued client certificate bundle for the phone to store.
#[tauri::command]
pub async fn confirm_pairing(
    state: State<'_, AppState>,
    device_id: String,
    display_name: String,
    token_b64: String,
) -> Result<crate::network::pairing::PairingResult, String> {
    use base64::Engine;

    let token_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&token_b64)
        .map_err(|e| format!("invalid token: {e}"))?;
    if token_bytes.len() != 32 {
        return Err("token must be 32 bytes".to_string());
    }
    let mut token = [0u8; 32];
    token.copy_from_slice(&token_bytes);

    let pairing_mgr = state.pairing_manager.lock().map_err(|e| e.to_string())?;
    let mgr = pairing_mgr.as_ref().ok_or("no active pairing session — call start_pairing first")?;

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    mgr.confirm_pairing(store.conn(), &device_id, &display_name, &token)
}

/// Revoke (unpair) a device by its ID.
#[tauri::command]
pub async fn revoke_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<bool, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    crate::network::pairing::revoke_device(store.conn(), &device_id)
}

/// List all currently paired devices.
#[tauri::command]
pub async fn list_paired_devices(state: State<'_, AppState>) -> Result<Vec<PairedDevice>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    crate::network::pairing::list_paired_devices(store.conn())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_lan_addresses_returns_only_private_ipv4() {
        let addrs = list_lan_addresses();
        for a in &addrs {
            assert!(a.addr.is_ipv4());
            assert_eq!(
                a.kind,
                crate::network::lan_addresses::LanAddressKind::Private
            );
        }
    }
}
