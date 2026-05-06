//! Diff impact overlay (Chunk 36B.4).
//!
//! Wraps [`super::diff_impact`] and produces a per-file "overlay" view
//! marking, for each changed file:
//! - Which symbols changed
//! - Which traced processes touch those symbols
//! - Which docs reference the changed file/symbols
//! - Which test files cover the changed symbols
//!
//! Intended as a pre-commit reviewer aid.

use std::collections::BTreeSet;
use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::diff_impact::{analyze_diff_impact, ChangedSymbol, DiffImpactReport};
use super::symbol_index::{open_db, IndexError};

/// Overlay information for a single changed file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOverlay {
    /// Repository-relative path to the changed file.
    pub file: String,
    /// Symbols in this file that the diff modified.
    pub changed_symbols: Vec<ChangedSymbol>,
    /// Process entry-point names whose traced flow touches a changed symbol.
    pub impacted_processes: Vec<String>,
    /// Doc files (under `docs/` or `wiki/`) that reference this file or one
    /// of its changed symbols.
    pub related_docs: Vec<String>,
    /// Test files that exercise one of the changed symbols.
    pub related_tests: Vec<String>,
}

/// The full overlay view, returned by [`build_overlay`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffOverlay {
    /// The git ref/range the overlay was built from.
    pub diff_ref: String,
    /// Per-file overlay entries.
    pub files: Vec<FileOverlay>,
    /// Total number of unique impacted processes across the overlay.
    pub total_processes: usize,
    /// Total number of unique related docs across the overlay.
    pub total_docs: usize,
    /// Total number of unique related tests across the overlay.
    pub total_tests: usize,
}

/// Build a diff overlay for the given git ref.
pub fn build_overlay(
    data_dir: &Path,
    repo_path: &Path,
    diff_ref: &str,
) -> Result<DiffOverlay, IndexError> {
    let report = analyze_diff_impact(data_dir, repo_path, diff_ref, 5)?;
    let conn = open_db(data_dir)?;
    let canon = repo_path.canonicalize().map_err(IndexError::Io)?;
    let canon_str = canon.to_string_lossy().to_string();
    let repo_id: i64 = conn
        .query_row(
            "SELECT id FROM code_repos WHERE path = ?1",
            params![canon_str],
            |r| r.get(0),
        )
        .map_err(IndexError::Sqlite)?;

    overlay_from_report(&conn, repo_id, &canon, &report)
}

/// Inner builder, factored out for testability.
pub fn overlay_from_report(
    conn: &Connection,
    repo_id: i64,
    repo_root: &Path,
    report: &DiffImpactReport,
) -> Result<DiffOverlay, IndexError> {
    super::processes::ensure_process_tables(conn)?;

    // Group changed symbols by file.
    let mut by_file: std::collections::BTreeMap<String, Vec<ChangedSymbol>> =
        std::collections::BTreeMap::new();
    for impact in &report.impacts {
        by_file
            .entry(impact.symbol.file.clone())
            .or_default()
            .push(impact.symbol.clone());
    }

    let mut files: Vec<FileOverlay> = Vec::new();
    let mut all_processes: BTreeSet<String> = BTreeSet::new();
    let mut all_docs: BTreeSet<String> = BTreeSet::new();
    let mut all_tests: BTreeSet<String> = BTreeSet::new();

    for (file, syms) in by_file.into_iter() {
        let symbol_ids: Vec<i64> = syms.iter().map(|s| s.id).collect();
        let symbol_names: Vec<String> = syms.iter().map(|s| s.name.clone()).collect();

        let processes = lookup_impacted_processes(conn, repo_id, &symbol_ids)?;
        let docs = scan_related_docs(repo_root, &file, &symbol_names);
        let tests = lookup_related_tests(conn, repo_id, &symbol_ids);

        for p in &processes {
            all_processes.insert(p.clone());
        }
        for d in &docs {
            all_docs.insert(d.clone());
        }
        for t in &tests {
            all_tests.insert(t.clone());
        }

        files.push(FileOverlay {
            file,
            changed_symbols: syms,
            impacted_processes: processes,
            related_docs: docs,
            related_tests: tests,
        });
    }

    Ok(DiffOverlay {
        diff_ref: report.diff_ref.clone(),
        files,
        total_processes: all_processes.len(),
        total_docs: all_docs.len(),
        total_tests: all_tests.len(),
    })
}

/// Find process entry-point names whose traced flow includes any of the given symbol ids.
fn lookup_impacted_processes(
    conn: &Connection,
    repo_id: i64,
    symbol_ids: &[i64],
) -> Result<Vec<String>, IndexError> {
    if symbol_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; symbol_ids.len()].join(",");
    let sql = format!(
        "SELECT DISTINCT p.entry_name \
         FROM code_processes p \
         JOIN code_process_steps ps \
           ON ps.repo_id = p.repo_id AND ps.process_id = p.process_id \
         WHERE p.repo_id = ? AND ps.symbol_id IN ({placeholders}) \
         ORDER BY p.entry_name"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(repo_id)];
    for id in symbol_ids {
        params_vec.push(Box::new(*id));
    }
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

/// Find indexed test files (path contains `test` or `spec`) that depend on
/// any of the given changed symbols (via incoming edges).
fn lookup_related_tests(conn: &Connection, repo_id: i64, symbol_ids: &[i64]) -> Vec<String> {
    if symbol_ids.is_empty() {
        return Vec::new();
    }
    let placeholders = vec!["?"; symbol_ids.len()].join(",");
    let sql = format!(
        "SELECT DISTINCT s.file \
         FROM code_edges e \
         JOIN code_symbols s ON s.id = e.from_symbol_id \
         WHERE e.repo_id = ? AND e.target_symbol_id IN ({placeholders}) \
           AND (s.file LIKE '%test%' OR s.file LIKE '%spec%' OR s.file LIKE 'tests/%') \
         ORDER BY s.file"
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(repo_id)];
    for id in symbol_ids {
        params_vec.push(Box::new(*id));
    }
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map(params_refs.as_slice(), |r| r.get::<_, String>(0))
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

/// Scan `<repo>/docs/` and `<repo>/wiki/` for `.md` files mentioning the
/// changed file path or any of the changed symbol names. Returns paths
/// relative to the repo root.
fn scan_related_docs(repo_root: &Path, changed_file: &str, symbol_names: &[String]) -> Vec<String> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    let needles: Vec<String> = std::iter::once(file_basename(changed_file))
        .chain(symbol_names.iter().cloned())
        .filter(|n| !n.is_empty())
        .collect();
    if needles.is_empty() {
        return Vec::new();
    }

    for sub in &["docs", "wiki"] {
        let dir = repo_root.join(sub);
        if !dir.is_dir() {
            continue;
        }
        scan_md_dir(&dir, repo_root, &needles, &mut found);
    }

    found.into_iter().collect()
}

fn scan_md_dir(dir: &Path, repo_root: &Path, needles: &[String], out: &mut BTreeSet<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_md_dir(&path, repo_root, needles, out);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if needles.iter().any(|n| text.contains(n)) {
            let rel = path
                .strip_prefix(repo_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel);
        }
    }
}

fn file_basename(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::super::diff_impact::{
        ChangedSymbol, DiffImpactReport, RiskLevel, RiskSummary, SymbolImpact,
    };
    use super::*;

    fn fake_report(symbols: Vec<ChangedSymbol>) -> DiffImpactReport {
        let impacts: Vec<SymbolImpact> = symbols
            .into_iter()
            .map(|s| SymbolImpact {
                symbol: s,
                risk: RiskLevel::Low,
                affected_count: 0,
                max_depth: 0,
                affected: vec![],
            })
            .collect();
        DiffImpactReport {
            diff_ref: "HEAD".to_string(),
            files_changed: 1,
            symbols_changed: impacts.len(),
            total_affected: 0,
            risk_summary: RiskSummary {
                critical: 0,
                high: 0,
                moderate: 0,
                low: impacts.len(),
            },
            impacts,
        }
    }

    fn cs(id: i64, name: &str, file: &str, line: u32) -> ChangedSymbol {
        ChangedSymbol {
            id,
            name: name.to_string(),
            kind: "function".to_string(),
            file: file.to_string(),
            line,
            end_line: Some(line + 5),
        }
    }

    #[test]
    fn file_basename_extracts_filename() {
        assert_eq!(file_basename("src/foo/bar.rs"), "bar.rs");
        assert_eq!(file_basename("bar.rs"), "bar.rs");
        assert_eq!(file_basename("a\\b\\c.md"), "c.md");
    }

    #[test]
    fn scan_related_docs_finds_mentions() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("docs")).unwrap();
        std::fs::write(
            tmp.path().join("docs").join("guide.md"),
            "Refers to handle_request in detail.",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("docs").join("unrelated.md"),
            "Nothing useful here.",
        )
        .unwrap();
        let docs = scan_related_docs(tmp.path(), "src/server.rs", &["handle_request".to_string()]);
        assert_eq!(docs.len(), 1);
        assert!(docs[0].contains("guide.md"));
    }

    #[test]
    fn scan_related_docs_empty_when_no_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = scan_related_docs(tmp.path(), "src/foo.rs", &["bar".to_string()]);
        assert!(docs.is_empty());
    }

    #[test]
    fn overlay_from_report_groups_by_file() {
        let tmp = tempfile::tempdir().unwrap();
        let conn = open_db(tmp.path()).unwrap();
        super::super::processes::ensure_process_tables(&conn).unwrap();
        conn.execute(
            "INSERT INTO code_repos (id, path, label, indexed_at) VALUES (1, ?1, 'r', 0)",
            params![tmp.path().to_string_lossy()],
        )
        .unwrap();

        let report = fake_report(vec![
            cs(1, "fn_a", "src/lib.rs", 10),
            cs(2, "fn_b", "src/lib.rs", 50),
            cs(3, "fn_c", "src/main.rs", 5),
        ]);

        let overlay = overlay_from_report(&conn, 1, tmp.path(), &report).unwrap();
        assert_eq!(overlay.files.len(), 2);
        let lib = overlay
            .files
            .iter()
            .find(|f| f.file == "src/lib.rs")
            .unwrap();
        assert_eq!(lib.changed_symbols.len(), 2);
        let main = overlay
            .files
            .iter()
            .find(|f| f.file == "src/main.rs")
            .unwrap();
        assert_eq!(main.changed_symbols.len(), 1);
    }

    #[test]
    fn overlay_finds_impacted_processes_and_tests() {
        let tmp = tempfile::tempdir().unwrap();
        let conn = open_db(tmp.path()).unwrap();
        super::super::processes::ensure_process_tables(&conn).unwrap();
        conn.execute(
            "INSERT INTO code_repos (id, path, label, indexed_at) VALUES (1, ?1, 'r', 0)",
            params![tmp.path().to_string_lossy()],
        )
        .unwrap();
        // Insert symbols (changed + a test caller).
        conn.execute(
            "INSERT INTO code_symbols (id, repo_id, name, kind, file, line, end_line) \
             VALUES (10, 1, 'target', 'function', 'src/lib.rs', 5, 10)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_symbols (id, repo_id, name, kind, file, line, end_line) \
             VALUES (20, 1, 'test_target', 'function', 'tests/lib_test.rs', 1, 8)",
            [],
        )
        .unwrap();
        // Edge: test_target -> target
        conn.execute(
            "INSERT INTO code_edges (repo_id, from_file, from_symbol_id, target_file, \
                                     target_symbol_id, kind, target_name, from_line, confidence) \
             VALUES (1, 'tests/lib_test.rs', 20, 'src/lib.rs', 10, 'calls', 'target', 1, 'exact')",
            [],
        )
        .unwrap();
        // Process touching target
        conn.execute(
            "INSERT INTO code_processes (repo_id, process_id, entry_symbol_id, entry_name) \
             VALUES (1, 1, 10, 'main_flow')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_process_steps (repo_id, process_id, symbol_id, depth, step_order) \
             VALUES (1, 1, 10, 0, 0)",
            [],
        )
        .unwrap();

        let report = fake_report(vec![cs(10, "target", "src/lib.rs", 5)]);
        let overlay = overlay_from_report(&conn, 1, tmp.path(), &report).unwrap();
        assert_eq!(overlay.files.len(), 1);
        let f = &overlay.files[0];
        assert_eq!(f.impacted_processes, vec!["main_flow".to_string()]);
        assert_eq!(f.related_tests, vec!["tests/lib_test.rs".to_string()]);
        assert_eq!(overlay.total_processes, 1);
        assert_eq!(overlay.total_tests, 1);
    }

    #[test]
    fn overlay_with_empty_report_returns_empty_files() {
        let tmp = tempfile::tempdir().unwrap();
        let conn = open_db(tmp.path()).unwrap();
        super::super::processes::ensure_process_tables(&conn).unwrap();
        conn.execute(
            "INSERT INTO code_repos (id, path, label, indexed_at) VALUES (1, ?1, 'r', 0)",
            params![tmp.path().to_string_lossy()],
        )
        .unwrap();
        let report = fake_report(vec![]);
        let overlay = overlay_from_report(&conn, 1, tmp.path(), &report).unwrap();
        assert!(overlay.files.is_empty());
        assert_eq!(overlay.total_processes, 0);
        assert_eq!(overlay.total_docs, 0);
        assert_eq!(overlay.total_tests, 0);
    }
}
