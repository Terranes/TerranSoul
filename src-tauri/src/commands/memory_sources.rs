//! Tauri command surface for the memory-sources registry
//! (BRAIN-REPO-RAG-1a). Thin wrappers over `crate::memory::sources` that
//! lock the shared `MemoryStore` connection.

use tauri::State;

use crate::memory::sources::{
    self, MemorySource, MemorySourceKind, SELF_SOURCE_ID,
};
use crate::AppState;

#[tauri::command]
pub async fn list_memory_sources(
    state: State<'_, AppState>,
) -> Result<Vec<MemorySource>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    sources::list_sources(store.conn()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_memory_source(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<MemorySource>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    sources::get_source(store.conn(), &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_memory_source(
    id: String,
    kind: String,
    label: String,
    repo_url: Option<String>,
    repo_ref: Option<String>,
    state: State<'_, AppState>,
) -> Result<MemorySource, String> {
    let parsed_kind: MemorySourceKind = match kind.as_str() {
        "repo" => MemorySourceKind::Repo,
        "topic" => MemorySourceKind::Topic,
        "self" => {
            return Err(format!(
                "kind 'self' is reserved (id '{SELF_SOURCE_ID}' is seeded automatically)"
            ));
        }
        other => return Err(format!("unknown memory source kind: {other}")),
    };
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    sources::create_source(
        store.conn(),
        &id,
        parsed_kind,
        &label,
        repo_url.as_deref(),
        repo_ref.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_memory_source(
    id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    sources::delete_source(store.conn(), &id).map_err(|e| e.to_string())
}

// ────────────────────────────────────────────────────────────────────────────
// REPO-PACK: import/export per-source knowledge bundles (.tsbrain files).
// ────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "repo-rag")]
use std::path::PathBuf;

#[cfg(feature = "repo-rag")]
use crate::memory::repo_pack::{
    self, ExportSummary, ImportMode, ImportSummary,
};

/// Default folder where exported `.tsbrain` packs are written if the
/// caller doesn't supply an explicit `outputPath`. Lives next to other
/// per-source data, separate from the user's personal brain.
#[cfg(feature = "repo-rag")]
fn default_export_dir(state: &AppState) -> PathBuf {
    state.data_dir.join("exports")
}

#[cfg(feature = "repo-rag")]
fn default_export_filename(source_id: &str) -> String {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{source_id}-{stamp}.tsbrain")
}

#[cfg(feature = "repo-rag")]
#[tauri::command]
pub async fn export_repo_source(
    source_id: String,
    output_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<ExportSummary, String> {
    let resolved: PathBuf = match output_path {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => default_export_dir(&state).join(default_export_filename(&source_id)),
    };
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let exporter = env!("CARGO_PKG_VERSION");
    repo_pack::export_source(store.conn(), &state.data_dir, &source_id, &resolved, exporter)
        .map_err(|e| e.to_string())
}

#[cfg(feature = "repo-rag")]
#[tauri::command]
pub async fn import_repo_source(
    archive_path: String,
    mode: String,
    target_source_id: Option<String>,
    label_override: Option<String>,
    state: State<'_, AppState>,
) -> Result<ImportSummary, String> {
    let mode = match mode.as_str() {
        "merge" => ImportMode::Merge,
        "overwrite" => ImportMode::Overwrite,
        other => return Err(format!("unknown import mode {other:?} (use 'merge' or 'overwrite')")),
    };
    let archive = PathBuf::from(archive_path);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    repo_pack::import_source(
        store.conn(),
        &state.data_dir,
        &archive,
        mode,
        target_source_id.as_deref(),
        label_override.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// Peek at a pack file's manifest without importing it. Used by the UI
/// to show "you're about to import N chunks from <repo>" before the user
/// commits to merge/overwrite.
#[cfg(feature = "repo-rag")]
#[tauri::command]
pub async fn inspect_repo_pack(
    archive_path: String,
) -> Result<crate::memory::repo_pack::PackManifest, String> {
    let (manifest, _bytes) =
        repo_pack::read_pack(std::path::Path::new(&archive_path)).map_err(|e| e.to_string())?;
    Ok(manifest)
}
