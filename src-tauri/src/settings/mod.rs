//! Application settings — persistence, schema validation, and env overrides.
//!
//! Settings that are persisted between sessions:
//!   - `selected_model_id` — active 3D character model (e.g. "shinra")
//!   - `camera_azimuth`    — horizontal orbit angle (radians)
//!   - `camera_distance`   — zoom / distance from origin
//!   - `bgm_enabled`       — whether background music is playing
//!   - `bgm_volume`        — background music volume (0.0–1.0)
//!   - `bgm_track_id`      — which ambient track is selected
//!   - `model_camera_positions` — per-model camera positions (HashMap)
//!   - `user_models`       — user-imported VRM models stored under
//!     `<app_data_dir>/user_models/<id>.vrm` so they survive fresh builds.
//!
//! Environment variable overrides (for dev/CI):
//!   - `TERRANSOUL_MODEL_ID`  — override `selected_model_id`
//!
//! Schema versioning: if the persisted `version` field does not match
//! `CURRENT_SCHEMA_VERSION`, the file is treated as corrupt/stale and the
//! default settings are returned. This prevents panics from outdated fields.
//! New fields use `#[serde(default)]` for forward-compat without bumping
//! the schema (which would wipe existing settings).

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

/// A VRM model the user imported. The bytes are stored under
/// `<app_data_dir>/user_models/<id>.vrm` so they survive app upgrades and
/// fresh builds; only this metadata lives inside `app_settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserModel {
    /// Stable id (UUID v4). Also used as the on-disk file stem.
    pub id: String,
    /// Display name (defaults to the source filename without extension).
    pub name: String,
    /// Original filename, kept for display only.
    pub original_filename: String,
    /// 'female' or 'male' — drives TTS voice selection. Defaults to 'female'.
    #[serde(default = "default_user_model_gender")]
    pub gender: String,
    /// Unix milliseconds when the model was imported.
    #[serde(default)]
    pub imported_at: u64,
}

fn default_user_model_gender() -> String {
    "female".to_string()
}

/// Default character model ID (must match `DEFAULT_MODEL_ID` in frontend).
pub const DEFAULT_MODEL_ID: &str = "shinra";

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

    /// User-imported VRM models persisted under `<app_data_dir>/user_models/`.
    #[serde(default)]
    pub user_models: Vec<UserModel>,

    /// Preferred container runtime for the local-LLM quest. `Auto` picks
    /// Docker first, then Podman; explicit values force a runtime even if
    /// the other is also installed (used in environments where Docker
    /// Desktop is forbidden by company policy).
    #[serde(default)]
    pub preferred_container_runtime: crate::container::RuntimePreference,

    /// Auto-learn policy — controls how often a chat session triggers
    /// automatic memory extraction. See `docs/brain-advanced-design.md`
    /// § 21 for the full write-back / learning loop. Persisted here so
    /// users can tune cadence (or disable entirely) from the Brain hub.
    #[serde(default)]
    pub auto_learn_policy: crate::memory::AutoLearnPolicy,

    /// Minimum hybrid-search score (0.0 – 1.0) required for a memory to
    /// be injected into the `[LONG-TERM MEMORY]` block of a chat turn.
    /// Memories scoring below this threshold are skipped *before*
    /// formatting, so the brain never sees weakly-matching context that
    /// would dilute the signal.
    ///
    /// Default `DEFAULT_RELEVANCE_THRESHOLD = 0.30` matches the
    /// `docs/brain-advanced-design.md` § 16 Phase 4 (Chunk 16.1) spec.
    /// Set to `0.0` to recover the legacy "always inject top-5"
    /// behaviour. Set higher (e.g. `0.45`) to be stricter.
    #[serde(default = "default_relevance_threshold")]
    pub relevance_threshold: f64,

    /// When `true`, every `add_memory` call runs a fast LLM pass that
    /// classifies the content into ≤ 4 tags drawn from the curated prefix
    /// vocabulary (`personal:*`, `domain:*`, `code:*`, …). Default off —
    /// adds one LLM call per insert.
    ///
    /// Maps to `docs/brain-advanced-design.md` §16 Phase 2 row
    /// "Auto-categorise via LLM on insert" (chunk 18.1).
    #[serde(default)]
    pub auto_tag: bool,

    /// When `true`, the ingest pipeline prepends a 50–100 token
    /// document-level context prefix to each chunk before embedding.
    /// This "Contextual Retrieval" technique (Anthropic 2024) reduces
    /// failed retrievals by ~49 % because each chunk embedding carries
    /// the broader document context. Default off — adds one LLM call
    /// per chunk at ingest time.
    ///
    /// Maps to `docs/brain-advanced-design.md` §19.2 row 3 (chunk 16.2).
    #[serde(default)]
    pub contextual_retrieval: bool,

    /// When `true`, CRAG's web-search fallback is allowed to fetch
    /// results from DuckDuckGo when local retrieval is rated `Incorrect`.
    /// Default off — requires explicit user opt-in (capability gate).
    ///
    /// Maps to Chunk 16.5b (CRAG query-rewrite + web-search fallback).
    #[serde(default)]
    pub web_search_enabled: bool,

    /// When `true`, the MCP/gRPC servers bind to `0.0.0.0` instead of
    /// `127.0.0.1`, exposing the brain to the local network. Default off
    /// — never silently expose a brain server to the LAN. Requires
    /// explicit user opt-in with a clear UI warning.
    ///
    /// Maps to Chunk 24.1b (LAN bind config).
    #[serde(default)]
    pub lan_enabled: bool,

    /// When `true` (default), every successful `extract_memories_from_session`
    /// run is followed by an automatic `extract_edges_via_brain` pass over the
    /// freshly-grown memory store, so newly-learned facts immediately
    /// participate in the typed-edge knowledge graph instead of waiting for
    /// the user to manually trigger edge extraction. Disabling skips the
    /// follow-up call (saves one LLM round-trip per learn cycle but leaves
    /// the graph stale until manual extraction runs).
    ///
    /// Maps to `docs/brain-advanced-design.md` §21.7 roadmap item
    /// "auto-fire edge extraction" (chunk 26.3).
    #[serde(default = "default_true")]
    pub auto_extract_edges: bool,

    /// Opt-in per-ARKit-blendshape passthrough for advanced VRM rigs
    /// (Chunk 27.3). Default `false` — the camera mirror routes through
    /// the 6-preset baseline (`mapBlendshapesToVRM`) so every VRM works.
    /// When `true`, the live mirror **also** writes each of the 52
    /// ARKit shapes directly to the VRM's `expressionManager` by name
    /// (`mouthSmileLeft`, `browInnerUp`, …); rigs that don't ship those
    /// channels silently no-op via `applyExpandedBlendshapes`. See
    /// `docs/persona-design.md` §6.3 "Mask of a Thousand Faces —
    /// Expanded".
    #[serde(default)]
    pub expanded_blendshapes: bool,

    /// Set to `true` after the first-launch wizard completes (recommended
    /// or manual path).  The frontend uses this flag to decide whether to
    /// show the welcome wizard on startup.
    #[serde(default)]
    pub first_launch_complete: bool,

    /// When `true`, the 3D character viewport is hidden and the UI
    /// switches to a clean chatbox-only layout — full-height message
    /// list, no Three.js rendering, lower resource usage.  Users who
    /// prefer a text-only experience toggle this from the mode-switch
    /// pill or the Brain settings hub.
    #[serde(default)]
    pub chatbox_mode: bool,

    /// Components auto-configured by the first-launch wizard or fallback
    /// paths. Entries are descriptive keys: `"brain"`, `"voice"`,
    /// `"quests"`. Factory reset uses this list to selectively undo
    /// auto-configuration while leaving user-chosen settings intact.
    #[serde(default)]
    pub auto_configured: Vec<String>,

    /// When `true` (default), the first-launch wizard tries local Ollama
    /// before falling back to a free cloud provider. When `false`, the
    /// wizard defaults to Pollinations cloud immediately.
    /// See `rules/local-first-brain.md`.
    #[serde(default = "default_true")]
    pub prefer_local_brain: bool,

    /// Model tags the user has explicitly dismissed when offered an upgrade
    /// (e.g. `["gemma4:31b"]`). The auto-update checker skips any model
    /// whose tag appears in this list so the same upgrade is never re-shown.
    #[serde(default)]
    pub dismissed_model_updates: Vec<String>,

    /// ISO-8601 date string (`YYYY-MM-DD`) of the last automatic model
    /// update check. Used to throttle the check to once per calendar day.
    #[serde(default)]
    pub last_update_check_date: String,

    /// When `true` (default), the brain background-maintenance scheduler
    /// (`brain::maintenance_runtime`) dispatches its four jobs (decay,
    /// GC, promotion, edge-extract) on its tick loop. Set to `false` to
    /// completely suppress automatic maintenance. Maps to Chunk 26.1.
    #[serde(default = "default_true")]
    pub background_maintenance_enabled: bool,

    /// Cool-down (in hours) between successive runs of each
    /// brain-maintenance job. Default `24` (one run per day per job),
    /// clamped to `1..=168` (one hour to one week). Read on every
    /// scheduler tick, so live-edits take effect on the next tick
    /// without restart. Maps to Chunk 26.1.
    #[serde(default = "default_maintenance_interval_hours")]
    pub maintenance_interval_hours: u32,

    /// Idle-guard threshold (minutes). When > 0, the scheduler skips a
    /// tick if the user has interacted with the app within the last N
    /// minutes — avoids fighting an active chat session. `0` (default)
    /// disables the guard. Maps to Chunk 26.1.
    #[serde(default)]
    pub maintenance_idle_minimum_minutes: u32,
}

fn default_true() -> bool {
    true
}

/// Default `maintenance_interval_hours` — once per day per job.
/// Matches the 23h-cooldown convention in [`crate::brain::maintenance_scheduler::MaintenanceConfig`].
pub const DEFAULT_MAINTENANCE_INTERVAL_HOURS: u32 = 24;

/// Min/max guardrails for `AppSettings::maintenance_interval_hours`.
pub const MIN_MAINTENANCE_INTERVAL_HOURS: u32 = 1;
pub const MAX_MAINTENANCE_INTERVAL_HOURS: u32 = 168;

fn default_maintenance_interval_hours() -> u32 {
    DEFAULT_MAINTENANCE_INTERVAL_HOURS
}

/// Default relevance threshold for `[LONG-TERM MEMORY]` injection — see
/// [`AppSettings::relevance_threshold`] and `docs/brain-advanced-design.md`
/// § 16 Phase 4 (Chunk 16.1).
pub const DEFAULT_RELEVANCE_THRESHOLD: f64 = 0.30;

fn default_relevance_threshold() -> f64 {
    DEFAULT_RELEVANCE_THRESHOLD
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
            user_models: Vec::new(),
            preferred_container_runtime: crate::container::RuntimePreference::Auto,
            auto_learn_policy: crate::memory::AutoLearnPolicy::default(),
            relevance_threshold: DEFAULT_RELEVANCE_THRESHOLD,
            auto_tag: false,
            contextual_retrieval: false,
            web_search_enabled: false,
            lan_enabled: false,
            auto_extract_edges: true,
            expanded_blendshapes: false,
            first_launch_complete: false,
            chatbox_mode: false,
            auto_configured: Vec::new(),
            prefer_local_brain: true,
            dismissed_model_updates: Vec::new(),
            last_update_check_date: String::new(),
            background_maintenance_enabled: true,
            maintenance_interval_hours: DEFAULT_MAINTENANCE_INTERVAL_HOURS,
            maintenance_idle_minimum_minutes: 0,
        }
    }
}

impl AppSettings {
    /// Returns the per-job cool-down (in milliseconds) the
    /// brain-maintenance scheduler should use, derived from
    /// `maintenance_interval_hours`. Clamped to
    /// `MIN_MAINTENANCE_INTERVAL_HOURS..=MAX_MAINTENANCE_INTERVAL_HOURS`
    /// so a corrupt settings file can't disable maintenance entirely
    /// (set `background_maintenance_enabled = false` for that).
    pub fn maintenance_cooldown_ms(&self) -> u64 {
        let hours = self.maintenance_interval_hours.clamp(
            MIN_MAINTENANCE_INTERVAL_HOURS,
            MAX_MAINTENANCE_INTERVAL_HOURS,
        ) as u64;
        hours.saturating_mul(60 * 60 * 1000)
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

    /// Record a component tag in `auto_configured` (deduplicates).
    pub fn track_auto_configured(&mut self, tag: &str) {
        let s = tag.to_string();
        if !self.auto_configured.contains(&s) {
            self.auto_configured.push(s);
        }
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
        std::env::set_var("TERRANSOUL_MODEL_ID", "komori");
        let mut s = AppSettings::default();
        s.apply_env_overrides();
        std::env::remove_var("TERRANSOUL_MODEL_ID");
        assert_eq!(s.selected_model_id, "komori");
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
        positions.insert(
            "shinra".to_string(),
            ModelCameraPosition {
                azimuth: 0.5,
                distance: 3.0,
            },
        );
        let s = AppSettings {
            version: CURRENT_SCHEMA_VERSION,
            selected_model_id: "komori".into(),
            camera_azimuth: 1.57,
            camera_distance: 3.2,
            bgm_enabled: true,
            bgm_volume: 0.3,
            bgm_track_id: "moonflow".into(),
            model_camera_positions: positions,
            user_models: Vec::new(),
            preferred_container_runtime: crate::container::RuntimePreference::Docker,
            auto_learn_policy: crate::memory::AutoLearnPolicy::default(),
            relevance_threshold: DEFAULT_RELEVANCE_THRESHOLD,
            auto_tag: false,
            contextual_retrieval: false,
            web_search_enabled: false,
            lan_enabled: false,
            auto_extract_edges: true,
            expanded_blendshapes: false,
            first_launch_complete: false,
            chatbox_mode: false,
            auto_configured: Vec::new(),
            prefer_local_brain: true,
            dismissed_model_updates: Vec::new(),
            last_update_check_date: String::new(),
            background_maintenance_enabled: true,
            maintenance_interval_hours: DEFAULT_MAINTENANCE_INTERVAL_HOURS,
            maintenance_idle_minimum_minutes: 0,
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
        let json = r#"{"version":2,"selected_model_id":"shinra","camera_azimuth":0,"camera_distance":2.8}"#;
        let parsed: AppSettings = serde_json::from_str(json).unwrap();
        assert!(!parsed.bgm_enabled);
        assert!((parsed.bgm_volume - DEFAULT_BGM_VOLUME).abs() < 0.001);
        assert_eq!(parsed.bgm_track_id, DEFAULT_BGM_TRACK_ID);
    }

    /// Chunk 26.3 — `auto_extract_edges` defaults to `true` so newly-learned
    /// facts auto-participate in the typed-edge graph without requiring the
    /// user to opt in. Persisted-config files that pre-date this field must
    /// also deserialise with `auto_extract_edges = true` (forward-compat).
    #[test]
    fn default_auto_extract_edges_is_on() {
        let s = AppSettings::default();
        assert!(
            s.auto_extract_edges,
            "auto_extract_edges must default to true (Chunk 26.3)"
        );
    }

    #[test]
    fn serde_fills_auto_extract_edges_default_when_missing() {
        let json = r#"{"version":2,"selected_model_id":"shinra","camera_azimuth":0,"camera_distance":2.8}"#;
        let parsed: AppSettings = serde_json::from_str(json).unwrap();
        assert!(
            parsed.auto_extract_edges,
            "missing auto_extract_edges must deserialise to true for forward-compat"
        );
    }

    #[test]
    fn auto_extract_edges_roundtrips_through_serde() {
        let s = AppSettings {
            auto_extract_edges: false,
            ..AppSettings::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        let parsed: AppSettings = serde_json::from_str(&json).unwrap();
        assert!(!parsed.auto_extract_edges);
    }

    /// Chunk 26.1 — `background_maintenance_enabled` defaults to `true`
    /// and `maintenance_interval_hours` defaults to 24h so a fresh install
    /// runs the brain-maintenance jobs automatically. Pre-existing config
    /// files (without the new fields) must deserialise to the same defaults.
    #[test]
    fn default_background_maintenance_is_on() {
        let s = AppSettings::default();
        assert!(s.background_maintenance_enabled);
        assert_eq!(
            s.maintenance_interval_hours,
            DEFAULT_MAINTENANCE_INTERVAL_HOURS
        );
        assert_eq!(s.maintenance_idle_minimum_minutes, 0);
    }

    #[test]
    fn serde_fills_maintenance_defaults_when_missing() {
        let json = r#"{"version":2,"selected_model_id":"shinra","camera_azimuth":0,"camera_distance":2.8}"#;
        let parsed: AppSettings = serde_json::from_str(json).unwrap();
        assert!(parsed.background_maintenance_enabled);
        assert_eq!(
            parsed.maintenance_interval_hours,
            DEFAULT_MAINTENANCE_INTERVAL_HOURS
        );
        assert_eq!(parsed.maintenance_idle_minimum_minutes, 0);
    }

    /// `maintenance_cooldown_ms` clamps to the documented 1h..168h
    /// range so a corrupt config file (`= 0` or `= u32::MAX`) cannot
    /// either disable maintenance entirely or push it out forever.
    #[test]
    fn maintenance_cooldown_clamps_below_minimum() {
        let s = AppSettings {
            maintenance_interval_hours: 0,
            ..AppSettings::default()
        };
        // Clamps up to MIN (1h).
        assert_eq!(s.maintenance_cooldown_ms(), 60 * 60 * 1000);
    }

    #[test]
    fn maintenance_cooldown_clamps_above_maximum() {
        let s = AppSettings {
            maintenance_interval_hours: 1_000,
            ..AppSettings::default()
        };
        // Clamps down to MAX (168h = 1 week).
        assert_eq!(s.maintenance_cooldown_ms(), 168 * 60 * 60 * 1000);
    }

    #[test]
    fn maintenance_cooldown_default_is_24h() {
        let s = AppSettings::default();
        assert_eq!(s.maintenance_cooldown_ms(), 24 * 60 * 60 * 1000);
    }

    #[test]
    fn default_model_camera_positions_is_empty() {
        let s = AppSettings::default();
        assert!(s.model_camera_positions.is_empty());
    }

    #[test]
    fn model_camera_position_serde_roundtrip() {
        let pos = ModelCameraPosition {
            azimuth: 1.23,
            distance: 4.5,
        };
        let json = serde_json::to_string(&pos).unwrap();
        let parsed: ModelCameraPosition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, pos);
    }

    #[test]
    fn model_camera_positions_independent_per_model() {
        let mut s = AppSettings::default();
        s.model_camera_positions.insert(
            "shinra".into(),
            ModelCameraPosition {
                azimuth: 0.5,
                distance: 3.0,
            },
        );
        s.model_camera_positions.insert(
            "komori".into(),
            ModelCameraPosition {
                azimuth: 1.2,
                distance: 2.0,
            },
        );

        assert_eq!(s.model_camera_positions.len(), 2);
        assert!((s.model_camera_positions["shinra"].azimuth - 0.5).abs() < 0.001);
        assert!((s.model_camera_positions["komori"].distance - 2.0).abs() < 0.001);
    }

    #[test]
    fn serde_fills_model_camera_positions_default_when_missing() {
        // JSON without model_camera_positions field — should default to empty map
        let json = r#"{"version":2,"selected_model_id":"shinra","camera_azimuth":0,"camera_distance":2.8,"bgm_enabled":false,"bgm_volume":0.15,"bgm_track_id":"prelude"}"#;
        let parsed: AppSettings = serde_json::from_str(json).unwrap();
        assert!(parsed.model_camera_positions.is_empty());
    }
}
