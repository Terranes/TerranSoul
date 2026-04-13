use serde::{Deserialize, Serialize};

/// Describes a free LLM API provider sourced from awesome-free-llm-apis.
///
/// All providers expose an OpenAI-compatible `/v1/chat/completions` endpoint
/// (or equivalent) and require no API key or a free-tier key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FreeProvider {
    /// Unique identifier (e.g. "groq", "cerebras").
    pub id: String,
    /// Human-readable name shown in the UI.
    pub display_name: String,
    /// Base URL for the OpenAI-compatible API (e.g. "https://api.groq.com/openai").
    pub base_url: String,
    /// Default model to use with this provider.
    pub model: String,
    /// Requests-per-minute limit (0 = unknown/unlimited).
    pub rpm_limit: u32,
    /// Requests-per-day limit (0 = unknown/unlimited).
    pub rpd_limit: u32,
    /// Whether this provider requires an API key (even if free-tier).
    pub requires_api_key: bool,
    /// Short description / notes.
    pub notes: String,
}

/// Return the curated catalogue of free LLM API providers.
///
/// Sorted roughly by reliability and speed (best first).
/// Source: <https://github.com/mnfst/awesome-free-llm-apis>
pub fn free_provider_catalogue() -> Vec<FreeProvider> {
    vec![
        FreeProvider {
            id: "groq".into(),
            display_name: "Groq".into(),
            base_url: "https://api.groq.com/openai".into(),
            model: "llama-3.3-70b-versatile".into(),
            rpm_limit: 30,
            rpd_limit: 1000,
            requires_api_key: true,
            notes: "Fast inference, free tier with API key".into(),
        },
        FreeProvider {
            id: "cerebras".into(),
            display_name: "Cerebras".into(),
            base_url: "https://api.cerebras.ai".into(),
            model: "llama-3.3-70b".into(),
            rpm_limit: 30,
            rpd_limit: 14400,
            requires_api_key: true,
            notes: "Generous free limits, fast inference".into(),
        },
        FreeProvider {
            id: "siliconflow".into(),
            display_name: "SiliconFlow".into(),
            base_url: "https://api.siliconflow.cn".into(),
            model: "Qwen/Qwen3-8B".into(),
            rpm_limit: 1000,
            rpd_limit: 0,
            requires_api_key: true,
            notes: "Very generous RPM, free tier".into(),
        },
        FreeProvider {
            id: "mistral".into(),
            display_name: "Mistral AI".into(),
            base_url: "https://api.mistral.ai".into(),
            model: "mistral-small-latest".into(),
            rpm_limit: 60,
            rpd_limit: 0,
            requires_api_key: true,
            notes: "EU-based, 1B tokens/month free".into(),
        },
        FreeProvider {
            id: "github-models".into(),
            display_name: "GitHub Models".into(),
            base_url: "https://models.inference.ai.azure.com".into(),
            model: "gpt-4o".into(),
            rpm_limit: 10,
            rpd_limit: 50,
            requires_api_key: true,
            notes: "Uses GitHub PAT as API key".into(),
        },
        FreeProvider {
            id: "openrouter".into(),
            display_name: "OpenRouter".into(),
            base_url: "https://openrouter.ai/api".into(),
            model: "meta-llama/llama-3.3-70b-instruct:free".into(),
            rpm_limit: 20,
            rpd_limit: 50,
            requires_api_key: true,
            notes: "Multi-model gateway, free tier available".into(),
        },
        FreeProvider {
            id: "nvidia-nim".into(),
            display_name: "NVIDIA NIM".into(),
            base_url: "https://integrate.api.nvidia.com".into(),
            model: "meta/llama-3.3-70b-instruct".into(),
            rpm_limit: 40,
            rpd_limit: 0,
            requires_api_key: true,
            notes: "NVIDIA hosted, free tier".into(),
        },
        FreeProvider {
            id: "gemini".into(),
            display_name: "Google Gemini".into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".into(),
            model: "gemini-2.0-flash".into(),
            rpm_limit: 15,
            rpd_limit: 1000,
            requires_api_key: true,
            notes: "Not available in EU/UK/CH".into(),
        },
    ]
}

/// Look up a free provider by its ID.
pub fn get_free_provider(id: &str) -> Option<FreeProvider> {
    free_provider_catalogue().into_iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_not_empty() {
        let providers = free_provider_catalogue();
        assert!(!providers.is_empty());
    }

    #[test]
    fn provider_ids_are_unique() {
        let providers = free_provider_catalogue();
        let mut ids: Vec<&str> = providers.iter().map(|p| p.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), providers.len(), "duplicate provider IDs");
    }

    #[test]
    fn all_providers_have_base_url() {
        for p in free_provider_catalogue() {
            assert!(!p.base_url.is_empty(), "{} has empty base_url", p.id);
            assert!(
                p.base_url.starts_with("https://"),
                "{} base_url must be HTTPS",
                p.id
            );
        }
    }

    #[test]
    fn all_providers_have_model() {
        for p in free_provider_catalogue() {
            assert!(!p.model.is_empty(), "{} has empty model", p.id);
        }
    }

    #[test]
    fn get_free_provider_found() {
        let p = get_free_provider("groq");
        assert!(p.is_some());
        assert_eq!(p.unwrap().display_name, "Groq");
    }

    #[test]
    fn get_free_provider_not_found() {
        assert!(get_free_provider("nonexistent").is_none());
    }

    #[test]
    fn groq_has_correct_limits() {
        let p = get_free_provider("groq").unwrap();
        assert_eq!(p.rpm_limit, 30);
        assert_eq!(p.rpd_limit, 1000);
    }

    #[test]
    fn serde_roundtrip() {
        let p = get_free_provider("cerebras").unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let parsed: FreeProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, p);
    }
}
