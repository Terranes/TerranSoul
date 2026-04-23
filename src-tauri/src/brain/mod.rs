pub mod brain_config;
pub mod brain_store;
pub mod docker_ollama;
pub mod free_api;
pub mod model_recommender;
pub mod ollama_agent;
pub mod openai_client;
pub mod provider_rotator;
pub mod ram_budget;
pub mod system_info;

pub use brain_config::BrainMode;
pub use brain_store::{clear as clear_brain, load as load_brain, save as save_brain};
pub use free_api::{free_provider_catalogue, get_free_provider, FreeProvider};
pub use model_recommender::{recommend, ModelRecommendation};
pub use ollama_agent::{check_status, list_models, pull_model, OllamaAgent, OllamaStatus};
pub use openai_client::OpenAiClient;
pub use provider_rotator::ProviderRotator;
pub use system_info::{collect as collect_system_info, SystemInfo};
