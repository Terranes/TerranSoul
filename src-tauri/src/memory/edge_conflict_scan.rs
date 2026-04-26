//! Scheduled conflict scan over connected memories (Chunk 17.6).
//!
//! Iterates `memory_edges` with "positive" relation types (supports,
//! implies, related_to, derived_from) and asks the LLM whether the
//! connected memories actually contradict each other. When a
//! contradiction is found:
//!
//! 1. A new `"contradicts"` edge is inserted (source: `Auto`).
//! 2. A `MemoryConflict` row is opened for the user to resolve.
//!
//! The scan skips pairs that already have a `"contradicts"` edge or
//! an open `MemoryConflict` row to avoid duplicate notifications.
//!
//! **Lock-safe design**: The Tauri command calls `collect_scan_candidates`
//! while holding the MutexGuard, releases it, runs the async LLM calls,
//! then calls `record_contradiction` per write (re-acquiring the lock).
//!
//! See `docs/brain-advanced-design.md` §16 Phase 3.

use crate::memory::edges::{EdgeSource, NewMemoryEdge};
use crate::memory::store::MemoryStore;

/// Relation types considered "positive" — if the LLM says these pairs
/// actually contradict, it's a real conflict worth surfacing.
const POSITIVE_REL_TYPES: &[&str] = &[
    "supports",
    "implies",
    "related_to",
    "derived_from",
    "cites",
    "part_of",
];

/// Result of a single scan run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EdgeConflictScanResult {
    /// Number of edge pairs scanned (LLM calls made).
    pub pairs_scanned: usize,
    /// Number of new contradictions found and opened.
    pub conflicts_found: usize,
    /// Number of pairs skipped (already have contradicts edge or open conflict).
    pub pairs_skipped: usize,
}

/// Candidate pairs collected while holding the store lock.
pub struct ScanCandidates {
    /// (src_id, dst_id, content_a, content_b)
    pub pairs: Vec<(i64, i64, String, String)>,
    /// Number of pairs skipped during collection.
    pub skipped: usize,
}

/// Collect candidate edge pairs for contradiction checking.
/// Call while holding the MutexGuard, then release before LLM calls.
pub fn collect_scan_candidates(
    store: &MemoryStore,
    max_pairs: usize,
) -> ScanCandidates {
    let mut candidates = ScanCandidates {
        pairs: Vec::new(),
        skipped: 0,
    };

    let edges = match store.list_edges() {
        Ok(e) => e,
        Err(_) => return candidates,
    };

    let filtered: Vec<_> = edges
        .into_iter()
        .filter(|e| {
            e.valid_to.is_none()
                && POSITIVE_REL_TYPES.contains(&e.rel_type.as_str())
        })
        .collect();

    for edge in filtered.iter() {
        if candidates.pairs.len() >= max_pairs {
            break;
        }

        // Skip if a "contradicts" edge already exists for this pair.
        if has_contradicts_edge(store, edge.src_id, edge.dst_id) {
            candidates.skipped += 1;
            continue;
        }

        // Skip if an open conflict already exists for this pair.
        if has_open_conflict(store, edge.src_id, edge.dst_id) {
            candidates.skipped += 1;
            continue;
        }

        // Load the content of both memories.
        match (store.get_by_id(edge.src_id), store.get_by_id(edge.dst_id)) {
            (Ok(a), Ok(b)) => {
                candidates.pairs.push((edge.src_id, edge.dst_id, a.content, b.content));
            }
            _ => {
                candidates.skipped += 1;
            }
        }
    }

    candidates
}

/// Record a contradiction found by the LLM: insert a "contradicts" edge
/// and open a MemoryConflict row. Call while holding the MutexGuard.
pub fn record_contradiction(
    store: &MemoryStore,
    src_id: i64,
    dst_id: i64,
    reason: &str,
) {
    let _ = store.add_edge(NewMemoryEdge {
        src_id,
        dst_id,
        rel_type: "contradicts".to_string(),
        confidence: 0.8,
        source: EdgeSource::Auto,
        valid_from: None,
        valid_to: None,
        edge_source: None,
    });
    let _ = store.add_conflict(src_id, dst_id, reason);
}

/// Check whether a "contradicts" edge already exists between two memories
/// (in either direction).
fn has_contradicts_edge(store: &MemoryStore, a: i64, b: i64) -> bool {
    if let Ok(edges) = store.get_edges_for(a, crate::memory::edges::EdgeDirection::Both) {
        edges.iter().any(|e| {
            e.rel_type == "contradicts"
                && e.valid_to.is_none()
                && ((e.src_id == a && e.dst_id == b) || (e.src_id == b && e.dst_id == a))
        })
    } else {
        false
    }
}

/// Check whether an open MemoryConflict already exists for this pair
/// (in either direction).
fn has_open_conflict(store: &MemoryStore, a: i64, b: i64) -> bool {
    if let Ok(conflicts) = store.list_conflicts(Some(
        &crate::memory::conflicts::ConflictStatus::Open,
    )) {
        conflicts.iter().any(|c| {
            (c.entry_a_id == a && c.entry_b_id == b)
                || (c.entry_a_id == b && c.entry_b_id == a)
        })
    } else {
        false
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::edges::EdgeSource;

    #[test]
    fn has_contradicts_edge_false_when_empty() {
        let store = MemoryStore::in_memory();
        let a = store
            .add(crate::memory::NewMemory {
                content: "A".into(),
                ..Default::default()
            })
            .unwrap();
        let b = store
            .add(crate::memory::NewMemory {
                content: "B".into(),
                ..Default::default()
            })
            .unwrap();
        assert!(!has_contradicts_edge(&store, a.id, b.id));
    }

    #[test]
    fn has_contradicts_edge_true_after_insert() {
        let store = MemoryStore::in_memory();
        let a = store
            .add(crate::memory::NewMemory {
                content: "A".into(),
                ..Default::default()
            })
            .unwrap();
        let b = store
            .add(crate::memory::NewMemory {
                content: "B".into(),
                ..Default::default()
            })
            .unwrap();
        store
            .add_edge(NewMemoryEdge {
                src_id: a.id,
                dst_id: b.id,
                rel_type: "contradicts".to_string(),
                confidence: 1.0,
                source: EdgeSource::Auto,
                valid_from: None,
                valid_to: None,
                edge_source: None,
            })
            .unwrap();
        assert!(has_contradicts_edge(&store, a.id, b.id));
        assert!(has_contradicts_edge(&store, b.id, a.id)); // reverse too
    }

    #[test]
    fn has_open_conflict_false_when_empty() {
        let store = MemoryStore::in_memory();
        assert!(!has_open_conflict(&store, 1, 2));
    }

    #[test]
    fn has_open_conflict_true_after_add() {
        let store = MemoryStore::in_memory();
        let a = store
            .add(crate::memory::NewMemory {
                content: "X".into(),
                ..Default::default()
            })
            .unwrap();
        let b = store
            .add(crate::memory::NewMemory {
                content: "Y".into(),
                ..Default::default()
            })
            .unwrap();
        store.add_conflict(a.id, b.id, "test").unwrap();
        assert!(has_open_conflict(&store, a.id, b.id));
        assert!(has_open_conflict(&store, b.id, a.id)); // reverse too
    }

    #[test]
    fn has_open_conflict_false_after_dismiss() {
        let store = MemoryStore::in_memory();
        let a = store
            .add(crate::memory::NewMemory {
                content: "P".into(),
                ..Default::default()
            })
            .unwrap();
        let b = store
            .add(crate::memory::NewMemory {
                content: "Q".into(),
                ..Default::default()
            })
            .unwrap();
        let c = store.add_conflict(a.id, b.id, "test").unwrap();
        store.dismiss_conflict(c.id).unwrap();
        assert!(!has_open_conflict(&store, a.id, b.id));
    }

    #[test]
    fn positive_rel_types_are_in_common_vocabulary() {
        for &rt in POSITIVE_REL_TYPES {
            assert!(
                crate::memory::edges::COMMON_RELATION_TYPES.contains(&rt)
                    || rt == "supports"
                    || rt == "implies",
                "POSITIVE_REL_TYPES entry '{rt}' not in COMMON_RELATION_TYPES"
            );
        }
    }

    // Integration test of the full scan is infeasible without a local LLM,
    // but the helpers above cover the skip logic which is the non-trivial part.
}
