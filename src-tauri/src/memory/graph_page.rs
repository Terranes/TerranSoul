//! Paged knowledge-graph view (Phase 1 of billion-scale plan).
//!
//! At billion-node scale the frontend cannot hold the whole graph in
//! memory and the screen cannot render it. This module exposes a paged,
//! level-of-detail (LOD) view of the memory knowledge graph:
//!
//! * **Overview zoom** — collapses every memory into a small fixed set of
//!   supernodes (one per `cognitive_kind`) plus aggregated edge weights.
//! * **Cluster zoom** — supernodes for off-focus kinds, real nodes for the
//!   focused kind. Used when the user has selected a category.
//! * **Detail zoom** — real nodes ranked by `degree desc, importance desc,
//!   recency desc`, capped at `limit`. Used near a focus node.
//!
//! The backend never returns more than `limit` nodes (hard-capped at
//! [`MAX_GRAPH_NODES`]). The frontend's viewport / focus changes drive new
//! requests; nothing ever streams the full graph.
//!
//! The pure entry point [`build_graph_page`] takes plain `Vec<MemoryEntry>`
//! / `Vec<MemoryEdge>` inputs so it can be unit-tested without Tauri state.
//! The Tauri command lives in `commands/memory.rs::memory_graph_page`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::cognitive_kind::{classify as classify_cognitive_kind, CognitiveKind};
use super::edges::MemoryEdge;
use super::store::{MemoryEntry, MemoryTier};

/// Default page size when the caller does not specify one.
pub const DEFAULT_GRAPH_LIMIT: usize = 2_000;
/// Hard upper bound. Anything beyond this is silently truncated.
pub const MAX_GRAPH_NODES: usize = 10_000;

/// LOD level for the graph page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GraphZoom {
    /// One node per `cognitive_kind`, weighted edges between kinds.
    Overview,
    /// Real nodes within the focused kind, supernodes for the rest.
    Cluster,
    /// Real nodes only, no supernodes.
    #[default]
    Detail,
}

/// Request shape for [`build_graph_page`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphPageRequest {
    /// Optional anchor memory id. Detail zoom prioritises this node and
    /// its 1-hop neighbours.
    #[serde(default)]
    pub focus_id: Option<i64>,
    /// Optional anchor cognitive kind (overrides classification of
    /// `focus_id` when both are set).
    #[serde(default)]
    pub focus_kind: Option<CognitiveKind>,
    /// LOD level.
    #[serde(default)]
    pub zoom: GraphZoom,
    /// Max nodes in the response. `None` means [`DEFAULT_GRAPH_LIMIT`].
    /// Hard-capped at [`MAX_GRAPH_NODES`].
    #[serde(default)]
    pub limit: Option<usize>,
}

/// A node in the paged graph response. Either a real memory or a
/// supernode that aggregates many memories of the same kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphNode {
    /// Stable string id. Real memories use `"m-{id}"`. Supernodes use
    /// `"super-{kind}"`. Strings keep frontend code simple at scale.
    pub id: String,
    pub label: String,
    /// `cognitive_kind` as a string (`"episodic"`, `"semantic"`, …).
    pub kind: String,
    /// `tier` as a string (`"short"`, `"working"`, `"long"`). Empty for
    /// supernodes.
    pub tier: String,
    /// 1–5 for real nodes, average importance for supernodes.
    pub importance: i64,
    /// In + out degree at the time of the page.
    pub degree: i64,
    pub is_supernode: bool,
    /// Number of underlying memories. `1` for real nodes.
    pub count: i64,
}

/// An edge in the paged graph response. May be an aggregated super-edge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    /// Relation type. Aggregated edges use `"aggregated"`.
    pub rel_type: String,
    /// `1.0` for real edges, or the count of underlying edges for
    /// aggregated super-edges.
    pub weight: f64,
}

/// Response from [`build_graph_page`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphPageResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    /// Total memory count before paging. Lets the UI show
    /// `"showing 2 000 of N"` honestly.
    pub total_nodes: i64,
    /// Total edge count before paging.
    pub total_edges: i64,
    /// `true` when the page was capped by `limit` / [`MAX_GRAPH_NODES`].
    pub truncated: bool,
    /// Echoed back so the frontend knows what it actually rendered.
    pub zoom: GraphZoom,
}

/// Build a paged graph view from raw memories and edges.
///
/// This is a pure function over plain data. The Tauri command in
/// `commands/memory.rs` does I/O (locks the store, fetches everything)
/// and then calls this function — keeping the logic testable in
/// `cargo test --lib` without spinning up Tauri.
pub fn build_graph_page(
    entries: &[MemoryEntry],
    edges: &[MemoryEdge],
    request: &GraphPageRequest,
) -> GraphPageResponse {
    let total_nodes = entries.len() as i64;
    let total_edges = edges.len() as i64;
    let limit = request
        .limit
        .unwrap_or(DEFAULT_GRAPH_LIMIT)
        .min(MAX_GRAPH_NODES)
        .max(1);

    match request.zoom {
        GraphZoom::Overview => build_overview(entries, edges, total_nodes, total_edges),
        GraphZoom::Cluster => build_cluster(entries, edges, request, limit, total_nodes, total_edges),
        GraphZoom::Detail => build_detail(entries, edges, request, limit, total_nodes, total_edges),
    }
}

// ─── Overview ────────────────────────────────────────────────────────────

fn build_overview(
    entries: &[MemoryEntry],
    edges: &[MemoryEdge],
    total_nodes: i64,
    total_edges: i64,
) -> GraphPageResponse {
    // Group memories by cognitive kind.
    let mut by_kind: HashMap<CognitiveKind, Vec<&MemoryEntry>> = HashMap::new();
    for e in entries {
        let k = classify_cognitive_kind(&e.memory_type, &e.tags, &e.content);
        by_kind.entry(k).or_default().push(e);
    }

    // Build a supernode per kind that has memories.
    let mut nodes = Vec::with_capacity(by_kind.len());
    let mut entry_kind: HashMap<i64, CognitiveKind> = HashMap::new();
    for (kind, group) in &by_kind {
        let count = group.len() as i64;
        let avg_imp = if group.is_empty() {
            0
        } else {
            (group.iter().map(|e| e.importance).sum::<i64>() / count).max(1)
        };
        nodes.push(GraphNode {
            id: format!("super-{}", kind.as_str()),
            label: format!("{} ({count})", pretty_kind(*kind)),
            kind: kind.as_str().to_string(),
            tier: String::new(),
            importance: avg_imp,
            degree: 0, // filled in after edge aggregation
            is_supernode: true,
            count,
        });
        for e in group {
            entry_kind.insert(e.id, *kind);
        }
    }

    // Aggregate edges by (src_kind, dst_kind).
    let mut weights: HashMap<(CognitiveKind, CognitiveKind), f64> = HashMap::new();
    let mut degree: HashMap<String, i64> = HashMap::new();
    for edge in edges {
        let (Some(sk), Some(dk)) = (entry_kind.get(&edge.src_id), entry_kind.get(&edge.dst_id))
        else {
            continue;
        };
        if sk == dk {
            continue; // skip self-loops at the supernode level
        }
        *weights.entry((*sk, *dk)).or_insert(0.0) += 1.0;
        *degree.entry(format!("super-{}", sk.as_str())).or_insert(0) += 1;
        *degree.entry(format!("super-{}", dk.as_str())).or_insert(0) += 1;
    }

    // Backfill degrees onto the supernodes.
    for n in &mut nodes {
        if let Some(d) = degree.get(&n.id) {
            n.degree = *d;
        }
    }

    let mut out_edges = Vec::with_capacity(weights.len());
    for ((sk, dk), w) in weights {
        out_edges.push(GraphEdge {
            source: format!("super-{}", sk.as_str()),
            target: format!("super-{}", dk.as_str()),
            rel_type: "aggregated".to_string(),
            weight: w,
        });
    }

    // Deterministic ordering for tests / stable rendering.
    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    out_edges.sort_by(|a, b| a.source.cmp(&b.source).then_with(|| a.target.cmp(&b.target)));

    GraphPageResponse {
        nodes,
        edges: out_edges,
        total_nodes,
        total_edges,
        truncated: false,
        zoom: GraphZoom::Overview,
    }
}

// ─── Detail ──────────────────────────────────────────────────────────────

fn build_detail(
    entries: &[MemoryEntry],
    edges: &[MemoryEdge],
    request: &GraphPageRequest,
    limit: usize,
    total_nodes: i64,
    total_edges: i64,
) -> GraphPageResponse {
    // Compute degree for ranking.
    let mut degree: HashMap<i64, i64> = HashMap::new();
    for e in edges {
        *degree.entry(e.src_id).or_insert(0) += 1;
        *degree.entry(e.dst_id).or_insert(0) += 1;
    }

    // 1-hop neighbours of the focus (if any) get priority.
    let mut neighbours: HashSet<i64> = HashSet::new();
    if let Some(focus) = request.focus_id {
        neighbours.insert(focus);
        for e in edges {
            if e.src_id == focus {
                neighbours.insert(e.dst_id);
            } else if e.dst_id == focus {
                neighbours.insert(e.src_id);
            }
        }
    }

    // Rank: focus & neighbours first, then degree desc, importance desc,
    // recency desc.
    let mut ranked: Vec<&MemoryEntry> = entries.iter().collect();
    ranked.sort_by(|a, b| {
        let na = neighbours.contains(&a.id);
        let nb = neighbours.contains(&b.id);
        nb.cmp(&na)
            .then_with(|| {
                degree
                    .get(&b.id)
                    .unwrap_or(&0)
                    .cmp(degree.get(&a.id).unwrap_or(&0))
            })
            .then_with(|| b.importance.cmp(&a.importance))
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| a.id.cmp(&b.id))
    });

    let truncated = ranked.len() > limit;
    ranked.truncate(limit);

    let kept: HashSet<i64> = ranked.iter().map(|e| e.id).collect();
    let nodes: Vec<GraphNode> = ranked
        .iter()
        .map(|e| {
            let kind = classify_cognitive_kind(&e.memory_type, &e.tags, &e.content);
            GraphNode {
                id: format!("m-{}", e.id),
                label: short_label(&e.content),
                kind: kind.as_str().to_string(),
                tier: tier_str(e.tier).to_string(),
                importance: e.importance,
                degree: *degree.get(&e.id).unwrap_or(&0),
                is_supernode: false,
                count: 1,
            }
        })
        .collect();

    let out_edges: Vec<GraphEdge> = edges
        .iter()
        .filter(|e| kept.contains(&e.src_id) && kept.contains(&e.dst_id))
        .map(|e| GraphEdge {
            source: format!("m-{}", e.src_id),
            target: format!("m-{}", e.dst_id),
            rel_type: e.rel_type.clone(),
            weight: 1.0,
        })
        .collect();

    GraphPageResponse {
        nodes,
        edges: out_edges,
        total_nodes,
        total_edges,
        truncated,
        zoom: GraphZoom::Detail,
    }
}

// ─── Cluster ─────────────────────────────────────────────────────────────

fn build_cluster(
    entries: &[MemoryEntry],
    edges: &[MemoryEdge],
    request: &GraphPageRequest,
    limit: usize,
    total_nodes: i64,
    total_edges: i64,
) -> GraphPageResponse {
    // Determine the focused kind.
    let focus_kind = request.focus_kind.or_else(|| {
        request.focus_id.and_then(|fid| {
            entries.iter().find(|e| e.id == fid).map(|e| {
                classify_cognitive_kind(&e.memory_type, &e.tags, &e.content)
            })
        })
    });

    // Split: real nodes for focus_kind, supernodes for the rest.
    let mut focus_entries: Vec<&MemoryEntry> = Vec::new();
    let mut off_groups: HashMap<CognitiveKind, Vec<&MemoryEntry>> = HashMap::new();
    for e in entries {
        let k = classify_cognitive_kind(&e.memory_type, &e.tags, &e.content);
        if Some(k) == focus_kind {
            focus_entries.push(e);
        } else {
            off_groups.entry(k).or_default().push(e);
        }
    }

    // Detail-rank the focused kind.
    let mut degree: HashMap<i64, i64> = HashMap::new();
    for e in edges {
        *degree.entry(e.src_id).or_insert(0) += 1;
        *degree.entry(e.dst_id).or_insert(0) += 1;
    }
    focus_entries.sort_by(|a, b| {
        degree
            .get(&b.id)
            .unwrap_or(&0)
            .cmp(degree.get(&a.id).unwrap_or(&0))
            .then_with(|| b.importance.cmp(&a.importance))
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| a.id.cmp(&b.id))
    });
    // Reserve a slot for each off-kind supernode.
    let super_slots = off_groups.len();
    let focus_cap = limit.saturating_sub(super_slots).max(1);
    let truncated = focus_entries.len() > focus_cap;
    focus_entries.truncate(focus_cap);

    let kept: HashSet<i64> = focus_entries.iter().map(|e| e.id).collect();

    let mut nodes: Vec<GraphNode> = focus_entries
        .iter()
        .map(|e| {
            let kind = classify_cognitive_kind(&e.memory_type, &e.tags, &e.content);
            GraphNode {
                id: format!("m-{}", e.id),
                label: short_label(&e.content),
                kind: kind.as_str().to_string(),
                tier: tier_str(e.tier).to_string(),
                importance: e.importance,
                degree: *degree.get(&e.id).unwrap_or(&0),
                is_supernode: false,
                count: 1,
            }
        })
        .collect();

    let mut entry_kind: HashMap<i64, CognitiveKind> = HashMap::new();
    for e in &focus_entries {
        let k = classify_cognitive_kind(&e.memory_type, &e.tags, &e.content);
        entry_kind.insert(e.id, k);
    }
    for (kind, group) in &off_groups {
        let count = group.len() as i64;
        let avg_imp = if group.is_empty() {
            0
        } else {
            (group.iter().map(|e| e.importance).sum::<i64>() / count).max(1)
        };
        nodes.push(GraphNode {
            id: format!("super-{}", kind.as_str()),
            label: format!("{} ({count})", pretty_kind(*kind)),
            kind: kind.as_str().to_string(),
            tier: String::new(),
            importance: avg_imp,
            degree: 0,
            is_supernode: true,
            count,
        });
        for e in group {
            entry_kind.insert(e.id, *kind);
        }
    }

    // Edges: real-real within focus, real-super between focus and other kinds,
    // super-super between off-groups when both endpoints map to a kind.
    let mut weighted: HashMap<(String, String, String), f64> = HashMap::new();
    let mut degree_by_id: HashMap<String, i64> = HashMap::new();
    for e in edges {
        let src_in = kept.contains(&e.src_id);
        let dst_in = kept.contains(&e.dst_id);
        let src_node = if src_in {
            Some(format!("m-{}", e.src_id))
        } else {
            entry_kind
                .get(&e.src_id)
                .map(|k| format!("super-{}", k.as_str()))
        };
        let dst_node = if dst_in {
            Some(format!("m-{}", e.dst_id))
        } else {
            entry_kind
                .get(&e.dst_id)
                .map(|k| format!("super-{}", k.as_str()))
        };
        let (Some(s), Some(d)) = (src_node, dst_node) else {
            continue;
        };
        if s == d {
            continue;
        }
        let rel = if src_in && dst_in {
            e.rel_type.clone()
        } else {
            "aggregated".to_string()
        };
        *weighted.entry((s.clone(), d.clone(), rel)).or_insert(0.0) += 1.0;
        *degree_by_id.entry(s).or_insert(0) += 1;
        *degree_by_id.entry(d).or_insert(0) += 1;
    }

    for n in &mut nodes {
        if n.is_supernode {
            if let Some(d) = degree_by_id.get(&n.id) {
                n.degree = *d;
            }
        }
    }

    let mut out_edges: Vec<GraphEdge> = weighted
        .into_iter()
        .map(|((s, d, rel), w)| GraphEdge {
            source: s,
            target: d,
            rel_type: rel,
            weight: w,
        })
        .collect();

    nodes.sort_by(|a, b| b.degree.cmp(&a.degree).then_with(|| a.id.cmp(&b.id)));
    out_edges.sort_by(|a, b| a.source.cmp(&b.source).then_with(|| a.target.cmp(&b.target)));

    GraphPageResponse {
        nodes,
        edges: out_edges,
        total_nodes,
        total_edges,
        truncated,
        zoom: GraphZoom::Cluster,
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn tier_str(t: MemoryTier) -> &'static str {
    match t {
        MemoryTier::Short => "short",
        MemoryTier::Working => "working",
        MemoryTier::Long => "long",
    }
}

fn pretty_kind(k: CognitiveKind) -> &'static str {
    match k {
        CognitiveKind::Episodic => "Episodic",
        CognitiveKind::Semantic => "Semantic",
        CognitiveKind::Procedural => "Procedural",
        CognitiveKind::Judgment => "Judgment",
        CognitiveKind::Negative => "Negative",
    }
}

/// Trim content to a short label suitable for graph rendering.
/// Strips whitespace, collapses newlines, caps at 60 graphemes.
fn short_label(content: &str) -> String {
    let mut s = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() > 60 {
        s = s.chars().take(57).collect::<String>();
        s.push_str("…");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::edges::EdgeSource;
    use crate::memory::store::MemoryType;

    fn entry(id: i64, tier: MemoryTier, tags: &str, content: &str, importance: i64) -> MemoryEntry {
        MemoryEntry {
            id,
            content: content.to_string(),
            tags: tags.to_string(),
            importance,
            memory_type: MemoryType::Fact,
            created_at: id, // deterministic ordering by id
            last_accessed: None,
            access_count: 0,
            embedding: None,
            tier,
            decay_score: 1.0,
            session_id: None,
            parent_id: None,
            token_count: 0,
            source_url: None,
            source_hash: None,
            expires_at: None,
            valid_to: None,
            obsidian_path: None,
            last_exported: None,
            updated_at: None,
            origin_device: None,
            hlc_counter: None,
            confidence: 1.0,
        }
    }

    fn edge(src: i64, dst: i64, rel: &str) -> MemoryEdge {
        MemoryEdge {
            id: src * 1000 + dst,
            src_id: src,
            dst_id: dst,
            rel_type: rel.to_string(),
            confidence: 1.0,
            source: EdgeSource::Llm,
            created_at: 0,
            valid_from: None,
            valid_to: None,
            edge_source: None,
        }
    }

    #[test]
    fn detail_zoom_respects_limit_and_marks_truncated() {
        let entries: Vec<MemoryEntry> = (1..=50)
            .map(|i| entry(i, MemoryTier::Long, "semantic", &format!("memory {i}"), 3))
            .collect();
        let edges: Vec<MemoryEdge> = Vec::new();
        let req = GraphPageRequest {
            limit: Some(10),
            zoom: GraphZoom::Detail,
            ..Default::default()
        };
        let page = build_graph_page(&entries, &edges, &req);
        assert_eq!(page.nodes.len(), 10);
        assert!(page.truncated);
        assert_eq!(page.total_nodes, 50);
        assert_eq!(page.zoom, GraphZoom::Detail);
    }

    #[test]
    fn detail_zoom_prioritises_focus_neighbours() {
        let entries = vec![
            entry(1, MemoryTier::Long, "semantic", "hub", 1),
            entry(2, MemoryTier::Long, "semantic", "neighbour", 1),
            entry(3, MemoryTier::Long, "semantic", "stranger", 5),
        ];
        let edges = vec![edge(1, 2, "links_to")];
        let req = GraphPageRequest {
            focus_id: Some(1),
            limit: Some(2),
            zoom: GraphZoom::Detail,
            ..Default::default()
        };
        let page = build_graph_page(&entries, &edges, &req);
        let ids: Vec<&str> = page.nodes.iter().map(|n| n.id.as_str()).collect();
        // Focus + neighbour win over the more important stranger.
        assert!(ids.contains(&"m-1"));
        assert!(ids.contains(&"m-2"));
        assert!(!ids.contains(&"m-3"));
        // Only 1 edge survives, both endpoints kept.
        assert_eq!(page.edges.len(), 1);
    }

    #[test]
    fn detail_zoom_drops_edges_to_paged_out_nodes() {
        let entries: Vec<MemoryEntry> = (1..=5)
            .map(|i| entry(i, MemoryTier::Long, "semantic", &format!("m{i}"), 3))
            .collect();
        let edges = vec![edge(1, 2, "r"), edge(2, 99, "r")];
        let req = GraphPageRequest {
            limit: Some(5),
            zoom: GraphZoom::Detail,
            ..Default::default()
        };
        let page = build_graph_page(&entries, &edges, &req);
        // edge to memory 99 (not in the set) must be dropped.
        assert!(page.edges.iter().all(|e| e.target != "m-99"));
    }

    #[test]
    fn overview_zoom_collapses_to_one_node_per_kind() {
        let entries = vec![
            entry(1, MemoryTier::Long, "semantic", "fact A", 3),
            entry(2, MemoryTier::Long, "semantic", "fact B", 3),
            entry(3, MemoryTier::Long, "procedural", "how to ship: bump tag push", 3),
            entry(4, MemoryTier::Long, "episodic", "yesterday we shipped", 3),
        ];
        let edges = vec![edge(1, 3, "links_to"), edge(2, 4, "links_to")];
        let req = GraphPageRequest {
            zoom: GraphZoom::Overview,
            ..Default::default()
        };
        let page = build_graph_page(&entries, &edges, &req);
        assert!(page.nodes.iter().all(|n| n.is_supernode));
        assert!(page.nodes.iter().any(|n| n.kind == "semantic"));
        assert!(page.nodes.iter().any(|n| n.kind == "procedural"));
        assert!(page.nodes.iter().any(|n| n.kind == "episodic"));
        // Two cross-kind edges → two aggregated super-edges (kind pairs differ).
        assert!(!page.edges.is_empty());
        assert!(page.edges.iter().all(|e| e.rel_type == "aggregated"));
    }

    #[test]
    fn cluster_zoom_keeps_focus_kind_real_and_others_super() {
        let entries = vec![
            entry(1, MemoryTier::Long, "semantic", "fact A", 3),
            entry(2, MemoryTier::Long, "semantic", "fact B", 3),
            entry(3, MemoryTier::Long, "procedural", "how to ship: bump tag push", 3),
            entry(4, MemoryTier::Long, "procedural", "how to deploy: ssh restart", 3),
            entry(5, MemoryTier::Long, "episodic", "yesterday we shipped", 3),
        ];
        let edges = vec![edge(1, 3, "r"), edge(2, 5, "r")];
        let req = GraphPageRequest {
            focus_kind: Some(CognitiveKind::Semantic),
            zoom: GraphZoom::Cluster,
            limit: Some(50),
            ..Default::default()
        };
        let page = build_graph_page(&entries, &edges, &req);
        // Semantic entries must be real, others must be supernodes.
        let semantic_nodes: Vec<_> = page.nodes.iter().filter(|n| n.kind == "semantic").collect();
        assert!(!semantic_nodes.is_empty());
        assert!(semantic_nodes.iter().all(|n| !n.is_supernode));
        let non_sem: Vec<_> = page.nodes.iter().filter(|n| n.kind != "semantic").collect();
        assert!(non_sem.iter().all(|n| n.is_supernode));
    }

    #[test]
    fn limit_is_clamped_to_max() {
        let entries: Vec<MemoryEntry> = (1..=20)
            .map(|i| entry(i, MemoryTier::Long, "semantic", "x", 3))
            .collect();
        let req = GraphPageRequest {
            limit: Some(MAX_GRAPH_NODES * 10),
            zoom: GraphZoom::Detail,
            ..Default::default()
        };
        let page = build_graph_page(&entries, &[], &req);
        assert!(page.nodes.len() <= MAX_GRAPH_NODES);
        assert_eq!(page.nodes.len(), 20);
        assert!(!page.truncated);
    }

    #[test]
    fn short_label_caps_long_content() {
        let s = "x".repeat(200);
        let label = short_label(&s);
        assert!(label.chars().count() <= 60);
        assert!(label.ends_with("…"));
    }
}
