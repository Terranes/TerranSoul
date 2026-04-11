use std::sync::{Arc, Mutex};
use super::capability::{Capability, CapabilityStore};

pub struct HostContext {
    pub agent_name: String,
    pub capability_store: Arc<Mutex<CapabilityStore>>,
}

impl HostContext {
    pub fn new(agent_name: &str, cap_store: Arc<Mutex<CapabilityStore>>) -> Self {
        HostContext {
            agent_name: agent_name.to_string(),
            capability_store: cap_store,
        }
    }

    pub fn check_capability(&self, cap: &Capability) -> Result<(), String> {
        if self
            .capability_store
            .lock()
            .unwrap()
            .has_capability(&self.agent_name, cap)
        {
            Ok(())
        } else {
            Err(format!(
                "capability {:?} not granted for agent {}",
                cap, self.agent_name
            ))
        }
    }

    pub fn read_file(&self, path: &str) -> Result<String, String> {
        self.check_capability(&Capability::FileRead)?;
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    }

    pub fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
        self.check_capability(&Capability::FileWrite)?;
        std::fs::write(path, content).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> Arc<Mutex<CapabilityStore>> {
        Arc::new(Mutex::new(CapabilityStore::in_memory()))
    }

    #[test]
    fn test_read_file_denied_without_capability() {
        let store = make_store();
        let ctx = HostContext::new("test-agent", store);
        let result = ctx.read_file("/some/file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not granted"));
    }

    #[test]
    fn test_write_file_denied_without_capability() {
        let store = make_store();
        let ctx = HostContext::new("test-agent", store);
        let result = ctx.write_file("/some/file.txt", "content");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not granted"));
    }
}
