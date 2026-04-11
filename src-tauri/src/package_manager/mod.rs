pub mod manifest;

pub use manifest::{
    parse_manifest, serialize_manifest, validate_manifest, AgentManifest, ArchTarget, Capability,
    InstallMethod, ManifestError, OsTarget, SystemRequirements, MAX_IPC_PROTOCOL_VERSION,
    MIN_IPC_PROTOCOL_VERSION,
};
