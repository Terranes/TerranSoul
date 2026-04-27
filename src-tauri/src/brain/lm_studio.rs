use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const LM_STUDIO_BASE_URL: &str = "http://127.0.0.1:1234";

#[derive(Debug, Clone, Serialize)]
pub struct LmStudioStatus {
    pub running: bool,
    pub model_count: usize,
    pub loaded_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LmStudioLoadedInstance {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LmStudioQuantization {
    pub name: Option<String>,
    pub bits_per_weight: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LmStudioModelEntry {
    pub key: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(rename = "type")]
    pub model_type: String,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub architecture: Option<String>,
    #[serde(default)]
    pub quantization: Option<LmStudioQuantization>,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub params_string: Option<String>,
    #[serde(default)]
    pub loaded_instances: Vec<LmStudioLoadedInstance>,
}

#[derive(Debug, Deserialize)]
struct LmStudioModelsResponse {
    models: Vec<LmStudioModelEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LmStudioDownloadStatus {
    pub status: String,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub total_size_bytes: Option<u64>,
    #[serde(default)]
    pub downloaded_size_bytes: Option<u64>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LmStudioLoadResult {
    #[serde(rename = "type")]
    pub model_type: String,
    pub instance_id: String,
    pub status: String,
    #[serde(default)]
    pub load_time_seconds: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LmStudioUnloadResult {
    pub instance_id: String,
}

fn client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| Client::new())
}

fn normalized_base_url(base_url: Option<&str>) -> String {
    base_url
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(LM_STUDIO_BASE_URL)
        .trim_end_matches('/')
        .to_string()
}

fn with_auth(req: reqwest::RequestBuilder, api_key: Option<&str>) -> reqwest::RequestBuilder {
    match api_key.filter(|s| !s.trim().is_empty()) {
        Some(key) => req.bearer_auth(key),
        None => req,
    }
}

pub async fn list_models(
    base_url: Option<&str>,
    api_key: Option<&str>,
) -> Result<Vec<LmStudioModelEntry>, String> {
    let base = normalized_base_url(base_url);
    let url = format!("{base}/api/v1/models");
    let req = with_auth(client().get(url), api_key);
    let resp = req
        .send()
        .await
        .map_err(|e| format!("LM Studio not reachable: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("LM Studio returned HTTP {}", resp.status()));
    }
    resp.json::<LmStudioModelsResponse>()
        .await
        .map(|r| r.models)
        .map_err(|e| format!("parse LM Studio models: {e}"))
}

pub async fn check_status(base_url: Option<&str>, api_key: Option<&str>) -> LmStudioStatus {
    match list_models(base_url, api_key).await {
        Ok(models) => {
            let loaded_count = models.iter().map(|m| m.loaded_instances.len()).sum();
            LmStudioStatus {
                running: true,
                model_count: models.len(),
                loaded_count,
            }
        }
        Err(_) => LmStudioStatus {
            running: false,
            model_count: 0,
            loaded_count: 0,
        },
    }
}

pub async fn download_model(
    base_url: Option<&str>,
    api_key: Option<&str>,
    model: &str,
    quantization: Option<&str>,
) -> Result<LmStudioDownloadStatus, String> {
    let base = normalized_base_url(base_url);
    let url = format!("{base}/api/v1/models/download");
    let mut body = serde_json::json!({ "model": model });
    if let Some(q) = quantization.filter(|s| !s.trim().is_empty()) {
        body["quantization"] = serde_json::Value::String(q.to_string());
    }
    let resp = with_auth(client().post(url).json(&body), api_key)
        .send()
        .await
        .map_err(|e| format!("LM Studio not reachable: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "LM Studio download failed with HTTP {}",
            resp.status()
        ));
    }
    resp.json::<LmStudioDownloadStatus>()
        .await
        .map_err(|e| format!("parse LM Studio download status: {e}"))
}

pub async fn download_status(
    base_url: Option<&str>,
    api_key: Option<&str>,
    job_id: &str,
) -> Result<LmStudioDownloadStatus, String> {
    let base = normalized_base_url(base_url);
    let url = format!("{base}/api/v1/models/download/status/{job_id}");
    let resp = with_auth(client().get(url), api_key)
        .send()
        .await
        .map_err(|e| format!("LM Studio not reachable: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "LM Studio status failed with HTTP {}",
            resp.status()
        ));
    }
    resp.json::<LmStudioDownloadStatus>()
        .await
        .map_err(|e| format!("parse LM Studio download status: {e}"))
}

pub async fn load_model(
    base_url: Option<&str>,
    api_key: Option<&str>,
    model: &str,
    context_length: Option<u32>,
) -> Result<LmStudioLoadResult, String> {
    let base = normalized_base_url(base_url);
    let url = format!("{base}/api/v1/models/load");
    let mut body = serde_json::json!({
        "model": model,
        "echo_load_config": false,
    });
    if let Some(context_length) = context_length {
        body["context_length"] = serde_json::Value::Number(context_length.into());
    }
    let resp = with_auth(client().post(url).json(&body), api_key)
        .send()
        .await
        .map_err(|e| format!("LM Studio not reachable: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("LM Studio load failed with HTTP {}", resp.status()));
    }
    resp.json::<LmStudioLoadResult>()
        .await
        .map_err(|e| format!("parse LM Studio load result: {e}"))
}

pub async fn unload_model(
    base_url: Option<&str>,
    api_key: Option<&str>,
    instance_id: &str,
) -> Result<LmStudioUnloadResult, String> {
    let base = normalized_base_url(base_url);
    let url = format!("{base}/api/v1/models/unload");
    let resp = with_auth(
        client()
            .post(url)
            .json(&serde_json::json!({ "instance_id": instance_id })),
        api_key,
    )
    .send()
    .await
    .map_err(|e| format!("LM Studio not reachable: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "LM Studio unload failed with HTTP {}",
            resp.status()
        ));
    }
    resp.json::<LmStudioUnloadResult>()
        .await
        .map_err(|e| format!("parse LM Studio unload result: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_base_url_defaults_to_lm_studio_port() {
        assert_eq!(normalized_base_url(None), LM_STUDIO_BASE_URL);
        assert_eq!(normalized_base_url(Some("")), LM_STUDIO_BASE_URL);
    }

    #[test]
    fn normalized_base_url_strips_trailing_slash() {
        assert_eq!(
            normalized_base_url(Some("http://127.0.0.1:1234/")),
            "http://127.0.0.1:1234"
        );
    }

    #[test]
    fn models_response_deserializes_loaded_instances() {
        let json = r#"{
            "models": [{
                "key": "qwen/qwen3-4b",
                "display_name": "Qwen 3 4B",
                "type": "llm",
                "publisher": "qwen",
                "size_bytes": 2500000000,
                "loaded_instances": [{ "id": "qwen/qwen3-4b/instance" }]
            }]
        }"#;
        let parsed: LmStudioModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.models.len(), 1);
        assert_eq!(parsed.models[0].key, "qwen/qwen3-4b");
        assert_eq!(
            parsed.models[0].loaded_instances[0].id,
            "qwen/qwen3-4b/instance"
        );
    }
}
