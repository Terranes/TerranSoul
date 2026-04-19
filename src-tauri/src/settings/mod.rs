//! Application settings — persistence, schema validation, and env overrides.
//!
//! Settings that are persisted between sessions:
//!   - `selected_model_id` — active 3D character model (e.g. "annabelle")
//!   - `camera_azimuth`    — horizontal orbit angle (radians)
//!   - `camera_distance`   — zoom / distance from origin
//!   - `bgm_enabled`       — whether background music is playing
//!   - `bgm_volume`        — background music volume (0.0–1.0)
//!   - `bgm_track_id`      — which ambient track is selected
//!
//! Environment variable overrides (for dev/CI):
//!   - `TERRANSOUL_MODEL_ID`  — override `selected_model_id`
//!
//! Schema versioning: if the persisted `version` field does not match
//! `CURRENT_SCHEMA_VERSION`, the file is treated as corrupt/stale and the
//! default settings are returned. This prevents panics from outdated fields.

pub mod config_store;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current schema version. Bump when adding non-backward-compatible fields.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// Per-model camera position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelCameraPosition {
    pub azimuth: f32,
    pub distance: f32,
}

/// Default character model ID (must match `DEFAULT_MODEL_ID` in frontend).
pub const DEFAULT_MODEL_ID: &str = "annabelle";

/// Default BGM volume (0.0–1.0).
pub const DEFAULT_BGM_VOLUME: f32 = 0.15;

/// Default BGM track.
pub const DEFAULT_BGM_TRACK_ID: &str = "prelude";

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

    /// Whether background music is enabled.
    #[serde(default)]
    pub bgm_enabled: bool,

    /// Background music volume (0.0–1.0).
    #[serde(default = "default_bgm_volume")]
    pub bgm_volume: f32,

    /// ID of the selected ambient track.
    #[serde(default = "default_bgm_track_id")]
    pub bgm_track_id: String,

    /// Per-model camera positions (keyed by model ID).
    #[serde(default)]
    pub model_camera_positions: HashMap<String, ModelCameraPosition>,
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

fn default_bgm_volume() -> f32 {
    DEFAULT_BGM_VOLUME
}

fn default_bgm_track_id() -> String {
    DEFAULT_BGM_TRACK_ID.to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: DEFAULT_MODEL_ID.to_string(),
            camera_azimuth: 0.0,
            camera_distance: 2.8,
            bgm_enabled: false,
            bgm_volume: DEFAULT_BGM_VOLUME,
            bgm_track_id: DEFAULT_BGM_TRACK_ID.to_string(),
            model_camera_positions: HashMap::new(),
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

/// Mutex shared across all test modules that mutate `TERRANSOUL_*` env vars.
/// `std::env::set_var` is process-global, so tests that touch env vars must
/// hold this lock to avoid data-races when Cargo runs tests in parallel.
#[cfg(test)]
pub(super) static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        let _lock = super::ENV_MUTEX.lock().unwrap();
        std::env::set_var("TERRANSOUL_MODEL_ID", "m58");
        let mut s = AppSettings::default();
        s.apply_env_overrides();
        std::env::remove_var("TERRANSOUL_MODEL_ID");
        assert_eq!(s.selected_model_id, "m58");
    }

    #[test]
    fn apply_env_overrides_ignores_empty_model_id() {
        let _lock = super::ENV_MUTEX.lock().unwrap();
        std::env::set_var("TERRANSOUL_MODEL_ID", "  ");
        let mut s = AppSettings::default();
        s.apply_env_overrides();
        std::env::remove_var("TERRANSOUL_MODEL_ID");
        assert_eq!(s.selected_model_id, DEFAULT_MODEL_ID);
    }

    #[test]
    fn apply_env_overrides_noop_when_unset() {
        let _lock = super::ENV_MUTEX.lock().unwrap();
        std::env::remove_var("TERRANSOUL_MODEL_ID");
        let mut s = AppSettings::default();
        s.apply_env_overrides();
        assert_eq!(s.selected_model_id, DEFAULT_MODEL_ID);
    }

    #[test]
    fn roundtrip_serde() {
        let mut positions = HashMap::new();
        positions.insert("annabelle".to_string(), ModelCameraPosition { azimuth: 0.5, distance: 3.0 });
        let s = AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: "genshin".into(),
            camera_azimuth: 1.57,
            camera_distance: 3.2,
            bgm_enabled: true,
            bgm_volume: 0.3,
            bgm_track_id: "moonflow".into(),
            model_camera_positions: positions,
        };
        let json = serde_json::to_string(&s).unwrap();
        let parsed: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, s);
    }

    #[test]
    fn default_bgm_settings() {
        let s = AppSettings::default();
        assert!(!s.bgm_enabled);
        assert!((s.bgm_volume - DEFAULT_BGM_VOLUME).abs() < 0.001);
        assert_eq!(s.bgm_track_id, DEFAULT_BGM_TRACK_ID);
    }

    #[test]
    fn serde_fills_bgm_defaults_when_missing() {
        // Simulate a JSON blob without the new BGM fields (forward compat)
        let json = r#"{"version":2,"selected_model_id":"annabelle","camera_azimuth":0,"camera_distance":2.8}"#;
        let parsed: AppSettings = serde_json::from_str(json).unwrap();
        assert!(!parsed.bgm_enabled);
        assert!((parsed.bgm_volume - DEFAULT_BGM_VOLUME).abs() < 0.001);
        assert_eq!(parsed.bgm_track_id, DEFAULT_BGM_TRACK_ID);
    }

    #[test]
    fn default_model_camera_positions_is_empty() {
        let s = AppSettings::default();
        assert!(s.model_camera_positions.is_empty());
    }

    #[test]
    fn model_camera_position_serde_roundtrip() {
        let pos = ModelCameraPosition { azimuth: 1.23, distance: 4.5 };
        let json = serde_json::to_string(&pos).unwrap();
        let parsed: ModelCameraPosition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, pos);
    }

    #[test]
    fn model_camera_positions_independent_per_model() {
        let mut s = AppSettings::default();
        s.model_camera_positions.insert("annabelle".into(), ModelCameraPosition { azimuth: 0.5, distance: 3.0 });
        s.model_camera_positions.insert("m58".into(), ModelCameraPosition { azimuth: 1.2, distance: 2.0 });

        assert_eq!(s.model_camera_positions.len(), 2);
        assert!((s.model_camera_positions["annabelle"].azimuth - 0.5).abs() < 0.001);
        assert!((s.model_camera_positions["m58"].distance - 2.0).abs() < 0.001);
    }

    #[test]
    fn serde_fills_model_camera_positions_default_when_missing() {
        // JSON without model_camera_positions field — should default to empty map
        let json = r#"{"version":2,"selected_model_id":"annabelle","camera_azimuth":0,"camera_distance":2.8,"bgm_enabled":false,"bgm_volume":0.15,"bgm_track_id":"prelude"}"#;
        let parsed: AppSettings = serde_json::from_str(json).unwrap();
        assert!(parsed.model_camera_positions.is_empty());
    }
}
