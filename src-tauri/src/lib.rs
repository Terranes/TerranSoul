use std::sync::Mutex;
use tauri::Manager;

pub mod agent;
pub mod commands;
pub mod orchestrator;

use commands::{
    agent::list_agents,
    character::load_vrm,
    chat::{get_conversation, send_message},
};

pub struct AppState {
    pub conversation: Mutex<Vec<commands::chat::Message>>,
    pub vrm_path: Mutex<Option<String>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            conversation: Mutex::new(Vec::new()),
            vrm_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            send_message,
            get_conversation,
            list_agents,
            load_vrm,
        ])
        .setup(|app| {
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
