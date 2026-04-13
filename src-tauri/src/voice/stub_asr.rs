use async_trait::async_trait;

use super::{AsrEngine, TranscriptionResult};

/// Stub ASR engine for testing. Always returns a fixed transcription.
pub struct StubAsr;

#[async_trait]
impl AsrEngine for StubAsr {
    fn id(&self) -> &str {
        "stub"
    }

    fn display_name(&self) -> &str {
        "Stub ASR (testing)"
    }

    async fn transcribe(&self, _audio: &[u8]) -> Result<TranscriptionResult, String> {
        Ok(TranscriptionResult {
            text: "Hello from stub ASR".into(),
            language: Some("en".into()),
            confidence: Some(1.0),
        })
    }

    async fn health_check(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_asr_transcribes() {
        let engine = StubAsr;
        let result = engine.transcribe(&[0u8; 100]).await.unwrap();
        assert_eq!(result.text, "Hello from stub ASR");
        assert_eq!(result.language, Some("en".to_string()));
        assert_eq!(result.confidence, Some(1.0));
    }

    #[tokio::test]
    async fn stub_asr_health() {
        let engine = StubAsr;
        assert!(engine.health_check().await);
    }

    #[test]
    fn stub_asr_id_and_name() {
        let engine = StubAsr;
        assert_eq!(engine.id(), "stub");
        assert_eq!(engine.display_name(), "Stub ASR (testing)");
    }
}
