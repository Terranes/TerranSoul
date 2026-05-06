//! Auto-configuration of the brain for MCP mode.
//!
//! When MCP starts with no existing `brain_config.json`, this module
//! detects/starts Ollama, picks the best model for the hardware, ensures
//! an embedding model is available, and persists the configuration. This
//! enables zero-config MCP brain usage for AI coding agents — search,
//! ingest, RAG, HyDE, reranking all work immediately without manual setup.

use std::path::Path;

use super::brain_config::{self, BrainMode};
use super::model_recommender;
use super::ollama_agent::{self, OLLAMA_BASE_URL};
use super::ollama_lifecycle;
use super::system_info;

/// The embedding model used for vector search in local Ollama mode.
const EMBEDDING_MODEL: &str = "nomic-embed-text";

/// Auto-configure the brain for MCP mode if no config exists yet.
///
/// Decision cascade:
/// 1. If `brain_config.json` already exists in `data_dir` → do nothing (user configured).
/// 2. Detect Ollama → if installed but not running, start it.
/// 3. If Ollama is running:
///    a. List local models → pick the best one already pulled.
///    b. If no suitable model, pull the top recommendation for this hardware.
///    c. Ensure the embedding model (`nomic-embed-text`) is available; pull if missing.
///    d. Save `BrainMode::LocalOllama { model }` to disk.
/// 4. If Ollama is NOT available (not installed, can't start):
///    a. Fall back to Pollinations free API (no key needed, always available).
///    b. Save `BrainMode::FreeApi { provider_id: "pollinations" }`.
///
/// This runs at startup in both `run_http_server()` and the Tauri MCP app mode.
/// It's idempotent — once the config exists on disk, subsequent runs skip it.
/// However, it ALWAYS checks whether the top hardware recommendation is
/// pulled and will spawn a background pull if not — so the user automatically
/// upgrades to a better model after enough restarts.
pub async fn auto_configure_mcp_brain(data_dir: &Path) {
    // Skip if already configured with a proper provider (not free API)
    if let Some(ref mode) = brain_config::load(data_dir) {
        match mode {
            BrainMode::FreeApi { .. } => {
                eprintln!("[mcp-brain] existing config is free API — reconfiguring with recommended setup");
                // Delete the stale free config so we don't fall back to it
                let _ = brain_config::clear(data_dir);
            }
            BrainMode::LocalOllama { model } => {
                // Even if configured, check if we should upgrade to a
                // better model for this hardware.
                maybe_background_upgrade_model(model).await;
                eprintln!("[mcp-brain] brain already configured ({model}), skipping auto-config");
                return;
            }
            _ => {
                eprintln!("[mcp-brain] brain already configured, skipping auto-config");
                return;
            }
        }
    }

    eprintln!("[mcp-brain] no brain config found — running auto-configuration…");

    // Try local Ollama first (privacy, speed, no cost)
    if let Some(mode) = try_configure_ollama(data_dir).await {
        if let Err(e) = brain_config::save(data_dir, &mode) {
            eprintln!("[mcp-brain] failed to save brain config: {e}");
        } else {
            let model = match &mode {
                BrainMode::LocalOllama { model } => model.as_str(),
                _ => "unknown",
            };
            eprintln!("[mcp-brain] ✓ configured local Ollama with model: {model}");
        }
        return;
    }

    // No Ollama available — leave brain unconfigured so the UI prompts
    // the user to set up an LLM provider. MCP mode should NOT silently
    // fall back to a free API; the connecting coding agent IS the LLM.
    eprintln!("[mcp-brain] Ollama not available — brain unconfigured, UI will prompt for setup");
    eprintln!(
        "[mcp-brain] hint: install Ollama (https://ollama.com) or configure a paid API in the UI"
    );
}

/// If the currently configured model is inferior to the hardware's top
/// recommendation and the better model isn't pulled yet, start a
/// background pull so the next restart picks it up automatically.
/// On successful pull, updates the config file immediately so a restart
/// will use the upgraded model.
async fn maybe_background_upgrade_model(current_model: &str) {
    let sys = super::system_info::collect();
    let recommendations = super::model_recommender::recommend(sys.total_ram_mb);
    let top_pick = recommendations
        .iter()
        .find(|r| !r.is_cloud && r.is_top_pick);

    let Some(top) = top_pick else { return };

    if top.model_tag == current_model {
        // Already running the best model for this hardware.
        return;
    }

    // Check if the top pick is already pulled locally.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();
    let local_models = super::ollama_agent::list_models(&client, OLLAMA_BASE_URL).await;
    let top_tag = &top.model_tag;
    let already_local = local_models.iter().any(|m| {
        m.name == *top_tag
            || m.name
                .starts_with(&format!("{}:", top_tag.split(':').next().unwrap_or("")))
                && m.name.contains(top_tag.split(':').nth(1).unwrap_or(""))
    });

    if already_local {
        // Top pick is pulled but not configured — user may have
        // downgraded intentionally. Log but don't force-switch.
        eprintln!(
            "[mcp-brain] note: better model '{top_tag}' is available locally \
             but you're using '{current_model}'. Restart with cleared config to upgrade."
        );
        return;
    }

    // Spawn a non-blocking background pull.
    let pull_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let tag = top_tag.clone();
    let current_owned = current_model.to_string();
    tokio::spawn(async move {
        eprintln!(
            "[mcp-brain] background upgrade pull starting: {tag} (better than {current_owned})",
        );
        match super::ollama_agent::pull_model(&pull_client, OLLAMA_BASE_URL, &tag).await {
            Ok(_) => eprintln!(
                "[mcp-brain] ✓ background pull complete: {tag} — restart MCP to auto-switch"
            ),
            Err(e) => eprintln!("[mcp-brain] background pull failed for {tag}: {e}"),
        }
    });
}

/// Attempt Ollama-based configuration. Returns `Some(BrainMode)` on success.
/// If Ollama is not installed, auto-installs it using the existing
/// `install_ollama` feature (silent install on Windows/Linux).
async fn try_configure_ollama(data_dir: &Path) -> Option<BrainMode> {
    let mut status = ollama_lifecycle::detect_ollama().await;

    if !status.installed {
        eprintln!("[mcp-brain] Ollama not installed — auto-installing recommended setup…");
        match ollama_lifecycle::install_ollama(|phase, pct| {
            eprintln!("[mcp-brain] install: [{pct:>3}%] {phase}");
        })
        .await
        {
            Ok(msg) => {
                eprintln!("[mcp-brain] ✓ {msg}");
                // Re-detect after install
                status = ollama_lifecycle::detect_ollama().await;
                if !status.installed {
                    eprintln!("[mcp-brain] Ollama still not found after install");
                    return None;
                }
            }
            Err(e) => {
                eprintln!("[mcp-brain] auto-install failed: {e}");
                eprintln!("[mcp-brain] hint: install manually from https://ollama.com");
                return None;
            }
        }
    }

    // Start Ollama if it's installed but not running
    if !status.running {
        eprintln!("[mcp-brain] Ollama installed but not running — starting…");
        match ollama_lifecycle::start_ollama(15).await {
            Ok(true) => eprintln!("[mcp-brain] Ollama started successfully"),
            Ok(false) => {
                eprintln!("[mcp-brain] Ollama failed to start within 15s");
                return None;
            }
            Err(e) => {
                eprintln!("[mcp-brain] Ollama start error: {e}");
                return None;
            }
        }
    }

    // Build reqwest clients for Ollama operations.
    //
    // - `client` is used for fast metadata calls (list models, manifest
    //   probes). 120 s is generous.
    // - `pull_client` is used for `/api/pull`, which streams a multi-GB
    //   download. A total-request timeout would silently cancel large
    //   pulls (e.g. gemma4:e4b is 9.6 GB) — we instead set
    //   `connect_timeout` so a dead Ollama still fails fast, but the
    //   stream can run as long as it needs.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_default();
    let pull_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();

    // List already-pulled models
    let local_models = ollama_agent::list_models(&client, OLLAMA_BASE_URL).await;
    let local_tags: Vec<&str> = local_models.iter().map(|m| m.name.as_str()).collect();
    eprintln!(
        "[mcp-brain] found {} local models: {:?}",
        local_tags.len(),
        &local_tags[..local_tags.len().min(10)]
    );

    // Get hardware-appropriate recommendations
    let sys = system_info::collect();
    let recommendations = model_recommender::recommend(sys.total_ram_mb);

    // Prefer the top pick (best for this hardware). If it's not already
    // pulled, attempt to pull it so users on capable hardware get the
    // recommended model instead of an inferior already-cached one.
    // Falls back to the best already-pulled model if the pull fails
    // (offline, Ollama down, disk full, etc.).
    let top_pick = recommendations
        .iter()
        .find(|r| !r.is_cloud)
        .map(|r| r.model_tag.clone());

    let top_already_local = top_pick
        .as_ref()
        .map(|tag| {
            local_tags.iter().any(|t| {
                *t == tag
                    || t.starts_with(&format!("{}:", tag.split(':').next().unwrap_or("")))
                        && t.contains(tag.split(':').nth(1).unwrap_or(""))
            })
        })
        .unwrap_or(false);

    let chosen_model: Option<String> = if top_already_local {
        eprintln!(
            "[mcp-brain] top recommendation already pulled: {}",
            top_pick.as_deref().unwrap_or("?")
        );
        top_pick.clone()
    } else {
        // Top pick is not local. Boot MCP immediately with the best
        // already-pulled model so the user is never blocked on a
        // multi-GB download, then kick off the recommended pull in the
        // background. The next MCP restart will pick up the upgraded
        // model automatically.
        let immediate = pick_best_local_model(&local_tags, &recommendations)
            .or_else(|| pick_any_local_chat_model(&local_tags));

        if let Some(top) = top_pick.clone() {
            let pc = pull_client.clone();
            tokio::spawn(async move {
                eprintln!("[mcp-brain] background pull starting for top recommendation: {top}");
                match ollama_agent::pull_model(&pc, OLLAMA_BASE_URL, &top).await {
                    Ok(_) => eprintln!(
                        "[mcp-brain] background pull complete: {top} (restart MCP to use it)"
                    ),
                    Err(e) => eprintln!("[mcp-brain] background pull failed for {top}: {e}"),
                }
            });
        }

        immediate
    };

    let model_tag = if let Some(tag) = chosen_model {
        eprintln!("[mcp-brain] using model: {tag}");
        tag
    } else {
        // Nothing local and no top pick — try the smallest fallback.
        let fallback = "gemma3:1b";
        eprintln!("[mcp-brain] no model available; trying fallback: {fallback}…");
        if let Err(e2) = ollama_agent::pull_model(&pull_client, OLLAMA_BASE_URL, fallback).await {
            eprintln!("[mcp-brain] failed to pull fallback: {e2}");
            return None;
        }
        fallback.to_string()
    };

    // Ensure embedding model is available for vector search
    ensure_embedding_model(&client, &local_tags).await;

    // Save legacy file too so OllamaAgent picks it up everywhere
    let _ = super::brain_store::save(data_dir, &model_tag);

    Some(BrainMode::LocalOllama { model: model_tag })
}

/// Pick the best already-pulled model that appears in our recommendations.
/// Returns the model tag as an owned String.
fn pick_best_local_model(
    local_tags: &[&str],
    recommendations: &[model_recommender::ModelRecommendation],
) -> Option<String> {
    for rec in recommendations {
        if rec.is_cloud {
            continue;
        }
        // Ollama tags might be "gemma3:4b" or "gemma3:4b-instruct-q4_0"
        // We match if local tag starts with the recommendation tag
        let found = local_tags.iter().any(|t| {
            *t == rec.model_tag
                || t.starts_with(&format!(
                    "{}:",
                    rec.model_tag.split(':').next().unwrap_or("")
                )) && t.contains(rec.model_tag.split(':').nth(1).unwrap_or(""))
        });
        if found {
            return Some(rec.model_tag.clone());
        }
    }
    None
}

/// Pick any locally available chat model (non-embedding) as last resort.
fn pick_any_local_chat_model(local_tags: &[&str]) -> Option<String> {
    let embedding_markers = ["embed", "nomic", "bge", "e5-"];
    local_tags
        .iter()
        .find(|t| {
            !embedding_markers
                .iter()
                .any(|m| t.to_lowercase().contains(m))
        })
        .map(|t| t.to_string())
}

/// Ensure the embedding model is pulled so vector search works.
async fn ensure_embedding_model(client: &reqwest::Client, local_tags: &[&str]) {
    let has_embed = local_tags
        .iter()
        .any(|t| t.contains("nomic-embed") || t.contains("embed"));

    if has_embed {
        eprintln!("[mcp-brain] embedding model already available");
        return;
    }

    eprintln!("[mcp-brain] pulling embedding model: {EMBEDDING_MODEL}…");
    match ollama_agent::pull_model(client, OLLAMA_BASE_URL, EMBEDDING_MODEL).await {
        Ok(()) => eprintln!("[mcp-brain] ✓ embedding model ready"),
        Err(e) => eprintln!(
            "[mcp-brain] ⚠ failed to pull embedding model: {e} — vector search will use keyword fallback"
        ),
    }
}

/// Update the in-memory `AppState` fields after auto-configuration.
/// Call this AFTER `auto_configure_mcp_brain` has saved to disk and the
/// state has been created (i.e., for the Tauri app path where state is
/// created before auto-config can run async).
pub fn apply_config_to_state(state: &crate::AppState, data_dir: &Path) {
    let mode = brain_config::load(data_dir);
    if let Some(ref m) = mode {
        if let Ok(mut guard) = state.brain_mode.lock() {
            *guard = Some(m.clone());
        }
        if let BrainMode::LocalOllama { ref model } = m {
            if let Ok(mut guard) = state.active_brain.lock() {
                *guard = Some(model.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_best_local_model_finds_match() {
        let local = vec!["gemma3:4b", "nomic-embed-text:latest", "phi4-mini:latest"];
        let sys = system_info::collect();
        let recs = model_recommender::recommend(sys.total_ram_mb);
        let result = pick_best_local_model(&local, &recs);
        // Should pick gemma3:4b or phi4-mini depending on RAM tier
        assert!(result.is_some());
    }

    #[test]
    fn pick_best_local_model_returns_none_on_empty() {
        let local: Vec<&str> = vec![];
        let recs = model_recommender::recommend(16_000);
        let result = pick_best_local_model(&local, &recs);
        assert!(result.is_none());
    }

    #[test]
    fn pick_any_local_skips_embedding_models() {
        let local = vec!["nomic-embed-text:latest", "bge-m3:latest"];
        let result = pick_any_local_chat_model(&local);
        assert!(result.is_none());
    }

    #[test]
    fn pick_any_local_finds_chat_model() {
        let local = vec!["nomic-embed-text:latest", "gemma3:4b"];
        let result = pick_any_local_chat_model(&local);
        assert_eq!(result, Some("gemma3:4b".to_string()));
    }
}
