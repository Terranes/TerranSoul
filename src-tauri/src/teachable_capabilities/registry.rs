//! User-configurable companion capabilities (Chunk 30.5).
//!
//! This registry records externally researched feature patterns as neutral,
//! user-teachable capability records. It intentionally avoids creator,
//! channel, project, or brand names in module names, identifiers, persisted
//! paths, comments, and user-facing strings.
//!
//! Each capability has:
//!
//! * a stable `id` (snake_case);
//! * a category (Voice, Vision, Persona, Phone, Files, Game, Visuals,
//!   Hardware, Integrations);
//! * an `enabled` flag the user can toggle from the management panel;
//! * a free-form `config` JSON blob whose schema is described by the
//!   `config_schema` field;
//! * Charisma-style usage / rating bookkeeping;
//! * a `target_files` list that the promotion workflow uses to know which
//!   source files this capability's bundled defaults live in.
//!
//! Like Charisma, "Promote" creates a coding `WorkflowPlan` so a user's
//! tuned configuration can be promoted to a bundled default via the existing
//! self-improve / multi-agent runner.
//!
//! Persisted as a single atomic JSON file:
//!
//! ```text
//! <app_data_dir>/teachable_capabilities/capabilities.json
//! ```

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Top-level grouping for the management panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityCategory {
    Voice,
    Vision,
    Persona,
    Phone,
    Files,
    Game,
    Visuals,
    Hardware,
    Integrations,
}

impl CapabilityCategory {
    pub fn label(self) -> &'static str {
        match self {
            CapabilityCategory::Voice => "Voice",
            CapabilityCategory::Vision => "Vision",
            CapabilityCategory::Persona => "Persona",
            CapabilityCategory::Phone => "Phone Control",
            CapabilityCategory::Files => "File Assistant",
            CapabilityCategory::Game => "Game Companion",
            CapabilityCategory::Visuals => "Visual Generation",
            CapabilityCategory::Hardware => "Hardware",
            CapabilityCategory::Integrations => "Integrations",
        }
    }
}

/// User-tuneable capability record. The `config` blob is intentionally
/// untyped at the Rust level: each capability publishes a `config_schema`
/// descriptor so the management panel can render an editor without adding a
/// Rust type for every capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeachableCapability {
    pub id: String,
    pub category: CapabilityCategory,
    pub display_name: String,
    pub summary: String,
    /// Neutral research pointer or internal note for where the feature pattern
    /// came from. Runtime code must not depend on third-party naming here.
    pub source_ref: String,
    pub enabled: bool,
    pub config: Value,
    pub config_schema: Value,
    pub target_files: Vec<String>,
    pub usage_count: u32,
    pub last_used_at: u64,
    pub rating_sum: u32,
    pub rating_count: u32,
    pub promoted_at: Option<u64>,
    pub last_promotion_plan_id: Option<String>,
}

impl TeachableCapability {
    pub fn avg_rating(&self) -> f32 {
        if self.rating_count == 0 {
            0.0
        } else {
            self.rating_sum as f32 / self.rating_count as f32
        }
    }

    pub fn maturity(&self) -> Maturity {
        use crate::coding::promotion_plan::{PROVEN_MIN_AVG_RATING, PROVEN_MIN_USES};
        if self.promoted_at.is_some() {
            return Maturity::Canon;
        }
        if !self.enabled || self.usage_count == 0 {
            return Maturity::Untested;
        }
        if self.usage_count >= PROVEN_MIN_USES && self.avg_rating() >= PROVEN_MIN_AVG_RATING {
            return Maturity::Proven;
        }
        Maturity::Learning
    }
}

/// Promotion maturity is re-exported from the shared module so all
/// user-taught source-promotion surfaces use the same thresholds.
pub use crate::coding::promotion_plan::Maturity;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityIndex {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub capabilities: HashMap<String, TeachableCapability>,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilitySummary {
    pub total: u32,
    pub enabled: u32,
    pub untested: u32,
    pub learning: u32,
    pub proven: u32,
    pub canon: u32,
}

impl CapabilitySummary {
    pub fn from_index(index: &CapabilityIndex) -> Self {
        let mut summary = Self::default();
        for cap in index.capabilities.values() {
            summary.total += 1;
            if cap.enabled {
                summary.enabled += 1;
            }
            match cap.maturity() {
                Maturity::Untested => summary.untested += 1,
                Maturity::Learning => summary.learning += 1,
                Maturity::Proven => summary.proven += 1,
                Maturity::Canon => summary.canon += 1,
            }
        }
        summary
    }
}

// ---------------------------------------------------------------------------
// Built-in catalogue - 17 neutral capability patterns
// ---------------------------------------------------------------------------

pub fn seed_catalogue(now_ms: u64) -> Vec<TeachableCapability> {
    use CapabilityCategory::*;
    let _ = now_ms;

    let mk = |id: &str,
              cat: CapabilityCategory,
              name: &str,
              summary: &str,
              source: &str,
              config: Value,
              schema: Value,
              targets: Vec<&str>| TeachableCapability {
        id: id.to_string(),
        category: cat,
        display_name: name.to_string(),
        summary: summary.to_string(),
        source_ref: source.to_string(),
        enabled: false,
        config,
        config_schema: schema,
        target_files: targets.into_iter().map(|s| s.to_string()).collect(),
        usage_count: 0,
        last_used_at: 0,
        rating_sum: 0,
        rating_count: 0,
        promoted_at: None,
        last_promotion_plan_id: None,
    };

    let registry_file = "src-tauri/src/teachable_capabilities/registry.rs";

    vec![
        mk(
            "wake_word",
            Voice,
            "Wake-word activation",
            "Listen ambiently for a configurable hotword and start ASR only after activation.",
            "external_research:voice/wake-word",
            json!({ "phrase": "hey terra", "sensitivity": 0.5, "engine": "porcupine" }),
            json!({
                "phrase": { "type": "string", "label": "Wake phrase", "hint": "Two- or three-syllable phrase works best." },
                "sensitivity": { "type": "number", "label": "Sensitivity", "min": 0.0, "max": 1.0, "step": 0.05 },
                "engine": { "type": "enum", "label": "Engine", "options": ["porcupine", "openwakeword", "whisper-vad"] }
            }),
            vec![registry_file, "src-tauri/src/voice/mod.rs", "src/stores/voice.ts"],
        ),
        mk(
            "push_to_talk",
            Voice,
            "Push-to-talk hotkey",
            "Hold a global hotkey to capture audio, then optionally send on release.",
            "external_research:voice/push-to-talk",
            json!({ "hotkey": "Ctrl+Space", "release_to_send": true }),
            json!({
                "hotkey": { "type": "string", "label": "Hotkey", "hint": "Use VS Code key syntax." },
                "release_to_send": { "type": "boolean", "label": "Send on release" }
            }),
            vec![registry_file, "src/stores/voice.ts"],
        ),
        mk(
            "live_mic",
            Voice,
            "Live-mic transcription",
            "Continuous microphone mode with VAD and streaming speech recognition.",
            "external_research:voice/live-mic",
            json!({ "vad_threshold": 0.5, "min_speech_ms": 250, "silence_ms": 700 }),
            json!({
                "vad_threshold": { "type": "number", "label": "VAD threshold", "min": 0.0, "max": 1.0, "step": 0.05 },
                "min_speech_ms": { "type": "integer", "label": "Min speech (ms)" },
                "silence_ms": { "type": "integer", "label": "Trailing silence (ms)" }
            }),
            vec![registry_file, "src-tauri/src/voice/whisper_api.rs"],
        ),
        mk(
            "voice_clone_tts",
            Voice,
            "Reference-voice TTS",
            "Generate speech from a short reference sample with user-controlled language and speed.",
            "external_research:voice/reference-tts",
            json!({
                "engine": "gpt-sovits",
                "ref_audio_path": "",
                "ref_text": "",
                "language": "en",
                "speed": 1.0
            }),
            json!({
                "engine": { "type": "enum", "options": ["gpt-sovits", "xtts-v2", "piper", "system"] },
                "ref_audio_path": { "type": "path", "label": "Reference audio (.wav/.mp3)" },
                "ref_text": { "type": "string", "label": "Reference transcript" },
                "language": { "type": "enum", "options": ["en", "ja", "zh", "ko", "es", "fr", "de"] },
                "speed": { "type": "number", "min": 0.5, "max": 2.0, "step": 0.05 }
            }),
            vec![registry_file, "src-tauri/src/voice/stub_tts.rs", "src/stores/voice.ts"],
        ),
        mk(
            "emotion_tts",
            Voice,
            "Emotion-aware TTS",
            "Drive TTS prosody from the brain's detected emotion tag.",
            "external_research:voice/emotion-tts",
            json!({
                "happy": { "rate": 1.05, "pitch": 1.10 },
                "sad": { "rate": 0.95, "pitch": 0.92 },
                "angry": { "rate": 1.10, "pitch": 1.05 },
                "smug": { "rate": 1.00, "pitch": 1.05 }
            }),
            json!({ "type": "prosody_table" }),
            vec![registry_file, "src-tauri/src/persona/prosody.rs"],
        ),
        mk(
            "vision_input",
            Vision,
            "Webcam vision input",
            "Let the companion inspect camera frames with per-session consent and privacy controls.",
            "external_research:vision/webcam",
            json!({
                "provider": "ollama",
                "model": "llava:13b",
                "consent_per_session": true,
                "blur_background": false
            }),
            json!({
                "provider": { "type": "enum", "options": ["ollama", "openai", "anthropic"] },
                "model": { "type": "string", "label": "Vision model" },
                "consent_per_session": { "type": "boolean", "label": "Re-ask consent each session" },
                "blur_background": { "type": "boolean", "label": "Privacy blur" }
            }),
            vec![registry_file, "src-tauri/src/commands/vision.rs", "src/components/PersonaTeacher.vue"],
        ),
        mk(
            "screen_vision",
            Vision,
            "Screen-share vision",
            "Periodically capture the focused screen region and feed it to a vision model.",
            "external_research:vision/screen",
            json!({ "interval_ms": 5000, "include_active_window_only": true }),
            json!({
                "interval_ms": { "type": "integer", "label": "Capture interval (ms)" },
                "include_active_window_only": { "type": "boolean", "label": "Only the focused window" }
            }),
            vec![registry_file],
        ),
        mk(
            "personality_archetype",
            Persona,
            "Personality archetype preset",
            "Choose a swappable personality archetype and intensity for the companion.",
            "external_research:persona/archetype",
            json!({ "archetype": "snarky", "intensity": 0.7, "swearing_allowed": false }),
            json!({
                "archetype": { "type": "enum", "options": ["snarky", "tsundere", "kuudere", "dandere", "genki", "yandere", "custom"] },
                "intensity": { "type": "number", "min": 0.0, "max": 1.0, "step": 0.05 },
                "swearing_allowed": { "type": "boolean" }
            }),
            vec![registry_file, "src-tauri/src/commands/persona.rs"],
        ),
        mk(
            "yaml_persona_config",
            Persona,
            "YAML persona config import",
            "Import user-authored persona prompt settings from a YAML file and optionally watch for changes.",
            "external_research:persona/yaml",
            json!({ "yaml_path": "", "auto_reload": true }),
            json!({
                "yaml_path": { "type": "path", "label": "config.yaml path" },
                "auto_reload": { "type": "boolean", "label": "Watch file" }
            }),
            vec![registry_file, "src-tauri/src/persona/pack.rs"],
        ),
        mk(
            "phone_control",
            Phone,
            "Phone control bridge",
            "Route user-approved mobile commands over the existing TerranSoul gRPC phone bridge.",
            "external_research:phone/control",
            json!({ "host": "192.168.1.10", "port": 4421, "use_tls": true }),
            json!({
                "host": { "type": "string", "label": "Phone IP" },
                "port": { "type": "integer", "label": "gRPC port" },
                "use_tls": { "type": "boolean", "label": "TLS" }
            }),
            vec![registry_file, "src-tauri/src/ai_integrations/grpc/phone_control.rs"],
        ),
        mk(
            "productivity_gating",
            Phone,
            "Productivity app gating",
            "Block distracting mobile apps until the user completes a configured task or quiz.",
            "external_research:phone/productivity",
            json!({
                "blocked_apps": ["TikTok", "Instagram", "YouTube"],
                "unlock_modes": ["task_complete", "code_quiz"],
                "quiz_difficulty": "medium"
            }),
            json!({
                "blocked_apps": { "type": "string_list", "label": "Blocked apps" },
                "unlock_modes": { "type": "enum_list", "options": ["task_complete", "code_quiz", "pushup_count", "schedule"] },
                "quiz_difficulty": { "type": "enum", "options": ["easy", "medium", "hard"] }
            }),
            vec![registry_file],
        ),
        mk(
            "file_assistant",
            Files,
            "File-cleanup assistant",
            "Sandboxed file-organisation operations with explicit folder allowlists and dry-run mode.",
            "external_research:files/cleanup",
            json!({
                "allowed_roots": [],
                "dry_run": true,
                "ops": ["categorize", "rename", "deduplicate"]
            }),
            json!({
                "allowed_roots": { "type": "path_list", "label": "Allowed folders" },
                "dry_run": { "type": "boolean", "label": "Dry run only" },
                "ops": { "type": "enum_list", "options": ["categorize", "rename", "deduplicate", "compress"] }
            }),
            vec![registry_file, "src-tauri/src/coding/apply_file.rs"],
        ),
        mk(
            "game_companion_bot",
            Game,
            "Block-building game companion",
            "Configure a sandboxed game helper for resource gathering, building, following, and combat tasks.",
            "external_research:game/block-building",
            json!({
                "host": "localhost",
                "port": 25565,
                "username": "Terra",
                "skills": ["mine", "build", "fight", "follow"]
            }),
            json!({
                "host": { "type": "string" },
                "port": { "type": "integer" },
                "username": { "type": "string" },
                "skills": { "type": "enum_list", "options": ["mine", "build", "fight", "follow", "explore", "trade"] }
            }),
            vec![registry_file],
        ),
        mk(
            "mcp_function_calling",
            Integrations,
            "MCP function calling",
            "Let the companion call user-configured MCP servers with explicit approval controls.",
            "external_research:integrations/mcp",
            json!({
                "servers": [],
                "auto_approve_safe_tools": false
            }),
            json!({
                "servers": { "type": "object_list", "shape": { "name": "string", "url": "string", "token": "string" } },
                "auto_approve_safe_tools": { "type": "boolean" }
            }),
            vec![registry_file, "src-tauri/src/ai_integrations/mcp/mod.rs"],
        ),
        mk(
            "concept_art_generator",
            Visuals,
            "Concept-art generator",
            "Send character or outfit prompts to a user-configured local image-generation endpoint.",
            "external_research:visuals/concept-art",
            json!({
                "endpoint": "http://127.0.0.1:7860",
                "model": "anime-pastel-dream",
                "default_prompt": "anime companion, casual, full body",
                "negative_prompt": "low quality, blurry"
            }),
            json!({
                "endpoint": { "type": "string", "label": "Image API URL" },
                "model": { "type": "string", "label": "Checkpoint" },
                "default_prompt": { "type": "string" },
                "negative_prompt": { "type": "string" }
            }),
            vec![registry_file],
        ),
        mk(
            "holographic_pod",
            Hardware,
            "Mirror-display pod",
            "Render a mirrored companion layout for transparent-prism or mirror-display hardware.",
            "external_research:hardware/display-pod",
            json!({ "enabled_layout": "pyramid", "mirror_axes": ["xz"], "tint": "#88c8ff" }),
            json!({
                "enabled_layout": { "type": "enum", "options": ["flat", "pyramid", "single-mirror"] },
                "mirror_axes": { "type": "enum_list", "options": ["xy", "xz", "yz"] },
                "tint": { "type": "color" }
            }),
            vec![registry_file, "src/renderer/scene.ts"],
        ),
        mk(
            "wakeup_routine",
            Persona,
            "Wakeup routine",
            "Time-triggered greeting, weather, and calendar readback.",
            "external_research:persona/routine",
            json!({
                "time": "08:00",
                "say_weather": true,
                "say_calendar": true,
                "voice_line": "Good morning! Time to take over the world."
            }),
            json!({
                "time": { "type": "string", "label": "Time (HH:MM)" },
                "say_weather": { "type": "boolean" },
                "say_calendar": { "type": "boolean" },
                "voice_line": { "type": "string", "label": "Greeting" }
            }),
            vec![registry_file],
        ),
    ]
}

// ---------------------------------------------------------------------------
// Disk persistence
// ---------------------------------------------------------------------------

const REGISTRY_DIR: &str = "teachable_capabilities";
const REGISTRY_FILE: &str = "capabilities.json";

pub fn registry_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(REGISTRY_DIR)
}

pub fn registry_path(data_dir: &Path) -> PathBuf {
    registry_dir(data_dir).join(REGISTRY_FILE)
}

pub fn load_index(data_dir: &Path, now_ms: u64) -> Result<CapabilityIndex, String> {
    let path = registry_path(data_dir);
    let mut index: CapabilityIndex = if path.exists() {
        let bytes = fs::read(&path).map_err(|e| format!("read capabilities.json: {e}"))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("parse capabilities.json: {e}"))?
    } else {
        CapabilityIndex {
            version: default_version(),
            capabilities: HashMap::new(),
        }
    };

    for seed in seed_catalogue(now_ms) {
        index.capabilities.entry(seed.id.clone()).or_insert(seed);
    }
    if index.version == 0 {
        index.version = 1;
    }
    Ok(index)
}

pub fn save_index(data_dir: &Path, index: &CapabilityIndex) -> Result<(), String> {
    let dir = registry_dir(data_dir);
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir capabilities dir: {e}"))?;
    let path = registry_path(data_dir);
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(index).map_err(|e| format!("serialise: {e}"))?;
    {
        let mut file = fs::File::create(&tmp).map_err(|e| format!("create temp: {e}"))?;
        file.write_all(&bytes)
            .map_err(|e| format!("write temp: {e}"))?;
        file.sync_all().map_err(|e| format!("sync temp: {e}"))?;
    }
    fs::rename(&tmp, &path).map_err(|e| format!("rename temp: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Promotion plan (capability config -> bundled default in source)
// ---------------------------------------------------------------------------

use crate::coding::multi_agent::WorkflowPlan;
use crate::coding::promotion_plan::{build_promotion_plan as build_shared_plan, PromotionPlanSpec};

pub fn build_promotion_plan(
    cap: &TeachableCapability,
    plan_id: String,
    now_ms: u64,
) -> WorkflowPlan {
    let title = format!(
        "Promote capability '{}' config to source defaults",
        cap.display_name
    );
    let pretty_config =
        serde_json::to_string_pretty(&cap.config).unwrap_or_else(|_| cap.config.to_string());
    let user_request = format!(
        "User-tuned capability '{}' (id `{}`) has been used {} times with average rating {:.1}/5. Promote the user's customised config to the bundled default in source so future installs ship with it.\n\nConfig to bake in:\n```json\n{}\n```",
        cap.display_name,
        cap.id,
        cap.usage_count,
        cap.avg_rating(),
        pretty_config,
    );

    build_shared_plan(PromotionPlanSpec {
        plan_id,
        now_ms,
        title,
        user_request,
        research_description: format!(
            "Open the listed target file(s) and locate the bundled default for capability `{}`. Report the exact insertion point so the Coder does not blindly overwrite. If no bundled default exists yet, propose where it should live, preferring `seed_catalogue` in `src-tauri/src/teachable_capabilities/registry.rs`.",
            cap.id
        ),
        code_description: format!(
            "Emit a `<file path=...>` block updating the bundled default for `{}` to the user-tuned config above. Preserve all surrounding code verbatim.",
            cap.id
        ),
        test_description:
            "Run the closest test slice for the touched files (`cargo test --lib teachable_capabilities` and `npx vitest run src/stores/teachable-capabilities`) to make sure the new default still loads."
                .to_string(),
        review_description:
            "Security and quality review: no secrets in the new default, no absolute filesystem paths containing usernames, and no third-party creator/project names in identifiers or user-facing strings."
                .to_string(),
        tags: vec![
            "teachable_capability".to_string(),
            "promotion".to_string(),
            cap.category.label().to_lowercase(),
        ],
        target_files: &cap.target_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn seed_catalogue_has_seventeen_capabilities() {
        let seeds = seed_catalogue(0);
        assert_eq!(seeds.len(), 17, "catalogue must have 17 entries");
        let mut ids: Vec<&str> = seeds.iter().map(|c| c.id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "duplicate capability id");
        for cap in &seeds {
            assert!(
                !cap.target_files.is_empty(),
                "{} has no target_files",
                cap.id
            );
            assert!(!cap.summary.is_empty(), "{} missing summary", cap.id);
            assert!(
                cap.config.is_object(),
                "{} config must be JSON object",
                cap.id
            );
            assert!(cap.source_ref.starts_with("external_research:"));
        }
    }

    #[test]
    fn maturity_progresses_with_usage_and_rating() {
        let mut caps = seed_catalogue(0);
        let mut cap = caps.remove(0);
        assert_eq!(cap.maturity(), Maturity::Untested);

        cap.enabled = true;
        cap.usage_count = 1;
        assert_eq!(cap.maturity(), Maturity::Learning);

        cap.usage_count = 12;
        cap.rating_sum = 16;
        cap.rating_count = 4;
        assert_eq!(cap.maturity(), Maturity::Proven);

        cap.promoted_at = Some(1);
        assert_eq!(cap.maturity(), Maturity::Canon);
    }

    #[test]
    fn disabled_capability_is_untested_even_with_usage() {
        let mut caps = seed_catalogue(0);
        let mut cap = caps.remove(0);
        cap.usage_count = 50;
        cap.rating_sum = 50;
        cap.rating_count = 10;
        cap.enabled = false;
        assert_eq!(cap.maturity(), Maturity::Untested);
    }

    #[test]
    fn save_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let mut idx = load_index(tmp.path(), 0).unwrap();
        assert_eq!(idx.capabilities.len(), 17);

        let cap = idx.capabilities.get_mut("wake_word").unwrap();
        cap.enabled = true;
        cap.config["phrase"] = json!("hey terra-soul");
        cap.usage_count = 3;

        save_index(tmp.path(), &idx).unwrap();

        let reloaded = load_index(tmp.path(), 0).unwrap();
        let reloaded_cap = reloaded.capabilities.get("wake_word").unwrap();
        assert!(reloaded_cap.enabled);
        assert_eq!(reloaded_cap.config["phrase"], "hey terra-soul");
        assert_eq!(reloaded_cap.usage_count, 3);
    }

    #[test]
    fn summary_counts_match() {
        let mut idx = CapabilityIndex::default();
        for seed in seed_catalogue(0) {
            idx.capabilities.insert(seed.id.clone(), seed);
        }
        let summary = CapabilitySummary::from_index(&idx);
        assert_eq!(summary.total, 17);
        assert_eq!(summary.untested, 17);
        assert_eq!(summary.enabled, 0);
    }

    #[test]
    fn build_promotion_plan_shape_is_correct() {
        let mut caps = seed_catalogue(0);
        let cap = caps.remove(0);
        let plan = build_promotion_plan(&cap, "plan_x".into(), 1_000);
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(plan.kind, crate::coding::multi_agent::WorkflowKind::Coding);
        assert!(plan.tags.contains(&"teachable_capability".to_string()));
        assert!(plan.steps[1].requires_approval);
        assert!(plan.steps[3].requires_approval);
    }
}
