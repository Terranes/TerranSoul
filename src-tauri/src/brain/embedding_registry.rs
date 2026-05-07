//! Embedding model registry (Chunk 44.5).
//!
//! A user-facing catalogue of embedding models that can be used for
//! vectorizing memories. Supports switching models with automatic
//! re-embedding migration.
//!
//! Key responsibilities:
//! 1. Maintain a catalogue of known embedding models (local + cloud)
//! 2. Track the active model per brain mode
//! 3. Detect model change → flag memories for re-embedding
//! 4. Provide migration progress tracking

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Where an embedding model runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbedProvider {
    /// Local Ollama `/api/embed`.
    Ollama,
    /// Cloud `/v1/embeddings` (free-tier provider).
    CloudFree,
    /// Cloud `/v1/embeddings` (paid API).
    CloudPaid,
    /// Local LM Studio `/v1/embeddings`.
    LmStudio,
}

/// A known embedding model in the catalogue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelEntry {
    /// Unique model identifier (e.g. "nomic-embed-text", "text-embedding-3-small").
    pub id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Output embedding dimension.
    pub dimensions: usize,
    /// Where this model runs.
    pub provider: EmbedProvider,
    /// Maximum input token count (0 = unknown).
    pub max_tokens: usize,
    /// Brief description.
    pub description: String,
}

/// Persisted state of the active embedding model.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbeddingRegistryState {
    /// Currently active embedding model ID.
    pub active_model_id: Option<String>,
    /// Previous model ID (set when switching, cleared when migration completes).
    pub previous_model_id: Option<String>,
    /// Whether a re-embedding migration is in progress.
    pub migration_pending: bool,
    /// Number of memories that still need re-embedding.
    pub migration_remaining: usize,
    /// Total memories that needed re-embedding when migration started.
    pub migration_total: usize,
}

/// Summary of a model switch action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSwitchResult {
    pub old_model: Option<String>,
    pub new_model: String,
    pub memories_to_reembed: usize,
}

// ---------------------------------------------------------------------------
// Catalogue
// ---------------------------------------------------------------------------

/// Built-in catalogue of known embedding models.
pub fn catalogue() -> Vec<EmbeddingModelEntry> {
    vec![
        // Local Ollama models
        EmbeddingModelEntry {
            id: "nomic-embed-text".to_string(),
            display_name: "Nomic Embed Text".to_string(),
            dimensions: 768,
            provider: EmbedProvider::Ollama,
            max_tokens: 8192,
            description: "Fast 768d general-purpose. Recommended for local use.".to_string(),
        },
        EmbeddingModelEntry {
            id: "mxbai-embed-large".to_string(),
            display_name: "mxbai Embed Large".to_string(),
            dimensions: 1024,
            provider: EmbedProvider::Ollama,
            max_tokens: 512,
            description: "Strong 1024d, sentence-level. Good for short texts.".to_string(),
        },
        EmbeddingModelEntry {
            id: "snowflake-arctic-embed".to_string(),
            display_name: "Snowflake Arctic Embed".to_string(),
            dimensions: 1024,
            provider: EmbedProvider::Ollama,
            max_tokens: 512,
            description: "Snowflake 1024d retrieval model.".to_string(),
        },
        EmbeddingModelEntry {
            id: "bge-m3".to_string(),
            display_name: "BGE-M3".to_string(),
            dimensions: 1024,
            provider: EmbedProvider::Ollama,
            max_tokens: 8192,
            description: "Multilingual 1024d, good for mixed-language memories.".to_string(),
        },
        EmbeddingModelEntry {
            id: "all-minilm".to_string(),
            display_name: "all-MiniLM".to_string(),
            dimensions: 384,
            provider: EmbedProvider::Ollama,
            max_tokens: 512,
            description: "Tiny 384d last-resort. Fast but lower quality.".to_string(),
        },
        // Cloud models (paid)
        EmbeddingModelEntry {
            id: "text-embedding-3-small".to_string(),
            display_name: "OpenAI text-embedding-3-small".to_string(),
            dimensions: 1536,
            provider: EmbedProvider::CloudPaid,
            max_tokens: 8191,
            description: "OpenAI's efficient 1536d model. Fast and affordable.".to_string(),
        },
        EmbeddingModelEntry {
            id: "text-embedding-3-large".to_string(),
            display_name: "OpenAI text-embedding-3-large".to_string(),
            dimensions: 3072,
            provider: EmbedProvider::CloudPaid,
            max_tokens: 8191,
            description: "OpenAI's highest quality 3072d model.".to_string(),
        },
        EmbeddingModelEntry {
            id: "voyage-3-lite".to_string(),
            display_name: "Voyage 3 Lite".to_string(),
            dimensions: 512,
            provider: EmbedProvider::CloudPaid,
            max_tokens: 32000,
            description: "Anthropic/Voyage's efficient 512d model with 32K context.".to_string(),
        },
        EmbeddingModelEntry {
            id: "mistral-embed".to_string(),
            display_name: "Mistral Embed".to_string(),
            dimensions: 1024,
            provider: EmbedProvider::CloudPaid,
            max_tokens: 8192,
            description: "Mistral's 1024d embedding model.".to_string(),
        },
        // Cloud models (free-tier)
        EmbeddingModelEntry {
            id: "BAAI/bge-m3".to_string(),
            display_name: "BGE-M3 (SiliconFlow)".to_string(),
            dimensions: 1024,
            provider: EmbedProvider::CloudFree,
            max_tokens: 8192,
            description: "Free multilingual 1024d via SiliconFlow.".to_string(),
        },
        EmbeddingModelEntry {
            id: "nvidia/nv-embedqa-e5-v5".to_string(),
            display_name: "NVIDIA NV-EmbedQA E5 v5".to_string(),
            dimensions: 1024,
            provider: EmbedProvider::CloudFree,
            max_tokens: 512,
            description: "Free 1024d via NVIDIA NIM.".to_string(),
        },
    ]
}

/// Look up a model by ID in the catalogue.
pub fn find_model(id: &str) -> Option<EmbeddingModelEntry> {
    catalogue().into_iter().find(|m| m.id == id)
}

// ---------------------------------------------------------------------------
// State persistence
// ---------------------------------------------------------------------------

const REGISTRY_FILE: &str = "embedding_registry.json";

/// Load the registry state from disk.
pub fn load_state(data_dir: &Path) -> EmbeddingRegistryState {
    let path = data_dir.join(REGISTRY_FILE);
    if path.exists() {
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(state) = serde_json::from_str::<EmbeddingRegistryState>(&contents) {
                return state;
            }
        }
    }
    EmbeddingRegistryState::default()
}

/// Save the registry state to disk.
pub fn save_state(data_dir: &Path, state: &EmbeddingRegistryState) -> Result<(), String> {
    let path = data_dir.join(REGISTRY_FILE);
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Model switching logic
// ---------------------------------------------------------------------------

/// Plan a model switch. Returns the count of memories that need re-embedding.
///
/// Does NOT perform the actual re-embedding — that's an async operation
/// done by the Tauri command layer with progress events.
pub fn plan_model_switch(
    data_dir: &Path,
    new_model_id: &str,
    embedded_count: usize,
) -> ModelSwitchResult {
    let current = load_state(data_dir);
    let old_model = current.active_model_id.clone();

    // If same model, no migration needed.
    let memories_to_reembed = if old_model.as_deref() == Some(new_model_id) {
        0
    } else {
        embedded_count
    };

    ModelSwitchResult {
        old_model,
        new_model: new_model_id.to_string(),
        memories_to_reembed,
    }
}

/// Commit a model switch: update the registry state, mark migration pending.
pub fn commit_model_switch(
    data_dir: &Path,
    new_model_id: &str,
    memories_to_reembed: usize,
) -> Result<EmbeddingRegistryState, String> {
    let current = load_state(data_dir);
    let new_state = EmbeddingRegistryState {
        active_model_id: Some(new_model_id.to_string()),
        previous_model_id: current.active_model_id,
        migration_pending: memories_to_reembed > 0,
        migration_remaining: memories_to_reembed,
        migration_total: memories_to_reembed,
    };
    save_state(data_dir, &new_state)?;
    Ok(new_state)
}

/// Update migration progress (called after each batch of re-embeddings).
pub fn update_migration_progress(
    data_dir: &Path,
    newly_embedded: usize,
) -> Result<EmbeddingRegistryState, String> {
    let mut state = load_state(data_dir);
    state.migration_remaining = state.migration_remaining.saturating_sub(newly_embedded);
    if state.migration_remaining == 0 {
        state.migration_pending = false;
        state.previous_model_id = None;
    }
    save_state(data_dir, &state)?;
    Ok(state)
}

/// Mark migration as complete (clears pending state).
pub fn complete_migration(data_dir: &Path) -> Result<EmbeddingRegistryState, String> {
    let mut state = load_state(data_dir);
    state.migration_pending = false;
    state.migration_remaining = 0;
    state.previous_model_id = None;
    save_state(data_dir, &state)?;
    Ok(state)
}

/// Get the data directory for embedding registry state.
pub fn registry_path(data_dir: &Path) -> PathBuf {
    data_dir.join(REGISTRY_FILE)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_is_non_empty() {
        let cat = catalogue();
        assert!(!cat.is_empty());
        assert!(cat.len() >= 10);
    }

    #[test]
    fn find_model_known() {
        let m = find_model("nomic-embed-text").unwrap();
        assert_eq!(m.dimensions, 768);
        assert_eq!(m.provider, EmbedProvider::Ollama);
    }

    #[test]
    fn find_model_unknown_returns_none() {
        assert!(find_model("nonexistent-model").is_none());
    }

    #[test]
    fn catalogue_has_unique_ids() {
        let cat = catalogue();
        let mut ids: Vec<&str> = cat.iter().map(|m| m.id.as_str()).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "Duplicate model IDs found");
    }

    #[test]
    fn state_round_trip_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let state = EmbeddingRegistryState {
            active_model_id: Some("nomic-embed-text".to_string()),
            previous_model_id: Some("all-minilm".to_string()),
            migration_pending: true,
            migration_remaining: 42,
            migration_total: 100,
        };
        save_state(dir.path(), &state).unwrap();
        let loaded = load_state(dir.path());
        assert_eq!(loaded.active_model_id, state.active_model_id);
        assert_eq!(loaded.previous_model_id, state.previous_model_id);
        assert!(loaded.migration_pending);
        assert_eq!(loaded.migration_remaining, 42);
        assert_eq!(loaded.migration_total, 100);
    }

    #[test]
    fn load_state_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let state = load_state(dir.path());
        assert!(state.active_model_id.is_none());
        assert!(!state.migration_pending);
    }

    #[test]
    fn plan_model_switch_same_model_no_migration() {
        let dir = tempfile::tempdir().unwrap();
        let initial = EmbeddingRegistryState {
            active_model_id: Some("nomic-embed-text".to_string()),
            ..Default::default()
        };
        save_state(dir.path(), &initial).unwrap();
        let result = plan_model_switch(dir.path(), "nomic-embed-text", 500);
        assert_eq!(result.memories_to_reembed, 0);
    }

    #[test]
    fn plan_model_switch_different_model_needs_migration() {
        let dir = tempfile::tempdir().unwrap();
        let initial = EmbeddingRegistryState {
            active_model_id: Some("nomic-embed-text".to_string()),
            ..Default::default()
        };
        save_state(dir.path(), &initial).unwrap();
        let result = plan_model_switch(dir.path(), "mxbai-embed-large", 500);
        assert_eq!(result.memories_to_reembed, 500);
        assert_eq!(result.old_model, Some("nomic-embed-text".to_string()));
        assert_eq!(result.new_model, "mxbai-embed-large");
    }

    #[test]
    fn commit_model_switch_persists_state() {
        let dir = tempfile::tempdir().unwrap();
        let initial = EmbeddingRegistryState {
            active_model_id: Some("all-minilm".to_string()),
            ..Default::default()
        };
        save_state(dir.path(), &initial).unwrap();

        let state = commit_model_switch(dir.path(), "nomic-embed-text", 200).unwrap();
        assert_eq!(state.active_model_id, Some("nomic-embed-text".to_string()));
        assert_eq!(state.previous_model_id, Some("all-minilm".to_string()));
        assert!(state.migration_pending);
        assert_eq!(state.migration_remaining, 200);
        assert_eq!(state.migration_total, 200);
    }

    #[test]
    fn update_migration_progress_decrements() {
        let dir = tempfile::tempdir().unwrap();
        let state = EmbeddingRegistryState {
            active_model_id: Some("nomic-embed-text".to_string()),
            previous_model_id: Some("all-minilm".to_string()),
            migration_pending: true,
            migration_remaining: 100,
            migration_total: 100,
        };
        save_state(dir.path(), &state).unwrap();

        let updated = update_migration_progress(dir.path(), 30).unwrap();
        assert_eq!(updated.migration_remaining, 70);
        assert!(updated.migration_pending);

        let final_state = update_migration_progress(dir.path(), 70).unwrap();
        assert_eq!(final_state.migration_remaining, 0);
        assert!(!final_state.migration_pending);
        assert!(final_state.previous_model_id.is_none());
    }

    #[test]
    fn complete_migration_clears_state() {
        let dir = tempfile::tempdir().unwrap();
        let state = EmbeddingRegistryState {
            active_model_id: Some("nomic-embed-text".to_string()),
            previous_model_id: Some("all-minilm".to_string()),
            migration_pending: true,
            migration_remaining: 50,
            migration_total: 100,
        };
        save_state(dir.path(), &state).unwrap();

        let done = complete_migration(dir.path()).unwrap();
        assert!(!done.migration_pending);
        assert_eq!(done.migration_remaining, 0);
        assert!(done.previous_model_id.is_none());
        assert_eq!(done.active_model_id, Some("nomic-embed-text".to_string()));
    }

    #[test]
    fn plan_switch_no_previous_model() {
        let dir = tempfile::tempdir().unwrap();
        // Fresh install, no active model yet.
        let result = plan_model_switch(dir.path(), "nomic-embed-text", 0);
        assert_eq!(result.memories_to_reembed, 0);
        assert!(result.old_model.is_none());
    }

    #[test]
    fn catalogue_dimensions_positive() {
        for model in catalogue() {
            assert!(model.dimensions > 0, "{} has 0 dimensions", model.id);
        }
    }
}
