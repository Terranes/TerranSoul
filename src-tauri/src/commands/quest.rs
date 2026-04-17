//! Tauri commands for quest tracker persistence.
//!
//! The quest tracker stores user-specific metadata (dismissed quests, pinned
//! quests, daily AI suggestions, activation timestamps). Skill completion
//! itself is derived from actual feature state (brain, voice, etc.) and is
//! NOT stored here — this means fresh installs auto-recover progress.
//!
//! Storage: simple JSON file in the app data directory. The frontend also
//! mirrors data to localStorage for cross-platform (browser/mobile) fallback.

use tauri::State;

use crate::AppState;

const QUEST_TRACKER_FILE: &str = "quest_tracker.json";

/// Return the raw quest tracker JSON string, or "{}" if none exists.
#[tauri::command]
pub async fn get_quest_tracker(state: State<'_, AppState>) -> Result<String, String> {
    let path = state.data_dir.join(QUEST_TRACKER_FILE);
    match std::fs::read_to_string(&path) {
        Ok(contents) => Ok(contents),
        Err(_) => Ok("{}".to_string()),
    }
}

/// Persist the quest tracker JSON string to disk.
#[tauri::command]
pub async fn save_quest_tracker(data: String, state: State<'_, AppState>) -> Result<(), String> {
    // Validate that it's valid JSON before writing
    serde_json::from_str::<serde_json::Value>(&data)
        .map_err(|e| format!("Invalid quest tracker JSON: {e}"))?;
    let path = state.data_dir.join(QUEST_TRACKER_FILE);
    // Ensure parent directory exists (survives data_dir wipes / first-launch)
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create quest tracker directory: {e}"))?;
    }
    std::fs::write(&path, &data).map_err(|e| format!("Failed to write quest tracker: {e}"))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    #[test]
    fn get_quest_tracker_returns_empty_when_no_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(super::QUEST_TRACKER_FILE);
        assert!(!path.exists());
        // Simulate the read logic
        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => "{}".to_string(),
        };
        assert_eq!(contents, "{}");
    }

    #[test]
    fn save_and_load_quest_tracker_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(super::QUEST_TRACKER_FILE);
        let data = r#"{"version":1,"dismissedQuestIds":["bgm"],"pinnedQuestIds":["tts"]}"#;
        std::fs::write(&path, data).unwrap();
        let loaded = std::fs::read_to_string(&path).unwrap();
        assert_eq!(loaded, data);
    }

    #[test]
    fn save_quest_tracker_rejects_invalid_json() {
        let bad = "not valid json{{{";
        let result = serde_json::from_str::<serde_json::Value>(bad);
        assert!(result.is_err());
    }

    #[test]
    fn save_quest_tracker_accepts_valid_json() {
        let good = r#"{"version":1,"dailySuggestionIds":["asr","tts"]}"#;
        let result = serde_json::from_str::<serde_json::Value>(good);
        assert!(result.is_ok());
    }

    #[test]
    fn quest_tracker_survives_reinstall_concept() {
        // Key design principle: quest completion is derived from actual feature
        // state, not stored in the tracker file. The tracker only holds metadata.
        // This test validates that a "fresh" empty tracker still works.
        let empty = "{}";
        let parsed: serde_json::Value = serde_json::from_str(empty).unwrap();
        assert!(parsed.is_object());
    }
}
