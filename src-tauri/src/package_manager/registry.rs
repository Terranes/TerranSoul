use async_trait::async_trait;

use super::manifest::{parse_manifest, AgentManifest, ManifestError};

/// A registry source that can fetch manifests and download agent binaries.
#[async_trait]
pub trait RegistrySource: Send + Sync {
    /// Fetch the manifest for a given agent name. Returns the manifest or an error.
    async fn fetch_manifest(&self, agent_name: &str) -> Result<AgentManifest, RegistryError>;

    /// Download the agent binary and return the raw bytes.
    async fn download_binary(
        &self,
        agent_name: &str,
        version: &str,
    ) -> Result<Vec<u8>, RegistryError>;

    /// Search for agents matching a query string.
    async fn search(&self, query: &str) -> Result<Vec<AgentManifest>, RegistryError>;
}

/// Errors from registry operations.
#[derive(Debug, Clone, PartialEq)]
pub enum RegistryError {
    /// The agent was not found in the registry.
    NotFound(String),
    /// A network or I/O error occurred.
    NetworkError(String),
    /// The manifest was invalid.
    InvalidManifest(ManifestError),
    /// Generic registry error.
    Other(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::NotFound(name) => write!(f, "registry: agent \"{name}\" not found"),
            RegistryError::NetworkError(e) => write!(f, "registry: network error: {e}"),
            RegistryError::InvalidManifest(e) => write!(f, "registry: invalid manifest: {e}"),
            RegistryError::Other(e) => write!(f, "registry: {e}"),
        }
    }
}

impl From<ManifestError> for RegistryError {
    fn from(e: ManifestError) -> Self {
        RegistryError::InvalidManifest(e)
    }
}

/// An in-memory mock registry for testing. Stores manifests and fake binaries keyed by agent name.
pub struct MockRegistry {
    manifests: std::collections::HashMap<String, String>,
    binaries: std::collections::HashMap<String, Vec<u8>>,
}

impl Default for MockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MockRegistry {
    /// Create a new empty mock registry.
    pub fn new() -> Self {
        MockRegistry {
            manifests: std::collections::HashMap::new(),
            binaries: std::collections::HashMap::new(),
        }
    }

    /// Register an agent with its manifest JSON and binary data.
    pub fn add_agent(&mut self, name: &str, manifest_json: &str, binary: Vec<u8>) {
        self.manifests
            .insert(name.to_string(), manifest_json.to_string());
        self.binaries.insert(name.to_string(), binary);
    }
}

#[async_trait]
impl RegistrySource for MockRegistry {
    async fn fetch_manifest(&self, agent_name: &str) -> Result<AgentManifest, RegistryError> {
        let json = self
            .manifests
            .get(agent_name)
            .ok_or_else(|| RegistryError::NotFound(agent_name.to_string()))?;
        let manifest = parse_manifest(json)?;
        Ok(manifest)
    }

    async fn download_binary(
        &self,
        agent_name: &str,
        _version: &str,
    ) -> Result<Vec<u8>, RegistryError> {
        self.binaries
            .get(agent_name)
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(agent_name.to_string()))
    }

    async fn search(&self, query: &str) -> Result<Vec<AgentManifest>, RegistryError> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        for json in self.manifests.values() {
            if let Ok(m) = parse_manifest(json) {
                if m.name.contains(&query_lower)
                    || m.description.to_lowercase().contains(&query_lower)
                {
                    results.push(m);
                }
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest_json(name: &str) -> String {
        format!(
            r#"{{
                "name": "{name}",
                "version": "1.0.0",
                "description": "Test agent {name}",
                "system_requirements": {{}},
                "install_method": {{ "type": "binary", "url": "https://example.com/{name}" }},
                "capabilities": ["chat"],
                "ipc_protocol_version": 1
            }}"#
        )
    }

    #[tokio::test]
    async fn test_mock_registry_fetch_manifest_success() {
        let mut registry = MockRegistry::new();
        registry.add_agent(
            "test-agent",
            &sample_manifest_json("test-agent"),
            vec![1, 2, 3],
        );
        let manifest = registry.fetch_manifest("test-agent").await.unwrap();
        assert_eq!(manifest.name, "test-agent");
    }

    #[tokio::test]
    async fn test_mock_registry_fetch_manifest_not_found() {
        let registry = MockRegistry::new();
        let result = registry.fetch_manifest("nonexistent").await;
        assert!(matches!(result, Err(RegistryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_mock_registry_download_binary() {
        let mut registry = MockRegistry::new();
        registry.add_agent(
            "test-agent",
            &sample_manifest_json("test-agent"),
            vec![0xDE, 0xAD],
        );
        let binary = registry
            .download_binary("test-agent", "1.0.0")
            .await
            .unwrap();
        assert_eq!(binary, vec![0xDE, 0xAD]);
    }

    #[tokio::test]
    async fn test_mock_registry_download_binary_not_found() {
        let registry = MockRegistry::new();
        let result = registry.download_binary("nonexistent", "1.0.0").await;
        assert!(matches!(result, Err(RegistryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_mock_registry_search_by_name() {
        let mut registry = MockRegistry::new();
        registry.add_agent("alpha-bot", &sample_manifest_json("alpha-bot"), vec![]);
        registry.add_agent("beta-helper", &sample_manifest_json("beta-helper"), vec![]);
        let results = registry.search("alpha").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "alpha-bot");
    }

    #[tokio::test]
    async fn test_mock_registry_search_by_description() {
        let mut registry = MockRegistry::new();
        registry.add_agent("my-agent", &sample_manifest_json("my-agent"), vec![]);
        let results = registry.search("Test agent").await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_registry_search_no_results() {
        let mut registry = MockRegistry::new();
        registry.add_agent("my-agent", &sample_manifest_json("my-agent"), vec![]);
        let results = registry.search("zzzzz").await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_registry_error_display() {
        let err = RegistryError::NotFound("test-agent".to_string());
        assert_eq!(err.to_string(), "registry: agent \"test-agent\" not found");
    }
}
