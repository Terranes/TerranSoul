//! GitNexus → TerranSoul knowledge-graph mirror (Phase 13 Tier 3).
//!
//! Mirrors the structured knowledge graph that the GitNexus sidecar
//! (Phase 13 Tier 1) builds for an indexed repository into TerranSoul's
//! [`crate::memory::MemoryStore`] so the brain's existing graph
//! traversal (`hybrid_search_with_graph`, `traverse_from`) and the
//! BrainView graph panel can reason over code structure alongside
//! free-text memories.
//!
//! ## Provenance
//!
//! Every edge inserted by [`mirror_kg`] carries an `edge_source` of
//! the form `gitnexus:<scope>` where `<scope>` is the caller-supplied
//! repo label (typically `repo:<owner>/<name>@<sha>`). This lets
//! [`unmirror`] cleanly remove every edge associated with a single
//! sync without touching native or LLM-extracted edges.
//!
//! ## Relation taxonomy mapping
//!
//! GitNexus emits a fixed vocabulary of structural relations. We
//! translate them into the existing 17-relation taxonomy defined by
//! [`crate::memory::edges::COMMON_RELATION_TYPES`] so the UI can keep
//! a single colour/legend system:
//!
//! | GitNexus | TerranSoul |
//! |----------|-----------|
//! | `CONTAINS`       | `contains`     |
//! | `CALLS`          | `depends_on`   |
//! | `IMPORTS`        | `depends_on`   |
//! | `EXTENDS`        | `derived_from` |
//! | `HANDLES_ROUTE`  | `governs`      |
//!
//! Relations outside this set are accepted in lower-snake-case form
//! and pass through normalisation. Future GitNexus versions can add
//! new relation labels without breaking the mirror — they will simply
//! land in the graph under their normalised name.
//!
//! ## Strictly opt-in
//!
//! No code in this module runs at startup. The frontend explicitly
//! invokes the `gitnexus_sync` Tauri command with a repo label, and
//! the mirror is written transactionally into the SQLite memory store
//! by [`mirror_kg`].

use serde::{Deserialize, Serialize};

use super::edges::{normalise_rel_type, EdgeSource, NewMemoryEdge};
use super::store::{MemoryStore, MemoryType, NewMemory};

/// Prefix used in `memory_edges.edge_source` to identify GitNexus mirrors.
pub const GITNEXUS_EDGE_SOURCE_PREFIX: &str = "gitnexus:";

/// Build the canonical `edge_source` value for a given repo scope.
///
/// `scope` is intentionally free-form — typical formats are
/// `repo:owner/name@sha` or `repo:owner/name`. The result is what
/// [`mirror_kg`] writes and what [`unmirror`] matches against.
pub fn edge_source_for(scope: &str) -> String {
    format!("{GITNEXUS_EDGE_SOURCE_PREFIX}{scope}")
}

/// One node in the GitNexus knowledge graph as exposed over MCP.
///
/// We keep the schema deliberately permissive: GitNexus tags nodes
/// with rich metadata (path, language, kind, …) but we only need a
/// stable identifier and a human-readable label to mirror them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgNode {
    /// GitNexus-internal node id (e.g. `n:1234`). Stable within one
    /// repo + sha.
    pub id: String,
    /// Human-readable label — typically the symbol's qualified name
    /// (`module::path::Function`) or file path.
    #[serde(default)]
    pub label: String,
    /// Optional node kind (e.g. `function`, `class`, `module`,
    /// `route`). Stored verbatim in the memory entry's tags.
    #[serde(default)]
    pub kind: Option<String>,
}

/// One edge in the GitNexus knowledge graph as exposed over MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgEdge {
    pub src: String,
    pub dst: String,
    /// Upstream relation label (e.g. `CONTAINS`, `CALLS`, `IMPORTS`).
    #[serde(rename = "type", alias = "rel_type", alias = "relation")]
    pub rel_type: String,
}

/// Full knowledge graph payload that GitNexus returns from its
/// `graph` MCP tool. The shape mirrors the upstream JSON.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KgPayload {
    #[serde(default)]
    pub nodes: Vec<KgNode>,
    #[serde(default)]
    pub edges: Vec<KgEdge>,
}

/// Summary returned by [`mirror_kg`] / [`unmirror`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MirrorReport {
    pub edge_source: String,
    pub nodes_inserted: usize,
    pub nodes_reused: usize,
    pub edges_inserted: usize,
    pub edges_skipped: usize,
}

/// Map a GitNexus relation label to a TerranSoul relation in the
/// existing 17-relation taxonomy.
///
/// Pure function — no I/O, no allocation beyond the result string.
/// Unknown / unmapped labels fall through `normalise_rel_type` so they
/// land in the graph under a sane snake_case name (and the UI can
/// treat them as "other"). This keeps the mapper forward-compatible
/// with future GitNexus releases that introduce new edge kinds.
pub fn map_relation(gitnexus_label: &str) -> String {
    // Match case-insensitively but normalise the comparison key
    // (GitNexus uses SCREAMING_SNAKE; we match against that).
    let upper = gitnexus_label.trim().to_ascii_uppercase();
    match upper.as_str() {
        "CONTAINS" => "contains".to_string(),
        "CALLS" => "depends_on".to_string(),
        "IMPORTS" => "depends_on".to_string(),
        "EXTENDS" => "derived_from".to_string(),
        "HANDLES_ROUTE" => "governs".to_string(),
        _ => normalise_rel_type(gitnexus_label),
    }
}

/// Memory tag prefix used for mirrored nodes. Lets the BrainView UI
/// filter / colour code-knowledge memories distinctly.
pub const NODE_TAG_PREFIX: &str = "code-graph";

/// Build the deterministic content payload stored on a mirrored node.
///
/// Embedding the GitNexus node id and the scope means the same logical
/// symbol from two different syncs becomes two separate memories — by
/// design, since the second sync may have updated the symbol's
/// definition. Re-running a sync against the same scope, however,
/// re-uses the existing memory because the content + source_hash
/// match.
fn node_content(scope: &str, node: &KgNode) -> String {
    let kind = node.kind.as_deref().unwrap_or("symbol");
    if node.label.is_empty() {
        format!("[{scope}] {kind} {}", node.id)
    } else {
        format!("[{scope}] {kind}: {}", node.label)
    }
}

fn node_tags(node: &KgNode) -> String {
    match &node.kind {
        Some(k) if !k.is_empty() => format!("{NODE_TAG_PREFIX},code-{}", k.to_ascii_lowercase()),
        _ => NODE_TAG_PREFIX.to_string(),
    }
}

/// Mirror a GitNexus knowledge-graph payload into the memory store.
///
/// `scope` should uniquely identify the indexed repo+revision (for
/// example `repo:owner/name@sha`); it becomes the suffix of every
/// inserted edge's `edge_source` so [`unmirror`] can later remove
/// exactly that mirror without touching anything else.
///
/// Idempotent: re-running with the same `scope` and an unchanged
/// payload re-uses existing memories (matched via the per-node
/// `source_hash`) and the unique `(src_id, dst_id, rel_type)` index
/// on `memory_edges` deduplicates edges. The returned [`MirrorReport`]
/// breaks down the inserted-vs-reused counts for UI feedback.
pub fn mirror_kg(
    store: &MemoryStore,
    scope: &str,
    payload: &KgPayload,
) -> Result<MirrorReport, String> {
    if scope.trim().is_empty() {
        return Err("mirror_kg: scope must be non-empty".to_string());
    }
    let edge_source = edge_source_for(scope);

    let mut report = MirrorReport {
        edge_source: edge_source.clone(),
        ..Default::default()
    };

    // Step 1 — upsert one memory entry per KG node and remember the
    // resulting database id for the edge insertion pass.
    let mut id_for: std::collections::HashMap<String, i64> =
        std::collections::HashMap::with_capacity(payload.nodes.len());

    for node in &payload.nodes {
        let content = node_content(scope, node);
        // Hash key = scope + node id, so two scopes that share a node
        // id (very unlikely, but possible in monorepos) don't collide.
        let source_hash_input = format!("{scope}|{}", node.id);
        let source_hash = format!("{:x}", stable_hash_64(&source_hash_input));

        // De-dup against existing memories with the same source_hash.
        let existing_id = store
            .find_by_source_hash(&source_hash)
            .map_err(|e| format!("mirror_kg: lookup by source_hash failed: {e}"))?
            .map(|m| m.id);

        let memory_id = if let Some(id) = existing_id {
            report.nodes_reused += 1;
            id
        } else {
            let new_mem = NewMemory {
                content,
                tags: node_tags(node),
                importance: 3,
                memory_type: MemoryType::Context,
                source_url: Some(edge_source.clone()),
                source_hash: Some(source_hash),
                expires_at: None,
            };
            let inserted = store
                .add(new_mem)
                .map_err(|e| format!("mirror_kg: insert node failed: {e}"))?;
            report.nodes_inserted += 1;
            inserted.id
        };
        id_for.insert(node.id.clone(), memory_id);
    }

    // Step 2 — translate every KG edge into a NewMemoryEdge and
    // batch-insert. Edges referencing nodes we never saw, self-loops,
    // and duplicates are silently skipped (the unique index handles
    // dupes; we count them in the report).
    let mut new_edges: Vec<NewMemoryEdge> = Vec::with_capacity(payload.edges.len());
    for e in &payload.edges {
        let (Some(&src_id), Some(&dst_id)) = (id_for.get(&e.src), id_for.get(&e.dst)) else {
            report.edges_skipped += 1;
            continue;
        };
        if src_id == dst_id {
            report.edges_skipped += 1;
            continue;
        }
        new_edges.push(NewMemoryEdge {
            src_id,
            dst_id,
            rel_type: map_relation(&e.rel_type),
            confidence: 1.0,
            source: EdgeSource::Auto,
            valid_from: None,
            valid_to: None,
            edge_source: Some(edge_source.clone()),
        });
    }

    let inserted = store
        .add_edges_batch(&new_edges)
        .map_err(|e| format!("mirror_kg: batch insert failed: {e}"))?;
    report.edges_inserted = inserted;
    report.edges_skipped += new_edges.len().saturating_sub(inserted);

    Ok(report)
}

/// Remove every edge that was inserted by a previous [`mirror_kg`]
/// call against the same `scope`. Returns the number of edges
/// deleted. Memory nodes themselves are left intact: they may have
/// accreted user-asserted edges or LLM-extracted edges that are still
/// valuable.
pub fn unmirror(store: &MemoryStore, scope: &str) -> Result<usize, String> {
    if scope.trim().is_empty() {
        return Err("unmirror: scope must be non-empty".to_string());
    }
    let edge_source = edge_source_for(scope);
    store
        .delete_edges_by_edge_source(&edge_source)
        .map_err(|e| format!("unmirror: {e}"))
}

/// Stable 64-bit non-cryptographic hash for `source_hash` dedup keys.
///
/// We deliberately avoid pulling in `sha2`/`md-5` for this — the hash
/// only needs to be stable within a single TerranSoul installation
/// (it's used as a string key for de-duplication, not for security).
/// The 64-bit `DefaultHasher` output rendered as hex is plenty.
fn stable_hash_64(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_relation_handles_known_labels() {
        assert_eq!(map_relation("CONTAINS"), "contains");
        assert_eq!(map_relation("CALLS"), "depends_on");
        assert_eq!(map_relation("IMPORTS"), "depends_on");
        assert_eq!(map_relation("EXTENDS"), "derived_from");
        assert_eq!(map_relation("HANDLES_ROUTE"), "governs");
    }

    #[test]
    fn map_relation_is_case_insensitive() {
        assert_eq!(map_relation("calls"), "depends_on");
        assert_eq!(map_relation("Contains"), "contains");
    }

    #[test]
    fn map_relation_normalises_unknown_labels() {
        // Unknown label flows through normalise_rel_type, so
        // "Refers To" lands as "refers_to" rather than blowing up.
        assert_eq!(map_relation("Refers To"), "refers_to");
        // Empty / whitespace falls back to "related_to" via normalisation.
        assert_eq!(map_relation(""), "related_to");
    }

    #[test]
    fn edge_source_includes_prefix_and_scope() {
        assert_eq!(
            edge_source_for("repo:foo/bar@abc"),
            "gitnexus:repo:foo/bar@abc"
        );
    }

    fn small_payload() -> KgPayload {
        KgPayload {
            nodes: vec![
                KgNode { id: "n1".into(), label: "module::A".into(), kind: Some("module".into()) },
                KgNode { id: "n2".into(), label: "module::A::foo".into(), kind: Some("function".into()) },
                KgNode { id: "n3".into(), label: "module::B::bar".into(), kind: Some("function".into()) },
            ],
            edges: vec![
                KgEdge { src: "n1".into(), dst: "n2".into(), rel_type: "CONTAINS".into() },
                KgEdge { src: "n2".into(), dst: "n3".into(), rel_type: "CALLS".into() },
                // Self-loop — must be skipped.
                KgEdge { src: "n3".into(), dst: "n3".into(), rel_type: "CALLS".into() },
                // Dangling reference — must be skipped.
                KgEdge { src: "missing".into(), dst: "n2".into(), rel_type: "CALLS".into() },
            ],
        }
    }

    #[test]
    fn mirror_kg_inserts_nodes_and_edges() {
        let store = MemoryStore::in_memory();
        let payload = small_payload();
        let report = mirror_kg(&store, "repo:foo/bar@abc", &payload).unwrap();
        assert_eq!(report.nodes_inserted, 3);
        assert_eq!(report.nodes_reused, 0);
        assert_eq!(report.edges_inserted, 2, "self-loop + dangling skipped");
        assert_eq!(report.edges_skipped, 2);

        let edges = store.list_edges().unwrap();
        assert_eq!(edges.len(), 2);
        let rels: Vec<&str> = edges.iter().map(|e| e.rel_type.as_str()).collect();
        assert!(rels.contains(&"contains"));
        assert!(rels.contains(&"depends_on"));
        for e in &edges {
            assert_eq!(
                e.edge_source.as_deref(),
                Some("gitnexus:repo:foo/bar@abc"),
                "every mirrored edge must carry the scoped edge_source"
            );
            assert_eq!(e.source, EdgeSource::Auto);
        }
    }

    #[test]
    fn mirror_kg_is_idempotent() {
        let store = MemoryStore::in_memory();
        let payload = small_payload();
        let r1 = mirror_kg(&store, "repo:foo/bar@abc", &payload).unwrap();
        let r2 = mirror_kg(&store, "repo:foo/bar@abc", &payload).unwrap();
        assert_eq!(r1.nodes_inserted, 3);
        assert_eq!(r2.nodes_inserted, 0);
        assert_eq!(r2.nodes_reused, 3, "second run must reuse all nodes");
        // Edge unique index dedups: second run inserts 0 new edges.
        assert_eq!(r2.edges_inserted, 0);
        assert_eq!(store.list_edges().unwrap().len(), 2);
    }

    #[test]
    fn unmirror_removes_only_scoped_edges() {
        let store = MemoryStore::in_memory();
        let payload = small_payload();
        mirror_kg(&store, "repo:foo/bar@abc", &payload).unwrap();
        // Add a native (non-mirrored) edge between the same two nodes
        // to prove unmirror leaves it alone.
        let edges_before = store.list_edges().unwrap();
        let (s, d) = (edges_before[0].src_id, edges_before[0].dst_id);
        store
            .add_edge(NewMemoryEdge {
                src_id: s,
                dst_id: d,
                rel_type: "user_says_related".into(),
                confidence: 1.0,
                source: EdgeSource::User,
                valid_from: None,
                valid_to: None,
                edge_source: None,
            })
            .unwrap();
        assert_eq!(store.list_edges().unwrap().len(), 3);

        let removed = unmirror(&store, "repo:foo/bar@abc").unwrap();
        assert_eq!(removed, 2);
        let after = store.list_edges().unwrap();
        assert_eq!(after.len(), 1, "only the native edge survives");
        assert_eq!(after[0].source, EdgeSource::User);
        assert!(after[0].edge_source.is_none());
    }

    #[test]
    fn unmirror_other_scope_is_a_noop() {
        let store = MemoryStore::in_memory();
        let payload = small_payload();
        mirror_kg(&store, "repo:foo/bar@abc", &payload).unwrap();
        let removed = unmirror(&store, "repo:other/repo@xyz").unwrap();
        assert_eq!(removed, 0);
        assert_eq!(store.list_edges().unwrap().len(), 2);
    }

    #[test]
    fn empty_scope_is_rejected() {
        let store = MemoryStore::in_memory();
        assert!(mirror_kg(&store, "", &KgPayload::default()).is_err());
        assert!(mirror_kg(&store, "   ", &KgPayload::default()).is_err());
        assert!(unmirror(&store, "").is_err());
    }

    #[test]
    fn relation_field_accepts_aliases() {
        // GitNexus implementations sometimes use `rel_type` or
        // `relation` instead of `type` in their JSON.
        let raw = r#"{
            "nodes": [
                {"id": "n1", "label": "x", "kind": "module"},
                {"id": "n2", "label": "y", "kind": "function"}
            ],
            "edges": [
                {"src": "n1", "dst": "n2", "rel_type": "CONTAINS"}
            ]
        }"#;
        let payload: KgPayload = serde_json::from_str(raw).unwrap();
        assert_eq!(payload.edges.len(), 1);
        assert_eq!(payload.edges[0].rel_type, "CONTAINS");
    }
}
