//! Edge TTS engine — free, high-quality Microsoft neural voices.
//!
//! Uses the `msedge-tts` crate which speaks the same WebSocket protocol as
//! Microsoft Edge's "Read Aloud" feature. No API key required.
//!
//! The sync `msedge-tts` client is used inside `spawn_blocking` to avoid
//! conflicts with Tauri's Tokio runtime (the crate's async API uses async-std).

use async_trait::async_trait;

use super::{SynthesisResult, TtsEngine};

/// Default voice — high-quality English (US) female neural voice.
const DEFAULT_VOICE: &str = "en-US-AriaNeural";

/// Output format: raw PCM 24 kHz 16-bit mono (easy to wrap in a WAV header).
const AUDIO_FORMAT: &str = "raw-24khz-16bit-mono-pcm";

/// Sample rate matching the audio format.
const SAMPLE_RATE: u32 = 24000;

/// Edge TTS engine that synthesizes text using Microsoft's free neural voices.
pub struct EdgeTts {
    voice_name: String,
    /// Pitch offset in Hz (e.g. 50 = +50Hz higher).
    pitch: i32,
    /// Rate offset in percent (e.g. 15 = +15% faster).
    rate: i32,
}

impl EdgeTts {
    /// Create an Edge TTS engine with the default voice.
    pub fn new() -> Self {
        Self {
            voice_name: DEFAULT_VOICE.to_string(),
            pitch: 0,
            rate: 0,
        }
    }

    /// Create an Edge TTS engine with a specific voice name.
    #[allow(dead_code)]
    pub fn with_voice(voice_name: impl Into<String>) -> Self {
        Self {
            voice_name: voice_name.into(),
            pitch: 0,
            rate: 0,
        }
    }

    /// Create an Edge TTS engine with voice, pitch and rate.
    pub fn with_prosody(voice_name: impl Into<String>, pitch: i32, rate: i32) -> Self {
        Self {
            voice_name: voice_name.into(),
            pitch,
            rate,
        }
    }
}

impl Default for EdgeTts {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrap raw PCM bytes in a WAV container.
fn pcm_to_wav(pcm: &[u8], sample_rate: u32) -> Vec<u8> {
    let data_size = pcm.len() as u32;
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + pcm.len());
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
    wav.extend_from_slice(pcm);
    wav
}

#[async_trait]
impl TtsEngine for EdgeTts {
    fn id(&self) -> &str {
        "edge-tts"
    }

    fn display_name(&self) -> &str {
        "Edge TTS (free)"
    }

    async fn synthesize(&self, text: &str) -> Result<SynthesisResult, String> {
        let voice_name = self.voice_name.clone();
        let text = text.to_string();
        let pitch = self.pitch;
        let rate = self.rate;

        // Run the sync WebSocket client on a blocking thread to avoid
        // conflicting with the Tokio runtime.
        let result = tokio::task::spawn_blocking(move || {
            use msedge_tts::tts::{client::connect, SpeechConfig};

            let config = SpeechConfig {
                voice_name,
                audio_format: AUDIO_FORMAT.to_string(),
                pitch,
                rate,
                volume: 0,
            };

            let mut client = connect().map_err(|e| format!("Edge TTS connect: {e}"))?;
            let audio = client
                .synthesize(&text, &config)
                .map_err(|e| format!("Edge TTS synthesize: {e}"))?;

            Ok::<Vec<u8>, String>(audio.audio_bytes)
        })
        .await
        .map_err(|e| format!("Edge TTS task join: {e}"))??;

        let wav = pcm_to_wav(&result, SAMPLE_RATE);

        Ok(SynthesisResult {
            audio: wav,
            mime_type: "audio/wav".into(),
            sample_rate: SAMPLE_RATE,
        })
    }

    async fn health_check(&self) -> bool {
        // Synthesize a tiny text to verify the connection works.
        let voice_name = self.voice_name.clone();
        let pitch = self.pitch;
        let rate = self.rate;
        let ok = tokio::task::spawn_blocking(move || {
            use msedge_tts::tts::{client::connect, SpeechConfig};

            let config = SpeechConfig {
                voice_name,
                audio_format: AUDIO_FORMAT.to_string(),
                pitch,
                rate,
                volume: 0,
            };

            connect()
                .and_then(|mut c| c.synthesize(".", &config))
                .is_ok()
        })
        .await
        .unwrap_or(false);
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_tts_id_and_name() {
        let engine = EdgeTts::new();
        assert_eq!(engine.id(), "edge-tts");
        assert_eq!(engine.display_name(), "Edge TTS (free)");
    }

    #[test]
    fn edge_tts_custom_voice() {
        let engine = EdgeTts::with_voice("ja-JP-NanamiNeural");
        assert_eq!(engine.voice_name, "ja-JP-NanamiNeural");
        assert_eq!(engine.pitch, 0);
        assert_eq!(engine.rate, 0);
    }

    #[test]
    fn edge_tts_with_prosody() {
        let engine = EdgeTts::with_prosody("en-US-AnaNeural", 50, 15);
        assert_eq!(engine.voice_name, "en-US-AnaNeural");
        assert_eq!(engine.pitch, 50);
        assert_eq!(engine.rate, 15);
    }

    #[test]
    fn edge_tts_default_has_zero_prosody() {
        let engine = EdgeTts::new();
        assert_eq!(engine.pitch, 0);
        assert_eq!(engine.rate, 0);
    }

    #[test]
    fn pcm_to_wav_creates_valid_header() {
        // 100 samples of silence
        let pcm = vec![0u8; 200]; // 100 samples × 2 bytes
        let wav = pcm_to_wav(&pcm, 24000);

        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
        // Data size at offset 40
        let data_size = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]);
        assert_eq!(data_size, 200);
        // Total length: 44 header + 200 data
        assert_eq!(wav.len(), 244);
    }

    #[test]
    fn pcm_to_wav_sample_rate() {
        let pcm = vec![0u8; 100];
        let wav = pcm_to_wav(&pcm, 24000);

        // Sample rate at offset 24
        let sr = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
        assert_eq!(sr, 24000);
    }
}
