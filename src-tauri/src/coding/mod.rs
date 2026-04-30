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
use std::io::Write;
use std::path::Path;

pub mod autostart;
pub mod client;
pub mod conversation_learning;
pub mod engine;
pub mod git_ops;
pub mod github;
pub mod metrics;
pub mod milestones;
pub mod prompting;
pub mod repo;
pub mod workflow;

pub use conversation_learning::{learn_from_message, LearnedChunk};
pub use engine::{ProgressEvent, SelfImproveEngine};
pub use git_ops::{pull_main, PullResult};
pub use prompting::{CodingPrompt, DocSnippet, OutputShape, PROMPT_SCHEMA_VERSION};
pub use workflow::{run_coding_task, CodingTask, CodingTaskResult, TaskDocument, TaskOutputKind};pub use github::{
    clear_github_config, load_github_config, open_or_update_pr, parse_owner_repo,
    save_github_config, GitHubConfig, PrSummary,
};
pub use metrics::{MetricsLog, MetricsSummary, RunRecord};
pub use repo::RepoState;

const CODING_LLM_FILE: &str = "coding_llm_config.json";
const SELF_IMPROVE_FILE: &str = "self_improve.json";
const CODING_WORKFLOW_FILE: &str = "coding_workflow_config.json";

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

/// Curated catalogue. The local Ollama entry is hardware-adaptive: it
/// reads the user's RAM and picks the best model from the same §26
/// catalogue used by the brain's `model_recommender.rs`. Cloud
/// providers use fixed default models.
///
/// `total_ram_mb` is the user's total system RAM (from `system_info`).
/// Pass `0` to fall back to the smallest usable model.
pub fn coding_llm_recommendations(total_ram_mb: u64) -> Vec<CodingLlmRecommendation> {
    // Pick the best local model for this hardware tier, consistent with
    // brain-advanced-design.md §26.
    let brain_recs = crate::brain::model_recommender::recommend(total_ram_mb);
    let top_local = brain_recs
        .iter()
        .find(|r| r.is_top_pick && !r.is_cloud)
        .or_else(|| brain_recs.iter().find(|r| !r.is_cloud));

    let (local_model, local_display, local_description) = match top_local {
        Some(r) => (
            r.model_tag.clone(),
            format!("Local Ollama — {} (free, private)", r.display_name),
            format!(
                "{}. Runs entirely on this machine via Ollama. No API key, no per-token cost, no data leaves your computer.",
                r.description.trim_end_matches('.')
            ),
        ),
        None => (
            "gemma3:4b".to_string(),
            "Local Ollama (free, private)".to_string(),
            "Runs entirely on this machine via Ollama. No API key, no per-token cost, no data leaves your computer.".to_string(),
        ),
    };

    vec![
        CodingLlmRecommendation {
            provider: CodingLlmProvider::Custom,
            display_name: local_display,
            default_model: local_model,
            // Base URL is the Ollama host root — `OpenAiClient` appends
            // `/v1/chat/completions` itself.
            base_url: "http://127.0.0.1:11434".to_string(),
            requires_api_key: false,
            notes: local_description,
            is_top_pick: true,
        },
        CodingLlmRecommendation {
            provider: CodingLlmProvider::Anthropic,
            display_name: "Anthropic Claude".to_string(),
            default_model: "claude-sonnet-4-5".to_string(),
            // Anthropic's OpenAI-compatible bridge lives at this host;
            // `OpenAiClient` appends `/v1/chat/completions` itself.
            base_url: "https://api.anthropic.com".to_string(),
            requires_api_key: true,
            notes: "Best-in-class for multi-file refactors and reasoning. Recommended when paying for cloud quality.".to_string(),
            is_top_pick: false,
        },
        CodingLlmRecommendation {
            provider: CodingLlmProvider::Openai,
            display_name: "OpenAI".to_string(),
            default_model: "gpt-5".to_string(),
            base_url: "https://api.openai.com".to_string(),
            requires_api_key: true,
            notes: "Strong general-purpose coding; reliable tool-calling.".to_string(),
            is_top_pick: false,
        },
        CodingLlmRecommendation {
            provider: CodingLlmProvider::Deepseek,
            display_name: "DeepSeek".to_string(),
            default_model: "deepseek-coder".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
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
            notes: "Bring-your-own endpoint (Groq, Together, LM Studio, vLLM, etc.).".to_string(),
            is_top_pick: false,
        },
    ]
}

/// Atomically serialise `value` to `path` as pretty JSON.
///
/// Implements the durability contract from
/// `rules/coding-workflow-reliability.md` §1: serialise to a sibling
/// `*.tmp` file in the same directory, `flush` + `sync_all` to force
/// the bytes to disk, then `rename` over the destination. A crash at
/// any point leaves either the previous good file or no change at all
/// — never a partially-written destination.
///
/// The temp file is best-effort cleaned up on serialisation failure.
pub fn atomic_write_json<T: serde::Serialize>(
    path: &Path,
    value: &T,
    label: &str,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create dir for {label}: {e}"))?;
    }
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| format!("serialize {label}: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = fs::File::create(&tmp)
            .map_err(|e| format!("create {label} tmp: {e}"))?;
        f.write_all(json.as_bytes())
            .map_err(|e| format!("write {label} tmp: {e}"))?;
        f.flush().map_err(|e| format!("flush {label} tmp: {e}"))?;
        // Best-effort fsync — on platforms without sync_all support
        // (rare), we still rely on the rename being atomic at the FS
        // layer. Errors here are upgraded to hard failures because
        // the durability contract requires it.
        f.sync_all().map_err(|e| format!("fsync {label} tmp: {e}"))?;
    }
    fs::rename(&tmp, path).map_err(|e| {
        // Clean up the orphaned temp file so retries start clean.
        let _ = fs::remove_file(&tmp);
        format!("rename {label} tmp -> dest: {e}")
    })
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
    let path = data_dir.join(CODING_LLM_FILE);
    atomic_write_json(&path, config, "coding llm config")
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
    let path = data_dir.join(SELF_IMPROVE_FILE);
    atomic_write_json(&path, settings, "self-improve")
}

/// Configurable context-loading rules for every coding workflow.
///
/// Controls which files the shared `run_coding_task` runner injects into
/// the prompt's `<documents>` block. Persisted as JSON so users can edit
/// it from the UI without recompiling. Provider-agnostic — the same
/// config applies regardless of which LLM (Claude / OpenAI / DeepSeek /
/// local Ollama via OpenAI-compatible proxy) is selected as the
/// Coding LLM.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodingWorkflowConfig {
    /// Repository-relative directories whose `*.md` files are loaded
    /// (non-recursive). Default: `rules`, `instructions`, `docs`.
    pub include_dirs: Vec<String>,
    /// Repository-relative individual file paths to load verbatim.
    /// Default: `README.md`, `AGENTS.md` (when present — missing files
    /// are silently skipped).
    pub include_files: Vec<String>,
    /// Repository-relative paths or simple file names to skip when
    /// scanning `include_dirs` or `include_files`. Default: empty —
    /// nothing excluded.
    pub exclude_paths: Vec<String>,
    /// Per-file character cap. Files larger than this are truncated
    /// with a `[truncated to N chars]` marker.
    pub max_file_chars: usize,
    /// Total character cap across all loaded files. The loader stops
    /// adding files once this cap is reached.
    pub max_total_chars: usize,
}

impl Default for CodingWorkflowConfig {
    fn default() -> Self {
        Self {
            include_dirs: vec![
                "rules".to_string(),
                "instructions".to_string(),
                "docs".to_string(),
            ],
            include_files: vec!["README.md".to_string(), "AGENTS.md".to_string()],
            exclude_paths: Vec::new(),
            // Defaults match the previous hardcoded constants in
            // `workflow::MAX_FILE_CHARS` / `workflow::MAX_CONTEXT_CHARS`.
            max_file_chars: 4_000,
            max_total_chars: 30_000,
        }
    }
}

/// Load the coding-workflow config from disk. Returns the default
/// config when the file is missing or unparseable so the workflow
/// always has a working set of rules.
pub fn load_coding_workflow_config(data_dir: &Path) -> CodingWorkflowConfig {
    let path = data_dir.join(CODING_WORKFLOW_FILE);
    if !path.exists() {
        return CodingWorkflowConfig::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist the coding-workflow config as JSON.
pub fn save_coding_workflow_config(
    data_dir: &Path,
    config: &CodingWorkflowConfig,
) -> Result<(), String> {
    let path = data_dir.join(CODING_WORKFLOW_FILE);
    atomic_write_json(&path, config, "coding workflow config")
}

/// Reset the coding-workflow config to defaults (deletes the file).
pub fn clear_coding_workflow_config(data_dir: &Path) -> Result<(), String> {
    let path = data_dir.join(CODING_WORKFLOW_FILE);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("remove coding workflow config: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn recommendations_include_local_ollama_first_with_claude_openai_deepseek() {
        // 16 GB RAM → should recommend Gemma 4 E2B per §26 top-picks.
        let recs = coding_llm_recommendations(16_384);
        let names: Vec<_> = recs.iter().map(|r| r.display_name.as_str()).collect();
        assert!(
            names.iter().any(|n| n.contains("Local Ollama")),
            "missing Local Ollama: {names:?}"
        );
        assert!(names.iter().any(|n| n.contains("Claude")));
        assert!(names.iter().any(|n| n.contains("OpenAI")));
        assert!(names.iter().any(|n| n.contains("DeepSeek")));
        // Local Ollama should be the single top pick — it is free,
        // private, and works fully offline.
        let top: Vec<_> = recs.iter().filter(|r| r.is_top_pick).collect();
        assert_eq!(top.len(), 1, "exactly one top pick");
        assert!(top[0].display_name.contains("Local Ollama"));
        assert!(!top[0].requires_api_key, "local provider needs no key");
        // No recommendation may have a doubled `/v1` suffix in its
        // base URL — the OpenAI client appends `/v1/chat/completions`.
        for rec in &recs {
            assert!(
                !rec.base_url.trim_end_matches('/').ends_with("/v1"),
                "{}: base_url must not end with /v1 (got {:?})",
                rec.display_name,
                rec.base_url
            );
        }
    }

    #[test]
    fn local_model_adapts_to_ram_tier() {
        // VeryHigh → gemma4:31b
        let recs = coding_llm_recommendations(65_536);
        let local = recs.iter().find(|r| r.is_top_pick).unwrap();
        assert_eq!(local.default_model, "gemma4:31b");

        // High → gemma4:e4b
        let recs = coding_llm_recommendations(24_000);
        let local = recs.iter().find(|r| r.is_top_pick).unwrap();
        assert_eq!(local.default_model, "gemma4:e4b");

        // Medium → gemma4:e2b
        let recs = coding_llm_recommendations(12_000);
        let local = recs.iter().find(|r| r.is_top_pick).unwrap();
        assert_eq!(local.default_model, "gemma4:e2b");

        // Low → gemma3:1b
        let recs = coding_llm_recommendations(6_000);
        let local = recs.iter().find(|r| r.is_top_pick).unwrap();
        assert_eq!(local.default_model, "gemma3:1b");

        // VeryLow → tinyllama
        let recs = coding_llm_recommendations(2_000);
        let local = recs.iter().find(|r| r.is_top_pick).unwrap();
        assert_eq!(local.default_model, "tinyllama");
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
