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
pub async fn auto_configure_mcp_brain(data_dir: &Path) {
    // Skip if already configured
    if brain_config::load(data_dir).is_some() {
        eprintln!("[mcp-brain] brain already configured, skipping auto-config");
        return;
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

    // Fallback: Pollinations free API (no key needed)
    eprintln!("[mcp-brain] Ollama not available — falling back to Pollinations free API");
    let mode = BrainMode::FreeApi {
        provider_id: "pollinations".to_string(),
        api_key: None,
        model: None,
    };
    if let Err(e) = brain_config::save(data_dir, &mode) {
        eprintln!("[mcp-brain] failed to save brain config: {e}");
    } else {
        eprintln!("[mcp-brain] ✓ configured Pollinations free API as fallback");
    }
}

/// Attempt Ollama-based configuration. Returns `Some(BrainMode)` on success.
async fn try_configure_ollama(data_dir: &Path) -> Option<BrainMode> {
    let status = ollama_lifecycle::detect_ollama().await;

    if !status.installed {
        eprintln!("[mcp-brain] Ollama not installed — skipping local mode");
        return None;
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

    // Build a reqwest client for Ollama operations
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
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

    // Find the best already-pulled model (matches a recommendation)
    let chosen_model = pick_best_local_model(&local_tags, &recommendations)
        .or_else(|| pick_any_local_chat_model(&local_tags));

    let model_tag = if let Some(tag) = chosen_model {
        eprintln!("[mcp-brain] using already-pulled model: {tag}");
        tag
    } else {
        // No suitable model pulled — pull the top recommendation
        let top = recommendations
            .iter()
            .find(|r| !r.is_cloud)
            .map(|r| r.model_tag.as_str())
            .unwrap_or("gemma3:4b");
        eprintln!("[mcp-brain] pulling recommended model: {top}…");
        if let Err(e) = ollama_agent::pull_model(&client, OLLAMA_BASE_URL, top).await {
            eprintln!("[mcp-brain] failed to pull {top}: {e}");
            // Try a small fallback
            let fallback = "gemma3:1b";
            eprintln!("[mcp-brain] trying fallback model: {fallback}…");
            if let Err(e2) = ollama_agent::pull_model(&client, OLLAMA_BASE_URL, fallback).await {
                eprintln!("[mcp-brain] failed to pull fallback: {e2}");
                return None;
            }
            fallback.to_string()
        } else {
            top.to_string()
        }
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
