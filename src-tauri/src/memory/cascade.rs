//! Cascade retrieval through `memory_edges` (Chunk 43.5).
//!
//! After RRF top-K, a BFS (depth ≤ 2) walks knowledge-graph edges and
//! adds neighbours at decayed scores:  `seed_rrf_score × edge_prior × 0.7^depth`.
//!
//! Edge priors:
//!
//! | Relation     | Prior |
//! |--------------|-------|
//! | supersedes   | 0.9   |
//! | has_tag      | 0.8   |
//! | relates_to   | edge confidence |
//! | in_cluster   | 0.6   |
//! | contradicts  | 0.3   |
//! | derived_from | 0.3   |
//! | *other*      | 0.5   |

use std::collections::{HashMap, HashSet, VecDeque};

use rusqlite::{params, Connection, Result as SqlResult};

/// Maximum BFS depth for cascade expansion.
const MAX_CASCADE_DEPTH: usize = 2;

/// Per-hop decay factor.
const HOP_DECAY: f64 = 0.7;

/// Edge-type prior weights.
fn edge_prior(rel_type: &str, edge_confidence: f64) -> f64 {
    match rel_type {
        "supersedes" => 0.9,
        "has_tag" => 0.8,
        "relates_to" | "related_to" => edge_confidence.clamp(0.0, 1.0),
        "in_cluster" => 0.6,
        "contradicts" | "derived_from" => 0.3,
        _ => 0.5,
    }
}

/// Result of cascade expansion: (memory_id, cascade_score).
/// Entries already in the seed set keep their original score.
/// Neighbours discovered via BFS get `seed_score × prior × 0.7^depth`.
pub fn cascade_expand(
    conn: &Connection,
    seeds: &[(i64, f64)],
    max_depth: Option<usize>,
) -> SqlResult<Vec<(i64, f64)>> {
    let depth_limit = max_depth.unwrap_or(MAX_CASCADE_DEPTH);
    if depth_limit == 0 || seeds.is_empty() {
        return Ok(seeds.to_vec());
    }

    // Best score seen per memory id.
    let mut scores: HashMap<i64, f64> = HashMap::new();
    for &(id, score) in seeds {
        scores.insert(id, score);
    }

    // BFS queue: (node_id, current_depth, incoming_score).
    let mut queue: VecDeque<(i64, usize, f64)> = VecDeque::new();
    let mut visited: HashSet<i64> = seeds.iter().map(|&(id, _)| id).collect();

    for &(id, score) in seeds {
        queue.push_back((id, 0, score));
    }

    while let Some((node, depth, node_score)) = queue.pop_front() {
        if depth >= depth_limit {
            continue;
        }

        // Fetch outgoing edges from this node (bidirectional).
        let mut stmt = conn.prepare_cached(
            "SELECT src_id, dst_id, rel_type, confidence
             FROM memory_edges
             WHERE (src_id = ?1 OR dst_id = ?1)
               AND valid_to IS NULL",
        )?;
        let rows = stmt.query_map(params![node], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })?;

        for r in rows.flatten() {
            let (src, dst, rel_type, conf) = r;
            let neighbour = if src == node { dst } else { src };
            let prior = edge_prior(&rel_type, conf);
            let cascade_score = node_score * prior * HOP_DECAY;

            // Only keep the best score for each neighbour.
            let entry = scores.entry(neighbour).or_insert(0.0);
            if cascade_score > *entry {
                *entry = cascade_score;
            }

            if visited.insert(neighbour) {
                queue.push_back((neighbour, depth + 1, cascade_score));
            }
        }
    }

    // Sort by score descending.
    let mut result: Vec<(i64, f64)> = scores.into_iter().collect();
    result.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::schema::create_canonical_schema;
    use rusqlite::Connection;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_canonical_schema(&conn).unwrap();

        // Insert 4 memories
        for i in 1..=4 {
            conn.execute(
                "INSERT INTO memories (id, content, created_at) VALUES (?1, ?2, ?3)",
                params![i, format!("mem-{i}"), 1000 + i],
            )
            .unwrap();
        }

        // Edges: 1 --supersedes--> 2, 1 --relates_to(0.8)--> 3, 3 --contradicts--> 4
        conn.execute(
            "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
             VALUES (1, 2, 'supersedes', 1.0, 'user', 100)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
             VALUES (1, 3, 'related_to', 0.8, 'llm', 101)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
             VALUES (3, 4, 'contradicts', 1.0, 'llm', 102)",
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn cascade_with_depth_zero_returns_seeds() {
        let conn = setup();
        let seeds = vec![(1, 1.0)];
        let result = cascade_expand(&conn, &seeds, Some(0)).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 1);
    }

    #[test]
    fn cascade_depth_one_finds_neighbours() {
        let conn = setup();
        let seeds = vec![(1, 1.0)];
        let result = cascade_expand(&conn, &seeds, Some(1)).unwrap();

        // Should find: seed 1 (1.0), neighbour 2 (1.0 * 0.9 * 0.7 = 0.63),
        // neighbour 3 (1.0 * 0.8 * 0.7 = 0.56). Not 4 (depth 2).
        assert_eq!(result.len(), 3);
        let ids: Vec<i64> = result.iter().map(|r| r.0).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));
        assert!(!ids.contains(&4));
    }

    #[test]
    fn cascade_depth_two_reaches_transitive() {
        let conn = setup();
        let seeds = vec![(1, 1.0)];
        let result = cascade_expand(&conn, &seeds, Some(2)).unwrap();

        // Should reach 4 through: 1 → 3 → 4
        let ids: Vec<i64> = result.iter().map(|r| r.0).collect();
        assert!(ids.contains(&4), "depth-2 should reach node 4");
    }

    #[test]
    fn cascade_scores_decay_with_depth() {
        let conn = setup();
        let seeds = vec![(1, 1.0)];
        let result = cascade_expand(&conn, &seeds, Some(2)).unwrap();

        let score_map: HashMap<i64, f64> = result.into_iter().collect();

        // Seed keeps full score
        assert!((score_map[&1] - 1.0).abs() < f64::EPSILON);

        // Node 2: supersedes prior 0.9, depth 1: 1.0 * 0.9 * 0.7 = 0.63
        assert!(
            (score_map[&2] - 0.63).abs() < 0.01,
            "node 2 score should be ~0.63, got {}",
            score_map[&2]
        );

        // Node 3: related_to prior 0.8, depth 1: 1.0 * 0.8 * 0.7 = 0.56
        assert!(
            (score_map[&3] - 0.56).abs() < 0.01,
            "node 3 score should be ~0.56, got {}",
            score_map[&3]
        );

        // Node 4: contradicts prior 0.3, depth 2 from node 3 (0.56):
        // 0.56 * 0.3 * 0.7 = 0.1176
        assert!(
            (score_map[&4] - 0.1176).abs() < 0.02,
            "node 4 score should be ~0.12, got {}",
            score_map[&4]
        );
    }

    #[test]
    fn cascade_empty_seeds_returns_empty() {
        let conn = setup();
        let result = cascade_expand(&conn, &[], None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn cascade_sorted_by_score_descending() {
        let conn = setup();
        let seeds = vec![(1, 1.0)];
        let result = cascade_expand(&conn, &seeds, Some(2)).unwrap();

        for w in result.windows(2) {
            assert!(w[0].1 >= w[1].1, "results should be sorted desc by score");
        }
    }
}
