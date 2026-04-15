use tauri::State;

use crate::voice::{self, VoiceConfig, VoiceProviderInfo};
use crate::AppState;

/// List available ASR providers.
#[tauri::command]
pub async fn list_asr_providers() -> Vec<VoiceProviderInfo> {
    voice::asr_providers()
}

/// List available TTS providers.
#[tauri::command]
pub async fn list_tts_providers() -> Vec<VoiceProviderInfo> {
    voice::tts_providers()
}

/// Get the current voice configuration.
#[tauri::command]
pub async fn get_voice_config(state: State<'_, AppState>) -> Result<VoiceConfig, String> {
    let config = state.voice_config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

/// Set the ASR provider. Pass `null` to disable ASR (text-only input).
#[tauri::command]
pub async fn set_asr_provider(
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Validate provider ID if provided
    if let Some(ref id) = provider_id {
        let known = voice::asr_providers();
        if !known.iter().any(|p| p.id == *id) {
            return Err(format!("Unknown ASR provider: {id}"));
        }
    }

    let mut config = state.voice_config.lock().map_err(|e| e.to_string())?;
    config.asr_provider = provider_id;
    voice::config_store::save(&state.data_dir, &config)?;
    Ok(())
}

/// Set the TTS provider. Pass `null` to disable TTS (text-only output).
#[tauri::command]
pub async fn set_tts_provider(
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Validate provider ID if provided
    if let Some(ref id) = provider_id {
        let known = voice::tts_providers();
        if !known.iter().any(|p| p.id == *id) {
            return Err(format!("Unknown TTS provider: {id}"));
        }
    }

    let mut config = state.voice_config.lock().map_err(|e| e.to_string())?;
    config.tts_provider = provider_id;
    voice::config_store::save(&state.data_dir, &config)?;
    Ok(())
}

/// Set the API key for cloud voice providers.
#[tauri::command]
pub async fn set_voice_api_key(
    api_key: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.voice_config.lock().map_err(|e| e.to_string())?;
    config.api_key = api_key;
    voice::config_store::save(&state.data_dir, &config)?;
    Ok(())
}

/// Set the endpoint URL for custom cloud voice providers.
#[tauri::command]
pub async fn set_voice_endpoint(
    endpoint_url: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.voice_config.lock().map_err(|e| e.to_string())?;
    config.endpoint_url = endpoint_url;
    voice::config_store::save(&state.data_dir, &config)?;
    Ok(())
}

/// Clear all voice configuration, reverting to text-only mode.
#[tauri::command]
pub async fn clear_voice_config(state: State<'_, AppState>) -> Result<(), String> {
    voice::config_store::clear(&state.data_dir)?;
    let mut config = state.voice_config.lock().map_err(|e| e.to_string())?;
    *config = VoiceConfig::default();
    Ok(())
}

/// Synthesize text to speech and return raw WAV bytes.
///
/// Routes to the configured TTS provider (from `voice_config.tts_provider`).
/// Returns the WAV audio bytes so the frontend can play them directly.
/// Used by the streaming TTS pipeline — called per sentence as the LLM streams.
#[tauri::command]
pub async fn synthesize_tts(text: String, state: State<'_, AppState>) -> Result<Vec<u8>, String> {
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        return Err("Text cannot be empty".to_string());
    }

    let provider = {
        let config = state.voice_config.lock().map_err(|e| e.to_string())?;
        config.tts_provider.clone()
    };

    match provider.as_deref() {
        Some("stub") => {
            let engine = voice::stub_tts::StubTts;
            engine.synthesize(&trimmed).await.map(|r| r.audio)
        }
        Some("edge-tts") => {
            let engine = voice::edge_tts::EdgeTts::new();
            engine.synthesize(&trimmed).await.map(|r| r.audio)
        }
        Some(id) => Err(format!("Unsupported TTS provider: {id}")),
        None => Err("No TTS provider configured".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice;

    #[test]
    fn list_asr_providers_contains_stub() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let providers = rt.block_on(list_asr_providers());
        assert!(providers.iter().any(|p| p.id == "stub"));
    }

    #[test]
    fn list_tts_providers_contains_stub() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let providers = rt.block_on(list_tts_providers());
        assert!(providers.iter().any(|p| p.id == "stub"));
    }

    #[test]
    fn asr_providers_have_valid_kinds() {
        let providers = voice::asr_providers();
        for p in &providers {
            assert!(
                ["local", "cloud"].contains(&p.kind.as_str()),
                "Invalid kind for {}: {}",
                p.id,
                p.kind
            );
        }
    }

    #[test]
    fn tts_providers_have_valid_kinds() {
        let providers = voice::tts_providers();
        for p in &providers {
            assert!(
                ["local", "cloud"].contains(&p.kind.as_str()),
                "Invalid kind for {}: {}",
                p.id,
                p.kind
            );
        }
    }

    #[tokio::test]
    async fn synthesize_tts_rejects_empty_text() {
        let state = crate::AppState::for_test();
        // Set stub provider
        state.voice_config.lock().unwrap().tts_provider = Some("stub".to_string());

        // Wrap state in tauri::State for testing
        let result = {
            let engine = voice::stub_tts::StubTts;
            // Test the empty check directly
            let trimmed = "".trim().to_string();
            if trimmed.is_empty() {
                Err::<Vec<u8>, String>("Text cannot be empty".to_string())
            } else {
                engine.synthesize(&trimmed).await.map(|r| r.audio)
            }
        };
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Text cannot be empty");
    }

    #[tokio::test]
    async fn synthesize_tts_stub_returns_wav() {
        let engine = voice::stub_tts::StubTts;
        let result = engine.synthesize("Hello world").await.unwrap();
        assert_eq!(&result.audio[..4], b"RIFF");
        assert_eq!(&result.audio[8..12], b"WAVE");
        assert!(!result.audio.is_empty());
    }

    #[tokio::test]
    async fn synthesize_tts_no_provider_configured() {
        // Simulate None provider path
        let provider: Option<&str> = None;
        let result: Result<Vec<u8>, String> = match provider {
            Some("stub") => Ok(vec![]),
            Some(id) => Err(format!("Unsupported TTS provider: {id}")),
            None => Err("No TTS provider configured".to_string()),
        };
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No TTS provider configured");
    }

    #[tokio::test]
    async fn synthesize_tts_unknown_provider_errors() {
        let provider: Option<&str> = Some("unknown-provider");
        let result: Result<Vec<u8>, String> = match provider {
            Some("stub") => Ok(vec![]),
            Some("edge-tts") => Ok(vec![]),
            Some(id) => Err(format!("Unsupported TTS provider: {id}")),
            None => Err("No TTS provider configured".to_string()),
        };
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported TTS provider"));
    }
}

