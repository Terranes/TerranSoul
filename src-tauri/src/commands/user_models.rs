//! Tauri commands for managing user-imported VRM models.
//!
//! User-imported VRMs are copied into `<app_data_dir>/user_models/<id>.vrm`
//! so they survive a fresh install / re-build / app upgrade. Metadata is
//! stored in `app_settings.json` under the `user_models` field.
//!
//! The frontend never reads the file path directly — it requests bytes via
//! [`read_user_model_bytes`] and wraps them in a `Blob` URL. This avoids
//! widening Tauri's asset-protocol scope and works identically across
//! Windows, macOS, and Linux.

use std::fs;
use std::path::{Path, PathBuf};

use tauri::State;
use uuid::Uuid;

use crate::settings::{config_store, UserModel};
use crate::AppState;

const USER_MODELS_DIR: &str = "user_models";
const VRM_EXTENSION: &str = "vrm";

/// Maximum allowed VRM file size (256 MiB). Prevents accidentally importing
/// a multi-gigabyte file that would exhaust memory.
const MAX_VRM_BYTES: u64 = 256 * 1024 * 1024;

/// Return the directory holding user-imported VRMs.
fn user_models_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(USER_MODELS_DIR)
}

/// Resolve the on-disk path for a user model id. Does not check existence.
fn user_model_path(data_dir: &Path, id: &str) -> PathBuf {
    user_models_dir(data_dir).join(format!("{id}.{VRM_EXTENSION}"))
}

/// Validate that an id is a plain UUID-like token (alphanumeric + hyphens
/// only) so it cannot escape the `user_models/` directory.
fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 64 {
        return Err("invalid user model id".into());
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("invalid user model id".into());
    }
    Ok(())
}

/// Derive a friendly display name from the original filename
/// (strip extension, replace separators with spaces).
fn derive_name(original_filename: &str) -> String {
    let stem = Path::new(original_filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(original_filename);
    let cleaned = stem.replace(['_', '-'], " ").trim().to_string();
    if cleaned.is_empty() {
        "Imported VRM".to_string()
    } else {
        cleaned
    }
}

fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Pure helper used by both the Tauri command and unit tests: copy the VRM
/// at `source_path` into `data_dir/user_models/`, append metadata, and
/// persist to `app_settings.json`.
pub fn import_user_model_inner(
    data_dir: &Path,
    settings: &mut crate::settings::AppSettings,
    source_path: &Path,
) -> Result<UserModel, String> {
    if !source_path.exists() {
        return Err(format!(
            "source file does not exist: {}",
            source_path.display()
        ));
    }
    let metadata = fs::metadata(source_path).map_err(|e| format!("read source metadata: {e}"))?;
    if !metadata.is_file() {
        return Err("source path is not a regular file".into());
    }
    if metadata.len() > MAX_VRM_BYTES {
        return Err(format!(
            "VRM file is too large ({} bytes; max {})",
            metadata.len(),
            MAX_VRM_BYTES
        ));
    }
    let original_filename = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("imported.vrm")
        .to_string();

    let dir = user_models_dir(data_dir);
    fs::create_dir_all(&dir).map_err(|e| format!("create user_models dir: {e}"))?;

    let id = Uuid::new_v4().to_string();
    let dest = user_model_path(data_dir, &id);
    fs::copy(source_path, &dest).map_err(|e| format!("copy VRM to {}: {e}", dest.display()))?;

    let entry = UserModel {
        id: id.clone(),
        name: derive_name(&original_filename),
        original_filename,
        gender: "female".to_string(),
        imported_at: current_unix_ms(),
    };
    settings.user_models.push(entry.clone());
    config_store::save(data_dir, settings)?;
    Ok(entry)
}

/// Import a VRM from `source_path` into the per-user data directory.
#[tauri::command(rename_all = "camelCase")]
pub async fn import_user_model(
    source_path: String,
    state: State<'_, AppState>,
) -> Result<UserModel, String> {
    let data_dir = state.data_dir.clone();
    let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    import_user_model_inner(&data_dir, &mut settings, Path::new(&source_path))
}

/// Return the persisted list of user-imported models.
#[tauri::command]
pub async fn list_user_models(state: State<'_, AppState>) -> Result<Vec<UserModel>, String> {
    let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.user_models.clone())
}

/// Delete a user-imported model: remove the file and the settings entry.
#[tauri::command]
pub async fn delete_user_model(id: String, state: State<'_, AppState>) -> Result<(), String> {
    validate_id(&id)?;
    let data_dir = state.data_dir.clone();
    let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    let original_len = settings.user_models.len();
    settings.user_models.retain(|m| m.id != id);
    if settings.user_models.len() == original_len {
        return Err(format!("user model not found: {id}"));
    }
    let path = user_model_path(&data_dir, &id);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("remove {}: {e}", path.display()))?;
    }
    config_store::save(&data_dir, &settings)?;
    Ok(())
}

/// Read the raw bytes of a user-imported model. The frontend wraps these
/// in a `Blob` URL for `GLTFLoader`.
#[tauri::command]
pub async fn read_user_model_bytes(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<u8>, String> {
    validate_id(&id)?;
    let data_dir = state.data_dir.clone();
    // Confirm the id is registered before touching the filesystem so we
    // never leak files that were orphaned outside our settings.
    {
        let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        if !settings.user_models.iter().any(|m| m.id == id) {
            return Err(format!("user model not found: {id}"));
        }
    }
    let path = user_model_path(&data_dir, &id);
    fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))
}

/// Update the friendly name and/or gender of a user-imported model.
#[tauri::command(rename_all = "camelCase")]
pub async fn update_user_model(
    id: String,
    name: Option<String>,
    gender: Option<String>,
    state: State<'_, AppState>,
) -> Result<UserModel, String> {
    validate_id(&id)?;
    let data_dir = state.data_dir.clone();
    let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    let entry = settings
        .user_models
        .iter_mut()
        .find(|m| m.id == id)
        .ok_or_else(|| format!("user model not found: {id}"))?;
    if let Some(n) = name {
        let trimmed = n.trim();
        if !trimmed.is_empty() {
            entry.name = trimmed.to_string();
        }
    }
    if let Some(g) = gender {
        if g == "female" || g == "male" {
            entry.gender = g;
        }
    }
    let updated = entry.clone();
    config_store::save(&data_dir, &settings)?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_dummy_vrm(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn validate_id_accepts_uuid_like() {
        assert!(validate_id("abc-123-def").is_ok());
        assert!(validate_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
    }

    #[test]
    fn validate_id_rejects_path_traversal() {
        assert!(validate_id("../etc/passwd").is_err());
        assert!(validate_id("a/b").is_err());
        assert!(validate_id("a\\b").is_err());
        assert!(validate_id("").is_err());
    }

    #[test]
    fn derive_name_strips_extension_and_separators() {
        assert_eq!(derive_name("My_Model-V2.vrm"), "My Model V2");
        assert_eq!(derive_name("plain.vrm"), "plain");
        assert_eq!(derive_name(""), "Imported VRM");
    }

    #[test]
    fn import_copies_file_and_appends_metadata() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.vrm");
        write_dummy_vrm(&src, b"VRM-fake-bytes");

        let mut settings = crate::settings::AppSettings::default();
        let entry = import_user_model_inner(dir.path(), &mut settings, &src).unwrap();

        assert_eq!(entry.original_filename, "source.vrm");
        assert_eq!(entry.name, "source");
        assert_eq!(entry.gender, "female");
        assert_eq!(settings.user_models.len(), 1);
        let copied = user_model_path(dir.path(), &entry.id);
        assert!(copied.exists());
        assert_eq!(fs::read(&copied).unwrap(), b"VRM-fake-bytes");
        // Settings file written
        assert!(dir.path().join("app_settings.json").exists());
    }

    #[test]
    fn import_rejects_missing_source() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("nope.vrm");
        let mut settings = crate::settings::AppSettings::default();
        let res = import_user_model_inner(dir.path(), &mut settings, &src);
        assert!(res.is_err());
        assert_eq!(settings.user_models.len(), 0);
    }

    #[test]
    fn import_two_files_creates_two_entries() {
        let dir = tempdir().unwrap();
        let src1 = dir.path().join("a.vrm");
        let src2 = dir.path().join("b.vrm");
        write_dummy_vrm(&src1, b"A");
        write_dummy_vrm(&src2, b"B");

        let mut settings = crate::settings::AppSettings::default();
        let e1 = import_user_model_inner(dir.path(), &mut settings, &src1).unwrap();
        let e2 = import_user_model_inner(dir.path(), &mut settings, &src2).unwrap();
        assert_ne!(e1.id, e2.id);
        assert_eq!(settings.user_models.len(), 2);
    }

    #[test]
    fn user_models_persist_across_load() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.vrm");
        write_dummy_vrm(&src, b"hello");

        let mut settings = crate::settings::AppSettings::default();
        let entry = import_user_model_inner(dir.path(), &mut settings, &src).unwrap();
        // Drop in-memory settings, reload from disk
        drop(settings);
        let _lock = crate::settings::ENV_MUTEX.lock().unwrap();
        let reloaded = config_store::load(dir.path());
        assert_eq!(reloaded.user_models.len(), 1);
        assert_eq!(reloaded.user_models[0].id, entry.id);
        assert_eq!(reloaded.user_models[0].original_filename, "source.vrm");
    }

    #[test]
    fn user_model_path_stays_inside_user_models_dir() {
        let dir = tempdir().unwrap();
        let path = user_model_path(dir.path(), "abc-123");
        assert!(path.starts_with(dir.path().join("user_models")));
    }
}
