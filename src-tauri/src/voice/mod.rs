pub mod config_store;
pub mod stub_asr;
pub mod stub_tts;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── ASR Engine Trait ──────────────────────────────────────────────────────────

/// Result of a speech-to-text transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// Transcribed text.
    pub text: String,
    /// Detected language code (e.g. "en"), if available.
    pub language: Option<String>,
    /// Confidence score (0.0–1.0), if available.
    pub confidence: Option<f64>,
}

/// Automatic Speech Recognition engine trait.
///
/// Implementors convert audio input into text. Each provider (Whisper API,
/// sherpa-onnx, Web Speech API, sidecar, etc.) implements this trait.
#[async_trait]
pub trait AsrEngine: Send + Sync {
    /// Unique provider identifier (e.g. "whisper-api", "sherpa-onnx").
    fn id(&self) -> &str;

    /// Human-readable display name (e.g. "OpenAI Whisper API").
    fn display_name(&self) -> &str;

    /// Transcribe raw audio bytes (PCM 16-bit mono 16kHz) into text.
    async fn transcribe(&self, audio: &[u8]) -> Result<TranscriptionResult, String>;

    /// Check whether the engine is available and healthy.
    async fn health_check(&self) -> bool;
}

// ── TTS Engine Trait ──────────────────────────────────────────────────────────

/// Result of a text-to-speech synthesis.
#[derive(Debug, Clone)]
pub struct SynthesisResult {
    /// Raw audio bytes (WAV or PCM).
    pub audio: Vec<u8>,
    /// MIME type of the audio (e.g. "audio/wav", "audio/pcm").
    pub mime_type: String,
    /// Sample rate in Hz.
    pub sample_rate: u32,
}

/// Text-to-Speech engine trait.
///
/// Implementors convert text into audio. Each provider (Edge TTS, OpenAI TTS,
/// VibeVoice sidecar, etc.) implements this trait.
#[async_trait]
pub trait TtsEngine: Send + Sync {
    /// Unique provider identifier (e.g. "edge-tts", "openai-tts").
    fn id(&self) -> &str;

    /// Human-readable display name (e.g. "Edge TTS (free)").
    fn display_name(&self) -> &str;

    /// Synthesize text into audio bytes.
    async fn synthesize(&self, text: &str) -> Result<SynthesisResult, String>;

    /// Check whether the engine is available and healthy.
    async fn health_check(&self) -> bool;
}

// ── Provider Descriptors ──────────────────────────────────────────────────────

/// Metadata describing an available voice provider (shown in the setup wizard).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProviderInfo {
    /// Unique identifier matching `AsrEngine::id()` or `TtsEngine::id()`.
    pub id: String,
    /// Human-readable name.
    pub display_name: String,
    /// Short description of the provider.
    pub description: String,
    /// Provider kind: "local", "cloud", or "sidecar".
    pub kind: String,
    /// Whether the provider requires an API key.
    pub requires_api_key: bool,
    /// Whether the provider requires a sidecar process.
    pub requires_sidecar: bool,
}

/// Persisted voice configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct VoiceConfig {
    /// Selected ASR provider ID, or `None` for text-only input.
    pub asr_provider: Option<String>,
    /// Selected TTS provider ID, or `None` for text-only output.
    pub tts_provider: Option<String>,
    /// Optional API key for cloud providers (stored in app-data, not source).
    pub api_key: Option<String>,
    /// Optional endpoint URL for sidecar or self-hosted providers.
    pub endpoint_url: Option<String>,
}

// ── Built-in Provider Catalogue ───────────────────────────────────────────────

/// Return the catalogue of ASR providers users can choose from.
pub fn asr_providers() -> Vec<VoiceProviderInfo> {
    vec![
        VoiceProviderInfo {
            id: "stub".into(),
            display_name: "Stub ASR (testing)".into(),
            description: "Returns fixed text. For development and testing only.".into(),
            kind: "local".into(),
            requires_api_key: false,
            requires_sidecar: false,
        },
        VoiceProviderInfo {
            id: "web-speech".into(),
            display_name: "Web Speech API".into(),
            description: "Browser-native speech recognition. Zero setup, works offline on supported browsers.".into(),
            kind: "local".into(),
            requires_api_key: false,
            requires_sidecar: false,
        },
        VoiceProviderInfo {
            id: "whisper-api".into(),
            display_name: "OpenAI Whisper API".into(),
            description: "Cloud-based transcription via OpenAI. High accuracy, requires API key.".into(),
            kind: "cloud".into(),
            requires_api_key: true,
            requires_sidecar: false,
        },
        VoiceProviderInfo {
            id: "sidecar-asr".into(),
            display_name: "Sidecar ASR (Python)".into(),
            description: "Local Python sidecar for engines like VibeVoice or sherpa-onnx.".into(),
            kind: "sidecar".into(),
            requires_api_key: false,
            requires_sidecar: true,
        },
        VoiceProviderInfo {
            id: "open-llm-vtuber".into(),
            display_name: "Open-LLM-VTuber".into(),
            description: "Connect to a running Open-LLM-VTuber server. Supports 7+ ASR engines via WebSocket.".into(),
            kind: "sidecar".into(),
            requires_api_key: false,
            requires_sidecar: true,
        },
    ]
}

/// Return the catalogue of TTS providers users can choose from.
pub fn tts_providers() -> Vec<VoiceProviderInfo> {
    vec![
        VoiceProviderInfo {
            id: "stub".into(),
            display_name: "Stub TTS (testing)".into(),
            description: "Returns silence. For development and testing only.".into(),
            kind: "local".into(),
            requires_api_key: false,
            requires_sidecar: false,
        },
        VoiceProviderInfo {
            id: "edge-tts".into(),
            display_name: "Edge TTS (free)".into(),
            description: "Microsoft Edge neural voices. Free, high quality, many languages.".into(),
            kind: "cloud".into(),
            requires_api_key: false,
            requires_sidecar: false,
        },
        VoiceProviderInfo {
            id: "openai-tts".into(),
            display_name: "OpenAI TTS".into(),
            description: "Cloud-based synthesis via OpenAI. Best quality, requires API key.".into(),
            kind: "cloud".into(),
            requires_api_key: true,
            requires_sidecar: false,
        },
        VoiceProviderInfo {
            id: "sidecar-tts".into(),
            display_name: "Sidecar TTS (Python)".into(),
            description: "Local Python sidecar for engines like VibeVoice or sherpa-onnx.".into(),
            kind: "sidecar".into(),
            requires_api_key: false,
            requires_sidecar: true,
        },
        VoiceProviderInfo {
            id: "open-llm-vtuber".into(),
            display_name: "Open-LLM-VTuber".into(),
            description: "Connect to a running Open-LLM-VTuber server. Supports 18+ TTS engines via WebSocket.".into(),
            kind: "sidecar".into(),
            requires_api_key: false,
            requires_sidecar: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asr_providers_not_empty() {
        let providers = asr_providers();
        assert!(!providers.is_empty());
        assert!(providers.iter().any(|p| p.id == "stub"));
    }

    #[test]
    fn tts_providers_not_empty() {
        let providers = tts_providers();
        assert!(!providers.is_empty());
        assert!(providers.iter().any(|p| p.id == "stub"));
    }

    #[test]
    fn provider_ids_are_unique() {
        let asr = asr_providers();
        let mut ids: Vec<&str> = asr.iter().map(|p| p.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), asr.len(), "duplicate ASR provider IDs");

        let tts = tts_providers();
        let mut ids: Vec<&str> = tts.iter().map(|p| p.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), tts.len(), "duplicate TTS provider IDs");
    }

    #[test]
    fn voice_config_default_is_empty() {
        let cfg = VoiceConfig::default();
        assert!(cfg.asr_provider.is_none());
        assert!(cfg.tts_provider.is_none());
        assert!(cfg.api_key.is_none());
        assert!(cfg.endpoint_url.is_none());
    }

    #[test]
    fn voice_config_serde_roundtrip() {
        let cfg = VoiceConfig {
            asr_provider: Some("whisper-api".into()),
            tts_provider: Some("edge-tts".into()),
            api_key: Some("sk-test".into()),
            endpoint_url: Some("http://localhost:8000".into()),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: VoiceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cfg);
    }
}
