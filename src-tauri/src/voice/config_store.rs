use std::fs;
use std::path::Path;

use super::VoiceConfig;

/// File name used to store voice provider configuration.
const VOICE_CONFIG_FILE: &str = "voice_config.json";

/// Load the voice configuration from disk.
/// Returns the default (empty) config if not yet configured.
pub fn load(data_dir: &Path) -> VoiceConfig {
    let path = data_dir.join(VOICE_CONFIG_FILE);
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist the voice configuration to disk.
pub fn save(data_dir: &Path, config: &VoiceConfig) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("create dir: {e}"))?;
    let path = data_dir.join(VOICE_CONFIG_FILE);
    let json = serde_json::to_string_pretty(config).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write voice config: {e}"))
}

/// Remove the persisted voice config, reverting to text-only mode.
pub fn clear(data_dir: &Path) -> Result<(), String> {
    let path = data_dir.join(VOICE_CONFIG_FILE);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("clear voice config: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_returns_default_when_no_file() {
        let dir = tempdir().unwrap();
        let cfg = load(dir.path());
        assert_eq!(cfg, VoiceConfig::default());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let cfg = VoiceConfig {
            asr_provider: Some("whisper-api".into()),
            tts_provider: Some("edge-tts".into()),
            api_key: None,
            endpoint_url: None,
            hotwords: vec![],
        };
        save(dir.path(), &cfg).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn save_with_all_fields() {
        let dir = tempdir().unwrap();
        let cfg = VoiceConfig {
            asr_provider: Some("whisper-api".into()),
            tts_provider: Some("openai-tts".into()),
            api_key: Some("sk-test-key".into()),
            endpoint_url: Some("https://api.openai.com/v1".into()),
            hotwords: vec![],
        };
        save(dir.path(), &cfg).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn clear_removes_config() {
        let dir = tempdir().unwrap();
        let cfg = VoiceConfig {
            asr_provider: Some("web-speech".into()),
            tts_provider: None,
            api_key: None,
            endpoint_url: None,
            hotwords: vec![],
        };
        save(dir.path(), &cfg).unwrap();
        clear(dir.path()).unwrap();
        assert_eq!(load(dir.path()), VoiceConfig::default());
    }

    #[test]
    fn clear_is_idempotent_when_no_file() {
        let dir = tempdir().unwrap();
        assert!(clear(dir.path()).is_ok());
    }

    #[test]
    fn load_ignores_corrupt_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("voice_config.json");
        fs::write(&path, "not valid json{{{").unwrap();
        let cfg = load(dir.path());
        assert_eq!(cfg, VoiceConfig::default());
    }
}
