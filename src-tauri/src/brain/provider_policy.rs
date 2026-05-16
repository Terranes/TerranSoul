//! Unified provider policy registry (Chunk 35.1, extended 35.3).
//!
//! A single app-wide configuration registry mapping per-task model/provider
//! overrides. When an override is configured for a given `TaskKind`, the
//! system uses that specific provider + model instead of the default brain
//! mode. When no override exists, the active `BrainMode` is used as the
//! fallback for all tasks.
//!
//! Chunk 35.3 adds **agent-role routing**: each `AgentRole` (Planner, Coder,
//! Reviewer, etc.) can have its own preferred model class, token budget, and
//! fallback chain. When a workflow step resolves its provider, it goes through
//! the agent routing policy first, then falls through to task-kind resolution.
//!
//! Persists to `<data_dir>/provider_policy.json`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::coding::multi_agent::AgentRole;

/// Distinct task types that can each have their own model/provider override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// General conversational chat.
    Chat,
    /// Text embedding for vector search / RAG.
    Embeddings,
    /// Cross-encoder / LLM-as-judge reranking.
    Rerank,
    /// Memory summarisation and contextual retrieval.
    Summarise,
    /// Code review and self-improve coding workflow.
    CodeReview,
    /// Long-context analysis (e.g. full-file or multi-file reasoning).
    LongContext,
}

impl TaskKind {
    /// All known task kinds, useful for iteration.
    pub const ALL: &'static [TaskKind] = &[
        TaskKind::Chat,
        TaskKind::Embeddings,
        TaskKind::Rerank,
        TaskKind::Summarise,
        TaskKind::CodeReview,
        TaskKind::LongContext,
    ];

    /// Human-readable label for display in UI / logs.
    pub fn label(self) -> &'static str {
        match self {
            TaskKind::Chat => "Chat",
            TaskKind::Embeddings => "Embeddings",
            TaskKind::Rerank => "Rerank",
            TaskKind::Summarise => "Summarise",
            TaskKind::CodeReview => "Code Review",
            TaskKind::LongContext => "Long Context",
        }
    }
}

/// A per-task provider override. When set for a `TaskKind`, this provider +
/// model will be used instead of the app-wide default brain mode.
///
/// All fields are optional except `kind` — this is intentionally flexible so
/// the system can derive sensible defaults from the active `BrainMode` when
/// only part of the override is specified (e.g. user overrides model but keeps
/// the same provider endpoint).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskOverride {
    /// Which task this applies to.
    pub kind: TaskKind,
    /// Provider identifier (e.g. "openai", "groq", "ollama", "lm-studio", or
    /// a free-provider id like "pollinations").
    pub provider_id: Option<String>,
    /// Model name (e.g. "gpt-4o-mini", "nomic-embed-text", "gemma3:4b").
    pub model: Option<String>,
    /// Base URL override. When `None`, the system uses the provider's
    /// default endpoint.
    pub base_url: Option<String>,
    /// API key override. When `None`, uses the key from the app-wide brain
    /// mode or the provider catalogue.
    pub api_key: Option<String>,
    /// Maximum token budget for this task (input + output). When `None`,
    /// uses the model's default context window.
    pub max_tokens: Option<u32>,
    /// Whether this override is active. Disabled overrides are persisted
    /// but not used at runtime.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Agent-Role Routing (Chunk 35.3)
// ---------------------------------------------------------------------------

/// Per-agent-role routing configuration. Specifies preferred model class,
/// token budget, and fallback provider chain for a specific role.
///
/// Resolution order when a `WorkflowStep` resolves its provider:
/// 1. If an `AgentRouteConfig` exists for the step's `AgentRole`, use it.
/// 2. Within the config, try `preferred_provider` first; if rate-limited or
///    unhealthy, try `fallback_providers` in order.
/// 3. Apply `max_tokens` as a token-cap constraint in the rotator.
/// 4. If nothing qualifies, fall through to `resolve_for_task(role_task_kind)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentRouteConfig {
    /// Which agent role this config applies to.
    pub role: AgentRole,
    /// Preferred model tier (Fast, Balanced, Premium).
    /// Used to pick from `AgentRole::recommended_llms()` when no explicit
    /// model is configured.
    #[serde(default = "default_tier")]
    pub preferred_tier: AgentTier,
    /// Explicit preferred provider id (e.g. "ollama", "anthropic", "groq").
    /// When set, this provider is tried first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_provider: Option<String>,
    /// Explicit model override (e.g. "claude-sonnet-4-20250514", "gemma3:4b").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_model: Option<String>,
    /// Ordered fallback provider ids. Tried in sequence if the preferred
    /// provider is unavailable (rate-limited/unhealthy).
    #[serde(default)]
    pub fallback_providers: Vec<String>,
    /// Maximum token budget for this role. Applied as a token-cap constraint
    /// during provider selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Whether this route config is active. Disabled configs are ignored.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Agent quality/speed tier for model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTier {
    /// Fast and cheap — local small models.
    Fast,
    /// Balanced — medium-size local or free cloud.
    Balanced,
    /// Premium — largest/best models (may be paid cloud).
    Premium,
}

fn default_tier() -> AgentTier {
    AgentTier::Balanced
}

/// Maps an `AgentRole` to the `TaskKind` it primarily performs.
/// Used as fallback when no explicit agent routing is configured.
pub fn role_to_task_kind(role: AgentRole) -> TaskKind {
    match role {
        AgentRole::Planner => TaskKind::Chat,
        AgentRole::Coder => TaskKind::CodeReview,
        AgentRole::Reviewer => TaskKind::CodeReview,
        AgentRole::Tester => TaskKind::CodeReview,
        AgentRole::Researcher => TaskKind::LongContext,
        AgentRole::Orchestrator => TaskKind::Chat,
    }
}

/// Resolved provider selection for an agent role, including which fallback
/// was used and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedAgentProvider {
    /// The agent role this was resolved for.
    pub role: AgentRole,
    /// How it was resolved: "agent_route", "task_override", "brain_mode".
    pub source: String,
    /// Provider id.
    pub provider_id: String,
    /// Model to use.
    pub model: String,
    /// Endpoint URL.
    pub base_url: String,
    /// API key (may be empty for local).
    pub api_key: String,
    /// Max token budget (if set).
    pub max_tokens: Option<u32>,
    /// If a fallback was used, the preferred provider that was skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_from: Option<String>,
}

/// The app-wide provider policy registry. Maps `TaskKind` → optional
/// per-task override. Tasks without an override use the active `BrainMode`.
///
/// Also holds **agent routing** (Chunk 35.3): per-`AgentRole` preferences
/// that specify model tier, token budget, and fallback provider chains.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderPolicy {
    /// Schema version for forward compatibility.
    #[serde(default = "default_version")]
    pub version: u32,
    /// Per-task overrides keyed by `TaskKind`. Only tasks with explicit
    /// overrides appear here; missing tasks use the default brain mode.
    #[serde(default)]
    pub overrides: HashMap<TaskKind, TaskOverride>,
    /// Per-agent-role routing configuration (Chunk 35.3).
    /// When a workflow step runs, the system first checks here for
    /// role-specific preferences before falling through to task-kind policy.
    #[serde(default)]
    pub agent_routing: HashMap<AgentRole, AgentRouteConfig>,
}

fn default_version() -> u32 {
    1
}

impl Default for ProviderPolicy {
    fn default() -> Self {
        Self {
            version: 1,
            overrides: HashMap::new(),
            agent_routing: HashMap::new(),
        }
    }
}

impl ProviderPolicy {
    /// Get the override for a specific task, or `None` if no active
    /// override is configured.
    pub fn get(&self, kind: TaskKind) -> Option<&TaskOverride> {
        self.overrides.get(&kind).filter(|o| o.enabled)
    }

    /// Set (or replace) the override for a task kind.
    pub fn set(&mut self, ovr: TaskOverride) {
        self.overrides.insert(ovr.kind, ovr);
    }

    /// Remove the override for a task kind (reverts to default brain mode).
    pub fn remove(&mut self, kind: TaskKind) -> Option<TaskOverride> {
        self.overrides.remove(&kind)
    }

    /// Returns `true` if any active override exists for any task.
    pub fn has_overrides(&self) -> bool {
        self.overrides.values().any(|o| o.enabled)
    }

    /// List all configured overrides (including disabled ones).
    pub fn all_overrides(&self) -> Vec<&TaskOverride> {
        self.overrides.values().collect()
    }

    // -----------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------

    fn file_path(data_dir: &Path) -> PathBuf {
        data_dir.join("provider_policy.json")
    }

    /// Load from disk. Returns `Default` if file doesn't exist or is invalid.
    pub fn load(data_dir: &Path) -> Self {
        let path = Self::file_path(data_dir);
        match std::fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save atomically (write then rename is not easy on Windows, so we
    /// write directly — acceptable since this is a small config file).
    pub fn save(&self, data_dir: &Path) -> Result<(), String> {
        let path = Self::file_path(data_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }

    // -----------------------------------------------------------------
    // Agent Routing (Chunk 35.3)
    // -----------------------------------------------------------------

    /// Get the agent route config for a specific role, if active.
    pub fn get_agent_route(&self, role: AgentRole) -> Option<&AgentRouteConfig> {
        self.agent_routing.get(&role).filter(|c| c.enabled)
    }

    /// Set (or replace) the agent route config for a role.
    pub fn set_agent_route(&mut self, config: AgentRouteConfig) {
        self.agent_routing.insert(config.role, config);
    }

    /// Remove the agent route for a role (reverts to default task-kind resolution).
    pub fn remove_agent_route(&mut self, role: AgentRole) -> Option<AgentRouteConfig> {
        self.agent_routing.remove(&role)
    }

    /// All configured agent routes (including disabled).
    pub fn all_agent_routes(&self) -> Vec<&AgentRouteConfig> {
        self.agent_routing.values().collect()
    }
}

/// Resolve the effective provider + model for a task, given the policy and
/// the current brain mode. Returns `(provider_id, model, base_url, api_key)`.
///
/// Resolution order:
/// 1. If an active override exists → use it (filling missing fields from brain_mode).
/// 2. Else → derive from the active `BrainMode` using task-appropriate defaults.
pub fn resolve_for_task(
    policy: &ProviderPolicy,
    kind: TaskKind,
    brain_mode: Option<&super::BrainMode>,
) -> ResolvedProvider {
    if let Some(ovr) = policy.get(kind) {
        return resolve_from_override(ovr, brain_mode);
    }
    resolve_from_brain_mode(kind, brain_mode)
}

/// The fully resolved provider selection for a single task invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedProvider {
    /// Human-readable source (e.g. "override:embeddings" or "brain_mode:paid_api")
    pub source: String,
    /// Provider id (e.g. "openai", "ollama", "groq")
    pub provider_id: String,
    /// Model to use.
    pub model: String,
    /// Endpoint URL.
    pub base_url: String,
    /// API key (may be empty for local providers).
    pub api_key: String,
    /// Optional max token budget.
    pub max_tokens: Option<u32>,
}

fn resolve_from_override(
    ovr: &TaskOverride,
    brain_mode: Option<&super::BrainMode>,
) -> ResolvedProvider {
    // Start with brain_mode as the base, then layer override fields on top.
    let base = resolve_from_brain_mode(ovr.kind, brain_mode);
    ResolvedProvider {
        source: format!("override:{}", serde_variant_name(ovr.kind)),
        provider_id: ovr.provider_id.clone().unwrap_or(base.provider_id),
        model: ovr.model.clone().unwrap_or(base.model),
        base_url: ovr.base_url.clone().unwrap_or(base.base_url),
        api_key: ovr.api_key.clone().unwrap_or(base.api_key),
        max_tokens: ovr.max_tokens.or(base.max_tokens),
    }
}

fn resolve_from_brain_mode(
    kind: TaskKind,
    brain_mode: Option<&super::BrainMode>,
) -> ResolvedProvider {
    match brain_mode {
        Some(super::BrainMode::PaidApi {
            provider,
            api_key,
            model,
            base_url,
        }) => {
            let effective_model = task_default_model_paid(kind, provider, model);
            ResolvedProvider {
                source: "brain_mode:paid_api".to_string(),
                provider_id: provider.clone(),
                model: effective_model.to_string(),
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                max_tokens: None,
            }
        }
        Some(super::BrainMode::FreeApi {
            provider_id,
            api_key,
            model,
        }) => {
            let provider = super::get_free_provider(provider_id);
            let (effective_base_url, effective_model) = match &provider {
                Some(p) => (
                    p.base_url.clone(),
                    task_default_model_free(kind, provider_id, model.as_deref(), &p.model),
                ),
                None => (
                    "https://api.openai.com/v1".to_string(),
                    model.clone().unwrap_or_else(|| "gpt-4o-mini".to_string()),
                ),
            };
            ResolvedProvider {
                source: "brain_mode:free_api".to_string(),
                provider_id: provider_id.clone(),
                model: effective_model,
                base_url: effective_base_url,
                api_key: api_key.clone().unwrap_or_default(),
                max_tokens: None,
            }
        }
        Some(super::BrainMode::LocalOllama { model }) => {
            let effective_model = task_default_model_ollama(kind, model);
            ResolvedProvider {
                source: "brain_mode:local_ollama".to_string(),
                provider_id: "ollama".to_string(),
                model: effective_model.to_string(),
                base_url: "http://localhost:11434".to_string(),
                api_key: String::new(),
                max_tokens: None,
            }
        }
        Some(super::BrainMode::LocalLmStudio {
            model,
            base_url,
            api_key,
            embedding_model,
        }) => {
            let effective_model = match kind {
                TaskKind::Embeddings => embedding_model.as_deref().unwrap_or(model).to_string(),
                _ => model.clone(),
            };
            ResolvedProvider {
                source: "brain_mode:local_lm_studio".to_string(),
                provider_id: "lm-studio".to_string(),
                model: effective_model,
                base_url: base_url.clone(),
                api_key: api_key.clone().unwrap_or_default(),
                max_tokens: None,
            }
        }
        None => ResolvedProvider {
            source: "none".to_string(),
            provider_id: "none".to_string(),
            model: String::new(),
            base_url: String::new(),
            api_key: String::new(),
            max_tokens: None,
        },
    }
}

/// For paid APIs, select the appropriate model for each task.
fn task_default_model_paid<'a>(kind: TaskKind, provider: &str, chat_model: &'a str) -> &'a str {
    match kind {
        TaskKind::Chat | TaskKind::CodeReview | TaskKind::LongContext => chat_model,
        TaskKind::Embeddings => match provider {
            "anthropic" => "voyage-3-lite",
            "mistral" => "mistral-embed",
            _ => "text-embedding-3-small",
        },
        TaskKind::Rerank | TaskKind::Summarise => chat_model,
    }
}

/// For free APIs, select the appropriate model for each task.
fn task_default_model_free(
    kind: TaskKind,
    provider_id: &str,
    configured_model: Option<&str>,
    provider_default: &str,
) -> String {
    match kind {
        TaskKind::Chat
        | TaskKind::CodeReview
        | TaskKind::LongContext
        | TaskKind::Rerank
        | TaskKind::Summarise => configured_model.unwrap_or(provider_default).to_string(),
        TaskKind::Embeddings => {
            // Only some free providers have embed endpoints
            match provider_id {
                "mistral" => "mistral-embed".to_string(),
                "github-models" => "text-embedding-3-small".to_string(),
                "siliconflow" => "BAAI/bge-m3".to_string(),
                "nvidia-nim" => "nvidia/nv-embedqa-e5-v5".to_string(),
                _ => configured_model.unwrap_or(provider_default).to_string(),
            }
        }
    }
}

/// For local Ollama, select the appropriate model for each task.
///
/// `mxbai-embed-large` is the default embedder per BENCH-LCM-5
/// (2026-05-12): +3.7pp R@10 overall on LoCoMo vs `nomic-embed-text`.
/// Users can override via the embedding-model picker; `nomic-embed-text`
/// remains in `EMBED_MODEL_FALLBACKS` as the lightweight 768d fallback.
fn task_default_model_ollama(kind: TaskKind, chat_model: &str) -> &str {
    match kind {
        TaskKind::Embeddings => "mxbai-embed-large",
        _ => chat_model,
    }
}

fn serde_variant_name(kind: TaskKind) -> &'static str {
    match kind {
        TaskKind::Chat => "chat",
        TaskKind::Embeddings => "embeddings",
        TaskKind::Rerank => "rerank",
        TaskKind::Summarise => "summarise",
        TaskKind::CodeReview => "code_review",
        TaskKind::LongContext => "long_context",
    }
}

// ---------------------------------------------------------------------------
// Agent-Role Resolution (Chunk 35.3)
// ---------------------------------------------------------------------------

/// Resolve the effective provider for an agent role in a workflow step.
///
/// Resolution cascade:
/// 1. Check `policy.agent_routing[role]` — if present and enabled, use its
///    preferred provider/model. If preferred is unavailable, try fallbacks.
/// 2. Fall through to `resolve_for_task(role_to_task_kind(role))`.
/// 3. If still nothing, derive from brain_mode.
///
/// The `provider_healthy` callback lets callers integrate with the
/// `ProviderRotator` to check rate-limit / health state without passing
/// the whole rotator in.
pub fn resolve_for_agent_role(
    policy: &ProviderPolicy,
    role: AgentRole,
    brain_mode: Option<&super::BrainMode>,
    provider_healthy: impl Fn(&str) -> bool,
) -> ResolvedAgentProvider {
    // 1. Check agent routing config
    if let Some(route) = policy.get_agent_route(role) {
        // Try preferred provider first
        if let Some(ref pref_provider) = route.preferred_provider {
            if provider_healthy(pref_provider) {
                let model = route
                    .preferred_model
                    .clone()
                    .unwrap_or_else(|| model_for_tier(role, route.preferred_tier));
                let (base_url, api_key) = derive_provider_endpoint(pref_provider, brain_mode);
                return ResolvedAgentProvider {
                    role,
                    source: "agent_route:preferred".to_string(),
                    provider_id: pref_provider.clone(),
                    model,
                    base_url,
                    api_key,
                    max_tokens: route.max_tokens,
                    fallback_from: None,
                };
            }

            // Preferred is unavailable — try fallback chain
            for fallback_id in &route.fallback_providers {
                if provider_healthy(fallback_id) {
                    let model = route
                        .preferred_model
                        .clone()
                        .unwrap_or_else(|| model_for_tier(role, route.preferred_tier));
                    let (base_url, api_key) = derive_provider_endpoint(fallback_id, brain_mode);
                    return ResolvedAgentProvider {
                        role,
                        source: "agent_route:fallback".to_string(),
                        provider_id: fallback_id.clone(),
                        model,
                        base_url,
                        api_key,
                        max_tokens: route.max_tokens,
                        fallback_from: Some(pref_provider.clone()),
                    };
                }
            }
        } else {
            // No preferred provider but tier/model set — resolve via tier
            let model = route
                .preferred_model
                .clone()
                .unwrap_or_else(|| model_for_tier(role, route.preferred_tier));
            let task_resolved = resolve_for_task(policy, role_to_task_kind(role), brain_mode);
            return ResolvedAgentProvider {
                role,
                source: "agent_route:tier".to_string(),
                provider_id: task_resolved.provider_id,
                model,
                base_url: task_resolved.base_url,
                api_key: task_resolved.api_key,
                max_tokens: route.max_tokens.or(task_resolved.max_tokens),
                fallback_from: None,
            };
        }
    }

    // 2. Fall through to task-kind resolution
    let task_kind = role_to_task_kind(role);
    let resolved = resolve_for_task(policy, task_kind, brain_mode);
    ResolvedAgentProvider {
        role,
        source: resolved.source,
        provider_id: resolved.provider_id,
        model: resolved.model,
        base_url: resolved.base_url,
        api_key: resolved.api_key,
        max_tokens: resolved.max_tokens,
        fallback_from: None,
    }
}

/// Pick a default model name based on the role's recommended LLMs at the given tier.
fn model_for_tier(role: AgentRole, tier: AgentTier) -> String {
    use crate::coding::multi_agent::LlmTier;
    let target_tier = match tier {
        AgentTier::Fast => LlmTier::Fast,
        AgentTier::Balanced => LlmTier::Balanced,
        AgentTier::Premium => LlmTier::Premium,
    };
    role.recommended_llms()
        .iter()
        .find(|r| r.tier == target_tier)
        .map(|r| r.model.to_string())
        .unwrap_or_else(|| "gemma-3:4b".to_string())
}

/// Derive base_url and api_key for a provider id by checking brain_mode.
fn derive_provider_endpoint(
    provider_id: &str,
    brain_mode: Option<&super::BrainMode>,
) -> (String, String) {
    match provider_id {
        "ollama" => ("http://localhost:11434".to_string(), String::new()),
        "lm-studio" => {
            if let Some(super::BrainMode::LocalLmStudio {
                base_url, api_key, ..
            }) = brain_mode
            {
                (base_url.clone(), api_key.clone().unwrap_or_default())
            } else {
                ("http://localhost:1234/v1".to_string(), String::new())
            }
        }
        _ => {
            // Check if it's a free provider
            if let Some(fp) = super::get_free_provider(provider_id) {
                (fp.base_url, String::new())
            } else if let Some(super::BrainMode::PaidApi {
                base_url, api_key, ..
            }) = brain_mode
            {
                (base_url.clone(), api_key.clone())
            } else {
                ("https://api.openai.com/v1".to_string(), String::new())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_has_no_overrides() {
        let policy = ProviderPolicy::default();
        assert!(!policy.has_overrides());
        assert!(policy.get(TaskKind::Chat).is_none());
    }

    #[test]
    fn set_and_get_override() {
        let mut policy = ProviderPolicy::default();
        policy.set(TaskOverride {
            kind: TaskKind::Embeddings,
            provider_id: Some("openai".to_string()),
            model: Some("text-embedding-3-large".to_string()),
            base_url: Some("https://api.openai.com/v1".to_string()),
            api_key: Some("sk-test".to_string()),
            max_tokens: None,
            enabled: true,
        });
        assert!(policy.has_overrides());
        let ovr = policy.get(TaskKind::Embeddings).unwrap();
        assert_eq!(ovr.model.as_deref(), Some("text-embedding-3-large"));
    }

    #[test]
    fn disabled_override_not_returned_by_get() {
        let mut policy = ProviderPolicy::default();
        policy.set(TaskOverride {
            kind: TaskKind::Rerank,
            provider_id: Some("groq".to_string()),
            model: Some("llama-3.1-8b-instant".to_string()),
            base_url: None,
            api_key: None,
            max_tokens: None,
            enabled: false,
        });
        assert!(policy.get(TaskKind::Rerank).is_none());
        assert!(!policy.has_overrides());
    }

    #[test]
    fn remove_override() {
        let mut policy = ProviderPolicy::default();
        policy.set(TaskOverride {
            kind: TaskKind::Chat,
            provider_id: Some("anthropic".to_string()),
            model: Some("claude-sonnet-4-20250514".to_string()),
            base_url: None,
            api_key: None,
            max_tokens: Some(8192),
            enabled: true,
        });
        assert!(policy.has_overrides());
        policy.remove(TaskKind::Chat);
        assert!(!policy.has_overrides());
    }

    #[test]
    fn resolve_falls_back_to_brain_mode_when_no_override() {
        let policy = ProviderPolicy::default();
        let brain_mode = super::super::BrainMode::LocalOllama {
            model: "gemma3:4b".to_string(),
        };
        let resolved = resolve_for_task(&policy, TaskKind::Chat, Some(&brain_mode));
        assert_eq!(resolved.provider_id, "ollama");
        assert_eq!(resolved.model, "gemma3:4b");
        assert_eq!(resolved.source, "brain_mode:local_ollama");
    }

    #[test]
    fn resolve_uses_override_when_present() {
        let mut policy = ProviderPolicy::default();
        policy.set(TaskOverride {
            kind: TaskKind::Embeddings,
            provider_id: Some("openai".to_string()),
            model: Some("text-embedding-3-large".to_string()),
            base_url: Some("https://api.openai.com/v1".to_string()),
            api_key: Some("sk-key".to_string()),
            max_tokens: None,
            enabled: true,
        });
        let brain_mode = super::super::BrainMode::LocalOllama {
            model: "gemma3:4b".to_string(),
        };
        let resolved = resolve_for_task(&policy, TaskKind::Embeddings, Some(&brain_mode));
        assert_eq!(resolved.provider_id, "openai");
        assert_eq!(resolved.model, "text-embedding-3-large");
        assert!(resolved.source.starts_with("override:"));
    }

    #[test]
    fn resolve_merges_partial_override_with_brain_mode() {
        let mut policy = ProviderPolicy::default();
        // Override only the model, keeping provider from brain_mode
        policy.set(TaskOverride {
            kind: TaskKind::Rerank,
            provider_id: None,
            model: Some("gpt-4o-mini".to_string()),
            base_url: None,
            api_key: None,
            max_tokens: Some(4096),
            enabled: true,
        });
        let brain_mode = super::super::BrainMode::PaidApi {
            provider: "openai".to_string(),
            api_key: "sk-prod".to_string(),
            model: "gpt-4o".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        };
        let resolved = resolve_for_task(&policy, TaskKind::Rerank, Some(&brain_mode));
        assert_eq!(resolved.provider_id, "openai"); // from brain_mode
        assert_eq!(resolved.model, "gpt-4o-mini"); // from override
        assert_eq!(resolved.api_key, "sk-prod"); // from brain_mode
        assert_eq!(resolved.max_tokens, Some(4096)); // from override
    }

    #[test]
    fn ollama_embeddings_defaults_to_mxbai() {
        // BENCH-LCM-5 (2026-05-12): mxbai-embed-large promoted as the
        // default local Ollama embedder (+3.7pp R@10 overall on LoCoMo
        // vs nomic-embed-text). nomic remains in EMBED_MODEL_FALLBACKS.
        let policy = ProviderPolicy::default();
        let brain_mode = super::super::BrainMode::LocalOllama {
            model: "gemma3:4b".to_string(),
        };
        let resolved = resolve_for_task(&policy, TaskKind::Embeddings, Some(&brain_mode));
        assert_eq!(resolved.model, "mxbai-embed-large");
    }

    #[test]
    fn paid_api_embeddings_defaults_correctly() {
        let policy = ProviderPolicy::default();
        let brain_mode = super::super::BrainMode::PaidApi {
            provider: "anthropic".to_string(),
            api_key: "sk-ant".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
        };
        let resolved = resolve_for_task(&policy, TaskKind::Embeddings, Some(&brain_mode));
        assert_eq!(resolved.model, "voyage-3-lite");
    }

    #[test]
    fn serde_roundtrip() {
        let mut policy = ProviderPolicy::default();
        policy.set(TaskOverride {
            kind: TaskKind::Summarise,
            provider_id: Some("groq".to_string()),
            model: Some("llama-3.1-70b-versatile".to_string()),
            base_url: None,
            api_key: Some("gsk-test".to_string()),
            max_tokens: Some(16384),
            enabled: true,
        });
        let json = serde_json::to_string(&policy).unwrap();
        let parsed: ProviderPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, parsed);
    }

    // ── Agent-Role Routing Tests (Chunk 35.3) ───────────────────────

    #[test]
    fn role_to_task_kind_maps_correctly() {
        assert_eq!(role_to_task_kind(AgentRole::Planner), TaskKind::Chat);
        assert_eq!(role_to_task_kind(AgentRole::Coder), TaskKind::CodeReview);
        assert_eq!(role_to_task_kind(AgentRole::Reviewer), TaskKind::CodeReview);
        assert_eq!(
            role_to_task_kind(AgentRole::Researcher),
            TaskKind::LongContext
        );
    }

    #[test]
    fn agent_route_config_set_and_get() {
        let mut policy = ProviderPolicy::default();
        policy.set_agent_route(AgentRouteConfig {
            role: AgentRole::Coder,
            preferred_tier: AgentTier::Premium,
            preferred_provider: Some("anthropic".to_string()),
            preferred_model: Some("claude-sonnet-4-20250514".to_string()),
            fallback_providers: vec!["groq".to_string()],
            max_tokens: Some(8192),
            enabled: true,
        });
        let route = policy.get_agent_route(AgentRole::Coder).unwrap();
        assert_eq!(route.preferred_provider.as_deref(), Some("anthropic"));
        assert_eq!(route.preferred_tier, AgentTier::Premium);
    }

    #[test]
    fn disabled_agent_route_not_returned() {
        let mut policy = ProviderPolicy::default();
        policy.set_agent_route(AgentRouteConfig {
            role: AgentRole::Planner,
            preferred_tier: AgentTier::Fast,
            preferred_provider: Some("ollama".to_string()),
            preferred_model: None,
            fallback_providers: vec![],
            max_tokens: None,
            enabled: false,
        });
        assert!(policy.get_agent_route(AgentRole::Planner).is_none());
    }

    #[test]
    fn resolve_for_agent_role_uses_preferred_provider() {
        let mut policy = ProviderPolicy::default();
        policy.set_agent_route(AgentRouteConfig {
            role: AgentRole::Coder,
            preferred_tier: AgentTier::Balanced,
            preferred_provider: Some("ollama".to_string()),
            preferred_model: Some("qwen2.5-coder:7b".to_string()),
            fallback_providers: vec![],
            max_tokens: Some(4096),
            enabled: true,
        });
        let brain_mode = super::super::BrainMode::LocalOllama {
            model: "gemma3:4b".to_string(),
        };
        let resolved = resolve_for_agent_role(
            &policy,
            AgentRole::Coder,
            Some(&brain_mode),
            |_| true, // all healthy
        );
        assert_eq!(resolved.provider_id, "ollama");
        assert_eq!(resolved.model, "qwen2.5-coder:7b");
        assert_eq!(resolved.source, "agent_route:preferred");
        assert_eq!(resolved.max_tokens, Some(4096));
    }

    #[test]
    fn resolve_for_agent_role_uses_fallback_when_preferred_unhealthy() {
        let mut policy = ProviderPolicy::default();
        policy.set_agent_route(AgentRouteConfig {
            role: AgentRole::Reviewer,
            preferred_tier: AgentTier::Premium,
            preferred_provider: Some("anthropic".to_string()),
            preferred_model: Some("claude-sonnet-4-20250514".to_string()),
            fallback_providers: vec!["groq".to_string(), "cerebras".to_string()],
            max_tokens: None,
            enabled: true,
        });
        let brain_mode = super::super::BrainMode::LocalOllama {
            model: "gemma3:4b".to_string(),
        };
        let resolved = resolve_for_agent_role(
            &policy,
            AgentRole::Reviewer,
            Some(&brain_mode),
            |id| id != "anthropic", // anthropic is unhealthy
        );
        assert_eq!(resolved.provider_id, "groq");
        assert_eq!(resolved.source, "agent_route:fallback");
        assert_eq!(resolved.fallback_from.as_deref(), Some("anthropic"));
    }

    #[test]
    fn resolve_for_agent_role_falls_through_to_task_kind() {
        // No agent routing configured — should fall through to task-kind resolution
        let policy = ProviderPolicy::default();
        let brain_mode = super::super::BrainMode::LocalOllama {
            model: "gemma3:4b".to_string(),
        };
        let resolved =
            resolve_for_agent_role(&policy, AgentRole::Planner, Some(&brain_mode), |_| true);
        // Planner maps to TaskKind::Chat, which uses brain_mode model
        assert_eq!(resolved.provider_id, "ollama");
        assert_eq!(resolved.model, "gemma3:4b");
        assert_eq!(resolved.source, "brain_mode:local_ollama");
    }

    #[test]
    fn resolve_for_agent_role_tier_only_config() {
        let mut policy = ProviderPolicy::default();
        policy.set_agent_route(AgentRouteConfig {
            role: AgentRole::Researcher,
            preferred_tier: AgentTier::Fast,
            preferred_provider: None, // no explicit provider
            preferred_model: None,    // derive from tier
            fallback_providers: vec![],
            max_tokens: Some(2048),
            enabled: true,
        });
        let brain_mode = super::super::BrainMode::LocalOllama {
            model: "gemma3:4b".to_string(),
        };
        let resolved =
            resolve_for_agent_role(&policy, AgentRole::Researcher, Some(&brain_mode), |_| true);
        assert_eq!(resolved.source, "agent_route:tier");
        // Fast tier for Researcher should be "gemma-3:4b"
        assert_eq!(resolved.model, "gemma-3:4b");
        assert_eq!(resolved.max_tokens, Some(2048));
    }

    #[test]
    fn agent_route_serde_roundtrip() {
        let mut policy = ProviderPolicy::default();
        policy.set_agent_route(AgentRouteConfig {
            role: AgentRole::Coder,
            preferred_tier: AgentTier::Premium,
            preferred_provider: Some("anthropic".to_string()),
            preferred_model: Some("claude-sonnet-4-20250514".to_string()),
            fallback_providers: vec!["groq".to_string()],
            max_tokens: Some(16384),
            enabled: true,
        });
        let json = serde_json::to_string(&policy).unwrap();
        let parsed: ProviderPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, parsed);
    }
}
