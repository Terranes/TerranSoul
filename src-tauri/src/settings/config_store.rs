use std::fs;
use std::path::{Path, PathBuf};

use super::AppSettings;

/// File name used to store application settings.
const SETTINGS_FILE: &str = "app_settings.json";

/// Resolve the effective data directory, applying the user's `data_root`
/// override from settings when present.
pub fn resolve_effective_data_dir(default_dir: &Path) -> PathBuf {
    let settings = load(default_dir);
    match settings.data_root.as_deref() {
        Some(root) if !root.is_empty() => PathBuf::from(root),
        _ => default_dir.to_path_buf(),
    }
}

/// Load application settings from disk, applying env overrides.
///
/// - Returns defaults if no settings file exists.
/// - Returns defaults (and removes the file) if the schema version is outdated.
/// - Returns defaults if the file contains invalid JSON.
/// - Applies `TERRANSOUL_*` env overrides on top of whatever was loaded.
pub fn load(data_dir: &Path) -> AppSettings {
    let path = data_dir.join(SETTINGS_FILE);

    let mut settings = if path.exists() {
        match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<AppSettings>(&contents) {
                Ok(s) if s.is_valid_schema() => s,
                Ok(_stale) => {
                    // Schema version mismatch — wipe and start fresh.
                    let _ = fs::remove_file(&path);
                    AppSettings::default()
                }
                Err(_corrupt) => {
                    // Corrupt JSON — wipe and start fresh.
                    let _ = fs::remove_file(&path);
                    AppSettings::default()
                }
            },
            Err(_) => AppSettings::default(),
        }
    } else {
        AppSettings::default()
    };

    settings.apply_env_overrides();
    settings
}

/// Persist application settings to disk.
pub fn save(data_dir: &Path, settings: &AppSettings) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("create dir: {e}"))?;
    let path = data_dir.join(SETTINGS_FILE);
    let json = serde_json::to_string_pretty(settings).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write settings: {e}"))
}

#[cfg(test)]
mod tests {
    use super::super::CURRENT_SCHEMA_VERSION;
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_returns_default_when_no_file() {
        let _lock = super::super::ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let s = load(dir.path());
        assert_eq!(s.selected_model_id, super::super::DEFAULT_MODEL_ID);
        assert!(s.is_valid_schema());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let _lock = super::super::ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let s = AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: "komori".into(),
            camera_azimuth: 0.78,
            camera_distance: 3.5,
            bgm_enabled: true,
            bgm_volume: 0.3,
            bgm_track_id: "moonflow".into(),
            model_camera_positions: std::collections::HashMap::new(),
            user_models: Vec::new(),
            preferred_container_runtime: crate::container::RuntimePreference::Auto,
            auto_learn_policy: crate::memory::AutoLearnPolicy::default(),
            relevance_threshold: crate::settings::DEFAULT_RELEVANCE_THRESHOLD,
            auto_tag: false,
            contextual_retrieval: false,
            late_chunking: false,
            web_search_enabled: false,
            lan_enabled: false,
            lan_auth_mode: crate::settings::LanAuthMode::TokenRequired,
            mobile_notifications_enabled: true,
            mobile_notification_threshold_ms:
                crate::settings::DEFAULT_MOBILE_NOTIFICATION_THRESHOLD_MS,
            mobile_notification_poll_ms: crate::settings::DEFAULT_MOBILE_NOTIFICATION_POLL_MS,
            auto_extract_edges: true,
            enable_kg_boost: false,
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
            max_memory_gb: crate::settings::DEFAULT_MAX_MEMORY_GB,
            max_memory_mb: crate::settings::DEFAULT_MAX_MEMORY_MB,
            max_long_term_entries: crate::memory::eviction::DEFAULT_MAX_LONG_TERM,
            data_root: None,
            hive_url: None,
            obsidian_layout: crate::settings::ObsidianLayout::Flat,
            sqlite_cache_mb: crate::settings::DEFAULT_SQLITE_CACHE_MB,
            sqlite_mmap_mb: crate::settings::DEFAULT_SQLITE_MMAP_MB,
            code_index_cache_mb: crate::settings::DEFAULT_CODE_INDEX_CACHE_MB,
            code_index_mmap_mb: crate::settings::DEFAULT_CODE_INDEX_MMAP_MB,
            context_folders: Vec::new(),
            scholar_crawl_enabled: false,
            scholar_crawl_max_depth: crate::settings::DEFAULT_SCHOLAR_CRAWL_MAX_DEPTH,
            scholar_crawl_max_pages: crate::settings::DEFAULT_SCHOLAR_CRAWL_MAX_PAGES,
            reasoning_effort: crate::settings::ReasoningEffort::Off,
            debug_logging: false,
        };
        save(dir.path(), &s).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded.selected_model_id, "komori");
        assert!((loaded.camera_azimuth - 0.78).abs() < 0.001);
        assert!((loaded.camera_distance - 3.5).abs() < 0.001);
        assert!(loaded.bgm_enabled);
        assert!((loaded.bgm_volume - 0.3).abs() < 0.001);
        assert_eq!(loaded.bgm_track_id, "moonflow");
    }

    #[test]
    fn load_wipes_corrupt_json() {
        let _lock = super::super::ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("app_settings.json");
        fs::write(&path, "{not valid json").unwrap();
        let s = load(dir.path());
        // Corrupt file replaced with defaults
        assert_eq!(s.selected_model_id, super::super::DEFAULT_MODEL_ID);
        // Corrupt file should be removed
        assert!(!path.exists());
    }

    #[test]
    fn load_wipes_stale_schema() {
        let _lock = super::super::ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let stale = AppSettings {
            version: 0, // old schema version
            selected_model_id: "old-model".into(),
            camera_azimuth: 0.0,
            camera_distance: 2.8,
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
            late_chunking: false,
            web_search_enabled: false,
            lan_enabled: false,
            lan_auth_mode: crate::settings::LanAuthMode::TokenRequired,
            mobile_notifications_enabled: true,
            mobile_notification_threshold_ms:
                crate::settings::DEFAULT_MOBILE_NOTIFICATION_THRESHOLD_MS,
            mobile_notification_poll_ms: crate::settings::DEFAULT_MOBILE_NOTIFICATION_POLL_MS,
            auto_extract_edges: true,
            enable_kg_boost: false,
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
            max_memory_gb: crate::settings::DEFAULT_MAX_MEMORY_GB,
            max_memory_mb: crate::settings::DEFAULT_MAX_MEMORY_MB,
            max_long_term_entries: crate::memory::eviction::DEFAULT_MAX_LONG_TERM,
            data_root: None,
            hive_url: None,
            obsidian_layout: crate::settings::ObsidianLayout::Flat,
            sqlite_cache_mb: crate::settings::DEFAULT_SQLITE_CACHE_MB,
            sqlite_mmap_mb: crate::settings::DEFAULT_SQLITE_MMAP_MB,
            code_index_cache_mb: crate::settings::DEFAULT_CODE_INDEX_CACHE_MB,
            code_index_mmap_mb: crate::settings::DEFAULT_CODE_INDEX_MMAP_MB,
            context_folders: Vec::new(),
            scholar_crawl_enabled: false,
            scholar_crawl_max_depth: crate::settings::DEFAULT_SCHOLAR_CRAWL_MAX_DEPTH,
            scholar_crawl_max_pages: crate::settings::DEFAULT_SCHOLAR_CRAWL_MAX_PAGES,
            reasoning_effort: crate::settings::ReasoningEffort::Off,
            debug_logging: false,
        };
        let json = serde_json::to_string(&stale).unwrap();
        fs::write(dir.path().join("app_settings.json"), json).unwrap();

        let s = load(dir.path());
        // Should return defaults, not the stale data
        assert_eq!(s.selected_model_id, super::super::DEFAULT_MODEL_ID);
    }

    #[test]
    fn load_applies_env_override() {
        let _lock = super::super::ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let s = AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: "shinra".into(),
            camera_azimuth: 0.0,
            camera_distance: 2.8,
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
            late_chunking: false,
            web_search_enabled: false,
            lan_enabled: false,
            lan_auth_mode: crate::settings::LanAuthMode::TokenRequired,
            mobile_notifications_enabled: true,
            mobile_notification_threshold_ms:
                crate::settings::DEFAULT_MOBILE_NOTIFICATION_THRESHOLD_MS,
            mobile_notification_poll_ms: crate::settings::DEFAULT_MOBILE_NOTIFICATION_POLL_MS,
            auto_extract_edges: true,
            enable_kg_boost: false,
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
            max_memory_gb: crate::settings::DEFAULT_MAX_MEMORY_GB,
            max_memory_mb: crate::settings::DEFAULT_MAX_MEMORY_MB,
            max_long_term_entries: crate::memory::eviction::DEFAULT_MAX_LONG_TERM,
            data_root: None,
            hive_url: None,
            obsidian_layout: crate::settings::ObsidianLayout::Flat,
            sqlite_cache_mb: crate::settings::DEFAULT_SQLITE_CACHE_MB,
            sqlite_mmap_mb: crate::settings::DEFAULT_SQLITE_MMAP_MB,
            code_index_cache_mb: crate::settings::DEFAULT_CODE_INDEX_CACHE_MB,
            code_index_mmap_mb: crate::settings::DEFAULT_CODE_INDEX_MMAP_MB,
            context_folders: Vec::new(),
            scholar_crawl_enabled: false,
            scholar_crawl_max_depth: crate::settings::DEFAULT_SCHOLAR_CRAWL_MAX_DEPTH,
            scholar_crawl_max_pages: crate::settings::DEFAULT_SCHOLAR_CRAWL_MAX_PAGES,
            reasoning_effort: crate::settings::ReasoningEffort::Off,
            debug_logging: false,
        };
        save(dir.path(), &s).unwrap();

        std::env::set_var("TERRANSOUL_MODEL_ID", "komori");
        let loaded = load(dir.path());
        std::env::remove_var("TERRANSOUL_MODEL_ID");

        assert_eq!(loaded.selected_model_id, "komori");
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c");
        let s = AppSettings::default();
        save(&nested, &s).unwrap();
        assert!(nested.join("app_settings.json").exists());
    }

    #[test]
    fn save_and_load_model_camera_positions() {
        let _lock = super::super::ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let mut positions = std::collections::HashMap::new();
        positions.insert(
            "shinra".to_string(),
            super::super::ModelCameraPosition {
                azimuth: 0.5,
                distance: 3.0,
            },
        );
        positions.insert(
            "komori".to_string(),
            super::super::ModelCameraPosition {
                azimuth: 1.2,
                distance: 2.5,
            },
        );
        let s = AppSettings {
            model_camera_positions: positions,
            user_models: Vec::new(),
            preferred_container_runtime: crate::container::RuntimePreference::Auto,
            ..AppSettings::default()
        };
        save(dir.path(), &s).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded.model_camera_positions.len(), 2);
        let ao = loaded.model_camera_positions.get("shinra").unwrap();
        assert!((ao.azimuth - 0.5).abs() < 0.001);
        assert!((ao.distance - 3.0).abs() < 0.001);
        let karina = loaded.model_camera_positions.get("komori").unwrap();
        assert!((karina.azimuth - 1.2).abs() < 0.001);
        assert!((karina.distance - 2.5).abs() < 0.001);
    }
}
