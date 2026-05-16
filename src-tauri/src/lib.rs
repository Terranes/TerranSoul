#![deny(unused_must_use)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::webview::Color;
use tauri::Emitter;
use tauri::Manager;
use tauri::WebviewWindow;
use tokio::sync::Mutex as TokioMutex;

pub mod agent;
pub mod agents;
pub mod ai_integrations;
pub mod brain;
pub mod coding;
pub mod commands;
pub mod container;
pub mod hive;
pub mod identity;
pub mod integrations;
pub mod link;
pub mod memory;
pub mod messaging;
pub mod network;
pub mod orchestrator;
pub mod package_manager;
pub mod persona;
pub mod plugins;
pub mod registry_server;
pub mod resilience;
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
        dispatch_hermes_job, hermes_office_status, roster_cancel_workflow, roster_create,
        roster_delete, roster_get_current, roster_get_ram_cap, roster_list,
        roster_list_pending_workflows, roster_list_workflows, roster_query_workflow,
        roster_set_working_folder, roster_start_cli_workflow, roster_switch,
    },
    auto_setup::{
        list_mcp_clients, remove_claude_mcp, remove_codex_mcp, remove_hermes_mcp,
        remove_vscode_mcp, setup_claude_mcp, setup_claude_mcp_stdio, setup_codex_mcp,
        setup_codex_mcp_stdio, setup_hermes_mcp, setup_hermes_mcp_stdio, setup_vscode_mcp,
        setup_vscode_mcp_stdio,
    },
    brain::{
        brain_eviction_log, check_lm_studio_status, check_ollama_status, classify_intent,
        clear_active_brain, download_lm_studio_model, embedding_queue_diagnostics,
        embedding_queue_status, factory_reset_brain,
        get_active_brain, get_agent_routing, get_brain_mode, get_brain_selection,
        get_embed_cache_status, get_embedding_registry_state, get_failover_policy,
        get_failover_summary, get_lm_studio_download_status, get_lm_studio_models,
        get_next_provider, get_ollama_models, get_provider_policy, get_system_info,
        health_check_providers, list_embedding_models, list_free_providers, load_lm_studio_model,
        plan_embedding_model_switch, pull_ollama_model, recommend_brain_models, remove_agent_route,
        remove_provider_task_override, reset_embed_cache, resolve_provider_for_role,
        resolve_provider_for_task, select_provider_with_constraints, set_active_brain,
        set_agent_route, set_brain_mode, set_failover_policy, set_provider_policy,
        set_provider_task_override, switch_embedding_model, unload_lm_studio_model,
        warmup_local_ollama,
    },
    character::load_vrm,
    charisma::{
        charisma_delete, charisma_list, charisma_promote, charisma_rate_turn,
        charisma_record_usage, charisma_set_rating, charisma_summary,
    },
    chat::{export_chat_log, get_conversation, send_message},
    coding::{
        clear_self_improve_log, code_add_repo_to_group, code_architecture_tours, code_call_graph,
        code_compute_processes, code_create_group, code_cross_repo_query, code_delete_group,
        code_detect_harnesses, code_diff_overlay, code_explain_graph, code_export_graph,
        code_extract_contracts, code_extract_negatives, code_generate_skills, code_generate_wiki,
        code_group_status, code_import_sessions, code_index_repo, code_list_clusters,
        code_list_group_contracts, code_list_groups, code_list_processes,
        code_remove_repo_from_group, code_replay_all_sessions, code_replay_session,
        code_resolve_edges, coding_session_clear_handoff, coding_session_list_handoffs,
        coding_session_load_handoff, coding_session_save_handoff, detect_self_improve_repo,
        get_coding_llm_config, get_coding_workflow_config, get_github_config,
        get_self_improve_gate_history, get_self_improve_gate_metrics, get_self_improve_metrics,
        get_self_improve_runs, get_self_improve_settings, get_self_improve_status,
        get_self_improve_workboard, learn_from_user_message, list_coding_llm_recommendations,
        list_local_coding_models, list_self_improve_worktrees, open_self_improve_pr,
        preview_coding_workflow_context, promote_to_milestone_chunk, pull_main_for_self_improve,
        reset_coding_workflow_config, run_coding_task, set_coding_llm_config,
        set_coding_workflow_config, set_github_config, set_self_improve_autostart,
        set_self_improve_enabled, set_self_improve_worktree_dir, start_self_improve,
        stop_self_improve, suggest_self_improve_branch, test_coding_llm_connection,
    },
    coding_sessions::{
        coding_session_append_message, coding_session_clear_chat, coding_session_fork,
        coding_session_list, coding_session_load_chat, coding_session_purge, coding_session_rename,
        coding_session_resume,
    },
    consolidation::{get_idle_status, run_sleep_consolidation, touch_activity},
    context_folder::{
        add_context_folder, convert_context_to_knowledge, export_kg_subtree,
        export_knowledge_to_folder, import_file_to_knowledge_graph, list_context_folder_memories,
        list_context_folders, remove_context_folder, scan_context_folder, sync_context_folders,
        toggle_context_folder,
    },
    crag::crag_retrieve,
    docker::{
        auto_setup_local_llm, auto_setup_local_llm_with_runtime, check_docker_status,
        check_ollama_container, detect_container_runtimes, docker_pull_model,
        ensure_ollama_container, get_runtime_preference, install_docker_desktop, install_podman,
        set_runtime_preference, start_docker_desktop, stop_docker_desktop, wait_for_docker,
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
    lan_share::{
        lan_share_connect, lan_share_disconnect, lan_share_discover, lan_share_remote_health,
        lan_share_search, lan_share_search_all, lan_share_start, lan_share_status, lan_share_stop,
        lan_share_stop_discovery,
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
        audit_memory_tags, auto_promote_memories, backfill_embedding_model_id, backfill_embeddings,
        clear_all_data, close_memory_edge, compact_ann, count_memory_conflicts, daily_brief_query,
        delete_memory, delete_memory_edge, detach_memory_node, disk_ann_migration_status,
        disk_ann_plan_preview, dismiss_memory_conflict, enforce_memory_storage_limit,
        evaluate_auto_learn, export_to_obsidian, extract_edges_via_brain,
        extract_memories_from_session, gc_memories, get_auto_learn_policy, get_edge_stats,
        get_edges_for_memory, get_memories, get_memories_by_tier, get_memory_history,
        get_memory_metrics, get_memory_provenance, get_memory_stats, get_relevant_memories,
        get_schema_info, get_search_cache_stats, get_short_term_memory, get_top_degree_nodes,
        graph_rag_detect_communities, graph_rag_build_hierarchy, graph_extract_entities, graph_rag_search, graph_rag_search_routed, graph_totals, hybrid_search_memories,
        hybrid_search_memories_rrf, hyde_search_memories, judgment_add, judgment_apply,
        judgment_list, list_memory_conflicts, list_memory_edges, list_relation_types,
        matryoshka_search_memories, memory_drilldown, memory_graph_page, multi_hop_search_memories, obsidian_sync,
        obsidian_sync_start, obsidian_sync_stop, progressive_search_memories, promote_memory,
        rebalance_ann_shards, rebuild_shard_router, reflect_on_session, refresh_graph_clusters,
        rerank_search_memories, resolve_memory_conflict, router_health, run_disk_ann_migration,
        build_ivf_pq_indexes, scan_edge_conflicts, search_memories, semantic_search_memories, set_ann_quantization,
        set_auto_learn_policy, shard_health, summarize_session, temporal_query, update_memory,
        update_memory_edge,
    },
    memory_sources::{
        create_memory_source, delete_memory_source, get_memory_source, list_memory_sources,
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
    prompt_commands::{delete_prompt_command, list_prompt_commands, save_prompt_command},
    quest::{get_quest_tracker, save_quest_tracker},
    registry::{
        get_registry_server_port, search_agents, start_registry_server, stop_registry_server,
    },
    routing::{
        approve_remote_command, deny_remote_command, get_device_permissions, list_pending_commands,
        match_ai_integration_intent, set_device_permission,
    },
    safety::{safety_check_promotion, safety_list_decisions, safety_request_permission},
    sandbox::{
        clear_agent_capabilities, grant_agent_capability, list_agent_capabilities,
        revoke_agent_capability, run_agent_in_sandbox,
    },
    settings::{
        get_app_settings, get_model_camera_positions, save_app_settings, save_model_camera_position,
    },
    streaming::{send_message_stream, send_message_stream_self_rag},
    teachable_capabilities::{
        teachable_capabilities_list, teachable_capabilities_promote,
        teachable_capabilities_record_usage, teachable_capabilities_reset,
        teachable_capabilities_set_config, teachable_capabilities_set_enabled,
        teachable_capabilities_set_rating, teachable_capabilities_summary,
    },
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
        supertonic_download_model, supertonic_install_path, supertonic_is_installed,
        supertonic_remove, supertonic_status, synthesize_tts, test_tts_provider, transcribe_audio,
    },
    vscode::{vscode_forget_window, vscode_list_known_windows, vscode_open_project},
    wiki::{
        brain_wiki_audit, brain_wiki_digest_text, brain_wiki_revisit, brain_wiki_serendipity,
        brain_wiki_spotlight,
    },
    window::{
        close_panel_window, exit_app, get_all_monitors, get_window_mode, is_dev_build, is_mcp_mode,
        open_panel_window, set_cursor_passthrough, set_pet_modal_backdrop, set_pet_mode_bounds,
        set_pet_window_size, set_window_mode, start_pet_cursor_poll, start_window_drag,
        stop_pet_cursor_poll, toggle_window_mode,
    },
    workflow_plans::{
        workflow_agent_recommendations, workflow_calendar_events, workflow_plan_create_blank,
        workflow_plan_delete, workflow_plan_list, workflow_plan_load, workflow_plan_override_llm,
        workflow_plan_save, workflow_plan_update_step, workflow_plan_validate,
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
    /// Failover policy (max retries, privacy, cooldown).
    pub failover_policy: Mutex<brain::FailoverPolicy>,
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
    /// Running MCP server handle (Chunk 15.1). `None` when the server is
    /// stopped. Start/stop via `mcp_server_start` / `mcp_server_stop`.
    pub mcp_server: TokioMutex<Option<ai_integrations::mcp::McpServerHandle>>,
    /// Pairing manager for mTLS device registry (Chunk 24.2b). `None` when
    /// LAN mode is disabled. Initialized on first `lan_enabled = true`.
    pub pairing_manager: Mutex<Option<network::pairing::PairingManager>>,
    /// LAN brain sharing state: UDP advertiser, browser, and remote connections.
    pub lan_share: Mutex<network::lan_share::LanShareState>,
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
    /// Unified provider policy registry (Chunk 35.1). Maps per-task
    /// model/provider overrides; tasks without an override fall back to
    /// the app-wide `brain_mode`.
    pub provider_policy: Mutex<brain::ProviderPolicy>,
    /// Concurrency limiter for ingest/embedding tasks (Chunk 38.1).
    /// Prevents stampeding the embedding provider when many files are
    /// ingested concurrently. Default: 4 concurrent ingest tasks.
    pub ingest_semaphore: Arc<tokio::sync::Semaphore>,
    /// Embedding queue worker metrics (Chunk 41.7). Exposes rate-limit
    /// pause state and hard-fail counter through `brain_health`.
    pub embed_worker_metrics: memory::embedding_queue::WorkerMetrics,
    /// Cancellation token for graceful shutdown of the embedding worker.
    pub embed_worker_shutdown: tokio::sync::watch::Sender<bool>,
    /// Debounced async flush handle for the ANN index (Chunk 41.10).
    pub ann_flush_handle: memory::ann_flush::AnnFlushHandle,
    /// LRU cache for bounded KG traversals (Chunk 41.13).
    pub kg_cache: memory::kg_cache::KgCache,
    /// Unix timestamp (ms) of the most recent chat activity. Set by the
    /// streaming chat path on every user turn. The embedding queue worker
    /// reads this to pause embeds for a few minutes after each chat in
    /// LocalOllama mode — embedding triggers a model swap that evicts the
    /// chat model from VRAM, costing 10-20s on the next chat. `0` means
    /// no chat has happened yet this session.
    pub last_chat_at_ms: AtomicU64,
    /// App-level safe-mode flag (RESILIENCE-1). Set by the crash-loop guard
    /// on startup if ≥ 3 crashes detected within 5 min. Subsystems check
    /// this before starting background work (plugins, embedding worker,
    /// hive relay, MCP server). Auto-clears after 10 min or manual user
    /// exit. See `docs/availability-slo.md`.
    pub safe_mode: Arc<AtomicBool>,
    /// Lazily-loaded on-device Supertonic TTS engine (chunk TTS-SUPERTONIC-1b).
    /// `get_or_init` runs the heavy 4-session ONNX load exactly once per
    /// process — the second call to `synthesize_tts` reuses the cached
    /// instance. Only present when the `tts-supertonic` feature is enabled
    /// (default on desktop builds; absent on mobile / headless-mcp).
    #[cfg(feature = "tts-supertonic")]
    pub supertonic: tokio::sync::OnceCell<Arc<voice::supertonic_tts::SupertonicTts>>,
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
    pub fn now_ms_u64() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    pub fn mark_chat_activity_now(&self) -> u64 {
        let now_ms = Self::now_ms_u64();
        self.last_chat_at_ms.store(now_ms, Ordering::Relaxed);
        now_ms
    }

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
                .no_proxy()
                .http1_only()
                .pool_max_idle_per_host(0)
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
            failover_policy: Mutex::new(brain::FailoverPolicy::default()),
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
            mcp_server: TokioMutex::new(None),
            pairing_manager: Mutex::new(None),
            lan_share: Mutex::new(network::lan_share::LanShareState::new()),
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
            provider_policy: Mutex::new(brain::ProviderPolicy::load(data_dir)),
            ingest_semaphore: Arc::new(tokio::sync::Semaphore::new(4)),
            embed_worker_metrics: memory::embedding_queue::WorkerMetrics::default(),
            embed_worker_shutdown: tokio::sync::watch::channel(false).0,
            ann_flush_handle: memory::ann_flush::AnnFlushHandle::new(),
            kg_cache: memory::kg_cache::KgCache::new(memory::kg_cache::DEFAULT_CACHE_CAPACITY),
            last_chat_at_ms: AtomicU64::new(Self::now_ms_u64()),
            safe_mode: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "tts-supertonic")]
            supertonic: tokio::sync::OnceCell::new(),
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
                .no_proxy()
                .http1_only()
                .pool_max_idle_per_host(0)
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
            failover_policy: Mutex::new(brain::FailoverPolicy::default()),
            task_manager: TokioMutex::new(tasks::manager::TaskManager::in_memory()),
            app_settings: Mutex::new(settings::AppSettings::default()),
            pet_cursor_active: Arc::new(AtomicBool::new(false)),
            workflow_engine: TokioMutex::new(
                workflows::WorkflowEngine::open(std::path::Path::new(":memory:"))
                    .expect("in-memory workflow engine must open"),
            ),
            persona_block: Mutex::new(String::new()),
            handoff_block: Mutex::new(String::new()),
            mcp_server: TokioMutex::new(None),
            pairing_manager: Mutex::new(None),
            lan_share: Mutex::new(network::lan_share::LanShareState::new()),
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
            provider_policy: Mutex::new(brain::ProviderPolicy::default()),
            ingest_semaphore: Arc::new(tokio::sync::Semaphore::new(4)),
            embed_worker_metrics: memory::embedding_queue::WorkerMetrics::default(),
            embed_worker_shutdown: tokio::sync::watch::channel(false).0,
            ann_flush_handle: memory::ann_flush::AnnFlushHandle::new(),
            kg_cache: memory::kg_cache::KgCache::new(memory::kg_cache::DEFAULT_CACHE_CAPACITY),
            last_chat_at_ms: AtomicU64::new(0),
            safe_mode: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "tts-supertonic")]
            supertonic: tokio::sync::OnceCell::new(),
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
///
/// Before creating local state, this entry point attaches to an existing
/// authenticated release/tray/dev HTTP MCP server when one is available.
/// That keeps Copilot, Claude, Cursor, Codex, and older stdio configs on
/// the same running brain instead of starting one process per agent.
pub fn run_stdio() -> std::io::Result<()> {
    if let Some(target) = find_existing_mcp_http_target() {
        eprintln!(
            "[mcp-stdio] proxying to existing TerranSoul {} MCP server on http://127.0.0.1:{}/mcp",
            target.label, target.port
        );
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        return runtime.block_on(ai_integrations::mcp::stdio::proxy_to_http(
            target.url(),
            target.token,
        ));
    }

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
        let _ = std::fs::create_dir_all(&data_dir);
        ai_integrations::mcp::enable_mcp_pet_mode();
    }

    eprintln!("[mcp-stdio] data dir: {}", data_dir.display());

    // Apply per-workspace data_root override from settings (chunk 33B.7).
    // Skip when repo-local mode is active (explicit TERRANSOUL_MCP_DATA_DIR).
    let data_dir = if repo_local {
        data_dir
    } else {
        settings::config_store::resolve_effective_data_dir(&data_dir)
    };

    let state = AppState::new(&data_dir);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(ai_integrations::mcp::stdio::run_with_state(state))
}

struct ExistingMcpHttpTarget {
    label: &'static str,
    port: u16,
    token: String,
}

impl ExistingMcpHttpTarget {
    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/mcp", self.port)
    }
}

fn find_existing_mcp_http_target() -> Option<ExistingMcpHttpTarget> {
    let release = ai_integrations::mcp::DEFAULT_PORT;
    let dev = ai_integrations::mcp::DEFAULT_DEV_PORT;
    let tray = resolve_headless_mcp_port();
    let app_root = app_data_root_for_cli();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let tray_data = resolve_headless_mcp_data_dir();

    let candidates = [
        (
            "release",
            release,
            vec!["TERRANSOUL_MCP_TOKEN"],
            vec![app_root.join("mcp-token.txt")],
        ),
        (
            "mcp-tray",
            tray,
            vec!["TERRANSOUL_MCP_TOKEN_MCP"],
            vec![
                tray_data.join("mcp-token.txt"),
                cwd.join(".vscode").join(".mcp-token"),
            ],
        ),
        (
            "dev",
            dev,
            vec!["TERRANSOUL_MCP_TOKEN_DEV"],
            vec![app_root.join("dev").join("mcp-token.txt")],
        ),
    ];

    for (label, port, env_names, token_paths) in candidates {
        let Some(token) = read_mcp_token_for_cli(&env_names, &token_paths) else {
            continue;
        };
        if probe_terransoul_status_on(port, &token) {
            return Some(ExistingMcpHttpTarget { label, port, token });
        }
    }

    None
}

fn app_data_root_for_cli() -> PathBuf {
    const BUNDLE_ID: &str = "com.terranes.terransoul";
    dirs::data_dir()
        .map(|dir| dir.join(BUNDLE_ID))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn read_mcp_token_for_cli(env_names: &[&str], token_paths: &[PathBuf]) -> Option<String> {
    for env_name in env_names {
        if let Ok(value) = std::env::var(env_name) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    for token_path in token_paths {
        if let Ok(value) = std::fs::read_to_string(token_path) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

fn probe_terransoul_status_on(port: u16, token: &str) -> bool {
    let url = format!("http://127.0.0.1:{port}/status");
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(750))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };
    let response = match client.get(url).bearer_auth(token).send() {
        Ok(response) if response.status().is_success() => response,
        _ => return false,
    };
    let Ok(body) = response.json::<serde_json::Value>() else {
        return false;
    };
    body.get("name")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|name| name.starts_with("terransoul-brain"))
}

/// Run TerranSoul as a standalone maintenance scheduler daemon
/// (Chunk 33B.10). Suitable for headless/server environments where
/// no GUI or MCP server is needed — just the periodic brain
/// maintenance loop (decay, GC, promote, edge-extract, obsidian-export).
///
/// Honors `TERRANSOUL_SCHEDULER_DATA_DIR` env var to point to the
/// data directory (defaults to the platform app-data path). Runs
/// until the process receives SIGTERM / Ctrl+C, then exits cleanly.
///
/// Tick interval defaults to 60 minutes (same as the embedded
/// scheduler). Override with `TERRANSOUL_SCHEDULER_INTERVAL_SECS`.
pub fn run_scheduler() -> std::io::Result<()> {
    let data_dir = if let Ok(p) = std::env::var("TERRANSOUL_SCHEDULER_DATA_DIR") {
        let trimmed = p.trim();
        if trimmed.is_empty() {
            resolve_data_dir_for_cli()
        } else {
            PathBuf::from(trimmed)
        }
    } else {
        resolve_data_dir_for_cli()
    };

    // Apply data_root override from settings (same as GUI/stdio paths).
    let data_dir = settings::config_store::resolve_effective_data_dir(&data_dir);
    let _ = std::fs::create_dir_all(&data_dir);

    eprintln!("[scheduler] data dir: {}", data_dir.display());

    let state = AppState::new(&data_dir);

    let tick_secs: u64 = std::env::var("TERRANSOUL_SCHEDULER_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60 * 60);
    let tick_interval = std::time::Duration::from_secs(tick_secs);
    eprintln!("[scheduler] tick interval: {tick_secs}s");

    let config = maintenance_config_from_settings(
        &state.app_settings.lock().unwrap_or_else(|e| e.into_inner()),
    );

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_clone = cancel.clone();

    // Spawn Ctrl+C handler
    runtime.spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        cancel_clone.cancel();
    });

    runtime.block_on(brain::maintenance_runtime::run_foreground(
        state,
        config,
        tick_interval,
        cancel,
    ));

    eprintln!("[scheduler] shutdown complete");
    Ok(())
}

/// Default port used by the MCP tray runtime (`--mcp-tray`).
///
/// Chosen so it does not collide with the in-app servers (release =
/// `7421`, dev `cargo tauri dev` = `7422`). External agents (Copilot,
/// Codex, Claude Code, Clawcode, etc.) launch this via `npm run mcp`
/// without conflicting with a running app.
pub const HEADLESS_MCP_PORT: u16 = 7423;

/// Resolve the data directory for the MCP tray runtime.
///
/// The MCP tray server is meant for repo-local agent sessions, so it
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

/// Load a shared MCP seed file from a resolved shared seed directory,
/// falling back to the compiled-in repository default when unavailable.
fn load_mcp_seed_text(
    shared_dir: Option<&std::path::Path>,
    file_name: &str,
    fallback: &str,
) -> String {
    let Some(dir) = shared_dir else {
        return fallback.to_string();
    };
    let shared_path = dir.join(file_name);
    std::fs::read_to_string(shared_path).unwrap_or_else(|_| fallback.to_string())
}

/// Resolve where shared MCP seed data should be loaded from.
///
/// Priority:
/// 1. `TERRANSOUL_MCP_SHARED_DIR` (when set)
/// 2. `<data_dir>/shared` (runtime colocated seed)
/// 3. `<cwd>/mcp-data/shared` (repo checkout during local dev/release runs)
///
/// Returns `None` when no on-disk shared directory exists; callers should then
/// use compiled-in seed defaults.
fn resolve_mcp_shared_seed_dir(data_dir: &std::path::Path) -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("TERRANSOUL_MCP_SHARED_DIR") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if path.is_dir() {
                return Some(path);
            }
            eprintln!(
                "[mcp] warning: TERRANSOUL_MCP_SHARED_DIR is set but not a directory: {}",
                path.display()
            );
        }
    }

    let runtime_shared = data_dir.join("shared");
    if runtime_shared.is_dir() {
        return Some(runtime_shared);
    }

    let repo_shared = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("mcp-data")
        .join("shared");
    if repo_shared.is_dir() {
        return Some(repo_shared);
    }

    None
}

/// Apply seed data and pending migrations to the MCP data directory.
///
/// On **first run** (no existing `memory.db`):
/// 1. Copies `brain_config.json` and `app_settings.json` from the
///    resolved shared dataset (or compiled fallback).
/// 2. Creates `memory.db` with the canonical schema.
/// 3. Runs all seed migrations from the resolved `shared/migrations/`.
///
/// On **subsequent runs** (existing `memory.db`):
/// 1. Ensures the `seed_migrations` tracking table exists.
/// 2. Runs only NEW migrations that haven't been applied yet.
///
/// Returns `true` when at least one migration was applied (so callers
/// can trigger first-run maintenance like embedding backfill).
fn seed_mcp_data(data_dir: &std::path::Path) -> bool {
    let db_path = data_dir.join("memory.db");
    let first_run = !db_path.exists();
    let shared_dir = resolve_mcp_shared_seed_dir(data_dir);

    if first_run {
        eprintln!("[mcp] first run detected — creating memory.db");
    }
    if let Some(dir) = shared_dir.as_deref() {
        eprintln!("[mcp] seed shared dir: {}", dir.display());
    } else {
        eprintln!("[mcp] seed shared dir: <compiled fallback>");
    }

    // Write config files (only if missing)
    let brain_cfg_path = data_dir.join("brain_config.json");
    if !brain_cfg_path.exists() {
        let seed_brain_cfg = load_mcp_seed_text(
            shared_dir.as_deref(),
            "brain_config.json",
            include_str!("../../mcp-data/shared/brain_config.json"),
        );
        if let Err(e) = std::fs::write(&brain_cfg_path, seed_brain_cfg) {
            eprintln!("[mcp] warning: failed to write seed brain_config.json: {e}");
        }
    }

    let app_cfg_path = data_dir.join("app_settings.json");
    if !app_cfg_path.exists() {
        let seed_app_cfg = load_mcp_seed_text(
            shared_dir.as_deref(),
            "app_settings.json",
            include_str!("../../mcp-data/shared/app_settings.json"),
        );
        if let Err(e) = std::fs::write(&app_cfg_path, seed_app_cfg) {
            eprintln!("[mcp] warning: failed to write seed app_settings.json: {e}");
        }
    }

    // Open (or create) memory.db
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[mcp] warning: failed to open memory.db: {e}");
            return false;
        }
    };

    // Ensure canonical schema exists (idempotent)
    if let Err(e) = memory::schema::create_canonical_schema(&conn) {
        eprintln!("[mcp] warning: failed to initialize schema: {e}");
        return false;
    }

    // Apply one-shot shared init seed
    let migration_shared_dir = shared_dir
        .as_deref()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| data_dir.join("shared"));
    match memory::seed_migrations::run_all(&conn, &migration_shared_dir) {
        Ok((applied, version)) => {
            if applied > 0 {
                eprintln!("[mcp] init seed applied (v{version:03})");
                true
            } else {
                eprintln!("[mcp] init seed already applied (v{version:03})");
                false
            }
        }
        Err(e) => {
            eprintln!("[mcp] warning: init seed failed: {e}");
            // On first run, this is fatal-ish. On subsequent runs,
            // partial progress is still committed.
            first_run
        }
    }
}

/// Backfill vectors for first-run MCP seed rows after the brain config has been
/// loaded into [`AppState`]. This makes the SQLite vector path active before the
/// first MCP agent query whenever the configured brain exposes an embedding
/// endpoint. Providers without embeddings can still use the deterministic
/// fallback embedder in headless mode.
async fn backfill_mcp_seed_embeddings(state: &AppState) -> usize {
    let unembedded = {
        let store = match state.memory_store.lock() {
            Ok(store) => store,
            Err(e) => {
                eprintln!("[mcp] mcp-seed-embedded failed to lock store: {e}");
                return 0;
            }
        };
        match store.unembedded_ids() {
            Ok(ids) => ids,
            Err(e) => {
                eprintln!("[mcp] mcp-seed-embedded failed to list rows: {e}");
                return 0;
            }
        }
    };

    if unembedded.is_empty() {
        eprintln!("[mcp] mcp-seed-embedded count=0 remaining=0");
        return 0;
    }

    let (brain_mode, active_brain) = (
        state.brain_mode.lock().ok().and_then(|g| g.clone()),
        state.active_brain.lock().ok().and_then(|g| g.clone()),
    );

    if brain_mode.is_none() && active_brain.is_none() {
        eprintln!("[mcp] mcp-seed-embedded skipped: no embedding-capable brain configured");
        return 0;
    }

    let mut count = 0usize;
    let mut offline_count = 0usize;
    for (id, content) in &unembedded {
        let (embedding, used_offline) = match brain::embed_for_mode(
            content,
            brain_mode.as_ref(),
            active_brain.as_deref(),
        )
        .await
        {
            Some(embedding) => (Some(embedding), false),
            None if ai_integrations::mcp::is_mcp_pet_mode() => {
                (memory::offline_embed::embed_text(content), true)
            }
            None => (None, false),
        };
        if let Some(embedding) = embedding {
            let store = match state.memory_store.lock() {
                Ok(store) => store,
                Err(e) => {
                    eprintln!("[mcp] mcp-seed-embedded stopped: store lock failed: {e}");
                    break;
                }
            };
            if store.set_embedding(*id, &embedding).is_ok() {
                count += 1;
                if used_offline {
                    offline_count += 1;
                }
            }
        }
    }

    let remaining = unembedded.len().saturating_sub(count);
    eprintln!(
        "[mcp] mcp-seed-embedded count={count} offline={offline_count} remaining={remaining}"
    );
    count
}

/// Run the `--mcp-setup` CLI subcommand.
///
/// Detects AI coding editor config directories (`.vscode/`, `~/.cursor/`,
/// `~/.codex/`, `~/.claude/`, `~/.config/opencode/`) and writes the
/// MCP server entry pointing at the MCP tray HTTP server.
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
        eprintln!(
            "[mcp-setup] checked: .vscode/, ~/.cursor/, ~/.codex/, ~/.claude/, ~/.config/opencode/"
        );
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

fn write_mcp_token_file(workspace_root: &std::path::Path, token: &str) -> std::io::Result<PathBuf> {
    let vscode_dir = workspace_root.join(".vscode");
    std::fs::create_dir_all(&vscode_dir)?;
    let token_file = vscode_dir.join(".mcp-token");
    std::fs::write(&token_file, token)?;
    Ok(token_file)
}

fn maintenance_config_from_settings(
    settings: &settings::AppSettings,
) -> brain::maintenance_scheduler::MaintenanceConfig {
    let cooldown_ms = settings.maintenance_cooldown_ms();
    brain::maintenance_scheduler::MaintenanceConfig {
        decay_cooldown_ms: cooldown_ms,
        garbage_collect_cooldown_ms: cooldown_ms,
        promote_tier_cooldown_ms: cooldown_ms,
        edge_extract_cooldown_ms: cooldown_ms,
        obsidian_export_cooldown_ms: cooldown_ms,
        ann_compact_cooldown_ms: cooldown_ms,
    }
}

fn spawn_shared_maintenance(state: &AppState, label: &str) {
    let config = state
        .app_settings
        .lock()
        .map(|settings| maintenance_config_from_settings(&settings))
        .unwrap_or_default();
    let runtime = brain::maintenance_runtime::spawn(
        state.clone(),
        config,
        brain::maintenance_runtime::DEFAULT_TICK_INTERVAL,
    );
    eprintln!(
        "[{label}] maintenance scheduler started; state={}",
        runtime.state_path().display()
    );
}

/// Spawn the self-healing embedding retry worker (Chunk 38.2).
///
/// Drains `pending_embeddings` every 10 s using the batch embedding
/// endpoint. On boot, backfills the queue with any memories that
/// already have NULL embeddings — this self-heals databases populated
/// before this worker existed.
fn spawn_embedding_queue_worker(state: &AppState, label: &str) {
    let mode = state.brain_mode.lock().ok().and_then(|m| m.clone());
    if matches!(mode, Some(brain::BrainMode::LocalOllama { .. }))
        && state.last_chat_at_ms.load(Ordering::Relaxed) == 0
    {
        state.mark_chat_activity_now();
    }
    let shutdown_rx = state.embed_worker_shutdown.subscribe();
    let metrics = state.embed_worker_metrics.clone();
    memory::embedding_queue::spawn_worker_with_metrics(state.clone(), shutdown_rx, metrics);
    eprintln!("[{label}] embedding-queue worker started; tick=10s, provider-adaptive batch");
}

/// Pre-warm the Ollama chat model so the first user reply lands in milliseconds
/// instead of paying a 10-20 s cold-load on consumer GPUs. Sends a load-only
/// `/api/chat` request (empty messages + `keep_alive: "30m"`) in the background
/// when the active brain mode is LocalOllama. No-op for cloud providers.
pub(crate) fn spawn_local_ollama_warmup(state: &AppState, label: &str) {
    let mode = state.brain_mode.lock().ok().and_then(|m| m.clone());
    let Some(brain::BrainMode::LocalOllama { model: mode_model }) = mode else {
        return;
    };
    let model = state
        .active_brain
        .lock()
        .ok()
        .and_then(|m| m.clone())
        .unwrap_or(mode_model);
    // Register this chat model so every embed call (app, MCP, gRPC, …)
    // re-warms it after running. See ollama_agent::set_chat_model_for_warmup.
    crate::brain::ollama_agent::set_chat_model_for_warmup(&model);
    // Push the chat-activity quiet window so the embedding worker does not
    // race the warm-up to load nomic-embed-text and evict the chat model.
    state.mark_chat_activity_now();
    let client = state.ollama_client.clone();
    let label = label.to_string();
    tauri::async_runtime::spawn(async move {
        let url = format!("{}/api/chat", brain::ollama_agent::OLLAMA_BASE_URL);
        // 1-token streamed chat forces Ollama to load the weights and warm
        // the same streaming endpoint used by the first real user reply.
        let body = serde_json::json!({
            "model": model,
            "messages": [{ "role": "user", "content": "Hi" }],
            // Must match the real chat path's num_ctx (see streaming.rs
            // line ~1217) — any mismatch forces Ollama to reload the
            // model weights on the first user reply, defeating the warmup.
            "options": { "num_predict": 1, "num_ctx": 2048, "num_batch": 512 },
            "keep_alive": "30m",
            "stream": true,
            "think": false,
        });
        let started = std::time::Instant::now();
        match client.post(&url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                let _ = resp.bytes().await;
                eprintln!(
                    "[{label}] ollama warm-up done in {} ms (status {})",
                    started.elapsed().as_millis(),
                    status
                );
            }
            Err(e) => {
                eprintln!("[{label}] ollama warm-up skipped: {e}");
            }
        }
    });
}

/// Spawn the debounced ANN flush background task (Chunk 41.10).
///
/// The task waits for flush signals, debounces them (200 ms window), then
/// acquires the `memory_store` mutex and saves all dirty ANN indices.
fn spawn_ann_flush_task(state: &AppState) {
    let handle = state.ann_flush_handle.clone();
    let store_mutex = state.0.clone();
    let rt = tauri::async_runtime::handle();
    let _guard = rt.inner().enter();
    memory::ann_flush::spawn_flush_task(handle, move || {
        let store = store_mutex
            .memory_store
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        store.ann_save_all()
    });
}

/// Check whether a process with the given PID is still running.
#[allow(dead_code)]
fn is_process_alive(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::io::Read;
        use std::process::{Command, Stdio};

        let mut child = match Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    if std::time::Instant::now() >= deadline {
                        let _ = child.kill();
                        return true; // assume alive if tasklist hangs
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(_) => return false,
            }
        }

        let mut out = String::new();
        if let Some(mut stdout) = child.stdout.take() {
            let _ = stdout.read_to_string(&mut out);
        }
        out.contains(&pid.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .is_ok_and(|o| o.status.success())
    }
}

/// Kill any running headless MCP service (port 7423) when the app starts
/// in dev or release mode. The headless runner writes its PID to
/// `mcp-data/self_improve_mcp_process.pid`; we read and kill that process.
///
/// First attempts a graceful `POST /shutdown` so the Tauri tray process
/// can clean up its system tray icon. Falls back to force-kill if the
/// graceful request fails or times out.
#[allow(dead_code)]
fn kill_headless_mcp_if_running() {
    let cwd = std::env::current_dir().unwrap_or_default();
    let mcp_data = cwd.join("mcp-data");
    let pid_path = mcp_data.join("self_improve_mcp_process.pid");

    // Read PID early — we need it for the fallback.
    let pid = std::fs::read_to_string(&pid_path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok());

    if pid.is_none() {
        // No PID file → nothing to kill.
        return;
    }
    let pid = pid.unwrap();

    // Try graceful shutdown via HTTP first.
    let token = std::fs::read_to_string(mcp_data.join("mcp-token.txt"))
        .or_else(|_| std::fs::read_to_string(cwd.join(".vscode").join(".mcp-token")))
        .ok()
        .map(|s| s.trim().to_string());

    let graceful_ok = if let Some(token) = token {
        // Blocking HTTP request — we are in sync Tauri setup, not async.
        let port = std::env::var("TERRANSOUL_MCP_PORT")
            .ok()
            .and_then(|s| s.trim().parse::<u16>().ok())
            .unwrap_or(HEADLESS_MCP_PORT);

        let url = format!("http://127.0.0.1:{port}/shutdown");
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build();

        if let Ok(client) = client {
            let resp = client
                .post(&url)
                .header("Authorization", format!("Bearer {token}"))
                .send();

            if resp.is_ok_and(|r| r.status().is_success()) {
                // Wait up to 3 seconds for the process to exit.
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
                loop {
                    if !is_process_alive(pid) {
                        break;
                    }
                    if std::time::Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                !is_process_alive(pid)
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if graceful_ok {
        let _ = std::fs::remove_file(&pid_path);
        eprintln!("[app] gracefully stopped headless MCP service (pid {pid})");
        return;
    }

    // Fallback: force-kill the process.
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }
    let _ = std::fs::remove_file(&pid_path);
    eprintln!("[app] force-killed headless MCP service (pid {pid})");
}

const MAIN_WINDOW_LABEL: &str = "main";

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name).map(|v| v == "1").unwrap_or(false)
}

fn should_hide_mcp_close(is_mcp_tray: bool, window_label: &str) -> bool {
    is_mcp_tray && window_label == MAIN_WINDOW_LABEL
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpFrontendState {
    Ready,
    Building,
    Failed,
}

fn mcp_frontend_status_path(data_dir: &Path) -> PathBuf {
    std::env::var("TERRANSOUL_MCP_FRONTEND_STATUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| data_dir.join("frontend-build-status.json"))
}

fn mcp_frontend_dist_index() -> PathBuf {
    std::env::var("TERRANSOUL_MCP_FRONTEND_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("dist")
                .join("index.html")
        })
}

fn mcp_frontend_state_from_dir(data_dir: &Path) -> McpFrontendState {
    let status_path = mcp_frontend_status_path(data_dir);
    let Ok(raw) = std::fs::read_to_string(status_path) else {
        return McpFrontendState::Ready;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return McpFrontendState::Ready;
    };
    match value.get("status").and_then(serde_json::Value::as_str) {
        Some("building") => McpFrontendState::Building,
        Some("failed") => McpFrontendState::Failed,
        _ => McpFrontendState::Ready,
    }
}

fn mcp_frontend_state(app: &tauri::AppHandle) -> McpFrontendState {
    app.try_state::<AppState>()
        .map(|state| mcp_frontend_state_from_dir(&state.data_dir))
        .unwrap_or(McpFrontendState::Ready)
}

fn mcp_frontend_ready(app: &tauri::AppHandle) -> bool {
    mcp_frontend_state(app) == McpFrontendState::Ready
}

fn mcp_ui_menu_state(data_dir: &Path) -> (&'static str, bool) {
    match mcp_frontend_state_from_dir(data_dir) {
        McpFrontendState::Ready => ("Show UI", true),
        McpFrontendState::Building => ("Building UI...", false),
        McpFrontendState::Failed => ("UI build failed", false),
    }
}

fn main_window_url() -> Result<tauri::WebviewUrl, String> {
    if env_flag_enabled("TERRANSOUL_MCP_TRAY_MODE") {
        let dist_index = mcp_frontend_dist_index();
        if dist_index.exists() {
            let url = url::Url::from_file_path(&dist_index)
                .map_err(|_| format!("invalid MCP frontend path: {}", dist_index.display()))?;
            return Ok(tauri::WebviewUrl::External(url));
        }
    }

    if cfg!(debug_assertions) {
        Ok(tauri::WebviewUrl::External(
            "http://localhost:1420"
                .parse()
                .map_err(|e: url::ParseError| e.to_string())?,
        ))
    } else {
        Ok(tauri::WebviewUrl::App("index.html".into()))
    }
}

fn ensure_main_window(app: &tauri::AppHandle) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        return Ok(window);
    }

    let mut builder = tauri::WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, main_window_url()?)
        .title("TerranSoul")
        .inner_size(420.0, 700.0)
        .resizable(true)
        .decorations(true)
        .transparent(true)
        .background_color(Color(0, 0, 0, 0))
        .always_on_top(false)
        .skip_taskbar(false);

    // Use a separate WebView2 user data directory in MCP tray mode so it
    // does not conflict with a concurrently-running main app instance.
    if env_flag_enabled("TERRANSOUL_MCP_TRAY_MODE") {
        let mcp_webview_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("EBWebView-mcp");
        builder = builder.data_directory(mcp_webview_dir);
    }

    let window = builder.build().map_err(|e| e.to_string())?;

    if let Some(icon) = app.default_window_icon().cloned() {
        let _ = window.set_icon(icon);
    }

    Ok(window)
}

fn show_mcp_ui(app: &tauri::AppHandle) {
    match ensure_main_window(app) {
        Ok(window) => {
            let _ =
                commands::window::apply_window_mode(&window, commands::window::WindowMode::Window);
            if let Some(state) = app.try_state::<AppState>() {
                if let Ok(mut mode) = state.window_mode.lock() {
                    *mode = commands::window::WindowMode::Window;
                }
            }
            let _ = window.show();
            let _ = window.set_skip_taskbar(false);
            let _ = window.set_focus();
        }
        Err(e) => eprintln!("[mcp-app] failed to show MCP UI: {e}"),
    }
}

fn toggle_mcp_ui(app: &tauri::AppHandle) {
    if !mcp_frontend_ready(app) {
        update_mcp_tray_labels(app, true);
        return;
    }

    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
            let _ = window.set_skip_taskbar(true);
            return;
        }
    }

    show_mcp_ui(app);
}

/// Toggle the MCP HTTP server on/off from the tray menu.
fn toggle_mcp_server(app: &tauri::AppHandle) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<AppState>();
        let is_running = state.mcp_server.lock().await.is_some();

        if is_running {
            // Stop the server
            let mut guard = state.mcp_server.lock().await;
            if let Some(handle) = guard.take() {
                handle.stop();
                let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle.task).await;
            }
            eprintln!("[mcp-tray] MCP server stopped via tray toggle");
            update_mcp_tray_labels(&app_handle, false);
        } else {
            // Start the server
            let data_dir = state.data_dir.clone();
            let token = match ai_integrations::mcp::auth::load_or_create(&data_dir) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("[mcp-tray] failed to load/create token: {e}");
                    return;
                }
            };
            let lan_public_read_only = state.app_settings.lock().ok().is_some_and(|settings| {
                settings.lan_enabled
                    && matches!(
                        settings.lan_auth_mode,
                        crate::settings::LanAuthMode::PublicReadOnly
                    )
            });
            match ai_integrations::mcp::start_server_with_activity(
                state.inner().clone(),
                HEADLESS_MCP_PORT,
                token,
                false,
                lan_public_read_only,
                None,
            )
            .await
            {
                Ok(handle) => {
                    eprintln!("[mcp-tray] MCP server started on port {}", handle.port);
                    *state.mcp_server.lock().await = Some(handle);
                    update_mcp_tray_labels(&app_handle, true);
                }
                Err(e) => eprintln!("[mcp-tray] failed to start MCP server: {e}"),
            }
        }
    });
}

/// Update the tray menu labels to reflect MCP server running state.
fn update_mcp_tray_labels(app: &tauri::AppHandle, running: bool) {
    let status_text = if running {
        "MCP Server ● Running"
    } else {
        "MCP Server ○ Stopped"
    };
    let toggle_text = if running {
        "Stop MCP Server"
    } else {
        "Start MCP Server"
    };

    let Ok(status_label) = MenuItem::with_id(app, "mcp_status", status_text, false, None::<&str>)
    else {
        return;
    };
    let Ok(toggle_server) =
        MenuItem::with_id(app, "mcp_toggle_server", toggle_text, true, None::<&str>)
    else {
        return;
    };
    let (ui_text, ui_enabled) = app
        .try_state::<AppState>()
        .map(|state| mcp_ui_menu_state(&state.data_dir))
        .unwrap_or(("Show UI", true));
    let Ok(toggle_ui) = MenuItem::with_id(app, "mcp_toggle_ui", ui_text, ui_enabled, None::<&str>)
    else {
        return;
    };
    let Ok(quit) = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>) else {
        return;
    };
    let Ok(menu) = Menu::with_items(app, &[&status_label, &toggle_server, &toggle_ui, &quit])
    else {
        return;
    };

    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_menu(Some(menu));
    }
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
            list_embedding_models,
            get_embedding_registry_state,
            plan_embedding_model_switch,
            switch_embedding_model,
            check_ollama_status,
            get_ollama_models,
            pull_ollama_model,
            warmup_local_ollama,
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
            progressive_search_memories,
            hyde_search_memories,
            rerank_search_memories,
            matryoshka_search_memories,
            memory_drilldown,
            backfill_embeddings,
            backfill_embedding_model_id,
            set_ann_quantization,
            compact_ann,
            get_schema_info,
            get_memory_stats,
            get_memory_metrics,
            get_search_cache_stats,
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
            update_memory_edge,
            detach_memory_node,
            list_memory_edges,
            // BRAIN-REPO-RAG-1a: memory-sources registry
            list_memory_sources,
            get_memory_source,
            create_memory_source,
            delete_memory_source,
            // BRAIN-REPO-RAG-1b-i: per-repo ingest backend (feature `repo-rag`)
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_add_source,
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_sync,
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_remove_source,
            // BRAIN-REPO-RAG-1c-a: source-scoped retrieval
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_search,
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_list_files,
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_read_file,
            // BRAIN-REPO-RAG-1d: Aider-style map + signature compression
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_map,
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_signatures,
            // BRAIN-REPO-RAG-1e: OAuth device flow for private repos
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_oauth_github_start,
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_oauth_github_poll,
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_oauth_github_status,
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::repo_oauth_github_clear,
            // BRAIN-REPO-RAG-2a: cross-source knowledge-graph projection
            #[cfg(feature = "repo-rag")]
            crate::commands::repos::cross_source_graph_nodes,
            // BRAIN-REPO-RAG-1c-b-ii-a: cross-source All-mode fan-out
            crate::commands::cross_source::cross_source_search,
            memory_graph_page,
            disk_ann_plan_preview,
            disk_ann_migration_status,
            run_disk_ann_migration,
            build_ivf_pq_indexes,
            // Shard health, router health, graph observability (Chunk 50.1)
            shard_health,
            router_health,
            rebuild_shard_router,
            rebalance_ann_shards,
            refresh_graph_clusters,
            get_top_degree_nodes,
            graph_totals,
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
            // Scope-routed retrieval (GRAPHRAG-1c)
            graph_rag_search_routed,
            // Hierarchical community summaries (GRAPHRAG-1a)
            graph_rag_build_hierarchy,
            // Structured entity/relationship extraction (GRAPHRAG-1b)
            graph_extract_entities,
            // Temporal reasoning queries (Chunk 17.3)
            temporal_query,
            daily_brief_query,
            // Judgment rules (Chunk 33B.1)
            judgment_add,
            judgment_list,
            judgment_apply,
            reflect_on_session,
            // Memory versioning (Chunk 16.12)
            get_memory_history,
            get_memory_provenance,
            brain_wiki_audit,
            brain_wiki_digest_text,
            brain_wiki_revisit,
            brain_wiki_serendipity,
            brain_wiki_spotlight,
            start_registry_server,
            stop_registry_server,
            get_registry_server_port,
            search_agents,
            grant_agent_capability,
            revoke_agent_capability,
            list_agent_capabilities,
            run_agent_in_sandbox,
            clear_agent_capabilities,
            safety_request_permission,
            safety_list_decisions,
            safety_check_promotion,
            publish_agent_message,
            subscribe_agent_topic,
            unsubscribe_agent_topic,
            get_agent_messages,
            list_agent_subscriptions,
            set_window_mode,
            get_window_mode,
            toggle_window_mode,
            set_cursor_passthrough,
            set_pet_modal_backdrop,
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
            dispatch_hermes_job,
            hermes_office_status,
            health_check_providers,
            get_next_provider,
            get_failover_summary,
            get_failover_policy,
            set_failover_policy,
            select_provider_with_constraints,
            get_brain_selection,
            get_provider_policy,
            set_provider_policy,
            set_provider_task_override,
            remove_provider_task_override,
            resolve_provider_for_task,
            embedding_queue_status,
            embedding_queue_diagnostics,
            brain_eviction_log,
            get_agent_routing,
            set_agent_route,
            remove_agent_route,
            resolve_provider_for_role,
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
            test_tts_provider,
            supertonic_install_path,
            supertonic_is_installed,
            supertonic_status,
            supertonic_remove,
            supertonic_download_model,
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
            install_docker_desktop,
            install_podman,
            ingest_document,
            cancel_ingest_task,
            resume_ingest_task,
            get_all_tasks,
            // Context folders — user-defined knowledge directories (brute-force scan)
            scan_context_folder,
            add_context_folder,
            remove_context_folder,
            toggle_context_folder,
            list_context_folders,
            sync_context_folders,
            // Context ↔ knowledge conversion
            list_context_folder_memories,
            export_knowledge_to_folder,
            convert_context_to_knowledge,
            // Knowledge graph ↔ context files
            export_kg_subtree,
            import_file_to_knowledge_graph,
            list_lan_addresses,
            get_copilot_session_status,
            start_pairing,
            confirm_pairing,
            revoke_device,
            list_paired_devices,
            // LAN brain sharing — knowledge exchange between TerranSoul instances
            lan_share_start,
            lan_share_stop,
            lan_share_discover,
            lan_share_stop_discovery,
            lan_share_connect,
            lan_share_disconnect,
            lan_share_search,
            lan_share_search_all,
            lan_share_remote_health,
            lan_share_status,
            import_user_model,
            list_user_models,
            delete_user_model,
            read_user_model_bytes,
            update_user_model,
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
            // Hermes Agent (NousResearch) — YAML, marker-managed
            setup_hermes_mcp,
            setup_hermes_mcp_stdio,
            remove_hermes_mcp,
            remove_vscode_mcp,
            remove_claude_mcp,
            remove_codex_mcp,
            list_mcp_clients,
            // Companion AI detect-and-link registry — INTEGRATE-1
            commands::companions::companions_list,
            commands::companions::companions_detect_one,
            commands::companions::companions_open_install_page,
            commands::companions::companions_run_guided_install,
            commands::companions::companions_check_update,
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
            get_self_improve_workboard,
            start_self_improve,
            stop_self_improve,
            set_self_improve_autostart,
            get_self_improve_metrics,
            get_self_improve_runs,
            clear_self_improve_log,
            get_self_improve_gate_metrics,
            get_self_improve_gate_history,
            promote_to_milestone_chunk,
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
            code_generate_skills,
            code_export_graph,
            code_explain_graph,
            code_architecture_tours,
            code_diff_overlay,
            code_list_groups,
            code_create_group,
            code_delete_group,
            code_add_repo_to_group,
            code_remove_repo_from_group,
            code_group_status,
            code_extract_contracts,
            code_extract_negatives,
            code_detect_harnesses,
            code_import_sessions,
            code_replay_session,
            code_replay_all_sessions,
            code_list_group_contracts,
            code_cross_repo_query,
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
            coding_session_resume,
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
            // Extensible prompt commands
            list_prompt_commands,
            save_prompt_command,
            delete_prompt_command,
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
            let mcp_app_mode = env_flag_enabled("TERRANSOUL_MCP_APP_MODE");
            let mcp_tray_mode = env_flag_enabled("TERRANSOUL_MCP_TRAY_MODE");

            // The MCP tray service (port 7423) uses a separate data dir
            // (mcp-data/) and target-dir (target-mcp/) so it can coexist
            // with the dev/release app without resource conflicts. Do NOT
            // kill it here — the user manages MCP lifecycle externally via
            // `npm run mcp` / copilot-start-mcp.mjs.

            let data_dir = if mcp_app_mode {
                ai_integrations::mcp::enable_mcp_pet_mode();
                let mcp_dir = std::env::var("TERRANSOUL_MCP_DATA_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| {
                        std::env::current_dir()
                            .unwrap_or_else(|_| PathBuf::from("."))
                            .join("mcp-data")
                    });
                std::fs::create_dir_all(&mcp_dir).expect("failed to create MCP data directory");
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

            // Apply per-workspace data_root override from settings (chunk 33B.7).
            // MCP/tray modes skip the override — they already have explicit paths.
            let data_dir = if mcp_app_mode || mcp_tray_mode {
                data_dir
            } else {
                settings::config_store::resolve_effective_data_dir(&data_dir)
            };

            app.manage(AppState::new(&data_dir));
            let state = app.state::<AppState>();

            // Attach the ANN flush handle to the memory store.
            {
                let mut store = state.memory_store.lock().unwrap();
                store.set_flush_handle(state.ann_flush_handle.clone());
            }

            spawn_shared_maintenance(&state, if mcp_app_mode { "mcp-app" } else { "app" });
            spawn_local_ollama_warmup(&state, if mcp_app_mode { "mcp-app" } else { "app" });
            spawn_embedding_queue_worker(&state, if mcp_app_mode { "mcp-app" } else { "app" });
            spawn_ann_flush_task(&state);

            // Apply shared seed data (mcp-data/shared/) to ALL modes.
            // The migration runner is idempotent: it only applies NEW
            // migrations and uses compiled-in fallback when the on-disk
            // shared/ directory is missing. This guarantees release,
            // dev, and MCP-tray runs share the same baseline knowledge
            // (rules, agent skills catalogue, reverse-engineering
            // lessons) without ever touching the user's runtime data.
            let shared_seeded = seed_mcp_data(&data_dir);

            // Auto-start the MCP HTTP server on the coding-agent port (7423)
            // when running in MCP full-UI mode so external agents can talk
            // to the live app while the user can still open brain config,
            // MCP config, memory, and graph panels from the tray UI.
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

                    // Write .vscode/.mcp-token for editor/agent pickup
                    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                    if let Err(e) = write_mcp_token_file(&cwd, &token) {
                        eprintln!("[mcp-app] warning: failed to write .vscode/.mcp-token: {e}");
                    }

                    let lan_public_read_only = app_state_inner
                        .app_settings
                        .lock()
                        .ok()
                        .is_some_and(|settings| {
                            settings.lan_enabled
                                && matches!(
                                    settings.lan_auth_mode,
                                    crate::settings::LanAuthMode::PublicReadOnly
                                )
                        });

                    // Headless MCP idle timeout: default 300s (5 min),
                    // configurable via TERRANSOUL_MCP_IDLE_TIMEOUT env var.
                    // Set to 0 to disable.
                    let idle_timeout_secs: u64 = std::env::var("TERRANSOUL_MCP_IDLE_TIMEOUT")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(300);

                    // Resume session if --resume <name> was passed.
                    if let Ok(resume_name) = std::env::var("TERRANSOUL_MCP_RESUME") {
                        match coding::session_registry::resolve(
                            &app_state_inner.data_dir,
                            &resume_name,
                        ) {
                            Ok(Some(entry)) => {
                                eprintln!(
                                    "[mcp-app] resuming session '{}' (id: {})",
                                    resume_name, entry.session_id
                                );
                            }
                            Ok(None) => {
                                eprintln!(
                                    "[mcp-app] session '{}' not found in registry; starting fresh",
                                    resume_name
                                );
                            }
                            Err(e) => {
                                eprintln!(
                                    "[mcp-app] failed to resolve session '{}': {e}",
                                    resume_name
                                );
                            }
                        }
                    }

                    match ai_integrations::mcp::start_server_full(
                        app_state_inner.clone(),
                        HEADLESS_MCP_PORT,
                        token.clone(),
                        false,
                        lan_public_read_only,
                        Some(app_handle),
                        idle_timeout_secs,
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

                    // Do not block MCP server startup on background seed embedding backfill.
                    if shared_seeded {
                        let app_state_for_backfill = app_state_inner.clone();
                        tauri::async_runtime::spawn(async move {
                            backfill_mcp_seed_embeddings(&app_state_for_backfill).await;
                        });
                    }
                });
            } else if shared_seeded {
                // Dev / release modes: backfill embeddings for any new
                // shared-seed rows in the background so RAG quality
                // catches up without blocking startup.
                let app_state_inner = state.inner().clone();
                tauri::async_runtime::spawn(async move {
                    backfill_mcp_seed_embeddings(&app_state_inner).await;
                });
            }

            let identity = load_or_generate_identity(&data_dir)
                .unwrap_or_else(|_| identity::DeviceIdentity::generate());
            let device_id = identity.device_id.clone();
            *state.device_identity.lock().unwrap() = Some(identity);

            let devices = load_trusted_devices(&data_dir);
            *state.trusted_devices.lock().unwrap() = devices;

            *state.command_router.blocking_lock() = routing::CommandRouter::new(&device_id);

            // System tray: MCP-tray mode gets a dedicated menu indicating
            // the server is running; normal mode keeps the existing layout.
            if mcp_tray_mode {
                let status_label = MenuItem::with_id(
                    app,
                    "mcp_status",
                    "MCP Server ● Running",
                    false,
                    None::<&str>,
                )?;
                let toggle_server = MenuItem::with_id(
                    app,
                    "mcp_toggle_server",
                    "Stop MCP Server",
                    true,
                    None::<&str>,
                )?;
                let (ui_text, ui_enabled) = mcp_ui_menu_state(&data_dir);
                let toggle_ui = MenuItem::with_id(
                    app,
                    "mcp_toggle_ui",
                    ui_text,
                    ui_enabled,
                    None::<&str>,
                )?;
                let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
                let menu =
                    Menu::with_items(app, &[&status_label, &toggle_server, &toggle_ui, &quit])?;

                TrayIconBuilder::with_id("main")
                    .icon(app.default_window_icon().cloned().unwrap())
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .tooltip("TerranSoul MCP Server")
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "mcp_toggle_server" => {
                            toggle_mcp_server(app);
                        }
                        "mcp_toggle_ui" => {
                            toggle_mcp_ui(app);
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        } = event
                        {
                            toggle_mcp_ui(tray.app_handle());
                        }
                    })
                    .build(app)?;

                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    for _ in 0..120 {
                        update_mcp_tray_labels(&app_handle, true);
                        if mcp_frontend_ready(&app_handle) {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                });

                // In MCP mode, destroy the auto-created main window. It will
                // be recreated on demand (via `ensure_main_window`) with a
                // separate WebView2 user-data folder when the user clicks
                // "Show UI" from the tray. This avoids ERR_CONNECTION_REFUSED
                // caused by two instances fighting over the same WebView2
                // user-data directory.
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.destroy();
                }
            } else {
                // Normal tray: mode label + Show/Hide + Window/Pet toggle + Quit
                let title = if cfg!(debug_assertions) {
                    "TerranSoul (Dev)".to_string()
                } else {
                    "TerranSoul".to_string()
                };
                let status_label =
                    MenuItem::with_id(app, "app_status", &title, false, None::<&str>)?;
                let show_hide =
                    MenuItem::with_id(app, "show_hide", "Show / Hide", true, None::<&str>)?;
                let mode_toggle = MenuItem::with_id(
                    app,
                    "mode_toggle",
                    "Switch to Pet Mode",
                    true,
                    None::<&str>,
                )?;
                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu =
                    Menu::with_items(app, &[&status_label, &show_hide, &mode_toggle, &quit])?;

                TrayIconBuilder::new()
                    .icon(app.default_window_icon().cloned().unwrap())
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .tooltip(&title)
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
                        if let tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        } = event
                        {
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            } // end else (normal tray)

            // Set the window icon so the taskbar / title bar shows the app icon
            // instead of the default WebView icon during development.
            if let Some(window) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon().cloned() {
                    let _ = window.set_icon(icon);
                }
                // Only open DevTools in debug builds for the normal GUI mode,
                // never for the MCP tray server (it has no meaningful page).
                #[cfg(debug_assertions)]
                if !mcp_tray_mode {
                    window.open_devtools();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // In MCP-tray mode, intercept the close button to hide the
            // window instead of destroying it. The user must right-click
            // tray → Exit to actually quit.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if should_hide_mcp_close(
                    env_flag_enabled("TERRANSOUL_MCP_TRAY_MODE"),
                    window.label(),
                ) {
                    api.prevent_close();
                    let _ = window.hide();
                    let _ = window.set_skip_taskbar(true);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // In MCP-tray mode, prevent the app from exiting when all
            // windows are hidden/closed. The tray icon keeps the process
            // alive; only the "Exit" menu item calls app.exit(0).
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if env_flag_enabled("TERRANSOUL_MCP_TRAY_MODE") {
                    api.prevent_exit();
                }
            }
        });
}

#[cfg(test)]
mod mcp_window_tests {
    use super::should_hide_mcp_close;

    #[test]
    fn mcp_tray_close_hides_main_window() {
        assert!(should_hide_mcp_close(true, "main"));
    }

    #[test]
    fn mcp_tray_close_does_not_capture_panel_windows() {
        assert!(!should_hide_mcp_close(true, "panel-brain"));
        assert!(!should_hide_mcp_close(true, "panel-memory"));
    }

    #[test]
    fn normal_mode_close_is_not_captured() {
        assert!(!should_hide_mcp_close(false, "main"));
    }
}

#[cfg(test)]
mod mcp_seed_tests {
    use super::{load_mcp_seed_text, resolve_mcp_shared_seed_dir, seed_mcp_data};

    #[test]
    fn resolve_shared_seed_prefers_runtime_shared() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let runtime_shared = tmp.path().join("shared");
        std::fs::create_dir_all(&runtime_shared).expect("shared dir");

        let resolved = resolve_mcp_shared_seed_dir(tmp.path()).expect("runtime shared");

        assert_eq!(resolved, runtime_shared);
    }

    #[test]
    fn load_mcp_seed_text_prefers_tracked_shared_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let shared = tmp.path().join("shared");
        std::fs::create_dir_all(&shared).expect("shared dir");
        std::fs::write(shared.join("memory-seed.sql"), "-- shared seed").expect("write seed");

        let loaded = load_mcp_seed_text(Some(&shared), "memory-seed.sql", "-- fallback seed");

        assert_eq!(loaded, "-- shared seed");
    }

    #[test]
    fn load_mcp_seed_text_falls_back_when_shared_file_missing() {
        let loaded = load_mcp_seed_text(None, "memory-seed.sql", "-- fallback seed");

        assert_eq!(loaded, "-- fallback seed");
    }

    #[test]
    fn seed_mcp_data_reports_first_run_only() {
        let tmp = tempfile::tempdir().expect("tempdir");

        assert!(seed_mcp_data(tmp.path()));
        assert!(!seed_mcp_data(tmp.path()));
        assert!(tmp.path().join("memory.db").exists());
    }
}
