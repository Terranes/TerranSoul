//! BRAIN-REPO-RAG-1b-i — per-repo ingest backend (foundation slice).
//!
//! Pipeline (synchronous, single-threaded for now):
//!   1. `shallow_clone()` — anonymous `gix` clone with depth=1 into
//!      `<data_dir>/repos/<source_id>/checkout/`.
//!   2. `walk_files()` — `ignore::WalkBuilder` with precedence
//!      user-includes → repo `.gitignore` / `.terransoulignore` → user-excludes
//!      → defaults; enforces a configurable per-file byte cap (default 10 MiB).
//!   3. `is_likely_secret()` — regex sweep (PEM blocks, AWS keys, GitHub PATs,
//!      generic `api_key=` lines); flagged files are skipped, not chunked.
//!   4. `chunk_text()` — reuses `memory::chunking::split_markdown` for prose
//!      chunking with byte spans. AST chunking via tree-sitter is deferred to
//!      1b-ii (see `rules/milestones.md`).
//!   5. `RepoStore` — per-repo SQLite at `<data_dir>/repos/<source_id>/memories.db`
//!      with a single `repo_chunks` table. Embeddings (BLOB) are persisted as
//!      NULL in 1b-i and populated by 1b-ii once the embed-worker hookup lands.
//!   6. `write_manifest()` — JSON sibling file recording counts + sync time.
//!
//! Tauri command surface lives in [`crate::commands::repos`]; this module
//! exposes only library primitives so it can be tested without an `AppHandle`.
//!
//! Feature-gated behind `repo-rag` (desktop default). Mobile / headless-mcp
//! builds compile without `gix` / `ignore` / `regex`.

#![cfg(feature = "repo-rag")]

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use sha2::{Digest, Sha256};

use crate::memory::chunking;

/// SHA-256 hex digest of `content` — used for per-file incremental sync
/// (`repo_files.file_hash`).
fn file_hash_hex(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    hex::encode(h.finalize())
}

// ────────────────────────────────────────────────────────────────────────────
// Progress sink (BRAIN-REPO-RAG-1b-ii)
// ────────────────────────────────────────────────────────────────────────────

/// Phase of the ingest pipeline reported to [`IngestSink::progress`].
///
/// Mirrors the phases listed in `rules/milestones.md::BRAIN-REPO-RAG-1b-ii`.
/// The `Embed` phase is emitted by the future `embed_repo` pass (1b-ii-b);
/// `ingest_repo` itself emits everything except `Embed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestPhase {
    Clone,
    Walk,
    ScanSecrets,
    Chunk,
    Embed,
    Persist,
    /// BRAIN-REPO-RAG-2b: a single file was skipped during the deep scan.
    /// The `IngestProgress::message` carries the relative path and the
    /// `skip_reason` field names the reason (`too_large`, `binary`,
    /// `unchanged`, `secret`).
    Skip,
    /// BRAIN-REPO-RAG-2b: human-readable final summary line emitted after
    /// every file has been processed, before `Done`. Carries no progress
    /// counter — `processed == total == files_scanned`.
    Summary,
    Done,
}

impl IngestPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            IngestPhase::Clone => "clone",
            IngestPhase::Walk => "walk",
            IngestPhase::ScanSecrets => "scan_secrets",
            IngestPhase::Chunk => "chunk",
            IngestPhase::Embed => "embed",
            IngestPhase::Persist => "persist",
            IngestPhase::Skip => "skip",
            IngestPhase::Summary => "summary",
            IngestPhase::Done => "done",
        }
    }
}

/// One progress event surfaced from [`ingest_repo_with`] (and the future
/// `embed_repo`). The caller wraps these in whatever transport layer it owns
/// — Tauri events, MCP notifications, log lines, or a `Vec` for tests.
#[derive(Debug, Clone)]
pub struct IngestProgress {
    pub source_id: String,
    pub phase: IngestPhase,
    pub processed: u64,
    pub total: u64,
    pub message: String,
    /// BRAIN-REPO-RAG-2b: when `phase == IngestPhase::Skip`, names the
    /// reason the file was skipped (`too_large`, `binary`, `unchanged`,
    /// `secret`). `None` for every other phase.
    pub skip_reason: Option<&'static str>,
}

/// Receiver for [`IngestProgress`] events. Implementors must be `Sync` so the
/// orchestrator can pass `&dyn IngestSink` across the file walk loop.
pub trait IngestSink: Sync {
    fn progress(&self, event: IngestProgress);
}

/// No-op sink used by tests and CLI callers that don't need progress events.
pub struct SilentSink;
impl IngestSink for SilentSink {
    fn progress(&self, _event: IngestProgress) {}
}

/// Test sink that captures all events into a `Mutex<Vec<_>>`.
#[cfg(test)]
pub(crate) struct CapturingSink {
    pub events: std::sync::Mutex<Vec<IngestProgress>>,
}
#[cfg(test)]
impl CapturingSink {
    pub(crate) fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }
    pub(crate) fn phases(&self) -> Vec<IngestPhase> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.phase)
            .collect()
    }
}
#[cfg(test)]
impl IngestSink for CapturingSink {
    fn progress(&self, event: IngestProgress) {
        self.events.lock().unwrap().push(event);
    }
}

/// Default per-file byte cap: 10 MiB.
pub const DEFAULT_MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
/// Default chunk size (characters, not bytes) for the prose chunker.
pub const DEFAULT_CHUNK_CHARS: usize = 1500;

/// Caller-supplied options for [`ingest_repo`].
#[derive(Debug, Clone)]
pub struct RepoIngestOptions {
    /// Stable identifier (matches `memory_sources.id`). Used as the directory
    /// name under `<data_dir>/repos/<source_id>/`.
    pub source_id: String,
    /// Git remote URL (HTTPS). Only anonymous public clone is supported in
    /// 1b-i; OAuth/token-authenticated clone lands in 1c.
    pub repo_url: String,
    /// Branch / tag / commit. `None` = remote default branch.
    pub repo_ref: Option<String>,
    /// Extra glob patterns the user explicitly wants included even if
    /// `.gitignore` would exclude them. (Forwarded to `ignore::overrides`.)
    pub include_globs: Vec<String>,
    /// Extra glob patterns to exclude on top of `.gitignore` / defaults.
    pub exclude_globs: Vec<String>,
    /// Per-file byte cap. `None` → [`DEFAULT_MAX_FILE_BYTES`].
    pub max_file_bytes: Option<u64>,
}

impl RepoIngestOptions {
    fn cap(&self) -> u64 {
        self.max_file_bytes.unwrap_or(DEFAULT_MAX_FILE_BYTES)
    }
}

/// Aggregate counters returned from [`ingest_repo`] and persisted in
/// `manifest.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoIngestStats {
    pub files_scanned: u64,
    pub files_skipped_size: u64,
    pub files_skipped_secret: u64,
    pub files_skipped_binary: u64,
    pub files_indexed: u64,
    pub chunks_inserted: u64,
    /// BRAIN-REPO-RAG-1b-ii: files whose `file_hash` was unchanged since the
    /// last sync, so they were skipped without re-chunking.
    #[serde(default)]
    pub files_skipped_unchanged: u64,
    /// BRAIN-REPO-RAG-1b-ii: files removed from the repo since the last sync;
    /// their chunks and `repo_files` rows were dropped.
    #[serde(default)]
    pub files_pruned: u64,
}

/// BRAIN-REPO-RAG-1b-ii-b: stats for the embedding pass.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoEmbedStats {
    /// Chunks for which an embedding was generated, persisted as a BLOB, and
    /// added to the per-repo HNSW index.
    pub chunks_embedded: u64,
    /// Chunks that the embedder rejected (returned `None` or wrong dimension).
    pub chunks_failed: u64,
}

/// BRAIN-REPO-RAG-1c-a: hit row returned by [`RepoStore::hybrid_search`].
/// Mirrors the on-disk `repo_chunks` shape with an extra `score` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSearchHit {
    pub id: i64,
    pub source_id: String,
    pub file_path: String,
    pub parent_symbol: Option<String>,
    /// Either `"text"` or `"code"`.
    pub kind: String,
    pub byte_start: u64,
    pub byte_end: u64,
    pub content: String,
    /// RRF fused score across the vector / keyword / recency rankings.
    pub score: f64,
}

/// BRAIN-REPO-RAG-2a: graph-projection of a single `repo_chunks` row.
/// Used by the cross-source knowledge-graph view to render repo nodes
/// alongside personal `MemoryEntry` nodes without dragging the full
/// `RepoSearchHit` envelope (no `byte_start`/`byte_end`/`score`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoChunkNode {
    pub id: i64,
    pub source_id: String,
    pub file_path: String,
    pub parent_symbol: Option<String>,
    pub kind: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Error)]
pub enum RepoIngestError {
    #[error("clone failed: {0}")]
    Clone(String),
    #[error("walk failed: {0}")]
    Walk(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("invalid source_id: {0}")]
    InvalidSourceId(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ann index error: {0}")]
    AnnIndex(String),
}

// ────────────────────────────────────────────────────────────────────────────
// Layout helpers
// ────────────────────────────────────────────────────────────────────────────

/// Validates that `source_id` is filesystem-safe (lowercase ASCII alphanumeric
/// + `-` / `_`). Mirrors the slugifier used by the frontend
///   (`src/views/MemoryView.vue::slugifySourceId`).
pub fn validate_source_id(id: &str) -> Result<(), RepoIngestError> {
    if id.is_empty() || id.len() > 80 {
        return Err(RepoIngestError::InvalidSourceId(id.to_string()));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(RepoIngestError::InvalidSourceId(id.to_string()));
    }
    if id == "self" || id == "__all__" {
        return Err(RepoIngestError::InvalidSourceId(format!(
            "reserved id: {id}"
        )));
    }
    Ok(())
}

pub fn repo_root(data_dir: &Path, source_id: &str) -> PathBuf {
    data_dir.join("repos").join(source_id)
}

pub fn checkout_dir(data_dir: &Path, source_id: &str) -> PathBuf {
    repo_root(data_dir, source_id).join("checkout")
}

pub fn manifest_path(data_dir: &Path, source_id: &str) -> PathBuf {
    repo_root(data_dir, source_id).join("manifest.json")
}

pub fn db_path(data_dir: &Path, source_id: &str) -> PathBuf {
    repo_root(data_dir, source_id).join("memories.db")
}

// ────────────────────────────────────────────────────────────────────────────
// Secret scanner
// ────────────────────────────────────────────────────────────────────────────

/// Compiled regex set for [`is_likely_secret`]. Initialised once and reused
/// across files.
fn secret_patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        // Patterns intentionally conservative to minimise false positives on
        // realistic source files. Inspired by `coding::session_import::
        // redact_secrets` (key=value heuristic) plus the canonical
        // `secrets-patterns-db` shortlist.
        let raw = [
            // PEM private-key block headers
            r"-----BEGIN (?:RSA|EC|DSA|OPENSSH|PGP|ENCRYPTED) PRIVATE KEY-----",
            // AWS access key id
            r"\bAKIA[0-9A-Z]{16}\b",
            // GitHub personal access tokens / fine-grained tokens
            r"\bgh[pousr]_[A-Za-z0-9]{36,}\b",
            // Slack tokens
            r"\bxox[abprs]-[A-Za-z0-9-]{10,}\b",
            // Google API key
            r"\bAIza[0-9A-Za-z_\-]{35}\b",
            // Generic high-entropy `api_key = "..."` style assignment with
            // >=20-char value (matches .env, config.toml, settings.py, etc.).
            r#"(?i)\b(?:api[_-]?key|secret[_-]?key|access[_-]?token|auth[_-]?token|password|passwd)\b\s*[:=]\s*['"]?[A-Za-z0-9_\-+/]{20,}"#,
        ];
        raw.iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect()
    })
}

/// Scans the first 256 KiB of a file's contents for likely secrets. Returns
/// `true` on the first hit. We deliberately bound the scan window so a
/// large file (e.g. a JSON corpus) doesn't dominate the ingest time.
pub fn is_likely_secret(content: &str) -> bool {
    let window = if content.len() > 256 * 1024 {
        &content[..256 * 1024]
    } else {
        content
    };
    secret_patterns().iter().any(|re| re.is_match(window))
}

// ────────────────────────────────────────────────────────────────────────────
// File classification
// ────────────────────────────────────────────────────────────────────────────

/// Subset of extensions we treat as "code" for `repo_chunks.kind`. The list is
/// intentionally short here — full AST chunking (with tree-sitter parsers) is
/// 1b-ii's responsibility.
const CODE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "go", "java", "kt", "kts", "c", "h", "cpp",
    "cc", "cxx", "hpp", "hh", "hxx", "cs", "swift", "rb", "php", "lua", "sh", "bash", "zsh",
    "ps1", "vue", "svelte", "sql", "toml", "yaml", "yml", "json", "md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkKind {
    Text,
    Code,
}

impl ChunkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChunkKind::Text => "text",
            ChunkKind::Code => "code",
        }
    }
    pub fn for_path(path: &Path) -> Self {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        match ext.as_deref() {
            Some(e) if CODE_EXTENSIONS.contains(&e) => ChunkKind::Code,
            _ => ChunkKind::Text,
        }
    }
}

/// Heuristic binary detector: returns `true` if the first 8 KiB contains a NUL
/// byte. Cheap and good enough for "skip the .png/.zip blobs" tier filtering.
pub fn looks_binary(bytes: &[u8]) -> bool {
    let window = if bytes.len() > 8192 { &bytes[..8192] } else { bytes };
    window.contains(&0u8)
}

// ────────────────────────────────────────────────────────────────────────────
// Chunking
// ────────────────────────────────────────────────────────────────────────────

/// Single ingested chunk persisted to `repo_chunks`.
#[derive(Debug, Clone)]
pub struct RepoChunk {
    pub file_path: String,
    pub parent_symbol: Option<String>,
    pub kind: ChunkKind,
    pub byte_start: usize,
    pub byte_end: usize,
    pub content: String,
    pub content_hash: String,
}

/// Chunk a file's text using the existing `chunking::split_markdown` splitter.
/// Each returned chunk carries an approximate byte span computed by scanning
/// `content` for the chunk text starting at the previous cursor — robust
/// because `text-splitter` produces ordered, non-overlapping chunks.
pub fn chunk_text(file_path: &str, content: &str, kind: ChunkKind) -> Vec<RepoChunk> {
    let splits = chunking::split_markdown(content, DEFAULT_CHUNK_CHARS);
    let mut cursor: usize = 0;
    let mut out = Vec::with_capacity(splits.len());
    for split in splits {
        let start = content[cursor..]
            .find(&split.text)
            .map(|off| cursor + off)
            .unwrap_or(cursor);
        let end = start + split.text.len();
        cursor = end;
        out.push(RepoChunk {
            file_path: file_path.to_string(),
            parent_symbol: split.heading.clone(),
            kind,
            byte_start: start,
            byte_end: end,
            content: split.text,
            content_hash: split.hash,
        });
    }
    out
}

// ────────────────────────────────────────────────────────────────────────────
// AST symbol annotation (BRAIN-REPO-RAG-1b-ii)
// ────────────────────────────────────────────────────────────────────────────

/// Annotate text-chunked `chunks` with `parent_symbol` derived from tree-sitter
/// symbol extraction. For each chunk we compute its starting line in `content`
/// and look up the innermost symbol (smallest enclosing line range) whose
/// `line..=end_line` covers that line. `parent_symbol` is formatted as
/// `"<kind>::<name>"` (e.g. `"function::ingest_repo"`,
/// `"method::Foo::bar"`).
///
/// Languages currently routed through this annotator: Rust (`.rs`), TypeScript
/// (`.ts`, `.tsx`). Other languages — and files where tree-sitter fails to
/// parse — pass through unchanged so the prose chunker's heading-derived
/// `parent_symbol` (if any) is preserved.
pub fn ast_annotate_chunks(file_rel: &str, content: &str, chunks: &mut [RepoChunk]) {
    use crate::coding::parser_registry::{create_parser, detect_language, Language};
    use crate::coding::symbol_index::{extract_rust_symbols, extract_ts_symbols};

    let ext = Path::new(file_rel)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let Some(lang) = ext.as_deref().and_then(detect_language) else {
        return;
    };

    let mut parser = create_parser(lang);
    let Some(tree) = parser.parse(content, None) else {
        return;
    };
    let root = tree.root_node();
    let (symbols, _edges) = match lang {
        Language::Rust => extract_rust_symbols(content, root, file_rel),
        Language::TypeScript => extract_ts_symbols(content, root, file_rel),
        #[allow(unreachable_patterns)]
        _ => return, // optional parsers (parser-python/go/java/c) not wired yet
    };
    if symbols.is_empty() {
        return;
    }

    // Sort symbols by range size (smaller = more specific) so the first hit
    // for a line is the innermost. The Symbol line numbers are 1-based.
    let mut by_span: Vec<&crate::coding::symbol_index::Symbol> = symbols.iter().collect();
    by_span.sort_by_key(|s| (s.end_line.saturating_sub(s.line)) as i64);

    // Pre-compute a cumulative byte→line table once per file.
    // Lines are 1-based; entry at index `i` is the byte offset of line `i+1`.
    let mut line_starts: Vec<usize> = Vec::with_capacity(content.len() / 40 + 1);
    line_starts.push(0);
    for (i, b) in content.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }
    let byte_to_line = |byte: usize| -> u32 {
        // Binary search for the largest start <= byte.
        match line_starts.binary_search(&byte) {
            Ok(idx) => (idx + 1) as u32,
            Err(idx) => idx as u32, // idx points one past the line containing `byte`
        }
    };

    for chunk in chunks.iter_mut() {
        // Only re-annotate code chunks where the prose chunker likely produced
        // a less informative heading; for `parent_symbol == None` always set.
        let chunk_line = byte_to_line(chunk.byte_start).max(1);
        if let Some(sym) = by_span
            .iter()
            .find(|s| chunk_line >= s.line && chunk_line <= s.end_line)
        {
            let formatted = match (sym.kind.as_str(), sym.parent.as_deref()) {
                (k, Some(p)) => format!("{k}::{p}::{}", sym.name),
                (k, None) => format!("{k}::{}", sym.name),
            };
            chunk.parent_symbol = Some(formatted);
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Per-repo SQLite store
// ────────────────────────────────────────────────────────────────────────────

pub struct RepoStore {
    conn: Connection,
    source_id: String,
}

impl RepoStore {
    pub fn open(db_file: &Path, source_id: &str) -> Result<Self, RepoIngestError> {
        if let Some(parent) = db_file.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_file)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(REPO_CHUNKS_SCHEMA)?;
        Ok(Self {
            conn,
            source_id: source_id.to_string(),
        })
    }

    /// Replace-on-sync: clears existing rows for this source so a re-sync
    /// always reflects the current HEAD. (Incremental sync via blob-sha
    /// comparison is a 1b-ii enhancement.)
    pub fn clear(&self) -> Result<(), RepoIngestError> {
        self.conn
            .execute("DELETE FROM repo_chunks WHERE source_id = ?1", [&self.source_id])?;
        Ok(())
    }

    pub fn insert_chunks(&mut self, chunks: &[RepoChunk]) -> Result<u64, RepoIngestError> {
        if chunks.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction()?;
        let mut inserted: u64 = 0;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO repo_chunks (
                    source_id, file_path, parent_symbol, kind,
                    byte_start, byte_end, content, content_hash,
                    embedding, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9)",
            )?;
            let now = unix_now_secs() as i64;
            for c in chunks {
                stmt.execute(params![
                    self.source_id,
                    c.file_path,
                    c.parent_symbol,
                    c.kind.as_str(),
                    c.byte_start as i64,
                    c.byte_end as i64,
                    c.content,
                    c.content_hash,
                    now,
                ])?;
                inserted += 1;
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    pub fn chunk_count(&self) -> Result<u64, RepoIngestError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM repo_chunks WHERE source_id = ?1",
            [&self.source_id],
            |row| row.get(0),
        )?;
        Ok(n as u64)
    }

    /// BRAIN-REPO-RAG-1b-ii: load `(file_path -> file_hash)` for this source.
    /// Used by [`ingest_repo_with`] to skip unchanged files.
    pub fn existing_file_hashes(&self) -> Result<HashMap<String, String>, RepoIngestError> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, file_hash FROM repo_files WHERE source_id = ?1",
        )?;
        let rows = stmt.query_map([&self.source_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut out = HashMap::new();
        for r in rows {
            let (p, h) = r?;
            out.insert(p, h);
        }
        Ok(out)
    }

    /// Delete all `repo_chunks` rows for one `(source_id, file_path)` — used
    /// when re-chunking a changed file.
    pub fn delete_file_chunks(&self, file_path: &str) -> Result<u64, RepoIngestError> {
        let n = self.conn.execute(
            "DELETE FROM repo_chunks WHERE source_id = ?1 AND file_path = ?2",
            params![self.source_id, file_path],
        )?;
        Ok(n as u64)
    }

    /// Upsert the file-level hash entry. Called after successful re-chunk of a
    /// file (or first-time ingest).
    pub fn upsert_file_hash(
        &self,
        file_path: &str,
        file_hash: &str,
        chunk_count: u64,
    ) -> Result<(), RepoIngestError> {
        self.conn.execute(
            "INSERT INTO repo_files (source_id, file_path, file_hash, last_synced_at, chunk_count)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (source_id, file_path) DO UPDATE SET
                file_hash = excluded.file_hash,
                last_synced_at = excluded.last_synced_at,
                chunk_count = excluded.chunk_count",
            params![
                self.source_id,
                file_path,
                file_hash,
                unix_now_secs() as i64,
                chunk_count as i64,
            ],
        )?;
        Ok(())
    }

    /// Garbage-collect: drop chunks + file rows for any `file_path` not in
    /// `seen`. Returns `(files_pruned, chunks_pruned)`.
    pub fn prune_missing(
        &self,
        seen: &HashSet<String>,
    ) -> Result<(u64, u64), RepoIngestError> {
        let existing: Vec<String> = {
            let mut stmt = self.conn.prepare(
                "SELECT file_path FROM repo_files WHERE source_id = ?1",
            )?;
            let rows = stmt.query_map([&self.source_id], |r| r.get::<_, String>(0))?;
            rows.filter_map(Result::ok).collect()
        };
        let mut files = 0u64;
        let mut chunks = 0u64;
        for path in existing {
            if !seen.contains(&path) {
                chunks += self.delete_file_chunks(&path)?;
                self.conn.execute(
                    "DELETE FROM repo_files WHERE source_id = ?1 AND file_path = ?2",
                    params![self.source_id, path],
                )?;
                files += 1;
            }
        }
        Ok((files, chunks))
    }

    pub fn file_count(&self) -> Result<u64, RepoIngestError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM repo_files WHERE source_id = ?1",
            [&self.source_id],
            |row| row.get(0),
        )?;
        Ok(n as u64)
    }

    /// BRAIN-REPO-RAG-1b-ii-b: rows whose `embedding` is still NULL, in `id`
    /// order. Used by [`embed_repo_with_fn`].
    pub fn pending_embedding_rows(&self) -> Result<Vec<(i64, String)>, RepoIngestError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content FROM repo_chunks
             WHERE source_id = ?1 AND embedding IS NULL
             ORDER BY id",
        )?;
        let rows = stmt.query_map([&self.source_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// BRAIN-REPO-RAG-1b-ii-b: persist an embedding as little-endian f32 bytes
    /// in the `embedding BLOB` column.
    pub fn set_embedding(&self, id: i64, embedding: &[f32]) -> Result<(), RepoIngestError> {
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        self.conn.execute(
            "UPDATE repo_chunks SET embedding = ?1
             WHERE id = ?2 AND source_id = ?3",
            params![bytes, id, self.source_id],
        )?;
        Ok(())
    }

    /// BRAIN-REPO-RAG-1b-ii-b: rows that already have an embedding BLOB.
    /// Used by tests to assert the embed pass populated every chunk.
    #[cfg(test)]
    pub fn embedded_chunk_count(&self) -> Result<u64, RepoIngestError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM repo_chunks
             WHERE source_id = ?1 AND embedding IS NOT NULL",
            [&self.source_id],
            |row| row.get(0),
        )?;
        Ok(n as u64)
    }

    /// BRAIN-REPO-RAG-1c-a: fetch one chunk by id, scoped to this source.
    pub fn get_chunk(&self, id: i64) -> Result<Option<RepoSearchHit>, RepoIngestError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, file_path, parent_symbol, kind,
                    byte_start, byte_end, content
             FROM repo_chunks WHERE id = ?1 AND source_id = ?2",
        )?;
        let mut rows = stmt.query(params![id, self.source_id])?;
        if let Some(r) = rows.next()? {
            Ok(Some(RepoSearchHit {
                id: r.get(0)?,
                source_id: r.get(1)?,
                file_path: r.get(2)?,
                parent_symbol: r.get(3)?,
                kind: r.get(4)?,
                byte_start: r.get::<_, i64>(5)? as u64,
                byte_end: r.get::<_, i64>(6)? as u64,
                content: r.get(7)?,
                score: 0.0,
            }))
        } else {
            Ok(None)
        }
    }

    /// BRAIN-REPO-RAG-1c-a: distinct `file_path` values for this source,
    /// alphabetically ordered.
    pub fn list_files(&self) -> Result<Vec<String>, RepoIngestError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT file_path FROM repo_chunks
             WHERE source_id = ?1 ORDER BY file_path",
        )?;
        let rows = stmt.query_map([&self.source_id], |r| r.get::<_, String>(0))?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    /// BRAIN-REPO-RAG-2a: lightweight projection of recent chunks for
    /// the cross-source knowledge-graph view. Returns up to `limit` rows
    /// ordered by `created_at DESC, id DESC`, preferring chunks that
    /// carry a `parent_symbol` so the graph surfaces meaningful nodes
    /// (functions / classes / sections) rather than raw text fragments.
    pub fn recent_chunks(
        &self,
        limit: usize,
    ) -> Result<Vec<RepoChunkNode>, RepoIngestError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn.prepare(
            "SELECT id, file_path, parent_symbol, kind, content, created_at
             FROM repo_chunks
             WHERE source_id = ?1
             ORDER BY (parent_symbol IS NULL) ASC, created_at DESC, id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![self.source_id, limit as i64], |r| {
            Ok(RepoChunkNode {
                id: r.get(0)?,
                source_id: self.source_id.clone(),
                file_path: r.get(1)?,
                parent_symbol: r.get(2)?,
                kind: r.get(3)?,
                content: r.get(4)?,
                created_at: r.get(5)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    /// BRAIN-REPO-RAG-1c-a: reassemble a single file's content by
    /// concatenating its chunks ordered by `byte_start`. Used as a fallback
    /// for `repo_read_file` when the on-disk checkout has been removed (e.g.
    /// after `repo_remove_source`); for accuracy production code should
    /// prefer reading the live checkout file from disk.
    pub fn read_file(&self, file_path: &str) -> Result<Option<String>, RepoIngestError> {
        let mut stmt = self.conn.prepare(
            "SELECT content FROM repo_chunks
             WHERE source_id = ?1 AND file_path = ?2
             ORDER BY byte_start, id",
        )?;
        let rows = stmt
            .query_map(params![self.source_id, file_path], |r| {
                r.get::<_, String>(0)
            })?;
        let mut acc = String::new();
        let mut any = false;
        for r in rows {
            let s = r?;
            if any && !acc.ends_with('\n') {
                acc.push('\n');
            }
            acc.push_str(&s);
            any = true;
        }
        if any {
            Ok(Some(acc))
        } else {
            Ok(None)
        }
    }

    /// BRAIN-REPO-RAG-1c-a: source-scoped hybrid search.
    ///
    /// Fuses three independent rankings via Reciprocal Rank Fusion (k=60):
    /// 1. **Vector** — pre-fetched `ann_matches` from the per-repo HNSW
    ///    (passed in so this method stays free of an `AnnIndex` dep and is
    ///    easy to test).
    /// 2. **Keyword** — case-insensitive `LIKE %term%` over `content` for
    ///    each whitespace-split token (≥2 chars), ranked by hit count.
    /// 3. **Recency** — `ORDER BY created_at DESC, id DESC` (top 100).
    ///
    /// Unlike the main-brain `hybrid_search`, repo chunks have no `tier` /
    /// `importance` / `decay_score`, so the signal set is intentionally
    /// narrower. Results are sorted by fused score descending, truncated to
    /// `limit`, and hydrated to full [`RepoSearchHit`] rows.
    pub fn hybrid_search(
        &self,
        query: &str,
        _query_embedding: Option<&[f32]>,
        ann_matches: &[(i64, f32)],
        limit: usize,
    ) -> Result<Vec<RepoSearchHit>, RepoIngestError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        // --- Keyword ranking -------------------------------------------------
        let terms: Vec<String> = query
            .split_whitespace()
            .filter(|t| t.len() >= 2)
            .map(|t| t.to_lowercase())
            .collect();

        let mut keyword_ranking: Vec<i64> = Vec::new();
        if !terms.is_empty() {
            let where_clause = terms
                .iter()
                .map(|_| "LOWER(content) LIKE ?")
                .collect::<Vec<_>>()
                .join(" OR ");
            let sql = format!(
                "SELECT id, content FROM repo_chunks
                 WHERE source_id = ?1 AND ({where_clause})
                 LIMIT 500"
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> =
                vec![Box::new(self.source_id.clone())];
            for t in &terms {
                params_vec.push(Box::new(format!("%{t}%")));
            }
            let params_ref: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|b| b.as_ref()).collect();
            let rows = stmt.query_map(rusqlite::params_from_iter(params_ref), |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?;
            let mut scored: Vec<(i64, usize)> = Vec::new();
            for row in rows {
                let (id, content) = row?;
                let low = content.to_lowercase();
                let hits = terms.iter().filter(|t| low.contains(t.as_str())).count();
                if hits > 0 {
                    scored.push((id, hits));
                }
            }
            scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            keyword_ranking = scored.into_iter().map(|(id, _)| id).collect();
        }

        // --- Recency ranking ------------------------------------------------
        let recency_ranking: Vec<i64> = {
            let mut stmt = self.conn.prepare(
                "SELECT id FROM repo_chunks
                 WHERE source_id = ?1
                 ORDER BY created_at DESC, id DESC
                 LIMIT 100",
            )?;
            let rows = stmt.query_map([&self.source_id], |r| r.get::<_, i64>(0))?;
            rows.filter_map(Result::ok).collect()
        };

        // --- Vector ranking -------------------------------------------------
        let vector_ranking: Vec<i64> = ann_matches.iter().map(|(id, _)| *id).collect();

        // --- RRF fuse (k=60) ------------------------------------------------
        let k = 60f64;
        let mut acc: HashMap<i64, f64> = HashMap::new();
        for (rank, id) in vector_ranking.iter().enumerate() {
            *acc.entry(*id).or_default() += 1.0 / (k + (rank + 1) as f64);
        }
        for (rank, id) in keyword_ranking.iter().enumerate() {
            *acc.entry(*id).or_default() += 1.0 / (k + (rank + 1) as f64);
        }
        for (rank, id) in recency_ranking.iter().enumerate() {
            *acc.entry(*id).or_default() += 1.0 / (k + (rank + 1) as f64);
        }
        let mut fused: Vec<(i64, f64)> = acc.into_iter().collect();
        fused.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        fused.truncate(limit);

        // --- Hydrate --------------------------------------------------------
        let mut hits = Vec::with_capacity(fused.len());
        for (id, score) in fused {
            if let Some(mut hit) = self.get_chunk(id)? {
                hit.score = score;
                hits.push(hit);
            }
        }
        Ok(hits)
    }

    /// BRAIN-REPO-RAG-1d-a: build an Aider-style repo map that fits inside a
    /// token budget. We don't yet have a per-repo symbol-edge graph (deferred
    /// to a future slice), so the importance signal is the simple
    /// **chunk-count per file** heuristic — files with more tree-sitter
    /// annotated chunks are typically modules with the most surface area and
    /// are surfaced first.
    ///
    /// For each ranked file we list up to [`REPO_MAP_MAX_SYMBOLS_PER_FILE`]
    /// distinct `parent_symbol` annotations together with a short signature
    /// preview taken from the first lines of the chunk that introduced the
    /// symbol. Files are appended greedily until adding the next file would
    /// blow the token budget (`chars / 4` estimate, Aider precedent).
    ///
    /// The output is a deterministic plain-text repo map intended for LLM
    /// system prompts — the same shape Aider emits, with `⋮` separators
    /// between symbol entries inside a file.
    pub fn build_repo_map(&self, budget_tokens: usize) -> Result<String, RepoIngestError> {
        if budget_tokens == 0 {
            return Ok(String::new());
        }
        let budget_chars = budget_tokens.saturating_mul(4);

        // Rank files by descending chunk count (importance proxy) then
        // alphabetical (stability).
        let mut stmt = self.conn.prepare(
            "SELECT file_path, COUNT(*) AS chunk_count
             FROM repo_chunks
             WHERE source_id = ?1
             GROUP BY file_path
             ORDER BY chunk_count DESC, file_path ASC",
        )?;
        let ranked: Vec<(String, i64)> = stmt
            .query_map([&self.source_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?
            .filter_map(Result::ok)
            .collect();
        drop(stmt);

        if ranked.is_empty() {
            return Ok(String::new());
        }

        let mut header = String::from("# Repository map for `");
        header.push_str(&self.source_id);
        header.push_str("`\n");
        header.push_str("# Importance ranked by tree-sitter chunk count.\n\n");

        let mut out = header;
        for (file_path, _chunk_count) in &ranked {
            let block = self.render_file_block(file_path)?;
            if block.is_empty() {
                continue;
            }
            // Pre-check the budget; once one file would push us over, stop.
            if out.len() + block.len() > budget_chars {
                if out.lines().count() <= 3 {
                    // Always keep at least the top-ranked file so callers
                    // with a tiny budget still see something useful.
                    out.push_str(&block);
                }
                break;
            }
            out.push_str(&block);
        }
        Ok(out)
    }

    /// BRAIN-REPO-RAG-1d-b: tree-sitter signature-only preview for a single
    /// file. Returns the same `# file_path:` / `⋮ │signature` shape used by
    /// `build_repo_map`, but unfiltered by budget so callers can drill into
    /// any file from the repo map. Returns `Ok(String::new())` when the file
    /// has no recorded chunks (consistent with `read_file` returning `None`).
    pub fn build_file_signatures(&self, file_path: &str) -> Result<String, RepoIngestError> {
        self.render_file_block(file_path)
    }

    /// Internal helper that renders one file's signature block. Used by both
    /// `build_repo_map` and `build_file_signatures` so the on-the-wire shape
    /// stays consistent.
    fn render_file_block(&self, file_path: &str) -> Result<String, RepoIngestError> {
        // Pull distinct (parent_symbol, content, byte_start) entries for this
        // file. Filter `WHERE parent_symbol IS NOT NULL` so we surface only
        // tree-sitter-recognised top-level definitions.
        let mut stmt = self.conn.prepare(
            "SELECT parent_symbol, content, byte_start
             FROM repo_chunks
             WHERE source_id = ?1
               AND file_path = ?2
               AND parent_symbol IS NOT NULL
             ORDER BY byte_start ASC",
        )?;
        let rows: Vec<(String, String, i64)> = stmt
            .query_map(params![self.source_id, file_path], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })?
            .filter_map(Result::ok)
            .collect();
        drop(stmt);

        if rows.is_empty() {
            return Ok(String::new());
        }

        // Deduplicate by parent_symbol keeping the earliest occurrence
        // (byte_start asc already gives us that ordering).
        let mut seen = std::collections::HashSet::<String>::new();
        let mut symbols: Vec<(String, String)> = Vec::new();
        for (sym, content, _) in rows {
            if seen.insert(sym.clone()) {
                symbols.push((sym, content));
            }
            if symbols.len() >= REPO_MAP_MAX_SYMBOLS_PER_FILE {
                break;
            }
        }
        if symbols.is_empty() {
            return Ok(String::new());
        }

        let mut block = String::new();
        block.push_str(file_path);
        block.push_str(":\n");
        for (idx, (_sym, content)) in symbols.iter().enumerate() {
            if idx > 0 {
                block.push_str("⋮\n");
            }
            block.push_str("⋮\n");
            let signature = signature_preview(content);
            for line in signature.lines() {
                block.push('│');
                block.push_str(line);
                block.push('\n');
            }
        }
        block.push('\n');
        Ok(block)
    }
}

/// Cap on the number of distinct `parent_symbol` entries surfaced per file
/// in the repo map. Aider's reference implementation uses a similar
/// per-file cap to keep wide modules from monopolising the budget.
const REPO_MAP_MAX_SYMBOLS_PER_FILE: usize = 8;

/// Number of leading non-blank lines of a chunk we keep as a "signature"
/// preview. tree-sitter usually anchors chunks at definition boundaries
/// (`fn ... {`, `class ... {`, `interface ...`), so the first 1-3 lines
/// reliably contain the signature line plus an opening brace.
const REPO_MAP_SIGNATURE_LINES: usize = 3;

/// Extract the signature preview from a code chunk. Trims trailing
/// whitespace per line and stops at [`REPO_MAP_SIGNATURE_LINES`].
fn signature_preview(content: &str) -> String {
    let mut out = String::new();
    let mut kept = 0usize;
    for raw in content.lines() {
        let line = raw.trim_end();
        if line.trim().is_empty() && kept == 0 {
            continue;
        }
        out.push_str(line);
        out.push('\n');
        kept += 1;
        if kept >= REPO_MAP_SIGNATURE_LINES {
            break;
        }
    }
    // Drop the trailing newline so the caller controls line breaks.
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

const REPO_CHUNKS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS repo_chunks (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id     TEXT    NOT NULL,
    file_path     TEXT    NOT NULL,
    parent_symbol TEXT,
    kind          TEXT    NOT NULL CHECK (kind IN ('text','code')),
    byte_start    INTEGER NOT NULL,
    byte_end      INTEGER NOT NULL,
    content       TEXT    NOT NULL,
    content_hash  TEXT    NOT NULL,
    embedding     BLOB,
    created_at    INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_repo_chunks_source     ON repo_chunks(source_id);
CREATE INDEX IF NOT EXISTS idx_repo_chunks_path       ON repo_chunks(source_id, file_path);
CREATE INDEX IF NOT EXISTS idx_repo_chunks_hash       ON repo_chunks(content_hash);

-- BRAIN-REPO-RAG-1b-ii: per-file content hash for incremental sync.
CREATE TABLE IF NOT EXISTS repo_files (
    source_id      TEXT NOT NULL,
    file_path      TEXT NOT NULL,
    file_hash      TEXT NOT NULL,
    last_synced_at INTEGER NOT NULL,
    chunk_count    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (source_id, file_path)
);
CREATE INDEX IF NOT EXISTS idx_repo_files_source ON repo_files(source_id);
";

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ────────────────────────────────────────────────────────────────────────────
// Shallow clone (gix)
// ────────────────────────────────────────────────────────────────────────────

/// Anonymous shallow clone of `url` into `dest`, removing any existing dir
/// first. Returns the resolved HEAD short hash for `manifest.json`.
pub fn shallow_clone(
    url: &str,
    dest: &Path,
    _repo_ref: Option<&str>,
) -> Result<String, RepoIngestError> {
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let depth = NonZeroU32::new(1).expect("1 is non-zero");
    let mut prep = gix::prepare_clone(url, dest)
        .map_err(|e| RepoIngestError::Clone(format!("prepare_clone: {e}")))?
        .with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(depth));

    let (mut checkout, _outcome) = prep
        .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
        .map_err(|e| RepoIngestError::Clone(format!("fetch_then_checkout: {e}")))?;

    let (repo, _outcome) = checkout
        .main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
        .map_err(|e| RepoIngestError::Clone(format!("main_worktree: {e}")))?;

    let head_hex = repo
        .head_id()
        .map_err(|e| RepoIngestError::Clone(format!("head_id: {e}")))?
        .to_hex_with_len(12)
        .to_string();
    Ok(head_hex)
}

// ────────────────────────────────────────────────────────────────────────────
// File walker
// ────────────────────────────────────────────────────────────────────────────

/// Returns a sorted list of files under `root` that pass:
///   - `.gitignore` / `.git/info/exclude` / `.terransoulignore`
///   - user includes/excludes (overrides applied on top)
///   - per-file byte cap
///   - not in `.git/`
pub fn walk_files(
    root: &Path,
    include_globs: &[String],
    exclude_globs: &[String],
    cap_bytes: u64,
) -> Result<Vec<(PathBuf, u64, bool)>, RepoIngestError> {
    use ignore::overrides::OverrideBuilder;
    use ignore::WalkBuilder;

    let mut overrides = OverrideBuilder::new(root);
    for glob in include_globs {
        overrides
            .add(glob)
            .map_err(|e| RepoIngestError::Walk(format!("include glob '{glob}': {e}")))?;
    }
    for glob in exclude_globs {
        // ignore::overrides treats `!pattern` as an exclude
        let neg = if glob.starts_with('!') {
            glob.clone()
        } else {
            format!("!{glob}")
        };
        overrides
            .add(&neg)
            .map_err(|e| RepoIngestError::Walk(format!("exclude glob '{glob}': {e}")))?;
    }
    let overrides = overrides
        .build()
        .map_err(|e| RepoIngestError::Walk(format!("overrides build: {e}")))?;

    let walker = WalkBuilder::new(root)
        .hidden(false) // .gitignore handles dotfiles already; allow .github/, .env.example, etc.
        .git_ignore(true)
        .git_exclude(true)
        .git_global(false)
        .add_custom_ignore_filename(".terransoulignore")
        .overrides(overrides)
        .build();

    let mut results: BTreeSet<PathBuf> = BTreeSet::new();
    let mut output: Vec<(PathBuf, u64, bool)> = Vec::new();
    for dent in walker.flatten() {
        let path = dent.path();
        if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        // Skip everything under .git/
        if path
            .components()
            .any(|c| c.as_os_str() == std::ffi::OsStr::new(".git"))
        {
            continue;
        }
        let meta = match fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let size = meta.len();
        let over_cap = size > cap_bytes;
        if results.insert(path.to_path_buf()) {
            output.push((path.to_path_buf(), size, over_cap));
        }
    }
    Ok(output)
}

// ────────────────────────────────────────────────────────────────────────────
// Manifest
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoManifest {
    pub source_id: String,
    pub repo_url: String,
    pub repo_ref: Option<String>,
    pub head_commit: Option<String>,
    pub last_synced_at: u64,
    pub stats: RepoIngestStats,
    /// Schema version of the manifest; bump on breaking shape changes.
    pub manifest_version: u32,
}

pub fn write_manifest(path: &Path, manifest: &RepoManifest) -> Result<(), RepoIngestError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(manifest)?;
    fs::write(path, bytes)?;
    Ok(())
}

pub fn read_manifest(path: &Path) -> Result<Option<RepoManifest>, RepoIngestError> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)?;
    let parsed: RepoManifest = serde_json::from_slice(&bytes)?;
    Ok(Some(parsed))
}

// ────────────────────────────────────────────────────────────────────────────
// Top-level orchestrator
// ────────────────────────────────────────────────────────────────────────────

/// End-to-end synchronous ingest. Returns the populated [`RepoManifest`].
///
/// Embeddings are NOT computed here (the `embedding` column is left NULL);
/// the async `embed_repo` pass (1b-ii-b) populates it via
/// `OllamaAgent::embed_text` / `cloud_embeddings::embed_for_mode`.
pub fn ingest_repo(
    data_dir: &Path,
    options: RepoIngestOptions,
) -> Result<RepoManifest, RepoIngestError> {
    ingest_repo_with(data_dir, options, &SilentSink)
}

/// Same as [`ingest_repo`], but emits progress events through `sink`. Used by
/// the Tauri command path to surface phase-by-phase progress to the UI, and
/// by tests via [`CapturingSink`].
pub fn ingest_repo_with(
    data_dir: &Path,
    options: RepoIngestOptions,
    sink: &dyn IngestSink,
) -> Result<RepoManifest, RepoIngestError> {
    validate_source_id(&options.source_id)?;

    let emit = |phase: IngestPhase, processed: u64, total: u64, message: &str| {
        sink.progress(IngestProgress {
            source_id: options.source_id.clone(),
            phase,
            processed,
            total,
            message: message.to_string(),
            skip_reason: None,
        });
    };
    let emit_skip = |processed: u64, total: u64, rel: &str, reason: &'static str| {
        sink.progress(IngestProgress {
            source_id: options.source_id.clone(),
            phase: IngestPhase::Skip,
            processed,
            total,
            message: rel.to_string(),
            skip_reason: Some(reason),
        });
    };

    let root = repo_root(data_dir, &options.source_id);
    fs::create_dir_all(&root)?;
    let checkout = checkout_dir(data_dir, &options.source_id);

    emit(IngestPhase::Clone, 0, 1, &options.repo_url);
    // Inject a stored OAuth token for private GitHub clones. Falls back
    // to the original URL when no token is persisted, preserving
    // anonymous-clone behaviour for public repos.
    let clone_url = match crate::memory::repo_oauth::load_token(data_dir) {
        Some(tok) if !tok.access_token.is_empty() => {
            crate::memory::repo_oauth::inject_https_token(&options.repo_url, &tok.access_token)
        }
        _ => options.repo_url.clone(),
    };
    let head_commit = shallow_clone(&clone_url, &checkout, options.repo_ref.as_deref())?;
    emit(IngestPhase::Clone, 1, 1, &head_commit);

    let cap = options.cap();
    emit(IngestPhase::Walk, 0, 0, "");
    let entries = walk_files(
        &checkout,
        &options.include_globs,
        &options.exclude_globs,
        cap,
    )?;
    let total_files = entries.len() as u64;
    emit(IngestPhase::Walk, total_files, total_files, "");

    let mut store = RepoStore::open(&db_path(data_dir, &options.source_id), &options.source_id)?;
    let existing_hashes = store.existing_file_hashes()?;

    let mut stats = RepoIngestStats::default();
    let mut buffer: Vec<RepoChunk> = Vec::with_capacity(64);
    let mut pending_hashes: Vec<(String, String, u64)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::with_capacity(entries.len());
    let mut processed_files: u64 = 0;

    // Hash files first (cheap), so we can emit a single ScanSecrets event for
    // the actual files we'll scan. We don't pre-buffer file bodies — that
    // would defeat the per-file streaming.
    for (path, size, over_cap) in entries {
        stats.files_scanned += 1;
        processed_files += 1;
        let rel = path
            .strip_prefix(&checkout)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if over_cap {
            stats.files_skipped_size += 1;
            emit_skip(processed_files, total_files, &rel, "too_large");
            continue;
        }

        let mut bytes = Vec::with_capacity(size as usize);
        if let Ok(mut f) = fs::File::open(&path) {
            if f.read_to_end(&mut bytes).is_err() {
                continue;
            }
        } else {
            continue;
        }
        if looks_binary(&bytes) {
            stats.files_skipped_binary += 1;
            emit_skip(processed_files, total_files, &rel, "binary");
            continue;
        }
        let content = match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => {
                stats.files_skipped_binary += 1;
                emit_skip(processed_files, total_files, &rel, "binary");
                continue;
            }
        };

        // Per-file SHA-256 hash, used for incremental skip.
        let file_hash = file_hash_hex(&content);
        seen.insert(rel.clone());
        if existing_hashes.get(&rel) == Some(&file_hash) {
            stats.files_skipped_unchanged += 1;
            emit_skip(processed_files, total_files, &rel, "unchanged");
            continue;
        }

        emit(IngestPhase::ScanSecrets, processed_files, total_files, &rel);
        if is_likely_secret(&content) {
            stats.files_skipped_secret += 1;
            emit_skip(processed_files, total_files, &rel, "secret");
            continue;
        }

        emit(IngestPhase::Chunk, processed_files, total_files, &rel);
        let kind = ChunkKind::for_path(&path);
        let mut chunks = chunk_text(&rel, &content, kind);
        if matches!(kind, ChunkKind::Code) {
            ast_annotate_chunks(&rel, &content, &mut chunks);
        }
        if chunks.is_empty() {
            continue;
        }
        // Replace-on-change: drop stale chunk rows before reinserting.
        store.delete_file_chunks(&rel)?;
        stats.files_indexed += 1;
        pending_hashes.push((rel.clone(), file_hash, chunks.len() as u64));
        buffer.extend(chunks);
        if buffer.len() >= 256 {
            emit(IngestPhase::Persist, processed_files, total_files, &rel);
            stats.chunks_inserted += store.insert_chunks(&buffer)?;
            buffer.clear();
        }
    }
    if !buffer.is_empty() {
        emit(IngestPhase::Persist, processed_files, total_files, "");
        stats.chunks_inserted += store.insert_chunks(&buffer)?;
    }
    // Upsert per-file hashes AFTER chunks are persisted so a mid-pipeline
    // error never leaves a stale hash that masks an empty chunk set.
    for (fp, fh, cc) in &pending_hashes {
        store.upsert_file_hash(fp, fh, *cc)?;
    }

    // Garbage-collect files that disappeared between syncs.
    let (pruned_files, _pruned_chunks) = store.prune_missing(&seen)?;
    stats.files_pruned = pruned_files;

    let manifest = RepoManifest {
        source_id: options.source_id.clone(),
        repo_url: options.repo_url.clone(),
        repo_ref: options.repo_ref.clone(),
        head_commit: Some(head_commit.clone()),
        last_synced_at: unix_now_secs(),
        stats: stats.clone(),
        manifest_version: 1,
    };
    write_manifest(&manifest_path(data_dir, &options.source_id), &manifest)?;
    let summary = format!(
        "scanned={} indexed={} skipped_size={} skipped_binary={} skipped_unchanged={} skipped_secret={} pruned={} chunks={}",
        stats.files_scanned,
        stats.files_indexed,
        stats.files_skipped_size,
        stats.files_skipped_binary,
        stats.files_skipped_unchanged,
        stats.files_skipped_secret,
        stats.files_pruned,
        stats.chunks_inserted,
    );
    emit(IngestPhase::Summary, total_files, total_files, &summary);
    emit(
        IngestPhase::Done,
        stats.files_indexed,
        total_files,
        &head_commit,
    );
    Ok(manifest)
}

/// **Test-only** seam: run the ingest pipeline against an already-checked-out
/// directory at `checkout`, bypassing the gix clone phase. Used by integration
/// tests that don't have a git binary available. Production callers should use
/// [`ingest_repo_with`].
#[cfg(test)]
pub(crate) fn ingest_from_checkout_for_tests(
    data_dir: &Path,
    options: RepoIngestOptions,
    checkout: &Path,
    sink: &dyn IngestSink,
) -> Result<RepoManifest, RepoIngestError> {
    validate_source_id(&options.source_id)?;
    let emit = |phase: IngestPhase, processed: u64, total: u64, message: &str| {
        sink.progress(IngestProgress {
            source_id: options.source_id.clone(),
            phase,
            processed,
            total,
            message: message.to_string(),
            skip_reason: None,
        });
    };
    let emit_skip = |processed: u64, total: u64, rel: &str, reason: &'static str| {
        sink.progress(IngestProgress {
            source_id: options.source_id.clone(),
            phase: IngestPhase::Skip,
            processed,
            total,
            message: rel.to_string(),
            skip_reason: Some(reason),
        });
    };
    let cap = options.cap();
    emit(IngestPhase::Walk, 0, 0, "");
    let entries = walk_files(
        checkout,
        &options.include_globs,
        &options.exclude_globs,
        cap,
    )?;
    let total_files = entries.len() as u64;
    emit(IngestPhase::Walk, total_files, total_files, "");

    let mut store = RepoStore::open(&db_path(data_dir, &options.source_id), &options.source_id)?;
    let existing_hashes = store.existing_file_hashes()?;
    let mut stats = RepoIngestStats::default();
    let mut buffer: Vec<RepoChunk> = Vec::with_capacity(64);
    let mut seen: HashSet<String> = HashSet::with_capacity(entries.len());
    let mut pending_hashes: Vec<(String, String, u64)> = Vec::new();
    let mut processed_files = 0u64;

    for (path, size, over_cap) in entries {
        stats.files_scanned += 1;
        processed_files += 1;
        let rel = path
            .strip_prefix(checkout)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if over_cap {
            stats.files_skipped_size += 1;
            emit_skip(processed_files, total_files, &rel, "too_large");
            continue;
        }
        let mut bytes = Vec::with_capacity(size as usize);
        let Ok(mut f) = fs::File::open(&path) else {
            continue;
        };
        if f.read_to_end(&mut bytes).is_err() {
            continue;
        }
        if looks_binary(&bytes) {
            stats.files_skipped_binary += 1;
            emit_skip(processed_files, total_files, &rel, "binary");
            continue;
        }
        let Ok(content) = String::from_utf8(bytes) else {
            stats.files_skipped_binary += 1;
            emit_skip(processed_files, total_files, &rel, "binary");
            continue;
        };
        let file_hash = file_hash_hex(&content);
        seen.insert(rel.clone());
        if existing_hashes.get(&rel) == Some(&file_hash) {
            stats.files_skipped_unchanged += 1;
            emit_skip(processed_files, total_files, &rel, "unchanged");
            continue;
        }
        emit(IngestPhase::ScanSecrets, processed_files, total_files, &rel);
        if is_likely_secret(&content) {
            stats.files_skipped_secret += 1;
            emit_skip(processed_files, total_files, &rel, "secret");
            continue;
        }
        emit(IngestPhase::Chunk, processed_files, total_files, &rel);
        let kind = ChunkKind::for_path(&path);
        let mut chunks = chunk_text(&rel, &content, kind);
        if matches!(kind, ChunkKind::Code) {
            ast_annotate_chunks(&rel, &content, &mut chunks);
        }
        if chunks.is_empty() {
            continue;
        }
        store.delete_file_chunks(&rel)?;
        stats.files_indexed += 1;
        let cc = chunks.len() as u64;
        pending_hashes.push((rel.clone(), file_hash, cc));
        buffer.extend(chunks);
        if buffer.len() >= 256 {
            emit(IngestPhase::Persist, processed_files, total_files, &rel);
            stats.chunks_inserted += store.insert_chunks(&buffer)?;
            buffer.clear();
        }
    }
    if !buffer.is_empty() {
        emit(IngestPhase::Persist, processed_files, total_files, "");
        stats.chunks_inserted += store.insert_chunks(&buffer)?;
    }
    for (fp, fh, cc) in &pending_hashes {
        store.upsert_file_hash(fp, fh, *cc)?;
    }
    let (pruned, _) = store.prune_missing(&seen)?;
    stats.files_pruned = pruned;
    let manifest = RepoManifest {
        source_id: options.source_id.clone(),
        repo_url: options.repo_url.clone(),
        repo_ref: options.repo_ref.clone(),
        head_commit: None,
        last_synced_at: unix_now_secs(),
        stats: stats.clone(),
        manifest_version: 1,
    };
    write_manifest(&manifest_path(data_dir, &options.source_id), &manifest)?;
    let summary = format!(
        "scanned={} indexed={} skipped_size={} skipped_binary={} skipped_unchanged={} skipped_secret={} pruned={} chunks={}",
        stats.files_scanned,
        stats.files_indexed,
        stats.files_skipped_size,
        stats.files_skipped_binary,
        stats.files_skipped_unchanged,
        stats.files_skipped_secret,
        stats.files_pruned,
        stats.chunks_inserted,
    );
    emit(IngestPhase::Summary, total_files, total_files, &summary);
    emit(IngestPhase::Done, stats.files_indexed, total_files, "");
    Ok(manifest)
}

/// Remove a previously-ingested source: deletes
/// `<data_dir>/repos/<source_id>/` recursively. Idempotent.
pub fn remove_repo(data_dir: &Path, source_id: &str) -> Result<bool, RepoIngestError> {
    validate_source_id(source_id)?;
    let root = repo_root(data_dir, source_id);
    if root.exists() {
        fs::remove_dir_all(&root)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// BRAIN-REPO-RAG-1b-ii-b: embed every `repo_chunks` row whose `embedding`
/// is NULL, persist the f32 BLOB, and add the vector to the per-repo HNSW
/// index at `<data_dir>/repos/<source_id>/vectors.usearch`.
///
/// `embed_fn` is invoked synchronously per chunk and must return `Some(vec)`
/// with `vec.len() == dimensions`. Tauri callers wrap an async embedder
/// (`cloud_embeddings::embed_for_mode`) by calling it through
/// `tokio::runtime::Handle::current().block_on(...)` inside `spawn_blocking`.
///
/// Progress is reported through `sink` using [`IngestPhase::Embed`].
pub fn embed_repo_with_fn<F>(
    data_dir: &Path,
    source_id: &str,
    dimensions: usize,
    mut embed_fn: F,
    sink: &dyn IngestSink,
) -> Result<RepoEmbedStats, RepoIngestError>
where
    F: FnMut(&str) -> Option<Vec<f32>>,
{
    validate_source_id(source_id)?;
    let root = repo_root(data_dir, source_id);
    fs::create_dir_all(&root)?;
    let store = RepoStore::open(&db_path(data_dir, source_id), source_id)?;
    let pending = store.pending_embedding_rows()?;
    let total = pending.len() as u64;
    let mut stats = RepoEmbedStats::default();

    let emit = |processed: u64, message: &str| {
        sink.progress(IngestProgress {
            source_id: source_id.to_string(),
            phase: IngestPhase::Embed,
            processed,
            total,
            message: message.to_string(),
            skip_reason: None,
        });
    };

    if pending.is_empty() {
        emit(0, "no-pending");
        return Ok(stats);
    }

    let index = crate::memory::ann_index::AnnIndex::open(&root, dimensions)
        .map_err(RepoIngestError::AnnIndex)?;

    for (i, (id, content)) in pending.into_iter().enumerate() {
        let processed = (i as u64) + 1;
        match embed_fn(&content) {
            Some(vec) if vec.len() == dimensions => {
                store.set_embedding(id, &vec)?;
                index.add(id, &vec).map_err(RepoIngestError::AnnIndex)?;
                stats.chunks_embedded += 1;
                emit(processed, "ok");
            }
            Some(_) => {
                stats.chunks_failed += 1;
                emit(processed, "dim-mismatch");
            }
            None => {
                stats.chunks_failed += 1;
                emit(processed, "embed-failed");
            }
        }
    }
    index.save().map_err(RepoIngestError::AnnIndex)?;
    Ok(stats)
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn validate_source_id_accepts_slug() {
        assert!(validate_source_id("my-repo").is_ok());
        assert!(validate_source_id("my_repo_2").is_ok());
    }

    #[test]
    fn validate_source_id_rejects_reserved_and_bad() {
        assert!(validate_source_id("self").is_err());
        assert!(validate_source_id("__all__").is_err());
        assert!(validate_source_id("").is_err());
        assert!(validate_source_id("path/with/slash").is_err());
        assert!(validate_source_id("space invalid").is_err());
        assert!(validate_source_id(&"x".repeat(81)).is_err()); // too long
    }

    #[test]
    fn secret_scanner_flags_obvious_secrets() {
        assert!(is_likely_secret("AKIAIOSFODNN7EXAMPLE")); // AWS access key
        assert!(is_likely_secret(
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789ABCD"
        ));
        assert!(is_likely_secret(
            "-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----"
        ));
        assert!(is_likely_secret(
            "api_key = \"sk-proj-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\""
        ));
    }

    #[test]
    fn secret_scanner_ignores_innocent_code() {
        assert!(!is_likely_secret("fn main() { println!(\"hello, world\"); }"));
        assert!(!is_likely_secret("const PI: f64 = 3.14159;"));
        // Short key=value pairs (e.g. config defaults) shouldn't match the
        // 20+ char threshold.
        assert!(!is_likely_secret("api_key = \"\""));
        assert!(!is_likely_secret("password = short"));
    }

    #[test]
    fn looks_binary_detects_nul_byte() {
        assert!(looks_binary(b"text\x00more"));
        assert!(!looks_binary(b"plain ascii text"));
        assert!(!looks_binary("héllo unicode".as_bytes()));
    }

    #[test]
    fn chunk_kind_classifies_by_extension() {
        assert_eq!(ChunkKind::for_path(Path::new("src/main.rs")), ChunkKind::Code);
        assert_eq!(ChunkKind::for_path(Path::new("ui/App.vue")), ChunkKind::Code);
        assert_eq!(
            ChunkKind::for_path(Path::new("docs/intro.md")),
            ChunkKind::Code
        ); // md is in CODE_EXTENSIONS by design (structured)
        assert_eq!(
            ChunkKind::for_path(Path::new("notes/random.bin")),
            ChunkKind::Text
        );
    }

    #[test]
    fn chunk_text_emits_spans_in_order() {
        let body =
            "# Heading A\n\nFirst paragraph.\n\n# Heading B\n\nSecond paragraph.\n".repeat(20);
        let chunks = chunk_text("notes.md", &body, ChunkKind::Code);
        assert!(!chunks.is_empty(), "should produce at least one chunk");
        // Spans monotonically increase
        for w in chunks.windows(2) {
            assert!(w[0].byte_start <= w[1].byte_start);
        }
        // Every chunk maps to a real substring of `body` at its span
        for c in &chunks {
            assert!(c.byte_end <= body.len());
            assert!(c.byte_start <= c.byte_end);
        }
    }

    #[test]
    fn repo_store_round_trip() {
        let tmp = TempDir::new().unwrap();
        let dbp = tmp.path().join("memories.db");
        let mut store = RepoStore::open(&dbp, "demo").unwrap();
        let chunks = vec![
            RepoChunk {
                file_path: "src/lib.rs".into(),
                parent_symbol: None,
                kind: ChunkKind::Code,
                byte_start: 0,
                byte_end: 12,
                content: "fn main() {}".into(),
                content_hash: "h1".into(),
            },
            RepoChunk {
                file_path: "README.md".into(),
                parent_symbol: Some("Intro".into()),
                kind: ChunkKind::Code,
                byte_start: 0,
                byte_end: 5,
                content: "hello".into(),
                content_hash: "h2".into(),
            },
        ];
        let n = store.insert_chunks(&chunks).unwrap();
        assert_eq!(n, 2);
        assert_eq!(store.chunk_count().unwrap(), 2);
        store.clear().unwrap();
        assert_eq!(store.chunk_count().unwrap(), 0);
    }

    #[test]
    fn walk_files_respects_gitignore() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join(".gitignore"), "ignored.txt\n*.log\n").unwrap();
        fs::write(root.join("kept.md"), "hello").unwrap();
        fs::write(root.join("ignored.txt"), "nope").unwrap();
        fs::write(root.join("trace.log"), "noisy").unwrap();
        // .git/ contents must be skipped
        fs::create_dir_all(root.join(".git/objects")).unwrap();
        fs::write(root.join(".git/objects/blob"), "internal").unwrap();

        let entries = walk_files(root, &[], &[], DEFAULT_MAX_FILE_BYTES).unwrap();
        let names: Vec<String> = entries
            .iter()
            .map(|(p, _, _)| {
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert!(names.iter().any(|n| n == "kept.md"), "kept: {names:?}");
        assert!(
            !names.iter().any(|n| n == "ignored.txt"),
            "gitignore not honoured: {names:?}"
        );
        assert!(
            !names.iter().any(|n| n == "trace.log"),
            "*.log glob not honoured: {names:?}"
        );
        assert!(
            !names.iter().any(|n| n.starts_with(".git/")),
            ".git/ leaked: {names:?}"
        );
    }

    #[test]
    fn walk_files_flags_oversize() {
        let tmp = TempDir::new().unwrap();
        let big = tmp.path().join("big.bin");
        fs::write(&big, vec![b'x'; 4096]).unwrap();
        // Use cap=1024 to force over_cap=true
        let entries = walk_files(tmp.path(), &[], &[], 1024).unwrap();
        let row = entries.iter().find(|(p, _, _)| p == &big).unwrap();
        assert!(row.2, "expected over_cap=true for 4096-byte file with 1024 cap");
    }

    #[test]
    fn manifest_round_trip() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("manifest.json");
        let m = RepoManifest {
            source_id: "demo".into(),
            repo_url: "https://example.com/x.git".into(),
            repo_ref: Some("main".into()),
            head_commit: Some("abc123".into()),
            last_synced_at: 42,
            stats: RepoIngestStats {
                files_scanned: 10,
                files_indexed: 7,
                chunks_inserted: 30,
                ..Default::default()
            },
            manifest_version: 1,
        };
        write_manifest(&p, &m).unwrap();
        let back = read_manifest(&p).unwrap().unwrap();
        assert_eq!(back.source_id, "demo");
        assert_eq!(back.stats.chunks_inserted, 30);
    }

    #[test]
    fn remove_repo_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        // Removing non-existent dir succeeds (returns false).
        assert!(!remove_repo(tmp.path(), "ghost").unwrap());
        // Create a dir under repos/<id>/ and remove it.
        fs::create_dir_all(repo_root(tmp.path(), "demo")).unwrap();
        fs::write(repo_root(tmp.path(), "demo").join("x"), "hi").unwrap();
        assert!(remove_repo(tmp.path(), "demo").unwrap());
        assert!(!repo_root(tmp.path(), "demo").exists());
        // Reject reserved id
        assert!(remove_repo(tmp.path(), "self").is_err());
    }

    // ── BRAIN-REPO-RAG-1b-ii tests ──────────────────────────────────────

    #[test]
    fn ast_annotate_sets_parent_symbol_for_rust() {
        // Build a small Rust file with one function spanning lines 1..=5
        let src = "fn alpha() {\n    let x = 1;\n    let y = 2;\n    x + y;\n}\n\nfn beta() {\n    println!(\"b\");\n}\n";
        // chunk_text on a tiny file produces one chunk
        let mut chunks = chunk_text("src/lib.rs", src, ChunkKind::Code);
        assert!(!chunks.is_empty());
        ast_annotate_chunks("src/lib.rs", src, &mut chunks);
        // At least one chunk should carry a function-typed parent_symbol
        let any_parent = chunks
            .iter()
            .filter_map(|c| c.parent_symbol.as_deref())
            .any(|s| s.starts_with("function::"));
        assert!(any_parent, "expected function::* parent_symbol on Rust chunks: {:?}",
            chunks.iter().map(|c| c.parent_symbol.clone()).collect::<Vec<_>>());
    }

    #[test]
    fn ast_annotate_noop_for_unknown_language() {
        let src = "hello world\nthis is plain text";
        let mut chunks = chunk_text("notes/random.txt", src, ChunkKind::Text);
        let before: Vec<_> = chunks.iter().map(|c| c.parent_symbol.clone()).collect();
        ast_annotate_chunks("notes/random.txt", src, &mut chunks);
        let after: Vec<_> = chunks.iter().map(|c| c.parent_symbol.clone()).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn file_hash_hex_is_stable() {
        let a = file_hash_hex("hello world");
        let b = file_hash_hex("hello world");
        let c = file_hash_hex("hello world!");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64); // SHA-256 hex
    }

    #[test]
    fn repo_files_table_round_trip() {
        let tmp = TempDir::new().unwrap();
        let dbp = tmp.path().join("memories.db");
        let store = RepoStore::open(&dbp, "demo").unwrap();
        assert_eq!(store.existing_file_hashes().unwrap().len(), 0);
        store
            .upsert_file_hash("src/a.rs", "hash-a", 3)
            .unwrap();
        store
            .upsert_file_hash("src/b.rs", "hash-b", 5)
            .unwrap();
        let map = store.existing_file_hashes().unwrap();
        assert_eq!(map.get("src/a.rs").map(String::as_str), Some("hash-a"));
        assert_eq!(map.get("src/b.rs").map(String::as_str), Some("hash-b"));
        // Upsert overwrites
        store
            .upsert_file_hash("src/a.rs", "hash-a2", 4)
            .unwrap();
        assert_eq!(
            store.existing_file_hashes().unwrap().get("src/a.rs").unwrap(),
            "hash-a2"
        );
        assert_eq!(store.file_count().unwrap(), 2);
    }

    #[test]
    fn prune_missing_drops_files_not_in_seen() {
        let tmp = TempDir::new().unwrap();
        let dbp = tmp.path().join("memories.db");
        let mut store = RepoStore::open(&dbp, "demo").unwrap();
        // Seed two files with chunks.
        let chunks_a = vec![RepoChunk {
            file_path: "a.rs".to_string(),
            parent_symbol: None,
            kind: ChunkKind::Code,
            byte_start: 0,
            byte_end: 3,
            content: "foo".to_string(),
            content_hash: "h1".to_string(),
        }];
        let chunks_b = vec![RepoChunk {
            file_path: "b.rs".to_string(),
            parent_symbol: None,
            kind: ChunkKind::Code,
            byte_start: 0,
            byte_end: 3,
            content: "bar".to_string(),
            content_hash: "h2".to_string(),
        }];
        store.insert_chunks(&chunks_a).unwrap();
        store.insert_chunks(&chunks_b).unwrap();
        store.upsert_file_hash("a.rs", "ha", 1).unwrap();
        store.upsert_file_hash("b.rs", "hb", 1).unwrap();
        // Only "a.rs" is seen this sync.
        let mut seen = HashSet::new();
        seen.insert("a.rs".to_string());
        let (pruned_files, pruned_chunks) = store.prune_missing(&seen).unwrap();
        assert_eq!(pruned_files, 1);
        assert_eq!(pruned_chunks, 1);
        assert_eq!(store.chunk_count().unwrap(), 1);
        assert_eq!(store.file_count().unwrap(), 1);
        assert!(store.existing_file_hashes().unwrap().contains_key("a.rs"));
    }

    #[test]
    fn ingest_phase_as_str_round_trip() {
        for p in [
            IngestPhase::Clone,
            IngestPhase::Walk,
            IngestPhase::ScanSecrets,
            IngestPhase::Chunk,
            IngestPhase::Embed,
            IngestPhase::Persist,
            IngestPhase::Done,
        ] {
            assert!(!p.as_str().is_empty());
        }
    }

    #[test]
    fn ingest_from_checkout_emits_phases_and_chunks() {
        // Build a small fake checkout with one Rust file + one text file.
        let tmp = TempDir::new().unwrap();
        let checkout = tmp.path().join("checkout");
        fs::create_dir_all(checkout.join("src")).unwrap();
        fs::write(
            checkout.join("src").join("lib.rs"),
            "fn hello() { println!(\"hi\"); }\nfn world() { println!(\"w\"); }\n",
        )
        .unwrap();
        fs::write(
            checkout.join("README.md"),
            "# Title\n\nSome prose. ".repeat(60),
        )
        .unwrap();

        let data_dir = tmp.path().join("data");
        let opts = RepoIngestOptions {
            source_id: "fixture".to_string(),
            repo_url: "file://local".to_string(),
            repo_ref: None,
            include_globs: vec![],
            exclude_globs: vec![],
            max_file_bytes: None,
        };
        let sink = CapturingSink::new();
        let manifest =
            ingest_from_checkout_for_tests(&data_dir, opts.clone(), &checkout, &sink).unwrap();
        assert!(manifest.stats.files_indexed >= 2);
        assert!(manifest.stats.chunks_inserted >= 2);
        let phases = sink.phases();
        assert!(phases.contains(&IngestPhase::Walk));
        assert!(phases.contains(&IngestPhase::Chunk));
        assert!(phases.contains(&IngestPhase::Done));
    }

    /// BRAIN-REPO-RAG-2b — every silent skip path now emits a `Skip` event
    /// with the matching `skip_reason`, plus a final `Summary` event before
    /// `Done`. Guards against the deep-scan-visibility regression where
    /// large/binary/unchanged/secret files were dropped without notice.
    #[test]
    fn ingest_emits_skip_and_summary_events_for_every_decision() {
        let tmp = TempDir::new().unwrap();
        let checkout = tmp.path().join("checkout");
        fs::create_dir_all(&checkout).unwrap();
        // A normal file that gets indexed.
        fs::write(checkout.join("ok.rs"), "fn ok() {}\n").unwrap();
        // A "secret" file — pattern matches `is_likely_secret`.
        fs::write(
            checkout.join("creds.env"),
            "AWS_SECRET_ACCESS_KEY=AKIAABCDEFGHIJKLMNOP\n",
        )
        .unwrap();
        // A binary file (NUL bytes in first 8 KiB).
        fs::write(checkout.join("blob.rs"), b"hello\x00world\x00binary").unwrap();
        // A too-large file when cap is forced small.
        fs::write(checkout.join("big.rs"), "x".repeat(2048)).unwrap();

        let data_dir = tmp.path().join("data");
        let opts = RepoIngestOptions {
            source_id: "skipfix".to_string(),
            repo_url: "file://local".to_string(),
            repo_ref: None,
            include_globs: vec![],
            exclude_globs: vec![],
            // 1 KiB cap forces big.rs to be over-size.
            max_file_bytes: Some(1024),
        };
        let sink = CapturingSink::new();
        let _manifest =
            ingest_from_checkout_for_tests(&data_dir, opts, &checkout, &sink).unwrap();

        let events = sink.events.lock().unwrap();
        let skip_reasons: Vec<&'static str> = events
            .iter()
            .filter(|e| e.phase == IngestPhase::Skip)
            .filter_map(|e| e.skip_reason)
            .collect();
        assert!(
            skip_reasons.contains(&"too_large"),
            "expected too_large skip event, got {skip_reasons:?}",
        );
        assert!(
            skip_reasons.contains(&"binary"),
            "expected binary skip event, got {skip_reasons:?}",
        );
        assert!(
            skip_reasons.contains(&"secret"),
            "expected secret skip event, got {skip_reasons:?}",
        );
        // Summary event fires once, before Done.
        let phases: Vec<IngestPhase> = events.iter().map(|e| e.phase).collect();
        let summary_idx = phases.iter().position(|p| *p == IngestPhase::Summary);
        let done_idx = phases.iter().position(|p| *p == IngestPhase::Done);
        assert!(summary_idx.is_some(), "expected Summary phase: {phases:?}");
        assert!(done_idx.is_some(), "expected Done phase: {phases:?}");
        assert!(summary_idx < done_idx, "Summary must precede Done");
    }

    #[test]
    fn ingest_from_checkout_is_incremental() {
        let tmp = TempDir::new().unwrap();
        let checkout = tmp.path().join("checkout");
        fs::create_dir_all(checkout.join("src")).unwrap();
        let file_a = checkout.join("src").join("a.rs");
        let file_b = checkout.join("src").join("b.rs");
        fs::write(&file_a, "fn alpha() {}\n").unwrap();
        fs::write(&file_b, "fn beta() {}\n").unwrap();

        let data_dir = tmp.path().join("data");
        let opts = RepoIngestOptions {
            source_id: "inc".to_string(),
            repo_url: "file://local".to_string(),
            repo_ref: None,
            include_globs: vec![],
            exclude_globs: vec![],
            max_file_bytes: None,
        };
        let first =
            ingest_from_checkout_for_tests(&data_dir, opts.clone(), &checkout, &SilentSink)
                .unwrap();
        assert_eq!(first.stats.files_indexed, 2);
        assert_eq!(first.stats.files_skipped_unchanged, 0);

        // Re-run with no changes: both files should be skipped as unchanged.
        let second =
            ingest_from_checkout_for_tests(&data_dir, opts.clone(), &checkout, &SilentSink)
                .unwrap();
        assert_eq!(second.stats.files_indexed, 0);
        assert_eq!(second.stats.files_skipped_unchanged, 2);
        assert_eq!(second.stats.chunks_inserted, 0);

        // Modify one file, delete the other.
        fs::write(&file_a, "fn alpha_v2() {}\nfn extra() {}\n").unwrap();
        fs::remove_file(&file_b).unwrap();
        let third =
            ingest_from_checkout_for_tests(&data_dir, opts, &checkout, &SilentSink).unwrap();
        assert_eq!(third.stats.files_indexed, 1);
        assert_eq!(third.stats.files_skipped_unchanged, 0);
        assert_eq!(third.stats.files_pruned, 1);
    }

    #[test]
    fn embed_repo_persists_blobs_and_writes_hnsw_index() {
        // Build a fixture repo, ingest it, then run the embed pass with a
        // deterministic 1024-dim mock embedder. Asserts the BLOB column is
        // populated, the HNSW file exists on disk, and the Embed phase
        // emitted progress events.
        let tmp = TempDir::new().unwrap();
        let checkout = tmp.path().join("checkout");
        fs::create_dir_all(checkout.join("src")).unwrap();
        fs::write(
            checkout.join("src").join("lib.rs"),
            "fn one() { println!(\"a\"); }\nfn two() { println!(\"b\"); }\n",
        )
        .unwrap();
        fs::write(checkout.join("README.md"), "# Title\n\nSome prose.\n").unwrap();

        let data_dir = tmp.path().join("data");
        let opts = RepoIngestOptions {
            source_id: "embed-fixture".to_string(),
            repo_url: "file://local".to_string(),
            repo_ref: None,
            include_globs: vec![],
            exclude_globs: vec![],
            max_file_bytes: None,
        };
        let manifest =
            ingest_from_checkout_for_tests(&data_dir, opts, &checkout, &SilentSink).unwrap();
        assert!(manifest.stats.chunks_inserted >= 2);

        let dims = 1024;
        let sink = CapturingSink::new();
        // Deterministic non-zero mock: vector slot 0 carries the text length
        // so different chunks produce different vectors.
        let mut call_count = 0usize;
        let embed_fn = |text: &str| -> Option<Vec<f32>> {
            call_count += 1;
            let mut v = vec![0.1f32; dims];
            v[0] = text.len() as f32;
            Some(v)
        };
        let stats =
            embed_repo_with_fn(&data_dir, "embed-fixture", dims, embed_fn, &sink).unwrap();
        assert!(stats.chunks_embedded >= 2);
        assert_eq!(stats.chunks_failed, 0);

        // Embed phase emitted events.
        let phases = sink.phases();
        assert!(phases.contains(&IngestPhase::Embed));

        // BLOBs populated.
        let store = RepoStore::open(
            &db_path(&data_dir, "embed-fixture"),
            "embed-fixture",
        )
        .unwrap();
        assert_eq!(store.embedded_chunk_count().unwrap(), stats.chunks_embedded);
        assert!(store.pending_embedding_rows().unwrap().is_empty());

        // HNSW file on disk under the per-repo root.
        let root = repo_root(&data_dir, "embed-fixture");
        assert!(root.join("vectors.usearch").exists());
    }

    #[test]
    fn embed_repo_skips_when_no_pending_rows() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().join("data");
        // Open a fresh per-repo store with no rows.
        {
            let _ = RepoStore::open(&db_path(&data_dir, "empty"), "empty").unwrap();
        }
        let sink = CapturingSink::new();
        let stats = embed_repo_with_fn(
            &data_dir,
            "empty",
            1024,
            |_| Some(vec![0.0; 1024]),
            &sink,
        )
        .unwrap();
        assert_eq!(stats.chunks_embedded, 0);
        assert_eq!(stats.chunks_failed, 0);
        // No HNSW file is created when there's nothing to embed.
        assert!(!repo_root(&data_dir, "empty")
            .join("vectors.usearch")
            .exists());
    }

    #[test]
    fn embed_repo_records_dim_mismatch_as_failure() {
        let tmp = TempDir::new().unwrap();
        let checkout = tmp.path().join("checkout");
        fs::create_dir_all(checkout.join("src")).unwrap();
        fs::write(
            checkout.join("src").join("lib.rs"),
            "fn solo() {}\n",
        )
        .unwrap();
        let data_dir = tmp.path().join("data");
        let opts = RepoIngestOptions {
            source_id: "dim".to_string(),
            repo_url: "file://local".to_string(),
            repo_ref: None,
            include_globs: vec![],
            exclude_globs: vec![],
            max_file_bytes: None,
        };
        let manifest =
            ingest_from_checkout_for_tests(&data_dir, opts, &checkout, &SilentSink).unwrap();
        assert!(manifest.stats.chunks_inserted >= 1);
        let stats = embed_repo_with_fn(
            &data_dir,
            "dim",
            1024,
            // Wrong dimension on purpose.
            |_| Some(vec![0.5f32; 8]),
            &SilentSink,
        )
        .unwrap();
        assert_eq!(stats.chunks_embedded, 0);
        assert!(stats.chunks_failed >= 1);
    }

    // --- BRAIN-REPO-RAG-1c-a: source-scoped retrieval tests ---------------

    fn seed_repo_for_search(source_id: &str) -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let checkout = tmp.path().join("checkout");
        fs::create_dir_all(checkout.join("src")).unwrap();
        fs::write(
            checkout.join("src").join("alpha.rs"),
            "fn alpha_finder() { println!(\"the alpha finder runs\"); }\n",
        )
        .unwrap();
        fs::write(
            checkout.join("src").join("beta.rs"),
            "fn beta_finder() { println!(\"different content here\"); }\n",
        )
        .unwrap();
        fs::write(
            checkout.join("README.md"),
            "# Title\n\nThe alpha module is documented in README prose.\n",
        )
        .unwrap();
        let data_dir = tmp.path().join("data");
        let opts = RepoIngestOptions {
            source_id: source_id.to_string(),
            repo_url: "file://local".to_string(),
            repo_ref: None,
            include_globs: vec![],
            exclude_globs: vec![],
            max_file_bytes: None,
        };
        let _ = ingest_from_checkout_for_tests(&data_dir, opts, &checkout, &SilentSink).unwrap();
        (tmp, data_dir)
    }

    #[test]
    fn repo_hybrid_search_finds_keyword_hits() {
        let (_tmp, data_dir) = seed_repo_for_search("kw");
        let store = RepoStore::open(&db_path(&data_dir, "kw"), "kw").unwrap();
        let hits = store
            .hybrid_search("alpha_finder", None, &[], 5)
            .unwrap();
        assert!(!hits.is_empty(), "expected keyword hits for 'alpha_finder'");
        assert!(hits[0].content.contains("alpha_finder"));
        // The other file should NOT be the top hit.
        assert!(!hits[0].content.contains("beta_finder"));
    }

    #[test]
    fn repo_hybrid_search_fuses_vector_keyword_and_recency() {
        let (_tmp, data_dir) = seed_repo_for_search("fuse");
        let store = RepoStore::open(&db_path(&data_dir, "fuse"), "fuse").unwrap();
        // Pull two real ids from the store to simulate an ANN result.
        let all: Vec<i64> = store
            .conn
            .prepare("SELECT id FROM repo_chunks WHERE source_id = ?1 ORDER BY id")
            .unwrap()
            .query_map(["fuse"], |r| r.get::<_, i64>(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert!(all.len() >= 2, "need at least 2 chunks for fusion test");
        // Promote the LAST chunk via vector ranking and the FIRST via keyword.
        let ann: Vec<(i64, f32)> = vec![(*all.last().unwrap(), 0.99), (all[0], 0.5)];
        let hits = store
            .hybrid_search("alpha_finder", None, &ann, 3)
            .unwrap();
        assert!(!hits.is_empty());
        // Both signals contribute — fused top hits must include at least one
        // id from each ranking.
        let ids: Vec<i64> = hits.iter().map(|h| h.id).collect();
        assert!(ids.contains(all.last().unwrap()) || ids.contains(&all[0]));
        // Scores are strictly positive (RRF always ≥ 1/(60+rank)).
        for h in &hits {
            assert!(h.score > 0.0);
        }
    }

    #[test]
    fn repo_hybrid_search_respects_top_k_and_empty_query() {
        let (_tmp, data_dir) = seed_repo_for_search("topk");
        let store = RepoStore::open(&db_path(&data_dir, "topk"), "topk").unwrap();
        let hits = store.hybrid_search("alpha", None, &[], 1).unwrap();
        assert_eq!(hits.len(), 1);
        // top_k = 0 → empty regardless of input.
        let none = store.hybrid_search("alpha", None, &[], 0).unwrap();
        assert!(none.is_empty());
        // Empty query with no ANN matches falls back to recency only.
        let recent = store.hybrid_search("", None, &[], 5).unwrap();
        assert!(!recent.is_empty());
    }

    #[test]
    fn repo_list_files_returns_distinct_paths_sorted() {
        let (_tmp, data_dir) = seed_repo_for_search("ls");
        let store = RepoStore::open(&db_path(&data_dir, "ls"), "ls").unwrap();
        let files = store.list_files().unwrap();
        assert!(files.contains(&"README.md".to_string()));
        assert!(files.iter().any(|f| f.ends_with("alpha.rs")));
        assert!(files.iter().any(|f| f.ends_with("beta.rs")));
        // Sorted ascending.
        let mut sorted = files.clone();
        sorted.sort();
        assert_eq!(files, sorted);
    }

    #[test]
    fn repo_read_file_reassembles_chunks() {
        let (_tmp, data_dir) = seed_repo_for_search("rd");
        let store = RepoStore::open(&db_path(&data_dir, "rd"), "rd").unwrap();
        let content = store
            .read_file("README.md")
            .unwrap()
            .expect("README chunks should exist");
        assert!(content.contains("Title"));
        assert!(content.contains("alpha module"));
        // Missing file returns None.
        assert!(store.read_file("does-not-exist.txt").unwrap().is_none());
    }

    #[test]
    fn repo_recent_chunks_prefers_parent_symbol_rows_and_respects_limit() {
        // BRAIN-REPO-RAG-2a: graph projection should prefer named symbol
        // rows (functions/sections) over raw text fragments so the
        // cross-source knowledge graph surfaces meaningful nodes.
        let (_tmp, data_dir) = seed_repo_for_search("recent");
        let store = RepoStore::open(&db_path(&data_dir, "recent"), "recent").unwrap();
        let nodes = store.recent_chunks(10).unwrap();
        assert!(!nodes.is_empty());
        // Every returned node carries the source_id of this store.
        assert!(nodes.iter().all(|n| n.source_id == "recent"));
        // limit=0 returns nothing.
        assert!(store.recent_chunks(0).unwrap().is_empty());
        // Among the first hits at least one carries a parent_symbol
        // (seed_repo_for_search inserts AST-annotated chunks for the
        // `alpha.rs` / `beta.rs` files).
        let with_symbol = nodes.iter().filter(|n| n.parent_symbol.is_some()).count();
        assert!(
            with_symbol >= 1,
            "expected at least one symbol-bearing chunk in recent projection"
        );
        // The first row in the projection has a non-NULL parent_symbol
        // because the ORDER BY puts `parent_symbol IS NULL ASC` first.
        assert!(nodes[0].parent_symbol.is_some());
    }

    // ─── BRAIN-REPO-RAG-1d: build_repo_map + build_file_signatures ──────

    #[test]
    fn build_repo_map_ranks_files_by_chunk_count_and_respects_budget() {
        let tmp = TempDir::new().unwrap();
        let dbp = tmp.path().join("memories.db");
        let mut store = RepoStore::open(&dbp, "map").unwrap();
        // file_a has 3 chunks, file_b has 1 chunk — file_a should rank first.
        let mut chunks = Vec::new();
        for i in 0..3 {
            chunks.push(RepoChunk {
                file_path: "src/file_a.rs".into(),
                parent_symbol: Some(format!("fn_a_{i}")),
                kind: ChunkKind::Code,
                byte_start: i * 32,
                byte_end: (i + 1) * 32,
                content: format!("fn fn_a_{i}() {{}}"),
                content_hash: format!("ha{i}"),
            });
        }
        chunks.push(RepoChunk {
            file_path: "src/file_b.rs".into(),
            parent_symbol: Some("fn_b".into()),
            kind: ChunkKind::Code,
            byte_start: 0,
            byte_end: 16,
            content: "fn fn_b() {}".into(),
            content_hash: "hb".into(),
        });
        store.insert_chunks(&chunks).unwrap();

        let map = store.build_repo_map(1024).unwrap();
        assert!(map.contains("Repository map"));
        let a_pos = map.find("src/file_a.rs").expect("file_a present");
        let b_pos = map.find("src/file_b.rs").expect("file_b present");
        assert!(a_pos < b_pos, "file_a (more chunks) should rank first");

        // Empty budget returns empty string.
        assert_eq!(store.build_repo_map(0).unwrap(), "");

        // Tiny budget keeps at least the top file.
        let small = store.build_repo_map(64).unwrap();
        assert!(small.contains("src/file_a.rs"));
    }

    #[test]
    fn build_file_signatures_dedupes_and_uses_aider_shape() {
        let tmp = TempDir::new().unwrap();
        let dbp = tmp.path().join("memories.db");
        let mut store = RepoStore::open(&dbp, "sig").unwrap();
        let chunks = vec![
            RepoChunk {
                file_path: "src/lib.rs".into(),
                parent_symbol: Some("fn alpha".into()),
                kind: ChunkKind::Code,
                byte_start: 0,
                byte_end: 20,
                content: "fn alpha() {\n    body\n}".into(),
                content_hash: "h1".into(),
            },
            // Duplicate parent_symbol — should be deduped, earliest kept.
            RepoChunk {
                file_path: "src/lib.rs".into(),
                parent_symbol: Some("fn alpha".into()),
                kind: ChunkKind::Code,
                byte_start: 30,
                byte_end: 50,
                content: "fn alpha() {\n    other\n}".into(),
                content_hash: "h2".into(),
            },
            RepoChunk {
                file_path: "src/lib.rs".into(),
                parent_symbol: Some("fn beta".into()),
                kind: ChunkKind::Code,
                byte_start: 60,
                byte_end: 80,
                content: "fn beta() {}".into(),
                content_hash: "h3".into(),
            },
        ];
        store.insert_chunks(&chunks).unwrap();

        let out = store.build_file_signatures("src/lib.rs").unwrap();
        assert!(out.starts_with("src/lib.rs:\n"));
        assert!(out.contains("⋮"));
        assert!(out.contains("│fn alpha()"));
        assert!(out.contains("│fn beta()"));
        // Dedup: only one "│fn alpha()" prefix line.
        let alpha_count = out.matches("│fn alpha()").count();
        assert_eq!(alpha_count, 1, "duplicate parent_symbol should be deduped");
    }
}
