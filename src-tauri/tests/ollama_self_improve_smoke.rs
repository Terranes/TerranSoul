//! Real-flow smoke test: drive the self-improve client against the
//! locally running Ollama daemon (must be reachable at
//! `http://localhost:11434`). Skips if `OLLAMA_REAL_TEST` is not set so
//! CI is never broken by a missing daemon.
//!
//! Run on Windows with a local Ollama:
//!     $env:OLLAMA_REAL_TEST = "1"
//!     cargo test --test ollama_self_improve_smoke -- --nocapture
//!
//! What this proves:
//! - `OpenAiClient` round-trips against Ollama's `/v1/chat/completions`.
//! - `coding::client::test_reachability` reports `ok=true` with a
//!   non-empty detail body from a real LLM.
//! - The metrics log records start + success rows and the summary
//!   computes `success_rate = 1.0` after a single successful round-trip.

use terransoul_lib::coding::{
    self,
    client::{client_from, test_reachability},
    metrics::MetricsLog,
    CodingLlmConfig, CodingLlmProvider,
};

fn ollama_cfg() -> CodingLlmConfig {
    CodingLlmConfig {
        provider: CodingLlmProvider::Custom,
        // Ollama exposes its OpenAI-compatible API at /v1.
        base_url: "http://localhost:11434".to_string(),
        model: std::env::var("OLLAMA_REAL_MODEL").unwrap_or_else(|_| "gemma3:4b".to_string()),
        // Ollama ignores the bearer token but the field is required.
        api_key: "ollama".to_string(),
    }
}

fn enabled() -> bool {
    std::env::var("OLLAMA_REAL_TEST").is_ok()
}

#[tokio::test]
async fn ollama_real_reachability_round_trip() {
    if !enabled() {
        eprintln!("skipping: set OLLAMA_REAL_TEST=1 to run");
        return;
    }
    let cfg = ollama_cfg();
    let result = test_reachability(&cfg).await;
    eprintln!("[real] reachability: {result:?}");
    assert!(result.ok, "expected ok=true; got {result:?}");
    let detail = result.detail.unwrap_or_default();
    assert!(
        !detail.trim().is_empty(),
        "expected non-empty reply from Ollama"
    );
}

#[tokio::test]
async fn ollama_real_chat_round_trip() {
    if !enabled() {
        eprintln!("skipping: set OLLAMA_REAL_TEST=1 to run");
        return;
    }
    use terransoul_lib::brain::openai_client::OpenAiMessage;
    let cfg = ollama_cfg();
    let client = client_from(&cfg);
    let reply = client
        .chat(vec![OpenAiMessage {
            role: "user".to_string(),
            content: "List two prime numbers under 10. Reply briefly.".to_string(),
        }])
        .await
        .expect("real Ollama chat must succeed");
    eprintln!("[real] reply: {reply}");
    assert!(!reply.trim().is_empty());
}

#[tokio::test]
async fn ollama_real_metrics_log_records_outcome() {
    if !enabled() {
        eprintln!("skipping: set OLLAMA_REAL_TEST=1 to run");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let log = MetricsLog::new(dir.path());

    let cfg = ollama_cfg();
    let started = log.record_start("smoke.1", "Smoke test", "custom", &cfg.model);

    use terransoul_lib::brain::openai_client::OpenAiMessage;
    let client = client_from(&cfg);
    let outcome = client
        .chat(vec![OpenAiMessage {
            role: "user".to_string(),
            content: "Reply with the word ok and nothing else.".to_string(),
        }])
        .await;

    match &outcome {
        Ok(reply) => log.record_outcome(
            started, "smoke.1", "Smoke test", "custom", &cfg.model, true, reply.len(), None,
            terransoul_lib::coding::metrics::TokenUsage::default(),
        ),
        Err(e) => log.record_outcome(
            started, "smoke.1", "Smoke test", "custom", &cfg.model, false, 0, Some(e),
            terransoul_lib::coding::metrics::TokenUsage::default(),
        ),
    }

    let summary = log.summary();
    eprintln!("[real] summary: {summary:?}");
    assert!(summary.total_runs >= 1);
    if outcome.is_ok() {
        assert_eq!(summary.successes, 1);
        assert!((summary.success_rate - 1.0).abs() < 1e-9);
    }

    // Use `coding::` to silence unused-import warnings on builds where
    // the env var isn't set (whole test body is gated above).
    let _ = coding::coding_llm_recommendations(16_384);
}

/// Loop test: drives `run_coding_task` end-to-end against a real,
/// installed Ollama using the recommended-provider configuration.
///
/// Proves the full self-improve stack works offline:
///   recommendations → CodingLlmConfig → workflow::run_coding_task →
///   prose output → result.well_formed && payload non-empty.
#[tokio::test]
async fn ollama_real_run_coding_task_prose() {
    if !enabled() {
        eprintln!("skipping: set OLLAMA_REAL_TEST=1 to run");
        return;
    }
    use terransoul_lib::coding::workflow::{run_coding_task, CodingTask, TaskOutputKind};

    // Source the Local-Ollama recommendation rather than hardcoding —
    // this validates the recommendation defaults are usable as-is.
    let recs = coding::coding_llm_recommendations(16_384);
    let local = recs
        .iter()
        .find(|r| !r.requires_api_key && r.base_url.contains("127.0.0.1"))
        .expect("Local Ollama recommendation must exist");

    let cfg = CodingLlmConfig {
        provider: local.provider.clone(),
        base_url: local.base_url.clone(),
        model: std::env::var("OLLAMA_REAL_MODEL").unwrap_or_else(|_| "gemma3:4b".to_string()),
        api_key: String::new(), // no key — proves the no-auth path
    };

    let task = CodingTask {
        id: "smoke.coding-task.1".to_string(),
        description: "Reply with exactly: ok".to_string(),
        repo_root: None, // skip context loading for a fast smoke
        include_rules: false,
        include_instructions: false,
        include_docs: false,
        output_kind: TaskOutputKind::Prose,
        extra_documents: Vec::new(),
        prior_handoff: None,
    };

    let result = run_coding_task(&cfg, &task, None)
        .await
        .expect("real Ollama coding task must succeed");
    eprintln!("[real] coding task payload: {}", result.payload);
    assert_eq!(result.task_id, "smoke.coding-task.1");
    assert!(result.well_formed, "prose output is always well-formed");
    assert!(
        !result.payload.trim().is_empty(),
        "expected non-empty payload from real Ollama"
    );
}
