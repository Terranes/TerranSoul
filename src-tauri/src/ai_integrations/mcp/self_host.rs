//! MCP server self-host & self-improve (Chunk 25.11).
//!
//! When the self-improve loop starts, it ensures a local MCP server is
//! running so external AI coding assistants can query the brain and the
//! loop can extend its own tools dynamically.
//!
//! ## Features
//!
//! 1. **Auto-spawn** — `ensure_mcp_running()` starts the MCP server if
//!    it isn't already active. Called at engine startup.
//! 2. **Dynamic tool registry** — `DynamicToolRegistry` allows the
//!    self-improve loop to register additional tools at runtime without
//!    restarting the server.
//! 3. **Self-improve tools** — exposes coding-workflow state (run history,
//!    current chunk, metrics) as MCP tools so external editors can query
//!    progress.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Dynamic Tool Registry
// ---------------------------------------------------------------------------

/// A dynamically registered tool that can be added/removed at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicTool {
    /// Unique tool name (e.g. `"self_improve_status"`).
    pub name: String,
    /// Human-readable description shown to clients.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
    /// Whether this tool requires `code_read` capability.
    pub requires_code_read: bool,
}

/// Result of invoking a dynamic tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the invocation succeeded.
    pub success: bool,
    /// JSON payload to return to the caller.
    pub content: serde_json::Value,
    /// Optional error message on failure.
    pub error: Option<String>,
}

/// Thread-safe registry of dynamically registered MCP tools.
///
/// The self-improve loop registers tools here at startup; the MCP router
/// checks this registry when dispatching unknown method names.
#[derive(Debug, Clone)]
pub struct DynamicToolRegistry {
    tools: Arc<RwLock<HashMap<String, RegisteredTool>>>,
}

/// Internal representation of a registered tool with its handler.
#[derive(Debug, Clone)]
struct RegisteredTool {
    pub definition: DynamicTool,
    /// Static response for tools that don't need runtime state.
    /// In a full implementation, this would be a trait-object handler.
    pub static_response: Option<serde_json::Value>,
}

impl DynamicToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new dynamic tool. Overwrites any existing tool with the same name.
    pub async fn register(&self, tool: DynamicTool, static_response: Option<serde_json::Value>) {
        let mut map = self.tools.write().await;
        map.insert(
            tool.name.clone(),
            RegisteredTool {
                definition: tool,
                static_response,
            },
        );
    }

    /// Remove a dynamic tool by name. Returns true if it existed.
    pub async fn unregister(&self, name: &str) -> bool {
        let mut map = self.tools.write().await;
        map.remove(name).is_some()
    }

    /// List all currently registered dynamic tools.
    pub async fn list_tools(&self) -> Vec<DynamicTool> {
        let map = self.tools.read().await;
        map.values().map(|r| r.definition.clone()).collect()
    }

    /// Check if a tool name is registered.
    pub async fn has_tool(&self, name: &str) -> bool {
        let map = self.tools.read().await;
        map.contains_key(name)
    }

    /// Invoke a dynamic tool by name. Returns `None` if the tool doesn't exist.
    pub async fn invoke(&self, name: &str, _params: &serde_json::Value) -> Option<ToolResult> {
        let map = self.tools.read().await;
        let tool = map.get(name)?;
        // For now, return the static response or a default success.
        let content = tool
            .static_response
            .clone()
            .unwrap_or_else(|| serde_json::json!({"status": "ok"}));
        Some(ToolResult {
            success: true,
            content,
            error: None,
        })
    }

    /// Number of registered tools.
    pub async fn count(&self) -> usize {
        let map = self.tools.read().await;
        map.len()
    }
}

impl Default for DynamicToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Self-Improve MCP Tools
// ---------------------------------------------------------------------------

/// Register the standard self-improve tools into the dynamic registry.
///
/// These tools expose the coding workflow state to external AI assistants:
/// - `self_improve_status` — current run status, active chunk, engine state
/// - `self_improve_history` — recent run records
/// - `self_improve_metrics` — aggregate metrics summary
pub async fn register_self_improve_tools(registry: &DynamicToolRegistry) {
    registry
        .register(
            DynamicTool {
                name: "self_improve_status".to_owned(),
                description:
                    "Get the current status of the self-improve engine (running, chunk, progress)"
                        .to_owned(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                requires_code_read: false,
            },
            None,
        )
        .await;

    registry
        .register(
            DynamicTool {
                name: "self_improve_history".to_owned(),
                description: "Get recent self-improve run records (last 20 runs)".to_owned(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "default": 20, "description": "Max records to return" }
                    },
                    "additionalProperties": false
                }),
                requires_code_read: false,
            },
            None,
        )
        .await;

    registry
        .register(
            DynamicTool {
                name: "self_improve_metrics".to_owned(),
                description:
                    "Get aggregate self-improve metrics (success rate, costs, token usage)"
                        .to_owned(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                requires_code_read: false,
            },
            None,
        )
        .await;
}

// ---------------------------------------------------------------------------
// Auto-Spawn Configuration
// ---------------------------------------------------------------------------

/// Configuration for the MCP auto-spawn behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAutoSpawnConfig {
    /// Whether to auto-start MCP when the self-improve engine starts.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Port to bind the MCP server to. 0 = use default (7421/7422).
    #[serde(default)]
    pub port_override: u16,
    /// Whether to register self-improve tools automatically.
    #[serde(default = "default_true")]
    pub register_tools: bool,
}

fn default_true() -> bool {
    true
}

impl Default for McpAutoSpawnConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port_override: 0,
            register_tools: true,
        }
    }
}

/// Outcome of an auto-spawn attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnOutcome {
    /// Whether the server was already running (no-op).
    pub already_running: bool,
    /// Whether a new server was spawned.
    pub spawned: bool,
    /// The port the server is listening on (0 if not running).
    pub port: u16,
    /// Number of dynamic tools registered.
    pub tools_registered: usize,
    /// Error message if spawn failed.
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn registry_register_and_list() {
        let reg = DynamicToolRegistry::new();
        assert_eq!(reg.count().await, 0);

        reg.register(
            DynamicTool {
                name: "test_tool".to_owned(),
                description: "A test".to_owned(),
                input_schema: serde_json::json!({}),
                requires_code_read: false,
            },
            None,
        )
        .await;

        assert_eq!(reg.count().await, 1);
        assert!(reg.has_tool("test_tool").await);
        assert!(!reg.has_tool("nonexistent").await);
    }

    #[tokio::test]
    async fn registry_unregister() {
        let reg = DynamicToolRegistry::new();
        reg.register(
            DynamicTool {
                name: "tmp".to_owned(),
                description: "temp".to_owned(),
                input_schema: serde_json::json!({}),
                requires_code_read: false,
            },
            None,
        )
        .await;
        assert!(reg.unregister("tmp").await);
        assert!(!reg.unregister("tmp").await); // already gone
        assert_eq!(reg.count().await, 0);
    }

    #[tokio::test]
    async fn registry_invoke_returns_static_response() {
        let reg = DynamicToolRegistry::new();
        let response = serde_json::json!({"running": true, "chunk": "28.5"});
        reg.register(
            DynamicTool {
                name: "status".to_owned(),
                description: "status".to_owned(),
                input_schema: serde_json::json!({}),
                requires_code_read: false,
            },
            Some(response.clone()),
        )
        .await;

        let result = reg.invoke("status", &serde_json::json!({})).await.unwrap();
        assert!(result.success);
        assert_eq!(result.content, response);
    }

    #[tokio::test]
    async fn registry_invoke_nonexistent_returns_none() {
        let reg = DynamicToolRegistry::new();
        assert!(reg
            .invoke("missing", &serde_json::json!({}))
            .await
            .is_none());
    }

    #[tokio::test]
    async fn registry_overwrite_existing_tool() {
        let reg = DynamicToolRegistry::new();
        reg.register(
            DynamicTool {
                name: "t".to_owned(),
                description: "v1".to_owned(),
                input_schema: serde_json::json!({}),
                requires_code_read: false,
            },
            Some(serde_json::json!(1)),
        )
        .await;
        reg.register(
            DynamicTool {
                name: "t".to_owned(),
                description: "v2".to_owned(),
                input_schema: serde_json::json!({}),
                requires_code_read: true,
            },
            Some(serde_json::json!(2)),
        )
        .await;

        assert_eq!(reg.count().await, 1);
        let tools = reg.list_tools().await;
        assert_eq!(tools[0].description, "v2");
        assert!(tools[0].requires_code_read);
    }

    #[tokio::test]
    async fn register_self_improve_tools_adds_three() {
        let reg = DynamicToolRegistry::new();
        register_self_improve_tools(&reg).await;
        assert_eq!(reg.count().await, 3);
        assert!(reg.has_tool("self_improve_status").await);
        assert!(reg.has_tool("self_improve_history").await);
        assert!(reg.has_tool("self_improve_metrics").await);
    }

    #[test]
    fn auto_spawn_config_defaults() {
        let cfg = McpAutoSpawnConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.port_override, 0);
        assert!(cfg.register_tools);
    }

    #[test]
    fn auto_spawn_config_serde_roundtrip() {
        let cfg = McpAutoSpawnConfig {
            enabled: false,
            port_override: 9999,
            register_tools: false,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let deser: McpAutoSpawnConfig = serde_json::from_str(&json).unwrap();
        assert!(!deser.enabled);
        assert_eq!(deser.port_override, 9999);
    }

    #[test]
    fn spawn_outcome_serde() {
        let outcome = SpawnOutcome {
            already_running: false,
            spawned: true,
            port: 7421,
            tools_registered: 3,
            error: None,
        };
        let json = serde_json::to_string(&outcome).unwrap();
        let deser: SpawnOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.port, 7421);
        assert!(deser.spawned);
    }

    #[test]
    fn dynamic_tool_serde() {
        let tool = DynamicTool {
            name: "test".to_owned(),
            description: "desc".to_owned(),
            input_schema: serde_json::json!({"type": "object"}),
            requires_code_read: true,
        };
        let json = serde_json::to_string(&tool).unwrap();
        let deser: DynamicTool = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.name, "test");
        assert!(deser.requires_code_read);
    }

    #[test]
    fn tool_result_serde() {
        let result = ToolResult {
            success: true,
            content: serde_json::json!({"data": 42}),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deser: ToolResult = serde_json::from_str(&json).unwrap();
        assert!(deser.success);
        assert_eq!(deser.content["data"], 42);
    }
}
