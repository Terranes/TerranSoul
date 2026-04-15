//! Application settings — persistence, schema validation, and env overrides.
//!
//! Settings that are persisted between sessions:
//!   - `selected_model_id` — active 3D character model (e.g. "annabelle")
//!   - `camera_azimuth`    — horizontal orbit angle (radians)
//!   - `camera_distance`   — zoom / distance from origin
//!
//! Environment variable overrides (for dev/CI):
//!   - `TERRANSOUL_MODEL_ID`  — override `selected_model_id`
//!
//! Schema versioning: if the persisted `version` field does not match
//! `CURRENT_SCHEMA_VERSION`, the file is treated as corrupt/stale and the
//! default settings are returned. This prevents panics from outdated fields.

pub mod config_store;

use serde::{Deserialize, Serialize};

/// Current schema version. Bump when adding non-backward-compatible fields.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Default character model ID (must match `DEFAULT_MODEL_ID` in frontend).
pub const DEFAULT_MODEL_ID: &str = "annabelle";

/// Persisted application settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    /// Schema version — used for migration/corruption detection.
    #[serde(default = "default_version")]
    pub version: u32,

    /// ID of the selected character model (maps to DEFAULT_MODELS on the frontend).
    #[serde(default = "default_model_id")]
    pub selected_model_id: String,

    /// Camera horizontal orbit angle (radians). 0 = facing +Z.
    #[serde(default)]
    pub camera_azimuth: f32,

    /// Camera distance from the orbit target (zoom level).
    /// Clamped to [0.5, 5.0] in the scene renderer.
    #[serde(default = "default_camera_distance")]
    pub camera_distance: f32,
}

fn default_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

fn default_model_id() -> String {
    DEFAULT_MODEL_ID.to_string()
}

fn default_camera_distance() -> f32 {
    2.8 // CAMERA_Z_LANDSCAPE
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: DEFAULT_MODEL_ID.to_string(),
            camera_azimuth: 0.0,
            camera_distance: 2.8,
        }
    }
}

impl AppSettings {
    /// Apply environment variable overrides on top of persisted values.
    ///
    /// This is intentionally limited to non-sensitive fields only.
    /// Useful for dev/CI to force a specific model without user interaction.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(model_id) = std::env::var("TERRANSOUL_MODEL_ID") {
            let trimmed = model_id.trim().to_string();
            if !trimmed.is_empty() {
                self.selected_model_id = trimmed;
            }
        }
    }

    /// Returns true if the settings schema version matches the current version.
    /// Stale or corrupt settings should be replaced with defaults.
    pub fn is_valid_schema(&self) -> bool {
        self.version == CURRENT_SCHEMA_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_valid() {
        let s = AppSettings::default();
        assert_eq!(s.version, CURRENT_SCHEMA_VERSION);
        assert_eq!(s.selected_model_id, DEFAULT_MODEL_ID);
        assert_eq!(s.camera_azimuth, 0.0);
        assert!((s.camera_distance - 2.8).abs() < 0.001);
        assert!(s.is_valid_schema());
    }

    #[test]
    fn is_valid_schema_false_for_old_version() {
        let s = AppSettings {
            version: 0,
            ..AppSettings::default()
        };
        assert!(!s.is_valid_schema());
    }

    #[test]
    fn apply_env_overrides_sets_model_id() {
        std::env::set_var("TERRANSOUL_MODEL_ID", "m58");
        let mut s = AppSettings::default();
        s.apply_env_overrides();
        assert_eq!(s.selected_model_id, "m58");
        std::env::remove_var("TERRANSOUL_MODEL_ID");
    }

    #[test]
    fn apply_env_overrides_ignores_empty_model_id() {
        std::env::set_var("TERRANSOUL_MODEL_ID", "  ");
        let mut s = AppSettings::default();
        s.apply_env_overrides();
        assert_eq!(s.selected_model_id, DEFAULT_MODEL_ID);
        std::env::remove_var("TERRANSOUL_MODEL_ID");
    }

    #[test]
    fn apply_env_overrides_noop_when_unset() {
        std::env::remove_var("TERRANSOUL_MODEL_ID");
        let mut s = AppSettings::default();
        s.apply_env_overrides();
        assert_eq!(s.selected_model_id, DEFAULT_MODEL_ID);
    }

    #[test]
    fn roundtrip_serde() {
        let s = AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: "genshin".into(),
            camera_azimuth: 1.57,
            camera_distance: 3.2,
        };
        let json = serde_json::to_string(&s).unwrap();
        let parsed: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, s);
    }
}
