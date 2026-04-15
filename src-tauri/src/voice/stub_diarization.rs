use async_trait::async_trait;

use super::{DiarizationEngine, DiarizedSegment};

/// Stub diarization engine for testing. Always returns two mock segments
/// from "Speaker 1" and "Speaker 2".
pub struct StubDiarization;

#[async_trait]
impl DiarizationEngine for StubDiarization {
    fn id(&self) -> &str {
        "stub"
    }

    fn display_name(&self) -> &str {
        "Stub Diarization (testing)"
    }

    async fn diarize(&self, _audio: &[u8]) -> Result<Vec<DiarizedSegment>, String> {
        Ok(vec![
            DiarizedSegment {
                speaker: "Speaker 1".into(),
                text: "Hello from speaker one".into(),
                start_secs: 0.0,
                end_secs: 1.5,
                confidence: Some(0.95),
            },
            DiarizedSegment {
                speaker: "Speaker 2".into(),
                text: "Hello from speaker two".into(),
                start_secs: 1.5,
                end_secs: 3.0,
                confidence: Some(0.90),
            },
        ])
    }

    async fn health_check(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_diarization_returns_segments() {
        let engine = StubDiarization;
        let segments = engine.diarize(&[0u8; 100]).await.unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].speaker, "Speaker 1");
        assert_eq!(segments[1].speaker, "Speaker 2");
        assert!(!segments[0].text.is_empty());
        assert!(!segments[1].text.is_empty());
    }

    #[tokio::test]
    async fn stub_diarization_health() {
        let engine = StubDiarization;
        assert!(engine.health_check().await);
    }

    #[test]
    fn stub_diarization_id_and_name() {
        let engine = StubDiarization;
        assert_eq!(engine.id(), "stub");
        assert_eq!(engine.display_name(), "Stub Diarization (testing)");
    }

    #[tokio::test]
    async fn stub_diarization_segments_have_valid_times() {
        let engine = StubDiarization;
        let segments = engine.diarize(&[0u8; 50]).await.unwrap();
        for seg in &segments {
            assert!(seg.end_secs > seg.start_secs);
            assert!(seg.confidence.unwrap_or(0.0) >= 0.0);
            assert!(seg.confidence.unwrap_or(0.0) <= 1.0);
        }
    }
}
