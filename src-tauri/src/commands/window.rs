use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn close_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.close();
    }
}

#[tauri::command]
pub fn start_drag(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.start_dragging();
    }
}

#[derive(serde::Serialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub screen_width: u32,
    pub screen_height: u32,
}

#[tauri::command]
pub fn get_window_state(app: AppHandle) -> Option<WindowState> {
    let window = app.get_webview_window("main")?;
    let pos = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    let monitor = window.current_monitor().ok()??;
    let screen = monitor.size();
    Some(WindowState {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
        screen_width: screen.width,
        screen_height: screen.height,
    })
}

#[tauri::command]
pub fn move_window(app: AppHandle, x: i32, y: i32) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }
}
