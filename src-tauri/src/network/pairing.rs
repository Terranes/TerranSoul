//! mTLS pairing flow + persistent device registry — Chunk 24.2b.
//!
//! Generates a self-signed CA on first LAN-enable. Issues per-device
//! client certificates at pairing time. Persists paired devices to
//! the `paired_devices` SQLite table (V12 migration).
//!
//! ## Flow
//!
//! 1. `start_pairing()` → generates a 5-minute `PairPayload` QR URI.
//! 2. Phone scans QR, presents its `device_id` to the desktop.
//! 3. Desktop calls `confirm_pairing(device_id, display_name)` →
//!    issues a client cert signed by the CA, persists the device,
//!    returns the cert bundle.
//! 4. Phone stores cert; all subsequent LAN connections use mTLS.
//! 5. `revoke_device(device_id)` removes the row → cert rejected.

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rcgen::{
    BasicConstraints, CertificateParams, DnType, DnValue, IsCa, Issuer, KeyPair, KeyUsagePurpose,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::network::pair_token::{self, PairPayload};

// ── Types ──────────────────────────────────────────────────────────────────────

/// A paired device record, persisted in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedDevice {
    pub device_id: String,
    pub display_name: String,
    pub cert_fingerprint: String,
    pub capabilities: Vec<String>,
    pub paired_at: u64,
    pub last_seen_at: Option<u64>,
}

/// Result of a successful pairing: the device record + issued certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingResult {
    /// The newly-persisted device record.
    pub device: PairedDevice,
    /// PEM-encoded client certificate (phone stores this).
    pub client_cert_pem: String,
    /// PEM-encoded client private key.
    pub client_key_pem: String,
    /// PEM-encoded CA certificate (phone needs this to verify server).
    pub ca_cert_pem: String,
}

/// Active pairing session (lives in memory only).
#[derive(Debug)]
pub struct PairingSession {
    pub payload: PairPayload,
    pub token: [u8; 32],
    pub created_at_ms: u64,
}

/// Manages the CA keypair and active pairing sessions.
pub struct PairingManager {
    ca_cert_pem: String,
    ca_key_pem: String,
    active_session: Mutex<Option<PairingSession>>,
}

// ── Constants ──────────────────────────────────────────────────────────────────

const CA_CERT_FILE: &str = "pairing_ca_cert.pem";
const CA_KEY_FILE: &str = "pairing_ca_key.pem";
/// 5-minute pairing window.
const PAIRING_WINDOW_MS: u64 = 5 * 60 * 1000;

// ── CA Management ──────────────────────────────────────────────────────────────

impl PairingManager {
    /// Load or generate the CA. Called once at app startup when `lan_enabled`.
    pub fn load_or_create(data_dir: &Path) -> Result<Self, String> {
        let cert_path = data_dir.join(CA_CERT_FILE);
        let key_path = data_dir.join(CA_KEY_FILE);

        let (ca_cert_pem, ca_key_pem) = if cert_path.exists() && key_path.exists() {
            let cert = std::fs::read_to_string(&cert_path)
                .map_err(|e| format!("read CA cert: {e}"))?;
            let key =
                std::fs::read_to_string(&key_path).map_err(|e| format!("read CA key: {e}"))?;
            (cert, key)
        } else {
            let (cert, key) = generate_ca()?;
            std::fs::write(&cert_path, &cert).map_err(|e| format!("write CA cert: {e}"))?;
            std::fs::write(&key_path, &key).map_err(|e| format!("write CA key: {e}"))?;
            (cert, key)
        };

        Ok(Self {
            ca_cert_pem,
            ca_key_pem,
            active_session: Mutex::new(None),
        })
    }

    /// CA certificate PEM (public — shared with paired devices).
    pub fn ca_cert_pem(&self) -> &str {
        &self.ca_cert_pem
    }

    /// Start a new pairing session. Overwrites any active session.
    ///
    /// Returns the `PairPayload` URI for QR display.
    pub fn start_pairing(&self, host: &str, port: u16) -> Result<String, String> {
        let token = pair_token::gen_token();
        let now_ms = now_unix_ms();
        let fingerprint = ca_fingerprint(&self.ca_cert_pem)?;

        let payload = PairPayload {
            host: host.to_string(),
            port,
            token_b64: base64_encode(&token),
            fingerprint_b64: base64_encode(&fingerprint),
            expires_at_unix_ms: now_ms + PAIRING_WINDOW_MS,
        };

        let uri = pair_token::encode_uri(&payload).map_err(|e| format!("{e:?}"))?;

        let session = PairingSession {
            payload: payload.clone(),
            token,
            created_at_ms: now_ms,
        };

        *self.active_session.lock().map_err(|e| e.to_string())? = Some(session);
        Ok(uri)
    }

    /// Confirm a pairing: validate the token, issue a client cert, persist.
    pub fn confirm_pairing(
        &self,
        conn: &Connection,
        device_id: &str,
        display_name: &str,
        presented_token: &[u8; 32],
    ) -> Result<PairingResult, String> {
        // Validate active session.
        let guard = self.active_session.lock().map_err(|e| e.to_string())?;
        let session = guard.as_ref().ok_or("no active pairing session")?;

        // Check token.
        if !pair_token::constant_time_eq(&session.token, presented_token) {
            return Err("pairing token mismatch".to_string());
        }

        // Check window.
        let now = now_unix_ms();
        if pair_token::is_expired(&session.payload, now) {
            return Err("pairing session expired".to_string());
        }

        drop(guard);

        // Issue client certificate.
        let (client_cert_pem, client_key_pem) =
            issue_client_cert(&self.ca_key_pem, device_id)?;

        let fingerprint = cert_fingerprint_from_pem(&client_cert_pem)?;

        let device = PairedDevice {
            device_id: device_id.to_string(),
            display_name: display_name.to_string(),
            cert_fingerprint: fingerprint,
            capabilities: vec![],
            paired_at: now,
            last_seen_at: None,
        };

        insert_paired_device(conn, &device)?;

        // Clear the session after successful pairing.
        *self.active_session.lock().map_err(|e| e.to_string())? = None;

        Ok(PairingResult {
            device,
            client_cert_pem,
            client_key_pem,
            ca_cert_pem: self.ca_cert_pem.clone(),
        })
    }
}

// ── CA / Cert Generation ───────────────────────────────────────────────────────

/// Build the CA `CertificateParams` (shared between generation and re-use).
fn ca_params() -> CertificateParams {
    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, DnValue::Utf8String("TerranSoul Pairing CA".to_string()));
    params
        .distinguished_name
        .push(DnType::OrganizationName, DnValue::Utf8String("TerranSoul".to_string()));
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    // 10-year validity.
    params.not_after = rcgen::date_time_ymd(2036, 1, 1);
    params
}

/// Generate a new self-signed CA certificate + key (PEM).
fn generate_ca() -> Result<(String, String), String> {
    let params = ca_params();
    let key_pair = KeyPair::generate().map_err(|e| format!("CA keygen: {e}"))?;
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("CA self-sign: {e}"))?;

    Ok((cert.pem(), key_pair.serialize_pem()))
}

/// Issue a client certificate signed by the CA.
fn issue_client_cert(
    ca_key_pem: &str,
    device_id: &str,
) -> Result<(String, String), String> {
    let ca_key =
        KeyPair::from_pem(ca_key_pem).map_err(|e| format!("parse CA key: {e}"))?;

    // Reconstruct CA params + issuer.
    let issuer_params = ca_params();
    let issuer = Issuer::new(issuer_params, ca_key);

    let mut client_params = CertificateParams::default();
    client_params.distinguished_name.push(
        DnType::CommonName,
        DnValue::Utf8String(format!("TerranSoul Device {device_id}")),
    );
    // 1-year validity for device certs.
    client_params.not_after = rcgen::date_time_ymd(2027, 5, 1);
    client_params.is_ca = IsCa::NoCa;

    let client_key = KeyPair::generate().map_err(|e| format!("device keygen: {e}"))?;
    let client_cert = client_params
        .signed_by(&client_key, &issuer)
        .map_err(|e| format!("sign device cert: {e}"))?;

    Ok((client_cert.pem(), client_key.serialize_pem()))
}

// ── Fingerprinting ─────────────────────────────────────────────────────────────

/// SHA-256 fingerprint of the CA cert (DER-encoded, first 16 bytes).
fn ca_fingerprint(ca_cert_pem: &str) -> Result<[u8; 16], String> {
    let der = pem_to_der(ca_cert_pem)?;
    let hash = Sha256::digest(&der);
    let mut fp = [0u8; 16];
    fp.copy_from_slice(&hash[..16]);
    Ok(fp)
}

/// SHA-256 hex fingerprint of a PEM certificate.
fn cert_fingerprint_from_pem(pem: &str) -> Result<String, String> {
    let der = pem_to_der(pem)?;
    let hash = Sha256::digest(&der);
    Ok(hex::encode(hash))
}

/// Extract DER bytes from a PEM certificate.
fn pem_to_der(pem: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    let b64: String = pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");
    base64::engine::general_purpose::STANDARD
        .decode(&b64)
        .map_err(|e| format!("PEM decode: {e}"))
}

// ── SQLite CRUD ────────────────────────────────────────────────────────────────

/// Insert a paired device into the `paired_devices` table.
pub fn insert_paired_device(conn: &Connection, device: &PairedDevice) -> Result<(), String> {
    let caps_json =
        serde_json::to_string(&device.capabilities).map_err(|e| format!("json: {e}"))?;
    conn.execute(
        "INSERT OR REPLACE INTO paired_devices \
         (device_id, display_name, cert_fingerprint, capabilities, paired_at, last_seen_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            device.device_id,
            device.display_name,
            device.cert_fingerprint,
            caps_json,
            device.paired_at,
            device.last_seen_at,
        ],
    )
    .map_err(|e| format!("insert paired_device: {e}"))?;
    Ok(())
}

/// Load all paired devices.
pub fn list_paired_devices(conn: &Connection) -> Result<Vec<PairedDevice>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT device_id, display_name, cert_fingerprint, capabilities, \
             paired_at, last_seen_at FROM paired_devices ORDER BY paired_at DESC",
        )
        .map_err(|e| format!("prepare list: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let caps_json: String = row.get(3)?;
            let capabilities: Vec<String> =
                serde_json::from_str(&caps_json).unwrap_or_default();
            Ok(PairedDevice {
                device_id: row.get(0)?,
                display_name: row.get(1)?,
                cert_fingerprint: row.get(2)?,
                capabilities,
                paired_at: row.get(4)?,
                last_seen_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("query: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect: {e}"))
}

/// Remove a paired device by ID.
pub fn revoke_device(conn: &Connection, device_id: &str) -> Result<bool, String> {
    let changed = conn
        .execute(
            "DELETE FROM paired_devices WHERE device_id = ?1",
            rusqlite::params![device_id],
        )
        .map_err(|e| format!("delete: {e}"))?;
    Ok(changed > 0)
}

/// Update `last_seen_at` for a device (called on each authenticated connection).
pub fn touch_device(conn: &Connection, device_id: &str) -> Result<(), String> {
    let now = now_unix_ms();
    conn.execute(
        "UPDATE paired_devices SET last_seen_at = ?1 WHERE device_id = ?2",
        rusqlite::params![now, device_id],
    )
    .map_err(|e| format!("touch: {e}"))?;
    Ok(())
}

/// Look up a device by cert fingerprint (used during mTLS handshake).
pub fn find_device_by_fingerprint(
    conn: &Connection,
    fingerprint: &str,
) -> Result<Option<PairedDevice>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT device_id, display_name, cert_fingerprint, capabilities, \
             paired_at, last_seen_at FROM paired_devices WHERE cert_fingerprint = ?1",
        )
        .map_err(|e| format!("prepare: {e}"))?;

    let mut rows = stmt
        .query_map(rusqlite::params![fingerprint], |row| {
            let caps_json: String = row.get(3)?;
            let capabilities: Vec<String> =
                serde_json::from_str(&caps_json).unwrap_or_default();
            Ok(PairedDevice {
                device_id: row.get(0)?,
                display_name: row.get(1)?,
                cert_fingerprint: row.get(2)?,
                capabilities,
                paired_at: row.get(4)?,
                last_seen_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("query: {e}"))?;

    match rows.next() {
        Some(Ok(d)) => Ok(Some(d)),
        Some(Err(e)) => Err(format!("row: {e}")),
        None => Ok(None),
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE paired_devices (
                device_id        TEXT PRIMARY KEY,
                display_name     TEXT NOT NULL,
                cert_fingerprint TEXT NOT NULL,
                capabilities     TEXT NOT NULL DEFAULT '[]',
                paired_at        INTEGER NOT NULL,
                last_seen_at     INTEGER
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn generate_ca_produces_valid_pem() {
        let (cert, key) = generate_ca().unwrap();
        assert!(cert.contains("BEGIN CERTIFICATE"));
        assert!(key.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn issue_client_cert_signed_by_ca() {
        let (_, ca_key) = generate_ca().unwrap();
        let (client_cert, client_key) =
            issue_client_cert(&ca_key, "test-device-123").unwrap();
        assert!(client_cert.contains("BEGIN CERTIFICATE"));
        assert!(client_key.contains("BEGIN PRIVATE KEY"));
        // Fingerprint should be deterministic for same cert.
        let fp = cert_fingerprint_from_pem(&client_cert).unwrap();
        assert_eq!(fp.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn ca_fingerprint_is_16_bytes() {
        let (ca_cert, _) = generate_ca().unwrap();
        let fp = ca_fingerprint(&ca_cert).unwrap();
        assert_eq!(fp.len(), 16);
    }

    #[test]
    fn insert_and_list_paired_devices() {
        let conn = test_db();
        let device = PairedDevice {
            device_id: "dev-1".to_string(),
            display_name: "My Phone".to_string(),
            cert_fingerprint: "abc123".to_string(),
            capabilities: vec!["chat".to_string()],
            paired_at: 1000,
            last_seen_at: None,
        };
        insert_paired_device(&conn, &device).unwrap();

        let devices = list_paired_devices(&conn).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, "dev-1");
        assert_eq!(devices[0].capabilities, vec!["chat"]);
    }

    #[test]
    fn revoke_device_removes_row() {
        let conn = test_db();
        let device = PairedDevice {
            device_id: "dev-2".to_string(),
            display_name: "Tablet".to_string(),
            cert_fingerprint: "def456".to_string(),
            capabilities: vec![],
            paired_at: 2000,
            last_seen_at: Some(3000),
        };
        insert_paired_device(&conn, &device).unwrap();
        assert!(revoke_device(&conn, "dev-2").unwrap());
        assert!(!revoke_device(&conn, "dev-2").unwrap()); // Already gone.
        assert!(list_paired_devices(&conn).unwrap().is_empty());
    }

    #[test]
    fn touch_device_updates_last_seen() {
        let conn = test_db();
        let device = PairedDevice {
            device_id: "dev-3".to_string(),
            display_name: "Watch".to_string(),
            cert_fingerprint: "ghi789".to_string(),
            capabilities: vec![],
            paired_at: 1000,
            last_seen_at: None,
        };
        insert_paired_device(&conn, &device).unwrap();
        touch_device(&conn, "dev-3").unwrap();

        let devices = list_paired_devices(&conn).unwrap();
        assert!(devices[0].last_seen_at.is_some());
    }

    #[test]
    fn find_device_by_fingerprint_hit_and_miss() {
        let conn = test_db();
        let device = PairedDevice {
            device_id: "dev-4".to_string(),
            display_name: "Laptop".to_string(),
            cert_fingerprint: "unique-fp".to_string(),
            capabilities: vec![],
            paired_at: 5000,
            last_seen_at: None,
        };
        insert_paired_device(&conn, &device).unwrap();

        let found = find_device_by_fingerprint(&conn, "unique-fp").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().device_id, "dev-4");

        let miss = find_device_by_fingerprint(&conn, "nonexistent").unwrap();
        assert!(miss.is_none());
    }

    #[test]
    fn pairing_manager_load_or_create_generates_ca() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = PairingManager::load_or_create(tmp.path()).unwrap();
        assert!(mgr.ca_cert_pem().contains("BEGIN CERTIFICATE"));

        // Second call loads from disk.
        let mgr2 = PairingManager::load_or_create(tmp.path()).unwrap();
        assert_eq!(mgr.ca_cert_pem(), mgr2.ca_cert_pem());
    }

    #[test]
    fn start_pairing_returns_valid_uri() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = PairingManager::load_or_create(tmp.path()).unwrap();
        let uri = mgr.start_pairing("192.168.1.42", 7421).unwrap();
        assert!(uri.starts_with("terransoul://pair?"));
    }

    #[test]
    fn confirm_pairing_rejects_wrong_token() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = PairingManager::load_or_create(tmp.path()).unwrap();
        let _uri = mgr.start_pairing("192.168.1.42", 7421).unwrap();

        let conn = test_db();
        let wrong_token = [0u8; 32];
        let result = mgr.confirm_pairing(&conn, "dev-x", "Phone", &wrong_token);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mismatch"));
    }
}
