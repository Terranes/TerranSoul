pub mod config_store;
pub mod stub_asr;
pub mod stub_diarization;
pub mod stub_tts;
pub mod supertonic_download;
pub mod supertonic_manifest;
pub mod supertonic_paths;
#[cfg(feature = "tts-supertonic")]
pub mod supertonic_tts;
pub mod whisper_api;

use std::path::Path;

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
/// Web Speech API, etc.) implements this trait.
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
/// etc.) implements this trait.
#[async_trait]
pub trait TtsEngine: Send + Sync {
    /// Unique provider identifier (e.g. "web-speech", "openai-tts").
    fn id(&self) -> &str;

    /// Human-readable display name (e.g. "Edge TTS (free)").
    fn display_name(&self) -> &str;

    /// Synthesize text into audio bytes.
    async fn synthesize(&self, text: &str) -> Result<SynthesisResult, String>;

    /// Check whether the engine is available and healthy.
    async fn health_check(&self) -> bool;
}

// ── Speaker Diarization ──────────────────────────────────────────────────────

/// A segment of speech attributed to a specific speaker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarizedSegment {
    /// Speaker label (e.g. "Speaker 1", "Speaker 2").
    pub speaker: String,
    /// Transcribed text for this segment.
    pub text: String,
    /// Start time in seconds from beginning of audio.
    pub start_secs: f64,
    /// End time in seconds from beginning of audio.
    pub end_secs: f64,
    /// Confidence score (0.0–1.0), if available.
    pub confidence: Option<f64>,
}

/// Speaker diarization engine trait.
///
/// Implementors split audio into speaker-attributed segments. Each provider
/// (VibeVoice-ASR-7B, pyannote, etc.) implements this trait.
#[async_trait]
pub trait DiarizationEngine: Send + Sync {
    /// Unique provider identifier (e.g. "stub", "vibevoice").
    fn id(&self) -> &str;

    /// Human-readable display name (e.g. "VibeVoice ASR-7B").
    fn display_name(&self) -> &str;

    /// Diarize audio into speaker-attributed segments.
    async fn diarize(&self, audio: &[u8]) -> Result<Vec<DiarizedSegment>, String>;

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
    /// Provider kind: "local" or "cloud".
    pub kind: String,
    /// Whether the provider requires an API key.
    pub requires_api_key: bool,
    /// Whether the provider is currently installed and ready to synthesize
    /// audio. Defaults to `true` for built-in providers; `false` for native
    /// providers that must download a model before first use (e.g.
    /// Supertonic). UI surfaces an "Install" button when `installed == false
    /// && requires_install == true`.
    #[serde(default = "default_true")]
    pub installed: bool,
    /// Whether the provider needs a separate install step before it can
    /// synthesize audio (e.g. downloading ONNX model files). Mutually
    /// orthogonal to `requires_api_key`.
    #[serde(default)]
    pub requires_install: bool,
}

fn default_true() -> bool {
    true
}

/// A user-defined hotword for ASR boosting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hotword {
    /// The word or phrase to boost recognition of.
    pub phrase: String,
    /// Boost weight (0.0–10.0). Higher = more likely to be recognized.
    pub boost: f32,
}

/// Persisted voice configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceConfig {
    /// Selected ASR provider ID, or `None` for text-only input.
    pub asr_provider: Option<String>,
    /// Selected TTS provider ID. Defaults to `"web-speech"` (the
    /// browser's built-in `SpeechSynthesis` API — free, offline-
    /// capable, no telemetry, no third-party ToS to worry about).
    pub tts_provider: Option<String>,
    /// TTS voice name (provider-specific — e.g. a `SpeechSynthesisVoice`
    /// name for the Web Speech engine). When `None`, the engine picks
    /// its default voice.
    pub tts_voice: Option<String>,
    /// TTS pitch offset (provider-specific scale; `0` = neutral).
    #[serde(default)]
    pub tts_pitch: i32,
    /// TTS rate offset (provider-specific scale; `0` = neutral).
    #[serde(default)]
    pub tts_rate: i32,
    /// Optional API key for cloud providers (stored in app-data, not source).
    pub api_key: Option<String>,
    /// Optional endpoint URL for custom cloud providers.
    pub endpoint_url: Option<String>,
    /// User-defined hotwords for ASR boosting.
    #[serde(default)]
    pub hotwords: Vec<Hotword>,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            asr_provider: None,
            tts_provider: Some("web-speech".into()),
            tts_voice: None,
            tts_pitch: 0,
            tts_rate: 0,
            api_key: None,
            endpoint_url: None,
            hotwords: vec![],
        }
    }
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
            installed: true,
            requires_install: false,
        },
        VoiceProviderInfo {
            id: "web-speech".into(),
            display_name: "Web Speech API".into(),
            description: "Browser-native speech recognition. Zero setup, works offline on supported browsers.".into(),
            kind: "local".into(),
            requires_api_key: false,
            installed: true,
            requires_install: false,
        },
        VoiceProviderInfo {
            id: "whisper-api".into(),
            display_name: "OpenAI Whisper API".into(),
            description: "Cloud-based transcription via OpenAI. High accuracy, requires API key.".into(),
            kind: "cloud".into(),
            requires_api_key: true,
            installed: true,
            requires_install: false,
        },
        VoiceProviderInfo {
            id: "groq-whisper".into(),
            display_name: "Groq Whisper (fast)".into(),
            description: "Whisper-compatible transcription via Groq. Very fast, generous free tier, requires API key.".into(),
            kind: "cloud".into(),
            requires_api_key: true,
            installed: true,
            requires_install: false,
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
            installed: true,
            requires_install: false,
        },
        VoiceProviderInfo {
            id: "web-speech".into(),
            display_name: "Web Speech (browser, free)".into(),
            description: "Browser-native SpeechSynthesis. Free, offline-capable, no telemetry, no third-party API.".into(),
            kind: "local".into(),
            requires_api_key: false,
            installed: true,
            requires_install: false,
        },
        VoiceProviderInfo {
            id: "openai-tts".into(),
            display_name: "OpenAI TTS".into(),
            description: "Cloud-based synthesis via OpenAI. Best quality, requires API key.".into(),
            kind: "cloud".into(),
            requires_api_key: true,
            installed: true,
            requires_install: false,
        },
        // Supertonic — on-device ONNX TTS (supertone-inc/supertonic, MIT sample
        // code + OpenRAIL-M model). Compact (~66M params, 5 languages, 10
        // preset voices) — see `supertonic_manifest` for the pinned download
        // shape. `installed` is `false` here; callers that have the
        // `app_data_dir` available should prefer `tts_providers_for` which
        // flips the flag based on on-disk state.
        VoiceProviderInfo {
            id: "supertonic".into(),
            display_name: "Supertonic (on-device, neural)".into(),
            description: "On-device neural TTS via ONNX Runtime. 5 languages, 10 preset voices, ~268 MB model. First-run download required.".into(),
            kind: "local".into(),
            requires_api_key: false,
            installed: false,
            requires_install: true,
        },
    ]
}

/// Return the TTS provider catalogue with the Supertonic `installed` flag
/// reflecting on-disk state at `app_data_dir`. UI callers should prefer this
/// over [`tts_providers`] so the Voice panel can flip Supertonic to "Active"
/// without an additional round-trip.
pub fn tts_providers_for(app_data_dir: &Path) -> Vec<VoiceProviderInfo> {
    let install_dir = supertonic_paths::install_dir(app_data_dir);
    let installed = supertonic_paths::is_installed(&install_dir);
    tts_providers()
        .into_iter()
        .map(|mut p| {
            if p.id == "supertonic" {
                p.installed = installed;
            }
            p
        })
        .collect()
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
    fn voice_config_default_uses_web_speech_tts() {
        let cfg = VoiceConfig::default();
        assert!(cfg.asr_provider.is_none());
        assert_eq!(cfg.tts_provider.as_deref(), Some("web-speech"));
        assert!(cfg.api_key.is_none());
        assert!(cfg.endpoint_url.is_none());
    }

    #[test]
    fn voice_config_serde_roundtrip() {
        let cfg = VoiceConfig {
            asr_provider: Some("whisper-api".into()),
            tts_provider: Some("web-speech".into()),
            tts_voice: None,
            tts_pitch: 0,
            tts_rate: 0,
            api_key: Some("sk-test".into()),
            endpoint_url: Some("http://localhost:8000".into()),
            hotwords: vec![],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: VoiceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn hotword_serde_roundtrip() {
        let hw = Hotword {
            phrase: "Kerrigan".into(),
            boost: 7.5,
        };
        let json = serde_json::to_string(&hw).unwrap();
        let parsed: Hotword = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, hw);
    }

    #[test]
    fn voice_config_with_hotwords_serde_roundtrip() {
        let cfg = VoiceConfig {
            asr_provider: Some("stub".into()),
            tts_provider: None,
            tts_voice: None,
            tts_pitch: 0,
            tts_rate: 0,
            api_key: None,
            endpoint_url: None,
            hotwords: vec![
                Hotword {
                    phrase: "Zeratul".into(),
                    boost: 8.0,
                },
                Hotword {
                    phrase: "Protoss".into(),
                    boost: 5.0,
                },
            ],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: VoiceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn voice_config_deserializes_without_hotwords_field() {
        let json =
            r#"{"asr_provider":"stub","tts_provider":null,"api_key":null,"endpoint_url":null}"#;
        let cfg: VoiceConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.hotwords.is_empty());
    }

    #[test]
    fn hotword_default_voice_config_has_empty_hotwords() {
        let cfg = VoiceConfig::default();
        assert!(cfg.hotwords.is_empty());
    }

    #[test]
    fn diarized_segment_serde_roundtrip() {
        let segment = DiarizedSegment {
            speaker: "Speaker 1".into(),
            text: "Hello there".into(),
            start_secs: 0.0,
            end_secs: 1.5,
            confidence: Some(0.95),
        };
        let json = serde_json::to_string(&segment).unwrap();
        let parsed: DiarizedSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.speaker, segment.speaker);
        assert_eq!(parsed.text, segment.text);
        assert!((parsed.start_secs - segment.start_secs).abs() < f64::EPSILON);
        assert!((parsed.end_secs - segment.end_secs).abs() < f64::EPSILON);
        assert_eq!(parsed.confidence, segment.confidence);
    }

    #[test]
    fn diarized_segment_serde_roundtrip_no_confidence() {
        let segment = DiarizedSegment {
            speaker: "Speaker 2".into(),
            text: "Hi".into(),
            start_secs: 1.5,
            end_secs: 2.0,
            confidence: None,
        };
        let json = serde_json::to_string(&segment).unwrap();
        let parsed: DiarizedSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.speaker, segment.speaker);
        assert_eq!(parsed.text, segment.text);
        assert!(parsed.confidence.is_none());
    }
}
