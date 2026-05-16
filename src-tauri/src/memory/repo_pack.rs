//! Per-source repo-knowledge pack/unpack (export & import).
//!
//! A `.tsbrain` file is a single shareable archive of one `memory_sources`
//! row plus its per-source `memories.db` (the `repo_chunks` + `repo_files`
//! SQLite database written by [`crate::memory::repo_ingest`]). Users can
//! export a fully-ingested repo brain, ship the file to a teammate, and
//! the teammate can import it without re-cloning or re-embedding.
//!
//! ## Pack format (v1, self-describing, no compression deps)
//!
//! ```text
//! [ 8 bytes ] MAGIC = b"TSBRAIN1"
//! [ 4 bytes ] u32 LE manifest length
//! [ N bytes ] JSON manifest (PackManifest)
//! [ 8 bytes ] u64 LE database length
//! [ M bytes ] raw SQLite file (snapshot via VACUUM INTO)
//! ```
//!
//! `VACUUM INTO` produces a clean, WAL-flushed copy so a concurrent writer
//! cannot tear the snapshot. The pack carries no checkout bytes — the
//! receiver can re-clone the source by `repo_url` if they want fresh files,
//! but the embedded chunks (which are what the brain searches over) are
//! already self-sufficient.
//!
//! ## Import modes
//!
//! * [`ImportMode::Overwrite`] — replace the target's `memories.db` outright
//!   with the imported bytes. Any existing rows for the target source_id
//!   are dropped. The receiver's `memory_sources` row is upserted.
//! * [`ImportMode::Merge`] — open the imported DB read-only and insert
//!   `repo_chunks` + `repo_files` rows that are not already present.
//!   Dedup keys: `(source_id, content_hash, file_path)` for chunks and
//!   `(source_id, file_path)` for files. Source-id collisions are resolved
//!   by rewriting incoming rows to the target's source_id.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::memory::repo_ingest::{db_path, repo_root, validate_source_id};
use crate::memory::sources::{
    self, get_source, MemorySourceKind, SELF_SOURCE_ID,
};

/// Magic header. Eight bytes so the file is trivially identifiable.
pub const MAGIC: &[u8; 8] = b"TSBRAIN1";

/// Bumped when the on-disk layout changes incompatibly. Importers refuse
/// any version they don't recognise.
pub const PACK_FORMAT_VERSION: u32 = 1;

/// JSON manifest stored at the head of every `.tsbrain` archive.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PackManifest {
    pub format_version: u32,
    pub source_id: String,
    pub label: String,
    pub kind: String,
    pub repo_url: Option<String>,
    pub repo_ref: Option<String>,
    pub created_at: i64,
    pub last_synced_at: Option<i64>,
    /// When the pack itself was written (ms since epoch).
    pub exported_at: i64,
    /// Rough payload accounting; not authoritative (the embedded DB is).
    pub chunk_count: u64,
    pub file_count: u64,
    /// SHA-256 of the embedded SQLite bytes — receivers can verify
    /// integrity before swapping it in.
    pub db_sha256: String,
    /// Free-form: source app version string for forensic diagnostics.
    pub exporter_version: String,
}

/// How the importer should reconcile incoming rows with anything already
/// present under the same source id.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    /// Insert only rows whose `(source_id, content_hash)` (chunks) or
    /// `(source_id, file_path)` (files) is not already present.
    Merge,
    /// Replace the target source's `memories.db` outright.
    Overwrite,
}

/// Return value of [`export_source`].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportSummary {
    pub source_id: String,
    pub output_path: String,
    pub manifest: PackManifest,
    pub bytes_written: u64,
}

/// Return value of [`import_source`].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportSummary {
    pub source_id: String,
    pub label: String,
    pub mode: ImportMode,
    pub chunks_inserted: u64,
    pub chunks_skipped: u64,
    pub files_inserted: u64,
    pub files_skipped: u64,
    pub repo_url: Option<String>,
}

#[derive(Debug, Error)]
pub enum PackError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown memory source: {0}")]
    UnknownSource(String),
    #[error("source {0:?} has no ingested data at {1}")]
    MissingDb(String, PathBuf),
    #[error("invalid pack: {0}")]
    InvalidPack(String),
    #[error("unsupported pack format version {found} (expected {expected})")]
    UnsupportedVersion { found: u32, expected: u32 },
    #[error("integrity check failed: db hash mismatch")]
    HashMismatch,
    #[error("invalid source id: {0}")]
    InvalidSourceId(String),
    #[error("the built-in 'self' source cannot be exported or overwritten")]
    SelfSourceForbidden,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn count_chunks(conn: &Connection, source_id: &str) -> rusqlite::Result<u64> {
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM repo_chunks WHERE source_id = ?1",
            params![source_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(n as u64)
}

fn count_files(conn: &Connection, source_id: &str) -> rusqlite::Result<u64> {
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM repo_files WHERE source_id = ?1",
            params![source_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(n as u64)
}

/// Snapshot the per-source SQLite to a temporary file via `VACUUM INTO`,
/// then return its raw bytes. Caller is responsible for deleting the temp.
fn snapshot_db_bytes(db_file: &Path) -> Result<(Vec<u8>, PathBuf), PackError> {
    let temp = db_file.with_extension("snapshot.tmp.db");
    let _ = fs::remove_file(&temp);
    let conn = Connection::open(db_file)?;
    let escaped = temp.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{}';", escaped))?;
    drop(conn);
    let bytes = fs::read(&temp)?;
    Ok((bytes, temp))
}

/// Build a `.tsbrain` pack for `source_id` and write it to `output_path`.
///
/// The `'self'` source is rejected — it represents the user's private
/// personal brain, not a shareable repo knowledge bundle.
pub fn export_source(
    conn_for_registry: &Connection,
    data_dir: &Path,
    source_id: &str,
    output_path: &Path,
    exporter_version: &str,
) -> Result<ExportSummary, PackError> {
    if source_id == SELF_SOURCE_ID {
        return Err(PackError::SelfSourceForbidden);
    }
    validate_source_id(source_id)
        .map_err(|e| PackError::InvalidSourceId(format!("{source_id}: {e}")))?;
    let source = get_source(conn_for_registry, source_id)?
        .ok_or_else(|| PackError::UnknownSource(source_id.to_string()))?;

    let db_file = db_path(data_dir, source_id);
    if !db_file.exists() {
        return Err(PackError::MissingDb(source_id.to_string(), db_file));
    }

    let (db_bytes, temp_db) = snapshot_db_bytes(&db_file)?;
    let cleanup = scopeguard_remove(temp_db);

    // Count from the snapshot itself so the manifest matches what we ship.
    let (chunk_count, file_count) = {
        let snap_path = cleanup.path();
        if snap_path.exists() {
            let c = Connection::open(snap_path)?;
            (
                count_chunks(&c, source_id).unwrap_or(0),
                count_files(&c, source_id).unwrap_or(0),
            )
        } else {
            (0, 0)
        }
    };

    let manifest = PackManifest {
        format_version: PACK_FORMAT_VERSION,
        source_id: source.id.clone(),
        label: source.label.clone(),
        kind: kind_to_string(source.kind),
        repo_url: source.repo_url.clone(),
        repo_ref: source.repo_ref.clone(),
        created_at: source.created_at,
        last_synced_at: source.last_synced_at,
        exported_at: now_ms(),
        chunk_count,
        file_count,
        db_sha256: sha256_hex(&db_bytes),
        exporter_version: exporter_version.to_string(),
    };

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut out = fs::File::create(output_path)?;
    out.write_all(MAGIC)?;

    let manifest_bytes = serde_json::to_vec(&manifest)?;
    let mlen = u32::try_from(manifest_bytes.len())
        .map_err(|_| PackError::InvalidPack("manifest too large".into()))?;
    out.write_all(&mlen.to_le_bytes())?;
    out.write_all(&manifest_bytes)?;

    let dlen = db_bytes.len() as u64;
    out.write_all(&dlen.to_le_bytes())?;
    out.write_all(&db_bytes)?;
    out.flush()?;
    let bytes_written = out.metadata()?.len();

    Ok(ExportSummary {
        source_id: source.id,
        output_path: output_path.to_string_lossy().to_string(),
        manifest,
        bytes_written,
    })
}

/// Read a `.tsbrain` file from disk and return its parsed manifest + the
/// embedded SQLite bytes.
pub fn read_pack(archive_path: &Path) -> Result<(PackManifest, Vec<u8>), PackError> {
    let mut f = fs::File::open(archive_path)?;
    let mut magic = [0u8; 8];
    f.read_exact(&mut magic)
        .map_err(|_| PackError::InvalidPack("missing magic header".into()))?;
    if &magic != MAGIC {
        return Err(PackError::InvalidPack(format!(
            "bad magic: expected TSBRAIN1, got {magic:?}"
        )));
    }
    let mut mlen_buf = [0u8; 4];
    f.read_exact(&mut mlen_buf)
        .map_err(|_| PackError::InvalidPack("truncated manifest length".into()))?;
    let mlen = u32::from_le_bytes(mlen_buf) as usize;
    if mlen == 0 || mlen > 16 * 1024 * 1024 {
        return Err(PackError::InvalidPack(format!(
            "implausible manifest length: {mlen}"
        )));
    }
    let mut manifest_bytes = vec![0u8; mlen];
    f.read_exact(&mut manifest_bytes)
        .map_err(|_| PackError::InvalidPack("truncated manifest".into()))?;
    let manifest: PackManifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest.format_version != PACK_FORMAT_VERSION {
        return Err(PackError::UnsupportedVersion {
            found: manifest.format_version,
            expected: PACK_FORMAT_VERSION,
        });
    }
    let mut dlen_buf = [0u8; 8];
    f.read_exact(&mut dlen_buf)
        .map_err(|_| PackError::InvalidPack("truncated db length".into()))?;
    let dlen = u64::from_le_bytes(dlen_buf) as usize;
    let mut db_bytes = vec![0u8; dlen];
    f.read_exact(&mut db_bytes)
        .map_err(|_| PackError::InvalidPack("truncated db body".into()))?;
    if sha256_hex(&db_bytes) != manifest.db_sha256 {
        return Err(PackError::HashMismatch);
    }
    Ok((manifest, db_bytes))
}

/// Import a `.tsbrain` archive into `data_dir`.
///
/// * `mode` controls how rows are reconciled (see [`ImportMode`]).
/// * `target_source_id` lets the caller pick a local id different from
///   the one baked into the pack (useful when the user already has a
///   source with the same id and wants to keep both). If `None`, the
///   manifest's `source_id` is used verbatim.
/// * `label_override` similarly lets the receiver pick a fresh label.
///
/// The `'self'` source can never be the import target.
pub fn import_source(
    conn_for_registry: &Connection,
    data_dir: &Path,
    archive_path: &Path,
    mode: ImportMode,
    target_source_id: Option<&str>,
    label_override: Option<&str>,
) -> Result<ImportSummary, PackError> {
    let (manifest, db_bytes) = read_pack(archive_path)?;
    let target_id = target_source_id.unwrap_or(&manifest.source_id).to_string();
    if target_id == SELF_SOURCE_ID {
        return Err(PackError::SelfSourceForbidden);
    }
    validate_source_id(&target_id)
        .map_err(|e| PackError::InvalidSourceId(format!("{target_id}: {e}")))?;
    let label = label_override.unwrap_or(&manifest.label).to_string();

    let target_db = db_path(data_dir, &target_id);
    if let Some(parent) = target_db.parent() {
        fs::create_dir_all(parent)?;
    }

    // Always materialise the incoming bytes to a side file so we can open
    // it with rusqlite and inspect / copy rows uniformly.
    let staged = repo_root(data_dir, &target_id).join("imported.staging.db");
    if let Some(parent) = staged.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&staged, &db_bytes)?;
    let stage_cleanup = scopeguard_remove(staged.clone());

    let (chunks_inserted, chunks_skipped, files_inserted, files_skipped) = match mode {
        ImportMode::Overwrite => {
            // Drop any existing target DB, then copy bytes into place. We
            // also need to rewrite source_id rows if it differs from the
            // pack's embedded source_id, otherwise hybrid_search would miss
            // them under the local id.
            if target_db.exists() {
                let _ = fs::remove_file(&target_db);
            }
            fs::copy(&staged, &target_db)?;
            if target_id != manifest.source_id {
                let c = Connection::open(&target_db)?;
                c.execute(
                    "UPDATE repo_chunks SET source_id = ?1 WHERE source_id = ?2",
                    params![target_id, manifest.source_id],
                )?;
                c.execute(
                    "UPDATE repo_files SET source_id = ?1 WHERE source_id = ?2",
                    params![target_id, manifest.source_id],
                )?;
            }
            let c = Connection::open(&target_db)?;
            let chunks = count_chunks(&c, &target_id).unwrap_or(0);
            let files = count_files(&c, &target_id).unwrap_or(0);
            (chunks, 0u64, files, 0u64)
        }
        ImportMode::Merge => merge_rows(&staged, &target_db, &manifest.source_id, &target_id)?,
    };

    // Upsert the registry row. We do not blindly overwrite the original
    // `created_at` if a row is already present; we just refresh the
    // mutable bits.
    let existing = get_source(conn_for_registry, &target_id)?;
    match existing {
        Some(_) => {
            conn_for_registry.execute(
                "UPDATE memory_sources
                    SET label = ?1, repo_url = ?2, repo_ref = ?3, last_synced_at = ?4
                  WHERE id = ?5",
                params![
                    label,
                    manifest.repo_url,
                    manifest.repo_ref,
                    now_ms(),
                    target_id
                ],
            )?;
        }
        None => {
            let kind = match manifest.kind.as_str() {
                "repo" => MemorySourceKind::Repo,
                "topic" => MemorySourceKind::Topic,
                other => {
                    return Err(PackError::InvalidPack(format!(
                        "unsupported source kind {other:?}"
                    )));
                }
            };
            sources::create_source(
                conn_for_registry,
                &target_id,
                kind,
                &label,
                manifest.repo_url.as_deref(),
                manifest.repo_ref.as_deref(),
            )?;
            // create_source leaves last_synced_at NULL; stamp it now so
            // the UI reflects the import time.
            sources::touch_synced(conn_for_registry, &target_id)?;
        }
    }

    drop(stage_cleanup);

    Ok(ImportSummary {
        source_id: target_id,
        label,
        mode,
        chunks_inserted,
        chunks_skipped,
        files_inserted,
        files_skipped,
        repo_url: manifest.repo_url,
    })
}

/// Merge `repo_chunks` + `repo_files` from `staged` into `target_db`,
/// rewriting `source_id` to `target_source_id` on the way in.
fn merge_rows(
    staged: &Path,
    target_db: &Path,
    pack_source_id: &str,
    target_source_id: &str,
) -> Result<(u64, u64, u64, u64), PackError> {
    // Ensure the target DB exists with the right schema. RepoStore::open
    // creates the file + schema if missing.
    let _ = crate::memory::repo_ingest::RepoStore::open(target_db, target_source_id)
        .map_err(|e| PackError::InvalidPack(format!("open target db: {e}")))?;
    let mut tgt = Connection::open(target_db)?;
    let src = Connection::open(staged)?;

    let tx = tgt.transaction()?;

    let mut chunks_inserted = 0u64;
    let mut chunks_skipped = 0u64;
    {
        let mut select = src.prepare(
            "SELECT file_path, parent_symbol, kind, byte_start, byte_end,
                    content, content_hash, embedding, created_at
               FROM repo_chunks
              WHERE source_id = ?1",
        )?;
        let mut insert = tx.prepare(
            "INSERT INTO repo_chunks (
                source_id, file_path, parent_symbol, kind,
                byte_start, byte_end, content, content_hash,
                embedding, created_at
             ) SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10
               WHERE NOT EXISTS (
                 SELECT 1 FROM repo_chunks
                  WHERE source_id = ?1 AND content_hash = ?8 AND file_path = ?2
               )",
        )?;
        let mut rows = select.query(params![pack_source_id])?;
        while let Some(r) = rows.next()? {
            let file_path: String = r.get(0)?;
            let parent_symbol: Option<String> = r.get(1)?;
            let kind: String = r.get(2)?;
            let byte_start: i64 = r.get(3)?;
            let byte_end: i64 = r.get(4)?;
            let content: String = r.get(5)?;
            let content_hash: String = r.get(6)?;
            let embedding: Option<Vec<u8>> = r.get(7)?;
            let created_at: i64 = r.get(8)?;
            let n = insert.execute(params![
                target_source_id,
                file_path,
                parent_symbol,
                kind,
                byte_start,
                byte_end,
                content,
                content_hash,
                embedding,
                created_at,
            ])?;
            if n > 0 {
                chunks_inserted += 1;
            } else {
                chunks_skipped += 1;
            }
        }
    }

    let mut files_inserted = 0u64;
    let mut files_skipped = 0u64;
    {
        let mut select = src.prepare(
            "SELECT file_path, file_hash, last_synced_at, chunk_count
               FROM repo_files WHERE source_id = ?1",
        )?;
        let mut insert = tx.prepare(
            "INSERT INTO repo_files (source_id, file_path, file_hash, last_synced_at, chunk_count)
             SELECT ?1, ?2, ?3, ?4, ?5
              WHERE NOT EXISTS (
                SELECT 1 FROM repo_files WHERE source_id = ?1 AND file_path = ?2
              )",
        )?;
        let mut rows = select.query(params![pack_source_id])?;
        while let Some(r) = rows.next()? {
            let file_path: String = r.get(0)?;
            let file_hash: String = r.get(1)?;
            let last_synced_at: i64 = r.get(2)?;
            let chunk_count: i64 = r.get(3)?;
            let n = insert.execute(params![
                target_source_id,
                file_path,
                file_hash,
                last_synced_at,
                chunk_count,
            ])?;
            if n > 0 {
                files_inserted += 1;
            } else {
                files_skipped += 1;
            }
        }
    }

    tx.commit()?;
    Ok((chunks_inserted, chunks_skipped, files_inserted, files_skipped))
}

fn kind_to_string(k: MemorySourceKind) -> String {
    match k {
        MemorySourceKind::SelfBrain => "self".into(),
        MemorySourceKind::Repo => "repo".into(),
        MemorySourceKind::Topic => "topic".into(),
    }
}

// ── Tiny RAII cleanup helper so we don't pull in scopeguard for one call ────

struct DeferRemove(PathBuf);
impl DeferRemove {
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for DeferRemove {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn scopeguard_remove(path: PathBuf) -> DeferRemove {
    DeferRemove(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::repo_ingest::RepoStore;
    use crate::memory::schema::create_canonical_schema;
    use rusqlite::Connection;

    fn fresh_registry() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_canonical_schema(&c).unwrap();
        c
    }

    fn seed_repo_store(data_dir: &Path, source_id: &str, n: usize) {
        use crate::memory::repo_ingest::{db_path, RepoChunk};
        let mut store = RepoStore::open(&db_path(data_dir, source_id), source_id).unwrap();
        let chunks: Vec<RepoChunk> = (0..n)
            .map(|i| RepoChunk {
                file_path: format!("src/lib_{}.rs", i % 3),
                parent_symbol: Some(format!("fn_{i}")),
                kind: crate::memory::repo_ingest::ChunkKind::Code,
                byte_start: 0,
                byte_end: 10,
                content: format!("chunk content {i}"),
                content_hash: format!("hash-{i:08x}"),
            })
            .collect();
        store.insert_chunks(&chunks).unwrap();
        for i in 0..3 {
            store
                .upsert_file_hash(&format!("src/lib_{i}.rs"), &format!("fhash-{i}"), 1)
                .unwrap();
        }
    }

    fn make_source(reg: &Connection, id: &str, label: &str) {
        sources::create_source(
            reg,
            id,
            MemorySourceKind::Repo,
            label,
            Some("https://github.com/example/demo"),
            Some("main"),
        )
        .unwrap();
    }

    #[test]
    fn round_trip_overwrite_preserves_counts() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path();
        let reg = fresh_registry();
        make_source(&reg, "src1", "Source One");
        seed_repo_store(data, "src1", 5);

        let pack_path = tmp.path().join("out.tsbrain");
        let summary =
            export_source(&reg, data, "src1", &pack_path, "test-0.0.0").expect("export");
        assert_eq!(summary.manifest.chunk_count, 5);
        assert_eq!(summary.manifest.file_count, 3);
        assert!(pack_path.exists());

        // Fresh data dir, fresh registry — simulate the recipient side.
        let tmp2 = tempfile::tempdir().unwrap();
        let reg2 = fresh_registry();
        let import_summary = import_source(
            &reg2,
            tmp2.path(),
            &pack_path,
            ImportMode::Overwrite,
            None,
            None,
        )
        .expect("import");
        assert_eq!(import_summary.chunks_inserted, 5);
        assert_eq!(import_summary.files_inserted, 3);

        // Registry row must now exist.
        let row = sources::get_source(&reg2, "src1").unwrap().expect("row");
        assert_eq!(row.label, "Source One");
        assert!(row.last_synced_at.is_some());
    }

    #[test]
    fn merge_dedupes_by_content_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path();
        let reg = fresh_registry();
        make_source(&reg, "src1", "Source One");
        seed_repo_store(data, "src1", 5);

        let pack_path = tmp.path().join("out.tsbrain");
        export_source(&reg, data, "src1", &pack_path, "test-0.0.0").unwrap();

        let tmp2 = tempfile::tempdir().unwrap();
        let reg2 = fresh_registry();
        // First import → all rows inserted.
        let s1 = import_source(
            &reg2,
            tmp2.path(),
            &pack_path,
            ImportMode::Merge,
            None,
            None,
        )
        .unwrap();
        assert_eq!(s1.chunks_inserted, 5);
        assert_eq!(s1.chunks_skipped, 0);
        // Second import of the same pack → all rows deduped.
        let s2 = import_source(
            &reg2,
            tmp2.path(),
            &pack_path,
            ImportMode::Merge,
            None,
            None,
        )
        .unwrap();
        assert_eq!(s2.chunks_inserted, 0);
        assert_eq!(s2.chunks_skipped, 5);
        assert_eq!(s2.files_skipped, 3);
    }

    #[test]
    fn import_can_rename_source_id() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path();
        let reg = fresh_registry();
        make_source(&reg, "src1", "Source One");
        seed_repo_store(data, "src1", 3);

        let pack_path = tmp.path().join("out.tsbrain");
        export_source(&reg, data, "src1", &pack_path, "test-0.0.0").unwrap();

        let tmp2 = tempfile::tempdir().unwrap();
        let reg2 = fresh_registry();
        let s = import_source(
            &reg2,
            tmp2.path(),
            &pack_path,
            ImportMode::Overwrite,
            Some("renamed-id"),
            Some("Renamed"),
        )
        .unwrap();
        assert_eq!(s.source_id, "renamed-id");
        assert_eq!(s.label, "Renamed");

        // Chunks must now be queryable under the renamed id.
        let target_db = db_path(tmp2.path(), "renamed-id");
        let c = Connection::open(&target_db).unwrap();
        let n = count_chunks(&c, "renamed-id").unwrap();
        assert_eq!(n, 3);
        // And nothing under the original id.
        let zero = count_chunks(&c, "src1").unwrap();
        assert_eq!(zero, 0);
    }

    #[test]
    fn export_rejects_self_source() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = fresh_registry();
        let err = export_source(
            &reg,
            tmp.path(),
            SELF_SOURCE_ID,
            &tmp.path().join("never.tsbrain"),
            "t",
        )
        .unwrap_err();
        matches!(err, PackError::SelfSourceForbidden);
    }

    #[test]
    fn read_pack_rejects_bad_magic() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("broken.tsbrain");
        fs::write(&f, b"NOPE----nothing here at all").unwrap();
        let err = read_pack(&f).unwrap_err();
        matches!(err, PackError::InvalidPack(_));
    }
}
