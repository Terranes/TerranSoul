//! Privacy policy engine — enforces `share_scope` rules on outbound bundles.
//!
//! Key guarantees:
//! - `Private` memories NEVER appear in any outbound payload.
//! - `Paired` memories only travel between the user's own linked devices.
//! - `Hive` memories may be shared with any hive relay.
//! - Edges are included only if BOTH endpoints satisfy the target scope.

use std::collections::HashMap;

use super::protocol::{Bundle, EdgeDelta, MemoryDelta, ShareScope};

/// Target audience for a bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleTarget {
    /// Own-device sync (paired scope and above).
    Paired,
    /// Hive relay (only hive-scoped content).
    Hive,
}

/// Default share scope per cognitive kind.
///
/// Users can override these defaults in settings. This provides sensible
/// factory defaults where personal/private knowledge stays local.
pub fn default_scope_for_kind(cognitive_kind: Option<&str>) -> ShareScope {
    match cognitive_kind {
        Some("episodic") | Some("preference") | Some("personal") => ShareScope::Private,
        Some("procedural") | Some("semantic") | Some("factual") => ShareScope::Paired,
        _ => ShareScope::Private,
    }
}

/// Filter a bundle to only include content appropriate for the given target.
///
/// Returns `None` if the filtered bundle would be empty.
pub fn filter_bundle(bundle: &Bundle, target: BundleTarget) -> Option<Bundle> {
    let min_scope = match target {
        BundleTarget::Paired => ShareScope::Paired,
        BundleTarget::Hive => ShareScope::Hive,
    };

    // Filter memories by scope
    let allowed_memories: Vec<MemoryDelta> = bundle
        .memory_deltas
        .iter()
        .filter(|m| scope_satisfies(m.share_scope, min_scope))
        .cloned()
        .collect();

    // Build a set of allowed content hashes
    let allowed_hashes: std::collections::HashSet<&str> = allowed_memories
        .iter()
        .map(|m| m.content_hash.as_str())
        .collect();

    // Filter edges: both endpoints must be in the allowed set
    let allowed_edges: Vec<EdgeDelta> = bundle
        .edge_deltas
        .iter()
        .filter(|e| {
            allowed_hashes.contains(e.src_content_hash.as_str())
                && allowed_hashes.contains(e.dst_content_hash.as_str())
        })
        .cloned()
        .collect();

    if allowed_memories.is_empty() && allowed_edges.is_empty() {
        return None;
    }

    Some(Bundle {
        bundle_id: bundle.bundle_id.clone(),
        hlc_from: bundle.hlc_from,
        hlc_to: bundle.hlc_to,
        memory_deltas: allowed_memories,
        edge_deltas: allowed_edges,
    })
}

/// Check if a scope satisfies the minimum required scope.
///
/// `Private < Paired < Hive` (higher scope = more sharing allowed).
fn scope_satisfies(actual: ShareScope, minimum: ShareScope) -> bool {
    scope_level(actual) >= scope_level(minimum)
}

/// Numeric ordering: Private(0) < Paired(1) < Hive(2).
fn scope_level(scope: ShareScope) -> u8 {
    match scope {
        ShareScope::Private => 0,
        ShareScope::Paired => 1,
        ShareScope::Hive => 2,
    }
}

/// Apply default scopes to a set of memory deltas based on their cognitive_kind.
///
/// This is used when memories don't have an explicit scope set yet.
pub fn apply_default_scopes(deltas: &mut [MemoryDelta], overrides: &HashMap<String, ShareScope>) {
    for delta in deltas.iter_mut() {
        // If the delta already has an explicit non-private scope from the user, respect it.
        // Otherwise, apply the cognitive-kind default or the user's per-kind override.
        if delta.share_scope == ShareScope::Private {
            let kind = delta.cognitive_kind.as_deref();
            let scope = if let Some(k) = kind {
                overrides
                    .get(k)
                    .copied()
                    .unwrap_or_else(|| default_scope_for_kind(Some(k)))
            } else {
                default_scope_for_kind(None)
            };
            delta.share_scope = scope;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_memory(hash: &str, scope: ShareScope) -> MemoryDelta {
        MemoryDelta {
            content_hash: hash.into(),
            operation: "upsert".into(),
            content: format!("content for {hash}"),
            tags: "".into(),
            importance: 3,
            memory_type: "fact".into(),
            cognitive_kind: None,
            created_at: 100,
            updated_at: 100,
            hlc_counter: 1,
            origin_device: "dev-a".into(),
            share_scope: scope,
            source_url: None,
            source_hash: None,
            context_prefix: None,
            valid_to: None,
        }
    }

    fn make_edge(src: &str, dst: &str) -> EdgeDelta {
        EdgeDelta {
            src_content_hash: src.into(),
            dst_content_hash: dst.into(),
            rel_type: "related_to".into(),
            confidence: 0.9,
            source: "user".into(),
            created_at: 100,
            valid_from: None,
            valid_to: None,
            edge_source: None,
            origin_device: "dev-a".into(),
            hlc_counter: 1,
        }
    }

    #[test]
    fn private_memory_never_appears_in_hive_bundle() {
        let bundle = Bundle {
            bundle_id: "test".into(),
            hlc_from: 0,
            hlc_to: 10,
            memory_deltas: vec![
                make_memory("private-1", ShareScope::Private),
                make_memory("hive-1", ShareScope::Hive),
            ],
            edge_deltas: vec![],
        };

        let filtered = filter_bundle(&bundle, BundleTarget::Hive).unwrap();
        assert_eq!(filtered.memory_deltas.len(), 1);
        assert_eq!(filtered.memory_deltas[0].content_hash, "hive-1");
    }

    #[test]
    fn private_memory_never_appears_in_paired_bundle() {
        let bundle = Bundle {
            bundle_id: "test".into(),
            hlc_from: 0,
            hlc_to: 10,
            memory_deltas: vec![
                make_memory("private-1", ShareScope::Private),
                make_memory("paired-1", ShareScope::Paired),
            ],
            edge_deltas: vec![],
        };

        let filtered = filter_bundle(&bundle, BundleTarget::Paired).unwrap();
        assert_eq!(filtered.memory_deltas.len(), 1);
        assert_eq!(filtered.memory_deltas[0].content_hash, "paired-1");
    }

    #[test]
    fn paired_memory_excluded_from_hive_bundle() {
        let bundle = Bundle {
            bundle_id: "test".into(),
            hlc_from: 0,
            hlc_to: 10,
            memory_deltas: vec![
                make_memory("paired-1", ShareScope::Paired),
                make_memory("hive-1", ShareScope::Hive),
            ],
            edge_deltas: vec![],
        };

        let filtered = filter_bundle(&bundle, BundleTarget::Hive).unwrap();
        assert_eq!(filtered.memory_deltas.len(), 1);
        assert_eq!(filtered.memory_deltas[0].content_hash, "hive-1");
    }

    #[test]
    fn edge_excluded_when_src_is_private() {
        let bundle = Bundle {
            bundle_id: "test".into(),
            hlc_from: 0,
            hlc_to: 10,
            memory_deltas: vec![
                make_memory("private-1", ShareScope::Private),
                make_memory("hive-1", ShareScope::Hive),
            ],
            edge_deltas: vec![make_edge("private-1", "hive-1")],
        };

        let filtered = filter_bundle(&bundle, BundleTarget::Hive).unwrap();
        assert_eq!(filtered.memory_deltas.len(), 1);
        assert!(
            filtered.edge_deltas.is_empty(),
            "Edge referencing private memory must be excluded"
        );
    }

    #[test]
    fn edge_excluded_when_dst_is_private() {
        let bundle = Bundle {
            bundle_id: "test".into(),
            hlc_from: 0,
            hlc_to: 10,
            memory_deltas: vec![
                make_memory("hive-1", ShareScope::Hive),
                make_memory("private-1", ShareScope::Private),
            ],
            edge_deltas: vec![make_edge("hive-1", "private-1")],
        };

        let filtered = filter_bundle(&bundle, BundleTarget::Hive).unwrap();
        assert_eq!(filtered.memory_deltas.len(), 1);
        assert!(filtered.edge_deltas.is_empty());
    }

    #[test]
    fn edge_included_when_both_endpoints_are_hive() {
        let bundle = Bundle {
            bundle_id: "test".into(),
            hlc_from: 0,
            hlc_to: 10,
            memory_deltas: vec![
                make_memory("hive-1", ShareScope::Hive),
                make_memory("hive-2", ShareScope::Hive),
            ],
            edge_deltas: vec![make_edge("hive-1", "hive-2")],
        };

        let filtered = filter_bundle(&bundle, BundleTarget::Hive).unwrap();
        assert_eq!(filtered.memory_deltas.len(), 2);
        assert_eq!(filtered.edge_deltas.len(), 1);
    }

    #[test]
    fn all_private_returns_none() {
        let bundle = Bundle {
            bundle_id: "test".into(),
            hlc_from: 0,
            hlc_to: 10,
            memory_deltas: vec![
                make_memory("private-1", ShareScope::Private),
                make_memory("private-2", ShareScope::Private),
            ],
            edge_deltas: vec![make_edge("private-1", "private-2")],
        };

        assert!(filter_bundle(&bundle, BundleTarget::Hive).is_none());
    }

    #[test]
    fn hive_memory_passes_paired_filter() {
        // Hive-scoped memories are MORE open than paired, so they should pass a paired filter
        let bundle = Bundle {
            bundle_id: "test".into(),
            hlc_from: 0,
            hlc_to: 10,
            memory_deltas: vec![make_memory("hive-1", ShareScope::Hive)],
            edge_deltas: vec![],
        };

        let filtered = filter_bundle(&bundle, BundleTarget::Paired).unwrap();
        assert_eq!(filtered.memory_deltas.len(), 1);
    }

    #[test]
    fn default_scope_for_personal_is_private() {
        assert_eq!(
            default_scope_for_kind(Some("personal")),
            ShareScope::Private
        );
        assert_eq!(
            default_scope_for_kind(Some("episodic")),
            ShareScope::Private
        );
        assert_eq!(
            default_scope_for_kind(Some("preference")),
            ShareScope::Private
        );
    }

    #[test]
    fn default_scope_for_factual_is_paired() {
        assert_eq!(default_scope_for_kind(Some("semantic")), ShareScope::Paired);
        assert_eq!(
            default_scope_for_kind(Some("procedural")),
            ShareScope::Paired
        );
        assert_eq!(default_scope_for_kind(Some("factual")), ShareScope::Paired);
    }

    #[test]
    fn default_scope_for_unknown_is_private() {
        assert_eq!(default_scope_for_kind(None), ShareScope::Private);
        assert_eq!(
            default_scope_for_kind(Some("unknown_kind")),
            ShareScope::Private
        );
    }

    #[test]
    fn apply_default_scopes_uses_overrides() {
        let mut deltas = vec![
            make_memory("m1", ShareScope::Private),
            make_memory("m2", ShareScope::Private),
        ];
        deltas[0].cognitive_kind = Some("factual".into());
        deltas[1].cognitive_kind = Some("factual".into());

        let mut overrides = HashMap::new();
        overrides.insert("factual".into(), ShareScope::Hive);

        apply_default_scopes(&mut deltas, &overrides);

        assert_eq!(deltas[0].share_scope, ShareScope::Hive);
        assert_eq!(deltas[1].share_scope, ShareScope::Hive);
    }

    #[test]
    fn apply_default_scopes_preserves_explicit_non_private() {
        let mut deltas = vec![make_memory("m1", ShareScope::Hive)];
        deltas[0].cognitive_kind = Some("personal".into()); // Would default to Private

        let overrides = HashMap::new();
        apply_default_scopes(&mut deltas, &overrides);

        // Should NOT downgrade an explicitly-set scope
        assert_eq!(deltas[0].share_scope, ShareScope::Hive);
    }
}
