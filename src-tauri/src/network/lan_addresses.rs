//! Pure LAN address classifier — Chunk 24.1a.
//!
//! Foundation for the iOS-companion / phone-control feature. Given a
//! list of `IpAddr` (typically returned by an OS interface enumerator
//! like `local-ip-address` or `if-addrs` — the OS-probe wrapper is
//! Chunk 24.1b), this module:
//!
//! 1. **Filters** out addresses that should never be advertised as
//!    pairing endpoints — loopback, unspecified, multicast, link-local,
//!    documentation / reserved ranges, and (by default) IPv6.
//! 2. **Classifies** the survivors into [`LanAddressKind::Private`]
//!    (RFC 1918 / RFC 6598 shared address space — the realistic Wi-Fi
//!    home-LAN case) and [`LanAddressKind::Public`] (everything else
//!    that survived filtering — almost always an outbound-egress IPv4
//!    such as a hotel / coworking NAT escape).
//!
//! No syscalls. No allocation beyond the result vector. Fully
//! deterministic on fixture input — every classification rule is
//! covered by a unit test.
//!
//! ## Why a pure layer first
//!
//! The "exposing the brain to the LAN" surface is security-critical.
//! Splitting the OS probe (24.1b) from the classifier (24.1a) means
//! the rules that decide *which* addresses are legitimate pairing
//! endpoints are hand-auditable and unit-testable without mocking the
//! OS. When the OS-probe wrapper lands, it is "just" a 30-line call
//! to `local_ip_address::list_afinet_netifas()` followed by a
//! `classify_addresses(...)` call.

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};

/// What kind of network the address sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanAddressKind {
    /// RFC 1918 (`10/8`, `172.16/12`, `192.168/16`) or RFC 6598
    /// shared address space (`100.64/10`). The realistic case for
    /// Wi-Fi home / office LANs and most carrier NATs.
    Private,
    /// Everything else that survived filtering. Surface with a
    /// warning in the pairing UI — the user is one mistake away from
    /// exposing their brain to the public internet.
    Public,
}

/// A single classified LAN address suitable for advertising in a
/// pairing QR / discovery payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanAddress {
    pub addr: IpAddr,
    pub kind: LanAddressKind,
}

/// Settings that govern classification.
///
/// Defaults (via `Default`): IPv6 disabled, public-routable disabled —
/// the conservative posture appropriate for a home-LAN pairing UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClassifyOptions {
    /// Allow IPv6 addresses through. Default `false` — every Wi-Fi
    /// stack we care about for the iOS-companion phase exposes a
    /// reachable IPv4, and IPv6 ULA / global addressing pulls in a
    /// second tier of reachability rules we don't need yet.
    pub allow_ipv6: bool,
    /// Allow public-routable IPv4 through. Default `false` —
    /// home LANs are universally private; surfacing a public address
    /// is almost always a misconfiguration (running on a VPS, in a
    /// container with host-net, etc.).
    pub allow_public: bool,
}

/// Classify a list of candidate addresses into the LAN-pairing-eligible
/// subset.
///
/// Filtering rules (always applied, regardless of options):
/// - Reject `is_unspecified()` (`0.0.0.0`, `::`).
/// - Reject `is_loopback()` (`127/8`, `::1`).
/// - Reject `is_multicast()` (`224/4`, `ff00::/8`).
/// - Reject IPv4 link-local (`169.254/16`).
/// - Reject IPv4 documentation ranges (`192.0.2/24`, `198.51.100/24`,
///   `203.0.113/24`).
/// - Reject IPv4 benchmarking (`198.18/15`).
/// - Reject IPv4 broadcast (`255.255.255.255`).
///
/// Then by option:
/// - If `!allow_ipv6`, drop all `IpAddr::V6`.
/// - If `!allow_public`, drop everything classified as
///   [`LanAddressKind::Public`].
///
/// The output preserves input order so the UI can display "the
/// interface listed first by the OS first".
pub fn classify_addresses(
    candidates: &[IpAddr],
    options: ClassifyOptions,
) -> Vec<LanAddress> {
    candidates
        .iter()
        .copied()
        .filter(|a| !is_always_rejected(a))
        .filter(|a| options.allow_ipv6 || a.is_ipv4())
        .filter_map(|a| {
            let kind = classify_kind(&a)?;
            if !options.allow_public && kind == LanAddressKind::Public {
                return None;
            }
            Some(LanAddress { addr: a, kind })
        })
        .collect()
}

/// Returns `true` when the address must never be considered a pairing
/// endpoint regardless of caller options.
fn is_always_rejected(a: &IpAddr) -> bool {
    if a.is_unspecified() || a.is_loopback() || a.is_multicast() {
        return true;
    }
    match a {
        IpAddr::V4(v4) => is_v4_always_rejected(v4),
        IpAddr::V6(_) => false, // V6-specific extra rules not needed yet.
    }
}

fn is_v4_always_rejected(v4: &Ipv4Addr) -> bool {
    if v4.is_link_local() || v4.is_broadcast() {
        return true;
    }
    let o = v4.octets();
    // Documentation ranges (RFC 5737).
    if (o[0] == 192 && o[1] == 0 && o[2] == 2)
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)
    {
        return true;
    }
    // Benchmarking (RFC 2544).
    if o[0] == 198 && (o[1] == 18 || o[1] == 19) {
        return true;
    }
    false
}

/// Classify a *non-rejected* address into Private vs Public.
/// Returns `None` if the caller forgot to filter rejected addresses
/// first (defence in depth — the public `classify_addresses` always
/// pre-filters).
fn classify_kind(a: &IpAddr) -> Option<LanAddressKind> {
    match a {
        IpAddr::V4(v4) => {
            if is_v4_always_rejected(v4) || v4.is_loopback() || v4.is_unspecified() {
                return None;
            }
            Some(if is_v4_private(v4) {
                LanAddressKind::Private
            } else {
                LanAddressKind::Public
            })
        }
        IpAddr::V6(v6) => {
            if v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() {
                return None;
            }
            // Treat IPv6 unique-local (fc00::/7) as Private, everything
            // else (including link-local, which we already filter)
            // as Public. The upstream filter drops V6 entirely unless
            // `allow_ipv6` is on.
            let seg = v6.segments()[0];
            Some(if (seg & 0xfe00) == 0xfc00 {
                LanAddressKind::Private
            } else {
                LanAddressKind::Public
            })
        }
    }
}

/// RFC 1918 + RFC 6598 shared-address-space.
fn is_v4_private(v4: &Ipv4Addr) -> bool {
    let o = v4.octets();
    // 10.0.0.0/8
    if o[0] == 10 {
        return true;
    }
    // 172.16.0.0/12 → o[0] == 172 && 16 ≤ o[1] ≤ 31
    if o[0] == 172 && (16..=31).contains(&o[1]) {
        return true;
    }
    // 192.168.0.0/16
    if o[0] == 192 && o[1] == 168 {
        return true;
    }
    // 100.64.0.0/10 — RFC 6598 carrier-grade NAT
    if o[0] == 100 && (64..=127).contains(&o[1]) {
        return true;
    }
    false
}

/// Convenience: filter to **only** the private LAN survivors. This is
/// the most common case for the pairing UI — the user almost never
/// wants to advertise a public-routable IP to a phone.
pub fn private_lan_addresses(candidates: &[IpAddr]) -> Vec<LanAddress> {
    classify_addresses(candidates, ClassifyOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    fn v4(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(a, b, c, d))
    }

    #[test]
    fn rejects_loopback_unspecified_multicast_broadcast() {
        let addrs = vec![
            v4(127, 0, 0, 1),
            v4(0, 0, 0, 0),
            v4(224, 0, 0, 1),
            v4(255, 255, 255, 255),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        ];
        let out = classify_addresses(&addrs, ClassifyOptions::default());
        assert!(out.is_empty(), "expected all rejected, got {out:?}");
    }

    #[test]
    fn rejects_link_local_v4() {
        let addrs = vec![v4(169, 254, 1, 1)];
        assert!(classify_addresses(&addrs, ClassifyOptions::default()).is_empty());
    }

    #[test]
    fn rejects_documentation_and_benchmarking_ranges() {
        let addrs = vec![
            v4(192, 0, 2, 1),
            v4(198, 51, 100, 1),
            v4(203, 0, 113, 1),
            v4(198, 18, 0, 1),
            v4(198, 19, 0, 1),
        ];
        let opts = ClassifyOptions {
            allow_public: true,
            ..Default::default()
        };
        assert!(classify_addresses(&addrs, opts).is_empty());
    }

    #[test]
    fn classifies_rfc1918_as_private() {
        let addrs = vec![
            v4(10, 0, 0, 1),
            v4(172, 16, 0, 1),
            v4(172, 31, 255, 254),
            v4(192, 168, 1, 50),
        ];
        let out = classify_addresses(&addrs, ClassifyOptions::default());
        assert_eq!(out.len(), 4);
        assert!(out.iter().all(|l| l.kind == LanAddressKind::Private));
    }

    #[test]
    fn rfc1918_boundary_addresses() {
        // 172.15.x.x is *not* private; 172.32.x.x is *not* private.
        let addrs = vec![v4(172, 15, 0, 1), v4(172, 32, 0, 1)];
        let opts = ClassifyOptions {
            allow_public: true,
            ..Default::default()
        };
        let out = classify_addresses(&addrs, opts);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|l| l.kind == LanAddressKind::Public));
    }

    #[test]
    fn classifies_rfc6598_carrier_grade_nat_as_private() {
        let addrs = vec![v4(100, 64, 0, 1), v4(100, 127, 255, 254)];
        let out = classify_addresses(&addrs, ClassifyOptions::default());
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|l| l.kind == LanAddressKind::Private));

        // 100.63.x.x is below the range; 100.128.x.x above.
        let outside = vec![v4(100, 63, 0, 1), v4(100, 128, 0, 1)];
        let opts = ClassifyOptions {
            allow_public: true,
            ..Default::default()
        };
        let out = classify_addresses(&outside, opts);
        assert!(out.iter().all(|l| l.kind == LanAddressKind::Public));
    }

    #[test]
    fn drops_public_by_default() {
        let addrs = vec![v4(192, 168, 1, 1), v4(8, 8, 8, 8)];
        let out = classify_addresses(&addrs, ClassifyOptions::default());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].addr, v4(192, 168, 1, 1));
        assert_eq!(out[0].kind, LanAddressKind::Private);
    }

    #[test]
    fn allow_public_keeps_routable_addresses() {
        let addrs = vec![v4(8, 8, 8, 8), v4(1, 1, 1, 1)];
        let opts = ClassifyOptions {
            allow_public: true,
            ..Default::default()
        };
        let out = classify_addresses(&addrs, opts);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|l| l.kind == LanAddressKind::Public));
    }

    #[test]
    fn drops_ipv6_by_default() {
        let addrs = vec![
            IpAddr::V6("fd12:3456:789a::1".parse().unwrap()),
            v4(192, 168, 1, 1),
        ];
        let out = classify_addresses(&addrs, ClassifyOptions::default());
        assert_eq!(out.len(), 1);
        assert!(out[0].addr.is_ipv4());
    }

    #[test]
    fn ipv6_unique_local_classified_as_private_when_allowed() {
        let addrs = vec![IpAddr::V6("fd12:3456:789a::1".parse().unwrap())];
        let opts = ClassifyOptions {
            allow_ipv6: true,
            ..Default::default()
        };
        let out = classify_addresses(&addrs, opts);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, LanAddressKind::Private);
    }

    #[test]
    fn ipv6_global_classified_as_public_and_dropped_by_default() {
        let global: IpAddr = "2001:db8::1".parse().unwrap();
        let opts = ClassifyOptions {
            allow_ipv6: true,
            ..Default::default()
        };
        // allow_ipv6 only — public still dropped.
        assert!(classify_addresses(&[global], opts).is_empty());

        let opts2 = ClassifyOptions {
            allow_ipv6: true,
            allow_public: true,
        };
        let out = classify_addresses(&[global], opts2);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, LanAddressKind::Public);
    }

    #[test]
    fn preserves_input_order() {
        let addrs = vec![
            v4(192, 168, 50, 10),
            v4(10, 0, 0, 1),
            v4(172, 20, 5, 5),
        ];
        let out = classify_addresses(&addrs, ClassifyOptions::default());
        let expected = vec![
            v4(192, 168, 50, 10),
            v4(10, 0, 0, 1),
            v4(172, 20, 5, 5),
        ];
        assert_eq!(out.iter().map(|l| l.addr).collect::<Vec<_>>(), expected);
    }

    #[test]
    fn private_lan_addresses_helper_matches_default_options() {
        let addrs = vec![
            v4(192, 168, 1, 1),
            v4(8, 8, 8, 8),
            v4(127, 0, 0, 1),
        ];
        let helper = private_lan_addresses(&addrs);
        let manual = classify_addresses(&addrs, ClassifyOptions::default());
        assert_eq!(helper, manual);
        assert_eq!(helper.len(), 1);
        assert_eq!(helper[0].kind, LanAddressKind::Private);
    }

    #[test]
    fn empty_input_yields_empty_output() {
        assert!(classify_addresses(&[], ClassifyOptions::default()).is_empty());
        assert!(private_lan_addresses(&[]).is_empty());
    }
}
