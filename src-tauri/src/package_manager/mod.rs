pub mod installer;
pub mod manifest;
pub mod registry;
pub mod signing;

pub use installer::{InstalledAgent, InstallerError, PackageInstaller};
pub use manifest::{
    parse_manifest, serialize_manifest, validate_manifest, AgentManifest, ArchTarget, Capability,
    InstallMethod, ManifestError, OsTarget, SystemRequirements, MAX_IPC_PROTOCOL_VERSION,
    MIN_IPC_PROTOCOL_VERSION,
};
pub use registry::{MockRegistry, RegistryError, RegistrySource};
pub use signing::{
    canonical_signing_payload, publisher_key, verify_manifest_signature, PublisherEntry,
    SigningError, PUBLISHER_ALLOW_LIST,
};
