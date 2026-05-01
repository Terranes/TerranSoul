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
    consolidation::{get_idle_status, run_sleep_consolidation, touch_activity},
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
    identity::{
        add_trusted_device_cmd, get_device_identity, get_pairing_qr, list_trusted_devices,
        remove_trusted_device_cmd,
    },
    ingest::{cancel_ingest_task, get_all_tasks, ingest_document, resume_ingest_task},
    link::{connect_to_peer, disconnect_link, get_link_status, start_link_server},
    mcp::{mcp_regenerate_token, mcp_server_start, mcp_server_status, mcp_server_stop},
    memory::{
        add_memory, add_memory_edge, adjust_memory_importance, apply_memory_decay,
        audit_memory_tags, auto_promote_memories, backfill_embeddings, clear_all_data,
        close_memory_edge, count_memory_conflicts, delete_memory, delete_memory_edge,
        dismiss_memory_conflict, evaluate_auto_learn, export_to_obsidian, extract_edges_via_brain,
        extract_memories_from_session, gc_memories, get_auto_learn_policy, get_edge_stats,
        get_edges_for_memory, get_memories, get_memories_by_tier, get_memory_history,
        get_memory_stats, get_relevant_memories, get_schema_info, get_short_term_memory,
        hybrid_search_memories, hybrid_search_memories_rrf, hyde_search_memories,
        list_memory_conflicts, list_memory_edges, list_relation_types, matryoshka_search_memories,
        multi_hop_search_memories, promote_memory, rerank_search_memories, resolve_memory_conflict,
        scan_edge_conflicts, search_memories, semantic_search_memories, set_auto_learn_policy,
        summarize_session, temporal_query, update_memory,
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
        list_learned_expressions, list_learned_motions, preview_persona_pack,
        record_motion_feedback, save_learned_expression, save_learned_motion, save_persona,
        set_handoff_block, set_persona_block,
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
        open_panel_window, set_cursor_passthrough, set_pet_mode_bounds, set_pet_window_size,
        set_window_mode, start_pet_cursor_poll, start_window_drag, stop_pet_cursor_poll,
        toggle_window_mode,
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
    /// Plugin system host — manages plugin lifecycle, contributions, and activation.
    pub plugin_host: plugins::PluginHost,
    /// Idle-detection tracker for sleep-time consolidation (Chunk 16.7).
    pub activity_tracker: memory::consolidation::ActivityTracker,
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
            plugin_host: plugins::PluginHost::with_builtin_plugins(data_dir),
            activity_tracker: memory::consolidation::ActivityTracker::new(),
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
            plugin_host: plugins::PluginHost::in_memory(),
            activity_tracker: memory::consolidation::ActivityTracker::new(),
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
pub fn run_stdio() -> std::io::Result<()> {
    let data_dir = resolve_data_dir_for_cli();
    eprintln!("[mcp-stdio] data dir: {}", data_dir.display());

    let state = AppState::new(&data_dir);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(ai_integrations::mcp::stdio::run_with_state(state))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
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
            let base_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));

            // In dev builds, use a separate data directory so dev never
            // touches release data.  Wipe it on every launch to guarantee
            // a fresh-install experience.
            let data_dir = if cfg!(debug_assertions) {
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
