//! Disk-backed ANN migration helpers (Phase 3 kickoff + execution, Chunk 49.2).
//!
//! Phase 49.1 shipped only deterministic planning (`DiskAnnPlan`).
//! Phase 49.2 adds the first executable migration path:
//! - write/read per-shard IVF-PQ sidecar metadata,
//! - report migration attempts/results,
//! - expose health/eligibility surfaces for maintenance and MCP health.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default shard-size threshold for disk-backed ANN planning.
/// The threshold is intentionally conservative for kickoff; operators can
/// override it from call sites once benchmark data is collected.
pub const DEFAULT_DISK_ANN_ENTRY_THRESHOLD: usize = 5_000_000;

/// Maximum candidate shards migrated per maintenance run.
pub const DEFAULT_DISK_ANN_MAX_SHARDS_PER_RUN: usize = 2;

/// Sidecar schema version for per-shard IVF-PQ metadata.
pub const DISK_ANN_SIDECAR_VERSION: u32 = 1;

/// Sidecar filename suffix for shard migration metadata.
pub const DISK_ANN_SIDECAR_SUFFIX: &str = ".ivfpq.json";

/// Default IVF-PQ parameters (planning metadata only in Phase 49.2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IvfPqParams {
    pub nlist: usize,
    pub pq_m: usize,
    pub pq_nbits: usize,
}

impl Default for IvfPqParams {
    fn default() -> Self {
        Self {
            nlist: 4096,
            pq_m: 96,
            pq_nbits: 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskAnnSidecar {
    pub version: u32,
    pub shard: String,
    pub entry_count: usize,
    pub threshold: usize,
    pub source_ann_index: String,
    pub generated_at: i64,
    pub ivf_pq: IvfPqParams,
    /// `planned` means sidecar metadata exists and shard is eligible for
    /// a future IVF-PQ build job.
    pub status: String,
}

impl DiskAnnSidecar {
    pub fn new(
        shard: String,
        entry_count: usize,
        threshold: usize,
        source_ann_index: String,
    ) -> Self {
        Self {
            version: DISK_ANN_SIDECAR_VERSION,
            shard,
            entry_count,
            threshold,
            source_ann_index,
            generated_at: now_ms(),
            ivf_pq: IvfPqParams::default(),
            status: "planned".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskAnnMigrationItem {
    pub shard: String,
    pub migrated: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskAnnMigrationReport {
    pub threshold: usize,
    pub max_shards: usize,
    pub attempted: usize,
    pub migrated: usize,
    pub skipped_missing_ann: usize,
    pub sidecars_written: usize,
    pub items: Vec<DiskAnnMigrationItem>,
}

impl DiskAnnMigrationReport {
    pub fn empty(threshold: usize, max_shards: usize) -> Self {
        Self {
            threshold,
            max_shards,
            attempted: 0,
            migrated: 0,
            skipped_missing_ann: 0,
            sidecars_written: 0,
            items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskAnnHealthSummary {
    pub threshold: usize,
    pub eligible_candidates: usize,
    pub sidecars_total: usize,
    pub ready_candidates: usize,
    pub missing_sidecars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskAnnShardPlan {
    pub shard: String,
    pub entry_count: usize,
    pub ann_index_exists: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskAnnPlan {
    pub threshold: usize,
    pub candidate_count: usize,
    pub candidates: Vec<DiskAnnShardPlan>,
}

impl DiskAnnPlan {
    pub fn empty(threshold: usize) -> Self {
        Self {
            threshold,
            candidate_count: 0,
            candidates: Vec::new(),
        }
    }
}

pub fn sidecar_file_name(shard: &str) -> String {
    format!("{shard}{DISK_ANN_SIDECAR_SUFFIX}")
}

pub fn sidecar_path(vectors_dir: &Path, shard: &str) -> PathBuf {
    vectors_dir.join(sidecar_file_name(shard))
}

pub fn write_sidecar(vectors_dir: &Path, sidecar: &DiskAnnSidecar) -> Result<(), String> {
    fs::create_dir_all(vectors_dir).map_err(|e| {
        format!(
            "failed to create vectors dir {}: {e}",
            vectors_dir.display()
        )
    })?;
    let path = sidecar_path(vectors_dir, &sidecar.shard);
    let encoded = serde_json::to_string_pretty(sidecar)
        .map_err(|e| format!("failed to serialize sidecar: {e}"))?;
    fs::write(&path, encoded)
        .map_err(|e| format!("failed to write sidecar {}: {e}", path.display()))
}

pub fn read_sidecar(vectors_dir: &Path, shard: &str) -> Result<Option<DiskAnnSidecar>, String> {
    let path = sidecar_path(vectors_dir, shard);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read sidecar {}: {e}", path.display()))?;
    let sidecar: DiskAnnSidecar = serde_json::from_str(&raw)
        .map_err(|e| format!("failed to decode sidecar {}: {e}", path.display()))?;
    Ok(Some(sidecar))
}

pub fn list_sidecars(vectors_dir: &Path) -> Result<Vec<DiskAnnSidecar>, String> {
    if !vectors_dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(vectors_dir)
        .map_err(|e| format!("failed to scan vectors dir {}: {e}", vectors_dir.display()))?
    {
        let entry = entry.map_err(|e| format!("failed to read vectors dir entry: {e}"))?;
        let path = entry.path();
        let is_sidecar = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(DISK_ANN_SIDECAR_SUFFIX))
            .unwrap_or(false);
        if !is_sidecar {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read sidecar {}: {e}", path.display()))?;
        let sidecar: DiskAnnSidecar = serde_json::from_str(&raw)
            .map_err(|e| format!("failed to decode sidecar {}: {e}", path.display()))?;
        out.push(sidecar);
    }
    out.sort_by(|a, b| {
        b.entry_count
            .cmp(&a.entry_count)
            .then_with(|| a.shard.cmp(&b.shard))
    });
    Ok(out)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Pure helper so planner behavior can be tested without DB setup.
pub fn plan_from_counts(
    threshold: usize,
    mut shard_rows: Vec<(String, usize, bool)>,
) -> DiskAnnPlan {
    if threshold == 0 {
        return DiskAnnPlan::empty(threshold);
    }

    // Deterministic ordering for stable snapshots/logs.
    shard_rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut candidates = Vec::new();
    for (shard, entry_count, ann_index_exists) in shard_rows {
        if entry_count < threshold {
            continue;
        }
        let reason = if ann_index_exists {
            format!(
                "entry_count {} >= threshold {}; candidate for IVF-PQ migration",
                entry_count, threshold
            )
        } else {
            format!(
                "entry_count {} >= threshold {} but ANN index file missing; rebuild before migration",
                entry_count, threshold
            )
        };
        candidates.push(DiskAnnShardPlan {
            shard,
            entry_count,
            ann_index_exists,
            reason,
        });
    }

    DiskAnnPlan {
        threshold,
        candidate_count: candidates.len(),
        candidates,
    }
}

/// Build an IVF-PQ index for a shard that has a `planned` sidecar.
///
/// This is the Phase 3 execution step: consumes the sidecar metadata,
/// reads all embeddings for the shard, trains coarse + PQ codebooks,
/// encodes all vectors, and writes the binary index file.
///
/// Returns build statistics on success and updates the sidecar status to `"built"`.
pub fn build_ivf_pq_for_shard(
    vectors_dir: &Path,
    shard: &str,
    embeddings: Vec<(i64, Vec<f32>)>,
    dim: usize,
) -> Result<super::ivf_pq::IvfPqBuildStats, String> {
    // Read sidecar to get params
    let sidecar = read_sidecar(vectors_dir, shard)?
        .ok_or_else(|| format!("No sidecar found for shard {shard}"))?;

    if sidecar.status != "planned" && sidecar.status != "stale" {
        return Err(format!(
            "Sidecar for {shard} has status '{}'; expected 'planned' or 'stale'",
            sidecar.status
        ));
    }

    if embeddings.is_empty() {
        return Err(format!("No embeddings provided for shard {shard}"));
    }

    // Build the IVF-PQ index
    let (index, stats) =
        super::ivf_pq::IvfPqIndex::build(&sidecar.ivf_pq, dim, embeddings)?;

    // Save the index to disk
    let index_path = super::ivf_pq::IvfPqIndex::index_path(vectors_dir, shard);
    index.save_to_file(&index_path)?;

    // Update sidecar status to "built"
    let updated_sidecar = DiskAnnSidecar {
        status: "built".to_string(),
        generated_at: now_ms(),
        entry_count: stats.total_vectors,
        ..sidecar
    };
    write_sidecar(vectors_dir, &updated_sidecar)?;

    Ok(stats)
}

/// Check if an IVF-PQ index is available for a shard.
pub fn ivf_pq_index_exists(vectors_dir: &Path, shard: &str) -> bool {
    super::ivf_pq::IvfPqIndex::exists(vectors_dir, shard)
}

/// Load an IVF-PQ index for a shard. Returns None if it doesn't exist.
pub fn load_ivf_pq_index(
    vectors_dir: &Path,
    shard: &str,
) -> Result<Option<super::ivf_pq::IvfPqIndex>, String> {
    let path = super::ivf_pq::IvfPqIndex::index_path(vectors_dir, shard);
    if !path.exists() {
        return Ok(None);
    }
    let index = super::ivf_pq::IvfPqIndex::load_from_file(&path)?;
    Ok(Some(index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn plan_from_counts_selects_only_shards_above_threshold() {
        let plan = plan_from_counts(
            100,
            vec![
                ("long__semantic".into(), 250, true),
                ("long__episodic".into(), 99, true),
                ("working__semantic".into(), 100, false),
            ],
        );

        assert_eq!(plan.candidate_count, 2);
        assert_eq!(plan.candidates[0].shard, "long__semantic");
        assert_eq!(plan.candidates[1].shard, "working__semantic");
        assert!(plan.candidates[1].reason.contains("missing"));
    }

    #[test]
    fn plan_from_counts_zero_threshold_returns_empty_plan() {
        let plan = plan_from_counts(0, vec![("long__semantic".into(), 10_000, true)]);
        assert_eq!(plan.candidate_count, 0);
        assert!(plan.candidates.is_empty());
    }

    #[test]
    fn sidecar_roundtrip_write_read() {
        let dir = std::env::temp_dir().join("ts_disk_ann_sidecar_roundtrip");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let sidecar = DiskAnnSidecar::new(
            "long__semantic".to_string(),
            42,
            10,
            "vectors/long__semantic.usearch".to_string(),
        );
        write_sidecar(&dir, &sidecar).unwrap();

        let loaded = read_sidecar(&dir, "long__semantic").unwrap().unwrap();
        assert_eq!(loaded.shard, "long__semantic");
        assert_eq!(loaded.entry_count, 42);
        assert_eq!(loaded.status, "planned");
        assert_eq!(loaded.version, DISK_ANN_SIDECAR_VERSION);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_sidecars_ignores_non_sidecar_files() {
        let dir = std::env::temp_dir().join("ts_disk_ann_sidecar_list");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let sidecar_a = DiskAnnSidecar::new(
            "long__semantic".to_string(),
            50,
            10,
            "vectors/long__semantic.usearch".to_string(),
        );
        let sidecar_b = DiskAnnSidecar::new(
            "long__procedural".to_string(),
            80,
            10,
            "vectors/long__procedural.usearch".to_string(),
        );
        write_sidecar(&dir, &sidecar_a).unwrap();
        write_sidecar(&dir, &sidecar_b).unwrap();
        fs::write(dir.join("not-a-sidecar.txt"), "ignore").unwrap();

        let all = list_sidecars(&dir).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].shard, "long__procedural");
        assert_eq!(all[1].shard, "long__semantic");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_ivf_pq_for_shard_creates_index_and_updates_status() {
        let dir = std::env::temp_dir().join("ts_ivfpq_build_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Write a planned sidecar
        let sidecar = DiskAnnSidecar {
            version: DISK_ANN_SIDECAR_VERSION,
            shard: "long__semantic".to_string(),
            entry_count: 200,
            threshold: 100,
            source_ann_index: "long__semantic.usearch".to_string(),
            generated_at: now_ms(),
            ivf_pq: IvfPqParams {
                nlist: 4,
                pq_m: 4,
                pq_nbits: 8,
            },
            status: "planned".to_string(),
        };
        write_sidecar(&dir, &sidecar).unwrap();

        // Generate test embeddings (dim=16, pq_m=4 → subspace_dim=4)
        let dim = 16;
        let mut embeddings = Vec::new();
        for i in 0..200 {
            let v: Vec<f32> = (0..dim)
                .map(|d| ((i * 7 + d * 13) as f32 * 0.01).sin())
                .collect();
            embeddings.push((i as i64, v));
        }

        // Build the index
        let stats = build_ivf_pq_for_shard(&dir, "long__semantic", embeddings, dim).unwrap();
        assert_eq!(stats.total_vectors, 200);
        assert_eq!(stats.nlist, 4);
        assert_eq!(stats.pq_m, 4);

        // Verify index file was created
        assert!(ivf_pq_index_exists(&dir, "long__semantic"));

        // Verify sidecar was updated
        let updated = read_sidecar(&dir, "long__semantic").unwrap().unwrap();
        assert_eq!(updated.status, "built");
        assert_eq!(updated.entry_count, 200);

        // Verify index can be loaded and searched
        let index = load_ivf_pq_index(&dir, "long__semantic").unwrap().unwrap();
        assert_eq!(index.len(), 200);

        let query: Vec<f32> = (0..dim)
            .map(|d| ((d * 13) as f32 * 0.01).sin())
            .collect();
        let results = index.search(&query, 5, 2);
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.id == 0));

        let _ = fs::remove_dir_all(&dir);
    }
}
