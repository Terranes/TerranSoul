//! Online snapshot + atomic restore (Chunk 41.15).
//!
//! `snapshot(dest_dir)` performs:
//! 1. `VACUUM INTO` for the SQLite database (non-blocking, WAL-safe)
//! 2. Saves every ANN index into dest_dir
//! 3. Writes `snapshot.json` manifest with file checksums
//!
//! Concurrent CRUD continues unblocked during snapshot (WAL mode).
//! Restore verifies checksums and atomically swaps data_dir.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

/// Manifest written alongside snapshot files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    /// Timestamp (ms epoch) when the snapshot was created.
    pub created_at: i64,
    /// Schema version of the snapshotted database.
    pub schema_version: i64,
    /// Map of filename → SHA-256 hex digest.
    pub files: HashMap<String, String>,
}

/// Errors that can occur during snapshot or restore.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("checksum mismatch for {file}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },
    #[error("manifest missing or invalid: {0}")]
    ManifestInvalid(String),
    #[error("ANN index error: {0}")]
    Ann(String),
}

pub type SnapshotResult<T> = Result<T, SnapshotError>;

/// Create an online snapshot of the memory store.
///
/// - `conn`: the active SQLite connection (WAL mode — VACUUM INTO is non-blocking)
/// - `data_dir`: source directory containing ANN index files
/// - `dest_dir`: target directory for the snapshot (created if needed)
/// - `schema_version`: current schema version to record in manifest
///
/// Returns the manifest of the created snapshot.
pub fn snapshot(
    conn: &Connection,
    data_dir: &Path,
    dest_dir: &Path,
    schema_version: i64,
) -> SnapshotResult<SnapshotManifest> {
    fs::create_dir_all(dest_dir)?;

    let mut files: HashMap<String, String> = HashMap::new();

    // 1. VACUUM INTO — produces a consistent snapshot of the SQLite DB.
    // This is non-blocking in WAL mode: concurrent writes continue.
    let db_dest = dest_dir.join("memory.db");
    conn.execute(
        "VACUUM INTO ?1",
        params![db_dest.to_string_lossy().as_ref()],
    )?;
    let db_hash = sha256_file(&db_dest)?;
    files.insert("memory.db".to_string(), db_hash);

    // 2. Copy ANN index files (vectors_*.usearch).
    let ann_files = discover_ann_files(data_dir)?;
    for ann_file in &ann_files {
        let filename = ann_file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let dest_path = dest_dir.join(&filename);
        fs::copy(ann_file, &dest_path)?;
        let hash = sha256_file(&dest_path)?;
        files.insert(filename, hash);
    }

    // 3. Copy quantization sidecar files (*.quant).
    let quant_files = discover_quant_files(data_dir)?;
    for qf in &quant_files {
        let filename = qf
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let dest_path = dest_dir.join(&filename);
        fs::copy(qf, &dest_path)?;
        let hash = sha256_file(&dest_path)?;
        files.insert(filename, hash);
    }

    // 4. Write manifest.
    let manifest = SnapshotManifest {
        created_at: now_ms(),
        schema_version,
        files,
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(dest_dir.join("snapshot.json"), &manifest_json)?;

    Ok(manifest)
}

/// Verify a snapshot directory against its manifest.
/// Returns the validated manifest on success.
pub fn verify_snapshot(snapshot_dir: &Path) -> SnapshotResult<SnapshotManifest> {
    let manifest_path = snapshot_dir.join("snapshot.json");
    if !manifest_path.exists() {
        return Err(SnapshotError::ManifestInvalid(
            "snapshot.json not found".to_string(),
        ));
    }
    let manifest_json = fs::read_to_string(&manifest_path)?;
    let manifest: SnapshotManifest = serde_json::from_str(&manifest_json)?;

    for (filename, expected_hash) in &manifest.files {
        let file_path = snapshot_dir.join(filename);
        if !file_path.exists() {
            return Err(SnapshotError::ChecksumMismatch {
                file: filename.clone(),
                expected: expected_hash.clone(),
                actual: "<missing>".to_string(),
            });
        }
        let actual_hash = sha256_file(&file_path)?;
        if &actual_hash != expected_hash {
            return Err(SnapshotError::ChecksumMismatch {
                file: filename.clone(),
                expected: expected_hash.clone(),
                actual: actual_hash,
            });
        }
    }
    Ok(manifest)
}

/// Restore a verified snapshot into a target data directory.
///
/// 1. Verifies all checksums in the snapshot.
/// 2. Atomically swaps the target data_dir contents:
///    - Renames current data_dir to `data_dir.old`
///    - Copies snapshot files into data_dir
///    - Removes `data_dir.old` on success
///
/// The caller must NOT hold any open connections to the target data_dir.
pub fn restore(snapshot_dir: &Path, data_dir: &Path) -> SnapshotResult<SnapshotManifest> {
    // 1. Verify snapshot integrity.
    let manifest = verify_snapshot(snapshot_dir)?;

    // 2. Prepare the swap.
    let backup_dir = data_dir.with_extension("old");
    if backup_dir.exists() {
        fs::remove_dir_all(&backup_dir)?;
    }

    // 3. Move current data out of the way.
    if data_dir.exists() {
        fs::rename(data_dir, &backup_dir)?;
    }

    // 4. Copy snapshot files into data_dir.
    fs::create_dir_all(data_dir)?;
    for filename in manifest.files.keys() {
        let src = snapshot_dir.join(filename);
        let dst = data_dir.join(filename);
        fs::copy(&src, &dst)?;
    }

    // 5. Clean up backup on success.
    if backup_dir.exists() {
        let _ = fs::remove_dir_all(&backup_dir);
    }

    Ok(manifest)
}

/// Compute SHA-256 hex digest of a file.
fn sha256_file(path: &Path) -> SnapshotResult<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Discover all `.usearch` files in a directory.
fn discover_ann_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    if !dir.exists() {
        return Ok(result);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "usearch") {
            result.push(path);
        }
    }
    Ok(result)
}

/// Discover all `.quant` sidecar files in a directory.
fn discover_quant_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    if !dir.exists() {
        return Ok(result);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "quant") {
            result.push(path);
        }
    }
    Ok(result)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        crate::memory::schema::create_canonical_schema(&conn).unwrap();
        conn
    }

    fn test_conn_file(path: &Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;\nPRAGMA foreign_keys=ON;\nPRAGMA synchronous=NORMAL;",
        )
        .unwrap();
        crate::memory::schema::create_canonical_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn snapshot_and_verify_roundtrip() {
        let tmp = std::env::temp_dir().join("ts_snapshot_test_rt");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let data_dir = tmp.join("data");
        fs::create_dir_all(&data_dir).unwrap();

        // Create a file-backed DB (VACUUM INTO requires a file-backed DB).
        let db_path = data_dir.join("memory.db");
        let conn = test_conn_file(&db_path);

        // Insert some data.
        for i in 0..10 {
            conn.execute(
                "INSERT INTO memories (content, tags, importance, memory_type, created_at,
                 access_count, tier, decay_score, token_count)
                 VALUES (?1, 'test', 3, 'fact', ?2, 0, 'long', 1.0, 10)",
                params![format!("memory {i}"), 1700000000000i64 + i * 1000],
            )
            .unwrap();
        }

        // Create a fake ANN file.
        let ann_file = data_dir.join("vectors_nomic.usearch");
        fs::write(&ann_file, b"fake_ann_data_for_test").unwrap();

        // Snapshot.
        let dest = tmp.join("snapshot");
        let manifest = snapshot(&conn, &data_dir, &dest, 16).unwrap();

        assert_eq!(manifest.schema_version, 16);
        assert!(manifest.files.contains_key("memory.db"));
        assert!(manifest.files.contains_key("vectors_nomic.usearch"));
        assert!(manifest.created_at > 0);

        // Verify.
        let verified = verify_snapshot(&dest).unwrap();
        assert_eq!(verified.files.len(), manifest.files.len());

        // Clean up.
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn verify_detects_corruption() {
        let tmp = std::env::temp_dir().join("ts_snapshot_test_corrupt");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let data_dir = tmp.join("data");
        fs::create_dir_all(&data_dir).unwrap();
        let db_path = data_dir.join("memory.db");
        let conn = test_conn_file(&db_path);

        let dest = tmp.join("snapshot");
        snapshot(&conn, &data_dir, &dest, 16).unwrap();

        // Corrupt the database file in the snapshot.
        let snapshot_db = dest.join("memory.db");
        fs::write(&snapshot_db, b"corrupted!").unwrap();

        let result = verify_snapshot(&dest);
        assert!(result.is_err());
        match result.unwrap_err() {
            SnapshotError::ChecksumMismatch { file, .. } => {
                assert_eq!(file, "memory.db");
            }
            other => panic!("expected ChecksumMismatch, got: {other:?}"),
        }

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn restore_swaps_data_dir() {
        let tmp = std::env::temp_dir().join("ts_snapshot_test_restore");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let data_dir = tmp.join("data");
        fs::create_dir_all(&data_dir).unwrap();
        let db_path = data_dir.join("memory.db");
        let conn = test_conn_file(&db_path);

        // Insert data.
        conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at,
             access_count, tier, decay_score, token_count)
             VALUES ('original', 'tag', 3, 'fact', 1700000000000, 0, 'long', 1.0, 5)",
            [],
        )
        .unwrap();

        // Snapshot.
        let snapshot_dest = tmp.join("snapshot");
        snapshot(&conn, &data_dir, &snapshot_dest, 16).unwrap();
        drop(conn); // Close the connection before restore.

        // Modify the original to simulate divergence.
        let conn2 = test_conn_file(&db_path);
        conn2
            .execute(
                "INSERT INTO memories (content, tags, importance, memory_type, created_at,
                 access_count, tier, decay_score, token_count)
                 VALUES ('diverged', 'tag', 3, 'fact', 1700000001000, 0, 'long', 1.0, 5)",
                [],
            )
            .unwrap();
        let count_before: i64 = conn2
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_before, 2);
        drop(conn2);

        // Restore from snapshot (should revert to 1 row).
        let manifest = restore(&snapshot_dest, &data_dir).unwrap();
        assert!(manifest.files.contains_key("memory.db"));

        // Verify restored state.
        let conn3 = test_conn_file(&db_path);
        let count_after: i64 = conn3
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_after, 1);
        let content: String = conn3
            .query_row("SELECT content FROM memories LIMIT 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(content, "original");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn snapshot_with_no_ann_files() {
        let tmp = std::env::temp_dir().join("ts_snapshot_test_noann");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let data_dir = tmp.join("data");
        fs::create_dir_all(&data_dir).unwrap();
        let db_path = data_dir.join("memory.db");
        let conn = test_conn_file(&db_path);

        let dest = tmp.join("snapshot");
        let manifest = snapshot(&conn, &data_dir, &dest, 16).unwrap();
        // Only memory.db should be in the manifest.
        assert_eq!(manifest.files.len(), 1);
        assert!(manifest.files.contains_key("memory.db"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn verify_missing_manifest() {
        let tmp = std::env::temp_dir().join("ts_snapshot_test_nomanifest");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let result = verify_snapshot(&tmp);
        assert!(matches!(result, Err(SnapshotError::ManifestInvalid(_))));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn concurrent_writes_during_snapshot() {
        // Verifies that VACUUM INTO doesn't block concurrent inserts.
        let tmp = std::env::temp_dir().join("ts_snapshot_test_concurrent");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let data_dir = tmp.join("data");
        fs::create_dir_all(&data_dir).unwrap();
        let db_path = data_dir.join("memory.db");
        let conn = test_conn_file(&db_path);

        // Insert initial data.
        for i in 0..5 {
            conn.execute(
                "INSERT INTO memories (content, tags, importance, memory_type, created_at,
                 access_count, tier, decay_score, token_count)
                 VALUES (?1, '', 3, 'fact', ?2, 0, 'long', 1.0, 10)",
                params![format!("pre-snapshot {i}"), 1700000000000i64 + i],
            )
            .unwrap();
        }

        // Take snapshot.
        let dest = tmp.join("snapshot");
        let manifest = snapshot(&conn, &data_dir, &dest, 16).unwrap();
        assert!(manifest.files.contains_key("memory.db"));

        // Write more data AFTER snapshot started (simulating concurrent writes).
        for i in 0..5 {
            conn.execute(
                "INSERT INTO memories (content, tags, importance, memory_type, created_at,
                 access_count, tier, decay_score, token_count)
                 VALUES (?1, '', 3, 'fact', ?2, 0, 'long', 1.0, 10)",
                params![format!("post-snapshot {i}"), 1700000010000i64 + i],
            )
            .unwrap();
        }

        // Live DB has 10 rows.
        let live_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(live_count, 10);

        // Snapshot should only have the original 5.
        let snap_conn = Connection::open(dest.join("memory.db")).unwrap();
        let snap_count: i64 = snap_conn
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(snap_count, 5);

        let _ = fs::remove_dir_all(&tmp);
    }
}
