use serde::{Deserialize, Serialize};
use tauri::webview::Color;
#[cfg(target_os = "windows")]
use tauri::Emitter;
use tauri::{Manager, State, WebviewWindow};

use crate::AppState;
use std::sync::atomic::Ordering;

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
            let _ = window.set_resizable(true);
            // Restore opaque background for desktop mode
            let wv: &tauri::Webview = window.as_ref();
            let _ = wv.set_background_color(Some(Color(11, 17, 32, 255)));
        }
        WindowMode::Pet => {
            let _ = window.set_decorations(false);
            let _ = window.set_always_on_top(true);
            let _ = window.set_skip_taskbar(true);
            let _ = window.set_resizable(false);
            // Transparent background so the character floats on the desktop.
            // On Windows, alpha=0 makes the WebView2 background fully transparent,
            // allowing see-through and click-through on transparent pixels.
            let wv: &tauri::Webview = window.as_ref();
            let _ = wv.set_background_color(Some(Color(0, 0, 0, 0)));
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
pub async fn get_window_mode(state: State<'_, AppState>) -> Result<WindowMode, String> {
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

/// Toggle the WebView2 background alpha between fully-transparent (alpha=0)
/// and near-transparent (alpha=1).  On Windows, DWM performs per-pixel
/// hit-testing against the composited surface — a fully-transparent pixel
/// is treated as "not part of the window" even when
/// `set_ignore_cursor_events(false)` is active.  Setting alpha=1 ensures
/// the entire viewport is hittable while remaining visually transparent.
///
/// Call with `opaque=true` when a modal is open in pet mode, and
/// `opaque=false` when it closes to restore desktop click-through.
#[tauri::command]
pub async fn set_pet_modal_backdrop(
    opaque: bool,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = opaque;
        let _ = app_handle;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let window = app_handle
            .get_webview_window("main")
            .ok_or("Main window not found")?;
        let wv: &tauri::Webview = window.as_ref();
        // alpha=1 is visually invisible but makes DWM treat the pixel as
        // belonging to this window for hit-testing purposes.
        let alpha = if opaque { 1 } else { 0 };
        let _ = wv.set_background_color(Some(Color(0, 0, 0, alpha)));
        Ok(())
    }
}

/// Initiate a window drag operation from the frontend.
/// Call this from a mousedown handler so the OS takes over dragging.
#[tauri::command]
pub async fn start_window_drag(app_handle: tauri::AppHandle) -> Result<(), String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window.start_dragging().map_err(|e| e.to_string())
}

/// Set the pet-mode window to a compact size centred on its current position.
/// This gives the window a size that just fits the character model, instead
/// of spanning all monitors.
#[tauri::command]
pub async fn set_pet_window_size(
    width: u32,
    height: u32,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    let _ = window.set_size(tauri::PhysicalSize::new(width, height));
    Ok(())
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
pub async fn get_all_monitors(app_handle: tauri::AppHandle) -> Result<Vec<MonitorInfo>, String> {
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
pub async fn set_pet_mode_bounds(app_handle: tauri::AppHandle) -> Result<(), String> {
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

// ── Pet-mode cursor tracking (Windows) ────────────────────────────────────────
// Replicates Electron's `setIgnoreMouseEvents(true, { forward: true })` pattern
// A background task polls the OS cursor position and
// emits `pet-cursor-pos` events so the frontend can toggle `set_ignore_cursor_events`
// based on whether the cursor is over an opaque canvas pixel or a transparent area.

#[cfg(target_os = "windows")]
mod win_cursor {
    #[repr(C)]
    #[allow(clippy::upper_case_acronyms)]
    pub struct POINT {
        pub x: i32,
        pub y: i32,
    }

    extern "system" {
        pub fn GetCursorPos(lp_point: *mut POINT) -> i32;
    }

    /// Get the cursor's current screen position (physical pixels).
    pub fn screen_pos() -> Option<(i32, i32)> {
        unsafe {
            let mut pt = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pt) != 0 {
                Some((pt.x, pt.y))
            } else {
                None
            }
        }
    }
}

/// Start polling the OS cursor position and emitting `pet-cursor-pos` events
/// to the frontend (~33 Hz).  The frontend uses the coordinates to read the
/// WebGL canvas alpha and toggle `set_ignore_cursor_events` accordingly.
///
/// Only performs real work on Windows; other platforms no-op gracefully since
/// macOS natively supports transparent-pixel click-through.
#[tauri::command]
pub async fn start_pet_cursor_poll(
    _app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Prevent duplicate polling tasks.
    if state.pet_cursor_active.load(Ordering::SeqCst) {
        return Ok(());
    }
    state.pet_cursor_active.store(true, Ordering::SeqCst);

    #[cfg(target_os = "windows")]
    {
        let active = state.pet_cursor_active.clone();
        let handle = _app_handle.clone();

        tokio::spawn(async move {
            while active.load(Ordering::Relaxed) {
                if let Some(window) = handle.get_webview_window("main") {
                    if let (Some((cx, cy)), Ok(win_pos), Ok(win_size)) = (
                        win_cursor::screen_pos(),
                        window.outer_position(),
                        window.inner_size(),
                    ) {
                        let scale = window.scale_factor().unwrap_or(1.0);
                        let rx = cx - win_pos.x;
                        let ry = cy - win_pos.y;
                        let inside = rx >= 0
                            && ry >= 0
                            && (rx as u32) < win_size.width
                            && (ry as u32) < win_size.height;
                        // Send logical (CSS) coordinates so the frontend can
                        // directly use them for DOM bounds + canvas readPixels.
                        let lx = (rx as f64 / scale).round() as i32;
                        let ly = (ry as f64 / scale).round() as i32;
                        let _ = handle.emit(
                            "pet-cursor-pos",
                            serde_json::json!({ "x": lx, "y": ly, "inside": inside }),
                        );
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            }
        });
    }

    Ok(())
}

/// Stop the cursor-position polling loop started by `start_pet_cursor_poll`.
#[tauri::command]
pub async fn stop_pet_cursor_poll(state: State<'_, AppState>) -> Result<(), String> {
    state.pet_cursor_active.store(false, Ordering::SeqCst);
    Ok(())
}

/// Terminate the entire application process (used by pet-mode Exit).
/// Closes all windows gracefully first so WebView2 can unregister its
/// window classes, then exits.  This avoids the Chromium error:
///   "Failed to unregister class Chrome_WidgetWin_0. Error = 1412"
#[tauri::command]
pub async fn exit_app(app_handle: tauri::AppHandle) -> Result<(), String> {
    // Close all webview windows so WebView2 cleans up its HWNDs and
    // unregisters window classes before the process terminates.
    for (_, window) in app_handle.webview_windows() {
        let _ = window.close();
    }
    // Brief delay to let the window destruction finish.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    app_handle.exit(0);
    Ok(())
}

/// Returns `true` when running a dev/debug build, `false` for release.
///
/// The frontend uses this to display a "DEV" badge and connect to the
/// correct MCP port (7422 for dev, 7421 for release).
#[tauri::command]
pub fn is_dev_build() -> bool {
    cfg!(debug_assertions)
}

/// Returns `true` when this Tauri instance was launched in MCP mode
/// (`npm run mcp` / `terransoul --mcp-app`).
///
/// The frontend uses this to display an "MCP" badge instead of "DEV"
/// in pet/sidebar/mobile layouts so developers can tell at a glance
/// that they are talking to the repo-local MCP runtime.
#[tauri::command]
pub fn is_mcp_mode() -> bool {
    crate::ai_integrations::mcp::is_mcp_pet_mode()
}

// ── Pet-mode panel windows ───────────────────────────────────────────────────
// In pet mode, each tab (Brain, Memory, Quests, Market, Voice) opens as its
// own floating window instead of being part of the single app shell.

/// Allowed panel identifiers for pet-mode child windows.
const ALLOWED_PANELS: &[&str] = &["brain", "memory", "skills", "marketplace", "voice"];

/// Open a panel as a separate floating window in pet mode.
/// The window loads the same frontend URL with `?panel=<id>` so the
/// frontend can render only that panel without the navigation shell.
#[tauri::command]
pub async fn open_panel_window(panel: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    if !ALLOWED_PANELS.contains(&panel.as_str()) {
        return Err(format!("Unknown panel: {panel}"));
    }

    let label = format!("panel-{panel}");

    // If this panel window already exists, just focus it.
    if let Some(existing) = app_handle.get_webview_window(&label) {
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Resolve the frontend URL from the running app.
    let base_url = if cfg!(debug_assertions) {
        "http://localhost:1420".to_string()
    } else {
        // In production, Tauri serves from tauri://localhost
        "tauri://localhost".to_string()
    };
    let url = format!("{base_url}?panel={panel}");

    let title = match panel.as_str() {
        "brain" => "TerranSoul — Brain",
        "memory" => "TerranSoul — Memory",
        "skills" => "TerranSoul — Quests",
        "marketplace" => "TerranSoul — Marketplace",
        "voice" => "TerranSoul — Voice",
        _ => "TerranSoul",
    };

    tauri::WebviewWindowBuilder::new(
        &app_handle,
        &label,
        tauri::WebviewUrl::External(url.parse().map_err(|e: url::ParseError| e.to_string())?),
    )
    .title(title)
    .inner_size(480.0, 640.0)
    .resizable(true)
    .decorations(true)
    .always_on_top(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Close a panel window that was opened via `open_panel_window`.
#[tauri::command]
pub async fn close_panel_window(panel: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let label = format!("panel-{panel}");
    if let Some(window) = app_handle.get_webview_window(&label) {
        window.close().map_err(|e| e.to_string())?;
    }
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

    #[test]
    fn allowed_panels_contains_expected_ids() {
        for id in &["brain", "memory", "skills", "marketplace", "voice"] {
            assert!(ALLOWED_PANELS.contains(id), "Missing panel: {id}");
        }
    }

    #[test]
    fn allowed_panels_rejects_unknown() {
        assert!(!ALLOWED_PANELS.contains(&"chat"));
        assert!(!ALLOWED_PANELS.contains(&"unknown"));
        assert!(!ALLOWED_PANELS.contains(&""));
    }
}
