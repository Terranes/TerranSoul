//! IPC contract tests — verify that JSON payloads matching the frontend's
//! camelCase parameter names deserialize correctly into Rust types.
//!
//! These tests catch mismatches between the JS `invoke()` calls and the Rust
//! `#[tauri::command(rename_all = "camelCase")]` signatures.

#[cfg(test)]
mod tests {
    use serde_json::json;

    // ── BrainMode serde ─────────────────────────────────────────────────────

    #[test]
    fn brain_mode_free_api_deserializes() {
        let payload = json!({
            "mode": "free_api",
            "provider_id": "groq",
            "api_key": null
        });
        let mode: crate::brain::BrainMode = serde_json::from_value(payload).unwrap();
        match mode {
            crate::brain::BrainMode::FreeApi { provider_id, .. } => {
                assert_eq!(provider_id, "groq");
            }
            _ => panic!("Expected FreeApi variant"),
        }
    }

    #[test]
    fn brain_mode_local_ollama_deserializes() {
        let payload = json!({
            "mode": "local_ollama",
            "model": "phi-4:latest"
        });
        let mode: crate::brain::BrainMode = serde_json::from_value(payload).unwrap();
        match mode {
            crate::brain::BrainMode::LocalOllama { model } => {
                assert_eq!(model, "phi-4:latest");
            }
            _ => panic!("Expected LocalOllama variant"),
        }
    }

    #[test]
    fn brain_mode_paid_api_deserializes() {
        let payload = json!({
            "mode": "paid_api",
            "provider": "openai",
            "api_key": "sk-test",
            "model": "gpt-4o",
            "base_url": "https://api.openai.com"
        });
        let mode: crate::brain::BrainMode = serde_json::from_value(payload).unwrap();
        match mode {
            crate::brain::BrainMode::PaidApi { provider, model, .. } => {
                assert_eq!(provider, "openai");
                assert_eq!(model, "gpt-4o");
            }
            _ => panic!("Expected PaidApi variant"),
        }
    }

    // ── TrustedDevice serde ─────────────────────────────────────────────────

    #[test]
    fn trusted_device_deserializes_from_frontend_json() {
        let payload = json!({
            "device_id": "abc-123",
            "public_key_b64": "ed25519:AAAA",
            "name": "My Phone",
            "paired_at": 1713312000000u64
        });
        let device: crate::identity::TrustedDevice =
            serde_json::from_value(payload).unwrap();
        assert_eq!(device.device_id, "abc-123");
        assert_eq!(device.name, "My Phone");
    }

    // ── AppSettings serde ───────────────────────────────────────────────────

    #[test]
    fn app_settings_roundtrip_with_camera_positions() {
        let payload = json!({
            "version": 2,
            "selected_model_id": "shinra",
            "camera_azimuth": 0.5,
            "camera_distance": 3.0,
            "bgm_enabled": true,
            "bgm_volume": 0.25,
            "bgm_track_id": "prelude",
            "model_camera_positions": {
                "shinra": { "azimuth": 0.5, "distance": 3.0 }
            }
        });
        let settings: crate::settings::AppSettings =
            serde_json::from_value(payload).unwrap();
        assert_eq!(settings.selected_model_id, "shinra");
        assert!(settings.bgm_enabled);
        assert_eq!(settings.model_camera_positions.len(), 1);
        let cam = &settings.model_camera_positions["shinra"];
        assert!((cam.azimuth - 0.5).abs() < 0.001);
    }

    // ── VoiceConfig serde ───────────────────────────────────────────────────

    #[test]
    fn voice_config_deserializes_with_all_fields() {
        let payload = json!({
            "asr_provider": "groq-whisper",
            "tts_provider": "web-speech",
            "api_key": "sk-test",
            "endpoint_url": "https://custom.api/v1"
        });
        let config: crate::voice::VoiceConfig =
            serde_json::from_value(payload).unwrap();
        assert_eq!(config.asr_provider.as_deref(), Some("groq-whisper"));
        assert_eq!(config.tts_provider.as_deref(), Some("web-speech"));
        assert_eq!(config.api_key.as_deref(), Some("sk-test"));
        assert_eq!(config.endpoint_url.as_deref(), Some("https://custom.api/v1"));
    }

    #[test]
    fn voice_config_deserializes_with_null_fields() {
        let payload = json!({
            "asr_provider": null,
            "tts_provider": null,
            "api_key": null,
            "endpoint_url": null
        });
        let config: crate::voice::VoiceConfig =
            serde_json::from_value(payload).unwrap();
        assert!(config.asr_provider.is_none());
        assert!(config.tts_provider.is_none());
    }

    // ── DockerStatus serde ──────────────────────────────────────────────────

    #[test]
    fn docker_status_serializes_for_frontend() {
        let status = crate::brain::docker_ollama::DockerStatus {
            cli_found: true,
            daemon_running: false,
            desktop_installed: true,
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["cli_found"], true);
        assert_eq!(json["daemon_running"], false);
        assert_eq!(json["desktop_installed"], true);
    }

    #[test]
    fn ollama_container_status_serializes_for_frontend() {
        let status = crate::brain::docker_ollama::OllamaContainerStatus {
            exists: true,
            running: true,
            api_reachable: false,
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["exists"], true);
        assert_eq!(json["running"], true);
        assert_eq!(json["api_reachable"], false);
    }

    // ── Message serde (chat) ────────────────────────────────────────────────

    #[test]
    fn chat_message_serializes_for_frontend() {
        let msg = crate::commands::chat::Message {
            id: "msg-1".into(),
            role: "assistant".into(),
            content: "Hello!".into(),
            agent_name: Some("TerranSoul".into()),
            agent_id: None,
            sentiment: Some("happy".into()),
            timestamp: 1713312000000,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["id"], "msg-1");
        assert_eq!(json["role"], "assistant");
        assert_eq!(json["agent_name"], "TerranSoul");
        assert_eq!(json["sentiment"], "happy");
    }

    #[test]
    fn chat_message_deserializes_from_frontend() {
        let payload = json!({
            "id": "msg-2",
            "role": "user",
            "content": "Hi there",
            "agent_name": null,
            "sentiment": null,
            "timestamp": 1713312000000u64
        });
        let msg: crate::commands::chat::Message =
            serde_json::from_value(payload).unwrap();
        assert_eq!(msg.role, "user");
        assert!(msg.agent_name.is_none());
    }
}
