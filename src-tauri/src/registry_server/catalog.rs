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
            name: "openclaw-bridge".to_string(),
            version: "1.0.0".to_string(),
            description: "Built-in reference bridge demonstrating capability-gated \
                          tool calls (read / fetch / chat)."
                .to_string(),
            system_requirements: req.clone(),
            install_method: InstallMethod::BuiltIn,
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
        // ----- GitNexus sidecar (Chunk 2.1) -----------------------------
        // Tier 1 of the GitNexus Code-Intelligence integration. The user
        // installs `gitnexus` from npm under its own PolyForm-Noncommercial
        // licence (we never bundle the binary); TerranSoul only spawns
        // `npx gitnexus mcp` over stdio when the `code_intelligence`
        // capability is granted.
        AgentManifest {
            name: "gitnexus-sidecar".to_string(),
            version: "1.0.0".to_string(),
            description: "Out-of-process bridge to the GitNexus MCP server. \
                          Exposes read-only code-intelligence tools (query, \
                          context, impact, detect_changes) to TerranSoul's \
                          brain. Requires the GitNexus npm package to be \
                          installed separately under its own license."
                .to_string(),
            system_requirements: req,
            install_method: InstallMethod::Sidecar {
                path: "npx gitnexus mcp".to_string(),
            },
            capabilities: vec![Capability::Network, Capability::Filesystem],
            ipc_protocol_version: 1,
            homepage: Some("https://github.com/abhigyanpatwari/GitNexus".to_string()),
            license: Some("PolyForm-Noncommercial-1.0.0".to_string()),
            author: Some("Abhigyan Patwari".to_string()),
            sha256: None,
            publisher: None,
            signature: None,
        },
    ]
}
