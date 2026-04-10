use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// In-memory representation of this device's Ed25519 identity.
pub struct DeviceIdentity {
    pub device_id: String,
    signing_key: SigningKey,
}

/// Serialisable summary safe to expose to the frontend or encode in a QR code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub public_key_b64: String,
    pub name: String,
}

impl DeviceIdentity {
    /// Generate a brand-new random identity.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        DeviceIdentity {
            device_id: Uuid::new_v4().to_string(),
            signing_key,
        }
    }

    /// Reconstruct an identity from persisted key bytes (32-byte raw signing key).
    pub fn from_bytes(device_id: String, key_bytes: &[u8]) -> Result<Self, String> {
        let bytes: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| "Invalid key length; expected 32 bytes".to_string())?;
        Ok(DeviceIdentity {
            device_id,
            signing_key: SigningKey::from_bytes(&bytes),
        })
    }

    /// Raw 32-byte signing key for persistent storage.
    pub fn to_key_bytes(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }

    /// Base64-encoded public (verifying) key.
    pub fn public_key_b64(&self) -> String {
        BASE64.encode(self.signing_key.verifying_key().to_bytes())
    }

    /// Build a `DeviceInfo` suitable for sending to the frontend or embedding in a QR code.
    pub fn device_info(&self, name: &str) -> DeviceInfo {
        DeviceInfo {
            device_id: self.device_id.clone(),
            public_key_b64: self.public_key_b64(),
            name: name.to_string(),
        }
    }
}

/// Best-effort hostname lookup; falls back to a generic label.
pub fn device_name() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "TerranSoul Device".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOSTNAME").unwrap_or_else(|_| "TerranSoul Device".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_different_device_ids() {
        let a = DeviceIdentity::generate();
        let b = DeviceIdentity::generate();
        assert_ne!(a.device_id, b.device_id);
    }

    #[test]
    fn device_identity_has_valid_uuid() {
        let identity = DeviceIdentity::generate();
        assert!(Uuid::parse_str(&identity.device_id).is_ok());
    }

    #[test]
    fn key_roundtrip_from_bytes() {
        let original = DeviceIdentity::generate();
        let key_bytes = original.to_key_bytes();
        let restored =
            DeviceIdentity::from_bytes(original.device_id.clone(), &key_bytes).unwrap();
        assert_eq!(restored.public_key_b64(), original.public_key_b64());
        assert_eq!(restored.device_id, original.device_id);
    }

    #[test]
    fn from_bytes_rejects_wrong_length() {
        let result = DeviceIdentity::from_bytes("id".to_string(), &[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn public_key_b64_is_valid_base64() {
        let identity = DeviceIdentity::generate();
        let b64 = identity.public_key_b64();
        assert!(BASE64.decode(&b64).is_ok());
        assert!(!b64.is_empty());
    }

    #[test]
    fn device_info_contains_expected_fields() {
        let identity = DeviceIdentity::generate();
        let info = identity.device_info("test-machine");
        assert_eq!(info.device_id, identity.device_id);
        assert_eq!(info.public_key_b64, identity.public_key_b64());
        assert_eq!(info.name, "test-machine");
    }
}
