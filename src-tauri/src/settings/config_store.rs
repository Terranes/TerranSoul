use std::fs;
use std::path::Path;

use super::{AppSettings, CURRENT_SCHEMA_VERSION};

/// File name used to store application settings.
const SETTINGS_FILE: &str = "app_settings.json";

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
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_returns_default_when_no_file() {
        let dir = tempdir().unwrap();
        let s = load(dir.path());
        assert_eq!(s.selected_model_id, super::super::DEFAULT_MODEL_ID);
        assert!(s.is_valid_schema());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let s = AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: "m58".into(),
            camera_azimuth: 0.78,
            camera_distance: 3.5,
        };
        save(dir.path(), &s).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded.selected_model_id, "m58");
        assert!((loaded.camera_azimuth - 0.78).abs() < 0.001);
        assert!((loaded.camera_distance - 3.5).abs() < 0.001);
    }

    #[test]
    fn load_wipes_corrupt_json() {
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
        let dir = tempdir().unwrap();
        let stale = AppSettings {
            version: 0, // old schema version
            selected_model_id: "old-model".into(),
            camera_azimuth: 0.0,
            camera_distance: 2.8,
        };
        let json = serde_json::to_string(&stale).unwrap();
        fs::write(dir.path().join("app_settings.json"), json).unwrap();

        let s = load(dir.path());
        // Should return defaults, not the stale data
        assert_eq!(s.selected_model_id, super::super::DEFAULT_MODEL_ID);
    }

    #[test]
    fn load_applies_env_override() {
        let dir = tempdir().unwrap();
        let s = AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: "annabelle".into(),
            camera_azimuth: 0.0,
            camera_distance: 2.8,
        };
        save(dir.path(), &s).unwrap();

        std::env::set_var("TERRANSOUL_MODEL_ID", "genshin");
        let loaded = load(dir.path());
        std::env::remove_var("TERRANSOUL_MODEL_ID");

        assert_eq!(loaded.selected_model_id, "genshin");
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c");
        let s = AppSettings::default();
        save(&nested, &s).unwrap();
        assert!(nested.join("app_settings.json").exists());
    }
}
