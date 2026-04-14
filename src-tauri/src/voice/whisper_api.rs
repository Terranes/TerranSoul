//! OpenAI Whisper API ASR engine.
//!
//! Sends audio to the OpenAI `/v1/audio/transcriptions` endpoint using
//! `reqwest` multipart form data. Requires an API key.

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

use super::{AsrEngine, TranscriptionResult};

/// Default API endpoint.
const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1/audio/transcriptions";

/// The model name expected by the API.
const MODEL: &str = "whisper-1";

/// OpenAI Whisper API ASR engine.
pub struct WhisperApi {
    api_key: String,
    endpoint: String,
    client: reqwest::Client,
}

impl WhisperApi {
    /// Create a new Whisper API engine with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let mut headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
            headers.insert(AUTHORIZATION, val);
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();
        Self {
            api_key,
            endpoint: DEFAULT_ENDPOINT.to_string(),
            client,
        }
    }

    /// Create a new Whisper API engine with a custom endpoint (for compatible APIs).
    #[allow(dead_code)]
    pub fn with_endpoint(api_key: impl Into<String>, endpoint: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let endpoint = endpoint.into();
        let mut headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
            headers.insert(AUTHORIZATION, val);
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();
        Self {
            api_key,
            endpoint,
            client,
        }
    }
}

/// Response shape from the OpenAI transcription endpoint.
#[derive(serde::Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// Wrap raw PCM 16-bit mono 16kHz bytes in a WAV container.
fn pcm_to_wav_16k(pcm: &[u8]) -> Vec<u8> {
    let sample_rate: u32 = 16000;
    let data_size = pcm.len() as u32;
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend_from_slice(pcm);
    wav
}

#[async_trait]
impl AsrEngine for WhisperApi {
    fn id(&self) -> &str {
        "whisper-api"
    }

    fn display_name(&self) -> &str {
        "OpenAI Whisper API"
    }

    async fn transcribe(&self, audio: &[u8]) -> Result<TranscriptionResult, String> {
        use reqwest::multipart::{Form, Part};

        // Build WAV from raw PCM (16-bit mono 16kHz) if no WAV header present.
        let audio_bytes = if audio.len() >= 4 && &audio[..4] == b"RIFF" {
            audio.to_vec()
        } else {
            pcm_to_wav_16k(audio)
        };

        let file_part = Part::bytes(audio_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| format!("Whisper API form: {e}"))?;

        let form = Form::new()
            .text("model", MODEL)
            .text("response_format", "json")
            .part("file", file_part);

        let resp = self
            .client
            .post(&self.endpoint)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Whisper API request: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Whisper API {status}: {body}"));
        }

        let result: TranscriptionResponse = resp
            .json()
            .await
            .map_err(|e| format!("Whisper API parse: {e}"))?;

        Ok(TranscriptionResult {
            text: result.text,
            language: None,
            confidence: None,
        })
    }

    async fn health_check(&self) -> bool {
        !self.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisper_api_id_and_name() {
        let engine = WhisperApi::new("sk-test");
        assert_eq!(engine.id(), "whisper-api");
        assert_eq!(engine.display_name(), "OpenAI Whisper API");
    }

    #[test]
    fn whisper_api_custom_endpoint() {
        let engine = WhisperApi::with_endpoint("sk-test", "http://localhost:8000/transcribe");
        assert_eq!(engine.endpoint, "http://localhost:8000/transcribe");
    }

    #[tokio::test]
    async fn whisper_api_health_with_key() {
        let engine = WhisperApi::new("sk-test");
        assert!(engine.health_check().await);
    }

    #[tokio::test]
    async fn whisper_api_health_without_key() {
        let engine = WhisperApi::new("");
        assert!(!engine.health_check().await);
    }
}
