//! Multi-agent fan-out / DAG runner for coding workflows.
//!
//! Accepts a `WorkflowGraph { nodes, edges }` and executes nodes respecting
//! dependency edges. Independent nodes run in parallel via `tokio::JoinSet`;
//! dependent nodes wait for predecessors to complete.
//!
//! Each node is a `DagNode` with a unique ID, a task description, and optional
//! capabilities required. The runner tracks status per-node and produces a
//! `DagRunResult` summarising pass/fail/skip for the full graph.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single node in the workflow DAG.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DagNode {
    pub id: String,
    pub label: String,
    /// Optional: which capabilities this node requires (e.g. "file_write", "test_run").
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// A directed edge: `from` must complete successfully before `to` can start.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DagEdge {
    pub from: String,
    pub to: String,
}

/// The full workflow graph definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowGraph {
    pub nodes: Vec<DagNode>,
    pub edges: Vec<DagEdge>,
}

/// Status of a single node after execution.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    /// Waiting for dependencies.
    Pending,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Success,
    /// Failed during execution.
    Failed,
    /// Skipped because a predecessor failed.
    Skipped,
}

/// Result for a single node execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    pub node_id: String,
    pub status: NodeStatus,
    /// Human-readable summary or error message.
    pub message: String,
    /// Elapsed time in milliseconds (0 if skipped).
    pub duration_ms: u128,
}

/// Overall result of running the full DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagRunResult {
    pub results: Vec<NodeResult>,
    pub all_success: bool,
    pub total_duration_ms: u128,
    pub failed_nodes: Vec<String>,
    pub skipped_nodes: Vec<String>,
}

/// Configuration for the DAG runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagRunnerConfig {
    /// Maximum number of nodes to execute in parallel.
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
    /// Whether to skip downstream nodes when a predecessor fails.
    #[serde(default = "default_true")]
    pub skip_on_failure: bool,
    /// Capabilities the current environment provides.
    #[serde(default)]
    pub available_capabilities: Vec<String>,
}

fn default_max_parallel() -> usize {
    4
}

fn default_true() -> bool {
    true
}

impl Default for DagRunnerConfig {
    fn default() -> Self {
        Self {
            max_parallel: default_max_parallel(),
            available_capabilities: Vec::new(),
            skip_on_failure: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Errors that can occur when validating a DAG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DagValidationError {
    /// Graph contains a cycle through the listed nodes.
    CycleDetected(Vec<String>),
    /// An edge references a node ID that doesn't exist.
    UnknownNode(String),
    /// Duplicate node IDs found.
    DuplicateNodeId(String),
    /// A node requires capabilities not available in the config.
    MissingCapability { node_id: String, capability: String },
}

impl std::fmt::Display for DagValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CycleDetected(nodes) => {
                write!(f, "Cycle detected through: {}", nodes.join(" → "))
            }
            Self::UnknownNode(id) => write!(f, "Edge references unknown node: {id}"),
            Self::DuplicateNodeId(id) => write!(f, "Duplicate node ID: {id}"),
            Self::MissingCapability {
                node_id,
                capability,
            } => {
                write!(
                    f,
                    "Node {node_id} requires capability '{capability}' not available"
                )
            }
        }
    }
}

impl std::error::Error for DagValidationError {}

/// Validate that the graph is a proper DAG (no cycles, valid references).
pub fn validate_graph(
    graph: &WorkflowGraph,
    config: &DagRunnerConfig,
) -> Result<(), DagValidationError> {
    // Check for duplicate node IDs.
    let mut seen = HashSet::new();
    for node in &graph.nodes {
        if !seen.insert(&node.id) {
            return Err(DagValidationError::DuplicateNodeId(node.id.clone()));
        }
    }

    // Check edges reference existing nodes.
    let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
    for edge in &graph.edges {
        if !node_ids.contains(edge.from.as_str()) {
            return Err(DagValidationError::UnknownNode(edge.from.clone()));
        }
        if !node_ids.contains(edge.to.as_str()) {
            return Err(DagValidationError::UnknownNode(edge.to.clone()));
        }
    }

    // Check capabilities.
    let available: HashSet<&str> = config
        .available_capabilities
        .iter()
        .map(|s| s.as_str())
        .collect();
    for node in &graph.nodes {
        for cap in &node.capabilities {
            if !available.contains(cap.as_str()) && !config.available_capabilities.is_empty() {
                return Err(DagValidationError::MissingCapability {
                    node_id: node.id.clone(),
                    capability: cap.clone(),
                });
            }
        }
    }

    // Cycle detection via topological sort (Kahn's algorithm).
    detect_cycle(graph)?;

    Ok(())
}

/// Returns Err if the graph contains a cycle.
fn detect_cycle(graph: &WorkflowGraph) -> Result<(), DagValidationError> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

    for node in &graph.nodes {
        in_degree.entry(node.id.as_str()).or_insert(0);
        adj.entry(node.id.as_str()).or_default();
    }

    for edge in &graph.edges {
        *in_degree.entry(edge.to.as_str()).or_insert(0) += 1;
        adj.entry(edge.from.as_str())
            .or_default()
            .push(edge.to.as_str());
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut visited = 0usize;

    while let Some(node) = queue.pop_front() {
        visited += 1;
        if let Some(neighbors) = adj.get(node) {
            for &neighbor in neighbors {
                let deg = in_degree.get_mut(neighbor).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(neighbor);
                }
            }
        }
    }

    if visited < graph.nodes.len() {
        // Find nodes still with in-degree > 0 (part of cycle).
        let cycle_nodes: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg > 0)
            .map(|(&id, _)| id.to_owned())
            .collect();
        return Err(DagValidationError::CycleDetected(cycle_nodes));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Execution plan (topological layers)
// ---------------------------------------------------------------------------

/// Compute execution layers: each layer contains nodes whose dependencies are
/// all in previous layers. Nodes within a layer can run in parallel.
pub fn compute_layers(graph: &WorkflowGraph) -> Vec<Vec<String>> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

    for node in &graph.nodes {
        in_degree.entry(node.id.as_str()).or_insert(0);
        adj.entry(node.id.as_str()).or_default();
    }

    for edge in &graph.edges {
        *in_degree.entry(edge.to.as_str()).or_insert(0) += 1;
        adj.entry(edge.from.as_str())
            .or_default()
            .push(edge.to.as_str());
    }

    let mut layers: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();
    current.sort_unstable(); // deterministic order

    while !current.is_empty() {
        layers.push(current.iter().map(|&s| s.to_owned()).collect());
        let mut next: Vec<&str> = Vec::new();
        for &node in &current {
            if let Some(neighbors) = adj.get(node) {
                for &neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        next.push(neighbor);
                    }
                }
            }
        }
        next.sort_unstable();
        current = next;
    }

    layers
}

/// Simulate DAG execution synchronously using a node executor function.
/// This is the core scheduling logic, usable in tests without async.
///
/// `executor` receives a node ID and returns `Ok(message)` on success or
/// `Err(message)` on failure.
pub fn execute_dag_sync<F>(
    graph: &WorkflowGraph,
    config: &DagRunnerConfig,
    mut executor: F,
) -> DagRunResult
where
    F: FnMut(&str) -> Result<String, String>,
{
    let start = std::time::Instant::now();
    let layers = compute_layers(graph);

    let mut results: Vec<NodeResult> = Vec::new();
    let mut failed_set: HashSet<String> = HashSet::new();
    let mut skip_set: HashSet<String> = HashSet::new();

    // Build reverse dependency map: node -> set of predecessors
    let mut predecessors: HashMap<&str, HashSet<&str>> = HashMap::new();
    for node in &graph.nodes {
        predecessors.entry(node.id.as_str()).or_default();
    }
    for edge in &graph.edges {
        predecessors
            .entry(edge.to.as_str())
            .or_default()
            .insert(edge.from.as_str());
    }

    for layer in &layers {
        // Respect max_parallel by chunking the layer.
        for chunk in layer.chunks(config.max_parallel) {
            for node_id in chunk {
                // Check if any predecessor failed.
                let should_skip = config.skip_on_failure
                    && predecessors
                        .get(node_id.as_str())
                        .map(|preds| preds.iter().any(|p| failed_set.contains(*p)))
                        .unwrap_or(false);

                if should_skip {
                    skip_set.insert(node_id.clone());
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        status: NodeStatus::Skipped,
                        message: "Skipped: predecessor failed".to_owned(),
                        duration_ms: 0,
                    });
                    continue;
                }

                let node_start = std::time::Instant::now();
                match executor(node_id) {
                    Ok(msg) => {
                        results.push(NodeResult {
                            node_id: node_id.clone(),
                            status: NodeStatus::Success,
                            message: msg,
                            duration_ms: node_start.elapsed().as_millis(),
                        });
                    }
                    Err(msg) => {
                        failed_set.insert(node_id.clone());
                        results.push(NodeResult {
                            node_id: node_id.clone(),
                            status: NodeStatus::Failed,
                            message: msg,
                            duration_ms: node_start.elapsed().as_millis(),
                        });
                    }
                }
            }
        }
    }

    let failed_nodes: Vec<String> = failed_set.into_iter().collect();
    let skipped_nodes: Vec<String> = skip_set.into_iter().collect();
    let all_success = failed_nodes.is_empty() && skipped_nodes.is_empty();

    DagRunResult {
        results,
        all_success,
        total_duration_ms: start.elapsed().as_millis(),
        failed_nodes,
        skipped_nodes,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> DagNode {
        DagNode {
            id: id.to_owned(),
            label: format!("Task {id}"),
            capabilities: Vec::new(),
        }
    }

    fn edge(from: &str, to: &str) -> DagEdge {
        DagEdge {
            from: from.to_owned(),
            to: to.to_owned(),
        }
    }

    // -- Validation tests --

    #[test]
    fn valid_linear_graph() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c")],
            edges: vec![edge("a", "b"), edge("b", "c")],
        };
        assert!(validate_graph(&graph, &DagRunnerConfig::default()).is_ok());
    }

    #[test]
    fn valid_diamond_graph() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c"), node("d")],
            edges: vec![
                edge("a", "b"),
                edge("a", "c"),
                edge("b", "d"),
                edge("c", "d"),
            ],
        };
        assert!(validate_graph(&graph, &DagRunnerConfig::default()).is_ok());
    }

    #[test]
    fn valid_parallel_graph() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c")],
            edges: vec![],
        };
        assert!(validate_graph(&graph, &DagRunnerConfig::default()).is_ok());
    }

    #[test]
    fn detects_simple_cycle() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b")],
            edges: vec![edge("a", "b"), edge("b", "a")],
        };
        let err = validate_graph(&graph, &DagRunnerConfig::default()).unwrap_err();
        assert!(matches!(err, DagValidationError::CycleDetected(_)));
    }

    #[test]
    fn detects_transitive_cycle() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c")],
            edges: vec![edge("a", "b"), edge("b", "c"), edge("c", "a")],
        };
        let err = validate_graph(&graph, &DagRunnerConfig::default()).unwrap_err();
        assert!(matches!(err, DagValidationError::CycleDetected(_)));
    }

    #[test]
    fn detects_unknown_node_in_edge() {
        let graph = WorkflowGraph {
            nodes: vec![node("a")],
            edges: vec![edge("a", "missing")],
        };
        let err = validate_graph(&graph, &DagRunnerConfig::default()).unwrap_err();
        assert!(matches!(err, DagValidationError::UnknownNode(ref id) if id == "missing"));
    }

    #[test]
    fn detects_duplicate_node_id() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("a")],
            edges: vec![],
        };
        let err = validate_graph(&graph, &DagRunnerConfig::default()).unwrap_err();
        assert!(matches!(err, DagValidationError::DuplicateNodeId(ref id) if id == "a"));
    }

    #[test]
    fn detects_missing_capability() {
        let mut n = node("a");
        n.capabilities = vec!["file_write".to_owned()];
        let graph = WorkflowGraph {
            nodes: vec![n],
            edges: vec![],
        };
        let config = DagRunnerConfig {
            available_capabilities: vec!["test_run".to_owned()],
            ..Default::default()
        };
        let err = validate_graph(&graph, &config).unwrap_err();
        assert!(matches!(err, DagValidationError::MissingCapability { .. }));
    }

    #[test]
    fn empty_capabilities_skips_check() {
        let mut n = node("a");
        n.capabilities = vec!["anything".to_owned()];
        let graph = WorkflowGraph {
            nodes: vec![n],
            edges: vec![],
        };
        // Default config has empty available_capabilities → skip check.
        assert!(validate_graph(&graph, &DagRunnerConfig::default()).is_ok());
    }

    // -- Layer computation tests --

    #[test]
    fn layers_linear() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c")],
            edges: vec![edge("a", "b"), edge("b", "c")],
        };
        let layers = compute_layers(&graph);
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0], vec!["a"]);
        assert_eq!(layers[1], vec!["b"]);
        assert_eq!(layers[2], vec!["c"]);
    }

    #[test]
    fn layers_diamond() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c"), node("d")],
            edges: vec![
                edge("a", "b"),
                edge("a", "c"),
                edge("b", "d"),
                edge("c", "d"),
            ],
        };
        let layers = compute_layers(&graph);
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0], vec!["a"]);
        assert_eq!(layers[1], vec!["b", "c"]); // parallel
        assert_eq!(layers[2], vec!["d"]);
    }

    #[test]
    fn layers_fully_parallel() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c")],
            edges: vec![],
        };
        let layers = compute_layers(&graph);
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0], vec!["a", "b", "c"]);
    }

    // -- Execution tests --

    #[test]
    fn execute_all_success_linear() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c")],
            edges: vec![edge("a", "b"), edge("b", "c")],
        };
        let result = execute_dag_sync(&graph, &DagRunnerConfig::default(), |id| {
            Ok(format!("{id} done"))
        });
        assert!(result.all_success);
        assert_eq!(result.results.len(), 3);
        assert!(result.failed_nodes.is_empty());
        assert!(result.skipped_nodes.is_empty());
    }

    #[test]
    fn execute_all_success_diamond() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c"), node("d")],
            edges: vec![
                edge("a", "b"),
                edge("a", "c"),
                edge("b", "d"),
                edge("c", "d"),
            ],
        };
        let result = execute_dag_sync(&graph, &DagRunnerConfig::default(), |id| {
            Ok(format!("{id} ok"))
        });
        assert!(result.all_success);
        assert_eq!(result.results.len(), 4);
    }

    #[test]
    fn execute_failure_skips_downstream() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c")],
            edges: vec![edge("a", "b"), edge("b", "c")],
        };
        let result = execute_dag_sync(&graph, &DagRunnerConfig::default(), |id| {
            if id == "b" {
                Err("b exploded".to_owned())
            } else {
                Ok(format!("{id} ok"))
            }
        });
        assert!(!result.all_success);
        assert_eq!(result.failed_nodes.len(), 1);
        assert!(result.failed_nodes.contains(&"b".to_owned()));
        assert_eq!(result.skipped_nodes.len(), 1);
        assert!(result.skipped_nodes.contains(&"c".to_owned()));
        // "a" should have succeeded.
        let a_result = result.results.iter().find(|r| r.node_id == "a").unwrap();
        assert_eq!(a_result.status, NodeStatus::Success);
    }

    #[test]
    fn execute_failure_no_skip_when_disabled() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c")],
            edges: vec![edge("a", "b"), edge("b", "c")],
        };
        let config = DagRunnerConfig {
            skip_on_failure: false,
            ..Default::default()
        };
        let result = execute_dag_sync(&graph, &config, |id| {
            if id == "b" {
                Err("b failed".to_owned())
            } else {
                Ok(format!("{id} ok"))
            }
        });
        assert!(!result.all_success);
        // "c" should NOT be skipped — it ran (and succeeded since only b fails).
        let c_result = result.results.iter().find(|r| r.node_id == "c").unwrap();
        assert_eq!(c_result.status, NodeStatus::Success);
    }

    #[test]
    fn execute_parallel_independent_all_pass() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c")],
            edges: vec![],
        };
        let result = execute_dag_sync(&graph, &DagRunnerConfig::default(), |id| {
            Ok(format!("{id} parallel"))
        });
        assert!(result.all_success);
        assert_eq!(result.results.len(), 3);
    }

    #[test]
    fn execute_diamond_partial_failure() {
        // a -> b, a -> c, b -> d, c -> d
        // c fails → d skipped, but b still runs.
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c"), node("d")],
            edges: vec![
                edge("a", "b"),
                edge("a", "c"),
                edge("b", "d"),
                edge("c", "d"),
            ],
        };
        let result = execute_dag_sync(&graph, &DagRunnerConfig::default(), |id| {
            if id == "c" {
                Err("c broke".to_owned())
            } else {
                Ok(format!("{id} ok"))
            }
        });
        assert!(!result.all_success);
        // b should succeed, c failed, d skipped (predecessor c failed).
        let b = result.results.iter().find(|r| r.node_id == "b").unwrap();
        assert_eq!(b.status, NodeStatus::Success);
        let d = result.results.iter().find(|r| r.node_id == "d").unwrap();
        assert_eq!(d.status, NodeStatus::Skipped);
    }

    #[test]
    fn execute_respects_max_parallel() {
        // 4 independent nodes with max_parallel=2 → still all succeed.
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b"), node("c"), node("d")],
            edges: vec![],
        };
        let config = DagRunnerConfig {
            max_parallel: 2,
            ..Default::default()
        };
        let result = execute_dag_sync(&graph, &config, |id| Ok(format!("{id} ok")));
        assert!(result.all_success);
        assert_eq!(result.results.len(), 4);
    }

    #[test]
    fn serde_roundtrip_graph() {
        let graph = WorkflowGraph {
            nodes: vec![node("a"), node("b")],
            edges: vec![edge("a", "b")],
        };
        let json = serde_json::to_string(&graph).unwrap();
        let deser: WorkflowGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(graph, deser);
    }

    #[test]
    fn serde_roundtrip_result() {
        let result = DagRunResult {
            results: vec![NodeResult {
                node_id: "a".to_owned(),
                status: NodeStatus::Success,
                message: "done".to_owned(),
                duration_ms: 42,
            }],
            all_success: true,
            total_duration_ms: 100,
            failed_nodes: vec![],
            skipped_nodes: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        let deser: DagRunResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.all_success, result.all_success);
        assert_eq!(deser.results.len(), 1);
    }
}
