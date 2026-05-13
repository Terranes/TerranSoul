//! Live MCP activity reporting for the Tauri MCP app mode.
//!
//! The HTTP MCP server runs in the backend, while the visible pet/app UI
//! runs in the WebView. This module bridges the two with a small snapshot
//! stored on `AppState` and, when a Tauri `AppHandle` is available, emitted
//! as the `mcp-activity` frontend event.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::brain::BrainMode;
use crate::AppState;

pub const MCP_ACTIVITY_EVENT: &str = "mcp-activity";
pub const MCP_SELF_IMPROVE_LOG_FILE: &str = "self_improve_mcp.jsonl";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpActivityStatus {
    #[default]
    Idle,
    Working,
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpActivitySnapshot {
    pub status: McpActivityStatus,
    pub phase: String,
    pub message: String,
    pub tool_name: Option<String>,
    pub tool_title: Option<String>,
    pub brain_provider: String,
    pub brain_model: Option<String>,
    pub updated_at_ms: u64,
    pub speak: bool,
}

impl Default for McpActivitySnapshot {
    fn default() -> Self {
        Self {
            status: McpActivityStatus::Idle,
            phase: "idle".to_string(),
            message: "MCP brain is idle.".to_string(),
            tool_name: None,
            tool_title: None,
            brain_provider: "none".to_string(),
            brain_model: None,
            updated_at_ms: now_ms(),
            speak: false,
        }
    }
}

#[derive(Clone)]
pub struct McpActivityReporter {
    state: AppState,
    app: Option<AppHandle>,
}

impl McpActivityReporter {
    pub fn new(state: AppState, app: Option<AppHandle>) -> Self {
        Self { state, app }
    }

    pub fn snapshot(&self) -> McpActivitySnapshot {
        self.state
            .mcp_activity
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn startup(&self, message: impl Into<String>) -> McpActivitySnapshot {
        self.record(
            McpActivityStatus::Working,
            "startup",
            message.into(),
            None,
            None,
            true,
        )
    }

    pub fn ready(&self, port: u16) -> McpActivitySnapshot {
        let (provider, model) = self.brain_identity();
        let model_label = describe_model(&provider, model.as_deref());
        self.record_with_identity(
            McpActivityStatus::Success,
            "ready",
            format!("MCP brain is ready on port {port} using {model_label}."),
            None,
            None,
            provider,
            model,
            true,
        )
    }

    pub fn failed(&self, message: impl Into<String>) -> McpActivitySnapshot {
        self.record(
            McpActivityStatus::Error,
            "error",
            message.into(),
            None,
            None,
            true,
        )
    }

    pub fn tool_started(&self, tool_name: &str, args: &Value) -> McpActivitySnapshot {
        let (title, subject) = describe_tool(tool_name, args);
        let (provider, model) = self.brain_identity();
        let model_label = describe_model(&provider, model.as_deref());
        self.record_with_identity(
            McpActivityStatus::Working,
            "tool_start",
            format!("Using {model_label}, I am {subject}."),
            Some(tool_name.to_string()),
            Some(title),
            provider,
            model,
            true,
        )
    }

    pub fn tool_finished(&self, tool_name: &str) -> McpActivitySnapshot {
        let (title, _) = describe_tool(tool_name, &Value::Null);
        self.record(
            McpActivityStatus::Success,
            "tool_done",
            format!("Finished {title}."),
            Some(tool_name.to_string()),
            Some(title),
            false,
        )
    }

    pub fn tool_failed(&self, tool_name: &str, error: &str) -> McpActivitySnapshot {
        let (title, _) = describe_tool(tool_name, &Value::Null);
        self.record(
            McpActivityStatus::Error,
            "tool_error",
            format!("{title} failed: {}", trim_text(error, 140)),
            Some(tool_name.to_string()),
            Some(title),
            true,
        )
    }

    fn record(
        &self,
        status: McpActivityStatus,
        phase: impl Into<String>,
        message: String,
        tool_name: Option<String>,
        tool_title: Option<String>,
        speak: bool,
    ) -> McpActivitySnapshot {
        let (provider, model) = self.brain_identity();
        self.record_with_identity(
            status, phase, message, tool_name, tool_title, provider, model, speak,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn record_with_identity(
        &self,
        status: McpActivityStatus,
        phase: impl Into<String>,
        message: String,
        tool_name: Option<String>,
        tool_title: Option<String>,
        brain_provider: String,
        brain_model: Option<String>,
        speak: bool,
    ) -> McpActivitySnapshot {
        let snapshot = McpActivitySnapshot {
            status,
            phase: phase.into(),
            message,
            tool_name,
            tool_title,
            brain_provider,
            brain_model,
            updated_at_ms: now_ms(),
            speak,
        };
        if let Ok(mut guard) = self.state.mcp_activity.lock() {
            *guard = snapshot.clone();
        }
        if crate::ai_integrations::mcp::is_mcp_pet_mode() {
            if let Err(error) = write_mcp_self_improve_log(&self.state.data_dir, &snapshot) {
                eprintln!("[mcp-log] failed to write self-improve activity log: {error}");
            }
        }
        if let Some(app) = &self.app {
            let _ = app.emit(MCP_ACTIVITY_EVENT, snapshot.clone());
        }
        snapshot
    }

    fn brain_identity(&self) -> (String, Option<String>) {
        let brain_mode = self
            .state
            .brain_mode
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        let active = self
            .state
            .active_brain
            .lock()
            .ok()
            .and_then(|guard| guard.clone());

        match brain_mode {
            Some(BrainMode::FreeApi {
                provider_id, model, ..
            }) => (format!("free/{provider_id}"), model),
            Some(BrainMode::PaidApi {
                provider, model, ..
            }) => (provider, Some(model)),
            Some(BrainMode::LocalOllama { model }) => ("ollama".to_string(), Some(model)),
            Some(BrainMode::LocalLmStudio { model, .. }) => ("lmstudio".to_string(), Some(model)),
            None => match active {
                Some(model) => ("ollama".to_string(), Some(model)),
                None => ("none".to_string(), None),
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct McpSelfImproveLogRecord<'a> {
    record_type: &'static str,
    snapshot: &'a McpActivitySnapshot,
}

fn write_mcp_self_improve_log(
    data_dir: &Path,
    snapshot: &McpActivitySnapshot,
) -> std::io::Result<()> {
    let path = data_dir.join(MCP_SELF_IMPROVE_LOG_FILE);
    let record = McpSelfImproveLogRecord {
        record_type: "mcp_activity",
        snapshot,
    };
    let json = serde_json::to_string(&record)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    crate::coding::rolling_log::append_line(&path, &json)
}

pub fn describe_model(provider: &str, model: Option<&str>) -> String {
    match (provider, model) {
        ("none", _) => "the configured fallback brain".to_string(),
        (provider, Some(model)) if !model.trim().is_empty() => format!("{provider} {model}"),
        (provider, _) => format!("{provider} brain"),
    }
}

fn describe_tool(tool_name: &str, args: &Value) -> (String, String) {
    match tool_name {
        "brain_search" => {
            let query = arg_text(args, "query").unwrap_or_else(|| "the requested topic".into());
            (
                "Brain search".into(),
                format!("searching memory for {query}"),
            )
        }
        "brain_get_entry" => {
            let id = args["id"]
                .as_i64()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "the requested entry".into());
            (
                "Memory entry lookup".into(),
                format!("loading memory entry {id}"),
            )
        }
        "brain_list_recent" => (
            "Recent memory list".into(),
            "listing recent memories".into(),
        ),
        "brain_kg_neighbors" => (
            "Knowledge graph lookup".into(),
            "checking nearby knowledge graph memories".into(),
        ),
        "brain_summarize" => (
            "Brain summary".into(),
            "summarizing the requested material".into(),
        ),
        "brain_suggest_context" => {
            let query =
                arg_text(args, "query").unwrap_or_else(|| "the active editor context".into());
            (
                "Context suggestion".into(),
                format!("building context for {query}"),
            )
        }
        "brain_ingest_url" => {
            let url = arg_text(args, "url").unwrap_or_else(|| "the requested source".into());
            ("URL ingest".into(), format!("ingesting {url}"))
        }
        "brain_append" => {
            let id = args["id"]
                .as_i64()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "an existing entry".into());
            (
                "Memory append".into(),
                format!("appending update to memory {id}"),
            )
        }
        "brain_health" => ("Brain health check".into(), "checking brain health".into()),
        other => (other.replace('_', " "), format!("running {other}")),
    }
}

fn arg_text(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(|s| trim_text(s, 90))
        .filter(|s| !s.trim().is_empty())
}

fn trim_text(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut out = normalized
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn search_activity_names_query() {
        let (_, subject) = describe_tool("brain_search", &json!({ "query": "rust async" }));
        assert!(subject.contains("rust async"));
    }

    #[test]
    fn long_arguments_are_trimmed() {
        let long = "a ".repeat(100);
        let trimmed = trim_text(&long, 20);
        assert!(trimmed.ends_with("..."));
        assert!(trimmed.len() <= 20);
    }

    #[test]
    fn default_snapshot_is_not_spoken() {
        let snapshot = McpActivitySnapshot::default();
        assert_eq!(snapshot.status, McpActivityStatus::Idle);
        assert!(!snapshot.speak);
    }
}
