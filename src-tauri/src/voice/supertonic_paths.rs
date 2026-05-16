//! Install-path and disk-state helpers for the Supertonic on-device TTS
//! model. Always compiled — the heavy `ort` integration in `supertonic_tts`
//! is feature-gated behind `tts-supertonic`, but install/remove/health-check
//! must remain available to surface the download UX on every desktop build.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::supertonic_manifest::{
    files, size_tolerance_bytes, INSTALL_SUBDIR, MODEL_REVISION,
};

/// Filename of the lockfile written next to the downloaded assets.
pub const MANIFEST_LOCK_FILENAME: &str = "manifest.lock";

/// Resolve the on-disk install directory rooted at `app_data_dir`.
/// Layout: `<app_data>/voice/supertonic/v2/`.
pub fn install_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("voice").join(INSTALL_SUBDIR)
}

/// Resolve the manifest-lock path inside an install directory.
pub fn lockfile_path(install_dir: &Path) -> PathBuf {
    install_dir.join(MANIFEST_LOCK_FILENAME)
}

/// Check whether every required manifest file is present on disk inside
/// `install_dir` with a size within tolerance of the pinned expectation.
///
/// This does **not** verify SHA-256 — that check lives in
/// `supertonic_download::verify_against_lock` and runs after first install.
/// `is_installed` exists so the UI can show "Active" without rehashing
/// hundreds of MB on every settings open.
pub fn is_installed(install_dir: &Path) -> bool {
    if !install_dir.is_dir() {
        return false;
    }
    for entry in files() {
        let path = install_dir.join(entry.rel_path);
        let Ok(meta) = fs::metadata(&path) else {
            return false;
        };
        if !meta.is_file() {
            return false;
        }
        let actual = meta.len();
        let tol = size_tolerance_bytes(entry.size_bytes);
        let min = entry.size_bytes.saturating_sub(tol);
        let max = entry.size_bytes.saturating_add(tol);
        if actual < min || actual > max {
            return false;
        }
    }
    true
}

/// Remove the entire install directory (and any partial-download cruft). No-op
/// if the directory does not exist.
pub fn remove(install_dir: &Path) -> io::Result<()> {
    match fs::remove_dir_all(install_dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Ensure the install directory (and any parents) exists. Idempotent.
pub fn ensure_install_dir(install_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(install_dir)
}

/// Snapshot of the install state for UI display. Cheap to compute — only
/// stats files, never opens them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallStatus {
    pub install_dir: PathBuf,
    pub installed: bool,
    pub revision: &'static str,
    pub manifest_lock_present: bool,
}

pub fn status(app_data_dir: &Path) -> InstallStatus {
    let dir = install_dir(app_data_dir);
    let installed = is_installed(&dir);
    let manifest_lock_present = lockfile_path(&dir).is_file();
    InstallStatus {
        install_dir: dir,
        installed,
        revision: MODEL_REVISION,
        manifest_lock_present,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn install_dir_nests_under_voice_supertonic_v2() {
        let tmp = tempdir().unwrap();
        let dir = install_dir(tmp.path());
        let suffix = dir.strip_prefix(tmp.path()).unwrap();
        // Should be voice/supertonic/v2 (platform separators normalised).
        let s = suffix.to_string_lossy().replace('\\', "/");
        assert_eq!(s, "voice/supertonic/v2");
    }

    #[test]
    fn is_installed_false_when_dir_missing() {
        let tmp = tempdir().unwrap();
        let dir = install_dir(tmp.path());
        assert!(!is_installed(&dir));
    }

    #[test]
    fn is_installed_false_when_files_missing() {
        let tmp = tempdir().unwrap();
        let dir = install_dir(tmp.path());
        ensure_install_dir(&dir).unwrap();
        assert!(!is_installed(&dir));
    }

    #[test]
    fn is_installed_true_with_all_files_at_expected_size() {
        let tmp = tempdir().unwrap();
        let dir = install_dir(tmp.path());
        ensure_install_dir(&dir).unwrap();
        for entry in files() {
            let path = dir.join(entry.rel_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            // Write a file of exactly the expected size — cheap because the
            // largest entry is ~132 MB and the tolerance accepts any sane
            // value within ±5%. Use a single zero byte then truncate to the
            // target size to avoid heap-allocating 132 MB.
            let f = fs::File::create(&path).unwrap();
            f.set_len(entry.size_bytes).unwrap();
        }
        assert!(is_installed(&dir));
    }

    #[test]
    fn is_installed_false_when_one_file_truncated() {
        let tmp = tempdir().unwrap();
        let dir = install_dir(tmp.path());
        ensure_install_dir(&dir).unwrap();
        for entry in files() {
            let path = dir.join(entry.rel_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let f = fs::File::create(&path).unwrap();
            // Truncate the vocoder severely.
            let len = if entry.rel_path == "onnx/vocoder.onnx" {
                1024
            } else {
                entry.size_bytes
            };
            f.set_len(len).unwrap();
        }
        assert!(!is_installed(&dir));
    }

    #[test]
    fn remove_is_idempotent_on_missing_dir() {
        let tmp = tempdir().unwrap();
        let dir = install_dir(tmp.path());
        // Not yet created.
        remove(&dir).unwrap();
        // And again.
        remove(&dir).unwrap();
    }

    #[test]
    fn remove_clears_an_existing_install() {
        let tmp = tempdir().unwrap();
        let dir = install_dir(tmp.path());
        ensure_install_dir(&dir).unwrap();
        fs::write(dir.join("dummy"), b"x").unwrap();
        assert!(dir.exists());
        remove(&dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn status_reports_revision() {
        let tmp = tempdir().unwrap();
        let s = status(tmp.path());
        assert_eq!(s.revision, MODEL_REVISION);
        assert!(!s.installed);
        assert!(!s.manifest_lock_present);
    }
}
