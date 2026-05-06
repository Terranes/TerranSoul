//! Functional clustering + entry-point scoring + execution-flow tracing (Chunk 31.5).
//!
//! Loads the symbol/edge graph from `code_index.sqlite` into petgraph,
//! runs community detection (label-propagation — lightweight Louvain alternative),
//! scores entry points via in-degree + name heuristics, and traces execution
//! flows via BFS along CALLS edges from each entry point.
//!
//! Persists results in `code_clusters` and `code_processes` tables.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::symbol_index::{open_db, IndexError};

// ─── Public types ───────────────────────────────────────────────────────────

/// A cluster of related symbols (detected community).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: u32,
    pub label: String,
    pub symbol_ids: Vec<i64>,
    pub size: u32,
}

/// An entry point with its score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    pub symbol_id: i64,
    pub name: String,
    pub file: String,
    pub line: u32,
    pub score: f64,
}

/// A step in an execution-flow trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStep {
    pub symbol_id: i64,
    pub name: String,
    pub file: String,
    pub line: u32,
    pub depth: u32,
}

/// A traced execution flow from an entry point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Process {
    pub id: u32,
    pub entry_point: String,
    pub entry_symbol_id: i64,
    pub steps: Vec<ProcessStep>,
}

/// Stats from the clustering + process tracing pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStats {
    pub clusters_found: u32,
    pub entry_points_found: u32,
    pub processes_traced: u32,
    pub total_steps: u32,
}

// ─── Schema extension ───────────────────────────────────────────────────────

fn ensure_process_tables(conn: &Connection) -> Result<(), IndexError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS code_clusters (
            id          INTEGER PRIMARY KEY,
            repo_id     INTEGER NOT NULL REFERENCES code_repos(id) ON DELETE CASCADE,
            cluster_id  INTEGER NOT NULL,
            label       TEXT NOT NULL,
            UNIQUE(repo_id, cluster_id)
        );

        CREATE TABLE IF NOT EXISTS code_cluster_members (
            id          INTEGER PRIMARY KEY,
            repo_id     INTEGER NOT NULL REFERENCES code_repos(id) ON DELETE CASCADE,
            cluster_id  INTEGER NOT NULL,
            symbol_id   INTEGER NOT NULL REFERENCES code_symbols(id) ON DELETE CASCADE,
            UNIQUE(repo_id, cluster_id, symbol_id)
        );

        CREATE INDEX IF NOT EXISTS idx_cluster_members_cluster
            ON code_cluster_members(repo_id, cluster_id);
        CREATE INDEX IF NOT EXISTS idx_cluster_members_symbol
            ON code_cluster_members(symbol_id);

        CREATE TABLE IF NOT EXISTS code_processes (
            id              INTEGER PRIMARY KEY,
            repo_id         INTEGER NOT NULL REFERENCES code_repos(id) ON DELETE CASCADE,
            process_id      INTEGER NOT NULL,
            entry_symbol_id INTEGER NOT NULL REFERENCES code_symbols(id) ON DELETE CASCADE,
            entry_name      TEXT NOT NULL,
            UNIQUE(repo_id, process_id)
        );

        CREATE TABLE IF NOT EXISTS code_process_steps (
            id          INTEGER PRIMARY KEY,
            repo_id     INTEGER NOT NULL REFERENCES code_repos(id) ON DELETE CASCADE,
            process_id  INTEGER NOT NULL,
            symbol_id   INTEGER NOT NULL REFERENCES code_symbols(id) ON DELETE CASCADE,
            depth       INTEGER NOT NULL,
            step_order  INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_process_steps_proc
            ON code_process_steps(repo_id, process_id);
        "#,
    )?;
    Ok(())
}

// ─── Main entry ─────────────────────────────────────────────────────────────

/// Run the full clustering + process-tracing pipeline for a repo.
///
/// Requires the repo to have been indexed first via `index_repo` + `resolve_edges`.
/// Maximum BFS depth for process tracing is capped at `max_depth`.
pub fn compute_processes(
    data_dir: &Path,
    repo_path: &Path,
    max_depth: u32,
) -> Result<ProcessStats, IndexError> {
    let repo_path = repo_path
        .canonicalize()
        .map_err(|e| IndexError::InvalidPath(format!("{}: {e}", repo_path.display())))?;
    let repo_path_str = repo_path.to_string_lossy().to_string();

    let conn = open_db(data_dir)?;
    ensure_process_tables(&conn)?;

    let repo_id: i64 = conn
        .query_row(
            "SELECT id FROM code_repos WHERE path = ?1",
            params![repo_path_str],
            |r| r.get(0),
        )
        .map_err(|_| {
            IndexError::InvalidPath(format!("repo not indexed: {}", repo_path.display()))
        })?;

    // Clear previous results.
    conn.execute(
        "DELETE FROM code_process_steps WHERE repo_id = ?1",
        params![repo_id],
    )?;
    conn.execute(
        "DELETE FROM code_processes WHERE repo_id = ?1",
        params![repo_id],
    )?;
    conn.execute(
        "DELETE FROM code_cluster_members WHERE repo_id = ?1",
        params![repo_id],
    )?;
    conn.execute(
        "DELETE FROM code_clusters WHERE repo_id = ?1",
        params![repo_id],
    )?;

    // Load symbols + edges into petgraph.
    let (graph, node_map, sym_info) = build_call_graph(&conn, repo_id)?;

    // 1. Community detection (label propagation).
    let clusters = label_propagation(&graph, &node_map, &sym_info);

    // 2. Entry-point scoring.
    let entry_points = score_entry_points(&graph, &node_map, &sym_info);

    // 3. BFS process tracing from each entry point.
    let processes = trace_processes(&graph, &node_map, &sym_info, &entry_points, max_depth);

    // Persist results.
    let tx = conn.unchecked_transaction()?;

    for cluster in &clusters {
        tx.execute(
            "INSERT OR IGNORE INTO code_clusters (repo_id, cluster_id, label) VALUES (?1, ?2, ?3)",
            params![repo_id, cluster.id, cluster.label],
        )?;
        for &sym_id in &cluster.symbol_ids {
            tx.execute(
                "INSERT OR IGNORE INTO code_cluster_members (repo_id, cluster_id, symbol_id) VALUES (?1, ?2, ?3)",
                params![repo_id, cluster.id, sym_id],
            )?;
        }
    }

    let mut total_steps: u32 = 0;
    for process in &processes {
        tx.execute(
            "INSERT OR IGNORE INTO code_processes (repo_id, process_id, entry_symbol_id, entry_name) VALUES (?1, ?2, ?3, ?4)",
            params![repo_id, process.id, process.entry_symbol_id, process.entry_point],
        )?;
        for (order, step) in process.steps.iter().enumerate() {
            tx.execute(
                "INSERT INTO code_process_steps (repo_id, process_id, symbol_id, depth, step_order) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![repo_id, process.id, step.symbol_id, step.depth, order as i64],
            )?;
            total_steps += 1;
        }
    }

    tx.commit()?;

    Ok(ProcessStats {
        clusters_found: clusters.len() as u32,
        entry_points_found: entry_points.len() as u32,
        processes_traced: processes.len() as u32,
        total_steps,
    })
}

// ─── Query helpers ──────────────────────────────────────────────────────────

/// List clusters for a repo.
pub fn list_clusters(conn: &Connection, repo_id: i64) -> Result<Vec<Cluster>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT c.cluster_id, c.label, m.symbol_id
         FROM code_clusters c
         JOIN code_cluster_members m ON m.repo_id = c.repo_id AND m.cluster_id = c.cluster_id
         WHERE c.repo_id = ?1
         ORDER BY c.cluster_id, m.symbol_id",
    )?;
    let rows: Vec<(u32, String, i64)> = stmt
        .query_map(params![repo_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .filter_map(|r| r.ok())
        .collect();

    let mut clusters_map: HashMap<u32, Cluster> = HashMap::new();
    for (cid, label, sym_id) in rows {
        let entry = clusters_map.entry(cid).or_insert_with(|| Cluster {
            id: cid,
            label: label.clone(),
            symbol_ids: Vec::new(),
            size: 0,
        });
        entry.symbol_ids.push(sym_id);
        entry.size += 1;
    }

    let mut result: Vec<Cluster> = clusters_map.into_values().collect();
    result.sort_by_key(|c| c.id);
    Ok(result)
}

/// List processes for a repo.
pub fn list_processes(conn: &Connection, repo_id: i64) -> Result<Vec<Process>, IndexError> {
    let mut proc_stmt = conn.prepare(
        "SELECT process_id, entry_name, entry_symbol_id FROM code_processes WHERE repo_id = ?1 ORDER BY process_id",
    )?;
    let procs: Vec<(u32, String, i64)> = proc_stmt
        .query_map(params![repo_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .filter_map(|r| r.ok())
        .collect();

    let mut step_stmt = conn.prepare(
        "SELECT ps.symbol_id, ps.depth, s.name, s.file, s.line
         FROM code_process_steps ps
         JOIN code_symbols s ON s.id = ps.symbol_id
         WHERE ps.repo_id = ?1 AND ps.process_id = ?2
         ORDER BY ps.step_order",
    )?;

    let mut result = Vec::new();
    for (pid, entry_name, entry_sym_id) in procs {
        let steps: Vec<ProcessStep> = step_stmt
            .query_map(params![repo_id, pid], |r| {
                Ok(ProcessStep {
                    symbol_id: r.get(0)?,
                    name: r.get(2)?,
                    file: r.get(3)?,
                    line: r.get(4)?,
                    depth: r.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        result.push(Process {
            id: pid,
            entry_point: entry_name,
            entry_symbol_id: entry_sym_id,
            steps,
        });
    }
    Ok(result)
}

// ─── Graph construction ─────────────────────────────────────────────────────

/// Info about a symbol node in the graph.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SymInfo {
    id: i64,
    name: String,
    file: String,
    line: u32,
    kind: String,
}

/// Graph data needed for clustering and process tracing.
type CallGraphData = (
    DiGraph<i64, ()>,
    HashMap<i64, NodeIndex>,
    HashMap<i64, SymInfo>,
);

/// Build a directed call graph from the DB.
/// Returns: (graph, node_index→sym_id map, sym_id→SymInfo map)
fn build_call_graph(conn: &Connection, repo_id: i64) -> Result<CallGraphData, IndexError> {
    let mut graph = DiGraph::new();
    let mut node_map: HashMap<i64, NodeIndex> = HashMap::new();
    let mut sym_info: HashMap<i64, SymInfo> = HashMap::new();

    // Load all symbols.
    let mut stmt =
        conn.prepare("SELECT id, name, kind, file, line FROM code_symbols WHERE repo_id = ?1")?;
    let symbols: Vec<(i64, String, String, String, u32)> = stmt
        .query_map(params![repo_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (id, name, kind, file, line) in symbols {
        let idx = graph.add_node(id);
        node_map.insert(id, idx);
        sym_info.insert(
            id,
            SymInfo {
                id,
                name,
                file,
                line,
                kind,
            },
        );
    }

    // Load resolved CALLS + heritage edges for graph construction.
    // Calls for flow tracing; heritage (implements/extends) for clustering affinity.
    let mut edge_stmt = conn.prepare(
        "SELECT from_symbol_id, target_symbol_id FROM code_edges
         WHERE repo_id = ?1 AND kind IN ('calls', 'implements', 'extends')
         AND from_symbol_id IS NOT NULL AND target_symbol_id IS NOT NULL",
    )?;
    let edges: Vec<(i64, i64)> = edge_stmt
        .query_map(params![repo_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    for (from_id, to_id) in edges {
        if let (Some(&from_idx), Some(&to_idx)) = (node_map.get(&from_id), node_map.get(&to_id)) {
            // Avoid self-loops.
            if from_idx != to_idx {
                graph.add_edge(from_idx, to_idx, ());
            }
        }
    }

    Ok((graph, node_map, sym_info))
}

// ─── Label propagation community detection ──────────────────────────────────

/// Simple label-propagation algorithm for community detection.
/// Each node starts with its own label; iteratively adopts the most common
/// label among its neighbors. Converges in a few iterations for typical
/// code graphs. Treats the graph as undirected for clustering.
fn label_propagation(
    graph: &DiGraph<i64, ()>,
    node_map: &HashMap<i64, NodeIndex>,
    sym_info: &HashMap<i64, SymInfo>,
) -> Vec<Cluster> {
    if graph.node_count() == 0 {
        return Vec::new();
    }

    // Initialize: each node gets its own label.
    let mut labels: HashMap<NodeIndex, u32> = HashMap::new();
    let node_indices: Vec<NodeIndex> = node_map.values().copied().collect();
    for (i, &idx) in node_indices.iter().enumerate() {
        labels.insert(idx, i as u32);
    }

    // Iterate up to 20 rounds (typically converges in 3-5).
    let max_iterations = 20;
    for _ in 0..max_iterations {
        let mut changed = false;

        for &idx in &node_indices {
            // Count labels of all neighbors (both directions — undirected view).
            let mut label_counts: HashMap<u32, u32> = HashMap::new();
            for neighbor in graph.neighbors_directed(idx, Direction::Outgoing) {
                if let Some(&lbl) = labels.get(&neighbor) {
                    *label_counts.entry(lbl).or_insert(0) += 1;
                }
            }
            for neighbor in graph.neighbors_directed(idx, Direction::Incoming) {
                if let Some(&lbl) = labels.get(&neighbor) {
                    *label_counts.entry(lbl).or_insert(0) += 1;
                }
            }

            // Adopt the most common neighbor label (break ties by keeping current).
            if let Some((&best_label, &best_count)) =
                label_counts.iter().max_by_key(|(_, &count)| count)
            {
                let current = labels[&idx];
                // Only switch if the majority label is different and has > 1 vote.
                if best_label != current && best_count > 1 {
                    labels.insert(idx, best_label);
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    // Group by label.
    let mut groups: HashMap<u32, Vec<i64>> = HashMap::new();
    for (&idx, &label) in &labels {
        let sym_id = graph[idx];
        groups.entry(label).or_default().push(sym_id);
    }

    // Assign sequential cluster IDs and generate labels.
    let mut clusters: Vec<Cluster> = groups
        .into_values()
        .filter(|members| !members.is_empty())
        .enumerate()
        .map(|(i, symbol_ids)| {
            let label = generate_cluster_label(&symbol_ids, sym_info);
            Cluster {
                id: i as u32,
                label,
                size: symbol_ids.len() as u32,
                symbol_ids,
            }
        })
        .collect();

    // Sort by size descending.
    clusters.sort_by_key(|c| std::cmp::Reverse(c.size));

    // Re-number after sort.
    for (i, c) in clusters.iter_mut().enumerate() {
        c.id = i as u32;
    }

    clusters
}

/// Generate a human-readable label for a cluster from its members.
fn generate_cluster_label(symbol_ids: &[i64], sym_info: &HashMap<i64, SymInfo>) -> String {
    // Use the most common file prefix among members.
    let mut file_counts: HashMap<&str, u32> = HashMap::new();
    for &id in symbol_ids {
        if let Some(info) = sym_info.get(&id) {
            // Use the first path segment after src/ or the file stem.
            let key = info
                .file
                .strip_prefix("src/")
                .or_else(|| info.file.strip_prefix("src-tauri/src/"))
                .unwrap_or(&info.file);
            let segment = key.split('/').next().unwrap_or(key);
            *file_counts.entry(segment).or_insert(0) += 1;
        }
    }

    file_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(seg, _)| seg.to_string())
        .unwrap_or_else(|| format!("cluster_{}", symbol_ids.len()))
}

// ─── Entry-point scoring ────────────────────────────────────────────────────

/// Score symbols as potential entry points.
/// Heuristics:
/// - Name matches: `main`, `run_*`, `start_*`, `handle_*`, `init_*` → bonus
/// - Kind is `function` (not `method`) → small bonus
/// - In-degree = 0 (no callers) → big bonus
/// - High out-degree (calls many things) → bonus
fn score_entry_points(
    graph: &DiGraph<i64, ()>,
    node_map: &HashMap<i64, NodeIndex>,
    sym_info: &HashMap<i64, SymInfo>,
) -> Vec<EntryPoint> {
    let mut scored: Vec<EntryPoint> = Vec::new();

    for (&sym_id, &idx) in node_map {
        let info = match sym_info.get(&sym_id) {
            Some(i) => i,
            None => continue,
        };

        // Skip non-callable kinds.
        if !matches!(info.kind.as_str(), "function" | "method") {
            continue;
        }

        let in_degree = graph.neighbors_directed(idx, Direction::Incoming).count();
        let out_degree = graph.neighbors_directed(idx, Direction::Outgoing).count();

        let mut score: f64 = 0.0;

        // Name heuristics.
        let name_lower = info.name.to_lowercase();
        if name_lower == "main" {
            score += 100.0;
        } else if name_lower.starts_with("run_") || name_lower.starts_with("run") {
            score += 50.0;
        } else if name_lower.starts_with("start_") || name_lower.starts_with("start") {
            score += 40.0;
        } else if name_lower.starts_with("handle_") {
            score += 30.0;
        } else if name_lower.starts_with("init_") || name_lower.starts_with("setup_") {
            score += 25.0;
        } else if name_lower.starts_with("serve") || name_lower.starts_with("listen") {
            score += 35.0;
        }

        // In-degree: roots (no callers) are likely entry points.
        if in_degree == 0 && out_degree > 0 {
            score += 60.0;
        } else if in_degree <= 1 {
            score += 20.0;
        }

        // Out-degree: entry points tend to orchestrate.
        score += (out_degree as f64).min(20.0) * 2.0;

        // Function (not method) bonus.
        if info.kind == "function" {
            score += 5.0;
        }

        // Exported/public heuristic: names starting with uppercase (Go)
        // or lacking a leading underscore are more likely entry points.
        if info.name.starts_with(|c: char| c.is_uppercase()) && info.kind == "function" {
            score += 10.0;
        }

        // Threshold: only include if score > 30.
        if score > 30.0 {
            scored.push(EntryPoint {
                symbol_id: sym_id,
                name: info.name.clone(),
                file: info.file.clone(),
                line: info.line,
                score,
            });
        }
    }

    // Sort by score descending.
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Cap at 50 entry points.
    scored.truncate(50);
    scored
}

// ─── Process tracing (BFS) ──────────────────────────────────────────────────

/// Trace execution flows from each entry point via BFS on CALLS edges.
fn trace_processes(
    graph: &DiGraph<i64, ()>,
    node_map: &HashMap<i64, NodeIndex>,
    sym_info: &HashMap<i64, SymInfo>,
    entry_points: &[EntryPoint],
    max_depth: u32,
) -> Vec<Process> {
    let mut processes = Vec::new();

    for (proc_idx, ep) in entry_points.iter().enumerate() {
        let start_idx = match node_map.get(&ep.symbol_id) {
            Some(&idx) => idx,
            None => continue,
        };

        let steps = bfs_trace(graph, start_idx, sym_info, max_depth);

        if !steps.is_empty() {
            processes.push(Process {
                id: proc_idx as u32,
                entry_point: ep.name.clone(),
                entry_symbol_id: ep.symbol_id,
                steps,
            });
        }
    }

    processes
}

/// BFS from a start node, collecting all reachable symbols up to `max_depth`.
fn bfs_trace(
    graph: &DiGraph<i64, ()>,
    start: NodeIndex,
    sym_info: &HashMap<i64, SymInfo>,
    max_depth: u32,
) -> Vec<ProcessStep> {
    let mut visited: HashSet<NodeIndex> = HashSet::new();
    let mut queue: VecDeque<(NodeIndex, u32)> = VecDeque::new();
    let mut steps: Vec<ProcessStep> = Vec::new();

    visited.insert(start);
    queue.push_back((start, 0));

    while let Some((current, depth)) = queue.pop_front() {
        let sym_id = graph[current];
        if let Some(info) = sym_info.get(&sym_id) {
            steps.push(ProcessStep {
                symbol_id: sym_id,
                name: info.name.clone(),
                file: info.file.clone(),
                line: info.line,
                depth,
            });
        }

        if depth >= max_depth {
            continue;
        }

        for neighbor in graph.neighbors_directed(current, Direction::Outgoing) {
            if visited.insert(neighbor) {
                queue.push_back((neighbor, depth + 1));
            }
        }
    }

    steps
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::resolver::resolve_edges;
    use crate::coding::symbol_index::index_repo;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, TempDir) {
        let data_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();

        let src = repo_dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        // Main entry point that orchestrates.
        std::fs::write(
            src.join("lib.rs"),
            r#"
mod server;
mod config;
mod utils;

use crate::server::run_http_server;
use crate::config::AppConfig;

pub fn main() {
    let cfg = AppConfig::new();
    run_http_server(cfg);
}
"#,
        )
        .unwrap();

        // Server module.
        std::fs::write(
            src.join("server.rs"),
            r#"
use crate::config::AppConfig;
use crate::utils::format_addr;

pub fn run_http_server(config: AppConfig) {
    let addr = start_server(&config);
    println!("listening on {addr}");
}

fn start_server(config: &AppConfig) -> String {
    format_addr(&config.host, config.port)
}
"#,
        )
        .unwrap();

        // Config module.
        std::fs::write(
            src.join("config.rs"),
            r#"
pub struct AppConfig {
    pub host: String,
    pub port: u16,
}

impl AppConfig {
    pub fn new() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}
"#,
        )
        .unwrap();

        // Utils module — separate cluster.
        std::fs::write(
            src.join("utils.rs"),
            r#"
pub fn format_addr(host: &str, port: u16) -> String {
    format!("{host}:{port}")
}

pub fn parse_port(s: &str) -> u16 {
    s.parse().unwrap_or(8080)
}
"#,
        )
        .unwrap();

        (data_dir, repo_dir)
    }

    #[test]
    fn test_compute_processes_basic() {
        let (data_dir, repo_dir) = setup_test_repo();

        // Index + resolve.
        let stats = index_repo(data_dir.path(), repo_dir.path()).unwrap();
        assert!(stats.symbols_extracted > 0);
        resolve_edges(data_dir.path(), repo_dir.path()).unwrap();

        // Compute processes.
        let proc_stats = compute_processes(data_dir.path(), repo_dir.path(), 10).unwrap();
        assert!(
            proc_stats.clusters_found > 0,
            "should find at least one cluster"
        );
        assert!(
            proc_stats.entry_points_found > 0,
            "should find entry points"
        );
        assert!(
            proc_stats.processes_traced > 0,
            "should trace at least one process"
        );
    }

    #[test]
    fn test_main_calls_run_http_server_in_process() {
        let (data_dir, repo_dir) = setup_test_repo();

        index_repo(data_dir.path(), repo_dir.path()).unwrap();
        resolve_edges(data_dir.path(), repo_dir.path()).unwrap();
        compute_processes(data_dir.path(), repo_dir.path(), 10).unwrap();

        let conn = open_db(data_dir.path()).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        let processes = list_processes(&conn, repo_id).unwrap();

        // Find a process that starts from main.
        let main_proc = processes.iter().find(|p| p.entry_point == "main");
        assert!(
            main_proc.is_some(),
            "should have a process from main: {processes:?}"
        );

        let main_proc = main_proc.unwrap();
        let step_names: Vec<&str> = main_proc.steps.iter().map(|s| s.name.as_str()).collect();
        assert!(
            step_names.contains(&"run_http_server"),
            "main process should contain run_http_server: {step_names:?}"
        );
    }

    #[test]
    fn test_clusters_not_empty() {
        let (data_dir, repo_dir) = setup_test_repo();

        index_repo(data_dir.path(), repo_dir.path()).unwrap();
        resolve_edges(data_dir.path(), repo_dir.path()).unwrap();
        compute_processes(data_dir.path(), repo_dir.path(), 10).unwrap();

        let conn = open_db(data_dir.path()).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        let clusters = list_clusters(&conn, repo_id).unwrap();
        assert!(!clusters.is_empty(), "should have clusters");
        for c in &clusters {
            assert!(c.size > 0, "cluster should not be empty");
            assert!(!c.label.is_empty(), "cluster should have a label");
        }
    }
}
