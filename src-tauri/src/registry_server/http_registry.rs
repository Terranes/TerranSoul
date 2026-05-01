use crate::package_manager::{AgentManifest, RegistryError, RegistrySource};
use async_trait::async_trait;

pub struct HttpRegistry {
    base_url: String,
    client: reqwest::Client,
}

impl HttpRegistry {
    pub fn new(port: u16) -> Self {
        HttpRegistry {
            base_url: format!("http://127.0.0.1:{port}"),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl RegistrySource for HttpRegistry {
    async fn fetch_manifest(&self, agent_name: &str) -> Result<AgentManifest, RegistryError> {
        let url = format!("{}/agents/{}", self.base_url, agent_name);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::NetworkError(e.to_string()))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(RegistryError::NotFound(agent_name.to_string()));
        }
        if !resp.status().is_success() {
            return Err(RegistryError::NetworkError(format!(
                "HTTP {}",
                resp.status()
            )));
        }
        resp.json::<AgentManifest>()
            .await
            .map_err(|e| RegistryError::NetworkError(e.to_string()))
    }

    async fn download_binary(
        &self,
        agent_name: &str,
        _version: &str,
    ) -> Result<Vec<u8>, RegistryError> {
        let url = format!("{}/agents/{}/download", self.base_url, agent_name);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::NetworkError(e.to_string()))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(RegistryError::NotFound(agent_name.to_string()));
        }
        if !resp.status().is_success() {
            return Err(RegistryError::NetworkError(format!(
                "HTTP {}",
                resp.status()
            )));
        }
        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| RegistryError::NetworkError(e.to_string()))
    }

    async fn search(&self, query: &str) -> Result<Vec<AgentManifest>, RegistryError> {
        let url = format!("{}/search", self.base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("q", query)])
            .send()
            .await
            .map_err(|e| RegistryError::NetworkError(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(RegistryError::NetworkError(format!(
                "HTTP {}",
                resp.status()
            )));
        }
        resp.json::<Vec<AgentManifest>>()
            .await
            .map_err(|e| RegistryError::NetworkError(e.to_string()))
    }
}
