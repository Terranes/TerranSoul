//! GraphRAG / LightRAG community summaries (Chunk 16.6).
//!
//! Implements Leiden-style community detection over `memory_edges`,
//! LLM-generated community summaries, and dual-level retrieval (entity +
//! community) fused via RRF.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

use super::edges::MemoryEdge;
use super::fusion::{reciprocal_rank_fuse, DEFAULT_RRF_K};
use super::store::{cosine_similarity, MemoryStore};

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
    if edges.is_empty() {
        return HashMap::new();
    }

    // Build adjacency list with weights.
    let mut adjacency: HashMap<i64, Vec<(i64, f64)>> = HashMap::new();
    let mut total_weight = 0.0_f64;

    for e in edges {
        let w = e.confidence;
        adjacency.entry(e.src_id).or_default().push((e.dst_id, w));
        adjacency.entry(e.dst_id).or_default().push((e.src_id, w));
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
        self.ensure_communities_schema()?;
        let edges = self.list_edges()?;
        let assignments = detect_communities(&edges);

        // Group by community.
        let mut groups: HashMap<usize, Vec<i64>> = HashMap::new();
        for (&node, &comm) in &assignments {
            groups.entry(comm).or_default().push(node);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let communities: Vec<Community> = groups
            .into_values()
            .map(|members| Community {
                id: 0, // assigned by DB
                level: 0,
                member_ids: members,
                summary: None,
                embedding: None,
                updated_at: now,
            })
            .collect();

        self.save_communities(&communities)?;
        self.load_communities()
    }

    /// Dual-level GraphRAG retrieval: entity (memory) search + community
    /// summary search, fused via RRF.
    ///
    /// Returns `(memory_id, score)` pairs sorted descending.
    pub fn graph_rag_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> SqlResult<Vec<(i64, f64)>> {
        // Level 1: entity-level keyword search.
        let entity_hits = self.search(query)?;
        let entity_ranking: Vec<i64> = entity_hits.iter().take(limit * 2).map(|e| e.id).collect();

        // Level 2: community-level search.
        let communities = self.load_communities()?;
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
}
