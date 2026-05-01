use crate::sandbox::{Capability, CapabilityStore, WasmRunner};
use crate::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct ConsentInfo {
    pub agent_name: String,
    pub capability: String,
    pub granted: bool,
}

fn parse_capability(s: &str) -> Result<Capability, String> {
    match s {
        "file_read" => Ok(Capability::FileRead),
        "file_write" => Ok(Capability::FileWrite),
        "clipboard" => Ok(Capability::Clipboard),
        "network" => Ok(Capability::Network),
        "process_spawn" => Ok(Capability::ProcessSpawn),
        "code_intelligence" => Ok(Capability::CodeIntelligence),
        _ => Err(format!("unknown capability: {s}")),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn grant_agent_capability(
    agent_name: String,
    capability: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cap = parse_capability(&capability)?;
    let mut store = state.capability_store.lock().await;
    store.grant(&agent_name, cap);
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn revoke_agent_capability(
    agent_name: String,
    capability: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cap = parse_capability(&capability)?;
    let mut store = state.capability_store.lock().await;
    store.revoke(&agent_name, &cap);
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn list_agent_capabilities(
    agent_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<ConsentInfo>, String> {
    let store = state.capability_store.lock().await;
    let records = store.list_for_agent(&agent_name);
    Ok(records
        .into_iter()
        .map(|r| {
            let cap_str = serde_json::to_value(&r.capability)
                .unwrap_or_default()
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            ConsentInfo {
                agent_name: r.agent_name,
                capability: cap_str,
                granted: r.granted,
            }
        })
        .collect())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn run_agent_in_sandbox(
    agent_name: String,
    wasm_bytes: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<i32, String> {
    let cap_store = {
        use std::sync::{Arc, Mutex};
        let guard = state.capability_store.lock().await;
        let records = guard.list_for_agent(&agent_name);
        let mut snapshot = CapabilityStore::in_memory();
        for record in records {
            if record.granted {
                snapshot.grant(&record.agent_name, record.capability);
            }
        }
        Arc::new(Mutex::new(snapshot))
    };

    let runner = WasmRunner::new()?;
    runner.run_module(&wasm_bytes, &agent_name, cap_store)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn clear_agent_capabilities(
    agent_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut store = state.capability_store.lock().await;
    for cap in crate::sandbox::Capability::all() {
        store.revoke(&agent_name, &cap);
    }
    Ok(())
}
