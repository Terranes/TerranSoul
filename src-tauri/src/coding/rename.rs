//! Code rename tool (Chunk 31.7, enhanced Chunk 37.9).
//!
//! Produces an edit plan for renaming a symbol across a codebase:
//! 1. **Graph-resolved edits** — symbol definitions + call edges from the index (high confidence)
//! 2. **Heritage edits** — extends/implements edges propagating the rename (medium confidence)
//! 3. **Re-export edits** — re-export chains referencing the symbol (medium confidence)
//! 4. **Text-search edits** — grep-style word-boundary matches in source files (lower confidence)
//!
//! The tool supports `dry_run` mode (returns the plan without applying) and
//! `apply` mode (writes edits to disk). The review payload groups edits by file
//! and includes summary statistics.

use std::collections::HashSet;
use std::path::Path;

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::symbol_index::{open_db, IndexError};

// ─── Public types ───────────────────────────────────────────────────────────

/// A single file edit produced by the rename tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameEdit {
    pub file: String,
    pub line: u32,
    pub old_text: String,
    pub new_text: String,
    /// "graph" (high confidence), "heritage" (medium), or "text" (lower confidence)
    pub confidence: String,
    /// Numeric confidence score: 1.0 (graph), 0.8 (heritage/re-export), 0.4 (text match)
    pub confidence_score: f64,
    /// What kind of reference: "definition", "call", "import", "heritage", "re_export", or "text_match"
    pub kind: String,
}

/// Result of a rename operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameResult {
    pub symbol_name: String,
    pub new_name: String,
    pub edits: Vec<RenameEdit>,
    pub applied: bool,
    pub files_affected: usize,
    /// Edits grouped by file for review UX.
    pub by_file: Vec<FileEditGroup>,
    /// Summary statistics.
    pub summary: RenameSummary,
}

/// Edits grouped by file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEditGroup {
    pub file: String,
    pub edits: Vec<RenameEdit>,
}

/// Summary statistics for a rename review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameSummary {
    pub graph_edits: usize,
    pub heritage_edits: usize,
    pub text_edits: usize,
    pub total_edits: usize,
}

// ─── Rename logic ───────────────────────────────────────────────────────────

/// Compute rename edits for a symbol. If `apply` is true, writes changes to disk.
pub fn rename_symbol(
    data_dir: &Path,
    repo_path: &Path,
    symbol_name: &str,
    new_name: &str,
    dry_run: bool,
) -> Result<RenameResult, IndexError> {
    let repo_path = repo_path
        .canonicalize()
        .map_err(|e| IndexError::InvalidPath(format!("{}: {e}", repo_path.display())))?;

    let conn = open_db(data_dir)?;

    let repo_str = repo_path.to_string_lossy().to_string();
    let repo_id: i64 = conn
        .query_row(
            "SELECT id FROM code_repos WHERE path = ?1",
            params![repo_str],
            |r| r.get(0),
        )
        .map_err(|_| IndexError::InvalidPath(format!("repo not indexed: {repo_str}")))?;

    let mut edits = Vec::new();

    // ─── Phase 1: Graph-resolved edits (high confidence = 1.0) ──────────

    // Find symbol definitions
    let mut def_stmt =
        conn.prepare("SELECT file, line, name FROM code_symbols WHERE repo_id = ?1 AND name = ?2")?;
    let definitions: Vec<(String, u32)> = def_stmt
        .query_map(params![repo_id, symbol_name], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (file, line) in &definitions {
        edits.push(RenameEdit {
            file: file.clone(),
            line: *line,
            old_text: symbol_name.to_string(),
            new_text: new_name.to_string(),
            confidence: "graph".into(),
            confidence_score: 1.0,
            kind: "definition".into(),
        });
    }

    // Find call/import edges that reference this symbol (by target_name or resolved target_symbol_id)
    let mut edge_stmt = conn.prepare(
        "SELECT from_file, from_line, kind FROM code_edges \
         WHERE repo_id = ?1 AND target_name = ?2",
    )?;
    let call_sites: Vec<(String, u32, String)> = edge_stmt
        .query_map(params![repo_id, symbol_name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (file, line, kind) in &call_sites {
        let (edit_kind, score) = match kind.as_str() {
            "imports" => ("import", 1.0),
            "calls" => ("call", 1.0),
            "extends" | "implements" => ("heritage", 0.8),
            "re_exports" => ("re_export", 0.8),
            _ => ("call", 1.0),
        };
        let conf = if score >= 1.0 { "graph" } else { "heritage" };
        edits.push(RenameEdit {
            file: file.clone(),
            line: *line,
            old_text: symbol_name.to_string(),
            new_text: new_name.to_string(),
            confidence: conf.into(),
            confidence_score: score,
            kind: edit_kind.to_string(),
        });
    }

    // ─── Phase 1b: Heritage edges (medium confidence = 0.8) ─────────────

    // Find symbols that extend/implement the target (heritage edges with target_name)
    let mut heritage_stmt = conn.prepare(
        "SELECT from_file, from_line FROM code_edges \
         WHERE repo_id = ?1 AND target_name = ?2 AND kind IN ('extends', 'implements')",
    )?;
    let heritage_sites: Vec<(String, u32)> = heritage_stmt
        .query_map(params![repo_id, symbol_name], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (file, line) in &heritage_sites {
        edits.push(RenameEdit {
            file: file.clone(),
            line: *line,
            old_text: symbol_name.to_string(),
            new_text: new_name.to_string(),
            confidence: "heritage".into(),
            confidence_score: 0.8,
            kind: "heritage".into(),
        });
    }

    // ─── Phase 1c: Re-export edges (medium confidence = 0.8) ────────────

    let mut reexport_stmt = conn.prepare(
        "SELECT from_file, from_line FROM code_edges \
         WHERE repo_id = ?1 AND target_name = ?2 AND kind = 're_exports'",
    )?;
    let reexport_sites: Vec<(String, u32)> = reexport_stmt
        .query_map(params![repo_id, symbol_name], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (file, line) in &reexport_sites {
        edits.push(RenameEdit {
            file: file.clone(),
            line: *line,
            old_text: symbol_name.to_string(),
            new_text: new_name.to_string(),
            confidence: "heritage".into(),
            confidence_score: 0.8,
            kind: "re_export".into(),
        });
    }

    // ─── Phase 2: Text-search edits (lower confidence = 0.4) ────────────

    // Scan source files for word-boundary occurrences not already covered by graph
    let graph_locations: HashSet<(String, u32)> =
        edits.iter().map(|e| (e.file.clone(), e.line)).collect();

    let text_edits = find_text_occurrences(&repo_path, symbol_name, &graph_locations);
    edits.extend(text_edits.into_iter().map(|(file, line)| RenameEdit {
        file,
        line,
        old_text: symbol_name.to_string(),
        new_text: new_name.to_string(),
        confidence: "text".into(),
        confidence_score: 0.4,
        kind: "text_match".into(),
    }));

    // Deduplicate by (file, line)
    let mut seen = HashSet::new();
    edits.retain(|e| seen.insert((e.file.clone(), e.line)));

    // Sort by confidence descending, then file/line
    edits.sort_by(|a, b| {
        b.confidence_score
            .partial_cmp(&a.confidence_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });

    let files_affected = edits
        .iter()
        .map(|e| e.file.as_str())
        .collect::<HashSet<_>>()
        .len();

    // Build file-grouped review payload
    let by_file = build_file_groups(&edits);

    let summary = RenameSummary {
        graph_edits: edits.iter().filter(|e| e.confidence == "graph").count(),
        heritage_edits: edits.iter().filter(|e| e.confidence == "heritage").count(),
        text_edits: edits.iter().filter(|e| e.confidence == "text").count(),
        total_edits: edits.len(),
    };

    // ─── Phase 3: Apply edits if not dry_run ────────────────────────────

    if !dry_run {
        apply_edits(&repo_path, &edits, symbol_name, new_name)?;
    }

    Ok(RenameResult {
        symbol_name: symbol_name.to_string(),
        new_name: new_name.to_string(),
        edits,
        applied: !dry_run,
        files_affected,
        by_file,
        summary,
    })
}

/// Group edits by file for review UX.
fn build_file_groups(edits: &[RenameEdit]) -> Vec<FileEditGroup> {
    let mut map: std::collections::HashMap<&str, Vec<RenameEdit>> =
        std::collections::HashMap::new();
    for edit in edits {
        map.entry(edit.file.as_str())
            .or_default()
            .push(edit.clone());
    }
    let mut groups: Vec<FileEditGroup> = map
        .into_iter()
        .map(|(file, edits)| FileEditGroup {
            file: file.to_string(),
            edits,
        })
        .collect();
    groups.sort_by(|a, b| a.file.cmp(&b.file));
    groups
}

/// Scan source files for word-boundary occurrences of `symbol_name`
/// that aren't already in the graph-resolved set.
fn find_text_occurrences(
    repo_path: &Path,
    symbol_name: &str,
    already_found: &HashSet<(String, u32)>,
) -> Vec<(String, u32)> {
    let mut results = Vec::new();
    let mut dirs = vec![repo_path.to_path_buf()];

    while let Some(dir) = dirs.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                // Skip hidden dirs, node_modules, target, .git, dist
                if name.starts_with('.')
                    || name == "node_modules"
                    || name == "target"
                    || name == "dist"
                {
                    continue;
                }
                dirs.push(path);
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "vue" | "py") {
                continue;
            }

            let rel_path = path
                .strip_prefix(repo_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for (line_num, line) in content.lines().enumerate() {
                let line_1based = (line_num + 1) as u32;
                if already_found.contains(&(rel_path.clone(), line_1based)) {
                    continue;
                }
                if contains_word(line, symbol_name) {
                    results.push((rel_path.clone(), line_1based));
                }
            }
        }
    }

    results
}

/// Check if a line contains `word` as a whole word (bounded by non-identifier chars).
fn contains_word(line: &str, word: &str) -> bool {
    let bytes = line.as_bytes();
    let word_bytes = word.as_bytes();
    let wlen = word_bytes.len();

    if wlen == 0 || bytes.len() < wlen {
        return false;
    }

    for i in 0..=(bytes.len() - wlen) {
        if &bytes[i..i + wlen] == word_bytes {
            // Check left boundary
            if i > 0 && is_ident_char(bytes[i - 1]) {
                continue;
            }
            // Check right boundary
            if i + wlen < bytes.len() && is_ident_char(bytes[i + wlen]) {
                continue;
            }
            return true;
        }
    }
    false
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Apply rename edits to files on disk.
fn apply_edits(
    repo_path: &Path,
    edits: &[RenameEdit],
    old_name: &str,
    new_name: &str,
) -> Result<(), IndexError> {
    // Group edits by file
    let mut by_file: std::collections::HashMap<&str, Vec<u32>> = std::collections::HashMap::new();
    for edit in edits {
        by_file
            .entry(edit.file.as_str())
            .or_default()
            .push(edit.line);
    }

    for (rel_path, lines) in &by_file {
        let full_path = repo_path.join(rel_path);
        let content = std::fs::read_to_string(&full_path).map_err(|e| {
            IndexError::InvalidPath(format!("cannot read {}: {e}", full_path.display()))
        })?;

        let line_set: HashSet<u32> = lines.iter().copied().collect();
        let mut output = String::with_capacity(content.len());

        for (idx, line) in content.lines().enumerate() {
            let line_1based = (idx + 1) as u32;
            if line_set.contains(&line_1based) {
                // Replace whole-word occurrences on this line
                output.push_str(&replace_word(line, old_name, new_name));
            } else {
                output.push_str(line);
            }
            output.push('\n');
        }

        // Preserve trailing newline behaviour
        if !content.ends_with('\n') && output.ends_with('\n') {
            output.pop();
        }

        std::fs::write(&full_path, &output).map_err(|e| {
            IndexError::InvalidPath(format!("cannot write {}: {e}", full_path.display()))
        })?;
    }

    Ok(())
}

/// Replace all whole-word occurrences of `old` with `new` in a line.
fn replace_word(line: &str, old: &str, new: &str) -> String {
    let bytes = line.as_bytes();
    let old_bytes = old.as_bytes();
    let olen = old_bytes.len();

    if olen == 0 || bytes.len() < olen {
        return line.to_string();
    }

    let mut result = String::with_capacity(line.len());
    let mut i = 0;

    while i <= bytes.len() - olen {
        if &bytes[i..i + olen] == old_bytes {
            let left_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let right_ok = i + olen >= bytes.len() || !is_ident_char(bytes[i + olen]);
            if left_ok && right_ok {
                result.push_str(new);
                i += olen;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    // Append remaining bytes
    while i < bytes.len() {
        result.push(bytes[i] as char);
        i += 1;
    }

    result
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_word_basic() {
        assert!(contains_word("fn hello_world() {", "hello_world"));
        assert!(!contains_word("fn hello_world_ext() {", "hello_world"));
        assert!(contains_word("use crate::hello_world;", "hello_world"));
        assert!(!contains_word("// no match here", "hello_world"));
    }

    #[test]
    fn replace_word_basic() {
        let line = "fn old_func(x: old_func) -> old_func_ext {";
        let result = replace_word(line, "old_func", "new_func");
        assert_eq!(result, "fn new_func(x: new_func) -> old_func_ext {");
    }

    #[test]
    fn rename_dry_run_on_fixture() {
        // Create a temp dir with a fixture repo
        let tmp = std::env::temp_dir().join("ts_rename_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();

        // Write fixture files
        std::fs::write(
            tmp.join("src/main.rs"),
            "fn compute_total() {\n    let x = compute_total();\n}\n",
        )
        .unwrap();
        std::fs::write(
            tmp.join("src/helper.rs"),
            "use crate::compute_total;\n\nfn helper() {\n    compute_total();\n}\n",
        )
        .unwrap();

        // Index the fixture repo
        let data_dir = tmp.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        crate::coding::symbol_index::index_repo(&data_dir, &tmp).unwrap();

        // Run rename in dry_run mode
        let result =
            rename_symbol(&data_dir, &tmp, "compute_total", "calculate_sum", true).unwrap();

        assert!(!result.applied);
        assert_eq!(result.symbol_name, "compute_total");
        assert_eq!(result.new_name, "calculate_sum");
        assert!(!result.edits.is_empty());
        assert!(result.files_affected >= 1);

        // Verify original files unchanged (dry_run)
        let content = std::fs::read_to_string(tmp.join("src/main.rs")).unwrap();
        assert!(content.contains("compute_total"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rename_apply_modifies_files() {
        let tmp = std::env::temp_dir().join("ts_rename_apply_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();

        std::fs::write(
            tmp.join("src/main.rs"),
            "fn old_name() {\n    old_name();\n}\n",
        )
        .unwrap();

        let data_dir = tmp.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        crate::coding::symbol_index::index_repo(&data_dir, &tmp).unwrap();

        // Apply rename
        let result = rename_symbol(&data_dir, &tmp, "old_name", "new_name", false).unwrap();
        assert!(result.applied);

        // Verify file was modified
        let content = std::fs::read_to_string(tmp.join("src/main.rs")).unwrap();
        assert!(content.contains("new_name"));
        assert!(!content.contains("old_name"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rename_review_payload_structure() {
        let tmp = std::env::temp_dir().join("ts_rename_review_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();

        std::fs::write(
            tmp.join("src/lib.rs"),
            "pub fn my_func() {}\nfn caller() { my_func(); }\n",
        )
        .unwrap();
        std::fs::write(
            tmp.join("src/other.rs"),
            "use crate::my_func;\nfn other() { my_func(); }\n",
        )
        .unwrap();

        let data_dir = tmp.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        crate::coding::symbol_index::index_repo(&data_dir, &tmp).unwrap();

        let result = rename_symbol(&data_dir, &tmp, "my_func", "renamed_func", true).unwrap();

        // Review payload should be structured
        assert!(!result.by_file.is_empty());
        assert!(result.summary.total_edits > 0);
        assert_eq!(result.summary.total_edits, result.edits.len());

        // Graph edits should have confidence_score 1.0
        for edit in result.edits.iter().filter(|e| e.confidence == "graph") {
            assert_eq!(edit.confidence_score, 1.0);
        }

        // Text edits should have confidence_score 0.4
        for edit in result.edits.iter().filter(|e| e.confidence == "text") {
            assert_eq!(edit.confidence_score, 0.4);
        }

        // Edits should be sorted by confidence (highest first)
        let scores: Vec<f64> = result.edits.iter().map(|e| e.confidence_score).collect();
        for w in scores.windows(2) {
            assert!(w[0] >= w[1], "Edits not sorted by confidence descending");
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rename_heritage_edges_included() {
        let tmp = std::env::temp_dir().join("ts_rename_heritage_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();

        // TypeScript with extends/implements
        std::fs::write(
            tmp.join("src/base.ts"),
            "export interface Animal {\n  name: string;\n}\n",
        )
        .unwrap();
        std::fs::write(
            tmp.join("src/dog.ts"),
            "import { Animal } from './base';\nexport class Dog implements Animal {\n  name = 'Rex';\n}\n",
        )
        .unwrap();

        let data_dir = tmp.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        crate::coding::symbol_index::index_repo(&data_dir, &tmp).unwrap();

        let result = rename_symbol(&data_dir, &tmp, "Animal", "Creature", true).unwrap();

        // Should find at least the definition + edges referencing "Animal"
        assert!(!result.edits.is_empty());
        assert!(result.summary.total_edits >= 1);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn build_file_groups_correct() {
        let edits = vec![
            RenameEdit {
                file: "b.rs".into(),
                line: 5,
                old_text: "x".into(),
                new_text: "y".into(),
                confidence: "graph".into(),
                confidence_score: 1.0,
                kind: "definition".into(),
            },
            RenameEdit {
                file: "a.rs".into(),
                line: 1,
                old_text: "x".into(),
                new_text: "y".into(),
                confidence: "text".into(),
                confidence_score: 0.4,
                kind: "text_match".into(),
            },
            RenameEdit {
                file: "b.rs".into(),
                line: 10,
                old_text: "x".into(),
                new_text: "y".into(),
                confidence: "graph".into(),
                confidence_score: 1.0,
                kind: "call".into(),
            },
        ];

        let groups = build_file_groups(&edits);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].file, "a.rs");
        assert_eq!(groups[0].edits.len(), 1);
        assert_eq!(groups[1].file, "b.rs");
        assert_eq!(groups[1].edits.len(), 2);
    }
}
