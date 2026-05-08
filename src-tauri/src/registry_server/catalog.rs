use crate::package_manager::{AgentManifest, Capability, InstallMethod, SystemRequirements};

/// In-process catalog of **built-in** agents that ship with TerranSoul.
///
/// Every entry here is a real `AgentProvider` impl compiled into the binary
/// (see `crate::agent::*`). Their [`InstallMethod`] is therefore
/// [`InstallMethod::BuiltIn`] — installing them only writes a manifest
/// record, not a downloadable binary. New community agents that ship as
/// downloadable binaries belong on the HTTP registry server, not in this
/// in-process catalog.
pub fn all_entries() -> Vec<AgentManifest> {
    let req = SystemRequirements {
        min_ram_mb: 0,
        os: vec![],
        arch: vec![],
        gpu_required: false,
    };
    vec![
        AgentManifest {
            name: "stub-agent".to_string(),
            version: "1.0.0".to_string(),
            description: "Built-in TerranSoul fallback agent. Returns canned \
                          responses when no LLM brain is configured."
                .to_string(),
            system_requirements: req.clone(),
            install_method: InstallMethod::BuiltIn,
            capabilities: vec![Capability::Chat],
            ipc_protocol_version: 1,
            homepage: Some("https://terranes.dev/agents/stub".to_string()),
            license: None,
            author: Some("TerranSoul Team".to_string()),
            sha256: None,
            publisher: None,
            signature: None,
        },
        AgentManifest {
            name: "claude-cowork".to_string(),
            version: "1.0.0".to_string(),
            description: "Built-in Claude collaborative workspace integration. \
                          Real Anthropic API calls are made via the Paid-API brain mode."
                .to_string(),
            system_requirements: req.clone(),
            install_method: InstallMethod::BuiltIn,
            capabilities: vec![
                Capability::Chat,
                Capability::Filesystem,
                Capability::Network,
            ],
            ipc_protocol_version: 1,
            homepage: Some("https://anthropic.com/claude".to_string()),
            license: None,
            author: Some("Anthropic".to_string()),
            sha256: None,
            publisher: None,
            signature: None,
        },
    ]
}
