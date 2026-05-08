use super::capability::CapabilityStore;
use std::sync::{Arc, Mutex};

pub struct WasmRunner;

impl WasmRunner {
    pub fn new() -> Result<Self, String> {
        Err(Self::disabled_message())
    }

    pub fn run_module(
        &self,
        _wasm_bytes: &[u8],
        _agent_name: &str,
        _cap_store: Arc<Mutex<CapabilityStore>>,
    ) -> Result<i32, String> {
        Err(Self::disabled_message())
    }

    pub fn run_memory_hook_json(
        &self,
        _wasm_bytes: &[u8],
        _agent_name: &str,
        _cap_store: Arc<Mutex<CapabilityStore>>,
        _input_json: &[u8],
    ) -> Result<Option<Vec<u8>>, String> {
        Err(Self::disabled_message())
    }

    pub fn run_command_json(
        &self,
        _wasm_bytes: &[u8],
        _agent_name: &str,
        _cap_store: Arc<Mutex<CapabilityStore>>,
        _input_json: &[u8],
    ) -> Result<Option<Vec<u8>>, String> {
        Err(Self::disabled_message())
    }

    fn disabled_message() -> String {
        "WASM sandbox support is disabled in this build; rebuild with --features wasm-sandbox"
            .to_string()
    }
}
