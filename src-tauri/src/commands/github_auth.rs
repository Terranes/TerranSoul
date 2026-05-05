//! GitHub authorization commands for the self-improve flow.

use tauri::State;

use crate::coding::{self, DeviceCodeResponse, DevicePollResult};
use crate::AppState;

#[tauri::command]
pub async fn github_request_device_code(
    scopes: Option<String>,
) -> Result<DeviceCodeResponse, String> {
    let mut cfg = coding::OAuthDeviceConfig::default();
    if let Some(scopes) = scopes.filter(|s| !s.trim().is_empty()) {
        cfg.scopes = scopes;
    }
    let client = reqwest::Client::new();
    coding::request_device_code(&client, &cfg).await
}

#[tauri::command]
pub async fn github_poll_device_token(
    device_code: String,
    state: State<'_, AppState>,
) -> Result<DevicePollResult, String> {
    let client = reqwest::Client::new();
    let cfg = coding::OAuthDeviceConfig::default();
    let result = coding::poll_for_token(&client, &cfg, &device_code).await?;

    if let DevicePollResult::Success { access_token, .. } = &result {
        let mut existing = coding::load_github_config(&state.data_dir).unwrap_or_default();
        existing.token = access_token.clone();
        existing = coding::apply_github_config_defaults(existing, &state.data_dir);
        coding::save_github_config(&state.data_dir, &existing)?;
    }

    Ok(result)
}
