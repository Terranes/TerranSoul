//! Tree-sitter powered symbol-table ingest pipeline (Chunks 31.3, 37.2).
//!
//! Walks a repository path, parses Rust and TypeScript files via tree-sitter,
//! extracts function/class/method/struct/enum nodes with file+line+symbol kind,
//! and stores them in a `code_symbols` SQLite table. Also extracts imports and
//! call sites (best-effort, no cross-file resolution yet) into `code_edges`.
//!
//! **Incremental indexing (37.2):** Per-file BLAKE3 content hashes are stored in
//! `code_file_hashes`. On subsequent runs, only files whose hash changed are
//! re-parsed — unchanged files keep their existing symbols and edges.
//!
//! The SQLite database lives alongside the memory store at
//! `<data_dir>/code_index.sqlite`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

// ─── Error type ─────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("tree-sitter parse error for {path}: {detail}")]
    Parse { path: String, detail: String },
    #[error("invalid path: {0}")]
    InvalidPath(String),
}

// ─── Public types ───────────────────────────────────────────────────────────

/// Kind of extracted symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Impl,
    Class,
    Interface,
    TypeAlias,
    Constant,
    Module,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Impl => "impl",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::TypeAlias => "type_alias",
            Self::Constant => "constant",
            Self::Module => "module",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "function" => Some(Self::Function),
            "method" => Some(Self::Method),
            "struct" => Some(Self::Struct),
            "enum" => Some(Self::Enum),
            "trait" => Some(Self::Trait),
            "impl" => Some(Self::Impl),
            "class" => Some(Self::Class),
            "interface" => Some(Self::Interface),
            "type_alias" => Some(Self::TypeAlias),
            "constant" => Some(Self::Constant),
            "module" => Some(Self::Module),
            _ => None,
        }
    }
}

/// A symbol extracted from a source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file: String,
    pub line: u32,
    pub end_line: u32,
    /// For methods: the parent struct/class/impl name.
    pub parent: Option<String>,
}

/// Edge type for code_edges table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Imports,
    Calls,
    /// `pub use` (Rust) or `export { X } from` (TypeScript) re-export chain.
    ReExports,
    /// Class/struct extends a base class/struct.
    Extends,
    /// Struct/class implements a trait/interface.
    Implements,
}

impl EdgeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Imports => "imports",
            Self::Calls => "calls",
            Self::ReExports => "re_exports",
            Self::Extends => "extends",
            Self::Implements => "implements",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "imports" => Some(Self::Imports),
            "calls" => Some(Self::Calls),
            "re_exports" => Some(Self::ReExports),
            "extends" => Some(Self::Extends),
            "implements" => Some(Self::Implements),
            _ => None,
        }
    }
}

/// A best-effort edge (import or call site).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEdge {
    pub from_file: String,
    pub from_line: u32,
    /// Column offset (0-based) of the edge's start within the line. Optional.
    pub from_col: Option<u32>,
    /// End line of the span (for multi-line expressions).
    pub end_line: Option<u32>,
    /// End column of the span.
    pub end_col: Option<u32>,
    pub kind: EdgeKind,
    /// The imported/called name (unresolved — just the identifier text).
    pub target_name: String,
}

impl CodeEdge {
    /// Construct from a tree-sitter node with full span information.
    pub fn from_node(
        file: &str,
        node: &tree_sitter::Node,
        kind: EdgeKind,
        target_name: String,
    ) -> Self {
        Self {
            from_file: file.to_string(),
            from_line: node.start_position().row as u32 + 1,
            from_col: Some(node.start_position().column as u32),
            end_line: Some(node.end_position().row as u32 + 1),
            end_col: Some(node.end_position().column as u32),
            kind,
            target_name,
        }
    }
}

/// Stats returned from an indexing run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub files_parsed: u32,
    pub files_skipped: u32,
    pub files_deleted: u32,
    pub symbols_extracted: u32,
    pub edges_extracted: u32,
    pub errors: Vec<String>,
}

// ─── Database ───────────────────────────────────────────────────────────────

/// Open (or create) the code-index SQLite database.
pub fn open_db(data_dir: &Path) -> Result<Connection, IndexError> {
    let db_path = data_dir.join("code_index.sqlite");
    let conn = Connection::open(db_path)?;
    init_schema(&conn)?;
    migrate_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<(), IndexError> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous  = NORMAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS code_repos (
            id          INTEGER PRIMARY KEY,
            path        TEXT NOT NULL UNIQUE,
            label       TEXT NOT NULL,
            indexed_at  INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS code_file_hashes (
            id          INTEGER PRIMARY KEY,
            repo_id     INTEGER NOT NULL REFERENCES code_repos(id) ON DELETE CASCADE,
            file        TEXT NOT NULL,
            hash        TEXT NOT NULL,
            indexed_at  INTEGER NOT NULL,
            UNIQUE(repo_id, file)
        );

        CREATE TABLE IF NOT EXISTS code_symbols (
            id          INTEGER PRIMARY KEY,
            repo_id     INTEGER NOT NULL REFERENCES code_repos(id) ON DELETE CASCADE,
            name        TEXT NOT NULL,
            kind        TEXT NOT NULL,
            file        TEXT NOT NULL,
            line        INTEGER NOT NULL,
            end_line    INTEGER NOT NULL,
            parent      TEXT,
            UNIQUE(repo_id, file, name, line)
        );

        CREATE INDEX IF NOT EXISTS idx_code_symbols_name ON code_symbols(name);
        CREATE INDEX IF NOT EXISTS idx_code_symbols_file ON code_symbols(file);
        CREATE INDEX IF NOT EXISTS idx_code_symbols_kind ON code_symbols(kind);

        CREATE TABLE IF NOT EXISTS code_edges (
            id          INTEGER PRIMARY KEY,
            repo_id     INTEGER NOT NULL REFERENCES code_repos(id) ON DELETE CASCADE,
            from_file   TEXT NOT NULL,
            from_line   INTEGER NOT NULL,
            from_col    INTEGER,
            end_line    INTEGER,
            end_col     INTEGER,
            kind        TEXT NOT NULL,
            target_name TEXT NOT NULL,
            target_file TEXT,
            target_symbol_id INTEGER REFERENCES code_symbols(id) ON DELETE SET NULL,
            from_symbol_id   INTEGER REFERENCES code_symbols(id) ON DELETE SET NULL,
            confidence  TEXT,
            resolver_tier TEXT,
            provenance  TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_code_edges_target ON code_edges(target_name);
        CREATE INDEX IF NOT EXISTS idx_code_edges_file ON code_edges(from_file);
        CREATE INDEX IF NOT EXISTS idx_code_edges_target_sym ON code_edges(target_symbol_id);
        CREATE INDEX IF NOT EXISTS idx_code_edges_from_sym ON code_edges(from_symbol_id);

        -- ── Multi-repo groups (chunk 37.13) ─────────────────────────────────
        CREATE TABLE IF NOT EXISTS code_repo_groups (
            id          INTEGER PRIMARY KEY,
            label       TEXT NOT NULL UNIQUE,
            description TEXT,
            created_at  INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS code_repo_group_members (
            id          INTEGER PRIMARY KEY,
            group_id    INTEGER NOT NULL REFERENCES code_repo_groups(id) ON DELETE CASCADE,
            repo_id     INTEGER NOT NULL REFERENCES code_repos(id) ON DELETE CASCADE,
            role        TEXT,
            UNIQUE(group_id, repo_id)
        );

        CREATE INDEX IF NOT EXISTS idx_repo_group_members_group
            ON code_repo_group_members(group_id);
        CREATE INDEX IF NOT EXISTS idx_repo_group_members_repo
            ON code_repo_group_members(repo_id);

        -- Public API contracts extracted from a repo's symbol surface.
        -- A "contract" is an exported/public symbol that other repos in
        -- the same group may depend on. The signature_hash detects
        -- breaking changes across re-indexing runs.
        CREATE TABLE IF NOT EXISTS code_contracts (
            id              INTEGER PRIMARY KEY,
            repo_id         INTEGER NOT NULL REFERENCES code_repos(id) ON DELETE CASCADE,
            symbol_id       INTEGER NOT NULL REFERENCES code_symbols(id) ON DELETE CASCADE,
            name            TEXT NOT NULL,
            kind            TEXT NOT NULL,
            file            TEXT NOT NULL,
            line            INTEGER NOT NULL,
            signature_hash  TEXT NOT NULL,
            extracted_at    INTEGER NOT NULL,
            UNIQUE(repo_id, symbol_id)
        );

        CREATE INDEX IF NOT EXISTS idx_code_contracts_repo
            ON code_contracts(repo_id);
        CREATE INDEX IF NOT EXISTS idx_code_contracts_name
            ON code_contracts(name);
        "#,
    )?;
    Ok(())
}

/// Migrate existing databases that were created before new columns were added.
fn migrate_schema(conn: &Connection) -> Result<(), IndexError> {
    // Check if the provenance columns exist; if not, add them.
    let has_resolver_tier: bool = conn
        .prepare("SELECT resolver_tier FROM code_edges LIMIT 0")
        .is_ok();
    if !has_resolver_tier {
        conn.execute_batch(
            "ALTER TABLE code_edges ADD COLUMN from_col INTEGER;
             ALTER TABLE code_edges ADD COLUMN end_line INTEGER;
             ALTER TABLE code_edges ADD COLUMN end_col INTEGER;
             ALTER TABLE code_edges ADD COLUMN resolver_tier TEXT;
             ALTER TABLE code_edges ADD COLUMN provenance TEXT;",
        )?;
    }
    Ok(())
}

// ─── Indexing pipeline ──────────────────────────────────────────────────────

/// Compute a hex-encoded SHA-256 content hash for a file's contents.
fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Index an entire repository **incrementally**. Walks Rust (`.rs`) and
/// TypeScript (`.ts`/`.tsx`) files, computes content hashes, and only re-parses
/// files whose content has changed since the last run. Deleted files have their
/// symbols and edges removed.
pub fn index_repo(data_dir: &Path, repo_path: &Path) -> Result<IndexStats, IndexError> {
    let repo_path = repo_path
        .canonicalize()
        .map_err(|e| IndexError::InvalidPath(format!("{}: {e}", repo_path.display())))?;

    let conn = open_db(data_dir)?;

    // Upsert the repo entry.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let repo_path_str = repo_path.to_string_lossy().to_string();
    let label = repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "repo".to_string());

    conn.execute(
        "INSERT INTO code_repos (path, label, indexed_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(path) DO UPDATE SET indexed_at = excluded.indexed_at",
        params![repo_path_str, label, now],
    )?;

    let repo_id: i64 = conn.query_row(
        "SELECT id FROM code_repos WHERE path = ?1",
        params![repo_path_str],
        |r| r.get(0),
    )?;

    // ── Incremental: load existing file hashes ──────────────────────────────
    let mut existing_hashes: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    {
        let mut stmt =
            conn.prepare("SELECT file, hash FROM code_file_hashes WHERE repo_id = ?1")?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (file, hash) = row?;
            existing_hashes.insert(file, hash);
        }
    }

    // Collect current source files.
    let files = collect_source_files(&repo_path);
    let current_rel_paths: HashSet<String> = files
        .iter()
        .map(|p| {
            p.strip_prefix(&repo_path)
                .unwrap_or(p)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();

    let mut stats = IndexStats {
        files_parsed: 0,
        files_skipped: 0,
        files_deleted: 0,
        symbols_extracted: 0,
        edges_extracted: 0,
        errors: Vec::new(),
    };

    let tx = conn.unchecked_transaction()?;

    // ── Remove data for deleted files ───────────────────────────────────────
    for old_file in existing_hashes.keys() {
        if !current_rel_paths.contains(old_file) {
            tx.execute(
                "DELETE FROM code_symbols WHERE repo_id = ?1 AND file = ?2",
                params![repo_id, old_file],
            )?;
            tx.execute(
                "DELETE FROM code_edges WHERE repo_id = ?1 AND from_file = ?2",
                params![repo_id, old_file],
            )?;
            tx.execute(
                "DELETE FROM code_file_hashes WHERE repo_id = ?1 AND file = ?2",
                params![repo_id, old_file],
            )?;
            stats.files_deleted += 1;
        }
    }

    // ── Parse changed files ─────────────────────────────────────────────────
    let mut rust_parser = new_rust_parser();
    let mut ts_parser = new_typescript_parser();
    // Extended parsers are created on demand based on feature flags.
    #[allow(unused_mut)]
    let mut extra_parsers: std::collections::HashMap<&str, tree_sitter::Parser> =
        std::collections::HashMap::new();

    for file_path in &files {
        let rel_path = file_path
            .strip_prefix(&repo_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");

        let source_bytes = match std::fs::read(file_path) {
            Ok(b) => b,
            Err(e) => {
                stats.errors.push(format!("{rel_path}: {e}"));
                continue;
            }
        };

        let hash = content_hash(&source_bytes);

        // Skip if hash matches — file unchanged.
        if existing_hashes.get(&rel_path).map(|h| h.as_str()) == Some(hash.as_str()) {
            stats.files_skipped += 1;
            continue;
        }

        let source = match String::from_utf8(source_bytes) {
            Ok(s) => s,
            Err(e) => {
                stats.errors.push(format!("{rel_path}: invalid UTF-8: {e}"));
                continue;
            }
        };

        let ext = file_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let lang = super::parser_registry::detect_language(&ext);

        // Parse with the appropriate parser.
        let (symbols, edges) = match lang {
            Some(super::parser_registry::Language::Rust) => {
                let tree = match rust_parser.parse(&source, None) {
                    Some(t) => t,
                    None => {
                        stats
                            .errors
                            .push(format!("{rel_path}: tree-sitter parse returned None"));
                        continue;
                    }
                };
                extract_rust_symbols(&source, tree.root_node(), &rel_path)
            }
            Some(super::parser_registry::Language::TypeScript) => {
                let tree = match ts_parser.parse(&source, None) {
                    Some(t) => t,
                    None => {
                        stats
                            .errors
                            .push(format!("{rel_path}: tree-sitter parse returned None"));
                        continue;
                    }
                };
                extract_ts_symbols(&source, tree.root_node(), &rel_path)
            }
            #[allow(unreachable_patterns)]
            Some(extended_lang) => {
                // Use registry for Python/Go/Java/C/C++
                let parser = extra_parsers
                    .entry(extended_lang.name())
                    .or_insert_with(|| super::parser_registry::create_parser(extended_lang));
                let tree = match parser.parse(&source, None) {
                    Some(t) => t,
                    None => {
                        stats
                            .errors
                            .push(format!("{rel_path}: tree-sitter parse returned None"));
                        continue;
                    }
                };
                super::parser_registry::extract_symbols(
                    extended_lang,
                    &source,
                    tree.root_node(),
                    &rel_path,
                )
            }
            None => {
                // Extension not recognized — skip.
                continue;
            }
        };

        // Clear old data for this specific file, then re-insert.
        tx.execute(
            "DELETE FROM code_symbols WHERE repo_id = ?1 AND file = ?2",
            params![repo_id, rel_path],
        )?;
        tx.execute(
            "DELETE FROM code_edges WHERE repo_id = ?1 AND from_file = ?2",
            params![repo_id, rel_path],
        )?;

        for sym in &symbols {
            tx.execute(
                "INSERT OR IGNORE INTO code_symbols (repo_id, name, kind, file, line, end_line, parent)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    repo_id,
                    sym.name,
                    sym.kind.as_str(),
                    sym.file,
                    sym.line,
                    sym.end_line,
                    sym.parent,
                ],
            )?;
        }

        for edge in &edges {
            tx.execute(
                "INSERT INTO code_edges (repo_id, from_file, from_line, from_col, end_line, end_col, kind, target_name)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    repo_id,
                    edge.from_file,
                    edge.from_line,
                    edge.from_col,
                    edge.end_line,
                    edge.end_col,
                    edge.kind.as_str(),
                    edge.target_name,
                ],
            )?;
        }

        // Upsert the content hash.
        tx.execute(
            "INSERT INTO code_file_hashes (repo_id, file, hash, indexed_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(repo_id, file) DO UPDATE SET hash = excluded.hash, indexed_at = excluded.indexed_at",
            params![repo_id, rel_path, hash, now],
        )?;

        stats.files_parsed += 1;
        stats.symbols_extracted += symbols.len() as u32;
        stats.edges_extracted += edges.len() as u32;
    }

    tx.commit()?;
    Ok(stats)
}

// ─── File collection ────────────────────────────────────────────────────────

fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_recursive(root, &mut files);
    files.sort();
    files
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let supported = super::parser_registry::supported_extensions();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip common non-source directories.
        if path.is_dir() {
            if matches!(
                name_str.as_ref(),
                "target" | "node_modules" | ".git" | "dist" | "build" | ".next" | "vendor"
            ) {
                continue;
            }
            collect_recursive(&path, out);
        } else if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();
            if supported.contains(&ext_str.as_ref()) {
                out.push(path);
            }
        }
    }
}

// ─── Parser construction ────────────────────────────────────────────────────

fn new_rust_parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("failed to load tree-sitter-rust grammar");
    parser
}

fn new_typescript_parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .expect("failed to load tree-sitter-typescript grammar");
    parser
}

// ─── Rust extraction ────────────────────────────────────────────────────────

fn extract_rust_symbols(
    source: &str,
    root: tree_sitter::Node,
    file: &str,
) -> (Vec<Symbol>, Vec<CodeEdge>) {
    let mut symbols = Vec::new();
    let mut edges = Vec::new();
    walk_rust_node(source, root, file, None, &mut symbols, &mut edges);
    (symbols, edges)
}

fn walk_rust_node(
    source: &str,
    node: tree_sitter::Node,
    file: &str,
    parent_name: Option<&str>,
    symbols: &mut Vec<Symbol>,
    edges: &mut Vec<CodeEdge>,
) {
    match node.kind() {
        "function_item" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Function,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(String::from),
                });
            }
        }
        "struct_item" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Struct,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: None,
                });
                // Walk children with this as parent.
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        walk_rust_node(source, child, file, Some(&name), symbols, edges);
                    }
                }
                return;
            }
        }
        "enum_item" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Enum,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: None,
                });
            }
        }
        "trait_item" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Trait,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: None,
                });
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        walk_rust_node(source, child, file, Some(&name), symbols, edges);
                    }
                }
                return;
            }
        }
        "impl_item" => {
            // Extract the type name being implemented.
            // For `impl Trait for Type`, we have: trait field + type field.
            let trait_name = node.child_by_field_name("trait").and_then(|t| {
                if t.kind() == "type_identifier" {
                    Some(node_text(source, &t))
                } else {
                    t.child_by_field_name("type")
                        .map(|inner| node_text(source, &inner))
                }
            });
            let impl_name = node.child_by_field_name("type").and_then(|t| {
                // Could be a simple type_identifier or a generic_type.
                if t.kind() == "type_identifier" {
                    Some(node_text(source, &t))
                } else {
                    t.child_by_field_name("type")
                        .map(|inner| node_text(source, &inner))
                }
            });
            if let Some(ref name) = impl_name {
                symbols.push(Symbol {
                    name: format!("impl {name}"),
                    kind: SymbolKind::Impl,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: None,
                });

                // If this is `impl Trait for Type`, emit an Implements edge.
                if let Some(ref tr) = trait_name {
                    edges.push(CodeEdge::from_node(
                        file,
                        &node,
                        EdgeKind::Implements,
                        tr.clone(),
                    ));
                }
            }
            // Walk body for methods.
            if let Some(body) = node.child_by_field_name("body") {
                let pname = impl_name.as_deref();
                for i in 0..body.child_count() {
                    if let Some(child) = body.child(i) {
                        if child.kind() == "function_item" {
                            if let Some(mname) = node_name(source, &child, "name") {
                                symbols.push(Symbol {
                                    name: mname,
                                    kind: SymbolKind::Method,
                                    file: file.to_string(),
                                    line: child.start_position().row as u32 + 1,
                                    end_line: child.end_position().row as u32 + 1,
                                    parent: pname.map(String::from),
                                });
                            }
                        }
                        // Recurse for call extraction.
                        walk_rust_node(source, child, file, pname, symbols, edges);
                    }
                }
            }
            return;
        }
        "const_item" | "static_item" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Constant,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(String::from),
                });
            }
        }
        "type_item" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::TypeAlias,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(String::from),
                });
            }
        }
        "mod_item" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Module,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: None,
                });
            }
        }
        "use_declaration" => {
            // Extract the path being imported.
            // Detect `pub use` re-exports vs plain `use` imports.
            let is_pub = {
                let line_start = node.start_byte();
                let preceding = &source[..line_start];
                // Check if `pub` keyword appears just before this node.
                let trimmed = preceding.trim_end();
                trimmed.ends_with("pub")
            };
            // Better: check if any sibling/child is a visibility modifier.
            let is_pub = is_pub
                || (0..node.child_count()).any(|i| {
                    node.child(i)
                        .map(|c| c.kind() == "visibility_modifier")
                        .unwrap_or(false)
                });
            if let Some(arg) = node.child_by_field_name("argument") {
                let text = node_text(source, &arg);
                // Extract last segment as the target name.
                let target = text.rsplit("::").next().unwrap_or(&text);
                if !target.is_empty() && target != "*" {
                    let kind = if is_pub {
                        EdgeKind::ReExports
                    } else {
                        EdgeKind::Imports
                    };
                    edges.push(CodeEdge::from_node(file, &node, kind, target.to_string()));
                }
            }
        }
        "call_expression" => {
            // Extract the function being called.
            if let Some(func) = node.child_by_field_name("function") {
                let text = node_text(source, &func);
                // Get the last segment (method or function name).
                let target = text.rsplit("::").next().unwrap_or(&text);
                let target = target.rsplit('.').next().unwrap_or(target);
                if !target.is_empty() && target.len() < 100 {
                    edges.push(CodeEdge::from_node(
                        file,
                        &node,
                        EdgeKind::Calls,
                        target.to_string(),
                    ));
                }
            }
        }
        _ => {}
    }

    // Recurse into children.
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_rust_node(source, child, file, parent_name, symbols, edges);
        }
    }
}

// ─── TypeScript extraction ──────────────────────────────────────────────────

fn extract_ts_symbols(
    source: &str,
    root: tree_sitter::Node,
    file: &str,
) -> (Vec<Symbol>, Vec<CodeEdge>) {
    let mut symbols = Vec::new();
    let mut edges = Vec::new();
    walk_ts_node(source, root, file, None, &mut symbols, &mut edges);
    (symbols, edges)
}

fn walk_ts_node(
    source: &str,
    node: tree_sitter::Node,
    file: &str,
    parent_name: Option<&str>,
    symbols: &mut Vec<Symbol>,
    edges: &mut Vec<CodeEdge>,
) {
    match node.kind() {
        "function_declaration" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Function,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(String::from),
                });
            }
        }
        "method_definition" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Method,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(String::from),
                });
            }
        }
        "class_declaration" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: None,
                });
                // Extract heritage: extends and implements.
                extract_ts_heritage(source, &node, file, edges);
                // Walk children with class as parent.
                if let Some(body) = node.child_by_field_name("body") {
                    for i in 0..body.child_count() {
                        if let Some(child) = body.child(i) {
                            walk_ts_node(source, child, file, Some(&name), symbols, edges);
                        }
                    }
                }
                return;
            }
        }
        "interface_declaration" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Interface,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: None,
                });
            }
        }
        "type_alias_declaration" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::TypeAlias,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: None,
                });
            }
        }
        "enum_declaration" => {
            if let Some(name) = node_name(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Enum,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: None,
                });
            }
        }
        // Variable declarations that are arrow functions or exported consts.
        "lexical_declaration" | "variable_declaration" => {
            // Check for `const X = ...` or `export function`.
            for i in 0..node.named_child_count() {
                if let Some(decl) = node.named_child(i) {
                    if decl.kind() == "variable_declarator" {
                        if let Some(name_node) = decl.child_by_field_name("name") {
                            let name = node_text(source, &name_node);
                            // Check if value is an arrow function.
                            if let Some(value) = decl.child_by_field_name("value") {
                                if value.kind() == "arrow_function" || value.kind() == "function" {
                                    symbols.push(Symbol {
                                        name,
                                        kind: SymbolKind::Function,
                                        file: file.to_string(),
                                        line: node.start_position().row as u32 + 1,
                                        end_line: node.end_position().row as u32 + 1,
                                        parent: parent_name.map(String::from),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        "import_statement" => {
            // Extract imported names.
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    if child.kind() == "import_clause" {
                        extract_ts_import_names(
                            source,
                            child,
                            file,
                            &mut *edges,
                            node.start_position().row as u32 + 1,
                        );
                    }
                }
            }
        }
        // Re-export: `export { X } from '...'` or `export { X }`
        "export_statement" => {
            // Check if this is a re-export (has a `source` field pointing to another module).
            let has_source = node.child_by_field_name("source").is_some();
            if has_source {
                // This is `export { ... } from '...'` — a re-export.
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "export_clause" {
                            for j in 0..child.named_child_count() {
                                if let Some(spec) = child.named_child(j) {
                                    if spec.kind() == "export_specifier" {
                                        if let Some(name_node) = spec.child_by_field_name("name") {
                                            let name = node_text(source, &name_node);
                                            if !name.is_empty() {
                                                edges.push(CodeEdge::from_node(
                                                    file,
                                                    &node,
                                                    EdgeKind::ReExports,
                                                    name,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        "call_expression" => {
            if let Some(func) = node.child_by_field_name("function") {
                let text = node_text(source, &func);
                let target = text.rsplit('.').next().unwrap_or(&text);
                if !target.is_empty() && target.len() < 100 && !target.starts_with('(') {
                    edges.push(CodeEdge::from_node(
                        file,
                        &node,
                        EdgeKind::Calls,
                        target.to_string(),
                    ));
                }
            }
        }
        _ => {}
    }

    // Recurse.
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_ts_node(source, child, file, parent_name, symbols, edges);
        }
    }
}

#[allow(clippy::only_used_in_recursion)]
fn extract_ts_import_names(
    source: &str,
    node: tree_sitter::Node,
    file: &str,
    edges: &mut Vec<CodeEdge>,
    line: u32,
) {
    // Recurse looking for import_specifier nodes.
    if node.kind() == "import_specifier" || node.kind() == "identifier" {
        let name = node_text(source, &node);
        if !name.is_empty() {
            edges.push(CodeEdge::from_node(file, &node, EdgeKind::Imports, name));
        }
        return;
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_ts_import_names(source, child, file, edges, line);
        }
    }
}

/// Extract heritage clauses (extends/implements) from a TS/JS class declaration.
fn extract_ts_heritage(
    source: &str,
    class_node: &tree_sitter::Node,
    file: &str,
    edges: &mut Vec<CodeEdge>,
) {
    let line = class_node.start_position().row as u32 + 1;

    // Look for class_heritage / extends_clause / implements_clause children.
    for i in 0..class_node.child_count() {
        if let Some(child) = class_node.child(i) {
            match child.kind() {
                "class_heritage" => {
                    // class_heritage contains extends_clause and/or implements_clause.
                    for j in 0..child.child_count() {
                        if let Some(clause) = child.child(j) {
                            match clause.kind() {
                                "extends_clause" => {
                                    extract_heritage_names(
                                        source,
                                        &clause,
                                        file,
                                        line,
                                        EdgeKind::Extends,
                                        edges,
                                    );
                                }
                                "implements_clause" => {
                                    extract_heritage_names(
                                        source,
                                        &clause,
                                        file,
                                        line,
                                        EdgeKind::Implements,
                                        edges,
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // tree-sitter-typescript may put extends_clause directly as child.
                "extends_clause" => {
                    extract_heritage_names(source, &child, file, line, EdgeKind::Extends, edges);
                }
                "implements_clause" => {
                    extract_heritage_names(source, &child, file, line, EdgeKind::Implements, edges);
                }
                _ => {}
            }
        }
    }
}

/// Helper to extract type identifiers from an extends/implements clause.
fn extract_heritage_names(
    source: &str,
    clause: &tree_sitter::Node,
    file: &str,
    _line: u32,
    kind: EdgeKind,
    edges: &mut Vec<CodeEdge>,
) {
    for i in 0..clause.named_child_count() {
        if let Some(child) = clause.named_child(i) {
            // The child may be a type_identifier, generic_type, or member_expression.
            let name = match child.kind() {
                "type_identifier" | "identifier" => Some(node_text(source, &child)),
                "generic_type" => child
                    .child_by_field_name("name")
                    .map(|n| node_text(source, &n)),
                _ => {
                    // Fallback: try the text if it's short enough to be a type name.
                    let text = node_text(source, &child);
                    if text.len() < 80 && !text.contains('{') {
                        // Try to get just the first identifier-like segment.
                        text.split('<').next().map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                }
            };
            if let Some(n) = name {
                if !n.is_empty() {
                    edges.push(CodeEdge::from_node(file, &child, kind, n));
                }
            }
        }
    }
}

// ─── Node helpers ───────────────────────────────────────────────────────────

fn node_text(source: &str, node: &tree_sitter::Node) -> String {
    source[node.byte_range()].to_string()
}

fn node_name(source: &str, node: &tree_sitter::Node, field: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|n| node_text(source, &n))
        .filter(|s| !s.is_empty())
}

// ─── Query helpers (used by Tauri commands and future MCP tools) ────────────

/// Look up symbols by name (exact match).
pub fn query_symbols_by_name(
    conn: &Connection,
    repo_id: i64,
    name: &str,
) -> Result<Vec<Symbol>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT name, kind, file, line, end_line, parent
         FROM code_symbols WHERE repo_id = ?1 AND name = ?2",
    )?;
    let rows = stmt.query_map(params![repo_id, name], |row| {
        Ok(Symbol {
            name: row.get(0)?,
            kind: SymbolKind::parse(&row.get::<_, String>(1)?).unwrap_or(SymbolKind::Function),
            file: row.get(2)?,
            line: row.get(3)?,
            end_line: row.get(4)?,
            parent: row.get(5)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Look up all symbols in a file.
pub fn query_symbols_in_file(
    conn: &Connection,
    repo_id: i64,
    file: &str,
) -> Result<Vec<Symbol>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT name, kind, file, line, end_line, parent
         FROM code_symbols WHERE repo_id = ?1 AND file = ?2 ORDER BY line",
    )?;
    let rows = stmt.query_map(params![repo_id, file], |row| {
        Ok(Symbol {
            name: row.get(0)?,
            kind: SymbolKind::parse(&row.get::<_, String>(1)?).unwrap_or(SymbolKind::Function),
            file: row.get(2)?,
            line: row.get(3)?,
            end_line: row.get(4)?,
            parent: row.get(5)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ─── Repo registry ─────────────────────────────────────────────────────────

/// A registered repository with its index status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoEntry {
    pub id: i64,
    pub path: String,
    pub label: String,
    pub indexed_at: i64,
    pub file_count: u32,
    pub symbol_count: u32,
}

/// List all registered (indexed) repositories.
pub fn list_repos(conn: &Connection) -> Result<Vec<RepoEntry>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.path, r.label, r.indexed_at,
                (SELECT COUNT(*) FROM code_file_hashes WHERE repo_id = r.id) AS file_count,
                (SELECT COUNT(*) FROM code_symbols WHERE repo_id = r.id) AS symbol_count
         FROM code_repos r ORDER BY r.label",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RepoEntry {
            id: row.get(0)?,
            path: row.get(1)?,
            label: row.get(2)?,
            indexed_at: row.get(3)?,
            file_count: row.get::<_, u32>(4)?,
            symbol_count: row.get::<_, u32>(5)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Remove a repository and all its associated data (symbols, edges, hashes).
pub fn remove_repo(conn: &Connection, repo_id: i64) -> Result<(), IndexError> {
    // CASCADE on foreign keys handles code_symbols, code_edges, code_file_hashes.
    conn.execute("DELETE FROM code_repos WHERE id = ?1", params![repo_id])?;
    Ok(())
}

/// Get the index freshness: number of files indexed, number stale (on disk but
/// hash differs), and number deleted (in index but not on disk).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexFreshness {
    pub total_indexed: u32,
    pub unchanged: u32,
    pub stale: u32,
    pub deleted: u32,
    pub new_files: u32,
}

/// Check how fresh the index is for a given repo without re-indexing.
pub fn check_freshness(data_dir: &Path, repo_path: &Path) -> Result<IndexFreshness, IndexError> {
    let repo_path = repo_path
        .canonicalize()
        .map_err(|e| IndexError::InvalidPath(format!("{}: {e}", repo_path.display())))?;
    let conn = open_db(data_dir)?;
    let repo_path_str = repo_path.to_string_lossy().to_string();

    let repo_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM code_repos WHERE path = ?1",
            params![repo_path_str],
            |r| r.get(0),
        )
        .ok();

    let repo_id = match repo_id {
        Some(id) => id,
        None => {
            // Never indexed — all files are new.
            let files = collect_source_files(&repo_path);
            return Ok(IndexFreshness {
                total_indexed: 0,
                unchanged: 0,
                stale: 0,
                deleted: 0,
                new_files: files.len() as u32,
            });
        }
    };

    let mut existing_hashes: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    {
        let mut stmt =
            conn.prepare("SELECT file, hash FROM code_file_hashes WHERE repo_id = ?1")?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (file, hash) = row?;
            existing_hashes.insert(file, hash);
        }
    }

    let files = collect_source_files(&repo_path);
    let mut unchanged = 0u32;
    let mut stale = 0u32;
    let mut new_files = 0u32;
    let mut seen: HashSet<String> = HashSet::new();

    for file_path in &files {
        let rel_path = file_path
            .strip_prefix(&repo_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");
        seen.insert(rel_path.clone());

        if let Ok(bytes) = std::fs::read(file_path) {
            let hash = content_hash(&bytes);
            match existing_hashes.get(&rel_path) {
                Some(old_hash) if old_hash == &hash => unchanged += 1,
                Some(_) => stale += 1,
                None => new_files += 1,
            }
        }
    }

    let deleted = existing_hashes
        .keys()
        .filter(|k| !seen.contains(k.as_str()))
        .count() as u32;

    Ok(IndexFreshness {
        total_indexed: existing_hashes.len() as u32,
        unchanged,
        stale,
        deleted,
        new_files,
    })
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_index_rust_file() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("main.rs"),
            r#"
pub struct AppState {
    pub name: String,
}

impl AppState {
    pub fn new() -> Self {
        Self { name: String::new() }
    }
}

pub fn run_http_server(port: u16) {
    println!("listening on {port}");
}

pub fn run_stdio() {
    run_http_server(8080);
}

enum Color {
    Red,
    Blue,
}

const MAX_SIZE: usize = 1024;
"#,
        )
        .unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        let stats = index_repo(&data_dir, &repo).unwrap();
        assert_eq!(stats.files_parsed, 1);
        assert!(stats.symbols_extracted >= 6); // struct + impl + 2 methods + 2 functions + enum + const

        // Query by name.
        let conn = open_db(&data_dir).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        let app_state = query_symbols_by_name(&conn, repo_id, "AppState").unwrap();
        assert!(!app_state.is_empty());
        assert_eq!(app_state[0].kind, SymbolKind::Struct);

        let run_http = query_symbols_by_name(&conn, repo_id, "run_http_server").unwrap();
        assert!(!run_http.is_empty());
        assert_eq!(run_http[0].kind, SymbolKind::Function);
        assert_eq!(run_http[0].line, 12);

        let run_stdio = query_symbols_by_name(&conn, repo_id, "run_stdio").unwrap();
        assert!(!run_stdio.is_empty());

        // Check edges — run_stdio calls run_http_server.
        let edges: Vec<(String, String)> = conn
            .prepare("SELECT from_file, target_name FROM code_edges WHERE repo_id = ?1 AND kind = 'calls'")
            .unwrap()
            .query_map(params![repo_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(edges.iter().any(|(_, t)| t == "run_http_server"));
    }

    #[test]
    fn test_index_typescript_file() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("app.ts"),
            r#"
import { ref, computed } from 'vue'
import { useStore } from './store'

export interface AppConfig {
    name: string;
    port: number;
}

export class Server {
    start() {
        console.log("starting");
    }
    stop() {}
}

export function createApp(config: AppConfig): Server {
    const store = useStore();
    return new Server();
}

const helper = (x: number) => x * 2;

export type UserId = string;

export enum Status {
    Active,
    Inactive,
}
"#,
        )
        .unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        let stats = index_repo(&data_dir, &repo).unwrap();
        assert_eq!(stats.files_parsed, 1);
        assert!(stats.symbols_extracted >= 6);

        let conn = open_db(&data_dir).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        let server = query_symbols_by_name(&conn, repo_id, "Server").unwrap();
        assert!(!server.is_empty());
        assert_eq!(server[0].kind, SymbolKind::Class);

        let create_app = query_symbols_by_name(&conn, repo_id, "createApp").unwrap();
        assert!(!create_app.is_empty());
        assert_eq!(create_app[0].kind, SymbolKind::Function);

        let iface = query_symbols_by_name(&conn, repo_id, "AppConfig").unwrap();
        assert!(!iface.is_empty());
        assert_eq!(iface[0].kind, SymbolKind::Interface);

        // Imports edge.
        let imports: Vec<String> = conn
            .prepare("SELECT target_name FROM code_edges WHERE repo_id = ?1 AND kind = 'imports'")
            .unwrap()
            .query_map(params![repo_id], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(imports.contains(&"ref".to_string()));
        assert!(imports.contains(&"useStore".to_string()));
    }

    #[test]
    fn test_skips_target_and_node_modules() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(repo.join("target")).unwrap();
        std::fs::create_dir_all(repo.join("node_modules")).unwrap();
        std::fs::create_dir_all(repo.join("src")).unwrap();

        std::fs::write(repo.join("target/main.rs"), "fn hidden() {}").unwrap();
        std::fs::write(repo.join("node_modules/lib.ts"), "function hidden() {}").unwrap();
        std::fs::write(repo.join("src/app.rs"), "pub fn visible() {}").unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        let stats = index_repo(&data_dir, &repo).unwrap();
        assert_eq!(stats.files_parsed, 1);
        assert_eq!(stats.symbols_extracted, 1);
    }

    #[test]
    fn test_incremental_skips_unchanged_files() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(repo.join("lib.rs"), "pub fn alpha() {}\npub fn beta() {}").unwrap();
        std::fs::write(repo.join("util.rs"), "pub fn gamma() {}").unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        // First run: parses all files.
        let stats1 = index_repo(&data_dir, &repo).unwrap();
        assert_eq!(stats1.files_parsed, 2);
        assert_eq!(stats1.files_skipped, 0);
        assert_eq!(stats1.symbols_extracted, 3);

        // Second run without changes: all files skipped.
        let stats2 = index_repo(&data_dir, &repo).unwrap();
        assert_eq!(stats2.files_parsed, 0);
        assert_eq!(stats2.files_skipped, 2);
        assert_eq!(stats2.symbols_extracted, 0);

        // Modify one file.
        std::fs::write(
            repo.join("lib.rs"),
            "pub fn alpha() {}\npub fn beta() {}\npub fn delta() {}",
        )
        .unwrap();

        // Third run: only modified file re-parsed.
        let stats3 = index_repo(&data_dir, &repo).unwrap();
        assert_eq!(stats3.files_parsed, 1);
        assert_eq!(stats3.files_skipped, 1);
        assert_eq!(stats3.symbols_extracted, 3); // alpha, beta, delta

        // Verify the new symbol exists.
        let conn = open_db(&data_dir).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();
        let delta = query_symbols_by_name(&conn, repo_id, "delta").unwrap();
        assert_eq!(delta.len(), 1);
    }

    #[test]
    fn test_incremental_handles_deleted_files() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(repo.join("keep.rs"), "pub fn keeper() {}").unwrap();
        std::fs::write(repo.join("remove.rs"), "pub fn doomed() {}").unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        let stats1 = index_repo(&data_dir, &repo).unwrap();
        assert_eq!(stats1.files_parsed, 2);

        // Delete one file.
        std::fs::remove_file(repo.join("remove.rs")).unwrap();

        let stats2 = index_repo(&data_dir, &repo).unwrap();
        assert_eq!(stats2.files_deleted, 1);
        assert_eq!(stats2.files_skipped, 1);
        assert_eq!(stats2.files_parsed, 0);

        // Verify doomed symbol is gone.
        let conn = open_db(&data_dir).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();
        let doomed = query_symbols_by_name(&conn, repo_id, "doomed").unwrap();
        assert!(doomed.is_empty());
    }

    #[test]
    fn test_list_repos_and_freshness() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(repo.join("main.rs"), "pub fn entry() {}").unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        index_repo(&data_dir, &repo).unwrap();

        let conn = open_db(&data_dir).unwrap();
        let repos = list_repos(&conn).unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].label, "repo");
        assert_eq!(repos[0].file_count, 1);
        assert_eq!(repos[0].symbol_count, 1);

        // Freshness: all unchanged.
        let fresh = check_freshness(&data_dir, &repo).unwrap();
        assert_eq!(fresh.unchanged, 1);
        assert_eq!(fresh.stale, 0);
        assert_eq!(fresh.new_files, 0);
        assert_eq!(fresh.deleted, 0);

        // Modify file → stale.
        std::fs::write(repo.join("main.rs"), "pub fn entry() {}\npub fn extra() {}").unwrap();
        let fresh2 = check_freshness(&data_dir, &repo).unwrap();
        assert_eq!(fresh2.stale, 1);
        assert_eq!(fresh2.unchanged, 0);
    }

    #[test]
    fn test_rust_heritage_edges() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("lib.rs"),
            r#"
pub trait Renderer {
    fn render(&self);
}

pub struct VrmRenderer;

impl Renderer for VrmRenderer {
    fn render(&self) {}
}

impl VrmRenderer {
    pub fn new() -> Self { Self }
}
"#,
        )
        .unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        index_repo(&data_dir, &repo).unwrap();

        let conn = open_db(&data_dir).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        // Should have an Implements edge: VrmRenderer implements Renderer.
        let impl_edges: Vec<String> = conn
            .prepare(
                "SELECT target_name FROM code_edges WHERE repo_id = ?1 AND kind = 'implements'",
            )
            .unwrap()
            .query_map(params![repo_id], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            impl_edges.contains(&"Renderer".to_string()),
            "Expected Implements edge for Renderer, got: {impl_edges:?}"
        );
    }

    #[test]
    fn test_rust_reexport_edges() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("lib.rs"),
            r#"
pub use crate::inner::Widget;
use std::collections::HashMap;

mod inner {
    pub struct Widget;
}
"#,
        )
        .unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        index_repo(&data_dir, &repo).unwrap();

        let conn = open_db(&data_dir).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        // `pub use` → ReExports edge.
        let reexport_edges: Vec<String> = conn
            .prepare(
                "SELECT target_name FROM code_edges WHERE repo_id = ?1 AND kind = 're_exports'",
            )
            .unwrap()
            .query_map(params![repo_id], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            reexport_edges.contains(&"Widget".to_string()),
            "Expected ReExports edge for Widget, got: {reexport_edges:?}"
        );

        // `use std::collections::HashMap` → plain Imports edge.
        let import_edges: Vec<String> = conn
            .prepare("SELECT target_name FROM code_edges WHERE repo_id = ?1 AND kind = 'imports'")
            .unwrap()
            .query_map(params![repo_id], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            import_edges.contains(&"HashMap".to_string()),
            "Expected Imports edge for HashMap, got: {import_edges:?}"
        );
    }

    #[test]
    fn test_typescript_heritage_edges() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("app.ts"),
            r#"
interface Disposable {
    dispose(): void;
}

class BaseComponent {
    init() {}
}

class MyComponent extends BaseComponent implements Disposable {
    dispose() {}
}
"#,
        )
        .unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        index_repo(&data_dir, &repo).unwrap();

        let conn = open_db(&data_dir).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        let extends_edges: Vec<String> = conn
            .prepare("SELECT target_name FROM code_edges WHERE repo_id = ?1 AND kind = 'extends'")
            .unwrap()
            .query_map(params![repo_id], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            extends_edges.contains(&"BaseComponent".to_string()),
            "Expected Extends edge for BaseComponent, got: {extends_edges:?}"
        );

        let impl_edges: Vec<String> = conn
            .prepare(
                "SELECT target_name FROM code_edges WHERE repo_id = ?1 AND kind = 'implements'",
            )
            .unwrap()
            .query_map(params![repo_id], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            impl_edges.contains(&"Disposable".to_string()),
            "Expected Implements edge for Disposable, got: {impl_edges:?}"
        );
    }

    #[test]
    fn test_typescript_reexport_edges() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("index.ts"),
            r#"
export { Widget } from './widget'
export { Helper, Utils } from './utils'
import { internal } from './internal'
"#,
        )
        .unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        index_repo(&data_dir, &repo).unwrap();

        let conn = open_db(&data_dir).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        let reexport_edges: Vec<String> = conn
            .prepare(
                "SELECT target_name FROM code_edges WHERE repo_id = ?1 AND kind = 're_exports'",
            )
            .unwrap()
            .query_map(params![repo_id], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            reexport_edges.contains(&"Widget".to_string()),
            "Expected ReExports edge for Widget, got: {reexport_edges:?}"
        );
        assert!(
            reexport_edges.contains(&"Helper".to_string()),
            "Expected ReExports edge for Helper, got: {reexport_edges:?}"
        );
    }
}
