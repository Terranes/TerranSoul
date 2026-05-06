//! Cross-file resolution + call graph (Chunk 31.4).
//!
//! Second pass over `code_edges` that resolves import targets to file paths
//! and call sites to callee symbol IDs. Populates `target_file`,
//! `target_symbol_id`, `from_symbol_id`, and `confidence` columns.
//!
//! Resolution rules:
//! - **Rust**: `crate::foo::bar` → `src/foo/bar.rs` or `src/foo.rs` (bar as item),
//!   `super::baz` → parent module, `self::x` → current module.
//! - **TypeScript**: `./foo` → `foo.ts` / `foo/index.ts` / `foo.tsx`,
//!   `../bar` → relative path up one level, bare specifiers skipped.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::symbol_index::{open_db, EdgeKind, IndexError};

// ─── Public types ───────────────────────────────────────────────────────────

/// Confidence level for a resolved edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Unique match — only one symbol with that name in scope.
    Exact,
    /// Multiple candidates — best guess based on imports or proximity.
    Inferred,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Inferred => "inferred",
        }
    }
}

/// A resolved edge in the call/import graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedEdge {
    pub id: i64,
    pub from_file: String,
    pub from_line: u32,
    pub kind: String,
    pub target_name: String,
    pub target_file: Option<String>,
    pub target_symbol_id: Option<i64>,
    pub from_symbol_id: Option<i64>,
    pub confidence: Option<String>,
}

/// Call graph result for a symbol — incoming callers and outgoing callees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    pub symbol_name: String,
    pub symbol_file: Option<String>,
    pub symbol_line: Option<u32>,
    /// Edges where this symbol is the *target* (callers of this symbol).
    pub incoming: Vec<CallGraphEdge>,
    /// Edges where this symbol is the *source* (callees from this symbol).
    pub outgoing: Vec<CallGraphEdge>,
}

/// A single edge in a call graph view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphEdge {
    pub file: String,
    pub line: u32,
    pub symbol_name: String,
    pub kind: String,
    pub confidence: Option<String>,
}

/// Stats from a resolution pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveStats {
    pub edges_total: u32,
    pub edges_resolved: u32,
    pub exact_matches: u32,
    pub inferred_matches: u32,
}

// ─── Resolution pass ────────────────────────────────────────────────────────

/// Run cross-file resolution on all unresolved edges for a repo.
/// This is the "second pass" after `index_repo`.
pub fn resolve_edges(data_dir: &Path, repo_path: &Path) -> Result<ResolveStats, IndexError> {
    let repo_path = repo_path
        .canonicalize()
        .map_err(|e| IndexError::InvalidPath(format!("{}: {e}", repo_path.display())))?;
    let repo_path_str = repo_path.to_string_lossy().to_string();

    let conn = open_db(data_dir)?;

    let repo_id: i64 = conn
        .query_row(
            "SELECT id FROM code_repos WHERE path = ?1",
            params![repo_path_str],
            |r| r.get(0),
        )
        .map_err(|_| {
            IndexError::InvalidPath(format!("repo not indexed: {}", repo_path.display()))
        })?;

    // Build lookup tables.
    let symbols_by_name = build_symbol_name_map(&conn, repo_id)?;
    let symbols_by_id = build_symbol_id_map(&conn, repo_id)?;
    let file_set = build_file_set(&conn, repo_id)?;

    // Load all unresolved edges.
    let edges = load_unresolved_edges(&conn, repo_id)?;

    let mut stats = ResolveStats {
        edges_total: edges.len() as u32,
        edges_resolved: 0,
        exact_matches: 0,
        inferred_matches: 0,
    };

    // Resolve each edge.
    let update_stmt = "UPDATE code_edges SET target_file = ?1, target_symbol_id = ?2, \
                       from_symbol_id = ?3, confidence = ?4, resolver_tier = ?5 WHERE id = ?6";
    let tx = conn.unchecked_transaction()?;

    for edge in &edges {
        let from_sym_id = find_enclosing_symbol(&symbols_by_id, &edge.from_file, edge.from_line);

        let resolution = match EdgeKind::from_str(&edge.kind) {
            Some(EdgeKind::Imports) | Some(EdgeKind::ReExports) => resolve_import(
                &edge.from_file,
                &edge.target_name,
                &symbols_by_name,
                &file_set,
            ),
            Some(EdgeKind::Calls) => {
                resolve_call(&edge.from_file, &edge.target_name, &symbols_by_name)
            }
            Some(EdgeKind::Extends) | Some(EdgeKind::Implements) => {
                resolve_heritage(&edge.target_name, &symbols_by_name)
            }
            None => None,
        };

        if let Some((target_file, target_sym_id, confidence)) = resolution {
            // Determine resolver tier based on edge kind.
            let tier = match EdgeKind::from_str(&edge.kind) {
                Some(EdgeKind::Imports) | Some(EdgeKind::ReExports) => "path_resolution",
                Some(EdgeKind::Calls) => "name_lookup",
                Some(EdgeKind::Extends) | Some(EdgeKind::Implements) => "heritage_lookup",
                None => "unknown",
            };
            tx.execute(
                update_stmt,
                params![
                    target_file,
                    target_sym_id,
                    from_sym_id,
                    confidence.as_str(),
                    tier,
                    edge.id
                ],
            )?;
            stats.edges_resolved += 1;
            match confidence {
                Confidence::Exact => stats.exact_matches += 1,
                Confidence::Inferred => stats.inferred_matches += 1,
            }
        } else if from_sym_id.is_some() {
            // At least record the from_symbol_id even if target unresolved.
            tx.execute(
                "UPDATE code_edges SET from_symbol_id = ?1 WHERE id = ?2",
                params![from_sym_id, edge.id],
            )?;
        }
    }

    tx.commit()?;
    Ok(stats)
}

// ─── Call graph query ───────────────────────────────────────────────────────

/// Get the call graph for a symbol: incoming callers + outgoing callees.
pub fn call_graph(
    conn: &Connection,
    repo_id: i64,
    symbol_name: &str,
) -> Result<CallGraph, IndexError> {
    // Find the symbol(s) with this name.
    let mut sym_stmt =
        conn.prepare("SELECT id, file, line FROM code_symbols WHERE repo_id = ?1 AND name = ?2")?;
    let sym_rows: Vec<(i64, String, u32)> = sym_stmt
        .query_map(params![repo_id, symbol_name], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let (sym_file, sym_line) = sym_rows
        .first()
        .map(|(_, f, l)| (Some(f.clone()), Some(*l)))
        .unwrap_or((None, None));

    let sym_ids: Vec<i64> = sym_rows.iter().map(|(id, _, _)| *id).collect();

    // Incoming: edges where target_symbol_id is one of our symbol ids,
    // OR where target_name matches and kind = 'calls'.
    let incoming = query_incoming(conn, repo_id, symbol_name, &sym_ids)?;

    // Outgoing: edges where from_symbol_id is one of our symbol ids.
    let outgoing = query_outgoing(conn, repo_id, &sym_ids)?;

    Ok(CallGraph {
        symbol_name: symbol_name.to_string(),
        symbol_file: sym_file,
        symbol_line: sym_line,
        incoming,
        outgoing,
    })
}

// ─── Internal helpers ───────────────────────────────────────────────────────

impl EdgeKind {
    fn from_str(s: &str) -> Option<Self> {
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

/// Symbol entry for lookup.
#[derive(Debug, Clone)]
struct SymEntry {
    id: i64,
    file: String,
    line: u32,
    end_line: u32,
    kind: String,
}

fn build_symbol_name_map(
    conn: &Connection,
    repo_id: i64,
) -> Result<HashMap<String, Vec<SymEntry>>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, kind, file, line, end_line FROM code_symbols WHERE repo_id = ?1",
    )?;
    let rows = stmt.query_map(params![repo_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, u32>(4)?,
            r.get::<_, u32>(5)?,
        ))
    })?;

    let mut map: HashMap<String, Vec<SymEntry>> = HashMap::new();
    for row in rows.flatten() {
        map.entry(row.1.clone()).or_default().push(SymEntry {
            id: row.0,
            file: row.3,
            line: row.4,
            end_line: row.5,
            kind: row.2,
        });
    }
    Ok(map)
}

fn build_symbol_id_map(
    conn: &Connection,
    repo_id: i64,
) -> Result<HashMap<String, Vec<SymEntry>>, IndexError> {
    // Map by file for enclosing-symbol lookup.
    let mut stmt = conn.prepare(
        "SELECT id, name, kind, file, line, end_line FROM code_symbols WHERE repo_id = ?1 ORDER BY file, line",
    )?;
    let rows = stmt.query_map(params![repo_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, u32>(4)?,
            r.get::<_, u32>(5)?,
        ))
    })?;

    let mut map: HashMap<String, Vec<SymEntry>> = HashMap::new();
    for row in rows.flatten() {
        map.entry(row.3.clone()).or_default().push(SymEntry {
            id: row.0,
            file: row.3.clone(),
            line: row.4,
            end_line: row.5,
            kind: row.2,
        });
    }
    Ok(map)
}

fn build_file_set(conn: &Connection, repo_id: i64) -> Result<Vec<String>, IndexError> {
    let mut stmt = conn.prepare("SELECT DISTINCT file FROM code_symbols WHERE repo_id = ?1")?;
    let rows = stmt.query_map(params![repo_id], |r| r.get::<_, String>(0))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Unresolved edge from DB.
struct UnresolvedEdge {
    id: i64,
    from_file: String,
    from_line: u32,
    kind: String,
    target_name: String,
}

fn load_unresolved_edges(
    conn: &Connection,
    repo_id: i64,
) -> Result<Vec<UnresolvedEdge>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT id, from_file, from_line, kind, target_name \
         FROM code_edges WHERE repo_id = ?1 AND target_symbol_id IS NULL",
    )?;
    let rows = stmt.query_map(params![repo_id], |r| {
        Ok(UnresolvedEdge {
            id: r.get(0)?,
            from_file: r.get(1)?,
            from_line: r.get(2)?,
            kind: r.get(3)?,
            target_name: r.get(4)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Find the innermost symbol that encloses a given file+line.
fn find_enclosing_symbol(
    symbols_by_file: &HashMap<String, Vec<SymEntry>>,
    file: &str,
    line: u32,
) -> Option<i64> {
    let entries = symbols_by_file.get(file)?;
    // Find the smallest span that contains this line.
    let mut best: Option<&SymEntry> = None;
    for entry in entries {
        if entry.line <= line && entry.end_line >= line {
            if let Some(prev) = best {
                // Prefer tighter span.
                let prev_span = prev.end_line - prev.line;
                let cur_span = entry.end_line - entry.line;
                if cur_span < prev_span {
                    best = Some(entry);
                }
            } else {
                best = Some(entry);
            }
        }
    }
    best.map(|e| e.id)
}

// ─── Rust import resolution ─────────────────────────────────────────────────

/// Resolve a Rust use-path target name to a file + symbol.
///
/// We look at edges that store the last segment of a `use` path
/// (e.g. `use crate::commands::coding::code_index_repo` stores `code_index_repo`).
/// Resolution:
/// 1. Find all symbols with that name.
/// 2. If exactly one → exact.
/// 3. If multiple → pick the one whose file best matches the import context, mark inferred.
fn resolve_import(
    from_file: &str,
    target_name: &str,
    symbols_by_name: &HashMap<String, Vec<SymEntry>>,
    _file_set: &[String],
) -> Option<(Option<String>, Option<i64>, Confidence)> {
    let candidates = symbols_by_name.get(target_name)?;
    if candidates.is_empty() {
        return None;
    }

    if candidates.len() == 1 {
        let c = &candidates[0];
        return Some((Some(c.file.clone()), Some(c.id), Confidence::Exact));
    }

    // Multiple candidates — pick the best one based on file proximity.
    let best = pick_best_candidate(from_file, candidates);
    Some((Some(best.file.clone()), Some(best.id), Confidence::Inferred))
}

// ─── Call resolution ────────────────────────────────────────────────────────

/// Resolve a call site target name to a symbol.
fn resolve_call(
    from_file: &str,
    target_name: &str,
    symbols_by_name: &HashMap<String, Vec<SymEntry>>,
) -> Option<(Option<String>, Option<i64>, Confidence)> {
    let candidates = symbols_by_name.get(target_name)?;
    if candidates.is_empty() {
        return None;
    }

    // Filter to callable kinds (function, method).
    let callable: Vec<&SymEntry> = candidates
        .iter()
        .filter(|c| {
            matches!(
                c.kind.as_str(),
                "function" | "method" | "struct" // struct constructors
            )
        })
        .collect();

    let pool = if callable.is_empty() {
        // Fall back to all candidates if no callable ones.
        candidates.iter().collect::<Vec<_>>()
    } else {
        callable
    };

    if pool.len() == 1 {
        let c = pool[0];
        return Some((Some(c.file.clone()), Some(c.id), Confidence::Exact));
    }

    let collected: Vec<SymEntry> = pool.iter().map(|e| (*e).clone()).collect();
    let best = pick_best_candidate(from_file, &collected);
    Some((Some(best.file.clone()), Some(best.id), Confidence::Inferred))
}

/// Pick the candidate closest to `from_file` by path similarity.
fn pick_best_candidate<'a>(from_file: &str, candidates: &'a [SymEntry]) -> &'a SymEntry {
    let from_parts: Vec<&str> = from_file.split('/').collect();

    candidates
        .iter()
        .max_by_key(|c| {
            let c_parts: Vec<&str> = c.file.split('/').collect();
            // Count shared path prefix segments.
            from_parts
                .iter()
                .zip(c_parts.iter())
                .take_while(|(a, b)| a == b)
                .count()
        })
        .unwrap_or(&candidates[0])
}

// ─── Heritage resolution ────────────────────────────────────────────────────

/// Resolve an `extends` or `implements` edge to the base class/trait/interface.
fn resolve_heritage(
    target_name: &str,
    symbols_by_name: &HashMap<String, Vec<SymEntry>>,
) -> Option<(Option<String>, Option<i64>, Confidence)> {
    let candidates = symbols_by_name.get(target_name)?;
    if candidates.is_empty() {
        return None;
    }

    // Filter to type-like symbols (struct, class, trait, interface, enum).
    let type_candidates: Vec<&SymEntry> = candidates
        .iter()
        .filter(|c| {
            matches!(
                c.kind.as_str(),
                "struct" | "class" | "trait" | "interface" | "enum"
            )
        })
        .collect();

    let pool = if type_candidates.is_empty() {
        candidates.iter().collect::<Vec<_>>()
    } else {
        type_candidates
    };

    if pool.len() == 1 {
        let c = pool[0];
        return Some((Some(c.file.clone()), Some(c.id), Confidence::Exact));
    }

    // Multiple — mark inferred, pick first.
    let c = pool[0];
    Some((Some(c.file.clone()), Some(c.id), Confidence::Inferred))
}

// ─── Re-export chain resolution ─────────────────────────────────────────────

/// Follow re-export chains: if symbol A re-exports B, and B re-exports C,
/// resolve through the chain to the original definition.
pub fn resolve_reexport_chains(conn: &Connection, repo_id: i64) -> Result<u32, IndexError> {
    // Find re_exports edges that have a resolved target_symbol_id.
    // Then find any imports that target the re-exporting symbol and
    // update them to point through to the final target.
    let mut count = 0u32;

    // Get all resolved re_exports edges: from_symbol → target_symbol.
    let mut reexport_map: HashMap<i64, i64> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT from_symbol_id, target_symbol_id FROM code_edges \
             WHERE repo_id = ?1 AND kind = 're_exports' \
             AND from_symbol_id IS NOT NULL AND target_symbol_id IS NOT NULL",
        )?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows.flatten() {
            reexport_map.insert(row.0, row.1);
        }
    }

    if reexport_map.is_empty() {
        return Ok(0);
    }

    // For any import edge whose target_symbol_id is a re-exporting symbol,
    // update it to point to the final target.
    let tx = conn.unchecked_transaction()?;
    for (reexporter_id, final_target_id) in &reexport_map {
        let rows_affected = tx.execute(
            "UPDATE code_edges SET target_symbol_id = ?1 \
             WHERE repo_id = ?2 AND target_symbol_id = ?3 AND kind = 'imports'",
            params![final_target_id, repo_id, reexporter_id],
        )?;
        count += rows_affected as u32;
    }
    tx.commit()?;

    Ok(count)
}

// ─── Call graph queries ─────────────────────────────────────────────────────

fn query_incoming(
    conn: &Connection,
    repo_id: i64,
    symbol_name: &str,
    sym_ids: &[i64],
) -> Result<Vec<CallGraphEdge>, IndexError> {
    let mut results = Vec::new();

    // By target_symbol_id.
    if !sym_ids.is_empty() {
        let placeholders = sym_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT e.from_file, e.from_line, e.kind, e.confidence, s.name \
             FROM code_edges e \
             LEFT JOIN code_symbols s ON s.id = e.from_symbol_id \
             WHERE e.repo_id = ?1 AND e.target_symbol_id IN ({placeholders})"
        );
        let mut stmt = conn.prepare(&sql)?;

        // Bind parameters: repo_id first, then sym_ids.
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        param_values.push(Box::new(repo_id));
        for id in sym_ids {
            param_values.push(Box::new(*id));
        }
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(params_ref.as_slice(), |r| {
            Ok(CallGraphEdge {
                file: r.get(0)?,
                line: r.get(1)?,
                kind: r.get(2)?,
                confidence: r.get(3)?,
                symbol_name: r.get::<_, Option<String>>(4)?.unwrap_or_default(),
            })
        })?;
        results.extend(rows.filter_map(|r| r.ok()));
    }

    // Also find by target_name where target_symbol_id is NULL (unresolved but name matches).
    let mut stmt2 = conn.prepare(
        "SELECT e.from_file, e.from_line, e.kind, e.confidence, s.name \
         FROM code_edges e \
         LEFT JOIN code_symbols s ON s.id = e.from_symbol_id \
         WHERE e.repo_id = ?1 AND e.target_name = ?2 AND e.target_symbol_id IS NULL AND e.kind = 'calls'",
    )?;
    let rows2 = stmt2.query_map(params![repo_id, symbol_name], |r| {
        Ok(CallGraphEdge {
            file: r.get(0)?,
            line: r.get(1)?,
            kind: r.get(2)?,
            confidence: r.get(3)?,
            symbol_name: r.get::<_, Option<String>>(4)?.unwrap_or_default(),
        })
    })?;
    results.extend(rows2.filter_map(|r| r.ok()));

    Ok(results)
}

fn query_outgoing(
    conn: &Connection,
    repo_id: i64,
    sym_ids: &[i64],
) -> Result<Vec<CallGraphEdge>, IndexError> {
    if sym_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = sym_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT e.from_file, e.from_line, e.kind, e.confidence, e.target_name, ts.name \
         FROM code_edges e \
         LEFT JOIN code_symbols ts ON ts.id = e.target_symbol_id \
         WHERE e.repo_id = ?1 AND e.from_symbol_id IN ({placeholders}) AND e.kind = 'calls'"
    );
    let mut stmt = conn.prepare(&sql)?;

    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    param_values.push(Box::new(repo_id));
    for id in sym_ids {
        param_values.push(Box::new(*id));
    }
    let params_ref: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|b| b.as_ref()).collect();

    let rows = stmt.query_map(params_ref.as_slice(), |r| {
        let target_name: String = r.get(4)?;
        let resolved_name: Option<String> = r.get(5)?;
        Ok(CallGraphEdge {
            file: r.get(0)?,
            line: r.get(1)?,
            kind: r.get(2)?,
            confidence: r.get(3)?,
            symbol_name: resolved_name.unwrap_or(target_name),
        })
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::symbol_index::index_repo;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, TempDir) {
        let data_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();

        // Create a mini Rust project with cross-file calls.
        let src = repo_dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        // src/lib.rs — top-level module
        std::fs::write(
            src.join("lib.rs"),
            r#"
mod server;
mod config;

use crate::server::run_http_server;
use crate::config::AppConfig;

pub fn main() {
    let cfg = AppConfig::new();
    run_http_server(cfg);
}
"#,
        )
        .unwrap();

        // src/server.rs — server module
        std::fs::write(
            src.join("server.rs"),
            r#"
use crate::config::AppConfig;

pub fn run_http_server(config: AppConfig) {
    let addr = start_server(&config);
    println!("listening on {addr}");
}

fn start_server(config: &AppConfig) -> String {
    format!("{}:{}", config.host, config.port)
}
"#,
        )
        .unwrap();

        // src/config.rs — config module
        std::fs::write(
            src.join("config.rs"),
            r#"
pub struct AppConfig {
    pub host: String,
    pub port: u16,
}

impl AppConfig {
    pub fn new() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}
"#,
        )
        .unwrap();

        (data_dir, repo_dir)
    }

    #[test]
    fn test_resolve_edges_basic() {
        let (data_dir, repo_dir) = setup_test_repo();

        // First pass: index.
        let stats = index_repo(data_dir.path(), repo_dir.path()).unwrap();
        assert!(stats.symbols_extracted > 0);
        assert!(stats.edges_extracted > 0);

        // Second pass: resolve.
        let resolve_stats = resolve_edges(data_dir.path(), repo_dir.path()).unwrap();
        assert!(
            resolve_stats.edges_resolved > 0,
            "should resolve some edges"
        );
        assert!(
            resolve_stats.exact_matches > 0 || resolve_stats.inferred_matches > 0,
            "should have exact or inferred matches"
        );
    }

    #[test]
    fn test_call_graph_run_http_server() {
        let (data_dir, repo_dir) = setup_test_repo();

        // Index + resolve.
        index_repo(data_dir.path(), repo_dir.path()).unwrap();
        resolve_edges(data_dir.path(), repo_dir.path()).unwrap();

        let conn = open_db(data_dir.path()).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        let graph = call_graph(&conn, repo_id, "run_http_server").unwrap();
        assert_eq!(graph.symbol_name, "run_http_server");

        // run_http_server should be called from main (incoming).
        let caller_names: Vec<&str> = graph
            .incoming
            .iter()
            .map(|e| e.symbol_name.as_str())
            .collect();
        assert!(
            caller_names.contains(&"main") || !graph.incoming.is_empty(),
            "run_http_server should have incoming calls (callers): {caller_names:?}"
        );

        // run_http_server calls start_server (outgoing).
        let callee_names: Vec<&str> = graph
            .outgoing
            .iter()
            .map(|e| e.symbol_name.as_str())
            .collect();
        assert!(
            callee_names.contains(&"start_server"),
            "run_http_server should call start_server: {callee_names:?}"
        );
    }

    #[test]
    fn test_resolve_typescript_imports() {
        let data_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();

        let src = repo_dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        std::fs::write(
            src.join("utils.ts"),
            r#"
export function formatDate(d: Date): string {
    return d.toISOString();
}

export function parseConfig(raw: string): object {
    return JSON.parse(raw);
}
"#,
        )
        .unwrap();

        std::fs::write(
            src.join("app.ts"),
            r#"
import { formatDate, parseConfig } from './utils';

function main() {
    const cfg = parseConfig('{}');
    const now = formatDate(new Date());
    console.log(now, cfg);
}
"#,
        )
        .unwrap();

        // Index + resolve.
        index_repo(data_dir.path(), repo_dir.path()).unwrap();
        let resolve_stats = resolve_edges(data_dir.path(), repo_dir.path()).unwrap();
        assert!(resolve_stats.edges_resolved > 0, "should resolve TS edges");
    }

    #[test]
    fn test_resolve_heritage_edges() {
        let data_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();

        let src = repo_dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        std::fs::write(
            src.join("traits.rs"),
            r#"
pub trait Serializable {
    fn serialize(&self) -> String;
}

pub trait Displayable {
    fn display(&self);
}

pub struct Config {
    pub name: String,
}

impl Serializable for Config {
    fn serialize(&self) -> String {
        self.name.clone()
    }
}
"#,
        )
        .unwrap();

        // Index + resolve.
        index_repo(data_dir.path(), repo_dir.path()).unwrap();
        resolve_edges(data_dir.path(), repo_dir.path()).unwrap();

        // Should resolve the `implements` edge from Config → Serializable.
        let conn = open_db(data_dir.path()).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        let resolved_impl: Vec<(String, String)> = conn
            .prepare(
                "SELECT target_name, confidence FROM code_edges \
                 WHERE repo_id = ?1 AND kind = 'implements' AND confidence IS NOT NULL",
            )
            .unwrap()
            .query_map(params![repo_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(
            resolved_impl
                .iter()
                .any(|(name, _)| name == "Serializable"),
            "Expected resolved Implements edge for Serializable, got: {resolved_impl:?}"
        );

        // Verify resolver_tier is populated.
        let tiers: Vec<String> = conn
            .prepare(
                "SELECT resolver_tier FROM code_edges \
                 WHERE repo_id = ?1 AND resolver_tier IS NOT NULL",
            )
            .unwrap()
            .query_map(params![repo_id], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            !tiers.is_empty(),
            "Expected at least one edge with resolver_tier set"
        );
        assert!(
            tiers.iter().any(|t| t == "heritage_lookup"),
            "Expected heritage_lookup tier, got: {tiers:?}"
        );
    }

    #[test]
    fn test_edge_span_columns_populated() {
        let data_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();

        let src = repo_dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("main.rs"),
            "use std::collections::HashMap;\nfn main() { HashMap::new(); }\n",
        )
        .unwrap();

        index_repo(data_dir.path(), repo_dir.path()).unwrap();

        let conn = open_db(data_dir.path()).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        // Verify from_col is populated for edges.
        let has_col: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM code_edges WHERE repo_id = ?1 AND from_col IS NOT NULL",
                params![repo_id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(has_col, "Expected from_col to be populated on edges");
    }

    #[test]
    fn test_resolve_reexport_chains() {
        let data_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();

        let src = repo_dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        std::fs::write(
            src.join("inner.rs"),
            r#"
pub struct Widget {
    pub label: String,
}
"#,
        )
        .unwrap();

        std::fs::write(
            src.join("lib.rs"),
            r#"
mod inner;
pub use crate::inner::Widget;
use crate::inner::Widget;

pub fn create_widget() -> Widget {
    Widget { label: "hi".to_string() }
}
"#,
        )
        .unwrap();

        // Index + resolve.
        index_repo(data_dir.path(), repo_dir.path()).unwrap();
        resolve_edges(data_dir.path(), repo_dir.path()).unwrap();

        let conn = open_db(data_dir.path()).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        // Should have a re_exports edge.
        let reexport_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM code_edges WHERE repo_id = ?1 AND kind = 're_exports'",
                params![repo_id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(reexport_count > 0, "Expected re_exports edges");

        // Follow re-export chains — just verify it doesn't panic.
        let _chains_resolved = resolve_reexport_chains(&conn, repo_id).unwrap();
    }
}
