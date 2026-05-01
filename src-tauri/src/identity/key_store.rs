use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use super::device::DeviceIdentity;

const KEY_FILE: &str = "device_key.json";

#[derive(Serialize, Deserialize)]
struct StoredKey {
    device_id: String,
    signing_key_b64: String,
}

/// Load the device identity from `data_dir/device_key.json`, or generate and
/// persist a fresh one if no file exists yet.
pub fn load_or_generate_identity(data_dir: &Path) -> Result<DeviceIdentity, String> {
    let key_path = data_dir.join(KEY_FILE);

    if key_path.exists() {
        let contents = fs::read_to_string(&key_path).map_err(|e| e.to_string())?;
        let stored: StoredKey = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
        let key_bytes = BASE64
            .decode(&stored.signing_key_b64)
            .map_err(|e| e.to_string())?;
        DeviceIdentity::from_bytes(stored.device_id, &key_bytes)
    } else {
        let identity = DeviceIdentity::generate();
        persist_identity(data_dir, &identity)?;
        Ok(identity)
    }
}

/// Write the identity's key bytes to disk.
pub fn persist_identity(data_dir: &Path, identity: &DeviceIdentity) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let stored = StoredKey {
        device_id: identity.device_id.clone(),
        signing_key_b64: BASE64.encode(identity.to_key_bytes()),
    };
    let contents = serde_json::to_string(&stored).map_err(|e| e.to_string())?;
    fs::write(data_dir.join(KEY_FILE), contents).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn generates_new_identity_when_no_file_exists() {
        let tmp = TempDir::new().unwrap();
        let identity = load_or_generate_identity(tmp.path()).unwrap();
        assert!(!identity.device_id.is_empty());
        assert!(tmp.path().join(KEY_FILE).exists());
    }

    #[test]
    fn loads_same_identity_on_second_call() {
        let tmp = TempDir::new().unwrap();
        let first = load_or_generate_identity(tmp.path()).unwrap();
        let second = load_or_generate_identity(tmp.path()).unwrap();
        assert_eq!(first.device_id, second.device_id);
        assert_eq!(first.public_key_b64(), second.public_key_b64());
    }
}
