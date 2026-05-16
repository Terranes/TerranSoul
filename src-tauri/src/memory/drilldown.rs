//! Provenance drill-down — walk `derived_from` edges back to source memories
//! (MEM-DRILLDOWN-1, inspired by Tencent/TencentDB-Agent-Memory).
//!
//! Convention (already established by [`crate::memory::consolidation`] and
//! [`crate::memory::audit`]): a `derived_from` edge has
//! `src_id = summary / parent / aggregated memory` and
//! `dst_id = the source memory it was distilled from`. So to find the
//! ancestors of a summary `M`, we walk `memory_edges` rows where
//! `src_id = M` and `rel_type = 'derived_from'` — the dst_ids are the
//! immediate parents. The traversal recurses on each parent up to
//! `max_depth` hops.
//!
//! Recall is the inverse direction TerranSoul is used to (cascade-expand
//! reads both directions for query expansion). This module is the
//! deterministic provenance walk a UI / agent uses to *audit* a summary,
//! not a retrieval expansion.

use std::collections::{HashSet, VecDeque};

use rusqlite::{params, Result as SqlResult};
use serde::{Deserialize, Serialize};

use crate::memory::store::MemoryStore;
use crate::memory::MemoryEntry;

/// Default depth cap for `source_chain`. Large enough for realistic
/// summary-of-summary trees, small enough to prevent runaway traversal
/// on accidentally cyclic edges (although `derived_from` should never be
/// cyclic by construction).
pub const DEFAULT_MAX_DEPTH: usize = 8;

/// One ancestor in a drill-down chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceAncestor {
    /// Hop distance from the query memory. 1 = immediate parent.
    pub depth: usize,
    /// Confidence on the `derived_from` edge that led to this ancestor.
    /// When the same ancestor is reachable via multiple paths, the
    /// **highest** confidence is reported.
    pub edge_confidence: f64,
    /// The ancestor memory itself.
    pub memory: MemoryEntry,
}

/// Full drill-down result for `MemoryStore::source_chain`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceChain {
    /// The starting memory the caller asked about.
    pub root: MemoryEntry,
    /// Ancestors ordered by `(depth ASC, id ASC)`.
    pub ancestors: Vec<SourceAncestor>,
    /// True iff the BFS was cut off at `max_depth` and there may be
    /// more ancestors beyond it.
    pub truncated: bool,
}

impl MemoryStore {
    /// BFS-walk `derived_from` edges OUT-from `memory_id` and return the
    /// full chain of source memories with their depth from `memory_id`.
    ///
    /// `max_depth = None` uses [`DEFAULT_MAX_DEPTH`]. Pass `Some(1)` to
    /// fetch only the immediate parents.
    ///
    /// Errors propagate from SQLite. Cycles (should never exist for
    /// `derived_from` but we defend anyway) are broken by a visited set.
    pub fn source_chain(
        &self,
        memory_id: i64,
        max_depth: Option<usize>,
    ) -> SqlResult<SourceChain> {
        let root = self.get_by_id(memory_id)?;
        let depth_limit = max_depth.unwrap_or(DEFAULT_MAX_DEPTH).max(1);

        let conn = self.conn();
        // (current_node, hop_from_root).
        let mut queue: VecDeque<(i64, usize)> = VecDeque::new();
        let mut visited: HashSet<i64> = HashSet::new();
        visited.insert(memory_id);
        queue.push_back((memory_id, 0));

        // ancestor_id -> (best_depth, best_edge_confidence)
        let mut best: std::collections::HashMap<i64, (usize, f64)> =
            std::collections::HashMap::new();
        let mut truncated = false;

        let mut stmt = conn.prepare(
            "SELECT dst_id, confidence
             FROM memory_edges
             WHERE src_id = ?1
               AND rel_type = 'derived_from'
               AND valid_to IS NULL",
        )?;

        while let Some((node, depth)) = queue.pop_front() {
            if depth >= depth_limit {
                // We popped a node at the limit; we won't expand it. If
                // it actually has outgoing `derived_from` edges, the
                // chain is truncated.
                let has_more: bool = conn
                    .query_row(
                        "SELECT EXISTS(
                            SELECT 1 FROM memory_edges
                            WHERE src_id = ?1
                              AND rel_type = 'derived_from'
                              AND valid_to IS NULL
                        )",
                        params![node],
                        |row| row.get::<_, bool>(0),
                    )
                    .unwrap_or(false);
                if has_more {
                    truncated = true;
                }
                continue;
            }

            let rows = stmt.query_map(params![node], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
            })?;

            for r in rows.flatten() {
                let (parent_id, conf) = r;
                // Defensive: a `derived_from` cycle would otherwise let
                // the root re-appear in its own ancestor list. Skip it.
                if parent_id == memory_id {
                    continue;
                }
                let next_depth = depth + 1;
                // Track best score for this ancestor.
                best.entry(parent_id)
                    .and_modify(|(d, c)| {
                        if conf > *c {
                            *c = conf;
                        }
                        if next_depth < *d {
                            *d = next_depth;
                        }
                    })
                    .or_insert((next_depth, conf));

                if visited.insert(parent_id) {
                    queue.push_back((parent_id, next_depth));
                }
            }
        }

        // Materialise ancestors. Use a single pass for efficiency on
        // wide chains.
        let mut ancestor_ids: Vec<i64> = best.keys().copied().collect();
        ancestor_ids.sort_unstable();

        let mut ancestors: Vec<SourceAncestor> = Vec::with_capacity(ancestor_ids.len());
        for id in &ancestor_ids {
            if let Ok(memory) = self.get_by_id(*id) {
                let (depth, edge_confidence) = best[id];
                ancestors.push(SourceAncestor {
                    depth,
                    edge_confidence,
                    memory,
                });
            }
        }

        ancestors.sort_by(|a, b| a.depth.cmp(&b.depth).then_with(|| a.memory.id.cmp(&b.memory.id)));

        Ok(SourceChain {
            root,
            ancestors,
            truncated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::edges::{EdgeSource, NewMemoryEdge};
    use crate::memory::{MemoryType, NewMemory};

    fn make_memory(store: &MemoryStore, content: &str) -> i64 {
        store
            .add(NewMemory {
                content: content.to_string(),
                tags: "test".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap()
            .id
    }

    fn link_derived(store: &MemoryStore, parent: i64, child: i64, confidence: f64) {
        store
            .add_edge(NewMemoryEdge {
                src_id: parent,
                dst_id: child,
                rel_type: "derived_from".to_string(),
                confidence,
                source: EdgeSource::Auto,
                valid_from: None,
                valid_to: None,
                edge_source: Some("test".to_string()),
            })
            .unwrap();
    }

    #[test]
    fn source_chain_with_no_edges_returns_only_root() {
        let store = MemoryStore::in_memory();
        let m = make_memory(&store, "lonely");
        let chain = store.source_chain(m, None).unwrap();
        assert_eq!(chain.root.id, m);
        assert!(chain.ancestors.is_empty());
        assert!(!chain.truncated);
    }

    #[test]
    fn source_chain_walks_immediate_parents() {
        let store = MemoryStore::in_memory();
        let summary = make_memory(&store, "summary");
        let src_a = make_memory(&store, "source A");
        let src_b = make_memory(&store, "source B");
        link_derived(&store, summary, src_a, 0.9);
        link_derived(&store, summary, src_b, 0.95);

        let chain = store.source_chain(summary, None).unwrap();
        assert_eq!(chain.ancestors.len(), 2);
        assert!(chain
            .ancestors
            .iter()
            .all(|a| a.depth == 1));
        let ids: Vec<i64> = chain.ancestors.iter().map(|a| a.memory.id).collect();
        assert!(ids.contains(&src_a));
        assert!(ids.contains(&src_b));
        assert!(!chain.truncated);
    }

    #[test]
    fn source_chain_walks_multi_hop_in_correct_order() {
        let store = MemoryStore::in_memory();
        // L3 persona -> L2 scenario -> L1 atom -> L0 conversation
        let l3 = make_memory(&store, "persona");
        let l2 = make_memory(&store, "scenario");
        let l1 = make_memory(&store, "atom");
        let l0 = make_memory(&store, "conversation");
        link_derived(&store, l3, l2, 0.9);
        link_derived(&store, l2, l1, 0.85);
        link_derived(&store, l1, l0, 0.8);

        let chain = store.source_chain(l3, None).unwrap();
        assert_eq!(chain.ancestors.len(), 3);
        // Depth ordering must be 1,2,3.
        assert_eq!(chain.ancestors[0].depth, 1);
        assert_eq!(chain.ancestors[0].memory.id, l2);
        assert_eq!(chain.ancestors[1].depth, 2);
        assert_eq!(chain.ancestors[1].memory.id, l1);
        assert_eq!(chain.ancestors[2].depth, 3);
        assert_eq!(chain.ancestors[2].memory.id, l0);
        assert!(!chain.truncated);
    }

    #[test]
    fn source_chain_truncated_flag_when_depth_exceeds_limit() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "a");
        let b = make_memory(&store, "b");
        let c = make_memory(&store, "c");
        link_derived(&store, a, b, 0.9);
        link_derived(&store, b, c, 0.9);

        // max_depth=1 should only see b, not c, and mark truncated.
        let chain = store.source_chain(a, Some(1)).unwrap();
        assert_eq!(chain.ancestors.len(), 1);
        assert_eq!(chain.ancestors[0].memory.id, b);
        assert!(chain.truncated);
    }

    #[test]
    fn source_chain_ignores_unrelated_rel_types() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "a");
        let b = make_memory(&store, "b");
        // related_to is NOT derived_from — drill-down must skip it.
        store
            .add_edge(NewMemoryEdge {
                src_id: a,
                dst_id: b,
                rel_type: "related_to".to_string(),
                confidence: 1.0,
                source: EdgeSource::Auto,
                valid_from: None,
                valid_to: None,
                edge_source: None,
            })
            .unwrap();

        let chain = store.source_chain(a, None).unwrap();
        assert!(chain.ancestors.is_empty());
    }

    #[test]
    fn source_chain_breaks_cycles_gracefully() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "a");
        let b = make_memory(&store, "b");
        // Defensive: even though `derived_from` should never be cyclic,
        // verify the BFS terminates if a buggy migration creates one.
        link_derived(&store, a, b, 0.9);
        link_derived(&store, b, a, 0.9);

        let chain = store.source_chain(a, Some(8)).unwrap();
        // We should see b exactly once and not loop back to a.
        assert_eq!(chain.ancestors.len(), 1);
        assert_eq!(chain.ancestors[0].memory.id, b);
    }

    #[test]
    fn source_chain_diamond_picks_best_edge_confidence() {
        let store = MemoryStore::in_memory();
        // Diamond: a derives from b1 and b2; both derive from c.
        let a = make_memory(&store, "a");
        let b1 = make_memory(&store, "b1");
        let b2 = make_memory(&store, "b2");
        let c = make_memory(&store, "c");
        link_derived(&store, a, b1, 0.6);
        link_derived(&store, a, b2, 0.9);
        link_derived(&store, b1, c, 0.5);
        link_derived(&store, b2, c, 0.95);

        let chain = store.source_chain(a, None).unwrap();
        let c_entry = chain
            .ancestors
            .iter()
            .find(|x| x.memory.id == c)
            .expect("c should appear");
        // Best edge confidence into c is 0.95 (via b2).
        assert!((c_entry.edge_confidence - 0.95).abs() < 1e-9);
        assert_eq!(c_entry.depth, 2);
    }
}
