use serde::{Deserialize, Serialize};
use std::fmt;

/// The minimum IPC protocol version supported by TerranSoul.
pub const MIN_IPC_PROTOCOL_VERSION: u32 = 1;
/// The maximum IPC protocol version supported by TerranSoul.
pub const MAX_IPC_PROTOCOL_VERSION: u32 = 1;

/// An agent package manifest describing an installable agent.
///
/// Every agent distributed through TerranSoul's package manager must include
/// a manifest that declares its identity, capabilities, system requirements,
/// and installation method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentManifest {
    /// Unique package name (lowercase, alphanumeric + hyphens, 1–64 chars).
    pub name: String,
    /// Semantic version string (e.g. "1.0.0").
    pub version: String,
    /// Human-readable description of the agent.
    pub description: String,
    /// System requirements for running this agent.
    pub system_requirements: SystemRequirements,
    /// How to install/run the agent binary.
    pub install_method: InstallMethod,
    /// Capabilities the agent declares.
    pub capabilities: Vec<Capability>,
    /// IPC protocol version this agent speaks.
    pub ipc_protocol_version: u32,
    /// Optional homepage URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// Optional license identifier (SPDX).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Optional author name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// SHA-256 hash of the agent binary for verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// System requirements for running an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemRequirements {
    /// Minimum RAM in megabytes (0 = no minimum).
    #[serde(default)]
    pub min_ram_mb: u64,
    /// Supported operating systems (empty = all).
    #[serde(default)]
    pub os: Vec<OsTarget>,
    /// Supported CPU architectures (empty = all).
    #[serde(default)]
    pub arch: Vec<ArchTarget>,
    /// Whether a GPU is required.
    #[serde(default)]
    pub gpu_required: bool,
}

/// Supported operating system targets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OsTarget {
    Windows,
    Macos,
    Linux,
    Ios,
    Android,
}

/// Supported CPU architecture targets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ArchTarget {
    X86_64,
    Aarch64,
}

/// How the agent is installed and run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallMethod {
    /// A native binary downloaded from a URL.
    Binary {
        /// Download URL for the binary.
        url: String,
    },
    /// A WASM module loaded into the sandbox.
    Wasm {
        /// Download URL for the WASM module.
        url: String,
    },
    /// A sidecar process bundled with the app.
    Sidecar {
        /// Relative path to the sidecar binary within the app bundle.
        path: String,
    },
    /// An agent that is **compiled into TerranSoul itself** (e.g. the
    /// reference [`crate::agent::stub_agent::StubAgent`] or
    /// [`crate::agent::openclaw_agent::OpenClawAgent`]). Built-in agents have
    /// no binary to download — installation only writes the manifest so the
    /// orchestrator can list/enable/disable them like any other agent.
    BuiltIn,
}

/// Agent capabilities that determine what the agent can access.
///
/// Sensitive capabilities (filesystem, clipboard, network, remote_exec) require
/// explicit user consent before the agent can exercise them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Generate chat responses.
    Chat,
    /// Access the file system.
    Filesystem,
    /// Read/write the system clipboard.
    Clipboard,
    /// Make outbound network requests.
    Network,
    /// Execute shell commands on the host.
    RemoteExec,
    /// Access the character/VRM system.
    Character,
    /// Manage conversation history.
    ConversationHistory,
}

impl Capability {
    /// Returns `true` if this capability requires explicit user consent.
    pub fn requires_consent(&self) -> bool {
        matches!(
            self,
            Capability::Filesystem
                | Capability::Clipboard
                | Capability::Network
                | Capability::RemoteExec
        )
    }
}

/// Errors that can occur when validating an agent manifest.
#[derive(Debug, Clone, PartialEq)]
pub enum ManifestError {
    /// The manifest JSON could not be parsed.
    ParseError(String),
    /// The package name is invalid.
    InvalidName(String),
    /// The version string is not valid semver.
    InvalidVersion(String),
    /// The description is empty.
    EmptyDescription,
    /// The IPC protocol version is not supported.
    UnsupportedIpcVersion(u32),
    /// The SHA-256 hash is malformed.
    InvalidSha256(String),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::ParseError(e) => write!(f, "manifest: parse error: {e}"),
            ManifestError::InvalidName(n) => write!(f, "manifest: invalid name: {n}"),
            ManifestError::InvalidVersion(v) => write!(f, "manifest: invalid version: {v}"),
            ManifestError::EmptyDescription => write!(f, "manifest: description is empty"),
            ManifestError::UnsupportedIpcVersion(v) => {
                write!(
                    f,
                    "manifest: unsupported ipc_protocol_version {v} (supported: {MIN_IPC_PROTOCOL_VERSION}–{MAX_IPC_PROTOCOL_VERSION})"
                )
            }
            ManifestError::InvalidSha256(h) => write!(f, "manifest: invalid sha256: {h}"),
        }
    }
}

/// Parse a JSON string into an `AgentManifest`.
pub fn parse_manifest(json: &str) -> Result<AgentManifest, ManifestError> {
    let manifest: AgentManifest =
        serde_json::from_str(json).map_err(|e| ManifestError::ParseError(e.to_string()))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

/// Validate an already-deserialized manifest.
pub fn validate_manifest(manifest: &AgentManifest) -> Result<(), ManifestError> {
    validate_name(&manifest.name)?;
    validate_version(&manifest.version)?;
    if manifest.description.trim().is_empty() {
        return Err(ManifestError::EmptyDescription);
    }
    if manifest.ipc_protocol_version < MIN_IPC_PROTOCOL_VERSION
        || manifest.ipc_protocol_version > MAX_IPC_PROTOCOL_VERSION
    {
        return Err(ManifestError::UnsupportedIpcVersion(
            manifest.ipc_protocol_version,
        ));
    }
    if let Some(ref sha) = manifest.sha256 {
        validate_sha256(sha)?;
    }
    Ok(())
}

/// Validate the package name.
/// Must be 1–64 characters, lowercase alphanumeric and hyphens, no leading/trailing hyphens.
fn validate_name(name: &str) -> Result<(), ManifestError> {
    if name.is_empty() || name.len() > 64 {
        return Err(ManifestError::InvalidName(format!(
            "name must be 1–64 characters, got {}",
            name.len()
        )));
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Err(ManifestError::InvalidName(
            "name must not start or end with a hyphen".to_string(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(ManifestError::InvalidName(
            "name must contain only lowercase letters, digits, and hyphens".to_string(),
        ));
    }
    Ok(())
}

/// Validate a semver-like version string (MAJOR.MINOR.PATCH).
fn validate_version(version: &str) -> Result<(), ManifestError> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err(ManifestError::InvalidVersion(format!(
            "expected MAJOR.MINOR.PATCH, got \"{version}\""
        )));
    }
    for part in &parts {
        if part.is_empty() {
            return Err(ManifestError::InvalidVersion(format!(
                "empty segment in \"{version}\""
            )));
        }
        if part.parse::<u64>().is_err() {
            return Err(ManifestError::InvalidVersion(format!(
                "non-numeric segment \"{part}\" in \"{version}\""
            )));
        }
    }
    Ok(())
}

/// Validate a SHA-256 hex string (64 lowercase hex chars).
fn validate_sha256(sha: &str) -> Result<(), ManifestError> {
    if sha.len() != 64 {
        return Err(ManifestError::InvalidSha256(format!(
            "expected 64 hex chars, got {}",
            sha.len()
        )));
    }
    if !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ManifestError::InvalidSha256(
            "must contain only hex characters".to_string(),
        ));
    }
    Ok(())
}

/// Serialize a manifest to a pretty-printed JSON string.
pub fn serialize_manifest(manifest: &AgentManifest) -> Result<String, ManifestError> {
    serde_json::to_string_pretty(manifest).map_err(|e| ManifestError::ParseError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to build a valid minimal manifest for test reuse.
    fn valid_manifest() -> AgentManifest {
        AgentManifest {
            name: "stub-agent".to_string(),
            version: "1.0.0".to_string(),
            description: "A stub agent for testing".to_string(),
            system_requirements: SystemRequirements {
                min_ram_mb: 0,
                os: vec![],
                arch: vec![],
                gpu_required: false,
            },
            install_method: InstallMethod::Binary {
                url: "https://example.com/agent".to_string(),
            },
            capabilities: vec![Capability::Chat],
            ipc_protocol_version: 1,
            homepage: None,
            license: None,
            author: None,
            sha256: None,
        }
    }

    fn valid_manifest_json() -> &'static str {
        r#"{
            "name": "stub-agent",
            "version": "1.0.0",
            "description": "A stub agent for testing",
            "system_requirements": {
                "min_ram_mb": 256,
                "os": ["windows", "macos", "linux"],
                "arch": ["x86_64", "aarch64"],
                "gpu_required": false
            },
            "install_method": {
                "type": "binary",
                "url": "https://example.com/stub-agent-v1.0.0"
            },
            "capabilities": ["chat", "network"],
            "ipc_protocol_version": 1,
            "license": "MIT",
            "author": "TerranSoul Team"
        }"#
    }

    #[test]
    fn test_parse_manifest_valid_json() {
        let manifest = parse_manifest(valid_manifest_json()).unwrap();
        assert_eq!(manifest.name, "stub-agent");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.description, "A stub agent for testing");
        assert_eq!(manifest.system_requirements.min_ram_mb, 256);
        assert_eq!(manifest.system_requirements.os.len(), 3);
        assert_eq!(manifest.system_requirements.arch.len(), 2);
        assert!(!manifest.system_requirements.gpu_required);
        assert_eq!(manifest.capabilities.len(), 2);
        assert_eq!(manifest.ipc_protocol_version, 1);
        assert_eq!(manifest.license, Some("MIT".to_string()));
        assert_eq!(manifest.author, Some("TerranSoul Team".to_string()));
    }

    #[test]
    fn test_parse_manifest_roundtrip() {
        let original = valid_manifest();
        let json = serialize_manifest(&original).unwrap();
        let parsed = parse_manifest(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_parse_manifest_wasm_install_method() {
        let json = r#"{
            "name": "wasm-agent",
            "version": "0.1.0",
            "description": "A WASM agent",
            "system_requirements": {},
            "install_method": { "type": "wasm", "url": "https://example.com/agent.wasm" },
            "capabilities": ["chat"],
            "ipc_protocol_version": 1
        }"#;
        let manifest = parse_manifest(json).unwrap();
        assert!(matches!(manifest.install_method, InstallMethod::Wasm { .. }));
    }

    #[test]
    fn test_parse_manifest_sidecar_install_method() {
        let json = r#"{
            "name": "sidecar-agent",
            "version": "2.3.1",
            "description": "A sidecar agent",
            "system_requirements": {},
            "install_method": { "type": "sidecar", "path": "bin/my-agent" },
            "capabilities": ["chat", "filesystem"],
            "ipc_protocol_version": 1
        }"#;
        let manifest = parse_manifest(json).unwrap();
        assert!(matches!(
            manifest.install_method,
            InstallMethod::Sidecar { .. }
        ));
    }

    #[test]
    fn test_parse_manifest_all_capabilities() {
        let json = r#"{
            "name": "full-agent",
            "version": "1.0.0",
            "description": "Agent with all capabilities",
            "system_requirements": { "gpu_required": true },
            "install_method": { "type": "binary", "url": "https://example.com/full" },
            "capabilities": ["chat", "filesystem", "clipboard", "network", "remote_exec", "character", "conversation_history"],
            "ipc_protocol_version": 1
        }"#;
        let manifest = parse_manifest(json).unwrap();
        assert_eq!(manifest.capabilities.len(), 7);
        assert!(manifest.system_requirements.gpu_required);
    }

    #[test]
    fn test_parse_manifest_minimal_system_requirements() {
        let json = r#"{
            "name": "minimal-agent",
            "version": "1.0.0",
            "description": "Minimal system requirements",
            "system_requirements": {},
            "install_method": { "type": "binary", "url": "https://example.com/min" },
            "capabilities": ["chat"],
            "ipc_protocol_version": 1
        }"#;
        let manifest = parse_manifest(json).unwrap();
        assert_eq!(manifest.system_requirements.min_ram_mb, 0);
        assert!(manifest.system_requirements.os.is_empty());
        assert!(manifest.system_requirements.arch.is_empty());
        assert!(!manifest.system_requirements.gpu_required);
    }

    #[test]
    fn test_parse_manifest_with_optional_fields() {
        let json = r#"{
            "name": "rich-agent",
            "version": "1.0.0",
            "description": "Agent with all optional fields",
            "system_requirements": {},
            "install_method": { "type": "binary", "url": "https://example.com/rich" },
            "capabilities": ["chat"],
            "ipc_protocol_version": 1,
            "homepage": "https://example.com",
            "license": "Apache-2.0",
            "author": "Test Author",
            "sha256": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
        }"#;
        let manifest = parse_manifest(json).unwrap();
        assert_eq!(manifest.homepage, Some("https://example.com".to_string()));
        assert_eq!(manifest.license, Some("Apache-2.0".to_string()));
        assert_eq!(manifest.author, Some("Test Author".to_string()));
        assert!(manifest.sha256.is_some());
    }

    // --- Invalid manifest tests ---

    #[test]
    fn test_parse_manifest_invalid_json() {
        let result = parse_manifest("not json at all");
        assert!(matches!(result, Err(ManifestError::ParseError(_))));
    }

    #[test]
    fn test_parse_manifest_missing_required_field() {
        let json = r#"{
            "name": "agent",
            "version": "1.0.0"
        }"#;
        let result = parse_manifest(json);
        assert!(matches!(result, Err(ManifestError::ParseError(_))));
    }

    #[test]
    fn test_validate_name_empty() {
        let mut m = valid_manifest();
        m.name = String::new();
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::InvalidName(_))));
    }

    #[test]
    fn test_validate_name_too_long() {
        let mut m = valid_manifest();
        m.name = "a".repeat(65);
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::InvalidName(_))));
    }

    #[test]
    fn test_validate_name_uppercase() {
        let mut m = valid_manifest();
        m.name = "MyAgent".to_string();
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::InvalidName(_))));
    }

    #[test]
    fn test_validate_name_leading_hyphen() {
        let mut m = valid_manifest();
        m.name = "-agent".to_string();
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::InvalidName(_))));
    }

    #[test]
    fn test_validate_name_trailing_hyphen() {
        let mut m = valid_manifest();
        m.name = "agent-".to_string();
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::InvalidName(_))));
    }

    #[test]
    fn test_validate_name_special_chars() {
        let mut m = valid_manifest();
        m.name = "agent_v2".to_string();
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::InvalidName(_))));
    }

    #[test]
    fn test_validate_version_missing_patch() {
        let mut m = valid_manifest();
        m.version = "1.0".to_string();
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::InvalidVersion(_))));
    }

    #[test]
    fn test_validate_version_non_numeric() {
        let mut m = valid_manifest();
        m.version = "1.0.beta".to_string();
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::InvalidVersion(_))));
    }

    #[test]
    fn test_validate_version_empty_segment() {
        let mut m = valid_manifest();
        m.version = "1..0".to_string();
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::InvalidVersion(_))));
    }

    #[test]
    fn test_validate_empty_description() {
        let mut m = valid_manifest();
        m.description = "   ".to_string();
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::EmptyDescription)));
    }

    #[test]
    fn test_validate_ipc_version_zero() {
        let mut m = valid_manifest();
        m.ipc_protocol_version = 0;
        let result = validate_manifest(&m);
        assert!(matches!(
            result,
            Err(ManifestError::UnsupportedIpcVersion(0))
        ));
    }

    #[test]
    fn test_validate_ipc_version_too_high() {
        let mut m = valid_manifest();
        m.ipc_protocol_version = 99;
        let result = validate_manifest(&m);
        assert!(matches!(
            result,
            Err(ManifestError::UnsupportedIpcVersion(99))
        ));
    }

    #[test]
    fn test_validate_sha256_wrong_length() {
        let mut m = valid_manifest();
        m.sha256 = Some("abcdef".to_string());
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::InvalidSha256(_))));
    }

    #[test]
    fn test_validate_sha256_non_hex() {
        let mut m = valid_manifest();
        m.sha256 = Some("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_string());
        let result = validate_manifest(&m);
        assert!(matches!(result, Err(ManifestError::InvalidSha256(_))));
    }

    #[test]
    fn test_capability_requires_consent() {
        assert!(!Capability::Chat.requires_consent());
        assert!(Capability::Filesystem.requires_consent());
        assert!(Capability::Clipboard.requires_consent());
        assert!(Capability::Network.requires_consent());
        assert!(Capability::RemoteExec.requires_consent());
        assert!(!Capability::Character.requires_consent());
        assert!(!Capability::ConversationHistory.requires_consent());
    }

    #[test]
    fn test_manifest_error_display() {
        let err = ManifestError::EmptyDescription;
        assert_eq!(err.to_string(), "manifest: description is empty");
    }

    #[test]
    fn test_validate_name_valid_with_numbers_and_hyphens() {
        let mut m = valid_manifest();
        m.name = "my-agent-v2".to_string();
        assert!(validate_manifest(&m).is_ok());
    }

    #[test]
    fn test_serialize_manifest_omits_none_fields() {
        let m = valid_manifest();
        let json = serialize_manifest(&m).unwrap();
        assert!(!json.contains("homepage"));
        assert!(!json.contains("license"));
        assert!(!json.contains("author"));
        assert!(!json.contains("sha256"));
    }

    #[test]
    fn test_parse_manifest_empty_capabilities_is_valid() {
        let json = r#"{
            "name": "passive-agent",
            "version": "1.0.0",
            "description": "An agent with no capabilities",
            "system_requirements": {},
            "install_method": { "type": "binary", "url": "https://example.com/passive" },
            "capabilities": [],
            "ipc_protocol_version": 1
        }"#;
        let manifest = parse_manifest(json).unwrap();
        assert!(manifest.capabilities.is_empty());
    }
}
