//! BRAIN-REPO-RAG-1c-b-ii-a — Tauri command for cross-source `All` mode
//! hybrid search across the main TerranSoul brain and every indexed
//! repo memory source, RRF-merged (k = 60).
//!
//! The heavy lifting lives in
//! [`crate::ai_integrations::gateway::AppStateGateway::cross_source_search`];
//! this is a thin Tauri wrapper that builds the adapter and forwards the
//! call with `READ_WRITE` caps (frontend is trusted).

use tauri::State;

use crate::ai_integrations::gateway::{
    AppStateGateway, BrainGateway, CrossSourceSearchRequest, GatewayCaps, MultiSourceHit,
};
use crate::AppState;

#[tauri::command]
pub async fn cross_source_search(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<MultiSourceHit>, String> {
    let gw = AppStateGateway::new(state.inner().clone());
    gw.cross_source_search(
        &GatewayCaps::READ_WRITE,
        CrossSourceSearchRequest { query, limit },
    )
    .await
    .map_err(|e| e.to_string())
}
