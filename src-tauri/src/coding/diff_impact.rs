//! Native diff-impact analysis (Chunk 37.8).
//!
//! Maps git diffs to changed symbols and affected processes through the native
//! code index. Surfaces risk buckets (critical / high / moderate / low) for
//! pre-commit review.
//!
//! Pipeline:
//! 1. Run `git diff --stat` and `git diff --unified=0` to get changed files + hunks.
//! 2. Intersect hunk line-ranges with indexed `code_symbols` to find *changed symbols*.
//! 3. BFS incoming callers (using existing `resolver::call_graph`) to find *affected symbols*.
//! 4. Classify into risk buckets based on depth, fan-in, and whether the symbol is an entry point.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::resolver::call_graph;
use super::symbol_index::{open_db, IndexError};

// ─── Public types ───────────────────────────────────────────────────────────

/// A hunk extracted from `git diff`.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub file: String,
    pub start_line: u32,
    pub end_line: u32,
}

/// A symbol directly modified in the diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedSymbol {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    pub end_line: Option<u32>,
}

/// A symbol transitively affected by the diff (caller of a changed symbol).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedSymbol {
    pub name: String,
    pub file: String,
    pub line: u32,
    pub depth: u32,
}

/// Risk classification for the diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Critical,
    High,
    Moderate,
    Low,
}

/// Per-symbol impact detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolImpact {
    pub symbol: ChangedSymbol,
    pub risk: RiskLevel,
    pub affected_count: usize,
    pub max_depth: u32,
    pub affected: Vec<AffectedSymbol>,
}

/// Full diff-impact report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffImpactReport {
    pub diff_ref: String,
    pub files_changed: usize,
    pub symbols_changed: usize,
    pub total_affected: usize,
    pub risk_summary: RiskSummary,
    pub impacts: Vec<SymbolImpact>,
}

/// Aggregated risk counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSummary {
    pub critical: usize,
    pub high: usize,
    pub moderate: usize,
    pub low: usize,
}

// ─── Main entry point ───────────────────────────────────────────────────────

/// Analyze diff impact for a given git ref/range (e.g. "HEAD", "HEAD~3..HEAD", "main..feature").
/// `repo_path` is the on-disk git repo root. `data_dir` is the TerranSoul code-index dir.
pub fn analyze_diff_impact(
    data_dir: &Path,
    repo_path: &Path,
    diff_ref: &str,
    max_depth: u32,
) -> Result<DiffImpactReport, IndexError> {
    let conn = open_db(data_dir)?;

    let repo_str = repo_path
        .canonicalize()
        .map_err(|e| IndexError::InvalidPath(format!("{}: {e}", repo_path.display())))?
        .to_string_lossy()
        .to_string();

    let repo_id: i64 = conn
        .query_row(
            "SELECT id FROM code_repos WHERE path = ?1",
            params![repo_str],
            |r| r.get(0),
        )
        .map_err(|_| IndexError::InvalidPath(format!("repo not indexed: {repo_str}")))?;

    // 1. Parse git diff hunks
    let hunks = parse_diff_hunks(repo_path, diff_ref)?;
    let files_changed = hunks.iter().map(|h| &h.file).collect::<HashSet<_>>().len();

    // 2. Map hunks to changed symbols
    let changed = find_changed_symbols(&conn, repo_id, &hunks)?;

    // 3. For each changed symbol, BFS callers to find affected set
    let mut impacts = Vec::new();
    let mut total_affected_set: HashSet<String> = HashSet::new();

    for sym in &changed {
        let (affected, max_d) = bfs_affected(&conn, repo_id, &sym.name, max_depth);
        for a in &affected {
            total_affected_set.insert(format!("{}:{}", a.file, a.name));
        }
        let risk = classify_risk(affected.len(), max_d, &sym.kind);
        impacts.push(SymbolImpact {
            symbol: sym.clone(),
            risk,
            affected_count: affected.len(),
            max_depth: max_d,
            affected,
        });
    }

    // Sort by risk (critical first)
    impacts.sort_by_key(|i| match i.risk {
        RiskLevel::Critical => 0,
        RiskLevel::High => 1,
        RiskLevel::Moderate => 2,
        RiskLevel::Low => 3,
    });

    let risk_summary = RiskSummary {
        critical: impacts
            .iter()
            .filter(|i| i.risk == RiskLevel::Critical)
            .count(),
        high: impacts.iter().filter(|i| i.risk == RiskLevel::High).count(),
        moderate: impacts
            .iter()
            .filter(|i| i.risk == RiskLevel::Moderate)
            .count(),
        low: impacts.iter().filter(|i| i.risk == RiskLevel::Low).count(),
    };

    Ok(DiffImpactReport {
        diff_ref: diff_ref.to_string(),
        files_changed,
        symbols_changed: changed.len(),
        total_affected: total_affected_set.len(),
        risk_summary,
        impacts,
    })
}

// ─── Git diff parsing ───────────────────────────────────────────────────────

/// Parse `git diff --unified=0` to extract changed line ranges per file.
fn parse_diff_hunks(repo_path: &Path, diff_ref: &str) -> Result<Vec<DiffHunk>, IndexError> {
    let output = Command::new("git")
        .args(["diff", "--unified=0", diff_ref])
        .current_dir(repo_path)
        .output()
        .map_err(|e| IndexError::InvalidPath(format!("git not available: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(IndexError::InvalidPath(format!(
            "git diff failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_unified_diff(&stdout)
}

/// Parse unified diff output into hunks. Exported for testing.
pub fn parse_unified_diff(diff_text: &str) -> Result<Vec<DiffHunk>, IndexError> {
    let mut hunks = Vec::new();
    let mut current_file: Option<String> = None;

    for line in diff_text.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            current_file = Some(rest.to_string());
        } else if line.starts_with("@@") {
            if let Some(ref file) = current_file {
                if let Some(hunk) = parse_hunk_header(line, file) {
                    hunks.push(hunk);
                }
            }
        }
    }

    Ok(hunks)
}

/// Parse a `@@ -old,count +new,count @@` header into a DiffHunk for the new side.
fn parse_hunk_header(line: &str, file: &str) -> Option<DiffHunk> {
    // Format: @@ -old_start[,old_count] +new_start[,new_count] @@
    let parts: Vec<&str> = line.split_whitespace().collect();
    let new_part = parts.iter().find(|p| p.starts_with('+'))?;
    let nums = new_part.strip_prefix('+')?;

    let (start, count) = if let Some((s, c)) = nums.split_once(',') {
        (s.parse::<u32>().ok()?, c.parse::<u32>().ok()?)
    } else {
        (nums.parse::<u32>().ok()?, 1)
    };

    if count == 0 {
        return None; // Pure deletion in old file, nothing new added at this position
    }

    Some(DiffHunk {
        file: file.to_string(),
        start_line: start,
        end_line: start + count.saturating_sub(1),
    })
}

// ─── Symbol intersection ────────────────────────────────────────────────────

/// Find indexed symbols whose line range overlaps with any diff hunk.
fn find_changed_symbols(
    conn: &Connection,
    repo_id: i64,
    hunks: &[DiffHunk],
) -> Result<Vec<ChangedSymbol>, IndexError> {
    if hunks.is_empty() {
        return Ok(Vec::new());
    }

    // Group hunks by file for efficient querying
    let mut by_file: HashMap<&str, Vec<(u32, u32)>> = HashMap::new();
    for h in hunks {
        by_file
            .entry(h.file.as_str())
            .or_default()
            .push((h.start_line, h.end_line));
    }

    let mut changed = Vec::new();

    // Query symbols per file
    let mut stmt = conn.prepare(
        "SELECT id, name, kind, line, end_line FROM code_symbols \
         WHERE repo_id = ?1 AND file = ?2 ORDER BY line",
    )?;

    for (file, ranges) in &by_file {
        let rows: Vec<(i64, String, String, u32, Option<u32>)> = stmt
            .query_map(params![repo_id, file], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, u32>(3)?,
                    row.get::<_, Option<u32>>(4)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        for (id, name, kind, line, end_line) in rows {
            let sym_end = end_line.unwrap_or(line);
            // Check if any hunk overlaps this symbol's line range
            for &(hunk_start, hunk_end) in ranges {
                if hunk_start <= sym_end && hunk_end >= line {
                    changed.push(ChangedSymbol {
                        id,
                        name,
                        kind,
                        file: file.to_string(),
                        line,
                        end_line: Some(sym_end),
                    });
                    break;
                }
            }
        }
    }

    Ok(changed)
}

// ─── BFS caller traversal ───────────────────────────────────────────────────

/// BFS incoming callers of `symbol_name`, returning affected symbols and max depth reached.
fn bfs_affected(
    conn: &Connection,
    repo_id: i64,
    symbol_name: &str,
    max_depth: u32,
) -> (Vec<AffectedSymbol>, u32) {
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(symbol_name.to_string());

    let mut frontier: Vec<String> = Vec::new();
    let mut affected: Vec<AffectedSymbol> = Vec::new();
    let mut max_d: u32 = 0;

    // Depth 1: direct callers
    if let Ok(cg) = call_graph(conn, repo_id, symbol_name) {
        for edge in &cg.incoming {
            if visited.insert(edge.symbol_name.clone()) {
                frontier.push(edge.symbol_name.clone());
                affected.push(AffectedSymbol {
                    name: edge.symbol_name.clone(),
                    file: edge.file.clone(),
                    line: edge.line,
                    depth: 1,
                });
                max_d = 1;
            }
        }
    }

    // Transitive callers
    for depth in 2..=max_depth {
        let mut next = Vec::new();
        for caller in &frontier {
            if let Ok(cg) = call_graph(conn, repo_id, caller) {
                for edge in &cg.incoming {
                    if visited.insert(edge.symbol_name.clone()) {
                        affected.push(AffectedSymbol {
                            name: edge.symbol_name.clone(),
                            file: edge.file.clone(),
                            line: edge.line,
                            depth,
                        });
                        next.push(edge.symbol_name.clone());
                        max_d = depth;
                    }
                }
            }
        }
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }

    (affected, max_d)
}

// ─── Risk classification ────────────────────────────────────────────────────

/// Classify risk based on blast radius size, depth, and symbol kind.
fn classify_risk(affected_count: usize, max_depth: u32, kind: &str) -> RiskLevel {
    // Entry-point-like symbols (public API surface) are higher risk
    let is_api = matches!(kind, "function" | "method" | "trait" | "interface");

    if affected_count >= 20 || (is_api && affected_count >= 10) {
        RiskLevel::Critical
    } else if affected_count >= 10 || (is_api && affected_count >= 5) || max_depth >= 4 {
        RiskLevel::High
    } else if affected_count >= 3 || max_depth >= 2 {
        RiskLevel::Moderate
    } else {
        RiskLevel::Low
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unified_diff_basic() {
        let diff = "\
diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -10,3 +10,5 @@ fn main() {
+    let x = 1;
+    let y = 2;
@@ -50 +52,3 @@ fn helper() {
+    new_code();
+    more();
+    stuff();
";
        let hunks = parse_unified_diff(diff).unwrap();
        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[0].file, "src/main.rs");
        assert_eq!(hunks[0].start_line, 10);
        assert_eq!(hunks[0].end_line, 14);
        assert_eq!(hunks[1].start_line, 52);
        assert_eq!(hunks[1].end_line, 54);
    }

    #[test]
    fn parse_hunk_header_single_line() {
        let hunk = parse_hunk_header("@@ -5 +5 @@ fn foo()", "test.rs").unwrap();
        assert_eq!(hunk.start_line, 5);
        assert_eq!(hunk.end_line, 5);
    }

    #[test]
    fn parse_hunk_header_deletion_only() {
        // +0,0 means no new lines added (pure deletion)
        let hunk = parse_hunk_header("@@ -5,3 +5,0 @@ fn foo()", "test.rs");
        assert!(hunk.is_none());
    }

    #[test]
    fn classify_risk_levels() {
        assert_eq!(classify_risk(25, 3, "function"), RiskLevel::Critical);
        assert_eq!(classify_risk(10, 2, "struct"), RiskLevel::High);
        assert_eq!(classify_risk(5, 2, "function"), RiskLevel::High); // API + 5 affected
        assert_eq!(classify_risk(4, 2, "struct"), RiskLevel::Moderate);
        assert_eq!(classify_risk(1, 1, "field"), RiskLevel::Low);
    }

    #[test]
    fn find_changed_symbols_overlap() {
        // Test the overlap logic with a mock in-memory DB
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE code_repos (id INTEGER PRIMARY KEY, path TEXT);
             INSERT INTO code_repos VALUES (1, '/tmp/test');
             CREATE TABLE code_symbols (
                 id INTEGER PRIMARY KEY,
                 repo_id INTEGER,
                 file TEXT,
                 name TEXT,
                 kind TEXT,
                 line INTEGER,
                 end_line INTEGER,
                 parent TEXT,
                 exported INTEGER DEFAULT 0,
                 from_col INTEGER,
                 end_col INTEGER
             );
             INSERT INTO code_symbols VALUES (1, 1, 'src/main.rs', 'foo', 'function', 10, 20, NULL, 1, 0, 0);
             INSERT INTO code_symbols VALUES (2, 1, 'src/main.rs', 'bar', 'function', 25, 35, NULL, 1, 0, 0);
             INSERT INTO code_symbols VALUES (3, 1, 'src/main.rs', 'baz', 'function', 50, 60, NULL, 0, 0, 0);",
        )
        .unwrap();

        // Hunk overlaps foo (10–20) and bar partially (line 25)
        let hunks = vec![DiffHunk {
            file: "src/main.rs".into(),
            start_line: 15,
            end_line: 25,
        }];

        let changed = find_changed_symbols(&conn, 1, &hunks).unwrap();
        assert_eq!(changed.len(), 2);
        assert!(changed.iter().any(|s| s.name == "foo"));
        assert!(changed.iter().any(|s| s.name == "bar"));
    }

    #[test]
    fn bfs_affected_empty_graph() {
        // Symbol with no callers
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE code_repos (id INTEGER PRIMARY KEY, path TEXT);
             INSERT INTO code_repos VALUES (1, '/tmp/test');
             CREATE TABLE code_symbols (
                 id INTEGER PRIMARY KEY,
                 repo_id INTEGER,
                 file TEXT,
                 name TEXT,
                 kind TEXT,
                 line INTEGER,
                 end_line INTEGER,
                 parent TEXT,
                 exported INTEGER DEFAULT 0,
                 from_col INTEGER,
                 end_col INTEGER
             );
             INSERT INTO code_symbols VALUES (1, 1, 'src/main.rs', 'lonely', 'function', 1, 10, NULL, 0, 0, 0);
             CREATE TABLE code_edges (
                 id INTEGER PRIMARY KEY,
                 repo_id INTEGER,
                 from_symbol_id INTEGER,
                 target_symbol_id INTEGER,
                 target_name TEXT,
                 kind TEXT,
                 from_file TEXT,
                 from_line INTEGER,
                 confidence REAL DEFAULT 1.0,
                 resolver_tier TEXT,
                 from_col INTEGER,
                 end_line INTEGER,
                 end_col INTEGER
             );",
        )
        .unwrap();

        let (affected, max_d) = bfs_affected(&conn, 1, "lonely", 5);
        assert!(affected.is_empty());
        assert_eq!(max_d, 0);
    }
}
