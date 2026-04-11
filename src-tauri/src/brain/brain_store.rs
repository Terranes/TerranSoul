use std::fs;
use std::path::Path;

/// File name used to store the active brain model name.
const BRAIN_FILE: &str = "active_brain.txt";

/// Load the active brain model from disk.
/// Returns `None` if no brain has been configured or the file cannot be read.
pub fn load(data_dir: &Path) -> Option<String> {
    let path = data_dir.join(BRAIN_FILE);
    fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Persist the active brain model to disk.
pub fn save(data_dir: &Path, model_name: &str) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("create dir: {e}"))?;
    let path = data_dir.join(BRAIN_FILE);
    fs::write(&path, model_name.trim()).map_err(|e| format!("write brain: {e}"))
}

/// Remove the persisted brain preference, reverting to the stub agent.
pub fn clear(data_dir: &Path) -> Result<(), String> {
    let path = data_dir.join(BRAIN_FILE);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("clear brain: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_returns_none_when_no_file() {
        let dir = tempdir().unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        save(dir.path(), "gemma3:4b").unwrap();
        assert_eq!(load(dir.path()), Some("gemma3:4b".to_string()));
    }

    #[test]
    fn save_trims_whitespace() {
        let dir = tempdir().unwrap();
        save(dir.path(), "  gemma3:4b  ").unwrap();
        assert_eq!(load(dir.path()), Some("gemma3:4b".to_string()));
    }

    #[test]
    fn clear_removes_preference() {
        let dir = tempdir().unwrap();
        save(dir.path(), "gemma3:4b").unwrap();
        clear(dir.path()).unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn clear_is_idempotent_when_no_file() {
        let dir = tempdir().unwrap();
        // Should not error when file doesn't exist.
        assert!(clear(dir.path()).is_ok());
    }
}
