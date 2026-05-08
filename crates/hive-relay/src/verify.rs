//! Ed25519 signature verification for incoming envelopes.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::proto::HiveEnvelope;

/// Verify an envelope's Ed25519 signature.
///
/// Returns `Ok(())` if the signature is valid, `Err(reason)` otherwise.
pub fn verify_envelope(envelope: &HiveEnvelope) -> Result<(), String> {
    if envelope.version != 1 {
        return Err(format!(
            "Unsupported protocol version: {} (expected 1)",
            envelope.version
        ));
    }

    let pubkey_bytes: [u8; 32] = envelope
        .sender_pubkey
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid public key length (expected 32 bytes)".to_string())?;

    let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
        .map_err(|e| format!("Invalid public key: {e}"))?;

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

/// Build canonical sign-input bytes matching the client's signing scheme.
///
/// Format: `version (1) ∥ msg_type (1) ∥ sender_id (var) ∥ timestamp (8 LE) ∥ hlc_counter (8 LE) ∥ payload (var)`
fn sign_input(envelope: &HiveEnvelope) -> Vec<u8> {
    let mut buf = Vec::with_capacity(
        1 + 1 + envelope.sender_id.len() + 8 + 8 + envelope.payload.len(),
    );
    buf.push(envelope.version as u8);
    buf.push(envelope.msg_type as u8);
    buf.extend_from_slice(envelope.sender_id.as_bytes());
    buf.extend_from_slice(&envelope.timestamp.to_le_bytes());
    buf.extend_from_slice(&envelope.hlc_counter.to_le_bytes());
    buf.extend_from_slice(&envelope.payload);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand_core::OsRng;

    fn make_signed_envelope(signing_key: &SigningKey) -> HiveEnvelope {
        let mut envelope = HiveEnvelope {
            version: 1,
            msg_type: 0, // BUNDLE
            sender_id: "test-device".into(),
            sender_pubkey: signing_key.verifying_key().to_bytes().to_vec(),
            timestamp: 1_700_000_000_000,
            hlc_counter: 42,
            payload: b"test payload".to_vec(),
            signature: Vec::new(),
            compressed: false,
        };

        let input = sign_input(&envelope);
        let sig = signing_key.sign(&input);
        envelope.signature = sig.to_bytes().to_vec();
        envelope
    }

    #[test]
    fn valid_signature_passes() {
        let key = SigningKey::generate(&mut OsRng);
        let envelope = make_signed_envelope(&key);
        assert!(verify_envelope(&envelope).is_ok());
    }

    #[test]
    fn tampered_payload_rejected() {
        let key = SigningKey::generate(&mut OsRng);
        let mut envelope = make_signed_envelope(&key);
        envelope.payload = b"tampered".to_vec();
        assert!(verify_envelope(&envelope).is_err());
    }

    #[test]
    fn wrong_key_rejected() {
        let key = SigningKey::generate(&mut OsRng);
        let other = SigningKey::generate(&mut OsRng);
        let mut envelope = make_signed_envelope(&key);
        envelope.sender_pubkey = other.verifying_key().to_bytes().to_vec();
        assert!(verify_envelope(&envelope).is_err());
    }
}
