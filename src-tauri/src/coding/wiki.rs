//! Wiki generation from the code symbol graph.
//!
//! Produces per-cluster Markdown pages with mermaid call graphs.
//! Optionally summarises each cluster via the active brain.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::processes::{list_clusters, Cluster};
use super::symbol_index::{open_db, IndexError};

/// Result of generating wiki pages for a repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiResult {
    /// Number of cluster pages written.
    pub pages_written: usize,
    /// Path to the wiki output directory.
    pub wiki_dir: String,
    /// Per-cluster summaries (only populated when brain is available).
    pub summaries: Vec<ClusterSummary>,
}

/// Summary of one cluster page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub cluster_id: u32,
    pub label: String,
    pub symbol_count: usize,
    /// LLM-generated summary (None if brain unavailable).
    pub summary: Option<String>,
}

/// Minimal symbol info for wiki rendering.
pub struct SymInfo {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
}

/// An edge between two symbols within a cluster.
pub struct IntraEdge {
    pub from_id: i64,
    pub to_id: i64,
}

/// Pre-loaded wiki data from the code index.
pub type WikiData = (Vec<Cluster>, Vec<Vec<SymInfo>>, Vec<Vec<IntraEdge>>);

/// Generate wiki pages for a given repo.
///
/// `brain_summarize` is an optional async callback for summarising clusters.
/// When `None`, pages are generated without LLM summaries.
pub fn generate_wiki_sync(
    data_dir: &Path,
    repo_path: &Path,
    wiki_dir: &Path,
) -> Result<WikiData, IndexError> {
    let conn = open_db(data_dir)?;
    let repo_str = repo_path.to_string_lossy().to_string();
    let repo_id: i64 = conn
        .query_row(
            "SELECT id FROM code_repos WHERE path = ?1",
            params![repo_str],
            |r| r.get(0),
        )
        .map_err(|_| IndexError::InvalidPath(format!("repo not indexed: {repo_str}")))?;

    let clusters = list_clusters(&conn, repo_id)?;
    std::fs::create_dir_all(wiki_dir)
        .map_err(|e| IndexError::InvalidPath(format!("failed to create wiki dir: {e}")))?;

    let mut all_syms = Vec::with_capacity(clusters.len());
    let mut all_edges = Vec::with_capacity(clusters.len());

    for cluster in &clusters {
        let syms = load_cluster_symbols(&conn, &cluster.symbol_ids)?;
        let edges = load_intra_cluster_edges(&conn, &cluster.symbol_ids)?;
        all_syms.push(syms);
        all_edges.push(edges);
    }

    Ok((clusters, all_syms, all_edges))
}

/// Write wiki pages given pre-loaded data and optional summaries.
pub fn write_wiki_pages(
    wiki_dir: &Path,
    clusters: &[Cluster],
    all_syms: &[Vec<SymInfo>],
    all_edges: &[Vec<IntraEdge>],
    summaries: &[Option<String>],
) -> Result<WikiResult, IndexError> {
    let mut result = WikiResult {
        pages_written: 0,
        wiki_dir: wiki_dir.to_string_lossy().to_string(),
        summaries: Vec::new(),
    };

    // Write index page
    let mut index_md = String::from("# Code Wiki\n\nGenerated from the TerranSoul symbol graph.\n\n");
    index_md.push_str("| Cluster | Symbols | Description |\n");
    index_md.push_str("|---------|---------|-------------|\n");

    for (i, cluster) in clusters.iter().enumerate() {
        let summary_text = summaries
            .get(i)
            .and_then(|s| s.as_deref())
            .unwrap_or("—");
        let filename = cluster_filename(cluster);
        index_md.push_str(&format!(
            "| [{}]({}) | {} | {} |\n",
            cluster.label, filename, cluster.size, summary_text
        ));
    }

    std::fs::write(wiki_dir.join("index.md"), &index_md)
        .map_err(|e| IndexError::InvalidPath(format!("write index.md: {e}")))?;

    // Write per-cluster pages
    for (i, cluster) in clusters.iter().enumerate() {
        let syms = &all_syms[i];
        let edges = &all_edges[i];
        let summary = summaries.get(i).and_then(|s| s.clone());

        let page = render_cluster_page(cluster, syms, edges, summary.as_deref());
        let filename = cluster_filename(cluster);
        std::fs::write(wiki_dir.join(&filename), &page)
            .map_err(|e| IndexError::InvalidPath(format!("write {filename}: {e}")))?;

        result.pages_written += 1;
        result.summaries.push(ClusterSummary {
            cluster_id: cluster.id,
            label: cluster.label.clone(),
            symbol_count: syms.len(),
            summary,
        });
    }

    // Count index page
    result.pages_written += 1;

    Ok(result)
}

/// Build the text for summarisation: list of symbols in the cluster.
pub fn build_cluster_description(cluster: &Cluster, syms: &[SymInfo]) -> String {
    let mut desc = format!(
        "Code cluster \"{}\" with {} symbols:\n\n",
        cluster.label, cluster.size
    );
    for sym in syms.iter().take(50) {
        desc.push_str(&format!("- {} {} ({}:{})\n", sym.kind, sym.name, sym.file, sym.line));
    }
    if syms.len() > 50 {
        desc.push_str(&format!("... and {} more symbols\n", syms.len() - 50));
    }
    desc
}

// ─── Internal helpers ──────────────────────────────────────────────────────

fn cluster_filename(cluster: &Cluster) -> String {
    let sanitized: String = cluster
        .label
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    format!("{:03}-{}.md", cluster.id, sanitized)
}

fn load_cluster_symbols(conn: &Connection, symbol_ids: &[i64]) -> Result<Vec<SymInfo>, IndexError> {
    if symbol_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: String = symbol_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id, name, kind, file, line FROM code_symbols WHERE id IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = symbol_ids
        .iter()
        .map(|id| id as &dyn rusqlite::types::ToSql)
        .collect();
    let rows = stmt
        .query_map(params.as_slice(), |r| {
            Ok(SymInfo {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: r.get::<_, String>(2)?,
                file: r.get(3)?,
                line: r.get(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

fn load_intra_cluster_edges(
    conn: &Connection,
    symbol_ids: &[i64],
) -> Result<Vec<IntraEdge>, IndexError> {
    if symbol_ids.is_empty() {
        return Ok(Vec::new());
    }
    let id_set: HashSet<i64> = symbol_ids.iter().copied().collect();
    let placeholders: String = symbol_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT from_symbol_id, target_symbol_id FROM code_edges \
         WHERE from_symbol_id IN ({placeholders}) AND target_symbol_id IS NOT NULL \
         AND kind = 'calls'"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = symbol_ids
        .iter()
        .map(|id| id as &dyn rusqlite::types::ToSql)
        .collect();
    let edges: Vec<IntraEdge> = stmt
        .query_map(params.as_slice(), |r| {
            Ok(IntraEdge {
                from_id: r.get(0)?,
                to_id: r.get(1)?,
            })
        })?
        .filter_map(|r| r.ok())
        .filter(|e| id_set.contains(&e.to_id))
        .collect();
    Ok(edges)
}

fn render_cluster_page(
    cluster: &Cluster,
    syms: &[SymInfo],
    edges: &[IntraEdge],
    summary: Option<&str>,
) -> String {
    let mut page = format!("# Cluster: {}\n\n", cluster.label);

    if let Some(s) = summary {
        page.push_str(&format!("{s}\n\n"));
    }

    page.push_str(&format!("**Symbols:** {}\n\n", syms.len()));

    // Mermaid call graph
    if !edges.is_empty() {
        let id_to_name: HashMap<i64, &str> = syms.iter().map(|s| (s.id, s.name.as_str())).collect();
        page.push_str("## Call Graph\n\n```mermaid\ngraph LR\n");
        let mut seen = HashSet::new();
        for edge in edges.iter().take(100) {
            let from_name = id_to_name.get(&edge.from_id).unwrap_or(&"?");
            let to_name = id_to_name.get(&edge.to_id).unwrap_or(&"?");
            let key = (edge.from_id, edge.to_id);
            if seen.insert(key) {
                // Sanitize names for mermaid (replace special chars)
                let from_safe = mermaid_safe(from_name);
                let to_safe = mermaid_safe(to_name);
                page.push_str(&format!("    {from_safe} --> {to_safe}\n"));
            }
        }
        if edges.len() > 100 {
            page.push_str(&format!("    %% ... and {} more edges\n", edges.len() - 100));
        }
        page.push_str("```\n\n");
    }

    // Symbol table
    page.push_str("## Symbols\n\n");
    page.push_str("| Name | Kind | File | Line |\n");
    page.push_str("|------|------|------|------|\n");
    for sym in syms {
        let short_file = shorten_path(&sym.file);
        page.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            sym.name, sym.kind, short_file, sym.line
        ));
    }

    page
}

fn mermaid_safe(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn shorten_path(path: &str) -> &str {
    // Show last 3 components
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    if parts.len() <= 3 {
        path
    } else {
        // Find the byte offset of the third-from-last separator
        let mut sep_count = 0;
        for (i, c) in path.char_indices().rev() {
            if c == '/' || c == '\\' {
                sep_count += 1;
                if sep_count == 3 {
                    return &path[i + 1..];
                }
            }
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_filename() {
        let cluster = Cluster {
            id: 3,
            label: "server/http".to_string(),
            symbol_ids: vec![],
            size: 5,
        };
        assert_eq!(cluster_filename(&cluster), "003-server-http.md");
    }

    #[test]
    fn test_mermaid_safe() {
        assert_eq!(mermaid_safe("foo::bar"), "foo__bar");
        assert_eq!(mermaid_safe("handle_request"), "handle_request");
    }

    #[test]
    fn test_render_cluster_page_with_summary() {
        let cluster = Cluster {
            id: 1,
            label: "core".to_string(),
            symbol_ids: vec![1, 2],
            size: 2,
        };
        let syms = vec![
            SymInfo { id: 1, name: "main".to_string(), kind: "function".to_string(), file: "src/main.rs".to_string(), line: 1 },
            SymInfo { id: 2, name: "run".to_string(), kind: "function".to_string(), file: "src/main.rs".to_string(), line: 10 },
        ];
        let edges = vec![IntraEdge { from_id: 1, to_id: 2 }];
        let page = render_cluster_page(&cluster, &syms, &edges, Some("Core application entry point."));

        assert!(page.contains("# Cluster: core"));
        assert!(page.contains("Core application entry point."));
        assert!(page.contains("```mermaid"));
        assert!(page.contains("main --> run"));
        assert!(page.contains("| `main` |"));
    }

    #[test]
    fn test_render_cluster_page_without_summary() {
        let cluster = Cluster {
            id: 2,
            label: "utils".to_string(),
            symbol_ids: vec![3],
            size: 1,
        };
        let syms = vec![
            SymInfo { id: 3, name: "helper".to_string(), kind: "function".to_string(), file: "src/utils.rs".to_string(), line: 5 },
        ];
        let page = render_cluster_page(&cluster, &syms, &[], None);

        assert!(page.contains("# Cluster: utils"));
        assert!(!page.contains("```mermaid")); // No edges = no graph
        assert!(page.contains("| `helper` |"));
    }

    #[test]
    fn test_build_cluster_description() {
        let cluster = Cluster {
            id: 1,
            label: "test-cluster".to_string(),
            symbol_ids: vec![1, 2],
            size: 2,
        };
        let syms = vec![
            SymInfo { id: 1, name: "foo".to_string(), kind: "function".to_string(), file: "a.rs".to_string(), line: 1 },
            SymInfo { id: 2, name: "bar".to_string(), kind: "struct".to_string(), file: "b.rs".to_string(), line: 10 },
        ];
        let desc = build_cluster_description(&cluster, &syms);
        assert!(desc.contains("test-cluster"));
        assert!(desc.contains("function foo"));
        assert!(desc.contains("struct bar"));
    }
}
