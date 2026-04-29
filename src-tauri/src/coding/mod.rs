//! Coding LLM + Self-Improve foundation.
//!
//! Phase 1 scaffolding for the autonomous self-improving coding system.
//! See `rules/milestones.md` Phase 25 for the full roadmap.
//!
//! This module currently provides:
//! - [`CodingLlmConfig`] — provider/model/key config for the dedicated
//!   "coding brain" used by the self-improve loop. Persisted as JSON.
//! - [`SelfImproveSettings`] — single-bit toggle (plus metadata) controlling
//!   whether the autonomous loop is permitted to run. Persisted as JSON.
//! - [`coding_llm_recommendations`] — curated provider catalogue
//!   (Claude, OpenAI, DeepSeek, custom OpenAI-compatible).
//!
//! No autonomous loop is implemented yet — that is gated behind future
//! chunks. The toggle is intentionally inert beyond persistence so the UI
//! surface, confirmation flow, and storage schema can land safely first.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub mod autostart;
pub mod client;
pub mod engine;
pub mod metrics;
pub mod milestones;
pub mod repo;

pub use engine::{ProgressEvent, SelfImproveEngine};
pub use metrics::{MetricsLog, MetricsSummary, RunRecord};
pub use repo::RepoState;

const CODING_LLM_FILE: &str = "coding_llm_config.json";
const SELF_IMPROVE_FILE: &str = "self_improve.json";

/// Provider identifier for the coding LLM. Kept as a string-typed enum so
/// the UI can pass `"anthropic"`, `"openai"`, `"deepseek"`, or `"custom"`
/// without an extra mapping layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CodingLlmProvider {
    Anthropic,
    Openai,
    Deepseek,
    Custom,
}

/// Persisted configuration for the dedicated coding LLM.
///
/// Always uses an OpenAI-compatible chat-completions endpoint. Anthropic
/// users supply their key via Anthropic's OpenAI-compatible bridge or a
/// proxy; DeepSeek and OpenAI both speak `/v1/chat/completions` natively.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodingLlmConfig {
    pub provider: CodingLlmProvider,
    pub model: String,
    pub base_url: String,
    /// Stored verbatim; treat as a secret (do not log).
    pub api_key: String,
}

/// Recommended, well-known coding-LLM defaults. The frontend picker shows
/// these as one-click presets; users can still override `model` and
/// `base_url`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingLlmRecommendation {
    pub provider: CodingLlmProvider,
    pub display_name: String,
    pub default_model: String,
    pub base_url: String,
    pub requires_api_key: bool,
    pub notes: String,
    pub is_top_pick: bool,
}

/// Curated catalogue. Keep this list small and opinionated — the design
/// doc explicitly recommends Claude, OpenAI, and DeepSeek for coding.
pub fn coding_llm_recommendations() -> Vec<CodingLlmRecommendation> {
    vec![
        CodingLlmRecommendation {
            provider: CodingLlmProvider::Anthropic,
            display_name: "Anthropic Claude".to_string(),
            default_model: "claude-sonnet-4-5".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            requires_api_key: true,
            notes: "Best-in-class for multi-file refactors and reasoning. Recommended for self-improve.".to_string(),
            is_top_pick: true,
        },
        CodingLlmRecommendation {
            provider: CodingLlmProvider::Openai,
            display_name: "OpenAI".to_string(),
            default_model: "gpt-5".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            requires_api_key: true,
            notes: "Strong general-purpose coding; reliable tool-calling.".to_string(),
            is_top_pick: false,
        },
        CodingLlmRecommendation {
            provider: CodingLlmProvider::Deepseek,
            display_name: "DeepSeek".to_string(),
            default_model: "deepseek-coder".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            requires_api_key: true,
            notes: "Cost-efficient coding-tuned model. Excellent value per token.".to_string(),
            is_top_pick: false,
        },
        CodingLlmRecommendation {
            provider: CodingLlmProvider::Custom,
            display_name: "Custom OpenAI-compatible".to_string(),
            default_model: "".to_string(),
            base_url: "".to_string(),
            requires_api_key: true,
            notes: "Bring-your-own endpoint (Groq, Together, local proxy, etc.).".to_string(),
            is_top_pick: false,
        },
    ]
}

/// Load the coding LLM config from disk. Returns `None` if not configured.
pub fn load_coding_llm(data_dir: &Path) -> Option<CodingLlmConfig> {
    let path = data_dir.join(CODING_LLM_FILE);
    if !path.exists() {
        return None;
    }
    let contents = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Persist the coding LLM config as JSON.
pub fn save_coding_llm(data_dir: &Path, config: &CodingLlmConfig) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("create dir: {e}"))?;
    let path = data_dir.join(CODING_LLM_FILE);
    let json = serde_json::to_string_pretty(config).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write coding llm config: {e}"))
}

/// Remove the persisted coding LLM config.
pub fn clear_coding_llm(data_dir: &Path) -> Result<(), String> {
    let path = data_dir.join(CODING_LLM_FILE);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("remove coding llm config: {e}"))?;
    }
    Ok(())
}

/// Self-improve toggle + acknowledgement metadata.
///
/// `enabled` is the only operationally meaningful field today; the rest
/// is forensic information so the autonomous loop (future chunks) can
/// resume idempotently and so audits can show *when* and *with which
/// coding model* a user opted in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SelfImproveSettings {
    pub enabled: bool,
    /// Unix-epoch seconds of the last toggle change (0 = never set).
    #[serde(default)]
    pub updated_at: u64,
    /// Unix-epoch seconds of the most recent user confirmation (0 = none).
    /// The UI re-prompts when this is older than the toggle change.
    #[serde(default)]
    pub last_acknowledged_at: u64,
    /// Snapshot of the coding-LLM provider name at activation time, for
    /// audit trails. Empty string when never enabled.
    #[serde(default)]
    pub last_provider: String,
}

/// Load the self-improve settings (defaults to disabled).
pub fn load_self_improve(data_dir: &Path) -> SelfImproveSettings {
    let path = data_dir.join(SELF_IMPROVE_FILE);
    if !path.exists() {
        return SelfImproveSettings::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist the self-improve settings.
pub fn save_self_improve(data_dir: &Path, settings: &SelfImproveSettings) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("create dir: {e}"))?;
    let path = data_dir.join(SELF_IMPROVE_FILE);
    let json = serde_json::to_string_pretty(settings).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write self-improve: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn recommendations_include_claude_openai_deepseek_with_claude_top_pick() {
        let recs = coding_llm_recommendations();
        let names: Vec<_> = recs.iter().map(|r| r.display_name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("Claude")));
        assert!(names.iter().any(|n| n.contains("OpenAI")));
        assert!(names.iter().any(|n| n.contains("DeepSeek")));
        let top: Vec<_> = recs.iter().filter(|r| r.is_top_pick).collect();
        assert_eq!(top.len(), 1, "exactly one top pick");
        assert_eq!(top[0].provider, CodingLlmProvider::Anthropic);
    }

    #[test]
    fn coding_llm_round_trip() {
        let dir = tempdir().unwrap();
        assert!(load_coding_llm(dir.path()).is_none());

        let cfg = CodingLlmConfig {
            provider: CodingLlmProvider::Anthropic,
            model: "claude-sonnet-4-5".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            api_key: "sk-test".to_string(),
        };
        save_coding_llm(dir.path(), &cfg).unwrap();

        let loaded = load_coding_llm(dir.path()).unwrap();
        assert_eq!(loaded, cfg);

        clear_coding_llm(dir.path()).unwrap();
        assert!(load_coding_llm(dir.path()).is_none());
    }

    #[test]
    fn self_improve_defaults_to_disabled_and_persists() {
        let dir = tempdir().unwrap();
        let loaded = load_self_improve(dir.path());
        assert!(!loaded.enabled);
        assert_eq!(loaded.updated_at, 0);

        let s = SelfImproveSettings {
            enabled: true,
            updated_at: 1714000000,
            last_acknowledged_at: 1714000000,
            last_provider: "anthropic".to_string(),
        };
        save_self_improve(dir.path(), &s).unwrap();

        let reloaded = load_self_improve(dir.path());
        assert_eq!(reloaded, s);
    }

    #[test]
    fn self_improve_corrupt_file_returns_default() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(SELF_IMPROVE_FILE), "{ not json").unwrap();
        let loaded = load_self_improve(dir.path());
        assert!(!loaded.enabled);
    }
}
