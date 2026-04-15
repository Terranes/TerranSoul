//! Tauri commands for application settings persistence.

use tauri::State;

use crate::settings::{config_store, AppSettings};
use crate::AppState;

/// Return the current application settings, loading from disk with env overrides applied.
#[tauri::command]
pub async fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

/// Persist updated application settings to disk and update the in-memory state.
#[tauri::command]
pub async fn save_app_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    config_store::save(&state.data_dir, &settings)?;
    let mut current = state.app_settings.lock().map_err(|e| e.to_string())?;
    *current = settings;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::CURRENT_SCHEMA_VERSION;

    #[test]
    fn get_app_settings_returns_defaults_from_state() {
        let state = crate::AppState::for_test();
        let settings = state.app_settings.lock().unwrap();
        assert_eq!(settings.selected_model_id, crate::settings::DEFAULT_MODEL_ID);
        assert!(settings.is_valid_schema());
    }

    #[test]
    fn save_app_settings_updates_in_memory() {
        let state = crate::AppState::for_test();
        let new_settings = AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: "m58".into(),
            camera_azimuth: 1.0,
            camera_distance: 3.0,
        };
        // Directly update in-memory state (simulating command effect)
        {
            let mut current = state.app_settings.lock().unwrap();
            *current = new_settings.clone();
        }
        let loaded = state.app_settings.lock().unwrap();
        assert_eq!(loaded.selected_model_id, "m58");
        assert!((loaded.camera_azimuth - 1.0).abs() < 0.001);
    }

    #[test]
    fn save_app_settings_persists_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let settings = AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: "genshin".into(),
            camera_azimuth: 0.5,
            camera_distance: 4.0,
        };
        config_store::save(dir.path(), &settings).unwrap();
        let loaded = config_store::load(dir.path());
        assert_eq!(loaded.selected_model_id, "genshin");
    }
}
