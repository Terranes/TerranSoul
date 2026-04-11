use tauri::State;

use crate::memory::{MemoryEntry, MemoryUpdate, NewMemory};
use crate::AppState;

/// Add a new long-term memory.
#[tauri::command]
pub async fn add_memory(
    content: String,
    tags: String,
    importance: i64,
    memory_type: String,
    state: State<'_, AppState>,
) -> Result<MemoryEntry, String> {
    let mt = serde_json::from_value(serde_json::Value::String(memory_type))
        .unwrap_or(crate::memory::MemoryType::Fact);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .add(NewMemory {
            content,
            tags,
            importance,
            memory_type: mt,
        })
        .map_err(|e| e.to_string())
}

/// Return all stored memories.
#[tauri::command]
pub async fn get_memories(state: State<'_, AppState>) -> Result<Vec<MemoryEntry>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.get_all().map_err(|e| e.to_string())
}

/// Search memories by keyword.
#[tauri::command]
pub async fn search_memories(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEntry>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.search(&query).map_err(|e| e.to_string())
}

/// Update fields on an existing memory.
#[tauri::command]
pub async fn update_memory(
    id: i64,
    content: Option<String>,
    tags: Option<String>,
    importance: Option<i64>,
    memory_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<MemoryEntry, String> {
    let mt = memory_type.and_then(|s| {
        serde_json::from_value(serde_json::Value::String(s)).ok()
    });
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .update(
            id,
            MemoryUpdate {
                content,
                tags,
                importance,
                memory_type: mt,
            },
        )
        .map_err(|e| e.to_string())
}

/// Delete a memory by id.
#[tauri::command]
pub async fn delete_memory(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.delete(id).map_err(|e| e.to_string())
}

/// Return the N most relevant long-term memories for a given message.
/// Used internally; also exposed for debugging.
#[tauri::command]
pub async fn get_relevant_memories(
    message: String,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    Ok(store.relevant_for(&message, limit))
}

/// Return the current short-term memory window (last N conversation messages).
#[tauri::command]
pub async fn get_short_term_memory(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::commands::chat::Message>, String> {
    let window = limit.unwrap_or(20);
    let conv = state
        .conversation
        .lock()
        .map_err(|e| e.to_string())?;
    let messages: Vec<_> = conv.iter().rev().take(window).rev().cloned().collect();
    Ok(messages)
}

// ── Brain-powered memory commands ────────────────────────────────────────────

/// Use the active brain to extract memorable facts from the current session
/// and store them automatically.  Returns the number of new memories saved.
#[tauri::command]
pub async fn extract_memories_from_session(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let model = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;

    let history: Vec<(String, String)> = {
        let conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    }; // lock released before await

    let facts = crate::memory::brain_memory::extract_facts(&model, &history).await;

    let count = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        crate::memory::brain_memory::save_facts(&facts, &store)
    };
    Ok(count)
}

/// Use the active brain to summarize the current session into a single memory entry.
#[tauri::command]
pub async fn summarize_session(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let model = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;

    let history: Vec<(String, String)> = {
        let conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    }; // lock released before await

    let summary = crate::memory::brain_memory::summarize(&model, &history)
        .await
        .ok_or_else(|| "Session is empty or brain is unreachable.".to_string())?;

    {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        crate::memory::brain_memory::save_summary(&summary, &store);
    }
    Ok(summary)
}

/// Use the active brain to perform a semantic search across stored memories.
/// Falls back to keyword search if the brain is unavailable.
#[tauri::command]
pub async fn semantic_search_memories(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::memory::MemoryEntry>, String> {
    let limit = limit.unwrap_or(10);
    let model_opt = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let entries: Vec<crate::memory::MemoryEntry> = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store.get_all().map_err(|e| e.to_string())?
    }; // lock released before any await

    let results = if let Some(model) = model_opt {
        crate::memory::brain_memory::semantic_search_entries(&model, &query, &entries, limit).await
    } else {
        entries
            .into_iter()
            .filter(|e| {
                let q = query.to_lowercase();
                e.content.to_lowercase().contains(&q) || e.tags.to_lowercase().contains(&q)
            })
            .take(limit)
            .collect()
    };

    Ok(results)
}
