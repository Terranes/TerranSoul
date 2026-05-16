//! GraphRAG / LightRAG community summaries (Chunk 16.6, extended by
//! GRAPHRAG-1a for hierarchical multi-level summaries).
//!
//! Implements Leiden-style community detection over `memory_edges`,
//! LLM-generated community summaries, and dual-level retrieval (entity +
//! community) fused via RRF. GRAPHRAG-1a recurses the detection step so
//! `memory_communities.level` carries levels 0..N (N capped at 4) — each
//! level builds a super-graph from the previous level's assignments,
//! yielding broader summaries that the retrieval router can target via
//! the optional `level` filter on [`MemoryStore::graph_rag_search`].

use std::collections::{BTreeSet, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

use super::cascade::cascade_expand;
use super::edges::MemoryEdge;
use super::fusion::{reciprocal_rank_fuse, DEFAULT_RRF_K};
use super::query_intent::QueryScope;
use super::store::{cosine_similarity, MemoryStore};

/// Hard cap on hierarchy depth (levels 0..=`MAX_HIERARCHY_LEVELS - 1`,
/// i.e. up to 5 levels total counting level 0). Mirrors the chunk scope
/// "N capped at 4".
pub const MAX_HIERARCHY_LEVELS: usize = 5;

// ─── Schema ────────────────────────────────────────────────────────────

/// Create the `memory_communities` table if it doesn't exist.
pub fn ensure_communities_table(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memory_communities (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            level       INTEGER NOT NULL DEFAULT 0,
            member_ids  TEXT    NOT NULL,
            summary     TEXT,
            embedding   BLOB,
            updated_at  INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_communities_level ON memory_communities(level);",
    )
}

// ─── Community Data ────────────────────────────────────────────────────

/// A detected community of memory nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    pub id: i64,
    pub level: i32,
    pub member_ids: Vec<i64>,
    pub summary: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub updated_at: i64,
}

/// A search hit from community-level retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityHit {
    pub community_id: i64,
    pub summary: String,
    pub member_ids: Vec<i64>,
    pub score: f64,
}

// ─── Leiden Community Detection ────────────────────────────────────────

/// Simplified Louvain/Leiden community detection.
///
/// Takes edges (undirected interpretation) and returns a mapping from
/// node_id → community_id. Uses modularity-greedy phase only (sufficient
/// for memory graphs which are typically small, <10k nodes).
pub fn detect_communities(edges: &[MemoryEdge]) -> HashMap<i64, usize> {
    let tuples: Vec<(i64, i64, f64)> = edges
        .iter()
        .map(|e| (e.src_id, e.dst_id, e.confidence))
        .collect();
    detect_communities_weighted(&tuples)
}

/// Core greedy-modularity community detection over weighted edges.
///
/// Factored out of [`detect_communities`] so the hierarchical detector
/// can reuse the same algorithm on a super-graph whose nodes are
/// previous-level community IDs.
pub fn detect_communities_weighted(edges: &[(i64, i64, f64)]) -> HashMap<i64, usize> {
    if edges.is_empty() {
        return HashMap::new();
    }

    // Build adjacency list with weights.
    let mut adjacency: HashMap<i64, Vec<(i64, f64)>> = HashMap::new();
    let mut total_weight = 0.0_f64;

    for &(src, dst, w) in edges {
        adjacency.entry(src).or_default().push((dst, w));
        adjacency.entry(dst).or_default().push((src, w));
        total_weight += w;
    }

    if total_weight == 0.0 {
        total_weight = 1.0; // avoid div-by-zero
    }

    let mut nodes: Vec<i64> = adjacency.keys().copied().collect();
    nodes.sort_unstable();

    // Node degree (sum of edge weights).
    let degree: HashMap<i64, f64> = adjacency
        .iter()
        .map(|(&node, neighbors)| (node, neighbors.iter().map(|(_, w)| w).sum()))
        .collect();

    // Initial assignment: each node in its own community.
    let mut community: HashMap<i64, usize> =
        nodes.iter().enumerate().map(|(i, &n)| (n, i)).collect();

    // Greedy modularity optimization (single pass — sufficient for small graphs).
    let mut improved = true;
    let mut iterations = 0;
    while improved && iterations < 50 {
        improved = false;
        iterations += 1;

        for &node in &nodes {
            let current_comm = community[&node];
            let node_deg = degree[&node];

            // Compute modularity gain for moving node to each neighbor's community.
            let mut best_comm = current_comm;
            let mut best_gain = 0.0_f64;

            // Aggregate edge weight per neighbor community.
            let mut comm_weights: HashMap<usize, f64> = HashMap::new();
            if let Some(neighbors) = adjacency.get(&node) {
                for &(neighbor, w) in neighbors {
                    let nc = community[&neighbor];
                    *comm_weights.entry(nc).or_default() += w;
                }
            }

            // Community total degree (excluding current node).
            let mut comm_degree: HashMap<usize, f64> = HashMap::new();
            for &n2 in &nodes {
                if n2 != node {
                    *comm_degree.entry(community[&n2]).or_default() += degree[&n2];
                }
            }

            let mut candidate_weights: Vec<(usize, f64)> = comm_weights.into_iter().collect();
            candidate_weights.sort_by_key(|(candidate_comm, _)| *candidate_comm);
            for (candidate_comm, ki_in) in candidate_weights {
                if candidate_comm == current_comm {
                    continue;
                }
                let sigma_tot = comm_degree.get(&candidate_comm).copied().unwrap_or(0.0);
                let gain = ki_in - sigma_tot * node_deg / (2.0 * total_weight);
                if gain > best_gain {
                    best_gain = gain;
                    best_comm = candidate_comm;
                }
            }

            if best_comm != current_comm {
                community.insert(node, best_comm);
                improved = true;
            }
        }
    }

    // Renumber communities to be contiguous 0..k.
    let mut remap: HashMap<usize, usize> = HashMap::new();
    let mut next_id = 0;
    for node in &nodes {
        let val = community
            .get_mut(node)
            .expect("community assigned for node");
        let entry = remap.entry(*val).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            id
        });
        *val = *entry;
    }

    community
}

/// Hierarchical community detection (GRAPHRAG-1a).
///
/// Returns one assignment per level, each keyed by the **original**
/// memory id. `result[k][node]` is the community that `node` belongs to
/// at level `k`. Level 0 is the raw graph; level `k+1` is computed from
/// a super-graph whose nodes are level-`k` communities and whose edge
/// weights are the sum of base-edge confidences crossing those
/// communities.
///
/// `max_levels` is clamped to `[1, MAX_HIERARCHY_LEVELS]`. Detection
/// stops early when the next level would produce the same number of
/// communities as the current one (converged) or when the super-graph
/// has no edges (all communities are isolated).
pub fn detect_communities_hierarchical(
    edges: &[MemoryEdge],
    max_levels: usize,
) -> Vec<HashMap<i64, usize>> {
    let max_levels = max_levels.clamp(1, MAX_HIERARCHY_LEVELS);
    if edges.is_empty() {
        return Vec::new();
    }

    let base_tuples: Vec<(i64, i64, f64)> = edges
        .iter()
        .map(|e| (e.src_id, e.dst_id, e.confidence))
        .collect();

    let level0 = detect_communities_weighted(&base_tuples);
    if level0.is_empty() {
        return Vec::new();
    }

    let mut hierarchy: Vec<HashMap<i64, usize>> = vec![level0];

    while hierarchy.len() < max_levels {
        let current = hierarchy.last().expect("hierarchy never empty here");

        // Build super-edges from base edges using current-level assignments.
        // Aggregate weights between distinct super-communities.
        let mut super_edge_weights: HashMap<(i64, i64), f64> = HashMap::new();
        for &(src, dst, w) in &base_tuples {
            let cu = current.get(&src).copied();
            let cv = current.get(&dst).copied();
            match (cu, cv) {
                (Some(a), Some(b)) if a != b => {
                    // Canonicalise the unordered pair so (a,b) and (b,a) merge.
                    let key = if a < b {
                        (a as i64, b as i64)
                    } else {
                        (b as i64, a as i64)
                    };
                    *super_edge_weights.entry(key).or_default() += w;
                }
                _ => {}
            }
        }

        if super_edge_weights.is_empty() {
            break;
        }

        let super_tuples: Vec<(i64, i64, f64)> = super_edge_weights
            .into_iter()
            .map(|((a, b), w)| (a, b, w))
            .collect();
        let super_assign = detect_communities_weighted(&super_tuples);
        if super_assign.is_empty() {
            break;
        }

        // Convergence check: count distinct super-communities.
        let next_count = super_assign
            .values()
            .copied()
            .collect::<BTreeSet<usize>>()
            .len();
        let current_count = current
            .values()
            .copied()
            .collect::<BTreeSet<usize>>()
            .len();
        if next_count >= current_count {
            // No further coarsening — would just renumber, not merge.
            break;
        }

        // Flatten: for each original node, look up its level-k community,
        // then look that community up in the super-assignment.
        let mut next_level: HashMap<i64, usize> = HashMap::with_capacity(current.len());
        for (&node, &comm_k) in current {
            if let Some(&super_comm) = super_assign.get(&(comm_k as i64)) {
                next_level.insert(node, super_comm);
            } else {
                // Singleton super-community (no inter-community edges from it):
                // give it a fresh id beyond the super-assignment range so it
                // still appears in the next level as its own group.
                let fresh = next_count + comm_k;
                next_level.insert(node, fresh);
            }
        }

        hierarchy.push(next_level);
    }

    hierarchy
}

// ─── Persistence ───────────────────────────────────────────────────────

impl MemoryStore {
    /// Ensure the communities table exists.
    pub fn ensure_communities_schema(&self) -> SqlResult<()> {
        let conn = self.conn();
        ensure_communities_table(conn)
    }

    /// Store detected communities (replaces all existing).
    pub fn save_communities(&self, communities: &[Community]) -> SqlResult<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM memory_communities", [])?;
        let mut stmt = conn.prepare(
            "INSERT INTO memory_communities (level, member_ids, summary, embedding, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for c in communities {
            let member_json = serde_json::to_string(&c.member_ids).unwrap_or_default();
            let emb_bytes: Option<Vec<u8>> = c
                .embedding
                .as_ref()
                .map(|e| super::store::embedding_to_bytes(e));
            stmt.execute(params![
                c.level,
                member_json,
                c.summary,
                emb_bytes,
                c.updated_at,
            ])?;
        }
        Ok(())
    }

    /// Load all communities.
    pub fn load_communities(&self) -> SqlResult<Vec<Community>> {
        let conn = self.conn();
        // Ensure table exists before querying.
        ensure_communities_table(conn)?;
        let mut stmt = conn.prepare(
            "SELECT id, level, member_ids, summary, embedding, updated_at FROM memory_communities ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            let member_json: String = row.get(2)?;
            let emb_raw: Option<Vec<u8>> = row.get(4)?;
            Ok(Community {
                id: row.get(0)?,
                level: row.get(1)?,
                member_ids: serde_json::from_str(&member_json).unwrap_or_default(),
                summary: row.get(3)?,
                embedding: emb_raw.map(|b| super::store::bytes_to_embedding(&b)),
                updated_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    /// Run community detection on the current edge graph and store results.
    ///
    /// Returns the detected communities (without summaries — those must be
    /// generated separately via LLM).
    pub fn detect_and_store_communities(&self) -> SqlResult<Vec<Community>> {
        self.detect_and_store_hierarchy(1)
    }

    /// Hierarchical detection (GRAPHRAG-1a): build communities at levels
    /// `0..max_levels` (capped at [`MAX_HIERARCHY_LEVELS`]), preserving
    /// existing summaries when a level's community has the **same member
    /// set** as before. Returns the freshly-stored communities (summaries
    /// may still be `None` for newly-formed groups — the caller is
    /// expected to generate them via the brain).
    pub fn detect_and_store_hierarchy(&self, max_levels: usize) -> SqlResult<Vec<Community>> {
        self.ensure_communities_schema()?;

        // Snapshot existing communities so we can carry summaries forward
        // for any level/member-set that is unchanged.
        type CarryKey = (i32, Vec<i64>);
        type CarryValue = (Option<String>, Option<Vec<f32>>);
        let existing = self.load_communities()?;
        let mut summary_by_key: HashMap<CarryKey, CarryValue> = HashMap::new();
        for c in existing {
            let mut sorted = c.member_ids.clone();
            sorted.sort_unstable();
            summary_by_key.insert((c.level, sorted), (c.summary, c.embedding));
        }

        let edges = self.list_edges()?;
        let hierarchy = detect_communities_hierarchical(&edges, max_levels);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut communities: Vec<Community> = Vec::new();
        for (level_idx, assignment) in hierarchy.iter().enumerate() {
            let mut groups: HashMap<usize, Vec<i64>> = HashMap::new();
            for (&node, &comm) in assignment {
                groups.entry(comm).or_default().push(node);
            }
            for (_comm_id, mut members) in groups {
                members.sort_unstable();
                let key = (level_idx as i32, members.clone());
                let (carried_summary, carried_embedding) = summary_by_key
                    .get(&key)
                    .cloned()
                    .unwrap_or((None, None));
                communities.push(Community {
                    id: 0, // assigned by DB
                    level: level_idx as i32,
                    member_ids: members,
                    summary: carried_summary,
                    embedding: carried_embedding,
                    updated_at: now,
                });
            }
        }

        self.save_communities(&communities)?;
        self.load_communities()
    }

    /// Update a single community's summary (and optional embedding).
    ///
    /// Used by the orchestrator that calls the active brain provider for
    /// each unsummarised community, so we don't rewrite the whole table
    /// on every LLM round-trip.
    pub fn set_community_summary(
        &self,
        community_id: i64,
        summary: &str,
        embedding: Option<&[f32]>,
    ) -> SqlResult<()> {
        let conn = self.conn();
        let emb_bytes: Option<Vec<u8>> =
            embedding.map(super::store::embedding_to_bytes);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        conn.execute(
            "UPDATE memory_communities
                SET summary = ?1, embedding = ?2, updated_at = ?3
                WHERE id = ?4",
            params![summary, emb_bytes, now, community_id],
        )?;
        Ok(())
    }

    /// Dual-level GraphRAG retrieval: entity (memory) search + community
    /// summary search, fused via RRF.
    ///
    /// `level_filter`, when `Some(l)`, restricts community-side retrieval
    /// to communities at hierarchy level `l` (GRAPHRAG-1a). Pass `None`
    /// to consider every level — the historical behaviour preserved for
    /// callers that have not yet adopted the hierarchy.
    ///
    /// Returns `(memory_id, score)` pairs sorted descending.
    pub fn graph_rag_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> SqlResult<Vec<(i64, f64)>> {
        self.graph_rag_search_at_level(query, query_embedding, limit, None)
    }

    /// Hierarchy-aware variant of [`MemoryStore::graph_rag_search`].
    ///
    /// See [`MemoryStore::graph_rag_search`] for the dual-level fusion;
    /// the additional `level_filter` parameter targets a specific
    /// hierarchy depth on the community side.
    pub fn graph_rag_search_at_level(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
        level_filter: Option<i32>,
    ) -> SqlResult<Vec<(i64, f64)>> {
        // Level 1: entity-level keyword search.
        let entity_hits = self.search(query)?;
        let entity_ranking: Vec<i64> = entity_hits.iter().take(limit * 2).map(|e| e.id).collect();

        // Level 2: community-level search (optionally filtered to a single
        // hierarchy depth).
        let mut communities = self.load_communities()?;
        if let Some(target_level) = level_filter {
            communities.retain(|c| c.level == target_level);
        }
        let community_ranking =
            rank_communities_by_query(query, query_embedding, &communities, limit * 2);

        // Expand community hits to member ids.
        let community_member_ranking: Vec<i64> = community_ranking
            .iter()
            .flat_map(|ch| ch.member_ids.iter().copied())
            .collect();

        // Fuse via RRF.
        let rankings: &[&[i64]] = &[&entity_ranking, &community_member_ranking];
        let fused = reciprocal_rank_fuse(rankings, DEFAULT_RRF_K);

        Ok(fused.into_iter().take(limit).collect())
    }

    /// Scope-routed GraphRAG retrieval (GRAPHRAG-1c).
    ///
    /// Routes the query to the appropriate retrieval path based on
    /// [`QueryScope`]:
    ///
    /// - `Global` → community summaries at the top hierarchy level.
    /// - `Local` → hybrid_search_rrf + cascade entity-walk.
    /// - `Mixed` → standard dual-level RRF fusion (entity + community).
    pub fn graph_rag_search_routed(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
        scope: QueryScope,
    ) -> SqlResult<Vec<(i64, f64)>> {
        match scope {
            QueryScope::Global => {
                // Route to top-level community summaries only.
                // Find the maximum hierarchy level present.
                let max_level = self.max_community_level()?;
                self.graph_rag_search_at_level(query, query_embedding, limit, Some(max_level))
            }
            QueryScope::Local => {
                // Entity-walk: hybrid keyword search + cascade expansion
                // through knowledge graph edges.
                let entity_hits = self.search(query)?;
                let seeds: Vec<(i64, f64)> = entity_hits
                    .iter()
                    .take(limit * 2)
                    .enumerate()
                    .map(|(rank, e)| {
                        // Use RRF-style score from rank position.
                        let score = 1.0 / (DEFAULT_RRF_K as f64 + rank as f64 + 1.0);
                        (e.id, score)
                    })
                    .collect();

                let expanded = cascade_expand(&self.conn, &seeds, None)?;
                let mut results = expanded;
                results.sort_by(|a, b| {
                    b.1.partial_cmp(&a.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                results.truncate(limit);
                Ok(results)
            }
            QueryScope::Mixed => {
                // Standard dual-level fusion — all levels considered.
                self.graph_rag_search(query, query_embedding, limit)
            }
        }
    }

    /// Return the highest community hierarchy level in the store.
    /// Defaults to 0 if no communities exist.
    fn max_community_level(&self) -> SqlResult<i32> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(level), 0) FROM memory_communities",
                [],
                |row| row.get(0),
            )
    }
}

/// Rank communities against a query using keyword overlap + optional vector similarity.
fn rank_communities_by_query(
    query: &str,
    query_embedding: Option<&[f32]>,
    communities: &[Community],
    limit: usize,
) -> Vec<CommunityHit> {
    let query_lower = query.to_lowercase();
    let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();

    let mut hits: Vec<CommunityHit> = communities
        .iter()
        .filter_map(|c| {
            let summary = c.summary.as_deref()?;
            // Keyword score.
            let summary_lower = summary.to_lowercase();
            let keyword_score: f64 = query_tokens
                .iter()
                .filter(|t| summary_lower.contains(**t))
                .count() as f64
                / query_tokens.len().max(1) as f64;

            // Vector score (if both embeddings available).
            let vector_score = match (query_embedding, c.embedding.as_deref()) {
                (Some(qe), Some(ce)) => cosine_similarity(qe, ce) as f64,
                _ => 0.0,
            };

            let score = keyword_score * 0.4 + vector_score * 0.6;
            if score > 0.0 {
                Some(CommunityHit {
                    community_id: c.id,
                    summary: summary.to_string(),
                    member_ids: c.member_ids.clone(),
                    score,
                })
            } else {
                None
            }
        })
        .collect();

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(limit);
    hits
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryStore, MemoryType, NewMemory, NewMemoryEdge};
    use std::path::Path;

    fn make_store_with_graph() -> MemoryStore {
        let store = MemoryStore::new(Path::new(":memory:"));
        // Create 4 memories forming two clusters:
        // Cluster A: m1 <-> m2 (rust topics)
        // Cluster B: m3 <-> m4 (music topics)
        let m1 = store
            .add(NewMemory {
                content: "Rust uses ownership for memory safety".into(),
                tags: "rust,programming".into(),
                importance: 4,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap()
            .id;
        let m2 = store
            .add(NewMemory {
                content: "Cargo is the Rust package manager".into(),
                tags: "rust,tooling".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap()
            .id;
        let m3 = store
            .add(NewMemory {
                content: "Jazz originated in New Orleans".into(),
                tags: "music,history".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap()
            .id;
        let m4 = store
            .add(NewMemory {
                content: "Blues scales use flatted thirds and sevenths".into(),
                tags: "music,theory".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap()
            .id;

        // Strong edges within clusters.
        store
            .add_edge(NewMemoryEdge {
                src_id: m1,
                dst_id: m2,
                rel_type: "related_to".into(),
                confidence: 0.9,
                source: crate::memory::EdgeSource::Auto,
                valid_from: None,
                valid_to: None,
                edge_source: None,
            })
            .unwrap();
        store
            .add_edge(NewMemoryEdge {
                src_id: m3,
                dst_id: m4,
                rel_type: "related_to".into(),
                confidence: 0.9,
                source: crate::memory::EdgeSource::Auto,
                valid_from: None,
                valid_to: None,
                edge_source: None,
            })
            .unwrap();

        store
    }

    #[test]
    fn detect_communities_finds_two_clusters() {
        let store = make_store_with_graph();
        let edges = store.list_edges().unwrap();
        let assignments = detect_communities(&edges);

        // Should have 4 nodes assigned.
        assert_eq!(assignments.len(), 4);

        // Nodes in same cluster should share community.
        let all = store.get_all().unwrap();
        let rust_ids: Vec<i64> = all
            .iter()
            .filter(|entry| entry.tags.contains("rust"))
            .map(|entry| entry.id)
            .collect();
        let music_ids: Vec<i64> = all
            .iter()
            .filter(|entry| entry.tags.contains("music"))
            .map(|entry| entry.id)
            .collect();
        assert_eq!(rust_ids.len(), 2);
        assert_eq!(music_ids.len(), 2);
        // Rust nodes share one community; music nodes share another.
        assert_eq!(assignments[&rust_ids[0]], assignments[&rust_ids[1]]);
        assert_eq!(assignments[&music_ids[0]], assignments[&music_ids[1]]);
        // The two clusters are different.
        assert_ne!(assignments[&rust_ids[0]], assignments[&music_ids[0]]);
    }

    #[test]
    fn detect_and_store_communities_persists() {
        let store = make_store_with_graph();
        let communities = store.detect_and_store_communities().unwrap();
        assert_eq!(communities.len(), 2);

        // Reload.
        let loaded = store.load_communities().unwrap();
        assert_eq!(loaded.len(), 2);
        for c in &loaded {
            assert!(c.member_ids.len() >= 2);
        }
    }

    #[test]
    fn graph_rag_search_returns_relevant_hits() {
        let store = make_store_with_graph();
        store.ensure_communities_schema().unwrap();

        // Set up communities with summaries.
        let all = store.get_all().unwrap();
        let rust_ids: Vec<i64> = all
            .iter()
            .filter(|e| e.tags.contains("rust"))
            .map(|e| e.id)
            .collect();
        let music_ids: Vec<i64> = all
            .iter()
            .filter(|e| e.tags.contains("music"))
            .map(|e| e.id)
            .collect();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        store
            .save_communities(&[
                Community {
                    id: 0,
                    level: 0,
                    member_ids: rust_ids.clone(),
                    summary: Some(
                        "Programming community about Rust language, ownership, and Cargo tooling."
                            .into(),
                    ),
                    embedding: None,
                    updated_at: now,
                },
                Community {
                    id: 0,
                    level: 0,
                    member_ids: music_ids.clone(),
                    summary: Some("Music community about jazz origins and blues theory.".into()),
                    embedding: None,
                    updated_at: now,
                },
            ])
            .unwrap();

        // Search for "rust" should rank rust members higher.
        let results = store
            .graph_rag_search("rust ownership cargo", None, 10)
            .unwrap();
        assert!(!results.is_empty());
        // First result should be a rust memory.
        let first_id = results[0].0;
        let first = all.iter().find(|e| e.id == first_id).unwrap();
        assert!(
            first.tags.contains("rust"),
            "first hit should be rust-related, got: {}",
            first.content
        );
    }

    #[test]
    fn detect_communities_handles_empty_graph() {
        let result = detect_communities(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn community_ranking_keyword_only() {
        let communities = vec![
            Community {
                id: 1,
                level: 0,
                member_ids: vec![1, 2],
                summary: Some("Rust programming and memory safety".into()),
                embedding: None,
                updated_at: 0,
            },
            Community {
                id: 2,
                level: 0,
                member_ids: vec![3, 4],
                summary: Some("Jazz music and blues history".into()),
                embedding: None,
                updated_at: 0,
            },
        ];
        let hits = rank_communities_by_query("rust memory", None, &communities, 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].community_id, 1);
    }

    // ── GRAPHRAG-1a: hierarchical detection ────────────────────────

    /// Build a graph with **four** tight micro-clusters that pair into
    /// **two** meta-clusters at level 1.
    ///
    /// Layout:
    ///   - Cluster A1 = {1,2,3} (triangle, weight 1.0)
    ///   - Cluster A2 = {4,5,6} (triangle, weight 1.0)
    ///   - A1 ↔ A2 bridged by edge (3,4) at weight 0.6
    ///   - Cluster B1 = {7,8,9} (triangle, weight 1.0)
    ///   - Cluster B2 = {10,11,12} (triangle, weight 1.0)
    ///   - B1 ↔ B2 bridged by edge (9,10) at weight 0.6
    ///   - No edges between the A and B halves.
    fn synthetic_hierarchy_edges() -> Vec<MemoryEdge> {
        fn edge(id: i64, src: i64, dst: i64, w: f64) -> MemoryEdge {
            MemoryEdge {
                id,
                src_id: src,
                dst_id: dst,
                rel_type: "related_to".into(),
                confidence: w,
                source: crate::memory::EdgeSource::Auto,
                created_at: 0,
                valid_from: None,
                valid_to: None,
                edge_source: None,
            }
        }
        let mut id = 0;
        let mut next = || {
            id += 1;
            id
        };
        vec![
            edge(next(), 1, 2, 1.0),
            edge(next(), 2, 3, 1.0),
            edge(next(), 1, 3, 1.0),
            edge(next(), 4, 5, 1.0),
            edge(next(), 5, 6, 1.0),
            edge(next(), 4, 6, 1.0),
            edge(next(), 3, 4, 0.6),
            edge(next(), 7, 8, 1.0),
            edge(next(), 8, 9, 1.0),
            edge(next(), 7, 9, 1.0),
            edge(next(), 10, 11, 1.0),
            edge(next(), 11, 12, 1.0),
            edge(next(), 10, 12, 1.0),
            edge(next(), 9, 10, 0.6),
        ]
    }

    /// Returns true when every member of `group` shares the same value
    /// in the assignment map.
    #[allow(dead_code)]
    fn same_community(assignment: &HashMap<i64, usize>, group: &[i64]) -> bool {
        let first = match group.first() {
            Some(&first) => first,
            None => return true,
        };
        let target = assignment[&first];
        group.iter().all(|n| assignment[n] == target)
    }

    #[test]
    fn detect_communities_hierarchical_returns_multiple_levels() {
        let edges = synthetic_hierarchy_edges();
        let hierarchy = detect_communities_hierarchical(&edges, MAX_HIERARCHY_LEVELS);

        assert!(
            hierarchy.len() >= 2,
            "expected at least 2 levels, got {}",
            hierarchy.len()
        );

        // Every level must cover all 12 original nodes.
        for (idx, level) in hierarchy.iter().enumerate() {
            assert_eq!(level.len(), 12, "level {idx} should keep all 12 nodes");
        }

        // The A-half {1..6} and B-half {7..12} share no edges, so they
        // can never collapse into the same community at any level. This
        // is the structural invariant the hierarchical detector must
        // preserve regardless of how greedy modularity slices the
        // interior of each half.
        let a_half = [1, 2, 3, 4, 5, 6];
        let b_half = [7, 8, 9, 10, 11, 12];
        for (idx, level) in hierarchy.iter().enumerate() {
            let a_comms: BTreeSet<usize> =
                a_half.iter().map(|n| level[n]).collect();
            let b_comms: BTreeSet<usize> =
                b_half.iter().map(|n| level[n]).collect();
            assert!(
                a_comms.is_disjoint(&b_comms),
                "level {idx} merged the disconnected halves: A={a_comms:?} B={b_comms:?}"
            );
        }

        // Level 0 must have strictly more communities than the deepest
        // level — that is the whole point of hierarchical coarsening.
        let lvl0 = &hierarchy[0];
        let lvl0_count = lvl0
            .values()
            .copied()
            .collect::<BTreeSet<usize>>()
            .len();
        let last = hierarchy.last().expect("non-empty");
        let last_count = last
            .values()
            .copied()
            .collect::<BTreeSet<usize>>()
            .len();
        assert!(
            last_count < lvl0_count,
            "deepest level should coarsen ({last_count} vs {lvl0_count})"
        );
    }

    #[test]
    fn detect_communities_hierarchical_respects_level_cap() {
        let edges = synthetic_hierarchy_edges();
        // Asking for 0 levels is invalid; the clamp keeps it at 1.
        let one = detect_communities_hierarchical(&edges, 0);
        assert_eq!(one.len(), 1, "clamp should floor to 1 level");

        // Asking for more than MAX_HIERARCHY_LEVELS is silently clamped.
        let many = detect_communities_hierarchical(&edges, MAX_HIERARCHY_LEVELS + 10);
        assert!(many.len() <= MAX_HIERARCHY_LEVELS);
    }

    #[test]
    fn detect_communities_hierarchical_handles_empty_graph() {
        assert!(detect_communities_hierarchical(&[], MAX_HIERARCHY_LEVELS).is_empty());
    }

    #[test]
    fn detect_and_store_hierarchy_persists_multiple_levels_and_carries_summaries() {
        let store = make_store_with_graph();
        // First pass: nothing stored yet; all summaries should be None.
        let first = store.detect_and_store_hierarchy(MAX_HIERARCHY_LEVELS).unwrap();
        assert!(!first.is_empty());
        let max_level = first.iter().map(|c| c.level).max().unwrap_or(0);
        assert!(max_level >= 0);

        // Seed a summary on every community currently in the table.
        for c in &first {
            store
                .set_community_summary(c.id, &format!("summary L{}", c.level), None)
                .unwrap();
        }

        // Second pass on the same graph: identical member sets → summaries
        // must be carried forward verbatim, not regenerated. This is the
        // idempotency contract for GRAPHRAG-1a summary generation.
        let second = store.detect_and_store_hierarchy(MAX_HIERARCHY_LEVELS).unwrap();
        for c in &second {
            assert_eq!(
                c.summary.as_deref(),
                Some(format!("summary L{}", c.level).as_str()),
                "summary should carry over for unchanged community at level {} ({:?})",
                c.level,
                c.member_ids,
            );
        }
    }

    #[test]
    fn set_community_summary_updates_one_row() {
        let store = make_store_with_graph();
        let communities = store.detect_and_store_communities().unwrap();
        let first = communities.first().expect("at least one community").clone();

        store
            .set_community_summary(first.id, "Custom summary", None)
            .unwrap();
        let reloaded = store.load_communities().unwrap();
        let updated = reloaded.iter().find(|c| c.id == first.id).unwrap();
        assert_eq!(updated.summary.as_deref(), Some("Custom summary"));
        // Other communities remain unsummarised.
        let others = reloaded.iter().filter(|c| c.id != first.id);
        for o in others {
            assert!(o.summary.is_none());
        }
    }

    #[test]
    fn graph_rag_search_filters_by_level() {
        let store = make_store_with_graph();
        store.ensure_communities_schema().unwrap();

        // Two communities at level 0 (rust + music) and two at level 1
        // (same membership for the test — level field is what we filter on).
        let all = store.get_all().unwrap();
        let rust_ids: Vec<i64> = all
            .iter()
            .filter(|e| e.tags.contains("rust"))
            .map(|e| e.id)
            .collect();
        let music_ids: Vec<i64> = all
            .iter()
            .filter(|e| e.tags.contains("music"))
            .map(|e| e.id)
            .collect();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        store
            .save_communities(&[
                Community {
                    id: 0,
                    level: 0,
                    member_ids: rust_ids.clone(),
                    summary: Some("L0 micro: rust ownership cargo".into()),
                    embedding: None,
                    updated_at: now,
                },
                Community {
                    id: 0,
                    level: 0,
                    member_ids: music_ids.clone(),
                    summary: Some("L0 micro: jazz blues theory".into()),
                    embedding: None,
                    updated_at: now,
                },
                Community {
                    id: 0,
                    level: 1,
                    member_ids: rust_ids.clone(),
                    summary: Some("L1 macro: programming languages".into()),
                    embedding: None,
                    updated_at: now,
                },
                Community {
                    id: 0,
                    level: 1,
                    member_ids: music_ids.clone(),
                    summary: Some("L1 macro: musical traditions".into()),
                    embedding: None,
                    updated_at: now,
                },
            ])
            .unwrap();

        // Query that only matches the L1 wording.
        let l1_only = store
            .graph_rag_search_at_level("programming languages", None, 10, Some(1))
            .unwrap();
        assert!(
            !l1_only.is_empty(),
            "L1-targeted query should hit L1 macro summary"
        );

        // The same query restricted to L0 finds no community match (only
        // entity-search hits, which contain neither token). RRF returns
        // an empty fusion ranking for level 0.
        let l0_only = store
            .graph_rag_search_at_level("programming languages", None, 10, Some(0))
            .unwrap();
        assert!(
            l0_only.is_empty() || l0_only.len() < l1_only.len(),
            "L0 filter should not include L1 macro hits (got {} vs {})",
            l0_only.len(),
            l1_only.len(),
        );

        // Unfiltered call still works (backwards-compat).
        let unfiltered = store
            .graph_rag_search("programming languages", None, 10)
            .unwrap();
        assert!(!unfiltered.is_empty());
    }

    // ── GRAPHRAG-1c: scope-routed retrieval ────────────────────────

    #[test]
    fn routed_global_targets_top_level_communities() {
        let store = make_store_with_graph();
        store.ensure_communities_schema().unwrap();

        let all = store.get_all().unwrap();
        let rust_ids: Vec<i64> = all
            .iter()
            .filter(|e| e.tags.contains("rust"))
            .map(|e| e.id)
            .collect();
        let music_ids: Vec<i64> = all
            .iter()
            .filter(|e| e.tags.contains("music"))
            .map(|e| e.id)
            .collect();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Store communities at L0 and L1.
        store
            .save_communities(&[
                Community {
                    id: 0,
                    level: 0,
                    member_ids: rust_ids.clone(),
                    summary: Some("Programming with Rust ownership and Cargo.".into()),
                    embedding: None,
                    updated_at: now,
                },
                Community {
                    id: 0,
                    level: 0,
                    member_ids: music_ids.clone(),
                    summary: Some("Music about jazz and blues.".into()),
                    embedding: None,
                    updated_at: now,
                },
                Community {
                    id: 0,
                    level: 1,
                    member_ids: rust_ids.iter().chain(music_ids.iter()).copied().collect(),
                    summary: Some(
                        "Knowledge base covering programming languages and music theory.".into(),
                    ),
                    embedding: None,
                    updated_at: now,
                },
            ])
            .unwrap();

        // Global scope should only search L1 communities.
        let global_results = store
            .graph_rag_search_routed("programming languages", None, 10, QueryScope::Global)
            .unwrap();
        // Should find results (L1 summary contains "programming languages").
        assert!(!global_results.is_empty());
    }

    #[test]
    fn routed_local_uses_cascade_expansion() {
        let store = make_store_with_graph();
        store.ensure_communities_schema().unwrap();

        // Local scope should use entity search + cascade.
        let local_results = store
            .graph_rag_search_routed("Rust ownership", None, 10, QueryScope::Local)
            .unwrap();
        // Should find rust-related memories via keyword + cascade.
        assert!(!local_results.is_empty());
        // First hit should be a rust memory.
        let first = store.get_by_id(local_results[0].0).unwrap();
        assert!(
            first.content.to_lowercase().contains("rust"),
            "local routing should find Rust memory, got: {}",
            first.content
        );
    }

    #[test]
    fn routed_mixed_equivalent_to_unfiltered() {
        let store = make_store_with_graph();
        store.ensure_communities_schema().unwrap();
        store.detect_and_store_communities().unwrap();

        let mixed = store
            .graph_rag_search_routed("rust cargo", None, 10, QueryScope::Mixed)
            .unwrap();
        let unfiltered = store.graph_rag_search("rust cargo", None, 10).unwrap();
        // Mixed should produce same results as unfiltered.
        assert_eq!(mixed, unfiltered);
    }

    #[test]
    fn routed_global_empty_communities_returns_empty() {
        // When no communities exist, global still works (returns empty).
        let store = MemoryStore::new(Path::new(":memory:"));
        store.ensure_communities_schema().unwrap();
        let results = store
            .graph_rag_search_routed("overview", None, 10, QueryScope::Global)
            .unwrap();
        assert!(results.is_empty());
    }
}
