use serde::{Deserialize, Serialize};
use tauri::{Manager, State, WebviewWindow};

use crate::AppState;

/// Window mode: either a normal decorated window or a pet-mode overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WindowMode {
    /// Normal desktop window with decorations, taskbar entry, resizable.
    #[default]
    Window,
    /// Pet-mode overlay: transparent, always-on-top, skip-taskbar, no decorations.
    Pet,
}

/// Apply window mode properties to the given Tauri window.
pub fn apply_window_mode(window: &WebviewWindow, mode: WindowMode) -> Result<(), String> {
    match mode {
        WindowMode::Window => {
            let _ = window.set_decorations(true);
            let _ = window.set_always_on_top(false);
            let _ = window.set_skip_taskbar(false);
            // Restore a reasonable size for window mode
            let _ = window.set_resizable(true);
        }
        WindowMode::Pet => {
            let _ = window.set_decorations(false);
            let _ = window.set_always_on_top(true);
            let _ = window.set_skip_taskbar(true);
            let _ = window.set_resizable(true);
        }
    }
    Ok(())
}

/// Default window size applied when leaving pet mode without a saved size.
/// Matches the value in `tauri.conf.json` so a fresh Pet → Window transition
/// always looks the same as first launch.
const DEFAULT_WINDOW_WIDTH: u32 = 420;
const DEFAULT_WINDOW_HEIGHT: u32 = 700;

/// Save the window's current inner size so it can be restored on the next
/// Pet → Window transition.  Silently no-ops on error — losing the size
/// just means falling back to the default dimensions on restore.
fn save_window_size(window: &WebviewWindow, state: &State<'_, AppState>) {
    if let Ok(size) = window.inner_size() {
        if let Ok(mut slot) = state.saved_window_size.lock() {
            *slot = Some((size.width, size.height));
        }
    }
}

/// Restore the window to its previously-saved size, or to the configured
/// default if nothing has been saved.  Used on Pet → Window transitions so
/// the desktop window never stays stretched from a pet-mode span-all-monitors.
fn restore_window_size(window: &WebviewWindow, state: &State<'_, AppState>) {
    let (w, h) = state
        .saved_window_size
        .lock()
        .ok()
        .and_then(|slot| *slot)
        .unwrap_or((DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT));
    let _ = window.set_size(tauri::PhysicalSize::new(w, h));
    // Also re-centre on the active monitor so the window doesn't land
    // off-screen after the resize.
    if let Ok(Some(monitor)) = window.current_monitor() {
        let screen = monitor.size();
        let pos = monitor.position();
        let cx = pos.x + ((screen.width as i32 - w as i32) / 2).max(0);
        let cy = pos.y + ((screen.height as i32 - h as i32) / 2).max(0);
        let _ = window.set_position(tauri::PhysicalPosition::new(cx, cy));
    }
}

/// Set the window mode (window or pet).
#[tauri::command]
pub async fn set_window_mode(
    mode: WindowMode,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    let previous = {
        let current = state.window_mode.lock().map_err(|e| e.to_string())?;
        *current
    };
    // Save the desktop size BEFORE flipping to pet mode so we can restore it.
    if previous == WindowMode::Window && mode == WindowMode::Pet {
        save_window_size(&window, &state);
    }
    apply_window_mode(&window, mode)?;
    // Restore the saved desktop size AFTER flipping back from pet mode.
    if previous == WindowMode::Pet && mode == WindowMode::Window {
        restore_window_size(&window, &state);
    }
    let mut current = state.window_mode.lock().map_err(|e| e.to_string())?;
    *current = mode;
    Ok(())
}

/// Get the current window mode.
#[tauri::command]
pub async fn get_window_mode(
    state: State<'_, AppState>,
) -> Result<WindowMode, String> {
    let mode = state.window_mode.lock().map_err(|e| e.to_string())?;
    Ok(*mode)
}

/// Toggle between window and pet mode.
#[tauri::command]
pub async fn toggle_window_mode(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<WindowMode, String> {
    let previous = {
        let current = state.window_mode.lock().map_err(|e| e.to_string())?;
        *current
    };
    let new_mode = match previous {
        WindowMode::Window => WindowMode::Pet,
        WindowMode::Pet => WindowMode::Window,
    };
    let window = app_handle
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    if previous == WindowMode::Window && new_mode == WindowMode::Pet {
        save_window_size(&window, &state);
    }
    apply_window_mode(&window, new_mode)?;
    if previous == WindowMode::Pet && new_mode == WindowMode::Window {
        restore_window_size(&window, &state);
    }
    let mut current = state.window_mode.lock().map_err(|e| e.to_string())?;
    *current = new_mode;
    Ok(new_mode)
}

/// Set whether the window should ignore cursor events (click-through).
/// In pet mode, this allows clicks to pass through transparent areas.
#[tauri::command]
pub async fn set_cursor_passthrough(
    ignore: bool,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window
        .set_ignore_cursor_events(ignore)
        .map_err(|e| e.to_string())
}

/// Return information about all connected monitors.
#[derive(Debug, Clone, Serialize)]
pub struct MonitorInfo {
    pub name: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

/// Get all available monitors and their positions/sizes.
#[tauri::command]
pub async fn get_all_monitors(
    app_handle: tauri::AppHandle,
) -> Result<Vec<MonitorInfo>, String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    Ok(monitors
        .iter()
        .map(|m| {
            let pos = m.position();
            let size = m.size();
            MonitorInfo {
                name: m.name().map(|s| s.to_string()),
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
                scale_factor: m.scale_factor(),
            }
        })
        .collect())
}

/// Set the window bounds to span all monitors (for pet mode).
#[tauri::command]
pub async fn set_pet_mode_bounds(
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;

    if monitors.is_empty() {
        return Err("No monitors found".to_string());
    }

    // Calculate the bounding rectangle that spans all monitors
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for m in &monitors {
        let pos = m.position();
        let size = m.size();
        min_x = min_x.min(pos.x);
        min_y = min_y.min(pos.y);
        max_x = max_x.max(pos.x + size.width as i32);
        max_y = max_y.max(pos.y + size.height as i32);
    }

    let total_width = (max_x - min_x) as u32;
    let total_height = (max_y - min_y) as u32;

    let _ = window.set_position(tauri::PhysicalPosition::new(min_x, min_y));
    let _ = window.set_size(tauri::PhysicalSize::new(total_width, total_height));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_mode_default_is_window() {
        assert_eq!(WindowMode::default(), WindowMode::Window);
    }

    #[test]
    fn window_mode_serializes_snake_case() {
        let json = serde_json::to_string(&WindowMode::Window).unwrap();
        assert_eq!(json, r#""window""#);
        let json = serde_json::to_string(&WindowMode::Pet).unwrap();
        assert_eq!(json, r#""pet""#);
    }

    #[test]
    fn window_mode_deserializes_snake_case() {
        let mode: WindowMode = serde_json::from_str(r#""window""#).unwrap();
        assert_eq!(mode, WindowMode::Window);
        let mode: WindowMode = serde_json::from_str(r#""pet""#).unwrap();
        assert_eq!(mode, WindowMode::Pet);
    }

    #[test]
    fn window_mode_roundtrip() {
        for mode in [WindowMode::Window, WindowMode::Pet] {
            let json = serde_json::to_string(&mode).unwrap();
            let parsed: WindowMode = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, mode);
        }
    }
}
