use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// File name for the JSON brain configuration.
const BRAIN_CONFIG_FILE: &str = "brain_config.json";

/// The three-tier brain mode: Free cloud API, Paid cloud API, or Local Ollama.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode")]
pub enum BrainMode {
    /// Use a free cloud LLM API provider (zero-setup default).
    #[serde(rename = "free_api")]
    FreeApi {
        /// ID of the free provider from the catalogue (e.g. "groq").
        provider_id: String,
        /// Optional API key (some free providers require a free-tier key).
        api_key: Option<String>,
    },
    /// Use a paid cloud API (user provides their own API key).
    #[serde(rename = "paid_api")]
    PaidApi {
        /// Provider identifier (e.g. "openai", "anthropic").
        provider: String,
        /// User-supplied API key.
        api_key: String,
        /// Model to use (e.g. "gpt-4o").
        model: String,
        /// Base URL for the API.
        base_url: String,
    },
    /// Use a locally running Ollama instance.
    #[serde(rename = "local_ollama")]
    LocalOllama {
        /// Ollama model tag (e.g. "gemma3:4b").
        model: String,
    },
}

impl Default for BrainMode {
    fn default() -> Self {
        // Default to free API with Groq as the first provider
        BrainMode::FreeApi {
            provider_id: "groq".to_string(),
            api_key: None,
        }
    }
}

/// Load the brain configuration from disk.
///
/// Returns `None` if no config file exists or it cannot be parsed.
/// Also checks for legacy `active_brain.txt` and migrates if found.
pub fn load(data_dir: &Path) -> Option<BrainMode> {
    let config_path = data_dir.join(BRAIN_CONFIG_FILE);

    // Try new JSON config first
    if config_path.exists() {
        if let Ok(contents) = fs::read_to_string(&config_path) {
            if let Ok(mode) = serde_json::from_str::<BrainMode>(&contents) {
                return Some(mode);
            }
        }
    }

    // Fall back to legacy active_brain.txt (migrate Ollama users)
    let legacy_path = data_dir.join("active_brain.txt");
    if legacy_path.exists() {
        if let Ok(model) = fs::read_to_string(&legacy_path) {
            let model = model.trim().to_string();
            if !model.is_empty() {
                return Some(BrainMode::LocalOllama { model });
            }
        }
    }

    None
}

/// Persist the brain configuration to disk as JSON.
pub fn save(data_dir: &Path, mode: &BrainMode) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("create dir: {e}"))?;
    let config_path = data_dir.join(BRAIN_CONFIG_FILE);
    let json = serde_json::to_string_pretty(mode).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&config_path, json).map_err(|e| format!("write brain config: {e}"))
}

/// Remove the persisted brain configuration, reverting to no brain (stub agent).
pub fn clear(data_dir: &Path) -> Result<(), String> {
    let config_path = data_dir.join(BRAIN_CONFIG_FILE);
    if config_path.exists() {
        fs::remove_file(&config_path).map_err(|e| format!("clear brain config: {e}"))?;
    }
    // Also remove legacy file if it exists
    let legacy_path = data_dir.join("active_brain.txt");
    if legacy_path.exists() {
        let _ = fs::remove_file(&legacy_path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_is_free_api_groq() {
        let mode = BrainMode::default();
        match &mode {
            BrainMode::FreeApi { provider_id, .. } => assert_eq!(provider_id, "groq"),
            _ => panic!("default should be FreeApi"),
        }
    }

    #[test]
    fn load_returns_none_when_no_file() {
        let dir = tempdir().unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn save_and_load_free_api() {
        let dir = tempdir().unwrap();
        let mode = BrainMode::FreeApi {
            provider_id: "cerebras".into(),
            api_key: Some("csk-test".into()),
        };
        save(dir.path(), &mode).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, mode);
    }

    #[test]
    fn save_and_load_paid_api() {
        let dir = tempdir().unwrap();
        let mode = BrainMode::PaidApi {
            provider: "openai".into(),
            api_key: "sk-test".into(),
            model: "gpt-4o".into(),
            base_url: "https://api.openai.com".into(),
        };
        save(dir.path(), &mode).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, mode);
    }

    #[test]
    fn save_and_load_local_ollama() {
        let dir = tempdir().unwrap();
        let mode = BrainMode::LocalOllama {
            model: "gemma3:4b".into(),
        };
        save(dir.path(), &mode).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, mode);
    }

    #[test]
    fn clear_removes_config() {
        let dir = tempdir().unwrap();
        let mode = BrainMode::FreeApi {
            provider_id: "groq".into(),
            api_key: None,
        };
        save(dir.path(), &mode).unwrap();
        clear(dir.path()).unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn clear_is_idempotent() {
        let dir = tempdir().unwrap();
        assert!(clear(dir.path()).is_ok());
    }

    #[test]
    fn legacy_migration_from_active_brain_txt() {
        let dir = tempdir().unwrap();
        let legacy_path = dir.path().join("active_brain.txt");
        fs::write(&legacy_path, "gemma3:4b").unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(
            loaded,
            BrainMode::LocalOllama {
                model: "gemma3:4b".into()
            }
        );
    }

    #[test]
    fn new_config_takes_priority_over_legacy() {
        let dir = tempdir().unwrap();
        // Write legacy file
        fs::write(dir.path().join("active_brain.txt"), "phi-4:latest").unwrap();
        // Write new config
        let mode = BrainMode::FreeApi {
            provider_id: "groq".into(),
            api_key: None,
        };
        save(dir.path(), &mode).unwrap();
        // New config should win
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, mode);
    }

    #[test]
    fn serde_tagged_json_format() {
        let mode = BrainMode::FreeApi {
            provider_id: "groq".into(),
            api_key: None,
        };
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains(r#""mode":"free_api""#));
        assert!(json.contains(r#""provider_id":"groq""#));
    }

    #[test]
    fn serde_all_variants_roundtrip() {
        let variants = vec![
            BrainMode::FreeApi {
                provider_id: "cerebras".into(),
                api_key: Some("key".into()),
            },
            BrainMode::PaidApi {
                provider: "anthropic".into(),
                api_key: "sk-ant-test".into(),
                model: "claude-3-opus".into(),
                base_url: "https://api.anthropic.com".into(),
            },
            BrainMode::LocalOllama {
                model: "phi-4:latest".into(),
            },
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let parsed: BrainMode = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, v);
        }
    }

    #[test]
    fn clear_also_removes_legacy_file() {
        let dir = tempdir().unwrap();
        let legacy_path = dir.path().join("active_brain.txt");
        fs::write(&legacy_path, "gemma3:4b").unwrap();
        let mode = BrainMode::FreeApi {
            provider_id: "groq".into(),
            api_key: None,
        };
        save(dir.path(), &mode).unwrap();
        clear(dir.path()).unwrap();
        assert!(!legacy_path.exists());
        assert!(load(dir.path()).is_none());
    }
}
