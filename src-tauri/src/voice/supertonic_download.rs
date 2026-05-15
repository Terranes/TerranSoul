//! SHA-256-verified streaming download of the Supertonic model assets.
//!
//! The flow:
//! 1. Iterate every manifest entry in order.
//! 2. Stream the bytes from the pinned Hugging Face URL into the install
//!    directory, hashing as we go so we can write the SHA-256 to the
//!    lockfile without re-reading the (large) files.
//! 3. After every file succeeds, persist `manifest.lock` so subsequent
//!    integrity checks (offline) can verify the install.
//!
//! The function is decoupled from Tauri's `AppHandle` so it stays unit-
//! testable: progress is reported through a `ProgressSink` closure. The
//! Tauri command in `commands::voice` adapts the sink to event emission.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::supertonic_manifest::{
    download_url, files, total_bytes, LockedFile, ManifestFile, ManifestLock, MODEL_REVISION,
};
use super::supertonic_paths::{ensure_install_dir, lockfile_path};

/// Progress event emitted as a download proceeds. The Tauri command adapter
/// serialises this payload onto the `supertonic-download-progress` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// Manifest-relative path of the file currently being fetched.
    pub current_file: String,
    /// Bytes written for the current file so far.
    pub current_bytes: u64,
    /// Total expected bytes for the current file.
    pub current_total: u64,
    /// Bytes written across all files so far.
    pub overall_bytes: u64,
    /// Total expected bytes across the full manifest.
    pub overall_total: u64,
    /// Index of the current file (0-based) within the manifest order.
    pub file_index: usize,
    /// Total number of files in the manifest.
    pub file_count: usize,
}

/// Callback receiving progress updates. Implemented as `Fn` (not `FnMut`) so
/// it can be cloned across the async download loop. The Tauri adapter wraps
/// an `AppHandle` and emits events.
pub type ProgressSink = dyn Fn(DownloadProgress) + Send + Sync;

/// Errors that can occur during a Supertonic download.
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("network error fetching {file}: {source}")]
    Http {
        file: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("HTTP {status} fetching {file}")]
    Status { file: String, status: u16 },
    #[error("size mismatch for {file}: expected ~{expected} bytes, got {actual}")]
    SizeMismatch {
        file: String,
        expected: u64,
        actual: u64,
    },
    #[error("io error writing {file}: {source}")]
    Io {
        file: String,
        #[source]
        source: io::Error,
    },
    #[error("lockfile serialisation failed: {0}")]
    Lock(#[from] serde_json::Error),
}

/// Stream every manifest entry into `install_dir` and write `manifest.lock`
/// at the end. Existing files that already match the manifest size are
/// skipped (resume support). Partial files are truncated and refetched.
pub async fn download_all(
    install_dir: &Path,
    client: &reqwest::Client,
    progress: &ProgressSink,
) -> Result<ManifestLock, DownloadError> {
    ensure_install_dir(install_dir).map_err(|e| DownloadError::Io {
        file: install_dir.display().to_string(),
        source: e,
    })?;

    let manifest = files();
    let manifest_total = total_bytes();
    let file_count = manifest.len();

    let mut overall_bytes = 0u64;
    let mut locked = Vec::with_capacity(file_count);

    for (idx, entry) in manifest.iter().enumerate() {
        let dest = install_dir.join(entry.rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| DownloadError::Io {
                file: entry.rel_path.to_string(),
                source: e,
            })?;
        }

        // Resume support: if the file already exists at roughly the right
        // size, hash it and add to the lockfile without redownloading.
        if let Ok(meta) = fs::metadata(&dest) {
            let tol = super::supertonic_manifest::size_tolerance_bytes(entry.size_bytes);
            if meta.len() >= entry.size_bytes.saturating_sub(tol)
                && meta.len() <= entry.size_bytes.saturating_add(tol)
            {
                let bytes = fs::read(&dest).map_err(|e| DownloadError::Io {
                    file: entry.rel_path.to_string(),
                    source: e,
                })?;
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                locked.push(LockedFile {
                    rel_path: entry.rel_path.to_string(),
                    size_bytes: bytes.len() as u64,
                    sha256_hex: hex::encode(hasher.finalize()),
                });
                overall_bytes = overall_bytes.saturating_add(bytes.len() as u64);
                progress(DownloadProgress {
                    current_file: entry.rel_path.to_string(),
                    current_bytes: bytes.len() as u64,
                    current_total: entry.size_bytes,
                    overall_bytes,
                    overall_total: manifest_total,
                    file_index: idx,
                    file_count,
                });
                continue;
            }
        }

        let written = stream_one(entry, &dest, client, idx, file_count, manifest_total, overall_bytes, progress).await?;
        overall_bytes = overall_bytes.saturating_add(written.size_bytes);
        locked.push(written);
    }

    let lock = ManifestLock {
        revision: MODEL_REVISION.to_string(),
        files: locked,
    };
    persist_lock(install_dir, &lock)?;
    Ok(lock)
}

async fn stream_one(
    entry: &ManifestFile,
    dest: &Path,
    client: &reqwest::Client,
    idx: usize,
    file_count: usize,
    manifest_total: u64,
    overall_before: u64,
    progress: &ProgressSink,
) -> Result<LockedFile, DownloadError> {
    let url = download_url(entry.rel_path);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| DownloadError::Http {
            file: entry.rel_path.to_string(),
            source: e,
        })?;
    let status = resp.status();
    if !status.is_success() {
        return Err(DownloadError::Status {
            file: entry.rel_path.to_string(),
            status: status.as_u16(),
        });
    }

    let mut file = File::create(dest).map_err(|e| DownloadError::Io {
        file: entry.rel_path.to_string(),
        source: e,
    })?;
    let mut hasher = Sha256::new();
    let mut written: u64 = 0;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| DownloadError::Http {
            file: entry.rel_path.to_string(),
            source: e,
        })?;
        hasher.update(&chunk);
        file.write_all(&chunk).map_err(|e| DownloadError::Io {
            file: entry.rel_path.to_string(),
            source: e,
        })?;
        written = written.saturating_add(chunk.len() as u64);
        progress(DownloadProgress {
            current_file: entry.rel_path.to_string(),
            current_bytes: written,
            current_total: entry.size_bytes,
            overall_bytes: overall_before.saturating_add(written),
            overall_total: manifest_total,
            file_index: idx,
            file_count,
        });
    }
    file.flush().map_err(|e| DownloadError::Io {
        file: entry.rel_path.to_string(),
        source: e,
    })?;

    let tol = super::supertonic_manifest::size_tolerance_bytes(entry.size_bytes);
    if written < entry.size_bytes.saturating_sub(tol)
        || written > entry.size_bytes.saturating_add(tol)
    {
        // Truncate so a retry won't see "looks complete".
        let _ = fs::remove_file(dest);
        return Err(DownloadError::SizeMismatch {
            file: entry.rel_path.to_string(),
            expected: entry.size_bytes,
            actual: written,
        });
    }

    Ok(LockedFile {
        rel_path: entry.rel_path.to_string(),
        size_bytes: written,
        sha256_hex: hex::encode(hasher.finalize()),
    })
}

fn persist_lock(install_dir: &Path, lock: &ManifestLock) -> Result<(), DownloadError> {
    let path = lockfile_path(install_dir);
    let json = serde_json::to_vec_pretty(lock)?;
    fs::write(&path, json).map_err(|e| DownloadError::Io {
        file: path.display().to_string(),
        source: e,
    })
}

/// Load `manifest.lock` from an install directory, if present.
pub fn read_lock(install_dir: &Path) -> io::Result<Option<ManifestLock>> {
    let path = lockfile_path(install_dir);
    match fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<ManifestLock>(&bytes) {
            Ok(lock) => Ok(Some(lock)),
            Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Hash the bytes of a single file on disk using SHA-256. Returns a
/// lower-case hex digest. Streaming so it doesn't allocate the whole file.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

/// Verify every file in `install_dir` against the persisted lockfile. Returns
/// the list of files that fail verification (empty when everything matches).
pub fn verify_against_lock(install_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let Some(lock) = read_lock(install_dir)? else {
        // No lockfile → can't verify; treat as failure of every required file.
        return Ok(files()
            .iter()
            .map(|f| install_dir.join(f.rel_path))
            .collect());
    };
    let mut failures = Vec::new();
    for entry in &lock.files {
        let path = install_dir.join(&entry.rel_path);
        match sha256_file(&path) {
            Ok(actual) if actual == entry.sha256_hex => {}
            _ => failures.push(path),
        }
    }
    Ok(failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn sha256_file_hashes_known_input() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("blob");
        fs::write(&path, b"hello world").unwrap();
        let hex = sha256_file(&path).unwrap();
        // SHA-256("hello world") — well-known constant.
        assert_eq!(
            hex,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn read_lock_returns_none_when_absent() {
        let tmp = tempdir().unwrap();
        assert!(read_lock(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn read_lock_roundtrips_a_persisted_lock() {
        let tmp = tempdir().unwrap();
        let lock = ManifestLock {
            revision: MODEL_REVISION.to_string(),
            files: vec![LockedFile {
                rel_path: "onnx/tts.json".to_string(),
                size_bytes: 1234,
                sha256_hex: "deadbeef".to_string(),
            }],
        };
        persist_lock(tmp.path(), &lock).unwrap();
        let loaded = read_lock(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded, lock);
    }

    #[test]
    fn verify_against_lock_reports_missing_files_when_lock_absent() {
        let tmp = tempdir().unwrap();
        let failures = verify_against_lock(tmp.path()).unwrap();
        assert_eq!(failures.len(), files().len());
    }

    #[test]
    fn verify_against_lock_passes_when_hashes_match() {
        let tmp = tempdir().unwrap();
        // Stage a tiny lock with a single file whose hash we precompute.
        let file_path = tmp.path().join("blob");
        fs::write(&file_path, b"hello world").unwrap();
        let expected_hash =
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string();
        let lock = ManifestLock {
            revision: MODEL_REVISION.to_string(),
            files: vec![LockedFile {
                rel_path: "blob".to_string(),
                size_bytes: 11,
                sha256_hex: expected_hash,
            }],
        };
        persist_lock(tmp.path(), &lock).unwrap();
        let failures = verify_against_lock(tmp.path()).unwrap();
        assert!(failures.is_empty(), "unexpected failures: {failures:?}");
    }

    #[test]
    fn verify_against_lock_reports_tampered_file() {
        let tmp = tempdir().unwrap();
        let file_path = tmp.path().join("blob");
        fs::write(&file_path, b"hello world").unwrap();
        let lock = ManifestLock {
            revision: MODEL_REVISION.to_string(),
            files: vec![LockedFile {
                rel_path: "blob".to_string(),
                size_bytes: 11,
                sha256_hex: "0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            }],
        };
        persist_lock(tmp.path(), &lock).unwrap();
        let failures = verify_against_lock(tmp.path()).unwrap();
        assert_eq!(failures, vec![file_path]);
    }
}
