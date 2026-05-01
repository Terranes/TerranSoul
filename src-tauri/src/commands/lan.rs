//! LAN discovery and pairing commands — Phase 24.
//!
//! Tauri commands for the mobile companion pairing UI.

use crate::network::lan_addresses::LanAddress;
use crate::network::lan_probe::discover_lan_addresses;
use crate::network::vscode_log::CopilotLogSummary;
use crate::network::vscode_probe;

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
