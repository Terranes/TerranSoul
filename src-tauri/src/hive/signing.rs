//! Ed25519 signing and verification for Hive protocol envelopes.
//!
//! Uses `ed25519-dalek` v2 (already a dependency via `identity/`).

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use super::protocol::{HiveEnvelope, MsgType, HIVE_PROTOCOL_VERSION};

/// Build the canonical sign-input bytes for an envelope.
///
/// Format: `version (1) ∥ msg_type (1) ∥ sender_id (var) ∥ timestamp (8 LE) ∥ hlc_counter (8 LE) ∥ payload (var)`
pub fn sign_input(envelope: &HiveEnvelope) -> Vec<u8> {
    let mut buf =
        Vec::with_capacity(1 + 1 + envelope.sender_id.len() + 8 + 8 + envelope.payload.len());
    buf.push(envelope.version);
    buf.push(envelope.msg_type as u8);
    buf.extend_from_slice(envelope.sender_id.as_bytes());
    buf.extend_from_slice(&envelope.timestamp.to_le_bytes());
    buf.extend_from_slice(&envelope.hlc_counter.to_le_bytes());
    buf.extend_from_slice(&envelope.payload);
    buf
}

/// Sign an envelope in-place using the device's Ed25519 signing key.
///
/// Populates `envelope.signature` and `envelope.sender_pubkey`.
pub fn sign_envelope(envelope: &mut HiveEnvelope, signing_key: &SigningKey) {
    envelope.sender_pubkey = signing_key.verifying_key().to_bytes().to_vec();
    let input = sign_input(envelope);
    let sig: Signature = signing_key.sign(&input);
    envelope.signature = sig.to_bytes().to_vec();
}

/// Verify an envelope's Ed25519 signature against its embedded public key.
///
/// Returns `Ok(())` if valid, `Err(reason)` otherwise.
pub fn verify_envelope(envelope: &HiveEnvelope) -> Result<(), String> {
    if envelope.version != HIVE_PROTOCOL_VERSION {
        return Err(format!(
            "Unsupported protocol version: {} (expected {})",
            envelope.version, HIVE_PROTOCOL_VERSION
        ));
    }

    let pubkey_bytes: [u8; 32] = envelope
        .sender_pubkey
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid public key length (expected 32 bytes)".to_string())?;

    let verifying_key =
        VerifyingKey::from_bytes(&pubkey_bytes).map_err(|e| format!("Invalid public key: {e}"))?;

    let sig_bytes: [u8; 64] = envelope
        .signature
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid signature length (expected 64 bytes)".to_string())?;

    let signature = Signature::from_bytes(&sig_bytes);

    let input = sign_input(envelope);
    verifying_key
        .verify(&input, &signature)
        .map_err(|e| format!("Signature verification failed: {e}"))
}

/// Create a new unsigned envelope for the given message type and payload.
pub fn new_envelope(
    msg_type: MsgType,
    sender_id: String,
    timestamp: u64,
    hlc_counter: u64,
    payload: Vec<u8>,
    compressed: bool,
) -> HiveEnvelope {
    HiveEnvelope {
        version: HIVE_PROTOCOL_VERSION,
        msg_type,
        sender_id,
        sender_pubkey: Vec::new(),
        timestamp,
        hlc_counter,
        payload,
        signature: Vec::new(),
        compressed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    fn sign_and_verify_roundtrip() {
        let signing_key = SigningKey::generate(&mut OsRng);

        let mut envelope = new_envelope(
            MsgType::Bundle,
            "device-test-001".into(),
            1_700_000_000_000,
            42,
            b"hello hive".to_vec(),
            false,
        );

        sign_envelope(&mut envelope, &signing_key);

        assert_eq!(envelope.sender_pubkey.len(), 32);
        assert_eq!(envelope.signature.len(), 64);

        // Verification should pass
        verify_envelope(&envelope).expect("valid signature should verify");
    }

    #[test]
    fn tampered_payload_fails_verification() {
        let signing_key = SigningKey::generate(&mut OsRng);

        let mut envelope = new_envelope(
            MsgType::Op,
            "device-test-002".into(),
            1_700_000_000_000,
            99,
            b"original payload".to_vec(),
            false,
        );

        sign_envelope(&mut envelope, &signing_key);

        // Tamper with the payload
        envelope.payload = b"tampered payload".to_vec();

        let result = verify_envelope(&envelope);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Signature verification failed"));
    }

    #[test]
    fn wrong_key_fails_verification() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let other_key = SigningKey::generate(&mut OsRng);

        let mut envelope = new_envelope(
            MsgType::Job,
            "device-test-003".into(),
            1_700_000_000_000,
            1,
            b"job payload".to_vec(),
            false,
        );

        sign_envelope(&mut envelope, &signing_key);

        // Replace pubkey with a different device's key
        envelope.sender_pubkey = other_key.verifying_key().to_bytes().to_vec();

        let result = verify_envelope(&envelope);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_version_rejected() {
        let signing_key = SigningKey::generate(&mut OsRng);

        let mut envelope = new_envelope(
            MsgType::Bundle,
            "device-test-004".into(),
            1_700_000_000_000,
            1,
            vec![],
            false,
        );

        sign_envelope(&mut envelope, &signing_key);
        envelope.version = 99; // Unsupported version

        let result = verify_envelope(&envelope);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported protocol version"));
    }

    #[test]
    fn sign_input_deterministic() {
        let envelope = new_envelope(
            MsgType::Bundle,
            "dev-001".into(),
            1000,
            5,
            b"data".to_vec(),
            false,
        );

        let a = sign_input(&envelope);
        let b = sign_input(&envelope);
        assert_eq!(a, b);
    }
}
