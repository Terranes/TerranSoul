use async_trait::async_trait;

use super::{SynthesisResult, TtsEngine};

/// Stub TTS engine for testing. Returns a minimal silent WAV.
pub struct StubTts;

/// Generate a minimal valid WAV file header with silence (all zero samples).
fn silent_wav(duration_ms: u32) -> Vec<u8> {
    let sample_rate: u32 = 16000;
    let num_samples = (sample_rate * duration_ms) / 1000;
    let data_size = num_samples * 2; // 16-bit PCM
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + data_size as usize);
    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    wav.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
                                                 // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.resize(44 + data_size as usize, 0); // zero samples = silence
    wav
}

#[async_trait]
impl TtsEngine for StubTts {
    fn id(&self) -> &str {
        "stub"
    }

    fn display_name(&self) -> &str {
        "Stub TTS (testing)"
    }

    async fn synthesize(&self, _text: &str) -> Result<SynthesisResult, String> {
        Ok(SynthesisResult {
            audio: silent_wav(500), // 500ms silence
            mime_type: "audio/wav".into(),
            sample_rate: 16000,
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
    async fn stub_tts_synthesizes() {
        let engine = StubTts;
        let result = engine.synthesize("Hello").await.unwrap();
        assert_eq!(result.mime_type, "audio/wav");
        assert_eq!(result.sample_rate, 16000);
        // WAV header is 44 bytes + 500ms × 16000 × 2 = 16000 bytes of data
        assert_eq!(result.audio.len(), 44 + 16000);
        // Verify WAV magic bytes
        assert_eq!(&result.audio[..4], b"RIFF");
        assert_eq!(&result.audio[8..12], b"WAVE");
    }

    #[tokio::test]
    async fn stub_tts_health() {
        let engine = StubTts;
        assert!(engine.health_check().await);
    }

    #[test]
    fn stub_tts_id_and_name() {
        let engine = StubTts;
        assert_eq!(engine.id(), "stub");
        assert_eq!(engine.display_name(), "Stub TTS (testing)");
    }

    #[test]
    fn silent_wav_has_valid_header() {
        let wav = silent_wav(100); // 100ms
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
    }
}
