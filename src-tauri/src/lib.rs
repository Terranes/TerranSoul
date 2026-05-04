#![deny(unused_must_use)]

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Emitter;
use tauri::Manager;
use tokio::sync::Mutex as TokioMutex;

pub mod agent;
pub mod agents;
pub mod ai_integrations;
pub mod brain;
pub mod coding;
pub mod commands;
pub mod container;
pub mod identity;
pub mod link;
pub mod memory;
pub mod messaging;
pub mod network;
pub mod orchestrator;
pub mod package_manager;
pub mod persona;
pub mod plugins;
pub mod registry_server;
pub mod routing;
pub mod sandbox;
pub mod settings;
pub mod sync;
pub mod tasks;
pub mod teachable_capabilities;
pub mod voice;
pub mod vscode_workspace;
pub mod workflows;

use commands::{
    agent::list_agents,
    agents_roster::{
        roster_cancel_workflow, roster_create, roster_delete, roster_get_current,
        roster_get_ram_cap, roster_list, roster_list_pending_workflows, roster_list_workflows,
        roster_query_workflow, roster_set_working_folder, roster_start_cli_workflow, roster_switch,
    },
    auto_setup::{
        list_mcp_clients, remove_claude_mcp, remove_codex_mcp, remove_vscode_mcp, setup_claude_mcp,
        setup_claude_mcp_stdio, setup_codex_mcp, setup_codex_mcp_stdio, setup_vscode_mcp,
        setup_vscode_mcp_stdio,
    },
    brain::{
        check_lm_studio_status, check_ollama_status, classify_intent, clear_active_brain,
        download_lm_studio_model, factory_reset_brain, get_active_brain, get_brain_mode,
        get_brain_selection, get_embed_cache_status, get_lm_studio_download_status,
        get_lm_studio_models, get_next_provider, get_ollama_models, get_system_info,
        health_check_providers, list_free_providers, load_lm_studio_model, pull_ollama_model,
        recommend_brain_models, reset_embed_cache, set_active_brain, set_brain_mode,
        unload_lm_studio_model,
    },
    character::load_vrm,
    chat::{export_chat_log, get_conversation, send_message},
    coding::{
        clear_self_improve_log, coding_session_clear_handoff, coding_session_list_handoffs,
        coding_session_load_handoff, coding_session_save_handoff, code_call_graph,
        code_compute_processes, code_generate_wiki, code_index_repo, code_list_clusters,
        code_list_processes,
        code_resolve_edges,
        detect_self_improve_repo,
        get_coding_llm_config, get_coding_workflow_config, get_github_config,
        get_self_improve_metrics, get_self_improve_runs, get_self_improve_settings,
        get_self_improve_status, learn_from_user_message, list_coding_llm_recommendations,
        list_local_coding_models, list_self_improve_worktrees, open_self_improve_pr,
        preview_coding_workflow_context, pull_main_for_self_improve,
        reset_coding_workflow_config, run_coding_task, set_coding_llm_config,
        set_coding_workflow_config, set_github_config, set_self_improve_autostart,
        set_self_improve_enabled, set_self_improve_worktree_dir, start_self_improve,
        stop_self_improve, suggest_self_improve_branch, test_coding_llm_connection,
    },
    coding_sessions::{
        coding_session_append_message, coding_session_clear_chat, coding_session_fork,
        coding_session_list, coding_session_load_chat, coding_session_purge,
        coding_session_rename,
    },
    consolidation::{get_idle_status, run_sleep_consolidation, touch_activity},    crag::crag_retrieve,
    docker::{
        auto_setup_local_llm, auto_setup_local_llm_with_runtime, check_docker_status,
        check_ollama_container, detect_container_runtimes, docker_pull_model,
        ensure_ollama_container, get_runtime_preference, set_runtime_preference,
        start_docker_desktop, stop_docker_desktop, wait_for_docker,
    },
    gitnexus::{
        configure_gitnexus_sidecar, get_gitnexus_sidecar_config, gitnexus_context,
        gitnexus_detect_changes, gitnexus_impact, gitnexus_list_mirrors, gitnexus_query,
        gitnexus_sidecar_status, gitnexus_sync, gitnexus_unmirror,
    },
    github_auth::{github_poll_device_token, github_request_device_code},
    grpc::{grpc_server_start, grpc_server_status, grpc_server_stop},
    identity::{
        add_trusted_device_cmd, get_device_identity, get_pairing_qr, list_trusted_devices,
        remove_trusted_device_cmd,
    },
    ingest::{cancel_ingest_task, get_all_tasks, ingest_document, resume_ingest_task},
    lan::{
        confirm_pairing, get_copilot_session_status, list_lan_addresses, list_paired_devices,
        revoke_device, start_pairing,
    },
    link::{
        apply_memory_deltas, connect_to_peer, disconnect_link, get_link_status, get_memory_deltas,
        start_link_server, sync_memories_with_peer,
    },
    mcp::{
        get_mcp_activity, mcp_regenerate_token, mcp_server_start, mcp_server_status,
        mcp_server_stop,
    },
    memory::{
        add_memory, add_memory_edge, adjust_memory_importance, apply_memory_decay,
        audit_memory_tags, auto_promote_memories, backfill_embeddings, clear_all_data,
        close_memory_edge, count_memory_conflicts, delete_memory, delete_memory_edge,
        dismiss_memory_conflict, enforce_memory_storage_limit, evaluate_auto_learn,
        export_to_obsidian, extract_edges_via_brain, extract_memories_from_session, gc_memories,
        get_auto_learn_policy, get_edge_stats, get_edges_for_memory, get_memories,
        get_memories_by_tier, get_memory_history, get_memory_stats, get_relevant_memories,
        get_schema_info, get_short_term_memory, graph_rag_detect_communities, graph_rag_search,
        hybrid_search_memories, hybrid_search_memories_rrf, hyde_search_memories,
        list_memory_conflicts, list_memory_edges, list_relation_types, matryoshka_search_memories,
        multi_hop_search_memories, obsidian_sync, obsidian_sync_start, obsidian_sync_stop,
        promote_memory, rerank_search_memories, resolve_memory_conflict, scan_edge_conflicts,
        search_memories, semantic_search_memories, set_auto_learn_policy, summarize_session,
        temporal_query, update_memory,
    },
    messaging::{
        get_agent_messages, list_agent_subscriptions, publish_agent_message, subscribe_agent_topic,
        unsubscribe_agent_topic,
    },
    package::{
        get_ipc_protocol_range, install_agent, list_installed_agents, parse_agent_manifest,
        remove_agent, update_agent, validate_agent_manifest,
    },
    persona::{
        check_persona_drift, delete_learned_expression, delete_learned_motion, export_persona_pack,
        extract_persona_from_brain, generate_motion_from_text, get_handoff_block,
        get_motion_feedback_stats, get_persona, get_persona_block, import_persona_pack,
        list_learned_expressions, list_learned_motions, polish_learned_motion,
        preview_persona_pack, record_motion_feedback, save_learned_expression, save_learned_motion,
        save_persona, set_handoff_block, set_persona_block,
    },
    plugins::{
        plugin_activate, plugin_deactivate, plugin_get, plugin_get_setting, plugin_host_status,
        plugin_install, plugin_list, plugin_list_commands, plugin_list_slash_commands,
        plugin_list_themes, plugin_parse_manifest, plugin_set_setting, plugin_uninstall,
    },
    quest::{get_quest_tracker, save_quest_tracker},
    registry::{
        get_registry_server_port, search_agents, start_registry_server, stop_registry_server,
    },
    routing::{
        approve_remote_command, deny_remote_command, get_device_permissions, list_pending_commands,
        match_ai_integration_intent, set_device_permission,
    },
    sandbox::{
        clear_agent_capabilities, grant_agent_capability, list_agent_capabilities,
        revoke_agent_capability, run_agent_in_sandbox,
    },
    settings::{
        get_app_settings, get_model_camera_positions, save_app_settings, save_model_camera_position,
    },
    streaming::{send_message_stream, send_message_stream_self_rag},
    translation::{detect_language, list_languages, translate_text},
    user_models::{
        delete_user_model, import_user_model, list_user_models, read_user_model_bytes,
        update_user_model,
    },
    vision::{analyze_screen, capture_screen},
    voice::{
        add_hotword, clear_hotwords, clear_voice_config, diarize_audio, get_hotwords,
        get_voice_config, list_asr_providers, list_tts_providers, remove_hotword, set_asr_provider,
        set_tts_prosody, set_tts_provider, set_tts_voice, set_voice_api_key, set_voice_endpoint,
        synthesize_tts, transcribe_audio,
    },
    vscode::{vscode_forget_window, vscode_list_known_windows, vscode_open_project},
    window::{
        close_panel_window, exit_app, get_all_monitors, get_window_mode, is_dev_build,
        is_mcp_mode, open_panel_window, set_cursor_passthrough, set_pet_mode_bounds,
        set_pet_window_size, set_window_mode, start_pet_cursor_poll, start_window_drag,
        stop_pet_cursor_poll, toggle_window_mode,
    },
    workflow_plans::{
        workflow_agent_recommendations, workflow_calendar_events, workflow_plan_create_blank,
        workflow_plan_delete, workflow_plan_list, workflow_plan_load,
        workflow_plan_override_llm, workflow_plan_save, workflow_plan_update_step,
        workflow_plan_validate,
    },
    charisma::{
        charisma_delete, charisma_list, charisma_promote, charisma_rate_turn,
        charisma_record_usage, charisma_set_rating, charisma_summary,
    },
    teachable_capabilities::{
        teachable_capabilities_list, teachable_capabilities_promote,
        teachable_capabilities_record_usage, teachable_capabilities_reset,
        teachable_capabilities_set_config, teachable_capabilities_set_enabled,
        teachable_capabilities_set_rating, teachable_capabilities_summary,
    },
};
use identity::{key_store::load_or_generate_identity, trusted_devices::load_trusted_devices};

/// Inner application state. All fields are public so Tauri commands can
/// access them through [`AppState`]'s `Deref` impl — no existing code
/// needs to change.
pub struct AppStateInner {
    pub conversation: Mutex<Vec<commands::chat::Message>>,
    pub vrm_path: Mutex<Option<String>>,
    pub device_identity: Mutex<Option<identity::DeviceIdentity>>,
    pub trusted_devices: Mutex<Vec<identity::TrustedDevice>>,
    pub link_manager: TokioMutex<link::manager::LinkManager>,
    pub link_server_port: TokioMutex<Option<u16>>,
    pub command_router: TokioMutex<routing::CommandRouter>,
    pub package_installer: TokioMutex<package_manager::PackageInstaller>,
    pub package_registry: TokioMutex<Box<dyn package_manager::RegistrySource + Send + Sync>>,
    /// Name of the active Ollama brain model (e.g. "gemma3:4b"), or None for stub agent.
    pub active_brain: Mutex<Option<String>>,
    /// Three-tier brain mode configuration (free API / paid API / local Ollama).
    pub brain_mode: Mutex<Option<brain::BrainMode>>,
    /// Shared reqwest client for all Ollama HTTP calls.
    pub ollama_client: reqwest::Client,
    /// Application data directory for persisting settings and installed agents.
    pub data_dir: PathBuf,
    /// Persistent long-term memory store (SQLite).
    pub memory_store: Mutex<memory::MemoryStore>,
    /// Running registry server handle and bound port, if started.
    pub registry_server_handle: TokioMutex<Option<(u16, tokio::task::JoinHandle<()>)>>,
    /// Per-agent sandbox capability consents.
    pub capability_store: TokioMutex<sandbox::CapabilityStore>,
    /// Agent-to-agent message bus for topic-based pub/sub.
    pub message_bus: TokioMutex<messaging::MessageBus>,
    /// Current window mode (window or pet).
    pub window_mode: Mutex<commands::window::WindowMode>,
    /// Saved window inner size (width, height) captured on the last
    /// Window → Pet transition so the desktop size can be restored exactly
    /// on the next Pet → Window transition.  Prevents the "stuck in pet
    /// resolution" bug when pet mode resizes the window implicitly. */
    pub saved_window_size: Mutex<Option<(u32, u32)>>,
    /// Voice provider configuration (ASR/TTS selections).
    pub voice_config: Mutex<voice::VoiceConfig>,
    /// Provider rotation and rate-limit tracking for free API providers.
    pub provider_rotator: Mutex<brain::ProviderRotator>,
    /// Background task manager with persistence for resume.
    pub task_manager: TokioMutex<tasks::manager::TaskManager>,
    /// Persistent application settings (model selection, camera state).
    pub app_settings: Mutex<settings::AppSettings>,
    /// Whether the pet-mode cursor-tracking poll loop is active.
    pub pet_cursor_active: Arc<AtomicBool>,
    /// Durable workflow engine for long-running agent tasks (Chunk 1.5).
    pub workflow_engine: TokioMutex<workflows::WorkflowEngine>,
    /// Rendered `[PERSONA]` block pushed from the frontend persona store.
    /// Server-driven streaming paths (Ollama / OpenAI) splice this into the
    /// system prompt, alongside the existing `[LONG-TERM MEMORY]` block.
    /// Empty string means "no persona injection". See
    /// `docs/persona-design.md` § 9.1.
    pub persona_block: Mutex<String>,
    /// Rendered `[HANDOFF FROM <prev>]` block pushed from the frontend
    /// agent-roster store on agent switch. Same splicing slot as
    /// `persona_block` — server-driven streaming paths inject it into
    /// the system prompt below `[PERSONA]` / `[LONG-TERM MEMORY]`.
    /// One-shot: streaming paths read-and-clear so the new agent gets
    /// briefed once and then operates on its own thread. See Chunk 23.2
    /// in `rules/milestones.md`.
    pub handoff_block: Mutex<String>,
    /// Configuration for spawning the GitNexus sidecar (Chunk 2.1).
    /// Defaults to `npx gitnexus mcp`. Mutable so the frontend can point at
    /// a globally-installed binary (`gitnexus`) or supply a working dir.
    pub gitnexus_config: TokioMutex<agent::gitnexus_sidecar::SidecarConfig>,
    /// Lazily-spawned GitNexus sidecar handle. The first command invocation
    /// spawns the child process and caches the bridge here; subsequent calls
    /// reuse it. Dropped (and the child reaped) on `configure_gitnexus_sidecar`.
    pub gitnexus_sidecar: TokioMutex<Option<Arc<agent::gitnexus_sidecar::GitNexusSidecar>>>,
    /// Running MCP server handle (Chunk 15.1). `None` when the server is
    /// stopped. Start/stop via `mcp_server_start` / `mcp_server_stop`.
    pub mcp_server: TokioMutex<Option<ai_integrations::mcp::McpServerHandle>>,
    /// Pairing manager for mTLS device registry (Chunk 24.2b). `None` when
    /// LAN mode is disabled. Initialized on first `lan_enabled = true`.
    pub pairing_manager: Mutex<Option<network::pairing::PairingManager>>,
    /// Running gRPC Brain server handle (Chunk 24.3). `None` when stopped.
    pub grpc_server: TokioMutex<Option<commands::grpc::GrpcServerHandle>>,
    /// Plugin system host — manages plugin lifecycle, contributions, and activation.
    pub plugin_host: plugins::PluginHost,
    /// Idle-detection tracker for sleep-time consolidation (Chunk 16.7).
    pub activity_tracker: memory::consolidation::ActivityTracker,
    /// Obsidian bidirectional sync watcher (Chunk 17.7). `None` when not watching.
    pub obsidian_watcher: TokioMutex<Option<memory::obsidian_sync::ObsidianWatcher>>,
    /// Last MCP activity snapshot shown/spoken in MCP app mode.
    pub mcp_activity: Mutex<ai_integrations::mcp::activity::McpActivitySnapshot>,
    /// Configured coding LLM for self-improve mode (Chunk 25).
    pub coding_llm_config: Mutex<Option<coding::CodingLlmConfig>>,
    /// Self-improve settings (enabled flag, worktree dir, etc.).
    pub self_improve: Mutex<coding::SelfImproveSettings>,
    /// Autonomous self-improve engine handle.
    pub self_improve_engine: Arc<coding::engine::SelfImproveEngine>,
    /// Coding workflow configuration (context injection, target paths, etc.).
    pub coding_workflow_config: Mutex<coding::CodingWorkflowConfig>,
}

/// Cheaply clonable handle to the shared application state. Wraps
/// `Arc<AppStateInner>` so background servers (MCP, gRPC) can hold a
/// reference without lifetime issues. Existing Tauri commands continue
/// to access fields via auto-`Deref` — no signature changes needed.
#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

impl std::ops::Deref for AppState {
    type Target = AppStateInner;
    fn deref(&self) -> &AppStateInner {
        &self.0
    }
}

impl AppState {
    /// Create a new `AppState` bound to `data_dir`, which is used to persist
    /// settings (active brain model) and the long-term memory database.
    /// In production this is the Tauri app-data directory; for tests use
    /// [`AppState::for_test`] instead.
    fn new(data_dir: &std::path::Path) -> Self {
        let active_brain = brain::load_brain(data_dir);
        let brain_mode = brain::brain_config::load(data_dir);
        Self(Arc::new(AppStateInner {
            conversation: Mutex::new(Vec::new()),
            vrm_path: Mutex::new(None),
            device_identity: Mutex::new(None),
            trusted_devices: Mutex::new(Vec::new()),
            link_manager: TokioMutex::new(link::manager::LinkManager::new()),
            link_server_port: TokioMutex::new(None),
            command_router: TokioMutex::new(routing::CommandRouter::new("uninitialized")),
            package_installer: TokioMutex::new(package_manager::PackageInstaller::new(data_dir)),
            package_registry: TokioMutex::new(Box::new(registry_server::CatalogRegistry::new())),
            active_brain: Mutex::new(active_brain),
            brain_mode: Mutex::new(brain_mode),
            ollama_client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            data_dir: data_dir.to_path_buf(),
            memory_store: Mutex::new(memory::MemoryStore::new(data_dir)),
            registry_server_handle: TokioMutex::new(None),
            capability_store: TokioMutex::new(sandbox::CapabilityStore::new(data_dir)),
            message_bus: TokioMutex::new(messaging::MessageBus::new()),
            window_mode: Mutex::new(commands::window::WindowMode::default()),
            saved_window_size: Mutex::new(None),
            voice_config: Mutex::new(voice::config_store::load(data_dir)),
            provider_rotator: Mutex::new(brain::ProviderRotator::new()),
            task_manager: TokioMutex::new(tasks::manager::TaskManager::new(data_dir)),
            app_settings: Mutex::new(settings::config_store::load(data_dir)),
            pet_cursor_active: Arc::new(AtomicBool::new(false)),
            workflow_engine: TokioMutex::new(
                workflows::WorkflowEngine::open(&data_dir.join("workflows.sqlite")).unwrap_or_else(
                    |e| {
                        eprintln!("[workflows] failed to open durable log: {e}; using in-memory");
                        workflows::WorkflowEngine::open(std::path::Path::new(":memory:"))
                            .expect("in-memory workflow engine must open")
                    },
                ),
            ),
            persona_block: Mutex::new(String::new()),
            handoff_block: Mutex::new(String::new()),
            gitnexus_config: TokioMutex::new(agent::gitnexus_sidecar::SidecarConfig::default()),
            gitnexus_sidecar: TokioMutex::new(None),
            mcp_server: TokioMutex::new(None),
            pairing_manager: Mutex::new(None),
            grpc_server: TokioMutex::new(None),
            plugin_host: plugins::PluginHost::with_builtin_plugins(data_dir),
            activity_tracker: memory::consolidation::ActivityTracker::new(),
            obsidian_watcher: TokioMutex::new(None),
            mcp_activity: Mutex::new(
                ai_integrations::mcp::activity::McpActivitySnapshot::default(),
            ),
            coding_llm_config: Mutex::new(coding::load_coding_llm(data_dir)),
            self_improve: Mutex::new(coding::load_self_improve(data_dir)),
            self_improve_engine: Arc::new(coding::engine::SelfImproveEngine::new()),
            coding_workflow_config: Mutex::new(coding::load_coding_workflow_config(data_dir)),
        }))
    }

    /// Convenience constructor for unit tests.
    #[cfg(test)]
    pub fn for_test() -> Self {
        Self(Arc::new(AppStateInner {
            conversation: Mutex::new(Vec::new()),
            vrm_path: Mutex::new(None),
            device_identity: Mutex::new(None),
            trusted_devices: Mutex::new(Vec::new()),
            link_manager: TokioMutex::new(link::manager::LinkManager::new()),
            link_server_port: TokioMutex::new(None),
            command_router: TokioMutex::new(routing::CommandRouter::new("uninitialized")),
            package_installer: TokioMutex::new(package_manager::PackageInstaller::new(
                std::path::Path::new("."),
            )),
            package_registry: TokioMutex::new(Box::new(package_manager::MockRegistry::new())),
            active_brain: Mutex::new(None),
            brain_mode: Mutex::new(None),
            ollama_client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            data_dir: std::path::PathBuf::from("."),
            memory_store: Mutex::new(memory::MemoryStore::in_memory()),
            registry_server_handle: TokioMutex::new(None),
            capability_store: TokioMutex::new(sandbox::CapabilityStore::in_memory()),
            message_bus: TokioMutex::new(messaging::MessageBus::new()),
            window_mode: Mutex::new(commands::window::WindowMode::default()),
            saved_window_size: Mutex::new(None),
            voice_config: Mutex::new(voice::VoiceConfig::default()),
            provider_rotator: Mutex::new(brain::ProviderRotator::new()),
            task_manager: TokioMutex::new(tasks::manager::TaskManager::in_memory()),
            app_settings: Mutex::new(settings::AppSettings::default()),
            pet_cursor_active: Arc::new(AtomicBool::new(false)),
            workflow_engine: TokioMutex::new(
                workflows::WorkflowEngine::open(std::path::Path::new(":memory:"))
                    .expect("in-memory workflow engine must open"),
            ),
            persona_block: Mutex::new(String::new()),
            handoff_block: Mutex::new(String::new()),
            gitnexus_config: TokioMutex::new(agent::gitnexus_sidecar::SidecarConfig::default()),
            gitnexus_sidecar: TokioMutex::new(None),
            mcp_server: TokioMutex::new(None),
            pairing_manager: Mutex::new(None),
            grpc_server: TokioMutex::new(None),
            plugin_host: plugins::PluginHost::in_memory(),
            activity_tracker: memory::consolidation::ActivityTracker::new(),
            obsidian_watcher: TokioMutex::new(None),
            mcp_activity: Mutex::new(
                ai_integrations::mcp::activity::McpActivitySnapshot::default(),
            ),
            coding_llm_config: Mutex::new(None),
            self_improve: Mutex::new(coding::SelfImproveSettings::default()),
            self_improve_engine: Arc::new(coding::engine::SelfImproveEngine::new()),
            coding_workflow_config: Mutex::new(coding::CodingWorkflowConfig::default()),
        }))
    }
}

/// Resolve the on-disk data directory the same way the GUI does, but
/// without requiring a Tauri `AppHandle`. Used by [`run_stdio`] so the
/// stdio shim sees identical persistent state to a running GUI.
///
/// In dev builds the path includes a `dev` subdirectory (matching the
/// GUI), but unlike the GUI we **do not** wipe it on launch — the
/// stdio shim must never destroy data the GUI may rely on.
fn resolve_data_dir_for_cli() -> PathBuf {
    // App identifier from `tauri.conf.json`. Hard-coded here because
    // there is no `AppHandle` available in CLI mode.
    const BUNDLE_ID: &str = "com.terranes.terransoul";

    let base = dirs::data_dir()
        .map(|d| d.join(BUNDLE_ID))
        .unwrap_or_else(|| PathBuf::from("."));

    let dir = if cfg!(debug_assertions) {
        base.join("dev")
    } else {
        base
    };

    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Run TerranSoul as an MCP **stdio** server. Reads newline-delimited
/// JSON-RPC 2.0 from `stdin`, writes responses to `stdout`. Exits when
/// stdin reaches EOF.
///
/// Triggered by `terransoul --mcp-stdio` from `main.rs`. See Chunk 15.9.
///
/// Honors `TERRANSOUL_MCP_DATA_DIR` so VS Code (and other agents) can
/// launch a repo-local stdio brain via `.vscode/mcp.json` without
/// touching the user's companion data dir. When the override is set,
/// pet mode is enabled so `serverInfo.name` advertises
/// `terransoul-brain-mcp`.
pub fn run_stdio() -> std::io::Result<()> {
    let (data_dir, repo_local) = if let Ok(p) = std::env::var("TERRANSOUL_MCP_DATA_DIR") {
        let trimmed = p.trim();
        if trimmed.is_empty() {
            (resolve_data_dir_for_cli(), false)
        } else {
            (PathBuf::from(trimmed), true)
        }
    } else {
        (resolve_data_dir_for_cli(), false)
    };

    if repo_local {
        // Pet-mode stdio launches honor the same release > dev > mcp
        // priority as `--mcp-http`. If the app is already running, we
        // emit a clear stderr message and exit cleanly so VS Code (or
        // any stdio MCP host) surfaces the reason instead of opening a
        // duplicate brain on a stale repo-local data dir.
        if let Some(label) = detect_running_terransoul_mcp() {
            eprintln!(
                "[mcp-stdio] TerranSoul {label} build is already serving MCP — \
                 refusing to start pet-mode stdio. Use the running app's MCP \
                 entry instead."
            );
            return Ok(());
        }
        let _ = std::fs::create_dir_all(&data_dir);
        ai_integrations::mcp::enable_mcp_pet_mode();
    }

    eprintln!("[mcp-stdio] data dir: {}", data_dir.display());

    let state = AppState::new(&data_dir);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(ai_integrations::mcp::stdio::run_with_state(state))
}

/// Default port used by the headless `--mcp-http` runtime.
///
/// Chosen so it does not collide with the in-app servers (release =
/// `7421`, dev `cargo tauri dev` = `7422`). External agents (Copilot,
/// Codex, Claude Code, Clawcode, etc.) launch this via `npm run mcp`
/// without conflicting with a running app.
pub const HEADLESS_MCP_PORT: u16 = 7423;

/// Resolve the data directory for the headless `--mcp-http` runtime.
///
/// The headless server is meant for repo-local agent sessions, so it
/// keeps state in `<cwd>/mcp-data/` by default — distinct from the
/// per-OS app-data dir that the GUI/stdio modes use, so a
/// `npm run mcp` session never touches the user's persistent companion
/// state.
///
/// Override with the `TERRANSOUL_MCP_DATA_DIR` env var when needed.
fn resolve_headless_mcp_data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("TERRANSOUL_MCP_DATA_DIR") {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.join("mcp-data")
}

/// Read `TERRANSOUL_MCP_PORT`, falling back to [`HEADLESS_MCP_PORT`].
fn resolve_headless_mcp_port() -> u16 {
    std::env::var("TERRANSOUL_MCP_PORT")
        .ok()
        .and_then(|s| s.trim().parse::<u16>().ok())
        .unwrap_or(HEADLESS_MCP_PORT)
}

/// Apply seed data to a fresh `mcp-data/` directory.
///
/// On **first run** (no existing `memory.db`), this function:
/// 1. Copies `brain_config.json` and `app_settings.json` from compile-time
///    embedded seed files into `data_dir`.
/// 2. Creates `memory.db` with the canonical schema and executes the
///    seed SQL to pre-populate TerranSoul knowledge.
///
/// If `memory.db` already exists, this function is a no-op.
fn seed_mcp_data(data_dir: &std::path::Path) {
    let db_path = data_dir.join("memory.db");
    if db_path.exists() {
        return; // Not first run — never overwrite existing data
    }

    eprintln!("[mcp-http] first run detected — applying seed data");

    // Write config files (only if missing)
    let brain_cfg_path = data_dir.join("brain_config.json");
    if !brain_cfg_path.exists() {
        let seed_brain_cfg = include_str!("../../mcp-data-seed/brain_config.json");
        if let Err(e) = std::fs::write(&brain_cfg_path, seed_brain_cfg) {
            eprintln!("[mcp-http] warning: failed to write seed brain_config.json: {e}");
        }
    }

    let app_cfg_path = data_dir.join("app_settings.json");
    if !app_cfg_path.exists() {
        let seed_app_cfg = include_str!("../../mcp-data-seed/app_settings.json");
        if let Err(e) = std::fs::write(&app_cfg_path, seed_app_cfg) {
            eprintln!("[mcp-http] warning: failed to write seed app_settings.json: {e}");
        }
    }

    // Create memory.db with schema + seed data
    match rusqlite::Connection::open(&db_path) {
        Ok(conn) => {
            if let Err(e) = memory::schema::create_canonical_schema(&conn) {
                eprintln!("[mcp-http] warning: failed to initialize schema: {e}");
                return;
            }
            let seed_sql = include_str!("../../mcp-data-seed/memory-seed.sql");
            if let Err(e) = conn.execute_batch(seed_sql) {
                eprintln!("[mcp-http] warning: failed to apply memory-seed.sql: {e}");
            } else {
                eprintln!("[mcp-http] seed data applied successfully");
            }
        }
        Err(e) => {
            eprintln!("[mcp-http] warning: failed to open memory.db for seeding: {e}");
        }
    }
}

/// Probe the canonical TerranSoul MCP HTTP ports (release 7421, dev
/// 7422) to see if the user already has a brain server running.
///
/// Priority order is **release > dev > mcp**: if either of the
/// app-owned ports answers, the headless runner refuses to start so a
/// `npm run mcp` invocation never shadows a running app. Returns the
/// label of the first port that answers, or `None` when neither is up.
///
/// **Service-name verification** — relying on an open port alone is
/// unreliable (any process can squat on 7421/7422). We therefore
/// follow up the TCP probe with an unauthenticated MCP `initialize`
/// JSON-RPC call. The response is **always** delivered (the MCP
/// dispatch layer answers `initialize` before checking the bearer
/// token, by spec — see `router::dispatch_method`), so we can read
/// `serverInfo.name` and confirm we are talking to TerranSoul before
/// refusing to start. If the probe answers but the handshake doesn't
/// look like TerranSoul, we treat the port as a foreign tenant and
/// continue startup on `7423` instead of refusing.
fn detect_running_terransoul_mcp() -> Option<&'static str> {
    let release = ai_integrations::mcp::DEFAULT_PORT;
    let dev = ai_integrations::mcp::DEFAULT_DEV_PORT;
    if probe_terransoul_on(release) {
        Some("release")
    } else if probe_terransoul_on(dev) {
        Some("dev")
    } else {
        None
    }
}

/// Confirm a TerranSoul MCP server is bound to `127.0.0.1:<port>` by
/// (1) opening a TCP connection and (2) issuing the unauthenticated
/// MCP `initialize` handshake, then checking that
/// `serverInfo.name` starts with `terransoul-brain`.
fn probe_terransoul_on(port: u16) -> bool {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(250)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(750)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

    // Minimal JSON-RPC initialize. Auth is not required for
    // initialize per the MCP spec, and our router answers it before
    // running the bearer check (see router::dispatch_method).
    let body = br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let req = format!(
        "POST /mcp HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\
         \r\n",
        len = body.len()
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    if stream.write_all(body).is_err() {
        return false;
    }

    let mut buf = Vec::with_capacity(2048);
    let mut chunk = [0u8; 1024];
    let deadline = std::time::Instant::now() + Duration::from_millis(750);
    loop {
        if buf.len() > 16 * 1024 {
            break; // hard cap; server name lives near the top
        }
        if std::time::Instant::now() >= deadline {
            break;
        }
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&buf);
    // We don't need to parse the full HTTP response; the JSON
    // payload is in the body and contains the marker we care about.
    text.contains("\"name\"")
        && (text.contains("\"terransoul-brain\"")
            || text.contains("\"terransoul-brain-dev\"")
            || text.contains("\"terransoul-brain-mcp\""))
}

/// Run the `--mcp-setup` CLI subcommand.
///
/// Detects AI coding editor config directories (`.vscode/`, `~/.cursor/`,
/// `~/.codex/`, `~/.claude/`, `~/.config/opencode/`) and writes the
/// MCP server entry pointing at the headless MCP HTTP server.
///
/// Generates a token if one doesn't exist yet, then writes configs.
pub fn run_mcp_setup() -> std::io::Result<()> {
    let data_dir = resolve_headless_mcp_data_dir();
    let port = resolve_headless_mcp_port();

    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("[mcp-setup] failed to create data dir: {e}");
        return Err(e);
    }

    // Load or create the bearer token
    let token = match ai_integrations::mcp::auth::load_or_create(&data_dir) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[mcp-setup] failed to load/create token: {e}");
            return Err(std::io::Error::other(e));
        }
    };

    let url = format!("http://127.0.0.1:{port}/mcp");
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    eprintln!("[mcp-setup] MCP URL: {url}");
    eprintln!("[mcp-setup] token: {token}");
    eprintln!("[mcp-setup] scanning for editor configs...\n");

    let results = ai_integrations::mcp::auto_setup::setup_all_clients(&cwd, &url, &token);

    if results.is_empty() {
        eprintln!("[mcp-setup] no supported editor config directories found.");
        eprintln!("[mcp-setup] checked: .vscode/, ~/.cursor/, ~/.codex/, ~/.claude/, ~/.config/opencode/");
        eprintln!("\n[mcp-setup] hint: run this from your project root, or create the config directories first.");
    } else {
        for result in &results {
            let status = if result.success { "✓" } else { "✗" };
            eprintln!("  {status} {}", result.message);
            eprintln!("    → {}", result.config_path);
        }
        let success_count = results.iter().filter(|r| r.success).count();
        eprintln!(
            "\n[mcp-setup] done — {success_count}/{} configs written.",
            results.len()
        );
    }

    eprintln!("\n[mcp-setup] next step: run `npm run mcp` to start the server.");
    Ok(())
}

/// Run TerranSoul as a headless MCP **HTTP** server.
///
/// This mode does **not** launch Tauri or the WebView; it only spins up
/// the brain/memory/RAG/gitnexus surface needed to serve MCP tool calls
/// to external AI coding agents over JSON-RPC on
/// `http://127.0.0.1:<port>/mcp`.
///
/// On startup it prints the bound URL, the bearer token (also persisted
/// to `<data_dir>/mcp-token.txt`), and blocks until Ctrl+C.
///
/// Triggered by `terransoul --mcp-http` from `main.rs`, which is the
/// binary `npm run mcp` invokes.
pub fn run_http_server() -> std::io::Result<()> {
    // Priority: release > dev > mcp. If the user already has the app
    // running with its MCP HTTP server bound, refuse to start so we
    // never shadow live companion state with a stale headless brain.
    if let Some(label) = detect_running_terransoul_mcp() {
        let port = if label == "release" {
            ai_integrations::mcp::DEFAULT_PORT
        } else {
            ai_integrations::mcp::DEFAULT_DEV_PORT
        };
        eprintln!(
            "[mcp-http] TerranSoul {label} build is already serving MCP on \
             127.0.0.1:{port} — refusing to start headless pet mode."
        );
        eprintln!(
            "[mcp-http] Use the running app's MCP server instead, or stop \
             the app and re-run `npm run mcp`."
        );
        return Ok(());
    }

    let data_dir = resolve_headless_mcp_data_dir();
    let port = resolve_headless_mcp_port();

    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!(
            "[mcp-http] failed to create data dir {}: {e}",
            data_dir.display()
        );
        return Err(e);
    }

    // Apply seed data on first run (no existing memory.db)
    seed_mcp_data(&data_dir);

    // Mark this process as MCP pet mode so the JSON-RPC initialize
    // handshake and `/status` endpoint advertise `buildMode: "mcp"`.
    ai_integrations::mcp::enable_mcp_pet_mode();

    eprintln!("[mcp-http] data dir: {}", data_dir.display());

    let state = AppState::new(&data_dir);
    let token = match ai_integrations::mcp::auth::load_or_create(&data_dir) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[mcp-http] failed to load/create token: {e}");
            return Err(std::io::Error::other(e));
        }
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async move {
        // Auto-configure brain if not yet set up (Ollama → free API fallback)
        brain::mcp_auto_config::auto_configure_mcp_brain(&data_dir).await;
        brain::mcp_auto_config::apply_config_to_state(&state, &data_dir);

        match ai_integrations::mcp::start_server(state, port, token.clone(), false).await {
            Ok(handle) => {
                eprintln!(
                    "[mcp-http] listening on http://127.0.0.1:{} (POST /mcp)",
                    handle.port
                );
                eprintln!("[mcp-http] bearer token: {token}");
                eprintln!(
                    "[mcp-http] health check: GET http://127.0.0.1:{}/health (no auth)",
                    handle.port
                );

                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                match write_mcp_token_file(&cwd, &token) {
                    Ok(token_file) => {
                        eprintln!("[mcp-http] token written to {}", token_file.display())
                    }
                    Err(e) => {
                        eprintln!("[mcp-http] warning: failed to write .vscode/.mcp-token: {e}")
                    }
                }

                eprintln!("[mcp-http] press Ctrl+C to stop");
                if let Err(e) = tokio::signal::ctrl_c().await {
                    eprintln!("[mcp-http] ctrl_c listener error: {e}");
                }
                eprintln!("[mcp-http] shutting down");
                handle.stop();
                let _ =
                    tokio::time::timeout(std::time::Duration::from_secs(2), handle.task).await;
                Ok(())
            }
            Err(e) => {
                eprintln!("[mcp-http] failed to start: {e}");
                Err(std::io::Error::other(e))
            }
        }
    })
}

fn write_mcp_token_file(
    workspace_root: &std::path::Path,
    token: &str,
) -> std::io::Result<PathBuf> {
    let vscode_dir = workspace_root.join(".vscode");
    std::fs::create_dir_all(&vscode_dir)?;
    let token_file = vscode_dir.join(".mcp-token");
    std::fs::write(&token_file, token)?;
    Ok(token_file)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init());
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_barcode_scanner::init());

    builder
        .invoke_handler(tauri::generate_handler![
            send_message,
            get_conversation,
            export_chat_log,
            list_agents,
            load_vrm,
            get_device_identity,
            get_pairing_qr,
            list_trusted_devices,
            add_trusted_device_cmd,
            remove_trusted_device_cmd,
            get_link_status,
            start_link_server,
            connect_to_peer,
            disconnect_link,
            // CRDT memory sync (Chunk 17.5)
            get_memory_deltas,
            apply_memory_deltas,
            sync_memories_with_peer,
            list_pending_commands,
            approve_remote_command,
            deny_remote_command,
            set_device_permission,
            get_device_permissions,
            match_ai_integration_intent,
            parse_agent_manifest,
            validate_agent_manifest,
            get_ipc_protocol_range,
            install_agent,
            update_agent,
            remove_agent,
            list_installed_agents,
            get_system_info,
            recommend_brain_models,
            check_ollama_status,
            get_ollama_models,
            pull_ollama_model,
            check_lm_studio_status,
            get_lm_studio_models,
            download_lm_studio_model,
            get_lm_studio_download_status,
            load_lm_studio_model,
            unload_lm_studio_model,
            set_active_brain,
            get_active_brain,
            clear_active_brain,
            factory_reset_brain,
            add_memory,
            get_memories,
            search_memories,
            update_memory,
            delete_memory,
            clear_all_data,
            get_relevant_memories,
            get_short_term_memory,
            extract_memories_from_session,
            summarize_session,
            semantic_search_memories,
            hybrid_search_memories,
            hybrid_search_memories_rrf,
            hyde_search_memories,
            rerank_search_memories,
            matryoshka_search_memories,
            backfill_embeddings,
            get_schema_info,
            get_memory_stats,
            enforce_memory_storage_limit,
            apply_memory_decay,
            auto_promote_memories,
            adjust_memory_importance,
            list_memory_conflicts,
            resolve_memory_conflict,
            dismiss_memory_conflict,
            count_memory_conflicts,
            scan_edge_conflicts,
            audit_memory_tags,
            gc_memories,
            promote_memory,
            get_memories_by_tier,
            // Entity-Relationship Graph (V5 schema)
            add_memory_edge,
            close_memory_edge,
            delete_memory_edge,
            list_memory_edges,
            get_edges_for_memory,
            get_edge_stats,
            list_relation_types,
            extract_edges_via_brain,
            multi_hop_search_memories,
            // Auto-learn (daily conversation → brain write-back, see docs §21)
            get_auto_learn_policy,
            set_auto_learn_policy,
            evaluate_auto_learn,
            // Obsidian vault export (Chunk 18.5)
            export_to_obsidian,
            // Bidirectional Obsidian sync (Chunk 17.7)
            obsidian_sync,
            obsidian_sync_start,
            obsidian_sync_stop,
            // GraphRAG community detection + dual-level search (Chunk 16.6)
            graph_rag_detect_communities,
            graph_rag_search,
            // Temporal reasoning queries (Chunk 17.3)
            temporal_query,
            // Memory versioning (Chunk 16.12)
            get_memory_history,
            start_registry_server,
            stop_registry_server,
            get_registry_server_port,
            search_agents,
            grant_agent_capability,
            revoke_agent_capability,
            list_agent_capabilities,
            run_agent_in_sandbox,
            clear_agent_capabilities,
            publish_agent_message,
            subscribe_agent_topic,
            unsubscribe_agent_topic,
            get_agent_messages,
            list_agent_subscriptions,
            set_window_mode,
            get_window_mode,
            toggle_window_mode,
            set_cursor_passthrough,
            get_all_monitors,
            set_pet_mode_bounds,
            start_window_drag,
            set_pet_window_size,
            start_pet_cursor_poll,
            stop_pet_cursor_poll,
            exit_app,
            is_dev_build,
            is_mcp_mode,
            open_panel_window,
            close_panel_window,
            send_message_stream,
            send_message_stream_self_rag,
            list_free_providers,
            classify_intent,
            get_brain_mode,
            set_brain_mode,
            get_embed_cache_status,
            reset_embed_cache,
            // Agent roster + durable workflows (Chunk 1.5)
            roster_list,
            roster_create,
            roster_delete,
            roster_switch,
            roster_get_current,
            roster_set_working_folder,
            roster_get_ram_cap,
            roster_start_cli_workflow,
            roster_query_workflow,
            roster_cancel_workflow,
            roster_list_workflows,
            roster_list_pending_workflows,
            health_check_providers,
            get_next_provider,
            get_brain_selection,
            list_asr_providers,
            list_tts_providers,
            get_voice_config,
            set_asr_provider,
            set_tts_provider,
            set_tts_voice,
            set_tts_prosody,
            set_voice_api_key,
            set_voice_endpoint,
            clear_voice_config,
            synthesize_tts,
            transcribe_audio,
            diarize_audio,
            get_hotwords,
            add_hotword,
            remove_hotword,
            clear_hotwords,
            get_app_settings,
            save_app_settings,
            get_model_camera_positions,
            save_model_camera_position,
            get_quest_tracker,
            save_quest_tracker,
            get_persona,
            save_persona,
            set_persona_block,
            get_persona_block,
            set_handoff_block,
            get_handoff_block,
            list_learned_expressions,
            save_learned_expression,
            delete_learned_expression,
            list_learned_motions,
            save_learned_motion,
            delete_learned_motion,
            extract_persona_from_brain,
            check_persona_drift,
            generate_motion_from_text,
            polish_learned_motion,
            record_motion_feedback,
            get_motion_feedback_stats,
            export_persona_pack,
            import_persona_pack,
            preview_persona_pack,
            capture_screen,
            analyze_screen,
            list_languages,
            translate_text,
            detect_language,
            check_docker_status,
            detect_container_runtimes,
            get_runtime_preference,
            set_runtime_preference,
            start_docker_desktop,
            stop_docker_desktop,
            wait_for_docker,
            check_ollama_container,
            ensure_ollama_container,
            docker_pull_model,
            auto_setup_local_llm,
            auto_setup_local_llm_with_runtime,
            ingest_document,
            cancel_ingest_task,
            resume_ingest_task,
            get_all_tasks,
            list_lan_addresses,
            get_copilot_session_status,
            start_pairing,
            confirm_pairing,
            revoke_device,
            list_paired_devices,
            import_user_model,
            list_user_models,
            delete_user_model,
            read_user_model_bytes,
            update_user_model,
            // GitNexus sidecar — Chunk 2.1 (Phase 13 Tier 1)
            configure_gitnexus_sidecar,
            get_gitnexus_sidecar_config,
            gitnexus_sidecar_status,
            gitnexus_query,
            gitnexus_context,
            gitnexus_impact,
            gitnexus_detect_changes,
            // GitNexus KG mirror — Chunk 2.3 (Phase 13 Tier 3)
            gitnexus_sync,
            gitnexus_unmirror,
            gitnexus_list_mirrors,
            // MCP server — Chunk 15.1 (Phase 15)
            mcp_server_start,
            mcp_server_stop,
            mcp_server_status,
            mcp_regenerate_token,
            get_mcp_activity,
            // gRPC Brain server — Chunk 24.3 (Phase 24)
            grpc_server_start,
            grpc_server_stop,
            grpc_server_status,
            // Auto-setup writers — Chunk 15.6 (Phase 15)
            setup_vscode_mcp,
            setup_claude_mcp,
            setup_codex_mcp,
            // Stdio transport variants — Chunk 15.9 (Phase 15)
            setup_vscode_mcp_stdio,
            setup_claude_mcp_stdio,
            setup_codex_mcp_stdio,
            remove_vscode_mcp,
            remove_claude_mcp,
            remove_codex_mcp,
            list_mcp_clients,
            // Consolidation (Chunk 16.7)
            run_sleep_consolidation,
            touch_activity,
            get_idle_status,
            // Self-improve engine commands (Chunk 25)
            list_coding_llm_recommendations,
            get_coding_llm_config,
            set_coding_llm_config,
            get_self_improve_settings,
            set_self_improve_enabled,
            set_self_improve_worktree_dir,
            detect_self_improve_repo,
            suggest_self_improve_branch,
            get_self_improve_status,
            start_self_improve,
            stop_self_improve,
            set_self_improve_autostart,
            get_self_improve_metrics,
            get_self_improve_runs,
            clear_self_improve_log,
            test_coding_llm_connection,
            list_local_coding_models,
            get_coding_workflow_config,
            set_coding_workflow_config,
            reset_coding_workflow_config,
            preview_coding_workflow_context,
            list_self_improve_worktrees,
            code_index_repo,
            code_resolve_edges,
            code_call_graph,
            code_compute_processes,
            code_list_clusters,
            code_list_processes,
            code_generate_wiki,
            get_github_config,
            set_github_config,
            open_self_improve_pr,
            pull_main_for_self_improve,
            learn_from_user_message,
            run_coding_task,
            coding_session_save_handoff,
            coding_session_load_handoff,
            coding_session_list_handoffs,
            coding_session_clear_handoff,
            // Self-improve coding sessions (Chunk 30.2)
            coding_session_list,
            coding_session_append_message,
            coding_session_load_chat,
            coding_session_clear_chat,
            coding_session_rename,
            coding_session_fork,
            coding_session_purge,
            // Multi-agent workflow plans + calendar (Chunk 30.3)
            workflow_plan_list,
            workflow_plan_load,
            workflow_plan_save,
            workflow_plan_delete,
            workflow_plan_create_blank,
            workflow_plan_validate,
            workflow_plan_update_step,
            workflow_plan_override_llm,
            workflow_calendar_events,
            workflow_agent_recommendations,
            // Charisma teaching system (Chunk 30.4)
            charisma_list,
            charisma_rate_turn,
            charisma_record_usage,
            charisma_set_rating,
            charisma_delete,
            charisma_promote,
            charisma_summary,
            // Teachable configurable capabilities (Chunk 30.5)
            teachable_capabilities_list,
            teachable_capabilities_set_enabled,
            teachable_capabilities_set_config,
            teachable_capabilities_record_usage,
            teachable_capabilities_set_rating,
            teachable_capabilities_reset,
            teachable_capabilities_promote,
            teachable_capabilities_summary,
            // GitHub browser authorization for self-improve mode
            github_request_device_code,
            github_poll_device_token,
            // CRAG retrieval (Chunk 16.5b)
            crag_retrieve,
            // VS Code workspace surfacing — Chunk 15.10 (Phase 15)
            vscode_open_project,
            vscode_list_known_windows,
            vscode_forget_window,
            // Plugin system
            plugin_install,
            plugin_activate,
            plugin_deactivate,
            plugin_uninstall,
            plugin_list,
            plugin_get,
            plugin_list_commands,
            plugin_list_slash_commands,
            plugin_list_themes,
            plugin_get_setting,
            plugin_set_setting,
            plugin_host_status,
            plugin_parse_manifest,
        ])
        .setup(|app| {
            let stronghold_dir = app
                .path()
                .app_local_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            std::fs::create_dir_all(&stronghold_dir)?;
            let stronghold_salt_path = stronghold_dir.join("stronghold-salt.txt");
            app.handle().plugin(
                tauri_plugin_stronghold::Builder::with_argon2(&stronghold_salt_path).build(),
            )?;

            let base_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));

            // MCP mode (`npm run mcp` / `--mcp-app`) takes priority over
            // dev/release: persist in `<repo>/mcp-data/` so the runtime
            // never touches the user's companion data dir, and flip the
            // pet-mode flag so the MCP server / frontend report "mcp"
            // instead of "dev" / "release".
            let mcp_app_mode = std::env::var("TERRANSOUL_MCP_APP_MODE")
                .map(|v| v == "1")
                .unwrap_or(false);
            let data_dir = if mcp_app_mode {
                ai_integrations::mcp::enable_mcp_pet_mode();
                let mcp_dir = std::env::var("TERRANSOUL_MCP_DATA_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| {
                        std::env::current_dir()
                            .unwrap_or_else(|_| PathBuf::from("."))
                            .join("mcp-data")
                    });
                std::fs::create_dir_all(&mcp_dir)
                    .expect("failed to create MCP data directory");
                mcp_dir
            } else if cfg!(debug_assertions) {
                // In dev builds, use a separate data directory so dev never
                // touches release data.  Wipe it on every launch to guarantee
                // a fresh-install experience.
                let dev_dir = base_data_dir.join("dev");
                if dev_dir.exists() {
                    let _ = std::fs::remove_dir_all(&dev_dir);
                }
                std::fs::create_dir_all(&dev_dir).expect("failed to create dev data directory");
                dev_dir
            } else {
                base_data_dir
            };

            app.manage(AppState::new(&data_dir));
            let state = app.state::<AppState>();

            // Auto-start the MCP HTTP server on the headless port (7423)
            // when running in MCP mode so external coding agents can talk
            // to the live app without the user clicking through the
            // Control Panel.
            if mcp_app_mode {
                let app_state_inner = state.inner().clone();
                let mcp_data_dir = data_dir.clone();
                let app_handle = app.handle().clone();
                ai_integrations::mcp::activity::McpActivityReporter::new(
                    app_state_inner.clone(),
                    Some(app_handle.clone()),
                )
                .startup("Auto-configuring the MCP brain.".to_string());
                tauri::async_runtime::spawn(async move {
                    // Auto-configure brain if not yet set up
                    brain::mcp_auto_config::auto_configure_mcp_brain(&mcp_data_dir).await;
                    brain::mcp_auto_config::apply_config_to_state(&app_state_inner, &mcp_data_dir);

                    let token = match ai_integrations::mcp::auth::load_or_create(&mcp_data_dir) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("[mcp-app] failed to load/create token: {e}");
                            return;
                        }
                    };
                    match ai_integrations::mcp::start_server_with_activity(
                        app_state_inner.clone(),
                        HEADLESS_MCP_PORT,
                        token.clone(),
                        false,
                        Some(app_handle),
                    )
                    .await
                    {
                        Ok(handle) => {
                            eprintln!(
                                "[mcp-app] MCP server listening on http://127.0.0.1:{} (POST /mcp)",
                                handle.port
                            );
                            eprintln!("[mcp-app] bearer token: {token}");
                            // Park the handle on AppState so the UI's
                            // existing controls (status/stop) work.
                            *app_state_inner.mcp_server.lock().await = Some(handle);
                        }
                        Err(e) => eprintln!("[mcp-app] failed to start MCP server: {e}"),
                    }
                });
            }

            let identity = load_or_generate_identity(&data_dir)
                .unwrap_or_else(|_| identity::DeviceIdentity::generate());
            let device_id = identity.device_id.clone();
            *state.device_identity.lock().unwrap() = Some(identity);

            let devices = load_trusted_devices(&data_dir);
            *state.trusted_devices.lock().unwrap() = devices;

            *state.command_router.blocking_lock() = routing::CommandRouter::new(&device_id);

            // System tray with Show/Hide + Window/Pet toggle + Quit
            let show_hide = MenuItem::with_id(app, "show_hide", "Show / Hide", true, None::<&str>)?;
            let mode_toggle =
                MenuItem::with_id(app, "mode_toggle", "Switch to Pet Mode", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_hide, &mode_toggle, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap())
                .menu(&menu)
                .tooltip("TerranSoul")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show_hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                    "mode_toggle" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let state = app.state::<AppState>();
                            let previous = { *state.window_mode.lock().unwrap() };
                            let new_mode = match previous {
                                commands::window::WindowMode::Window => {
                                    commands::window::WindowMode::Pet
                                }
                                commands::window::WindowMode::Pet => {
                                    commands::window::WindowMode::Window
                                }
                            };
                            // Mirror the save/restore behaviour from the
                            // commands so a tray-driven toggle also restores
                            // the desktop window size correctly.
                            if previous == commands::window::WindowMode::Window
                                && new_mode == commands::window::WindowMode::Pet
                            {
                                if let Ok(size) = window.inner_size() {
                                    *state.saved_window_size.lock().unwrap() =
                                        Some((size.width, size.height));
                                }
                            }
                            let _ = commands::window::apply_window_mode(&window, new_mode);
                            if previous == commands::window::WindowMode::Pet
                                && new_mode == commands::window::WindowMode::Window
                            {
                                let (w, h) = state
                                    .saved_window_size
                                    .lock()
                                    .ok()
                                    .and_then(|s| *s)
                                    .unwrap_or((420, 700));
                                let _ = window.set_size(tauri::PhysicalSize::new(w, h));
                            }
                            *state.window_mode.lock().unwrap() = new_mode;
                            // Emit event so frontend can react
                            let _ = window.emit("window-mode-changed", new_mode);
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Set the window icon so the taskbar / title bar shows the app icon
            // instead of the default WebView icon during development.
            if let Some(window) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon().cloned() {
                    let _ = window.set_icon(icon);
                }
                #[cfg(debug_assertions)]
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
