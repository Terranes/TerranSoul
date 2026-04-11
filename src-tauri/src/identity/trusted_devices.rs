use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use std::time::{SystemTime, UNIX_EPOCH};

const DEVICES_FILE: &str = "trusted_devices.json";

/// A remote device that has been paired with this instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub device_id: String,
    pub name: String,
    pub public_key_b64: String,
    pub paired_at: u64,
}

impl TrustedDevice {
    pub fn new(device_id: String, name: String, public_key_b64: String) -> Self {
        let paired_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        TrustedDevice { device_id, name, public_key_b64, paired_at }
    }
}

/// Add `device` to `devices`, replacing any existing entry with the same `device_id`.
pub fn add_trusted_device(devices: &mut Vec<TrustedDevice>, device: TrustedDevice) {
    devices.retain(|d| d.device_id != device.device_id);
    devices.push(device);
}

/// Remove the device with `device_id` from `devices`.  Returns `true` if it was present.
pub fn remove_trusted_device(devices: &mut Vec<TrustedDevice>, device_id: &str) -> bool {
    let before = devices.len();
    devices.retain(|d| d.device_id != device_id);
    devices.len() < before
}

/// Load trusted devices from `data_dir/trusted_devices.json`, returning an empty list on any error.
pub fn load_trusted_devices(data_dir: &Path) -> Vec<TrustedDevice> {
    let path = data_dir.join(DEVICES_FILE);
    if !path.exists() {
        return vec![];
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist the current trusted device list to `data_dir/trusted_devices.json`.
pub fn save_trusted_devices(data_dir: &Path, devices: &[TrustedDevice]) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let contents = serde_json::to_string_pretty(devices).map_err(|e| e.to_string())?;
    fs::write(data_dir.join(DEVICES_FILE), contents).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_device(id: &str) -> TrustedDevice {
        TrustedDevice {
            device_id: id.to_string(),
            name: format!("Device {id}"),
            public_key_b64: "dGVzdA==".to_string(),
            paired_at: 0,
        }
    }

    #[test]
    fn add_device_appends_to_list() {
        let mut devices = vec![];
        add_trusted_device(&mut devices, make_device("a"));
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, "a");
    }

    #[test]
    fn add_device_replaces_existing_same_id() {
        let mut devices = vec![make_device("a")];
        let mut updated = make_device("a");
        updated.name = "Updated A".to_string();
        add_trusted_device(&mut devices, updated);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Updated A");
    }

    #[test]
    fn remove_existing_device_returns_true() {
        let mut devices = vec![make_device("a"), make_device("b")];
        let removed = remove_trusted_device(&mut devices, "a");
        assert!(removed);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, "b");
    }

    #[test]
    fn remove_nonexistent_device_returns_false() {
        let mut devices = vec![make_device("a")];
        let removed = remove_trusted_device(&mut devices, "z");
        assert!(!removed);
        assert_eq!(devices.len(), 1);
    }

    #[test]
    fn save_and_load_roundtrip() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let devices = vec![make_device("x"), make_device("y")];
        save_trusted_devices(tmp.path(), &devices).unwrap();
        let loaded = load_trusted_devices(tmp.path());
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].device_id, "x");
        assert_eq!(loaded[1].device_id, "y");
    }

    #[test]
    fn load_returns_empty_when_file_absent() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let devices = load_trusted_devices(tmp.path());
        assert!(devices.is_empty());
    }
}
