//! Pinned manifest for the Supertonic ONNX TTS model assets.
//!
//! The model assets live in the Hugging Face repository
//! [`Supertone/supertonic-2`](https://huggingface.co/Supertone/supertonic-2).
//! We pin against a specific commit revision so future Hub mutations cannot
//! change the bytes we verify on disk. Downloads stream from the
//! [`/resolve/<commit>/<path>`](https://huggingface.co/docs/hub/en/api#resolve)
//! URL pattern, which serves the LFS-backed binary bytes.
//!
//! Total on-disk footprint: ~268 MB across 16 files.
//!
//! License: model weights are distributed under the **OpenRAIL-M v1** license
//! (see `docs/licensing-audit.md` 🟡 Conditional clearance section and the
//! upstream model card). Use must propagate the use-based restrictions to
//! end users — TerranSoul does this via the first-run consent dialog
//! introduced in stage 1c.

use serde::{Deserialize, Serialize};

/// Pinned Hugging Face commit revision for the model assets. Changing this
/// value invalidates every previously-downloaded `manifest.lock` and forces
/// re-download. Bump only when intentionally adopting a new model snapshot.
pub const MODEL_REVISION: &str = "75e6727618a02f323c720cba9478152d4bc16ca4";

/// Hugging Face repo identifier (`<owner>/<repo>`) the manifest pins.
pub const MODEL_REPO: &str = "Supertone/supertonic-2";

/// Sub-directory under `app_data_dir/voice/` where the install lands.
/// Versioned (`v2`) so future model upgrades can live side-by-side without
/// nuking an in-progress download.
pub const INSTALL_SUBDIR: &str = "supertonic/v2";

/// Description of a single file in the pinned model manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFile {
    /// Path relative to the install root (and to the HF repo root). Always
    /// uses forward slashes to match the upstream layout.
    pub rel_path: &'static str,
    /// Expected byte length. Used as a fast pre-flight integrity check and
    /// to drive the progress UI before download starts.
    pub size_bytes: u64,
    /// Whether this entry is required for synthesis to succeed. All entries
    /// are required in v2 — kept as a struct field so future variants
    /// (e.g. optional slim-vocoder shards) can be modelled without breaking
    /// the manifest shape.
    pub required: bool,
}

/// Voice preset identifiers exposed by the v2 model. The Rust API consumes
/// one or more `<voice>.json` files from `voice_styles/` and concatenates
/// their style vectors. TerranSoul ships the per-voice JSON files but
/// defaults to a single voice at synthesis time (see `SupertonicTts`).
pub const VOICES: &[&str] = &["F1", "F2", "F3", "F4", "F5", "M1", "M2", "M3", "M4", "M5"];

/// Return the pinned file manifest. Sizes match the bytes served by Hugging
/// Face at `MODEL_REVISION`; cross-checked against the public file listing.
pub fn files() -> Vec<ManifestFile> {
    vec![
        // ONNX network weights (pipeline stages).
        ManifestFile { rel_path: "onnx/duration_predictor.onnx", size_bytes: 1_520_000, required: true },
        ManifestFile { rel_path: "onnx/text_encoder.onnx",       size_bytes: 27_400_000, required: true },
        ManifestFile { rel_path: "onnx/vector_estimator.onnx",   size_bytes: 132_000_000, required: true },
        ManifestFile { rel_path: "onnx/vocoder.onnx",            size_bytes: 101_000_000, required: true },
        // Pipeline configuration.
        ManifestFile { rel_path: "onnx/tts.json",                size_bytes: 8_700,       required: true },
        ManifestFile { rel_path: "onnx/unicode_indexer.json",    size_bytes: 262_000,     required: true },
        // Voice style vectors.
        ManifestFile { rel_path: "voice_styles/F1.json", size_bytes: 420_000, required: true },
        ManifestFile { rel_path: "voice_styles/F2.json", size_bytes: 420_000, required: true },
        ManifestFile { rel_path: "voice_styles/F3.json", size_bytes: 420_000, required: true },
        ManifestFile { rel_path: "voice_styles/F4.json", size_bytes: 420_000, required: true },
        ManifestFile { rel_path: "voice_styles/F5.json", size_bytes: 420_000, required: true },
        ManifestFile { rel_path: "voice_styles/M1.json", size_bytes: 421_000, required: true },
        ManifestFile { rel_path: "voice_styles/M2.json", size_bytes: 420_000, required: true },
        ManifestFile { rel_path: "voice_styles/M3.json", size_bytes: 420_000, required: true },
        ManifestFile { rel_path: "voice_styles/M4.json", size_bytes: 420_000, required: true },
        ManifestFile { rel_path: "voice_styles/M5.json", size_bytes: 420_000, required: true },
    ]
}

/// File-size tolerance (bytes) when checking on-disk artifacts against the
/// manifest. Hugging Face occasionally re-encodes LFS pointers with marginal
/// padding differences; the byte sizes published in the file listing are
/// rounded to 3 significant figures (e.g. "132 MB"). The tolerance avoids
/// spurious re-downloads, but is tight enough that a truncated or corrupt
/// file still trips verification.
pub const SIZE_TOLERANCE_PERCENT: u64 = 5;

/// Compute the absolute tolerance window (in bytes) around a manifest size.
pub fn size_tolerance_bytes(expected: u64) -> u64 {
    expected.saturating_mul(SIZE_TOLERANCE_PERCENT) / 100
}

/// Sum of all manifest file sizes — useful for showing a "downloading XXX MB"
/// figure before any byte hits the wire.
pub fn total_bytes() -> u64 {
    files().iter().map(|f| f.size_bytes).sum()
}

/// Compose the Hugging Face download URL for a manifest entry pinned at
/// `MODEL_REVISION`. Public-repo URLs require no auth.
pub fn download_url(rel_path: &str) -> String {
    format!(
        "https://huggingface.co/{repo}/resolve/{rev}/{path}",
        repo = MODEL_REPO,
        rev = MODEL_REVISION,
        path = rel_path,
    )
}

/// JSON payload persisted at `<install_dir>/manifest.lock` after a successful
/// download. Subsequent integrity checks compare on-disk SHA-256 against the
/// lockfile so the user can verify nothing has been tampered with offline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestLock {
    /// Pinned upstream commit revision the lock was produced against.
    pub revision: String,
    /// One entry per downloaded file.
    pub files: Vec<LockedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedFile {
    pub rel_path: String,
    pub size_bytes: u64,
    /// Lower-case hex SHA-256 of the file bytes as written to disk.
    pub sha256_hex: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_has_sixteen_files() {
        let m = files();
        assert_eq!(m.len(), 16, "expected 4 onnx + 2 json + 10 voice styles");
        assert!(m.iter().all(|f| f.required), "v2 manifest has no optional files");
    }

    #[test]
    fn manifest_paths_use_forward_slashes() {
        for f in files() {
            assert!(
                !f.rel_path.contains('\\'),
                "rel_path must use forward slashes: {}",
                f.rel_path
            );
        }
    }

    #[test]
    fn manifest_total_size_is_roughly_268_mb() {
        let total = total_bytes();
        // Upstream HF listing reports ~268 MB; permit a generous window.
        assert!(total > 200_000_000, "total too small: {total}");
        assert!(total < 350_000_000, "total too large: {total}");
    }

    #[test]
    fn download_url_pins_revision() {
        let url = download_url("onnx/tts.json");
        assert!(url.contains(MODEL_REVISION), "url should embed pinned commit");
        assert!(url.starts_with("https://huggingface.co/Supertone/supertonic-2/"));
        assert!(url.ends_with("/onnx/tts.json"));
    }

    #[test]
    fn voices_list_is_ten_presets() {
        assert_eq!(VOICES.len(), 10);
        assert!(VOICES.contains(&"F1"));
        assert!(VOICES.contains(&"M5"));
    }

    #[test]
    fn size_tolerance_scales_with_size() {
        let t_small = size_tolerance_bytes(1_000_000);
        let t_large = size_tolerance_bytes(132_000_000);
        assert!(t_large > t_small);
        assert_eq!(t_small, 50_000); // 5% of 1 MB
    }
}
