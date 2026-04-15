use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;
use tauri::Emitter;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tokio::sync::Mutex as TokioMutex;

pub mod agent;
pub mod brain;
pub mod commands;
pub mod identity;
pub mod link;
pub mod memory;
pub mod messaging;
pub mod orchestrator;
pub mod package_manager;
pub mod registry_server;
pub mod routing;
pub mod sandbox;
pub mod settings;
pub mod sync;
pub mod voice;

use commands::{
    agent::list_agents,
    brain::{
        check_ollama_status, clear_active_brain, get_active_brain, get_brain_mode,
        get_next_provider, get_ollama_models, get_system_info, health_check_providers,
        list_free_providers, pull_ollama_model, recommend_brain_models, set_active_brain,
        set_brain_mode,
    },
    character::load_vrm,
    chat::{get_conversation, send_message},
    identity::{
        add_trusted_device_cmd, get_device_identity, get_pairing_qr, list_trusted_devices,
        remove_trusted_device_cmd,
    },
    link::{connect_to_peer, disconnect_link, get_link_status, start_link_server},
    memory::{
        add_memory, delete_memory, extract_memories_from_session, get_memories,
        get_relevant_memories, get_short_term_memory, search_memories,
        semantic_search_memories, summarize_session, update_memory,
    },
    messaging::{
        get_agent_messages, list_agent_subscriptions, publish_agent_message,
        subscribe_agent_topic, unsubscribe_agent_topic,
    },
    package::{
        get_ipc_protocol_range, install_agent, list_installed_agents, parse_agent_manifest,
        remove_agent, update_agent, validate_agent_manifest,
    },
    registry::{
        get_registry_server_port, search_agents, start_registry_server, stop_registry_server,
    },
    routing::{
        approve_remote_command, deny_remote_command, get_device_permissions,
        list_pending_commands, set_device_permission,
    },
    sandbox::{
        clear_agent_capabilities, grant_agent_capability, list_agent_capabilities,
        revoke_agent_capability, run_agent_in_sandbox,
    },
    window::{
        get_all_monitors, get_window_mode, set_cursor_passthrough, set_pet_mode_bounds,
        set_window_mode, toggle_window_mode,
    },
    streaming::send_message_stream,
    settings::{get_app_settings, save_app_settings, get_model_camera_positions, save_model_camera_position},
    voice::{
        clear_voice_config, get_voice_config, list_asr_providers, list_tts_providers,
        set_asr_provider, set_tts_provider, set_voice_api_key, set_voice_endpoint,
        synthesize_tts, transcribe_audio,
    },
};
use identity::{key_store::load_or_generate_identity, trusted_devices::load_trusted_devices};

pub struct AppState {
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
    /// Voice provider configuration (ASR/TTS selections).
    pub voice_config: Mutex<voice::VoiceConfig>,
    /// Provider rotation and rate-limit tracking for free API providers.
    pub provider_rotator: Mutex<brain::ProviderRotator>,
    /// Persistent application settings (model selection, camera state).
    pub app_settings: Mutex<settings::AppSettings>,
}

impl AppState {
    /// Create a new `AppState` bound to `data_dir`, which is used to persist
    /// settings (active brain model) and the long-term memory database.
    /// In production this is the Tauri app-data directory; for tests use
    /// [`AppState::for_test`] instead.
    fn new(data_dir: &std::path::Path) -> Self {
        let active_brain = brain::load_brain(data_dir);
        let brain_mode = brain::brain_config::load(data_dir);
        AppState {
            conversation: Mutex::new(Vec::new()),
            vrm_path: Mutex::new(None),
            device_identity: Mutex::new(None),
            trusted_devices: Mutex::new(Vec::new()),
            link_manager: TokioMutex::new(link::manager::LinkManager::new()),
            link_server_port: TokioMutex::new(None),
            command_router: TokioMutex::new(routing::CommandRouter::new("uninitialized")),
            package_installer: TokioMutex::new(package_manager::PackageInstaller::new(data_dir)),
            package_registry: TokioMutex::new(Box::new(package_manager::MockRegistry::new())),
            active_brain: Mutex::new(active_brain),
            brain_mode: Mutex::new(brain_mode),
            ollama_client: reqwest::Client::new(),
            data_dir: data_dir.to_path_buf(),
            memory_store: Mutex::new(memory::MemoryStore::new(data_dir)),
            registry_server_handle: TokioMutex::new(None),
            capability_store: TokioMutex::new(sandbox::CapabilityStore::new(data_dir)),
            message_bus: TokioMutex::new(messaging::MessageBus::new()),
            window_mode: Mutex::new(commands::window::WindowMode::default()),
            voice_config: Mutex::new(voice::config_store::load(data_dir)),
            provider_rotator: Mutex::new(brain::ProviderRotator::new()),
            app_settings: Mutex::new(settings::config_store::load(data_dir)),
        }
    }

    /// Convenience constructor for unit tests.
    #[cfg(test)]
    pub fn for_test() -> Self {
        AppState {
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
            ollama_client: reqwest::Client::new(),
            data_dir: std::path::PathBuf::from("."),
            memory_store: Mutex::new(memory::MemoryStore::in_memory()),
            registry_server_handle: TokioMutex::new(None),
            capability_store: TokioMutex::new(sandbox::CapabilityStore::in_memory()),
            message_bus: TokioMutex::new(messaging::MessageBus::new()),
            window_mode: Mutex::new(commands::window::WindowMode::default()),
            voice_config: Mutex::new(voice::VoiceConfig::default()),
            provider_rotator: Mutex::new(brain::ProviderRotator::new()),
            app_settings: Mutex::new(settings::AppSettings::default()),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            send_message,
            get_conversation,
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
            set_active_brain,
            get_active_brain,
            clear_active_brain,
            add_memory,
            get_memories,
            search_memories,
            update_memory,
            delete_memory,
            get_relevant_memories,
            get_short_term_memory,
            extract_memories_from_session,
            summarize_session,
            semantic_search_memories,
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
            send_message_stream,
            list_free_providers,
            get_brain_mode,
            set_brain_mode,
            health_check_providers,
            get_next_provider,
            list_asr_providers,
            list_tts_providers,
            get_voice_config,
            set_asr_provider,
            set_tts_provider,
            set_voice_api_key,
            set_voice_endpoint,
            clear_voice_config,
            synthesize_tts,
            transcribe_audio,
            get_app_settings,
            save_app_settings,
            get_model_camera_positions,
            save_model_camera_position,
        ])
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));

            app.manage(AppState::new(&data_dir));
            let state = app.state::<AppState>();

            let identity = load_or_generate_identity(&data_dir)
                .unwrap_or_else(|_| identity::DeviceIdentity::generate());
            let device_id = identity.device_id.clone();
            *state.device_identity.lock().unwrap() = Some(identity);

            let devices = load_trusted_devices(&data_dir);
            *state.trusted_devices.lock().unwrap() = devices;

            *state.command_router.blocking_lock() =
                routing::CommandRouter::new(&device_id);

            // System tray with Show/Hide + Window/Pet toggle + Quit
            let show_hide = MenuItem::with_id(app, "show_hide", "Show / Hide", true, None::<&str>)?;
            let mode_toggle = MenuItem::with_id(app, "mode_toggle", "Switch to Pet Mode", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_hide, &mode_toggle, &quit])?;

            TrayIconBuilder::new()
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
                            let new_mode = {
                                let current = state.window_mode.lock().unwrap();
                                match *current {
                                    commands::window::WindowMode::Window => commands::window::WindowMode::Pet,
                                    commands::window::WindowMode::Pet => commands::window::WindowMode::Window,
                                }
                            };
                            let _ = commands::window::apply_window_mode(&window, new_mode);
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

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
