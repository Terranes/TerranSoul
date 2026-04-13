use async_trait::async_trait;
use reqwest::Client;

use super::{AsrEngine, SynthesisResult, TranscriptionResult, TtsEngine};

/// HTTP client for a voice sidecar process (FastAPI or similar).
///
/// Expected sidecar endpoints:
///   GET  /health         → 200 OK
///   POST /api/asr        → { "text": "...", "language": "en", "confidence": 0.95 }
///   POST /api/tts        → audio/wav binary response
///
/// The sidecar can be Open-LLM-VTuber, VibeVoice, sherpa-onnx, or any server
/// implementing this simple REST API.
pub struct SidecarVoiceClient {
    client: Client,
    base_url: String,
}

impl SidecarVoiceClient {
    pub fn new(base_url: &str) -> Self {
        let url = base_url.trim_end_matches('/').to_string();
        Self {
            client: Client::new(),
            base_url: url,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

// ── ASR via Sidecar ───────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct AsrResponse {
    text: String,
    language: Option<String>,
    confidence: Option<f64>,
}

#[async_trait]
impl AsrEngine for SidecarVoiceClient {
    fn id(&self) -> &str {
        "sidecar-asr"
    }

    fn display_name(&self) -> &str {
        "Sidecar ASR"
    }

    async fn transcribe(&self, audio: &[u8]) -> Result<TranscriptionResult, String> {
        let url = format!("{}/api/asr", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "audio/wav")
            .body(audio.to_vec())
            .send()
            .await
            .map_err(|e| format!("sidecar ASR request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!(
                "sidecar ASR returned {}",
                resp.status()
            ));
        }

        let body: AsrResponse = resp
            .json()
            .await
            .map_err(|e| format!("sidecar ASR invalid JSON: {e}"))?;

        Ok(TranscriptionResult {
            text: body.text,
            language: body.language,
            confidence: body.confidence,
        })
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

// ── TTS via Sidecar ───────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct TtsRequest<'a> {
    text: &'a str,
}

#[async_trait]
impl TtsEngine for SidecarVoiceClient {
    fn id(&self) -> &str {
        "sidecar-tts"
    }

    fn display_name(&self) -> &str {
        "Sidecar TTS"
    }

    async fn synthesize(&self, text: &str) -> Result<SynthesisResult, String> {
        let url = format!("{}/api/tts", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&TtsRequest { text })
            .send()
            .await
            .map_err(|e| format!("sidecar TTS request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!(
                "sidecar TTS returned {}",
                resp.status()
            ));
        }

        let mime = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("audio/wav")
            .to_string();

        let audio = resp
            .bytes()
            .await
            .map_err(|e| format!("sidecar TTS read body: {e}"))?
            .to_vec();

        Ok(SynthesisResult {
            audio,
            mime_type: mime,
            sample_rate: 16000, // Default; sidecar can set via header if needed
        })
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_creation() {
        let client = SidecarVoiceClient::new("http://localhost:8000");
        assert_eq!(client.base_url(), "http://localhost:8000");
    }

    #[test]
    fn client_strips_trailing_slash() {
        let client = SidecarVoiceClient::new("http://localhost:8000/");
        assert_eq!(client.base_url(), "http://localhost:8000");
    }

    #[test]
    fn asr_id_and_name() {
        let client = SidecarVoiceClient::new("http://localhost:8000");
        let engine: &dyn AsrEngine = &client;
        assert_eq!(engine.id(), "sidecar-asr");
        assert_eq!(engine.display_name(), "Sidecar ASR");
    }

    #[test]
    fn tts_id_and_name() {
        let client = SidecarVoiceClient::new("http://localhost:8000");
        let engine: &dyn TtsEngine = &client;
        assert_eq!(engine.id(), "sidecar-tts");
        assert_eq!(engine.display_name(), "Sidecar TTS");
    }

    #[tokio::test]
    async fn health_check_returns_false_when_no_server() {
        let client = SidecarVoiceClient::new("http://127.0.0.1:19999");
        let asr_healthy: bool = AsrEngine::health_check(&client).await;
        assert!(!asr_healthy);
        let tts_healthy: bool = TtsEngine::health_check(&client).await;
        assert!(!tts_healthy);
    }

    #[tokio::test]
    async fn transcribe_fails_when_no_server() {
        let client = SidecarVoiceClient::new("http://127.0.0.1:19999");
        let result = client.transcribe(&[0u8; 100]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn synthesize_fails_when_no_server() {
        let client = SidecarVoiceClient::new("http://127.0.0.1:19999");
        let result = client.synthesize("Hello").await;
        assert!(result.is_err());
    }

    #[test]
    fn asr_response_deserializes() {
        let json = r#"{"text":"hello","language":"en","confidence":0.95}"#;
        let resp: AsrResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.text, "hello");
        assert_eq!(resp.language, Some("en".to_string()));
        assert_eq!(resp.confidence, Some(0.95));
    }

    #[test]
    fn asr_response_optional_fields() {
        let json = r#"{"text":"hello"}"#;
        let resp: AsrResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.text, "hello");
        assert!(resp.language.is_none());
        assert!(resp.confidence.is_none());
    }

    #[test]
    fn tts_request_serializes() {
        let req = TtsRequest { text: "Hello world" };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Hello world"));
    }
}
