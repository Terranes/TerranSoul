//! Brain selection snapshot — a typed, JSON-serialisable view of every
//! routing decision currently in effect across the brain stack.
//!
//! This module is the **operational answer** to the question
//! *"with so many brain components, how does the LLM know what to use?"*
//! See `docs/brain-advanced-design.md` § 20 for the full decision matrix.
//!
//! The snapshot is intentionally a pure value type: no I/O, no locks, no
//! `tauri::*` types — so it can be unit-tested in isolation and mirrored
//! verbatim by the frontend `BrainView.vue` "Active selection" panel.
//!
//! Adding a new brain component? Per architecture rule 11, add a field
//! here, populate it in `commands::brain::get_brain_selection`, and render
//! it in `src/views/BrainView.vue`.

use serde::{Deserialize, Serialize};

use crate::brain::brain_config::BrainMode;

/// A snapshot of every active brain selection at a single moment in time.
///
/// Designed to be cheap to compute (just reads `Mutex` state) and called
/// on demand from the Brain hub UI, not on every chat turn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrainSelection {
    /// Which provider mode is currently answering chats.
    pub provider: ProviderSelection,
    /// Which embedding model (if any) is available for vector RAG.
    pub embedding: EmbeddingSelection,
    /// Long-term memory state (counts per tier).
    pub memory: MemorySelection,
    /// Default search method used by the chat RAG pipeline.
    pub search: SearchSelection,
    /// Active storage backend.
    pub storage: StorageSelection,
    /// Registered agents and the dispatch default.
    pub agents: AgentSelection,
    /// End-to-end RAG quality estimate (0–100), surfaced to the user.
    pub rag_quality_percent: u8,
    /// Plain-English explanation of the RAG quality (for the UI tooltip).
    pub rag_quality_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderSelection {
    /// No brain configured — stub responses only.
    None,
    /// Free-tier cloud API (Pollinations, Groq, …).
    FreeApi {
        /// Configured provider id (what the user picked).
        configured_provider_id: String,
        /// Provider id actually used by the rotator (may differ if the
        /// configured one is rate-limited).
        effective_provider_id: String,
        /// Whether the rotator considers the effective provider healthy.
        rotator_healthy: bool,
    },
    /// User-supplied paid API.
    PaidApi {
        provider: String,
        model: String,
        base_url: String,
    },
    /// Local Ollama instance.
    LocalOllama { model: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddingSelection {
    /// True iff vector RAG is currently usable (i.e. an embedding model
    /// is reachable).
    pub available: bool,
    /// Model id we'd attempt first (e.g. `nomic-embed-text`).
    pub preferred_model: String,
    /// Plain-English reason why embeddings are off, when `available=false`.
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemorySelection {
    pub total: i64,
    pub short_count: i64,
    pub working_count: i64,
    pub long_count: i64,
    pub embedded_count: i64,
    /// Schema version actually applied to the live database.
    pub schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchSelection {
    /// Default RAG method used by the chat pipeline.
    pub default_method: SearchMethod,
    /// Top-k results injected into the system prompt.
    pub top_k: u32,
    /// Optional minimum score to inject; `None` means "always inject top-k"
    /// (the documented Phase 4 gap, see `docs/brain-advanced-design.md` §16).
    pub relevance_threshold: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchMethod {
    /// SQL `LIKE '%keyword%'` — fastest, exact match.
    Keyword,
    /// Cosine similarity only.
    Semantic,
    /// 6-signal hybrid (vector + keyword + recency + importance + decay + tier).
    Hybrid,
    /// LLM-ranked semantic search (used as fallback when embeddings missing).
    LlmRanked,
}

impl SearchMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchMethod::Keyword => "keyword",
            SearchMethod::Semantic => "semantic",
            SearchMethod::Hybrid => "hybrid",
            SearchMethod::LlmRanked => "llm_ranked",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageSelection {
    /// One of: `sqlite`, `postgres`, `mssql`, `cassandra`.
    pub backend: String,
    /// True iff the backend is the bundled offline-first default.
    pub is_local: bool,
    /// Schema label (e.g. `V5 — memory_edges`).
    pub schema_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentSelection {
    /// Registered agent ids in the orchestrator.
    pub registered: Vec<String>,
    /// What `agent_id="auto"` resolves to right now.
    pub default_agent_id: String,
}

impl BrainSelection {
    /// Build a snapshot from already-resolved component state.
    ///
    /// All inputs are owned-or-cloned to keep this helper cheap and
    /// independent of any locks; the caller is expected to gather them
    /// from `AppState` before invoking this.
    pub fn from_parts(
        brain_mode: Option<&BrainMode>,
        legacy_active_brain: Option<&str>,
        rotator_pick: Option<(&str, bool)>,
        embedding_available: bool,
        embedding_preferred_model: &str,
        memory: MemorySelection,
        storage: StorageSelection,
        agents: AgentSelection,
    ) -> Self {
        let provider = resolve_provider(brain_mode, legacy_active_brain, rotator_pick);

        let embedding = match &provider {
            ProviderSelection::LocalOllama { .. } if embedding_available => EmbeddingSelection {
                available: true,
                preferred_model: embedding_preferred_model.to_string(),
                unavailable_reason: None,
            },
            ProviderSelection::LocalOllama { .. } => EmbeddingSelection {
                available: false,
                preferred_model: embedding_preferred_model.to_string(),
                unavailable_reason: Some(
                    "Ollama did not return an embedding model (try `ollama pull nomic-embed-text`)"
                        .to_string(),
                ),
            },
            ProviderSelection::None => EmbeddingSelection {
                available: false,
                preferred_model: embedding_preferred_model.to_string(),
                unavailable_reason: Some("No brain configured".to_string()),
            },
            _ => EmbeddingSelection {
                available: false,
                preferred_model: embedding_preferred_model.to_string(),
                unavailable_reason: Some(
                    "Cloud APIs cannot compute embeddings — switch to Local Ollama for vector RAG"
                        .to_string(),
                ),
            },
        };

        let search = SearchSelection {
            default_method: if embedding.available {
                SearchMethod::Hybrid
            } else {
                SearchMethod::Keyword
            },
            top_k: 5,
            // Phase 4 gap: no threshold yet (see §16 / §20.2 row 9).
            relevance_threshold: None,
        };

        let (rag_quality_percent, rag_quality_note) = compute_rag_quality(&provider, &embedding);

        Self {
            provider,
            embedding,
            memory,
            search,
            storage,
            agents,
            rag_quality_percent,
            rag_quality_note,
        }
    }
}

fn resolve_provider(
    brain_mode: Option<&BrainMode>,
    legacy_active_brain: Option<&str>,
    rotator_pick: Option<(&str, bool)>,
) -> ProviderSelection {
    match brain_mode {
        Some(BrainMode::FreeApi { provider_id, .. }) => {
            let (effective, healthy) = match rotator_pick {
                Some((eff, h)) => (eff.to_string(), h),
                None => (provider_id.clone(), false),
            };
            ProviderSelection::FreeApi {
                configured_provider_id: provider_id.clone(),
                effective_provider_id: effective,
                rotator_healthy: healthy,
            }
        }
        Some(BrainMode::PaidApi {
            provider,
            model,
            base_url,
            ..
        }) => ProviderSelection::PaidApi {
            provider: provider.clone(),
            model: model.clone(),
            base_url: base_url.clone(),
        },
        Some(BrainMode::LocalOllama { model }) => ProviderSelection::LocalOllama {
            model: model.clone(),
        },
        None => match legacy_active_brain {
            Some(model) => ProviderSelection::LocalOllama {
                model: model.to_string(),
            },
            None => ProviderSelection::None,
        },
    }
}

fn compute_rag_quality(
    provider: &ProviderSelection,
    embedding: &EmbeddingSelection,
) -> (u8, String) {
    match provider {
        ProviderSelection::None => (
            0,
            "No brain configured — chat falls back to persona-only stub responses.".to_string(),
        ),
        ProviderSelection::LocalOllama { .. } if embedding.available => (
            100,
            "Full hybrid 6-signal RAG with local vector embeddings.".to_string(),
        ),
        ProviderSelection::LocalOllama { .. } => (
            60,
            "Local chat works, but vector search is offline — pull `nomic-embed-text` to enable it."
                .to_string(),
        ),
        ProviderSelection::FreeApi { .. } | ProviderSelection::PaidApi { .. } => (
            60,
            "Cloud APIs cannot compute embeddings — vector signal is offline (keyword + recency + importance + decay + tier still active).".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_memory() -> MemorySelection {
        MemorySelection {
            total: 10,
            short_count: 1,
            working_count: 2,
            long_count: 7,
            embedded_count: 0,
            schema_version: 5,
        }
    }
    fn dummy_storage() -> StorageSelection {
        StorageSelection {
            backend: "sqlite".to_string(),
            is_local: true,
            schema_label: "V5 — memory_edges".to_string(),
        }
    }
    fn dummy_agents() -> AgentSelection {
        AgentSelection {
            registered: vec!["stub".to_string()],
            default_agent_id: "stub".to_string(),
        }
    }

    #[test]
    fn no_brain_yields_zero_quality() {
        let sel = BrainSelection::from_parts(
            None,
            None,
            None,
            false,
            "nomic-embed-text",
            dummy_memory(),
            dummy_storage(),
            dummy_agents(),
        );
        assert_eq!(sel.provider, ProviderSelection::None);
        assert!(!sel.embedding.available);
        assert_eq!(sel.rag_quality_percent, 0);
        assert_eq!(sel.search.default_method, SearchMethod::Keyword);
    }

    #[test]
    fn legacy_active_brain_treated_as_local_ollama() {
        let sel = BrainSelection::from_parts(
            None,
            Some("gemma3:4b"),
            None,
            true,
            "nomic-embed-text",
            dummy_memory(),
            dummy_storage(),
            dummy_agents(),
        );
        match &sel.provider {
            ProviderSelection::LocalOllama { model } => assert_eq!(model, "gemma3:4b"),
            other => panic!("expected LocalOllama, got {other:?}"),
        }
        assert!(sel.embedding.available);
        assert_eq!(sel.rag_quality_percent, 100);
        assert_eq!(sel.search.default_method, SearchMethod::Hybrid);
    }

    #[test]
    fn local_ollama_without_embedding_drops_to_60() {
        let mode = BrainMode::LocalOllama {
            model: "phi-4".to_string(),
        };
        let sel = BrainSelection::from_parts(
            Some(&mode),
            None,
            None,
            false,
            "nomic-embed-text",
            dummy_memory(),
            dummy_storage(),
            dummy_agents(),
        );
        assert!(!sel.embedding.available);
        assert_eq!(sel.rag_quality_percent, 60);
        assert!(sel.embedding.unavailable_reason.is_some());
        assert_eq!(sel.search.default_method, SearchMethod::Keyword);
    }

    #[test]
    fn free_api_records_rotator_pick_and_health() {
        let mode = BrainMode::FreeApi {
            provider_id: "groq".to_string(),
            api_key: None,
        };
        // Rotator picked pollinations because groq is rate-limited but
        // pollinations is healthy.
        let sel = BrainSelection::from_parts(
            Some(&mode),
            None,
            Some(("pollinations", true)),
            false,
            "nomic-embed-text",
            dummy_memory(),
            dummy_storage(),
            dummy_agents(),
        );
        match &sel.provider {
            ProviderSelection::FreeApi {
                configured_provider_id,
                effective_provider_id,
                rotator_healthy,
            } => {
                assert_eq!(configured_provider_id, "groq");
                assert_eq!(effective_provider_id, "pollinations");
                assert!(*rotator_healthy);
            }
            other => panic!("expected FreeApi, got {other:?}"),
        }
        assert_eq!(sel.rag_quality_percent, 60); // cloud → no embeddings
    }

    #[test]
    fn free_api_with_no_rotator_pick_falls_back_to_configured() {
        let mode = BrainMode::FreeApi {
            provider_id: "groq".to_string(),
            api_key: None,
        };
        let sel = BrainSelection::from_parts(
            Some(&mode),
            None,
            None,
            false,
            "nomic-embed-text",
            dummy_memory(),
            dummy_storage(),
            dummy_agents(),
        );
        match &sel.provider {
            ProviderSelection::FreeApi {
                configured_provider_id,
                effective_provider_id,
                rotator_healthy,
            } => {
                assert_eq!(configured_provider_id, "groq");
                assert_eq!(effective_provider_id, "groq");
                assert!(!rotator_healthy);
            }
            other => panic!("expected FreeApi, got {other:?}"),
        }
    }

    #[test]
    fn paid_api_carries_endpoint_and_model() {
        let mode = BrainMode::PaidApi {
            provider: "openai".to_string(),
            api_key: "sk-…".to_string(),
            model: "gpt-4o".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        };
        let sel = BrainSelection::from_parts(
            Some(&mode),
            None,
            None,
            false,
            "nomic-embed-text",
            dummy_memory(),
            dummy_storage(),
            dummy_agents(),
        );
        match &sel.provider {
            ProviderSelection::PaidApi {
                provider,
                model,
                base_url,
            } => {
                assert_eq!(provider, "openai");
                assert_eq!(model, "gpt-4o");
                assert_eq!(base_url, "https://api.openai.com/v1");
            }
            other => panic!("expected PaidApi, got {other:?}"),
        }
        // Paid APIs never expose embeddings via our pipeline.
        assert!(!sel.embedding.available);
    }

    #[test]
    fn search_relevance_threshold_is_phase4_gap() {
        // Documented gap: we always inject top-k regardless of score.
        let sel = BrainSelection::from_parts(
            None,
            Some("gemma3:4b"),
            None,
            true,
            "nomic-embed-text",
            dummy_memory(),
            dummy_storage(),
            dummy_agents(),
        );
        assert_eq!(sel.search.top_k, 5);
        assert_eq!(sel.search.relevance_threshold, None);
    }

    #[test]
    fn snapshot_is_serialisable_round_trip() {
        let sel = BrainSelection::from_parts(
            None,
            Some("gemma3:4b"),
            None,
            true,
            "nomic-embed-text",
            dummy_memory(),
            dummy_storage(),
            dummy_agents(),
        );
        let json = serde_json::to_string(&sel).expect("serialise");
        let back: BrainSelection = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, sel);
    }
}
