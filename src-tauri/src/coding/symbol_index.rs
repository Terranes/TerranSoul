//! Tree-sitter powered symbol-table ingest pipeline (Chunk 31.3).
//!
//! Walks a repository path, parses Rust and TypeScript files via tree-sitter,
//! extracts function/class/method/struct/enum nodes with file+line+symbol kind,
//! and stores them in a `code_symbols` SQLite table. Also extracts imports and
//! call sites (best-effort, no cross-file resolution yet) into `code_edges`.
//!
//! The SQLite database lives alongside the memory store at
//! `<data_dir>/code_index.sqlite`.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
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
}

impl EdgeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Imports => "imports",
            Self::Calls => "calls",
        }
    }
}

/// A best-effort edge (import or call site).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEdge {
    pub from_file: String,
    pub from_line: u32,
    pub kind: EdgeKind,
    /// The imported/called name (unresolved — just the identifier text).
    pub target_name: String,
}

/// Stats returned from an indexing run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub files_parsed: u32,
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
            kind        TEXT NOT NULL,
            target_name TEXT NOT NULL,
            target_file TEXT,
            target_symbol_id INTEGER REFERENCES code_symbols(id) ON DELETE SET NULL,
            from_symbol_id   INTEGER REFERENCES code_symbols(id) ON DELETE SET NULL,
            confidence  TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_code_edges_target ON code_edges(target_name);
        CREATE INDEX IF NOT EXISTS idx_code_edges_file ON code_edges(from_file);
        CREATE INDEX IF NOT EXISTS idx_code_edges_target_sym ON code_edges(target_symbol_id);
        CREATE INDEX IF NOT EXISTS idx_code_edges_from_sym ON code_edges(from_symbol_id);
        "#,
    )?;
    Ok(())
}

// ─── Indexing pipeline ──────────────────────────────────────────────────────

/// Index an entire repository. Walks Rust (`.rs`) and TypeScript (`.ts`/`.tsx`)
/// files, parses them with tree-sitter, and stores symbols + edges.
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

    // Clear existing data for this repo.
    conn.execute("DELETE FROM code_symbols WHERE repo_id = ?1", params![repo_id])?;
    conn.execute("DELETE FROM code_edges WHERE repo_id = ?1", params![repo_id])?;

    // Collect files.
    let files = collect_source_files(&repo_path);

    let mut stats = IndexStats {
        files_parsed: 0,
        symbols_extracted: 0,
        edges_extracted: 0,
        errors: Vec::new(),
    };

    // Parse each file.
    let mut rust_parser = new_rust_parser();
    let mut ts_parser = new_typescript_parser();

    let tx = conn.unchecked_transaction()?;

    for file_path in &files {
        let rel_path = file_path
            .strip_prefix(&repo_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");

        let source = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                stats.errors.push(format!("{rel_path}: {e}"));
                continue;
            }
        };

        let is_rust = file_path
            .extension()
            .map(|e| e == "rs")
            .unwrap_or(false);

        let parser = if is_rust {
            &mut rust_parser
        } else {
            &mut ts_parser
        };

        let tree = match parser.parse(&source, None) {
            Some(t) => t,
            None => {
                stats.errors.push(format!("{rel_path}: tree-sitter parse returned None"));
                continue;
            }
        };

        let (symbols, edges) = if is_rust {
            extract_rust_symbols(&source, tree.root_node(), &rel_path)
        } else {
            extract_ts_symbols(&source, tree.root_node(), &rel_path)
        };

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
                "INSERT INTO code_edges (repo_id, from_file, from_line, kind, target_name)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    repo_id,
                    edge.from_file,
                    edge.from_line,
                    edge.kind.as_str(),
                    edge.target_name,
                ],
            )?;
        }

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
            if ext == "rs" || ext == "ts" || ext == "tsx" {
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
            let impl_name = node
                .child_by_field_name("type")
                .and_then(|t| {
                    // Could be a simple type_identifier or a generic_type.
                    if t.kind() == "type_identifier" {
                        Some(node_text(source, &t))
                    } else {
                        t.child_by_field_name("type").map(|inner| node_text(source, &inner))
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
            if let Some(arg) = node.child_by_field_name("argument") {
                let text = node_text(source, &arg);
                // Extract last segment as the target name.
                let target = text.rsplit("::").next().unwrap_or(&text);
                if !target.is_empty() && target != "*" {
                    edges.push(CodeEdge {
                        from_file: file.to_string(),
                        from_line: node.start_position().row as u32 + 1,
                        kind: EdgeKind::Imports,
                        target_name: target.to_string(),
                    });
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
                    edges.push(CodeEdge {
                        from_file: file.to_string(),
                        from_line: node.start_position().row as u32 + 1,
                        kind: EdgeKind::Calls,
                        target_name: target.to_string(),
                    });
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
                                if value.kind() == "arrow_function"
                                    || value.kind() == "function"
                                {
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
                        extract_ts_import_names(source, child, file, &mut *edges, node.start_position().row as u32 + 1);
                    }
                }
            }
        }
        "call_expression" => {
            if let Some(func) = node.child_by_field_name("function") {
                let text = node_text(source, &func);
                let target = text.rsplit('.').next().unwrap_or(&text);
                if !target.is_empty() && target.len() < 100 && !target.starts_with('(') {
                    edges.push(CodeEdge {
                        from_file: file.to_string(),
                        from_line: node.start_position().row as u32 + 1,
                        kind: EdgeKind::Calls,
                        target_name: target.to_string(),
                    });
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
            edges.push(CodeEdge {
                from_file: file.to_string(),
                from_line: line,
                kind: EdgeKind::Imports,
                target_name: name,
            });
        }
        return;
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_ts_import_names(source, child, file, edges, line);
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
}
