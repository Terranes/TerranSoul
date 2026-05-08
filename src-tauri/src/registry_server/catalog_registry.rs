//! In-process registry source backed by the static [`catalog`] manifest list.
//!
//! Unlike [`crate::registry_server::http_registry::HttpRegistry`], this source
//! does not require the local registry HTTP server to be running. It is used
//! as the **default** `package_registry` in [`crate::AppState::new`] so the
//! Agent Marketplace browse tab is populated with the official agents
//! immediately on app launch — without the user having to manually call
//! `start_registry_server` first.
//!
//! When the user (or a test) starts the HTTP registry server via the
//! `start_registry_server` Tauri command, the registry source is swapped to
//! `HttpRegistry` so that cross-device discovery works as before.

use async_trait::async_trait;
use std::collections::HashMap;

use super::catalog;
use crate::package_manager::{AgentManifest, RegistryError, RegistrySource};

/// In-process registry source backed by the static catalog.
pub struct CatalogRegistry {
    manifests: HashMap<String, AgentManifest>,
}

impl Default for CatalogRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CatalogRegistry {
    /// Build a new catalog-backed registry from [`catalog::all_entries`].
    pub fn new() -> Self {
        let mut manifests = HashMap::new();
        for entry in catalog::all_entries() {
            manifests.insert(entry.name.clone(), entry);
        }
        CatalogRegistry { manifests }
    }
}

#[async_trait]
impl RegistrySource for CatalogRegistry {
    async fn fetch_manifest(&self, agent_name: &str) -> Result<AgentManifest, RegistryError> {
        self.manifests
            .get(agent_name)
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(agent_name.to_string()))
    }

    async fn download_binary(
        &self,
        agent_name: &str,
        _version: &str,
    ) -> Result<Vec<u8>, RegistryError> {
        // Every entry in this in-process catalog is a built-in agent
        // (see `catalog::all_entries` — all use `InstallMethod::BuiltIn`).
        // Built-in agents have no binary to download — they are compiled
        // into TerranSoul. Returning an empty vector signals that the
        // installer should skip writing a binary file. The installer
        // already special-cases `InstallMethod::BuiltIn` so this path
        // is a defensive fallback for callers that ignore the manifest.
        if !self.manifests.contains_key(agent_name) {
            return Err(RegistryError::NotFound(agent_name.to_string()));
        }
        Ok(Vec::new())
    }

    async fn search(&self, query: &str) -> Result<Vec<AgentManifest>, RegistryError> {
        let q = query.to_lowercase();
        let mut results: Vec<AgentManifest> = if q.is_empty() {
            self.manifests.values().cloned().collect()
        } else {
            self.manifests
                .values()
                .filter(|m| {
                    m.name.to_lowercase().contains(&q) || m.description.to_lowercase().contains(&q)
                })
                .cloned()
                .collect()
        };
        // Stable, alphabetical ordering for deterministic UI display.
        results.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_query_returns_full_catalog() {
        let reg = CatalogRegistry::new();
        let results = reg.search("").await.unwrap();
        assert!(
            results.len() >= 2,
            "catalog should contain at least 2 agents"
        );
        let names: Vec<_> = results.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"stub-agent"));
        assert!(names.contains(&"claude-cowork"));
    }

    #[tokio::test]
    async fn query_filters_by_name() {
        let reg = CatalogRegistry::new();
        let results = reg.search("stub").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "stub-agent");
    }

    #[tokio::test]
    async fn query_filters_by_description() {
        let reg = CatalogRegistry::new();
        let results = reg.search("claude").await.unwrap();
        assert!(results.iter().any(|m| m.name == "claude-cowork"));
    }

    #[tokio::test]
    async fn fetch_manifest_returns_known_agent() {
        let reg = CatalogRegistry::new();
        let m = reg.fetch_manifest("stub-agent").await.unwrap();
        assert_eq!(m.name, "stub-agent");
    }

    #[tokio::test]
    async fn fetch_manifest_unknown_agent_is_not_found() {
        let reg = CatalogRegistry::new();
        let err = reg.fetch_manifest("does-not-exist").await.unwrap_err();
        assert!(matches!(err, RegistryError::NotFound(_)));
    }

    #[tokio::test]
    async fn download_binary_returns_empty_for_builtin_agent() {
        // Every catalog entry is a built-in agent — installer skips the
        // binary write. Defensive fallback: return Ok(empty) so any caller
        // that ignores the manifest still gets a valid (zero-length) blob
        // instead of opaque placeholder bytes.
        let reg = CatalogRegistry::new();
        let bytes = reg.download_binary("stub-agent", "1.0.0").await.unwrap();
        assert!(
            bytes.is_empty(),
            "built-in agents have no downloadable binary"
        );
    }

    #[tokio::test]
    async fn results_are_alphabetically_sorted() {
        let reg = CatalogRegistry::new();
        let results = reg.search("").await.unwrap();
        let names: Vec<_> = results.iter().map(|m| m.name.clone()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }
}
