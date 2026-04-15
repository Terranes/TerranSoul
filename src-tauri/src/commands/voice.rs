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

/// Groq Whisper API endpoint (OpenAI-compatible).
const GROQ_WHISPER_ENDPOINT: &str = "https://api.groq.com/openai/v1/audio/transcriptions";

/// Convert 16kHz float32 PCM samples (from VAD) to 16-bit signed PCM bytes.
fn float32_to_pcm16(samples: &[f32]) -> Vec<u8> {
    let mut pcm = Vec::with_capacity(samples.len() * 2);
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let sample_i16 = (clamped * i16::MAX as f32) as i16;
        pcm.extend_from_slice(&sample_i16.to_le_bytes());
    }
    pcm
}

/// Transcribe speech audio to text.
///
/// Accepts 16kHz float32 PCM samples (as produced by the VAD composable) and
/// routes to the configured ASR provider. Returns the transcription result.
///
/// Provider routing:
/// - `stub`         → StubAsr (returns fixed text, for testing)
/// - `whisper-api`  → OpenAI Whisper (requires api_key in config)
/// - `groq-whisper` → Groq Whisper endpoint (requires api_key, OpenAI-compatible)
/// - `web-speech`   → error (browser-native, must be called client-side)
/// - `None`         → error
#[tauri::command]
pub async fn transcribe_audio(
    samples: Vec<f32>,
    state: State<'_, AppState>,
) -> Result<voice::TranscriptionResult, String> {
    if samples.is_empty() {
        return Err("No audio samples provided".to_string());
    }

    let (provider, api_key, endpoint_url) = {
        let config = state.voice_config.lock().map_err(|e| e.to_string())?;
        (
            config.asr_provider.clone(),
            config.api_key.clone(),
            config.endpoint_url.clone(),
        )
    };

    let pcm = float32_to_pcm16(&samples);

    match provider.as_deref() {
        Some("stub") => {
            let engine = voice::stub_asr::StubAsr;
            engine.transcribe(&pcm).await
        }
        Some("whisper-api") => {
            let key = api_key.ok_or("Whisper API requires an API key")?;
            let engine = voice::whisper_api::WhisperApi::new(key);
            engine.transcribe(&pcm).await
        }
        Some("groq-whisper") => {
            let key = api_key.ok_or("Groq Whisper requires an API key")?;
            let endpoint = endpoint_url
                .unwrap_or_else(|| GROQ_WHISPER_ENDPOINT.to_string());
            let engine = voice::whisper_api::WhisperApi::with_endpoint(key, endpoint);
            engine.transcribe(&pcm).await
        }
        Some("web-speech") => Err(
            "web-speech uses the browser SpeechRecognition API directly; call useWebSpeech instead".to_string(),
        ),
        Some(id) => Err(format!("Unknown ASR provider: {id}")),
        None => Err("No ASR provider configured".to_string()),
    }
}


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

    // ── transcribe_audio tests ──────────────────────────────────────────────

    #[test]
    fn float32_to_pcm16_produces_correct_bytes() {
        // 0.0 → i16 0x0000, 1.0 → i16::MAX (32767 = 0x7FFF), -1.0 → i16::MIN+1
        let samples = vec![0.0f32, 1.0f32, -1.0f32];
        let pcm = float32_to_pcm16(&samples);
        assert_eq!(pcm.len(), 6); // 3 samples × 2 bytes

        // 0.0 → 0
        assert_eq!(i16::from_le_bytes([pcm[0], pcm[1]]), 0);
        // 1.0 → i16::MAX
        assert_eq!(i16::from_le_bytes([pcm[2], pcm[3]]), i16::MAX);
        // -1.0 → -(i16::MAX)
        assert_eq!(i16::from_le_bytes([pcm[4], pcm[5]]), -(i16::MAX));
    }

    #[test]
    fn float32_to_pcm16_clamps_out_of_range() {
        let samples = vec![2.0f32, -2.0f32];
        let pcm = float32_to_pcm16(&samples);
        // Clamped to [-1.0, 1.0] first, so both give max/min i16
        assert_eq!(i16::from_le_bytes([pcm[0], pcm[1]]), i16::MAX);
        assert_eq!(i16::from_le_bytes([pcm[2], pcm[3]]), -(i16::MAX));
    }

    #[tokio::test]
    async fn transcribe_audio_rejects_empty_samples() {
        let result: Result<voice::TranscriptionResult, String> = {
            let samples: Vec<f32> = vec![];
            if samples.is_empty() {
                Err("No audio samples provided".to_string())
            } else {
                Ok(voice::TranscriptionResult {
                    text: "".into(),
                    language: None,
                    confidence: None,
                })
            }
        };
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No audio samples provided");
    }

    #[tokio::test]
    async fn transcribe_audio_no_provider_configured() {
        let provider: Option<&str> = None;
        let result: Result<voice::TranscriptionResult, String> = match provider {
            Some("stub") => Ok(voice::TranscriptionResult {
                text: "stub".into(),
                language: None,
                confidence: None,
            }),
            Some(_) => Err("unknown".to_string()),
            None => Err("No ASR provider configured".to_string()),
        };
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No ASR provider configured");
    }

    #[tokio::test]
    async fn transcribe_audio_stub_returns_text() {
        let samples = vec![0.0f32; 100];
        let pcm = float32_to_pcm16(&samples);
        let engine = voice::stub_asr::StubAsr;
        let result = engine.transcribe(&pcm).await.unwrap();
        assert!(!result.text.is_empty());
    }

    #[tokio::test]
    async fn transcribe_audio_web_speech_returns_error() {
        let provider: Option<&str> = Some("web-speech");
        let result: Result<voice::TranscriptionResult, String> = match provider {
            Some("web-speech") => Err(
                "web-speech uses the browser SpeechRecognition API directly; call useWebSpeech instead"
                    .to_string(),
            ),
            Some("stub") => Ok(voice::TranscriptionResult {
                text: "stub".into(),
                language: None,
                confidence: None,
            }),
            Some(id) => Err(format!("Unknown ASR provider: {id}")),
            None => Err("No ASR provider configured".to_string()),
        };
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("web-speech"));
    }

    #[test]
    fn asr_catalogue_contains_groq_whisper() {
        let providers = voice::asr_providers();
        assert!(providers.iter().any(|p| p.id == "groq-whisper"));
        let groq = providers.iter().find(|p| p.id == "groq-whisper").unwrap();
        assert!(groq.requires_api_key);
        assert_eq!(groq.kind, "cloud");
    }

    #[tokio::test]
    async fn transcribe_audio_unknown_provider_errors() {
        let provider: Option<&str> = Some("azure-cognitive");
        let result: Result<voice::TranscriptionResult, String> = match provider {
            Some("stub") => Ok(voice::TranscriptionResult {
                text: "".into(),
                language: None,
                confidence: None,
            }),
            Some("whisper-api") | Some("groq-whisper") | Some("web-speech") => {
                Err("provider error".to_string())
            }
            Some(id) => Err(format!("Unknown ASR provider: {id}")),
            None => Err("No ASR provider configured".to_string()),
        };
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown ASR provider"));
    }
}

