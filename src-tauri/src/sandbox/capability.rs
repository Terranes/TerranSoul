use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    FileRead,
    FileWrite,
    Clipboard,
    Network,
    ProcessSpawn,
}

impl Capability {
    pub fn all() -> Vec<Capability> {
        vec![
            Capability::FileRead,
            Capability::FileWrite,
            Capability::Clipboard,
            Capability::Network,
            Capability::ProcessSpawn,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Capability::FileRead => "File Read",
            Capability::FileWrite => "File Write",
            Capability::Clipboard => "Clipboard",
            Capability::Network => "Network",
            Capability::ProcessSpawn => "Process Spawn",
        }
    }

    pub fn is_sensitive(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRecord {
    pub agent_name: String,
    pub capability: Capability,
    pub granted: bool,
}

pub struct CapabilityStore {
    consents: HashMap<(String, Capability), bool>,
    store_path: Option<PathBuf>,
}

impl CapabilityStore {
    pub fn new(data_dir: &Path) -> Self {
        let store_path = data_dir.join("capability_consents.json");
        let consents = Self::load(&store_path);
        CapabilityStore {
            consents,
            store_path: Some(store_path),
        }
    }

    pub fn in_memory() -> Self {
        CapabilityStore {
            consents: HashMap::new(),
            store_path: None,
        }
    }

    pub fn has_capability(&self, agent_name: &str, cap: &Capability) -> bool {
        self.consents
            .get(&(agent_name.to_string(), cap.clone()))
            .copied()
            .unwrap_or(false)
    }

    pub fn grant(&mut self, agent_name: &str, cap: Capability) {
        self.consents.insert((agent_name.to_string(), cap), true);
        self.save();
    }

    pub fn revoke(&mut self, agent_name: &str, cap: &Capability) {
        self.consents
            .insert((agent_name.to_string(), cap.clone()), false);
        self.save();
    }

    pub fn list_for_agent(&self, agent_name: &str) -> Vec<ConsentRecord> {
        self.consents
            .iter()
            .filter(|((name, _), _)| name == agent_name)
            .map(|((name, cap), &granted)| ConsentRecord {
                agent_name: name.clone(),
                capability: cap.clone(),
                granted,
            })
            .collect()
    }

    fn load(path: &Path) -> HashMap<(String, Capability), bool> {
        if !path.exists() {
            return HashMap::new();
        }
        let data = match std::fs::read_to_string(path) {
            Ok(d) => d,
            Err(_) => return HashMap::new(),
        };
        let records: Vec<ConsentRecord> = match serde_json::from_str(&data) {
            Ok(r) => r,
            Err(_) => return HashMap::new(),
        };
        records
            .into_iter()
            .map(|r| ((r.agent_name, r.capability), r.granted))
            .collect()
    }

    fn save(&self) {
        let Some(path) = &self.store_path else {
            return;
        };
        let records: Vec<ConsentRecord> = self
            .consents
            .iter()
            .map(|((name, cap), &granted)| ConsentRecord {
                agent_name: name.clone(),
                capability: cap.clone(),
                granted,
            })
            .collect();
        if let Ok(data) = serde_json::to_string(&records) {
            let _ = std::fs::write(path, data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_all_returns_five() {
        assert_eq!(Capability::all().len(), 5);
    }

    #[test]
    fn test_capability_grant_and_check() {
        let mut store = CapabilityStore::in_memory();
        assert!(!store.has_capability("my-agent", &Capability::FileRead));
        store.grant("my-agent", Capability::FileRead);
        assert!(store.has_capability("my-agent", &Capability::FileRead));
    }

    #[test]
    fn test_capability_revoke() {
        let mut store = CapabilityStore::in_memory();
        store.grant("my-agent", Capability::Network);
        assert!(store.has_capability("my-agent", &Capability::Network));
        store.revoke("my-agent", &Capability::Network);
        assert!(!store.has_capability("my-agent", &Capability::Network));
    }

    #[test]
    fn test_capability_denied_by_default() {
        let store = CapabilityStore::in_memory();
        for cap in Capability::all() {
            assert!(!store.has_capability("some-agent", &cap));
        }
    }

    #[test]
    fn test_list_for_agent() {
        let mut store = CapabilityStore::in_memory();
        store.grant("agent-a", Capability::FileRead);
        store.grant("agent-a", Capability::Network);
        store.grant("agent-b", Capability::Clipboard);
        let records = store.list_for_agent("agent-a");
        assert_eq!(records.len(), 2);
        assert!(records.iter().all(|r| r.agent_name == "agent-a"));
    }
}
