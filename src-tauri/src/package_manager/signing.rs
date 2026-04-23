//! Manifest signing for downloadable third-party agents.
//!
//! Real binary distribution (Chunk 1.7) requires a curated trust model:
//!
//! 1. **Mandatory `sha256`** on every `Binary` / `Wasm` install method —
//!    enforced by [`crate::package_manager::PackageInstaller::install`].
//! 2. **Optional Ed25519 signature** on the manifest itself, verified
//!    against a curated allow-list of publisher public keys recorded in
//!    [`PUBLISHER_ALLOW_LIST`].
//!
//! When a manifest declares a `publisher`, the installer looks up that
//! publisher's Ed25519 public key in the allow-list, then verifies the
//! detached signature against the canonical "to-be-signed" payload
//! produced by [`canonical_signing_payload`]. The signing payload is
//! intentionally narrow — only the fields that pin the binary identity
//! (`name`, `version`, `install_method`, `sha256`) are signed — so
//! cosmetic edits (description, homepage, capabilities ordering) do not
//! invalidate signatures.
//!
//! ## Hosting model (informational)
//!
//! Third-party agent binaries are hosted on either:
//! - **GitHub Releases** attached to a tagged version of the publisher's
//!   repo (recommended — free, durable, integrity-friendly), or
//! - **S3 / Cloudflare R2** with a stable HTTPS URL.
//!
//! The `Binary { url }` field in the manifest points directly at the
//! download URL. The TerranSoul HTTP registry's `/agents/{name}/download`
//! route returns a `307 Temporary Redirect` to that URL so the registry
//! itself does not consume bandwidth proxying agent binaries.

use crate::package_manager::{AgentManifest, InstallMethod};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// A curated entry mapping a publisher identifier to their Ed25519 public key.
///
/// Publisher identifiers are short, stable strings (e.g. `"terransoul-team"`).
/// Public keys are 32 raw bytes. To rotate a key, ship a new TerranSoul
/// release with the new entry; old signatures continue to verify against
/// the old entry as long as both rows are present.
#[derive(Debug, Clone, Copy)]
pub struct PublisherEntry {
    /// Publisher identifier referenced by the manifest's `publisher` field.
    pub id: &'static str,
    /// Raw 32-byte Ed25519 public key.
    pub public_key: [u8; 32],
}

/// Curated allow-list of publishers trusted to ship downloadable agents.
///
/// **Empty by default.** Real publisher keys are added here in PRs reviewed
/// by maintainers; the allow-list is compiled into the binary so a
/// compromised registry cannot inject new "trusted" publishers at runtime.
pub const PUBLISHER_ALLOW_LIST: &[PublisherEntry] = &[];

/// Errors produced by signature verification.
#[derive(Debug, Clone, PartialEq)]
pub enum SigningError {
    /// The manifest declared a `publisher` that is not on the allow-list.
    UnknownPublisher(String),
    /// The `signature` field could not be hex-decoded.
    InvalidSignatureEncoding(String),
    /// The signature was the wrong length (Ed25519 signatures are 64 bytes).
    InvalidSignatureLength(usize),
    /// The signature did not verify against the publisher's public key.
    SignatureMismatch,
    /// A publisher's allow-list entry contained an invalid public key.
    InvalidPublicKey(String),
}

impl std::fmt::Display for SigningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SigningError::UnknownPublisher(p) => {
                write!(f, "signing: publisher \"{p}\" is not on the allow-list")
            }
            SigningError::InvalidSignatureEncoding(e) => {
                write!(f, "signing: signature is not valid hex: {e}")
            }
            SigningError::InvalidSignatureLength(n) => {
                write!(f, "signing: signature has wrong length (expected 64 bytes, got {n})")
            }
            SigningError::SignatureMismatch => {
                write!(f, "signing: signature does not match publisher key")
            }
            SigningError::InvalidPublicKey(e) => {
                write!(f, "signing: publisher public key is invalid: {e}")
            }
        }
    }
}

/// Build the canonical "to-be-signed" payload for a manifest.
///
/// Only the fields that pin binary identity are signed:
/// - `name`
/// - `version`
/// - install method discriminator + URL/path
/// - `sha256` (if present)
///
/// This format is intentionally simple, deterministic, and stable: a
/// cosmetic edit (description, homepage, capability ordering) must not
/// invalidate an existing signature, but swapping the binary URL or the
/// hash must.
pub fn canonical_signing_payload(manifest: &AgentManifest) -> Vec<u8> {
    let install_descriptor = match &manifest.install_method {
        InstallMethod::Binary { url } => format!("binary|{url}"),
        InstallMethod::Wasm { url } => format!("wasm|{url}"),
        InstallMethod::Sidecar { path } => format!("sidecar|{path}"),
        InstallMethod::BuiltIn => "builtin".to_string(),
    };
    let sha = manifest.sha256.as_deref().unwrap_or("");
    let payload = format!(
        "terransoul-agent-manifest:v1\nname={}\nversion={}\ninstall={}\nsha256={}",
        manifest.name, manifest.version, install_descriptor, sha
    );
    payload.into_bytes()
}

/// Look up a publisher's public key in the curated allow-list.
pub fn publisher_key(publisher: &str) -> Option<[u8; 32]> {
    publisher_key_in(PUBLISHER_ALLOW_LIST, publisher)
}

/// Look up a publisher's public key in a caller-supplied allow-list.
///
/// Exposed separately so tests can inject a fixture allow-list without
/// mutating global state.
pub fn publisher_key_in(allow_list: &[PublisherEntry], publisher: &str) -> Option<[u8; 32]> {
    allow_list
        .iter()
        .find(|e| e.id == publisher)
        .map(|e| e.public_key)
}

/// Verify a detached Ed25519 signature against a manifest using the
/// global [`PUBLISHER_ALLOW_LIST`].
pub fn verify_manifest_signature(manifest: &AgentManifest) -> Result<(), SigningError> {
    verify_manifest_signature_with(manifest, PUBLISHER_ALLOW_LIST)
}

/// Verify a detached Ed25519 signature against a manifest using a
/// caller-supplied allow-list. Used by tests with fixture keys.
///
/// Returns `Ok(())` if the manifest carries no `publisher` (signing is
/// optional). When a `publisher` is set, both `publisher` and `signature`
/// must be present and the signature must verify.
pub fn verify_manifest_signature_with(
    manifest: &AgentManifest,
    allow_list: &[PublisherEntry],
) -> Result<(), SigningError> {
    let Some(publisher) = manifest.publisher.as_deref() else {
        // No publisher → no signature requirement (the SHA-256 check on
        // the binary itself is still mandatory; that is enforced by the
        // installer, not here).
        return Ok(());
    };
    let signature_hex = manifest
        .signature
        .as_deref()
        .ok_or_else(|| SigningError::UnknownPublisher(publisher.to_string()))?;

    let pk_bytes = publisher_key_in(allow_list, publisher)
        .ok_or_else(|| SigningError::UnknownPublisher(publisher.to_string()))?;

    let sig_bytes =
        hex::decode(signature_hex).map_err(|e| SigningError::InvalidSignatureEncoding(e.to_string()))?;
    if sig_bytes.len() != 64 {
        return Err(SigningError::InvalidSignatureLength(sig_bytes.len()));
    }
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);
    let signature = Signature::from_bytes(&sig_arr);

    let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|e| SigningError::InvalidPublicKey(e.to_string()))?;

    let payload = canonical_signing_payload(manifest);
    verifying_key
        .verify(&payload, &signature)
        .map_err(|_| SigningError::SignatureMismatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package_manager::{Capability, SystemRequirements};
    use ed25519_dalek::{Signer, SigningKey};

    fn manifest_with_publisher(
        publisher: Option<&str>,
        signature: Option<&str>,
        sha: &str,
    ) -> AgentManifest {
        AgentManifest {
            name: "third-party-agent".to_string(),
            version: "1.2.3".to_string(),
            description: "Test third-party agent".to_string(),
            system_requirements: SystemRequirements {
                min_ram_mb: 0,
                os: vec![],
                arch: vec![],
                gpu_required: false,
            },
            install_method: InstallMethod::Binary {
                url: "https://example.com/third-party-agent-v1.2.3.bin".to_string(),
            },
            capabilities: vec![Capability::Chat],
            ipc_protocol_version: 1,
            homepage: None,
            license: None,
            author: None,
            sha256: Some(sha.to_string()),
            publisher: publisher.map(str::to_string),
            signature: signature.map(str::to_string),
        }
    }

    fn fixture_keypair() -> (SigningKey, [u8; 32]) {
        // Deterministic signing key for tests — never used in production.
        // The 32-byte seed is arbitrary; we just need stability across runs.
        let seed: [u8; 32] = *b"terransoul-test-fixture-seed-32!";
        let signing_key = SigningKey::from_bytes(&seed);
        let pk = signing_key.verifying_key().to_bytes();
        (signing_key, pk)
    }

    #[test]
    fn no_publisher_means_no_signature_check() {
        let m = manifest_with_publisher(
            None,
            None,
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        assert!(verify_manifest_signature(&m).is_ok());
    }

    #[test]
    fn signature_verifies_against_canonical_payload() {
        let (signing_key, pk) = fixture_keypair();
        let allow_list = &[PublisherEntry {
            id: "fixture-publisher",
            public_key: pk,
        }];

        let mut m = manifest_with_publisher(
            Some("fixture-publisher"),
            None,
            "1111111111111111111111111111111111111111111111111111111111111111",
        );
        let payload = canonical_signing_payload(&m);
        let sig = signing_key.sign(&payload);
        m.signature = Some(hex::encode(sig.to_bytes()));

        verify_manifest_signature_with(&m, allow_list).expect("signature must verify");
    }

    #[test]
    fn cosmetic_changes_do_not_invalidate_signature() {
        let (signing_key, pk) = fixture_keypair();
        let allow_list = &[PublisherEntry {
            id: "fixture-publisher",
            public_key: pk,
        }];

        let mut m = manifest_with_publisher(
            Some("fixture-publisher"),
            None,
            "2222222222222222222222222222222222222222222222222222222222222222",
        );
        let payload = canonical_signing_payload(&m);
        let sig = signing_key.sign(&payload);
        m.signature = Some(hex::encode(sig.to_bytes()));

        // Mutate cosmetic-only fields.
        m.description = "A different description".to_string();
        m.homepage = Some("https://example.org/new-homepage".to_string());
        m.license = Some("Apache-2.0".to_string());
        m.author = Some("New Author".to_string());

        verify_manifest_signature_with(&m, allow_list)
            .expect("cosmetic changes must not invalidate signature");
    }

    #[test]
    fn changing_binary_url_invalidates_signature() {
        let (signing_key, pk) = fixture_keypair();
        let allow_list = &[PublisherEntry {
            id: "fixture-publisher",
            public_key: pk,
        }];

        let mut m = manifest_with_publisher(
            Some("fixture-publisher"),
            None,
            "3333333333333333333333333333333333333333333333333333333333333333",
        );
        let payload = canonical_signing_payload(&m);
        let sig = signing_key.sign(&payload);
        m.signature = Some(hex::encode(sig.to_bytes()));

        // Swap the binary URL — must invalidate.
        m.install_method = InstallMethod::Binary {
            url: "https://attacker.example.com/malicious.bin".to_string(),
        };

        let err = verify_manifest_signature_with(&m, allow_list).unwrap_err();
        assert_eq!(err, SigningError::SignatureMismatch);
    }

    #[test]
    fn changing_sha256_invalidates_signature() {
        let (signing_key, pk) = fixture_keypair();
        let allow_list = &[PublisherEntry {
            id: "fixture-publisher",
            public_key: pk,
        }];

        let mut m = manifest_with_publisher(
            Some("fixture-publisher"),
            None,
            "4444444444444444444444444444444444444444444444444444444444444444",
        );
        let payload = canonical_signing_payload(&m);
        let sig = signing_key.sign(&payload);
        m.signature = Some(hex::encode(sig.to_bytes()));

        m.sha256 =
            Some("5555555555555555555555555555555555555555555555555555555555555555".to_string());

        let err = verify_manifest_signature_with(&m, allow_list).unwrap_err();
        assert_eq!(err, SigningError::SignatureMismatch);
    }

    #[test]
    fn unknown_publisher_is_rejected() {
        let m = manifest_with_publisher(
            Some("not-on-allow-list"),
            Some(&"aa".repeat(64)),
            "6666666666666666666666666666666666666666666666666666666666666666",
        );
        let err = verify_manifest_signature_with(&m, &[]).unwrap_err();
        assert!(matches!(err, SigningError::UnknownPublisher(_)));
    }

    #[test]
    fn publisher_without_signature_is_rejected() {
        let (_signing_key, pk) = fixture_keypair();
        let allow_list = &[PublisherEntry {
            id: "fixture-publisher",
            public_key: pk,
        }];
        let m = manifest_with_publisher(
            Some("fixture-publisher"),
            None, // missing signature
            "7777777777777777777777777777777777777777777777777777777777777777",
        );
        let err = verify_manifest_signature_with(&m, allow_list).unwrap_err();
        assert!(matches!(err, SigningError::UnknownPublisher(_)));
    }

    #[test]
    fn invalid_hex_signature_is_rejected() {
        let (_signing_key, pk) = fixture_keypair();
        let allow_list = &[PublisherEntry {
            id: "fixture-publisher",
            public_key: pk,
        }];
        let m = manifest_with_publisher(
            Some("fixture-publisher"),
            Some("not-hex!"),
            "8888888888888888888888888888888888888888888888888888888888888888",
        );
        let err = verify_manifest_signature_with(&m, allow_list).unwrap_err();
        assert!(matches!(err, SigningError::InvalidSignatureEncoding(_)));
    }

    #[test]
    fn wrong_length_signature_is_rejected() {
        let (_signing_key, pk) = fixture_keypair();
        let allow_list = &[PublisherEntry {
            id: "fixture-publisher",
            public_key: pk,
        }];
        let m = manifest_with_publisher(
            Some("fixture-publisher"),
            Some(&"aa".repeat(32)), // 32 bytes, not 64
            "9999999999999999999999999999999999999999999999999999999999999999",
        );
        let err = verify_manifest_signature_with(&m, allow_list).unwrap_err();
        assert!(matches!(err, SigningError::InvalidSignatureLength(32)));
    }

    #[test]
    fn allow_list_is_empty_by_default() {
        // Publishers must be added by maintainers in code-reviewed PRs.
        // Runtime injection is not supported.
        assert!(PUBLISHER_ALLOW_LIST.is_empty());
    }

    #[test]
    fn canonical_payload_is_deterministic() {
        let m = manifest_with_publisher(
            None,
            None,
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        );
        let p1 = canonical_signing_payload(&m);
        let p2 = canonical_signing_payload(&m);
        assert_eq!(p1, p2);
        let s = String::from_utf8(p1).unwrap();
        assert!(s.contains("name=third-party-agent"));
        assert!(s.contains("version=1.2.3"));
        assert!(s.contains("install=binary|https://example.com/third-party-agent-v1.2.3.bin"));
        assert!(s.contains("sha256=abcdef0123456789"));
    }
}
