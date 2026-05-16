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
