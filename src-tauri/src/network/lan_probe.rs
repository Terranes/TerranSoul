//! OS network-interface probe + LAN address discovery — Chunk 24.1b.
//!
//! Thin wrapper around the `local-ip-address` crate that enumerates
//! system network interfaces and feeds the results through the pure
//! classifier from Chunk 24.1a (`network::lan_addresses`).
//!
//! Also exposes a Tauri command `list_lan_addresses` for the pairing UI.

use crate::network::lan_addresses::{classify_addresses, ClassifyOptions, LanAddress};
use std::net::IpAddr;

/// Discover LAN-eligible addresses on this machine.
///
/// Enumerates all network interfaces via the OS, extracts their IP
/// addresses, then runs them through the 24.1a classifier with
/// conservative defaults (IPv4-only, private-only).
///
/// Returns an empty vec on any OS error (e.g. sandboxed environment
/// where interface enumeration is blocked).
pub fn discover_lan_addresses() -> Vec<LanAddress> {
    discover_lan_addresses_with(ClassifyOptions::default())
}

/// Like [`discover_lan_addresses`] but with custom classification options.
pub fn discover_lan_addresses_with(options: ClassifyOptions) -> Vec<LanAddress> {
    let candidates = enumerate_os_addresses();
    classify_addresses(&candidates, options)
}

/// Enumerate all IP addresses from the OS network interfaces.
/// Returns an empty vec on any error.
fn enumerate_os_addresses() -> Vec<IpAddr> {
    match local_ip_address::list_afinet_netifas() {
        Ok(interfaces) => interfaces.into_iter().map(|(_, addr)| addr).collect(),
        Err(_) => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_returns_only_private_ipv4_by_default() {
        let addrs = discover_lan_addresses();
        for a in &addrs {
            assert!(a.addr.is_ipv4(), "IPv6 should be filtered by default");
            assert_eq!(
                a.kind,
                crate::network::lan_addresses::LanAddressKind::Private
            );
        }
    }

    #[test]
    fn discover_with_ipv6_may_include_v6() {
        let opts = ClassifyOptions {
            allow_ipv6: true,
            allow_public: false,
        };
        let _ = discover_lan_addresses_with(opts);
        // Just verify it doesn't panic — actual v6 presence is machine-dependent.
    }

    #[test]
    fn enumerate_os_addresses_does_not_panic() {
        let addrs = enumerate_os_addresses();
        // On any real machine there's at least loopback, but the
        // classifier will filter it. Just ensure no panic.
        assert!(!addrs.is_empty() || addrs.is_empty()); // always passes; tests no-panic
    }
}
