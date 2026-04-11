use crate::package_manager::{AgentManifest, Capability, InstallMethod, SystemRequirements};

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
            description: "Built-in TerranSoul stub agent for testing".to_string(),
            system_requirements: req.clone(),
            install_method: InstallMethod::Binary {
                url: "https://terranes.dev/agents/stub/stub-agent-1.0.0".to_string(),
            },
            capabilities: vec![Capability::Chat],
            ipc_protocol_version: 1,
            homepage: Some("https://terranes.dev/agents/stub".to_string()),
            license: None,
            author: Some("TerranSoul Team".to_string()),
            sha256: None,
        },
        AgentManifest {
            name: "openclaw-bridge".to_string(),
            version: "1.0.0".to_string(),
            description: "Bridge to OpenClaw open-source AI platform".to_string(),
            system_requirements: req.clone(),
            install_method: InstallMethod::Binary {
                url: "https://openclaw.dev/releases/openclaw-bridge-1.0.0".to_string(),
            },
            capabilities: vec![
                Capability::Chat,
                Capability::Filesystem,
                Capability::Network,
            ],
            ipc_protocol_version: 1,
            homepage: Some("https://openclaw.dev".to_string()),
            license: None,
            author: Some("OpenClaw Community".to_string()),
            sha256: None,
        },
        AgentManifest {
            name: "claude-cowork".to_string(),
            version: "1.0.0".to_string(),
            description: "Claude collaborative workspace integration".to_string(),
            system_requirements: req,
            install_method: InstallMethod::Binary {
                url: "https://anthropic.com/claude/releases/cowork-1.0.0".to_string(),
            },
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
        },
    ]
}
