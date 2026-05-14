//! Live smoke test exercising [`companions::check_update_real`] against
//! the real GitHub Releases API + locally-installed companions, and
//! Hermes ↔ TerranSoul MCP integration verification.
//!
//! `#[ignore]` so it never runs in CI (depends on the developer machine
//! having Hermes Desktop / Hermes Agent installed and network access to
//! `api.github.com`). Invoke manually with:
//!
//! ```pwsh
//! $env:CARGO_TARGET_DIR="D:\Git\TerranSoul\target-ci-local"
//! cargo test --test companions_live -- --ignored --nocapture
//! ```

use terransoul_lib::integrations::companions;

#[tokio::test]
#[ignore]
async fn hermes_desktop_live_update_check() {
    let app = companions::get("hermes-desktop").expect("hermes-desktop registered");
    let result = companions::check_update_real(&app).await;
    eprintln!("hermes-desktop update check: {:#?}", result);
    // The latest-release half must succeed when network is reachable.
    assert!(
        result.latest.is_some(),
        "expected latest release info from GitHub API; note={:?}",
        result.note
    );
}

#[tokio::test]
#[ignore]
async fn hermes_agent_live_update_check() {
    let app = companions::get("hermes-agent").expect("hermes-agent registered");
    let result = companions::check_update_real(&app).await;
    eprintln!("hermes-agent update check: {:#?}", result);
    assert!(
        result.latest.is_some(),
        "expected latest release info from GitHub API; note={:?}",
        result.note
    );
}

// ─── Hermes ↔ TerranSoul MCP + shared LLM integration ─────────────

use terransoul_lib::ai_integrations::mcp::auto_setup;

/// Verify Hermes `cli-config.yaml` exists on disk and contains the
/// TerranSoul MCP managed block with correct markers, URL, and bearer token.
#[test]
#[ignore]
fn hermes_config_has_terransoul_mcp_block() {
    let path = auto_setup::hermes_config_path()
        .expect("hermes_config_path() should resolve on this machine");

    eprintln!("Hermes config path: {}", path.display());
    assert!(
        path.exists(),
        "cli-config.yaml not found at {}. Run setup_hermes_mcp first.",
        path.display()
    );

    let content = std::fs::read_to_string(&path).unwrap();
    eprintln!("Config length: {} chars", content.len());

    // Managed block markers
    assert!(
        content.contains(auto_setup::HERMES_BLOCK_BEGIN),
        "Missing TerranSoul block begin marker"
    );
    assert!(
        content.contains(auto_setup::HERMES_BLOCK_END),
        "Missing TerranSoul block end marker"
    );

    // MCP server entry
    assert!(
        content.contains("terransoul-brain"),
        "Missing terransoul-brain MCP server entry"
    );
    assert!(
        content.contains("127.0.0.1"),
        "MCP URL should point to localhost"
    );
    assert!(content.contains("Bearer "), "Missing bearer token");
}

/// Verify Hermes config includes Ollama LLM sharing (same local Ollama
/// instance TerranSoul uses).
#[test]
#[ignore]
fn hermes_config_shares_ollama_llm() {
    let path = auto_setup::hermes_config_path()
        .expect("hermes_config_path() should resolve");
    assert!(path.exists(), "cli-config.yaml not found");

    let content = std::fs::read_to_string(&path).unwrap();

    // Ollama provider
    let has_ollama = content.contains("provider: \"ollama\"")
        || content.contains("provider: \"custom\"");
    assert!(has_ollama, "Config should set provider to ollama or custom");

    // Ollama base URL
    assert!(
        content.contains("127.0.0.1:11434"),
        "Config should point to local Ollama at 127.0.0.1:11434"
    );
}

/// Live test: call TerranSoul MCP /health endpoint the way Hermes would,
/// using the bearer token from the Hermes config.
#[tokio::test]
#[ignore]
async fn hermes_mcp_brain_health_live() {
    let path = auto_setup::hermes_config_path()
        .expect("hermes_config_path() should resolve");
    let content = std::fs::read_to_string(&path).unwrap_or_default();

    // Extract URL and token from config
    let url = extract_yaml_value(&content, "url")
        .expect("Could not extract MCP URL from Hermes config");
    let token = extract_bearer_token(&content)
        .expect("Could not extract bearer token from Hermes config");

    // Derive health endpoint from MCP URL (e.g. http://127.0.0.1:7423/mcp → /health)
    let health_url = url.replace("/mcp", "/health");
    eprintln!("Calling health endpoint: {health_url}");

    let client = reqwest::Client::new();
    let resp = client
        .get(&health_url)
        .header("Authorization", format!("Bearer {token}"))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .expect("Failed to reach TerranSoul MCP health endpoint");

    assert!(
        resp.status().is_success(),
        "MCP health returned {}",
        resp.status()
    );

    let body = resp.text().await.unwrap();
    eprintln!("Health response: {}", &body[..body.len().min(300)]);
    assert!(
        body.contains("version") || body.contains("ok"),
        "Health response should contain version info"
    );
}

/// Live test: call brain_search via MCP JSON-RPC, simulating what Hermes
/// Agent does when a user asks a question that hits TerranSoul's brain.
#[tokio::test]
#[ignore]
async fn hermes_mcp_brain_search_live() {
    let path = auto_setup::hermes_config_path()
        .expect("hermes_config_path() should resolve");
    let content = std::fs::read_to_string(&path).unwrap_or_default();

    let url = extract_yaml_value(&content, "url")
        .expect("Could not extract MCP URL from Hermes config");
    let token = extract_bearer_token(&content)
        .expect("Could not extract bearer token from Hermes config");

    eprintln!("Calling brain_search via MCP JSON-RPC at: {url}");

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "brain_search",
            "arguments": {
                "query": "TerranSoul architecture overview",
                "limit": 3,
                "rerank": false
            }
        }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {token}"))
        .timeout(std::time::Duration::from_secs(30))
        .json(&body)
        .send()
        .await
        .expect("Failed to reach TerranSoul MCP endpoint");

    assert!(
        resp.status().is_success(),
        "MCP brain_search returned {}",
        resp.status()
    );

    let result: serde_json::Value = resp.json().await.unwrap();
    let pretty = serde_json::to_string_pretty(&result).unwrap_or_default();
    let truncate = 500.min(pretty.len());
    eprintln!("brain_search response: {}", &pretty[..truncate]);

    // JSON-RPC result should contain content
    assert!(
        result["result"]["content"].is_array(),
        "Expected result.content array in JSON-RPC response"
    );
    let text = result["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert!(!text.is_empty(), "brain_search returned empty content");

    // Parse the memories array
    let memories: Vec<serde_json::Value> = serde_json::from_str(text)
        .expect("brain_search content should be a JSON array of memories");
    assert!(
        !memories.is_empty(),
        "brain_search should return at least one memory"
    );
    eprintln!(
        "Got {} memories, first score: {}",
        memories.len(),
        memories[0]["score"]
    );
}

/// Live test: verify Ollama is reachable and has the shared model available.
#[tokio::test]
#[ignore]
async fn hermes_shared_ollama_reachable() {
    let client = reqwest::Client::new();

    // Check Ollama version
    let resp = client
        .get("http://127.0.0.1:11434/api/version")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .expect("Ollama not reachable at 127.0.0.1:11434");
    assert!(resp.status().is_success());
    let version: serde_json::Value = resp.json().await.unwrap();
    eprintln!("Ollama version: {}", version["version"]);

    // Check models list includes gemma3
    let resp = client
        .get("http://127.0.0.1:11434/api/tags")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .unwrap();
    let tags: serde_json::Value = resp.json().await.unwrap();
    let empty = vec![];
    let models: Vec<&str> = tags["models"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .filter_map(|m| m["name"].as_str())
        .collect();
    eprintln!("Available models: {:?}", models);
    assert!(
        models.iter().any(|n| n.contains("gemma3")),
        "Shared model gemma3 not found in Ollama; available: {:?}",
        models
    );
}

// ─── Helpers ───────────────────────────────────────────────────────

/// Extract a simple YAML value like `url: "http://..."` from raw text.
fn extract_yaml_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&format!("{key}:")) {
            let val = trimmed
                .strip_prefix(&format!("{key}:"))
                .unwrap()
                .trim()
                .trim_matches('"');
            return Some(val.to_string());
        }
    }
    None
}

/// Extract bearer token from `Authorization: "Bearer <token>"` in YAML.
fn extract_bearer_token(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Authorization:") {
            if let Some(bearer) = trimmed.strip_prefix("Authorization:") {
                let token = bearer
                    .trim()
                    .trim_matches('"')
                    .strip_prefix("Bearer ")
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !token.is_empty() {
                    return Some(token);
                }
            }
        }
    }
    None
}
