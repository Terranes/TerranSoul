//! Phone-control gRPC service — Chunk 24.4.
//!
//! Implements `phone_control.v1.PhoneControl` over tonic, exposing system
//! status, VS Code workspace discovery, Copilot session status, workflow
//! management, chat, and paired-device listing to the mobile companion.

use sysinfo::System;
use tonic::{Request, Response, Status};

use crate::brain::BrainMode;
use crate::network::vscode_probe;
use crate::AppState;

pub mod proto {
    tonic::include_proto!("terransoul.phone_control.v1");
}

use proto::phone_control_server::{PhoneControl, PhoneControlServer};

/// Shared state for the phone-control service.
#[derive(Clone)]
pub struct PhoneControlService {
    state: AppState,
}

impl PhoneControlService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub fn into_server(self) -> PhoneControlServer<Self> {
        PhoneControlServer::new(self)
    }
}

/// Derive a display label from a `BrainMode`.
fn brain_mode_label(mode: &BrainMode) -> &str {
    match mode {
        BrainMode::FreeApi { .. } => "free_api",
        BrainMode::PaidApi { .. } => "paid_api",
        BrainMode::LocalOllama { .. } => "local_ollama",
        BrainMode::LocalLmStudio { .. } => "local_lm_studio",
    }
}

/// Derive a model display name from a `BrainMode`.
fn brain_model_name(mode: &BrainMode) -> String {
    match mode {
        BrainMode::FreeApi { provider_id, .. } => provider_id.clone(),
        BrainMode::PaidApi { model, .. } => model.clone(),
        BrainMode::LocalOllama { model } => model.clone(),
        BrainMode::LocalLmStudio { model, .. } => model.clone(),
    }
}

#[tonic::async_trait]
impl PhoneControl for PhoneControlService {
    async fn get_system_status(
        &self,
        _request: Request<proto::SystemStatusRequest>,
    ) -> Result<Response<proto::SystemStatusResponse>, Status> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let (provider, model) = {
            let mode_guard = self
                .state
                .brain_mode
                .lock()
                .map_err(|e| Status::internal(e.to_string()))?;
            match mode_guard.as_ref() {
                Some(mode) => (
                    brain_mode_label(mode).to_string(),
                    brain_model_name(mode),
                ),
                None => ("none".to_string(), String::new()),
            }
        };

        let memory_count = {
            let store = self
                .state
                .memory_store
                .lock()
                .map_err(|e| Status::internal(e.to_string()))?;
            store.count() as u32
        };

        Ok(Response::new(proto::SystemStatusResponse {
            total_memory_bytes: sys.total_memory(),
            used_memory_bytes: sys.used_memory(),
            cpu_usage_pct: sys.global_cpu_usage(),
            brain_provider: provider,
            brain_model: model,
            memory_entry_count: memory_count,
        }))
    }

    async fn list_vs_code_workspaces(
        &self,
        _request: Request<proto::ListWorkspacesRequest>,
    ) -> Result<Response<proto::ListWorkspacesResponse>, Status> {
        let workspaces = discover_vscode_workspaces().unwrap_or_default();
        Ok(Response::new(proto::ListWorkspacesResponse { workspaces }))
    }

    async fn get_copilot_session_status(
        &self,
        _request: Request<proto::CopilotSessionRequest>,
    ) -> Result<Response<proto::CopilotSessionResponse>, Status> {
        match vscode_probe::probe_copilot_session().await {
            Some(summary) => Ok(Response::new(proto::CopilotSessionResponse {
                found: true,
                workspace_folder: summary.workspace_folder.unwrap_or_default(),
                session_id: summary.session_id.unwrap_or_default(),
                model: summary.model.unwrap_or_default(),
                last_user_turn_ts: summary.last_user_turn_ts.unwrap_or_default(),
                last_user_preview: summary.last_user_preview.unwrap_or_default(),
                last_assistant_turn_ts: summary.last_assistant_turn_ts.unwrap_or_default(),
                last_assistant_preview: summary.last_assistant_preview.unwrap_or_default(),
                tool_invocation_count: summary.tool_invocation_count,
                event_count: summary.event_count,
            })),
            None => Ok(Response::new(proto::CopilotSessionResponse {
                found: false,
                ..Default::default()
            })),
        }
    }

    async fn list_workflow_runs(
        &self,
        request: Request<proto::ListWorkflowsRequest>,
    ) -> Result<Response<proto::ListWorkflowsResponse>, Status> {
        let include_finished = request.into_inner().include_finished;
        let engine = self.state.workflow_engine.lock().await;
        let summaries = if include_finished {
            engine.list_all().await
        } else {
            engine.list_pending().await
        }
        .map_err(Status::internal)?;

        let runs = summaries
            .into_iter()
            .map(|s| proto::WorkflowRun {
                workflow_id: s.workflow_id,
                name: s.name,
                status: format!("{:?}", s.status),
                started_at_unix_ms: s.started_at,
                last_event_at_unix_ms: s.last_event_at,
                event_count: s.event_count,
            })
            .collect();

        Ok(Response::new(proto::ListWorkflowsResponse { runs }))
    }

    async fn get_workflow_progress(
        &self,
        request: Request<proto::WorkflowProgressRequest>,
    ) -> Result<Response<proto::WorkflowProgressResponse>, Status> {
        let id = request.into_inner().workflow_id;
        let engine = self.state.workflow_engine.lock().await;

        let summaries = engine.list_all().await.map_err(Status::internal)?;
        let summary = summaries
            .into_iter()
            .find(|s| s.workflow_id == id)
            .ok_or_else(|| Status::not_found(format!("workflow {id} not found")))?;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        Ok(Response::new(proto::WorkflowProgressResponse {
            workflow_id: summary.workflow_id,
            name: summary.name,
            status: format!("{:?}", summary.status),
            started_at_unix_ms: summary.started_at,
            last_event_at_unix_ms: summary.last_event_at,
            event_count: summary.event_count,
            summary: format!(
                "{} events, last activity {}ms ago",
                summary.event_count,
                now_ms - summary.last_event_at
            ),
        }))
    }

    async fn continue_workflow(
        &self,
        request: Request<proto::ContinueWorkflowRequest>,
    ) -> Result<Response<proto::ContinueWorkflowResponse>, Status> {
        let id = request.into_inner().workflow_id;
        let engine = self.state.workflow_engine.lock().await;

        match engine.heartbeat(&id, Some("resumed via phone".to_string())).await {
            Ok(()) => Ok(Response::new(proto::ContinueWorkflowResponse {
                accepted: true,
                message: format!("workflow {id} heartbeat sent"),
            })),
            Err(e) => Ok(Response::new(proto::ContinueWorkflowResponse {
                accepted: false,
                message: e,
            })),
        }
    }

    async fn send_chat_message(
        &self,
        request: Request<proto::ChatRequest>,
    ) -> Result<Response<proto::ChatResponse>, Status> {
        let msg = request.into_inner().message;

        // Non-streaming one-shot completion via the OpenAI-compatible client
        // (works for free, paid, and LM Studio modes — all expose /v1/chat).
        let mode = {
            self.state
                .brain_mode
                .lock()
                .map_err(|e| Status::internal(e.to_string()))?
                .clone()
        };

        let reply = match mode {
            Some(ref brain_mode) => {
                phone_chat_complete(brain_mode, &msg)
                    .await
                    .unwrap_or_else(|e| format!("Error: {e}"))
            }
            None => "Brain not configured".to_string(),
        };

        Ok(Response::new(proto::ChatResponse { reply }))
    }

    async fn list_paired_devices(
        &self,
        _request: Request<proto::ListDevicesRequest>,
    ) -> Result<Response<proto::ListDevicesResponse>, Status> {
        let store = self
            .state
            .memory_store
            .lock()
            .map_err(|e| Status::internal(e.to_string()))?;
        let devices = crate::network::pairing::list_paired_devices(store.conn())
            .map_err(Status::internal)?;

        let infos = devices
            .into_iter()
            .map(|d| proto::PairedDeviceInfo {
                device_id: d.device_id,
                display_name: d.display_name,
                paired_at_unix_ms: d.paired_at as i64,
                last_seen_at_unix_ms: d.last_seen_at.unwrap_or(0) as i64,
                capabilities: d.capabilities,
            })
            .collect();

        Ok(Response::new(proto::ListDevicesResponse { devices: infos }))
    }
}

// ── One-shot chat completion ───────────────────────────────────────────────────

/// Perform a non-streaming chat completion using the configured brain mode.
async fn phone_chat_complete(
    mode: &BrainMode,
    message: &str,
) -> Result<String, String> {
    use crate::brain::openai_client::{OpenAiClient, OpenAiMessage};

    let (base_url, model, api_key) = match mode {
        BrainMode::FreeApi { provider_id, api_key } => {
            let provider = crate::brain::get_free_provider(provider_id)
                .ok_or_else(|| format!("unknown free provider: {provider_id}"))?;
            (provider.base_url, provider.model, api_key.clone())
        }
        BrainMode::PaidApi { base_url, model, api_key, .. } => {
            (base_url.clone(), model.clone(), Some(api_key.clone()))
        }
        BrainMode::LocalOllama { model } => {
            ("http://127.0.0.1:11434/v1".to_string(), model.clone(), None)
        }
        BrainMode::LocalLmStudio { base_url, model, api_key, .. } => {
            (base_url.clone(), model.clone(), api_key.clone())
        }
    };

    let client = OpenAiClient::new(&base_url, &model, api_key.as_deref());
    let messages = vec![
        OpenAiMessage {
            role: "user".to_string(),
            content: message.to_string(),
        },
    ];
    client.chat(messages).await
}

// ── VS Code workspace discovery ────────────────────────────────────────────────

fn discover_vscode_workspaces() -> Option<Vec<proto::VsCodeWorkspace>> {
    let user_data = vscode_probe::vscode_user_data_dir()?;
    let storage_path = user_data.parent()?.join("storage.json");

    let content = std::fs::read_to_string(&storage_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    let entries = json
        .pointer("/openedPathsList/workspaces3")
        .or_else(|| json.pointer("/openedPathsList/entries"))
        .and_then(|v| v.as_array())?;

    let workspaces = entries
        .iter()
        .filter_map(|entry| {
            let path = entry
                .as_str()
                .or_else(|| entry.get("folderUri").and_then(|u| u.as_str()))?;
            let clean_path = path.strip_prefix("file:///").unwrap_or(path);
            let name = std::path::Path::new(clean_path)
                .file_name()?
                .to_str()?
                .to_string();
            Some(proto::VsCodeWorkspace {
                path: clean_path.to_string(),
                name,
                last_opened_unix_ms: 0,
            })
        })
        .collect();

    Some(workspaces)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_control_service_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<PhoneControlService>();
    }

    #[test]
    fn brain_mode_label_variants() {
        let free = BrainMode::FreeApi {
            provider_id: "groq".into(),
            api_key: None,
        };
        assert_eq!(brain_mode_label(&free), "free_api");

        let local = BrainMode::LocalOllama {
            model: "gemma3:4b".into(),
        };
        assert_eq!(brain_mode_label(&local), "local_ollama");
    }

    #[test]
    fn brain_model_name_variants() {
        let paid = BrainMode::PaidApi {
            provider: "openai".into(),
            api_key: "sk-test".into(),
            model: "gpt-4o".into(),
            base_url: "https://api.openai.com/v1".into(),
        };
        assert_eq!(brain_model_name(&paid), "gpt-4o");
    }

    #[test]
    fn discover_vscode_workspaces_returns_option() {
        let _ = discover_vscode_workspaces();
    }
}
