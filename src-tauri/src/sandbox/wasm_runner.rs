use wasmtime::{Config, Engine, Linker, Module, OptLevel, Store};
use super::capability::CapabilityStore;
use super::host_api::HostContext;
use std::sync::{Arc, Mutex};

pub struct WasmRunner {
    engine: Engine,
}

impl WasmRunner {
    pub fn new() -> Result<Self, String> {
        let mut config = Config::new();
        config.cranelift_opt_level(OptLevel::Speed);
        let engine = Engine::new(&config).map_err(|e| e.to_string())?;
        Ok(WasmRunner { engine })
    }

    pub fn run_module(
        &self,
        wasm_bytes: &[u8],
        agent_name: &str,
        cap_store: Arc<Mutex<CapabilityStore>>,
    ) -> Result<i32, String> {
        let module =
            Module::from_binary(&self.engine, wasm_bytes).map_err(|e| e.to_string())?;
        let ctx = HostContext::new(agent_name, cap_store);
        let mut store = Store::new(&self.engine, ctx);
        let linker: Linker<HostContext> = Linker::new(&self.engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| e.to_string())?;
        let run_fn = instance
            .get_typed_func::<(), i32>(&mut store, "run")
            .map_err(|e| e.to_string())?;
        run_fn.call(&mut store, ()).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cap_store() -> Arc<Mutex<CapabilityStore>> {
        Arc::new(Mutex::new(CapabilityStore::in_memory()))
    }

    #[test]
    fn test_wasm_runner_new() {
        let runner = WasmRunner::new();
        assert!(runner.is_ok());
    }

    #[test]
    fn test_run_invalid_wasm_returns_error() {
        let runner = WasmRunner::new().unwrap();
        let result = runner.run_module(b"not wasm bytes", "test-agent", make_cap_store());
        assert!(result.is_err());
    }

    #[test]
    fn test_run_module_missing_export() {
        let runner = WasmRunner::new().unwrap();
        // A minimal valid WASM binary: just the magic header + version, no sections
        let wasm = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let result = runner.run_module(wasm, "test-agent", make_cap_store());
        assert!(result.is_err());
    }

    #[test]
    fn test_run_module_with_run_export() {
        let runner = WasmRunner::new().unwrap();
        // (module (func (export "run") (result i32) i32.const 42))
        let wasm = &[
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // magic + version
            0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // type section: () -> i32
            0x03, 0x02, 0x01, 0x00, // function section: func 0 uses type 0
            0x07, 0x07, 0x01, 0x03, 0x72, 0x75, 0x6e, 0x00, 0x00, // export "run"
            0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b, // code: i32.const 42, end
        ];
        let result = runner.run_module(wasm, "test-agent", make_cap_store());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_capability_enforcement_in_host_api() {
        use super::super::host_api::HostContext;
        use super::super::capability::Capability;
        let store = make_cap_store();
        let ctx = HostContext::new("agent", store.clone());
        assert!(ctx.check_capability(&Capability::FileRead).is_err());
        store.lock().unwrap().grant("agent", Capability::FileRead);
        assert!(ctx.check_capability(&Capability::FileRead).is_ok());
    }
}
