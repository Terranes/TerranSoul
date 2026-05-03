use super::capability::CapabilityStore;
use super::host_api::HostContext;
use std::sync::{Arc, Mutex};
use wasmtime::{Config, Engine, Linker, Module, OptLevel, Store};

const DEFAULT_WASM_FUEL: u64 = 2_000_000;
const HOOK_INPUT_OFFSET: usize = 0;
const MAX_HOOK_INPUT_BYTES: usize = 32 * 1024;
const MAX_HOOK_OUTPUT_BYTES: usize = 32 * 1024;

pub struct WasmRunner {
    engine: Engine,
}

impl WasmRunner {
    pub fn new() -> Result<Self, String> {
        let mut config = Config::new();
        config.cranelift_opt_level(OptLevel::Speed);
        config.consume_fuel(true);
        let engine = Engine::new(&config).map_err(|e| e.to_string())?;
        Ok(WasmRunner { engine })
    }

    pub fn run_module(
        &self,
        wasm_bytes: &[u8],
        agent_name: &str,
        cap_store: Arc<Mutex<CapabilityStore>>,
    ) -> Result<i32, String> {
        let module = Module::from_binary(&self.engine, wasm_bytes).map_err(|e| e.to_string())?;
        let ctx = HostContext::new(agent_name, cap_store);
        let mut store = Store::new(&self.engine, ctx);
        store
            .set_fuel(DEFAULT_WASM_FUEL)
            .map_err(|e| e.to_string())?;
        let linker: Linker<HostContext> = Linker::new(&self.engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| e.to_string())?;
        let run_fn = instance
            .get_typed_func::<(), i32>(&mut store, "run")
            .map_err(|e| e.to_string())?;
        run_fn.call(&mut store, ()).map_err(|e| e.to_string())
    }

    /// Run a memory pipeline hook with the TerranSoul JSON hook ABI.
    ///
    /// ABI contract:
    /// - module exports `memory`
    /// - host writes the UTF-8 JSON payload at offset 0
    /// - module exports `memory_hook(ptr: i32, len: i32) -> i64`
    /// - return value packs `(output_ptr << 32) | output_len`
    /// - `0` means "no patch"
    pub fn run_memory_hook_json(
        &self,
        wasm_bytes: &[u8],
        agent_name: &str,
        cap_store: Arc<Mutex<CapabilityStore>>,
        input_json: &[u8],
    ) -> Result<Option<Vec<u8>>, String> {
        if input_json.len() > MAX_HOOK_INPUT_BYTES {
            return Err(format!(
                "memory hook input too large: {} bytes",
                input_json.len()
            ));
        }

        let module = Module::from_binary(&self.engine, wasm_bytes).map_err(|e| e.to_string())?;
        let ctx = HostContext::new(agent_name, cap_store);
        let mut store = Store::new(&self.engine, ctx);
        store
            .set_fuel(DEFAULT_WASM_FUEL)
            .map_err(|e| e.to_string())?;
        let linker: Linker<HostContext> = Linker::new(&self.engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| e.to_string())?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| "memory hook module must export memory".to_string())?;
        memory
            .write(&mut store, HOOK_INPUT_OFFSET, input_json)
            .map_err(|e| format!("write memory hook input: {e}"))?;
        let hook_fn = instance
            .get_typed_func::<(i32, i32), i64>(&mut store, "memory_hook")
            .map_err(|e| e.to_string())?;
        let packed = hook_fn
            .call(
                &mut store,
                (HOOK_INPUT_OFFSET as i32, input_json.len() as i32),
            )
            .map_err(|e| e.to_string())? as u64;
        if packed == 0 {
            return Ok(None);
        }

        let output_ptr = (packed >> 32) as usize;
        let output_len = (packed & 0xffff_ffff) as usize;
        if output_len > MAX_HOOK_OUTPUT_BYTES {
            return Err(format!("memory hook output too large: {output_len} bytes"));
        }
        let mut output = vec![0u8; output_len];
        memory
            .read(&store, output_ptr, &mut output)
            .map_err(|e| format!("read memory hook output: {e}"))?;
        Ok(Some(output))
    }

    /// Run a plugin command through the TerranSoul command JSON ABI.
    ///
    /// ABI contract:
    /// - module exports `memory`
    /// - host writes `{ "command_id": string, "args": any }` at offset 0
    /// - module exports `handle_command(ptr: i32, len: i32) -> i64`
    /// - return value packs `(output_ptr << 32) | output_len`
    /// - `0` means success with no textual output
    pub fn run_command_json(
        &self,
        wasm_bytes: &[u8],
        agent_name: &str,
        cap_store: Arc<Mutex<CapabilityStore>>,
        input_json: &[u8],
    ) -> Result<Option<Vec<u8>>, String> {
        if input_json.len() > MAX_HOOK_INPUT_BYTES {
            return Err(format!(
                "command input too large: {} bytes",
                input_json.len()
            ));
        }

        let module = Module::from_binary(&self.engine, wasm_bytes).map_err(|e| e.to_string())?;
        let ctx = HostContext::new(agent_name, cap_store);
        let mut store = Store::new(&self.engine, ctx);
        store
            .set_fuel(DEFAULT_WASM_FUEL)
            .map_err(|e| e.to_string())?;
        let linker: Linker<HostContext> = Linker::new(&self.engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| e.to_string())?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| "command module must export memory".to_string())?;
        memory
            .write(&mut store, HOOK_INPUT_OFFSET, input_json)
            .map_err(|e| format!("write command input: {e}"))?;
        let command_fn = instance
            .get_typed_func::<(i32, i32), i64>(&mut store, "handle_command")
            .map_err(|e| e.to_string())?;
        let packed = command_fn
            .call(
                &mut store,
                (HOOK_INPUT_OFFSET as i32, input_json.len() as i32),
            )
            .map_err(|e| e.to_string())? as u64;
        if packed == 0 {
            return Ok(None);
        }

        let output_ptr = (packed >> 32) as usize;
        let output_len = (packed & 0xffff_ffff) as usize;
        if output_len > MAX_HOOK_OUTPUT_BYTES {
            return Err(format!("command output too large: {output_len} bytes"));
        }
        let mut output = vec![0u8; output_len];
        memory
            .read(&store, output_ptr, &mut output)
            .map_err(|e| format!("read command output: {e}"))?;
        Ok(Some(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cap_store() -> Arc<Mutex<CapabilityStore>> {
        Arc::new(Mutex::new(CapabilityStore::in_memory()))
    }

    fn memory_hook_wasm(output_json: &str) -> Vec<u8> {
        json_output_wasm("memory_hook", output_json)
    }

    fn command_wasm(output: &str) -> Vec<u8> {
        json_output_wasm("handle_command", output)
    }

    fn json_output_wasm(export_name: &str, output_json: &str) -> Vec<u8> {
        use wasm_encoder::{
            CodeSection, ConstExpr, DataSection, ExportKind, ExportSection, Function,
            FunctionSection, Instruction, MemorySection, MemoryType, Module, TypeSection, ValType,
        };

        const OUTPUT_OFFSET: u64 = 1024;
        let packed = ((OUTPUT_OFFSET << 32) | output_json.len() as u64) as i64;
        let mut module = Module::new();
        let mut types = TypeSection::new();
        types
            .ty()
            .function([ValType::I32, ValType::I32], [ValType::I64]);
        module.section(&types);

        let mut functions = FunctionSection::new();
        functions.function(0);
        module.section(&functions);

        let mut memories = MemorySection::new();
        memories.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&memories);

        let mut exports = ExportSection::new();
        exports.export("memory", ExportKind::Memory, 0);
        exports.export(export_name, ExportKind::Func, 0);
        module.section(&exports);

        let mut code = CodeSection::new();
        let mut function = Function::new([]);
        function.instruction(&Instruction::I64Const(packed));
        function.instruction(&Instruction::End);
        code.function(&function);
        module.section(&code);

        let mut data = DataSection::new();
        data.active(
            0,
            &ConstExpr::i32_const(OUTPUT_OFFSET as i32),
            output_json.as_bytes().iter().copied(),
        );
        module.section(&data);
        module.finish()
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
    fn test_run_memory_hook_json_returns_patch() {
        let runner = WasmRunner::new().unwrap();
        let wasm = memory_hook_wasm(r#"{"tags":"auto:test","content":"rewritten"}"#);
        let result = runner
            .run_memory_hook_json(
                &wasm,
                "hook-agent",
                make_cap_store(),
                br#"{"stage":"pre_store","content":"old","tags":"test"}"#,
            )
            .unwrap()
            .unwrap();
        let output = String::from_utf8(result).unwrap();
        assert!(output.contains("auto:test"));
        assert!(output.contains("rewritten"));
    }

    #[test]
    fn test_run_command_json_returns_output() {
        let runner = WasmRunner::new().unwrap();
        let wasm = command_wasm("command-ok");
        let result = runner
            .run_command_json(
                &wasm,
                "command-agent",
                make_cap_store(),
                br#"{"command_id":"demo.run","args":{"text":"hi"}}"#,
            )
            .unwrap()
            .unwrap();
        assert_eq!(String::from_utf8(result).unwrap(), "command-ok");
    }

    #[test]
    fn test_capability_enforcement_in_host_api() {
        use super::super::capability::Capability;
        use super::super::host_api::HostContext;
        let store = make_cap_store();
        let ctx = HostContext::new("agent", store.clone());
        assert!(ctx.check_capability(&Capability::FileRead).is_err());
        store.lock().unwrap().grant("agent", Capability::FileRead);
        assert!(ctx.check_capability(&Capability::FileRead).is_ok());
    }
}
