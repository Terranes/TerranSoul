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

/// Return per-model camera positions from the current settings.
#[tauri::command]
pub async fn get_model_camera_positions(
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, crate::settings::ModelCameraPosition>, String> {
    let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.model_camera_positions.clone())
}

/// Save a camera position for a specific model and persist to disk.
#[tauri::command(rename_all = "camelCase")]
pub async fn save_model_camera_position(
    model_id: String,
    azimuth: f32,
    distance: f32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    settings.model_camera_positions.insert(
        model_id,
        crate::settings::ModelCameraPosition { azimuth, distance },
    );
    config_store::save(&state.data_dir, &settings)?;
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
        assert_eq!(
            settings.selected_model_id,
            crate::settings::DEFAULT_MODEL_ID
        );
        assert!(settings.is_valid_schema());
    }

    #[test]
    fn save_app_settings_updates_in_memory() {
        let state = crate::AppState::for_test();
        let new_settings = AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: "komori".into(),
            camera_azimuth: 1.0,
            camera_distance: 3.0,
            bgm_enabled: true,
            bgm_volume: 0.25,
            bgm_track_id: "sanctuary".into(),
            model_camera_positions: std::collections::HashMap::new(),
            user_models: Vec::new(),
            preferred_container_runtime: crate::container::RuntimePreference::Auto,
            auto_learn_policy: crate::memory::AutoLearnPolicy::default(),
            relevance_threshold: crate::settings::DEFAULT_RELEVANCE_THRESHOLD,
            auto_tag: false,
            contextual_retrieval: false,
            web_search_enabled: false,
            auto_extract_edges: true,
            expanded_blendshapes: false,
            first_launch_complete: false,
            chatbox_mode: false,
            auto_configured: Vec::new(),
            prefer_local_brain: true,
            dismissed_model_updates: Vec::new(),
            last_update_check_date: String::new(),
            background_maintenance_enabled: true,
            maintenance_interval_hours: crate::settings::DEFAULT_MAINTENANCE_INTERVAL_HOURS,
            maintenance_idle_minimum_minutes: 0,
        };
        // Directly update in-memory state (simulating command effect)
        {
            let mut current = state.app_settings.lock().unwrap();
            *current = new_settings.clone();
        }
        let loaded = state.app_settings.lock().unwrap();
        assert_eq!(loaded.selected_model_id, "komori");
        assert!((loaded.camera_azimuth - 1.0).abs() < 0.001);
        assert!(loaded.bgm_enabled);
        assert!((loaded.bgm_volume - 0.25).abs() < 0.001);
        assert_eq!(loaded.bgm_track_id, "sanctuary");
    }

    #[test]
    fn save_app_settings_persists_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let settings = AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: "komori".into(),
            camera_azimuth: 0.5,
            camera_distance: 4.0,
            bgm_enabled: false,
            bgm_volume: 0.15,
            bgm_track_id: "prelude".into(),
            model_camera_positions: std::collections::HashMap::new(),
            user_models: Vec::new(),
            preferred_container_runtime: crate::container::RuntimePreference::Auto,
            auto_learn_policy: crate::memory::AutoLearnPolicy::default(),
            relevance_threshold: crate::settings::DEFAULT_RELEVANCE_THRESHOLD,
            auto_tag: false,
            contextual_retrieval: false,
            web_search_enabled: false,
            auto_extract_edges: true,
            expanded_blendshapes: false,
            first_launch_complete: false,
            chatbox_mode: false,
            auto_configured: Vec::new(),
            prefer_local_brain: true,
            dismissed_model_updates: Vec::new(),
            last_update_check_date: String::new(),
            background_maintenance_enabled: true,
            maintenance_interval_hours: crate::settings::DEFAULT_MAINTENANCE_INTERVAL_HOURS,
            maintenance_idle_minimum_minutes: 0,
        };
        config_store::save(dir.path(), &settings).unwrap();
        let loaded = config_store::load(dir.path());
        assert_eq!(loaded.selected_model_id, "komori");
    }

    #[test]
    fn save_model_camera_position_updates_in_memory() {
        let state = crate::AppState::for_test();
        {
            let mut settings = state.app_settings.lock().unwrap();
            settings.model_camera_positions.insert(
                "shinra".into(),
                crate::settings::ModelCameraPosition {
                    azimuth: 0.5,
                    distance: 3.0,
                },
            );
        }
        let settings = state.app_settings.lock().unwrap();
        let pos = settings.model_camera_positions.get("shinra").unwrap();
        assert!((pos.azimuth - 0.5).abs() < 0.001);
        assert!((pos.distance - 3.0).abs() < 0.001);
    }

    #[test]
    fn model_camera_positions_are_independent() {
        let state = crate::AppState::for_test();
        {
            let mut settings = state.app_settings.lock().unwrap();
            settings.model_camera_positions.insert(
                "shinra".into(),
                crate::settings::ModelCameraPosition {
                    azimuth: 0.5,
                    distance: 3.0,
                },
            );
            settings.model_camera_positions.insert(
                "komori".into(),
                crate::settings::ModelCameraPosition {
                    azimuth: 1.2,
                    distance: 2.0,
                },
            );
        }
        let settings = state.app_settings.lock().unwrap();
        assert_eq!(settings.model_camera_positions.len(), 2);
        assert!((settings.model_camera_positions["shinra"].azimuth - 0.5).abs() < 0.001);
        assert!((settings.model_camera_positions["komori"].azimuth - 1.2).abs() < 0.001);
    }
}
