use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;
use tokio::sync::Mutex as TokioMutex;

pub mod agent;
pub mod commands;
pub mod identity;
pub mod link;
pub mod orchestrator;
pub mod package_manager;
pub mod routing;
pub mod sync;

use commands::{
    agent::list_agents,
    character::load_vrm,
    chat::{get_conversation, send_message},
    identity::{
        add_trusted_device_cmd, get_device_identity, get_pairing_qr, list_trusted_devices,
        remove_trusted_device_cmd,
    },
    link::{connect_to_peer, disconnect_link, get_link_status, start_link_server},
    package::{
        get_ipc_protocol_range, install_agent, list_installed_agents, parse_agent_manifest,
        remove_agent, update_agent, validate_agent_manifest,
    },
    routing::{
        approve_remote_command, deny_remote_command, get_device_permissions,
        list_pending_commands, set_device_permission,
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
    pub package_registry: TokioMutex<package_manager::MockRegistry>,
}

impl AppState {
    fn new(data_dir: &std::path::Path) -> Self {
        AppState {
            conversation: Mutex::new(Vec::new()),
            vrm_path: Mutex::new(None),
            device_identity: Mutex::new(None),
            trusted_devices: Mutex::new(Vec::new()),
            link_manager: TokioMutex::new(link::manager::LinkManager::new()),
            link_server_port: TokioMutex::new(None),
            command_router: TokioMutex::new(routing::CommandRouter::new("uninitialized")),
            package_installer: TokioMutex::new(package_manager::PackageInstaller::new(data_dir)),
            package_registry: TokioMutex::new(package_manager::MockRegistry::new()),
        }
    }

    /// Convenience constructor for unit tests that only exercise chat/character logic.
    #[cfg(test)]
    pub fn for_test() -> Self {
        Self::new(std::path::Path::new("."))
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

            // Initialize the command router with this device's identity.
            // We use blocking_lock since we're in synchronous setup code.
            *state.command_router.blocking_lock() =
                routing::CommandRouter::new(&device_id);

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
