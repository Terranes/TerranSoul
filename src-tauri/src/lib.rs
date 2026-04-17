use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tokio::sync::Mutex as TokioMutex;

pub mod agent;
pub mod brain;
pub mod commands;
pub mod identity;
pub mod link;
pub mod memory;
pub mod orchestrator;
pub mod package_manager;
pub mod registry_server;
pub mod routing;
pub mod sandbox;
pub mod sync;

use commands::{
    agent::list_agents,
    window::{close_window, start_drag, get_window_state, move_window},
    brain::{
        check_ollama_status, clear_active_brain, get_active_brain, get_ollama_models,
        get_system_info, pull_ollama_model, recommend_brain_models, set_active_brain,
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
}

impl AppState {
    /// Create a new `AppState` bound to `data_dir`, which is used to persist
    /// settings (active brain model) and the long-term memory database.
    /// In production this is the Tauri app-data directory; for tests use
    /// [`AppState::for_test`] instead.
    fn new(data_dir: &std::path::Path) -> Self {
        let active_brain = brain::load_brain(data_dir);
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
            ollama_client: reqwest::Client::new(),
            data_dir: data_dir.to_path_buf(),
            memory_store: Mutex::new(memory::MemoryStore::new(data_dir)),
            registry_server_handle: TokioMutex::new(None),
            capability_store: TokioMutex::new(sandbox::CapabilityStore::new(data_dir)),
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
            ollama_client: reqwest::Client::new(),
            data_dir: std::path::PathBuf::from("."),
            memory_store: Mutex::new(memory::MemoryStore::in_memory()),
            registry_server_handle: TokioMutex::new(None),
            capability_store: TokioMutex::new(sandbox::CapabilityStore::in_memory()),
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
            close_window,
            start_drag,
            get_window_state,
            move_window,
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

            // System tray with Show/Hide + Quit
            let show_hide = MenuItem::with_id(app, "show_hide", "Show / Hide", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_hide, &quit])?;

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
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button, .. } = event {
                        if button == tauri::tray::MouseButton::Left {
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
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
