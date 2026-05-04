//! Phone-control gRPC service — Chunk 24.4.
//!
//! Implements `phone_control.v1.PhoneControl` over tonic, exposing system
//! status, VS Code workspace discovery, Copilot session status, workflow
//! management, chat, and paired-device listing to the mobile companion.

use std::pin::Pin;

use futures_util::{stream, Stream};
use sysinfo::System;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

use crate::brain::BrainMode;
use crate::commands::streaming::{strip_anim_blocks, StreamTagParser};
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
    type StreamChatMessageStream =
        Pin<Box<dyn Stream<Item = Result<proto::ChatChunk, Status>> + Send>>;

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
                Some(mode) => (brain_mode_label(mode).to_string(), brain_model_name(mode)),
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

        match engine
            .heartbeat(&id, Some("resumed via phone".to_string()))
            .await
        {
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
        let reply = phone_chat_complete(&self.state, &msg)
            .await
            .unwrap_or_else(|e| format!("Error: {e}"));

        Ok(Response::new(proto::ChatResponse { reply }))
    }

    async fn stream_chat_message(
        &self,
        request: Request<proto::ChatRequest>,
    ) -> Result<Response<Self::StreamChatMessageStream>, Status> {
        let msg = request.into_inner().message;
        let state = self.state.clone();
        let (tx, rx) = mpsc::unbounded_channel::<Result<proto::ChatChunk, Status>>();

        tokio::spawn(async move {
            if let Err(err) = phone_chat_stream(state, msg, tx.clone()).await {
                let _ = tx.send(Err(Status::internal(err)));
            }
        });

        let output = stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        });
        Ok(Response::new(
            Box::pin(output) as Self::StreamChatMessageStream
        ))
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
        let devices =
            crate::network::pairing::list_paired_devices(store.conn()).map_err(Status::internal)?;

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

// ── Chat completion / streaming ────────────────────────────────────────────────

type ChatChunkTx = mpsc::UnboundedSender<Result<proto::ChatChunk, Status>>;

/// Perform a non-streaming chat completion using the configured brain mode,
/// with the same desktop-side persona / memory / handoff prompt assembly used
/// by the streaming Tauri chat path.
async fn phone_chat_complete(state: &AppState, message: &str) -> Result<String, String> {
    append_phone_user_message(state, message)?;
    let Some((client, messages)) = build_phone_chat_request(state, message).await? else {
        let reply = "Brain not configured".to_string();
        store_phone_assistant_message(state, &reply)?;
        return Ok(reply);
    };
    let reply = client.chat(messages).await?;
    let clean = strip_anim_blocks(&reply);
    store_phone_assistant_message(state, &clean)?;
    Ok(clean)
}

async fn phone_chat_stream(
    state: AppState,
    message: String,
    tx: ChatChunkTx,
) -> Result<(), String> {
    append_phone_user_message(&state, &message)?;
    let Some((client, messages)) = build_phone_chat_request(&state, &message).await? else {
        let reply = "Brain not configured".to_string();
        send_chat_chunk(&tx, reply.clone(), false)?;
        send_chat_chunk(&tx, String::new(), true)?;
        store_phone_assistant_message(&state, &reply)?;
        return Ok(());
    };

    let mut parser = StreamTagParser::new();
    let tx_for_chunks = tx.clone();
    let result = client
        .chat_stream(messages, |chunk_text| {
            let feed = parser.feed(chunk_text);
            if !feed.text.is_empty() {
                let _ = tx_for_chunks.send(Ok(proto::ChatChunk {
                    text: feed.text,
                    done: false,
                }));
            }
        })
        .await;

    match result {
        Ok(reply) => {
            let feed = parser.flush();
            if !feed.text.is_empty() {
                send_chat_chunk(&tx, feed.text, false)?;
            }
            send_chat_chunk(&tx, String::new(), true)?;
            store_phone_assistant_message(&state, &strip_anim_blocks(&reply))?;
            Ok(())
        }
        Err(err) => Err(err),
    }
}

async fn build_phone_chat_request(
    state: &AppState,
    user_query: &str,
) -> Result<
    Option<(
        crate::brain::openai_client::OpenAiClient,
        Vec<crate::brain::openai_client::OpenAiMessage>,
    )>,
    String,
> {
    use crate::brain::openai_client::{OpenAiClient, OpenAiMessage};

    let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?.clone();
    let active_brain = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let Some((base_url, model, api_key)) = (match brain_mode.as_ref() {
        Some(BrainMode::FreeApi {
            provider_id,
            api_key,
            model,
        }) => {
            let provider = crate::brain::get_free_provider(provider_id)
                .ok_or_else(|| format!("unknown free provider: {provider_id}"))?;
            Some((
                provider.base_url,
                model.clone().unwrap_or(provider.model),
                api_key.clone(),
            ))
        }
        Some(BrainMode::PaidApi {
            base_url,
            model,
            api_key,
            ..
        }) => Some((base_url.clone(), model.clone(), Some(api_key.clone()))),
        Some(BrainMode::LocalOllama { model }) => {
            Some(("http://127.0.0.1:11434/v1".to_string(), model.clone(), None))
        }
        Some(BrainMode::LocalLmStudio {
            base_url,
            model,
            api_key,
            ..
        }) => Some((base_url.clone(), model.clone(), api_key.clone())),
        None => active_brain
            .clone()
            .map(|model| ("http://127.0.0.1:11434/v1".to_string(), model, None)),
    }) else {
        return Ok(None);
    };

    let client = OpenAiClient::new(&base_url, &model, api_key.as_deref());
    let history = recent_phone_history(state)?;
    let mut messages = vec![OpenAiMessage {
        role: "system".to_string(),
        content: build_phone_system_prompt(
            state,
            user_query,
            brain_mode.as_ref(),
            active_brain.as_deref(),
        )
        .await,
    }];
    for (role, content) in history {
        messages.push(OpenAiMessage { role, content });
    }
    Ok(Some((client, messages)))
}

async fn build_phone_system_prompt(
    state: &AppState,
    user_query: &str,
    brain_mode: Option<&BrainMode>,
    active_brain: Option<&str>,
) -> String {
    let mut system_prompt = crate::commands::chat::SYSTEM_PROMPT_FOR_STREAMING.to_string();
    let query_emb = crate::brain::embed_for_mode(user_query, brain_mode, active_brain).await;
    let threshold = state
        .app_settings
        .lock()
        .map(|s| s.relevance_threshold)
        .unwrap_or(crate::settings::DEFAULT_RELEVANCE_THRESHOLD);

    let relevant = state
        .memory_store
        .lock()
        .ok()
        .and_then(|store| {
            store
                .hybrid_search_with_threshold(user_query, query_emb.as_deref(), 5, threshold)
                .ok()
        })
        .unwrap_or_default();
    if !relevant.is_empty() {
        let memory_block = relevant
            .iter()
            .map(|entry| format!("- [{}] {}", entry.tier.as_str(), entry.content))
            .collect::<Vec<_>>()
            .join("\n");
        system_prompt.push_str(&format!(
            "\n\n[LONG-TERM MEMORY]\nThe following facts from your memory are relevant to this conversation:\n{memory_block}\n[/LONG-TERM MEMORY]"
        ));
    }

    if let Ok(persona) = state.persona_block.lock() {
        if !persona.is_empty() {
            system_prompt.push_str(persona.as_str());
        }
    }
    if let Ok(mut handoff) = state.handoff_block.lock() {
        if !handoff.is_empty() {
            system_prompt.push_str(handoff.as_str());
            handoff.clear();
        }
    }
    system_prompt
}

fn append_phone_user_message(state: &AppState, message: &str) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("Message cannot be empty".to_string());
    }
    let mut conversation = state.conversation.lock().map_err(|e| e.to_string())?;
    conversation.push(crate::commands::chat::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "user".to_string(),
        content: message.to_string(),
        agent_name: None,
        agent_id: None,
        sentiment: None,
        timestamp: now_ms(),
    });
    Ok(())
}

fn store_phone_assistant_message(state: &AppState, content: &str) -> Result<(), String> {
    let mut conversation = state.conversation.lock().map_err(|e| e.to_string())?;
    conversation.push(crate::commands::chat::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: content.to_string(),
        agent_name: Some("TerranSoul".to_string()),
        agent_id: None,
        sentiment: None,
        timestamp: now_ms(),
    });
    Ok(())
}

fn recent_phone_history(state: &AppState) -> Result<Vec<(String, String)>, String> {
    let conversation = state.conversation.lock().map_err(|e| e.to_string())?;
    Ok(conversation
        .iter()
        .rev()
        .take(20)
        .rev()
        .map(|message| (message.role.clone(), message.content.clone()))
        .collect())
}

fn send_chat_chunk(tx: &ChatChunkTx, text: String, done: bool) -> Result<(), String> {
    tx.send(Ok(proto::ChatChunk { text, done }))
        .map_err(|_| "phone chat receiver closed".to_string())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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
            model: None,
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
