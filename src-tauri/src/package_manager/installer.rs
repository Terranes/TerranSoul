use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::manifest::{parse_manifest, serialize_manifest, AgentManifest};
use super::registry::{RegistryError, RegistrySource};
use super::signing::{verify_manifest_signature, SigningError};

/// Directory name within the app data dir where agents are installed.
const AGENTS_DIR: &str = "agents";
/// Filename for the installed manifest within each agent's directory.
const MANIFEST_FILE: &str = "manifest.json";
/// Filename for the agent binary within each agent's directory.
const BINARY_FILE: &str = "agent.bin";

/// Summary of an installed agent package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstalledAgent {
    pub name: String,
    pub version: String,
    pub description: String,
    pub install_path: String,
}

/// Errors from installer operations.
#[derive(Debug, Clone, PartialEq)]
pub enum InstallerError {
    /// The agent is already installed.
    AlreadyInstalled(String),
    /// The agent is not installed.
    NotInstalled(String),
    /// SHA-256 hash verification failed.
    HashMismatch {
        expected: String,
        actual: String,
    },
    /// A `Binary` / `Wasm` install method was missing the mandatory `sha256` field.
    /// Built-in agents are exempt because they have no downloadable binary.
    MissingSha256(String),
    /// The manifest's Ed25519 signature could not be verified.
    SignatureVerificationFailed(SigningError),
    /// Registry error during fetch or download.
    Registry(RegistryError),
    /// Filesystem I/O error.
    IoError(String),
}

impl std::fmt::Display for InstallerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallerError::AlreadyInstalled(name) => {
                write!(f, "installer: agent \"{name}\" is already installed")
            }
            InstallerError::NotInstalled(name) => {
                write!(f, "installer: agent \"{name}\" is not installed")
            }
            InstallerError::HashMismatch { expected, actual } => {
                write!(
                    f,
                    "installer: sha256 mismatch — expected {expected}, got {actual}"
                )
            }
            InstallerError::MissingSha256(name) => {
                write!(
                    f,
                    "installer: agent \"{name}\" has a downloadable install method but no sha256 — \
                     refusing to install untrusted binary"
                )
            }
            InstallerError::SignatureVerificationFailed(e) => {
                write!(f, "installer: {e}")
            }
            InstallerError::Registry(e) => write!(f, "installer: {e}"),
            InstallerError::IoError(e) => write!(f, "installer: I/O error: {e}"),
        }
    }
}

impl From<RegistryError> for InstallerError {
    fn from(e: RegistryError) -> Self {
        InstallerError::Registry(e)
    }
}

impl From<SigningError> for InstallerError {
    fn from(e: SigningError) -> Self {
        InstallerError::SignatureVerificationFailed(e)
    }
}

/// Compute the SHA-256 hex digest of raw bytes.
fn sha256_hex(data: &[u8]) -> String {
    use std::fmt::Write;
    // Minimal SHA-256 using Rust stdlib-style approach.
    // We use a simple implementation for portability.
    let hash = sha256_digest(data);
    let mut hex = String::with_capacity(64);
    for byte in &hash {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// SHA-256 digest implementation (pure Rust, no external crate needed for this small use).
fn sha256_digest(data: &[u8]) -> [u8; 32] {
    // Constants: first 32 bits of the fractional parts of the cube roots of the first 64 primes.
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    // Initial hash values: first 32 bits of the fractional parts of the square roots of the first 8 primes.
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    // Pre-processing: adding padding bits and length.
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 512-bit (64-byte) chunk.
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut digest = [0u8; 32];
    for (i, val) in h.iter().enumerate() {
        digest[4 * i..4 * i + 4].copy_from_slice(&val.to_be_bytes());
    }
    digest
}

/// Manages installation, update, and removal of agent packages on the local filesystem.
pub struct PackageInstaller {
    agents_dir: PathBuf,
    installed: HashMap<String, AgentManifest>,
}

impl PackageInstaller {
    /// Create a new installer rooted at the given app data directory.
    pub fn new(data_dir: &Path) -> Self {
        let agents_dir = data_dir.join(AGENTS_DIR);
        let installed = load_installed_manifests(&agents_dir);
        PackageInstaller {
            agents_dir,
            installed,
        }
    }

    /// Create a new installer for testing with an explicit agents directory.
    #[cfg(test)]
    pub fn new_in(agents_dir: PathBuf) -> Self {
        let installed = load_installed_manifests(&agents_dir);
        PackageInstaller {
            agents_dir,
            installed,
        }
    }

    /// List all installed agents.
    pub fn list_installed(&self) -> Vec<InstalledAgent> {
        self.installed
            .values()
            .map(|m| InstalledAgent {
                name: m.name.clone(),
                version: m.version.clone(),
                description: m.description.clone(),
                install_path: self
                    .agents_dir
                    .join(&m.name)
                    .to_string_lossy()
                    .to_string(),
            })
            .collect()
    }

    /// Check if an agent is installed.
    pub fn is_installed(&self, name: &str) -> bool {
        self.installed.contains_key(name)
    }

    /// Get the manifest for an installed agent.
    pub fn get_installed(&self, name: &str) -> Option<&AgentManifest> {
        self.installed.get(name)
    }

    /// Install an agent from a registry source.
    ///
    /// 1. Fetches the manifest from the registry.
    /// 2. Verifies the manifest's Ed25519 signature when a `publisher` is set.
    /// 3. For non-built-in install methods, requires a mandatory `sha256` field.
    /// 4. Downloads the binary.
    /// 5. Verifies the SHA-256 hash.
    /// 6. Writes the manifest and binary to `agents/<name>/`.
    pub async fn install(
        &mut self,
        agent_name: &str,
        registry: &dyn RegistrySource,
    ) -> Result<InstalledAgent, InstallerError> {
        if self.installed.contains_key(agent_name) {
            return Err(InstallerError::AlreadyInstalled(agent_name.to_string()));
        }

        let manifest = registry.fetch_manifest(agent_name).await?;
        verify_manifest_trust(&manifest)?;

        let is_builtin =
            matches!(manifest.install_method, crate::package_manager::InstallMethod::BuiltIn);

        // Built-in agents are compiled into TerranSoul — skip the download
        // step entirely. Other install methods fetch the binary normally.
        let binary = if is_builtin {
            Vec::new()
        } else {
            registry
                .download_binary(agent_name, &manifest.version)
                .await?
        };

        // Verify SHA-256 hash. `verify_manifest_trust` has already enforced
        // that downloadable agents declare it, so the `expect` is unreachable.
        if !is_builtin {
            let expected_hash = manifest
                .sha256
                .as_ref()
                .expect("verify_manifest_trust enforces sha256 on downloadable agents");
            let actual_hash = sha256_hex(&binary);
            if actual_hash != *expected_hash {
                return Err(InstallerError::HashMismatch {
                    expected: expected_hash.clone(),
                    actual: actual_hash,
                });
            }
        }

        // Write to disk. Built-ins write only the manifest; downloadable
        // agents also write their binary.
        let agent_dir = self.agents_dir.join(agent_name);
        write_agent_files(&agent_dir, &manifest, &binary, is_builtin)?;

        let info = InstalledAgent {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            install_path: agent_dir.to_string_lossy().to_string(),
        };
        self.installed.insert(agent_name.to_string(), manifest);
        Ok(info)
    }

    /// Update an installed agent to the latest version from the registry.
    ///
    /// Fetches the latest manifest, downloads and verifies the new binary,
    /// then replaces the installed files. If the installed version matches
    /// the registry version, no action is taken.
    pub async fn update(
        &mut self,
        agent_name: &str,
        registry: &dyn RegistrySource,
    ) -> Result<InstalledAgent, InstallerError> {
        if !self.installed.contains_key(agent_name) {
            return Err(InstallerError::NotInstalled(agent_name.to_string()));
        }

        let manifest = registry.fetch_manifest(agent_name).await?;
        verify_manifest_trust(&manifest)?;

        let is_builtin =
            matches!(manifest.install_method, crate::package_manager::InstallMethod::BuiltIn);

        // Check if already at latest version.
        if let Some(current) = self.installed.get(agent_name) {
            if current.version == manifest.version {
                return Ok(InstalledAgent {
                    name: current.name.clone(),
                    version: current.version.clone(),
                    description: current.description.clone(),
                    install_path: self
                        .agents_dir
                        .join(agent_name)
                        .to_string_lossy()
                        .to_string(),
                });
            }
        }

        // Built-ins skip the download step (they are compiled into TerranSoul).
        let binary = if is_builtin {
            Vec::new()
        } else {
            registry
                .download_binary(agent_name, &manifest.version)
                .await?
        };

        // Verify SHA-256 hash. `verify_manifest_trust` has already enforced
        // that downloadable agents declare it.
        if !is_builtin {
            let expected_hash = manifest
                .sha256
                .as_ref()
                .expect("verify_manifest_trust enforces sha256 on downloadable agents");
            let actual_hash = sha256_hex(&binary);
            if actual_hash != *expected_hash {
                return Err(InstallerError::HashMismatch {
                    expected: expected_hash.clone(),
                    actual: actual_hash,
                });
            }
        }

        let agent_dir = self.agents_dir.join(agent_name);
        write_agent_files(&agent_dir, &manifest, &binary, is_builtin)?;

        let info = InstalledAgent {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            install_path: agent_dir.to_string_lossy().to_string(),
        };
        self.installed.insert(agent_name.to_string(), manifest);
        Ok(info)
    }

    /// Remove an installed agent, deleting its directory from disk.
    pub fn remove(&mut self, agent_name: &str) -> Result<(), InstallerError> {
        if !self.installed.contains_key(agent_name) {
            return Err(InstallerError::NotInstalled(agent_name.to_string()));
        }

        let agent_dir = self.agents_dir.join(agent_name);
        if agent_dir.exists() {
            std::fs::remove_dir_all(&agent_dir).map_err(|e| InstallerError::IoError(e.to_string()))?;
        }

        self.installed.remove(agent_name);
        Ok(())
    }
}

/// Combined "is this manifest safe to install" check used by both `install`
/// and `update`. Two policies, in order:
///
/// 1. **Mandatory `sha256`** — every non-built-in install method must
///    declare a SHA-256 hash of its binary. Built-in agents (compiled
///    into TerranSoul) are exempt because they have no binary to hash.
///    `Sidecar` install methods are also exempt: their binaries ship
///    inside the Tauri app bundle and are validated by the Tauri
///    resource scope, not by the registry.
/// 2. **Ed25519 signature verification** — when the manifest declares a
///    `publisher`, the detached signature is verified against the
///    publisher's compiled-in public key (via
///    [`super::signing::verify_manifest_signature`]).
fn verify_manifest_trust(manifest: &AgentManifest) -> Result<(), InstallerError> {
    use crate::package_manager::InstallMethod;
    let needs_hash = matches!(
        manifest.install_method,
        InstallMethod::Binary { .. } | InstallMethod::Wasm { .. }
    );
    if needs_hash && manifest.sha256.is_none() {
        return Err(InstallerError::MissingSha256(manifest.name.clone()));
    }
    verify_manifest_signature(manifest)?;
    Ok(())
}

/// Write agent manifest and (optionally) binary files to disk.
///
/// `is_builtin` agents have no binary to write — only the manifest is
/// persisted so the installed-agents listing can show them. Downloadable
/// agents always write `agent.bin` alongside `manifest.json`.
fn write_agent_files(
    agent_dir: &Path,
    manifest: &AgentManifest,
    binary: &[u8],
    is_builtin: bool,
) -> Result<(), InstallerError> {
    std::fs::create_dir_all(agent_dir).map_err(|e| InstallerError::IoError(e.to_string()))?;

    let manifest_json =
        serialize_manifest(manifest).map_err(|e| InstallerError::IoError(e.to_string()))?;
    std::fs::write(agent_dir.join(MANIFEST_FILE), manifest_json)
        .map_err(|e| InstallerError::IoError(e.to_string()))?;

    if !is_builtin {
        std::fs::write(agent_dir.join(BINARY_FILE), binary)
            .map_err(|e| InstallerError::IoError(e.to_string()))?;
    }

    Ok(())
}

/// Load all installed manifests from the agents directory on startup.
fn load_installed_manifests(agents_dir: &Path) -> HashMap<String, AgentManifest> {
    let mut installed = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(agents_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join(MANIFEST_FILE);
                if manifest_path.exists() {
                    if let Ok(contents) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = parse_manifest(&contents) {
                            installed.insert(manifest.name.clone(), manifest);
                        }
                    }
                }
            }
        }
    }
    installed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package_manager::registry::MockRegistry;
    use tempfile::TempDir;

    /// Build a manifest JSON for a `Binary` install method whose `sha256`
    /// matches the supplied binary bytes. Mandatory-sha256 enforcement
    /// (Chunk 1.7) means downloadable agents must always declare a hash.
    fn sample_manifest_json(name: &str, version: &str, binary: &[u8]) -> String {
        let sha = sha256_hex(binary);
        format!(
            r#"{{
                "name": "{name}",
                "version": "{version}",
                "description": "Test agent {name}",
                "system_requirements": {{}},
                "install_method": {{ "type": "binary", "url": "https://example.com/{name}" }},
                "capabilities": ["chat"],
                "ipc_protocol_version": 1,
                "sha256": "{sha}"
            }}"#
        )
    }

    fn sample_manifest_json_with_sha(name: &str, version: &str, sha: &str) -> String {
        format!(
            r#"{{
                "name": "{name}",
                "version": "{version}",
                "description": "Test agent {name}",
                "system_requirements": {{}},
                "install_method": {{ "type": "binary", "url": "https://example.com/{name}" }},
                "capabilities": ["chat"],
                "ipc_protocol_version": 1,
                "sha256": "{sha}"
            }}"#
        )
    }

    /// Build a manifest JSON for a `Binary` install method **without** a
    /// `sha256`. Used by the test that asserts the installer rejects
    /// downloadable agents that omit the mandatory hash.
    fn sample_manifest_json_without_sha(name: &str, version: &str) -> String {
        format!(
            r#"{{
                "name": "{name}",
                "version": "{version}",
                "description": "Test agent {name}",
                "system_requirements": {{}},
                "install_method": {{ "type": "binary", "url": "https://example.com/{name}" }},
                "capabilities": ["chat"],
                "ipc_protocol_version": 1
            }}"#
        )
    }

    #[test]
    fn test_sha256_hex_known_value() {
        // SHA-256 of empty input.
        let hash = sha256_hex(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_hex_hello() {
        let hash = sha256_hex(b"hello");
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[tokio::test]
    async fn test_install_agent_success() {
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let mut registry = MockRegistry::new();
        registry.add_agent(
            "test-agent",
            &sample_manifest_json("test-agent", "1.0.0", &[1, 2, 3]),
            vec![1, 2, 3],
        );

        let result = installer.install("test-agent", &registry).await.unwrap();
        assert_eq!(result.name, "test-agent");
        assert_eq!(result.version, "1.0.0");
        assert!(installer.is_installed("test-agent"));
    }

    #[tokio::test]
    async fn test_install_agent_already_installed() {
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let mut registry = MockRegistry::new();
        registry.add_agent(
            "test-agent",
            &sample_manifest_json("test-agent", "1.0.0", &[1, 2, 3]),
            vec![1, 2, 3],
        );

        installer.install("test-agent", &registry).await.unwrap();
        let result = installer.install("test-agent", &registry).await;
        assert!(matches!(result, Err(InstallerError::AlreadyInstalled(_))));
    }

    #[tokio::test]
    async fn test_install_agent_not_found() {
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let registry = MockRegistry::new();

        let result = installer.install("nonexistent", &registry).await;
        assert!(matches!(
            result,
            Err(InstallerError::Registry(RegistryError::NotFound(_)))
        ));
    }

    #[tokio::test]
    async fn test_install_builtin_agent_writes_manifest_only() {
        // Built-in agents are compiled into TerranSoul. The installer must
        // record their manifest (so they appear in the installed-agents list)
        // but must NOT write a binary file — there is no binary to write.
        // This guards against the previous "PLACEHOLDER_BINARY" mock.
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let mut registry = MockRegistry::new();
        let manifest_json = r#"{
            "name": "builtin-agent",
            "version": "1.0.0",
            "description": "Compiled-in agent",
            "system_requirements": {},
            "install_method": { "type": "built_in" },
            "capabilities": ["chat"],
            "ipc_protocol_version": 1
        }"#;
        // The mock supplies bytes but the installer should ignore them for
        // built-in install methods.
        registry.add_agent("builtin-agent", manifest_json, b"ignored".to_vec());

        let result = installer
            .install("builtin-agent", &registry)
            .await
            .expect("built-in install should succeed");
        assert_eq!(result.name, "builtin-agent");
        assert!(installer.is_installed("builtin-agent"));

        let agent_dir = tmp.path().join(AGENTS_DIR).join("builtin-agent");
        assert!(agent_dir.join(MANIFEST_FILE).exists(), "manifest must be written");
        assert!(
            !agent_dir.join(BINARY_FILE).exists(),
            "no binary file should be written for built-in agents"
        );
    }

    #[tokio::test]
    async fn test_install_agent_hash_verification_success() {
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let binary = b"hello".to_vec();
        let sha = sha256_hex(&binary);
        let mut registry = MockRegistry::new();
        registry.add_agent(
            "hash-agent",
            &sample_manifest_json_with_sha("hash-agent", "1.0.0", &sha),
            binary,
        );

        let result = installer.install("hash-agent", &registry).await.unwrap();
        assert_eq!(result.name, "hash-agent");
    }

    #[tokio::test]
    async fn test_install_agent_hash_verification_failure() {
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let mut registry = MockRegistry::new();
        registry.add_agent(
            "hash-agent",
            &sample_manifest_json_with_sha(
                "hash-agent",
                "1.0.0",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            vec![1, 2, 3],
        );

        let result = installer.install("hash-agent", &registry).await;
        assert!(matches!(result, Err(InstallerError::HashMismatch { .. })));
    }

    #[tokio::test]
    async fn test_install_agent_missing_sha256_is_rejected() {
        // Chunk 1.7: downloadable agents (Binary / Wasm install methods)
        // must declare a `sha256`. Omitting it is rejected before any
        // bytes are written to disk.
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let mut registry = MockRegistry::new();
        registry.add_agent(
            "no-hash-agent",
            &sample_manifest_json_without_sha("no-hash-agent", "1.0.0"),
            vec![0xAA, 0xBB],
        );

        let err = installer.install("no-hash-agent", &registry).await.unwrap_err();
        assert!(matches!(err, InstallerError::MissingSha256(ref n) if n == "no-hash-agent"));
        // Nothing should have been persisted.
        assert!(!installer.is_installed("no-hash-agent"));
        let agent_dir = tmp.path().join(AGENTS_DIR).join("no-hash-agent");
        assert!(!agent_dir.exists());
    }

    #[tokio::test]
    async fn test_install_agent_with_publisher_unknown_is_rejected() {
        // Chunk 1.7: a `publisher` that is not on the curated allow-list
        // is rejected even when the manifest is otherwise valid (sha256
        // present + correct).
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let binary = b"signed-payload".to_vec();
        let sha = sha256_hex(&binary);
        let manifest_json = format!(
            r#"{{
                "name": "signed-agent",
                "version": "1.0.0",
                "description": "Signed agent",
                "system_requirements": {{}},
                "install_method": {{ "type": "binary", "url": "https://example.com/signed-agent" }},
                "capabilities": ["chat"],
                "ipc_protocol_version": 1,
                "sha256": "{sha}",
                "publisher": "totally-unknown-publisher",
                "signature": "{sig}"
            }}"#,
            sha = sha,
            sig = "aa".repeat(64),
        );
        let mut registry = MockRegistry::new();
        registry.add_agent("signed-agent", &manifest_json, binary);

        let err = installer.install("signed-agent", &registry).await.unwrap_err();
        match err {
            InstallerError::SignatureVerificationFailed(SigningError::UnknownPublisher(p)) => {
                assert_eq!(p, "totally-unknown-publisher");
            }
            other => panic!("expected UnknownPublisher, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_list_installed_empty() {
        let tmp = TempDir::new().unwrap();
        let installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        assert!(installer.list_installed().is_empty());
    }

    #[tokio::test]
    async fn test_list_installed_after_install() {
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let mut registry = MockRegistry::new();
        registry.add_agent(
            "agent-a",
            &sample_manifest_json("agent-a", "1.0.0", &[1]),
            vec![1],
        );
        registry.add_agent(
            "agent-b",
            &sample_manifest_json("agent-b", "2.0.0", &[2]),
            vec![2],
        );

        installer.install("agent-a", &registry).await.unwrap();
        installer.install("agent-b", &registry).await.unwrap();

        let list = installer.list_installed();
        assert_eq!(list.len(), 2);
        let names: Vec<&str> = list.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"agent-a"));
        assert!(names.contains(&"agent-b"));
    }

    #[tokio::test]
    async fn test_remove_agent_success() {
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let mut registry = MockRegistry::new();
        registry.add_agent(
            "test-agent",
            &sample_manifest_json("test-agent", "1.0.0", &[1]),
            vec![1],
        );

        installer.install("test-agent", &registry).await.unwrap();
        assert!(installer.is_installed("test-agent"));

        installer.remove("test-agent").unwrap();
        assert!(!installer.is_installed("test-agent"));
        assert!(!tmp.path().join(AGENTS_DIR).join("test-agent").exists());
    }

    #[tokio::test]
    async fn test_remove_agent_not_installed() {
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let result = installer.remove("nonexistent");
        assert!(matches!(result, Err(InstallerError::NotInstalled(_))));
    }

    #[tokio::test]
    async fn test_update_agent_success() {
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let mut registry = MockRegistry::new();
        registry.add_agent(
            "test-agent",
            &sample_manifest_json("test-agent", "1.0.0", &[1]),
            vec![1],
        );

        installer.install("test-agent", &registry).await.unwrap();

        // Update registry with new version.
        registry = MockRegistry::new();
        registry.add_agent(
            "test-agent",
            &sample_manifest_json("test-agent", "2.0.0", &[2]),
            vec![2],
        );

        let result = installer.update("test-agent", &registry).await.unwrap();
        assert_eq!(result.version, "2.0.0");
        assert_eq!(
            installer.get_installed("test-agent").unwrap().version,
            "2.0.0"
        );
    }

    #[tokio::test]
    async fn test_update_agent_already_latest() {
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let mut registry = MockRegistry::new();
        registry.add_agent(
            "test-agent",
            &sample_manifest_json("test-agent", "1.0.0", &[1]),
            vec![1],
        );

        installer.install("test-agent", &registry).await.unwrap();
        let result = installer.update("test-agent", &registry).await.unwrap();
        assert_eq!(result.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_update_agent_not_installed() {
        let tmp = TempDir::new().unwrap();
        let mut installer = PackageInstaller::new_in(tmp.path().join(AGENTS_DIR));
        let registry = MockRegistry::new();
        let result = installer.update("nonexistent", &registry).await;
        assert!(matches!(result, Err(InstallerError::NotInstalled(_))));
    }

    #[tokio::test]
    async fn test_persist_and_reload() {
        let tmp = TempDir::new().unwrap();
        let agents_dir = tmp.path().join(AGENTS_DIR);

        {
            let mut installer = PackageInstaller::new_in(agents_dir.clone());
            let mut registry = MockRegistry::new();
            registry.add_agent(
                "persist-agent",
                &sample_manifest_json("persist-agent", "1.0.0", &[1]),
                vec![1],
            );
            installer.install("persist-agent", &registry).await.unwrap();
        }

        // Create a new installer from the same directory — it should reload.
        let installer2 = PackageInstaller::new_in(agents_dir);
        assert!(installer2.is_installed("persist-agent"));
        let list = installer2.list_installed();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "persist-agent");
    }

    #[test]
    fn test_installer_error_display() {
        let err = InstallerError::AlreadyInstalled("my-agent".to_string());
        assert_eq!(
            err.to_string(),
            "installer: agent \"my-agent\" is already installed"
        );
    }
}
