//! Generated repo skills + enhanced code wiki (Chunk 37.12).
//!
//! Generates reviewable skill context documents from the native code graph.
//! Skills follow the standard agent-skill format (YAML frontmatter + Markdown)
//! compatible with Copilot, Claude Code, Cursor, Codex, etc.
//!
//! Two artifact types:
//! 1. **Cluster skills** — per-cluster SKILL.md files describing the cluster's
//!    API, key symbols, call graph, and usage patterns.
//! 2. **Process skills** — per-process SKILL.md describing the execution flow
//!    from entry point through the call chain.

use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::processes::{list_clusters, list_processes, Cluster, Process};
use super::symbol_index::{open_db, IndexError};

// ─── Public types ───────────────────────────────────────────────────────────

/// Symbol info for skill rendering (includes top-level flag as export proxy).
#[derive(Debug, Clone)]
pub struct SkillSymbol {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    /// True if the symbol has no parent (module-level = public API heuristic).
    pub top_level: bool,
}

/// Result of generating skill files for a repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillGenResult {
    pub skills_written: usize,
    pub output_dir: String,
    pub cluster_skills: Vec<SkillEntry>,
    pub process_skills: Vec<SkillEntry>,
}

/// A single generated skill entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub name: String,
    pub path: String,
    pub description: String,
}

// ─── Skill generation ───────────────────────────────────────────────────────

/// Generate skill files for a repo at the given output directory.
///
/// Creates:
/// - `skills/<cluster-name>/SKILL.md` for each cluster
/// - `skills/<process-entry>/SKILL.md` for top entry-point processes
pub fn generate_skills(
    data_dir: &Path,
    repo_path: &Path,
    output_dir: &Path,
) -> Result<SkillGenResult, IndexError> {
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

    let repo_label: String = conn
        .query_row(
            "SELECT label FROM code_repos WHERE id = ?1",
            params![repo_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "unknown".to_string());

    let clusters = list_clusters(&conn, repo_id)?;
    let processes = list_processes(&conn, repo_id)?;

    let skills_dir = output_dir.join("skills");
    std::fs::create_dir_all(&skills_dir)
        .map_err(|e| IndexError::InvalidPath(format!("cannot create skills dir: {e}")))?;

    let mut result = SkillGenResult {
        skills_written: 0,
        output_dir: output_dir.to_string_lossy().to_string(),
        cluster_skills: Vec::new(),
        process_skills: Vec::new(),
    };

    // Generate cluster skills
    for cluster in &clusters {
        let syms = load_skill_symbols(&conn, &cluster.symbol_ids)?;
        let edges = load_skill_edges(&conn, &cluster.symbol_ids)?;
        let skill = render_cluster_skill(&repo_label, cluster, &syms, &edges);

        let dir_name = sanitize_name(&cluster.label);
        let skill_dir = skills_dir.join(&dir_name);
        std::fs::create_dir_all(&skill_dir)
            .map_err(|e| IndexError::InvalidPath(format!("create skill dir: {e}")))?;

        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_path, &skill)
            .map_err(|e| IndexError::InvalidPath(format!("write skill: {e}")))?;

        result.cluster_skills.push(SkillEntry {
            name: cluster.label.clone(),
            path: skill_path.to_string_lossy().to_string(),
            description: format!("{} ({} symbols)", cluster.label, cluster.size),
        });
        result.skills_written += 1;
    }

    // Generate process skills (top 15 by step count)
    let mut sorted_processes = processes.clone();
    sorted_processes.sort_by_key(|p| std::cmp::Reverse(p.steps.len()));

    for proc in sorted_processes.iter().take(15) {
        let skill = render_process_skill(&repo_label, proc);
        let dir_name = sanitize_name(&proc.entry_point);
        let skill_dir = skills_dir.join(format!("process-{dir_name}"));
        std::fs::create_dir_all(&skill_dir)
            .map_err(|e| IndexError::InvalidPath(format!("create process skill dir: {e}")))?;

        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_path, &skill)
            .map_err(|e| IndexError::InvalidPath(format!("write process skill: {e}")))?;

        result.process_skills.push(SkillEntry {
            name: proc.entry_point.clone(),
            path: skill_path.to_string_lossy().to_string(),
            description: format!("{} ({} steps)", proc.entry_point, proc.steps.len()),
        });
        result.skills_written += 1;
    }

    Ok(result)
}

// ─── Rendering ──────────────────────────────────────────────────────────────

fn render_cluster_skill(
    repo_label: &str,
    cluster: &Cluster,
    syms: &[SkillSymbol],
    edges: &[(String, String)],
) -> String {
    let mut skill = String::new();

    // YAML frontmatter
    skill.push_str("---\n");
    skill.push_str(&format!("name: {}\n", cluster.label));
    skill.push_str(&format!(
        "description: \"Functional cluster '{}' in {} ({} symbols)\"\n",
        cluster.label, repo_label, cluster.size
    ));
    skill.push_str("---\n\n");

    // Overview
    skill.push_str(&format!("# {}\n\n", cluster.label));
    skill.push_str(&format!(
        "Functional cluster in **{}** containing {} symbols.\n\n",
        repo_label, cluster.size
    ));

    // When to use
    skill.push_str("## When to use this skill\n\n");
    skill.push_str(&format!(
        "- When working on files related to the `{}` module\n",
        cluster.label
    ));
    skill.push_str("- When you need to understand the internal call structure\n");
    skill.push_str("- When modifying symbols that belong to this cluster\n\n");

    // Key symbols (top-level = public API heuristic)
    let exported: Vec<&SkillSymbol> = syms.iter().filter(|s| s.top_level).collect();
    if !exported.is_empty() {
        skill.push_str("## Public API\n\n");
        skill.push_str("| Symbol | Kind | File | Line |\n");
        skill.push_str("|--------|------|------|------|\n");
        for sym in exported.iter().take(30) {
            skill.push_str(&format!(
                "| `{}` | {} | {} | {} |\n",
                sym.name, sym.kind, sym.file, sym.line
            ));
        }
        skill.push('\n');
    }

    // Internal structure
    let internal: Vec<&SkillSymbol> = syms.iter().filter(|s| !s.top_level).collect();
    if !internal.is_empty() {
        skill.push_str(&format!(
            "## Internal symbols ({} total)\n\n",
            internal.len()
        ));
        for sym in internal.iter().take(20) {
            skill.push_str(&format!(
                "- `{}` ({}) — {}:{}\n",
                sym.name, sym.kind, sym.file, sym.line
            ));
        }
        if internal.len() > 20 {
            skill.push_str(&format!("- ... and {} more\n", internal.len() - 20));
        }
        skill.push('\n');
    }

    // Call graph (mermaid)
    if !edges.is_empty() {
        skill.push_str("## Call Graph\n\n```mermaid\ngraph LR\n");
        for (from, to) in edges.iter().take(50) {
            skill.push_str(&format!(
                "    {} --> {}\n",
                mermaid_safe(from),
                mermaid_safe(to)
            ));
        }
        if edges.len() > 50 {
            skill.push_str(&format!("    %% ... and {} more edges\n", edges.len() - 50));
        }
        skill.push_str("```\n\n");
    }

    skill
}

fn render_process_skill(repo_label: &str, proc: &Process) -> String {
    let mut skill = String::new();

    // YAML frontmatter
    skill.push_str("---\n");
    skill.push_str(&format!(
        "name: process-{}\n",
        sanitize_name(&proc.entry_point)
    ));
    skill.push_str(&format!(
        "description: \"Execution flow from '{}' ({} steps) in {}\"\n",
        proc.entry_point,
        proc.steps.len(),
        repo_label
    ));
    skill.push_str("---\n\n");

    // Overview
    skill.push_str(&format!("# Process: {}\n\n", proc.entry_point));
    skill.push_str(&format!(
        "Execution flow starting from `{}` in **{}**.\n\n",
        proc.entry_point, repo_label
    ));
    // Derive entry file/line from the first step
    if let Some(first) = proc.steps.first() {
        skill.push_str(&format!("- **Entry file:** {}\n", first.file));
        skill.push_str(&format!("- **Entry line:** {}\n", first.line));
    }
    skill.push_str(&format!("- **Total steps:** {}\n\n", proc.steps.len()));

    // When to use
    skill.push_str("## When to use this skill\n\n");
    skill.push_str(&format!(
        "- When tracing execution from `{}`\n",
        proc.entry_point
    ));
    skill.push_str("- When debugging call chains through this flow\n");
    skill.push_str("- When assessing impact of changes to symbols in this process\n\n");

    // Execution flow
    skill.push_str("## Execution Flow\n\n");
    skill.push_str("```mermaid\ngraph TD\n");
    for (i, step) in proc.steps.iter().take(20).enumerate() {
        let safe = mermaid_safe(&step.name);
        if i > 0 {
            let prev_safe = mermaid_safe(&proc.steps[i - 1].name);
            skill.push_str(&format!("    {prev_safe} --> {safe}\n"));
        }
    }
    skill.push_str("```\n\n");

    // Step details
    skill.push_str("## Steps\n\n");
    skill.push_str("| # | Symbol | File | Line | Depth |\n");
    skill.push_str("|---|--------|------|------|-------|\n");
    for (i, step) in proc.steps.iter().enumerate() {
        skill.push_str(&format!(
            "| {} | `{}` | {} | {} | {} |\n",
            i + 1,
            step.name,
            step.file,
            step.line,
            step.depth
        ));
    }

    skill
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

fn mermaid_safe(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn load_skill_symbols(
    conn: &Connection,
    symbol_ids: &[i64],
) -> Result<Vec<SkillSymbol>, IndexError> {
    if symbol_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: String = symbol_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT name, kind, file, line, parent FROM code_symbols WHERE id IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = symbol_ids
        .iter()
        .map(|id| id as &dyn rusqlite::types::ToSql)
        .collect();
    let rows = stmt
        .query_map(params.as_slice(), |r| {
            let parent: Option<String> = r.get(4)?;
            Ok(SkillSymbol {
                name: r.get(0)?,
                kind: r.get(1)?,
                file: r.get(2)?,
                line: r.get(3)?,
                top_level: parent.is_none(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

fn load_skill_edges(
    conn: &Connection,
    symbol_ids: &[i64],
) -> Result<Vec<(String, String)>, IndexError> {
    if symbol_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: String = symbol_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT s1.name, s2.name FROM code_edges e \
         JOIN code_symbols s1 ON s1.id = e.from_symbol_id \
         JOIN code_symbols s2 ON s2.id = e.target_symbol_id \
         WHERE e.from_symbol_id IN ({placeholders}) AND e.target_symbol_id IS NOT NULL \
         AND e.kind = 'calls'"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = symbol_ids
        .iter()
        .map(|id| id as &dyn rusqlite::types::ToSql)
        .collect();
    let edges: Vec<(String, String)> = stmt
        .query_map(params.as_slice(), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(edges)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::processes::ProcessStep;

    #[test]
    fn sanitize_name_works() {
        assert_eq!(sanitize_name("server/http"), "server-http");
        assert_eq!(sanitize_name("MyClass"), "myclass");
        assert_eq!(sanitize_name("foo_bar-baz"), "foo_bar-baz");
    }

    #[test]
    fn render_cluster_skill_has_frontmatter() {
        let cluster = Cluster {
            id: 1,
            label: "core".to_string(),
            symbol_ids: vec![],
            size: 3,
        };
        let syms = vec![SkillSymbol {
            name: "main".to_string(),
            kind: "function".to_string(),
            file: "src/main.rs".to_string(),
            line: 1,
            top_level: true,
        }];
        let edges = vec![("main".to_string(), "run".to_string())];

        let skill = render_cluster_skill("my-repo", &cluster, &syms, &edges);

        assert!(skill.starts_with("---\n"));
        assert!(skill.contains("name: core"));
        assert!(skill.contains("description:"));
        assert!(skill.contains("## Public API"));
        assert!(skill.contains("| `main` |"));
        assert!(skill.contains("```mermaid"));
        assert!(skill.contains("main --> run"));
    }

    #[test]
    fn render_process_skill_has_flow() {
        let proc = Process {
            id: 1,
            entry_point: "handle_request".to_string(),
            entry_symbol_id: 1,
            steps: vec![
                ProcessStep {
                    symbol_id: 1,
                    name: "handle_request".to_string(),
                    file: "src/server.rs".to_string(),
                    line: 10,
                    depth: 0,
                },
                ProcessStep {
                    symbol_id: 2,
                    name: "parse_body".to_string(),
                    file: "src/server.rs".to_string(),
                    line: 20,
                    depth: 1,
                },
            ],
        };

        let skill = render_process_skill("my-repo", &proc);

        assert!(skill.starts_with("---\n"));
        assert!(skill.contains("name: process-handle_request"));
        assert!(skill.contains("## Execution Flow"));
        assert!(skill.contains("handle_request --> parse_body"));
        assert!(skill.contains("## Steps"));
        assert!(skill.contains("| 1 | `handle_request`"));
    }

    #[test]
    fn render_cluster_skill_no_exports() {
        let cluster = Cluster {
            id: 2,
            label: "internal".to_string(),
            symbol_ids: vec![],
            size: 1,
        };
        let syms = vec![SkillSymbol {
            name: "helper".to_string(),
            kind: "function".to_string(),
            file: "src/lib.rs".to_string(),
            line: 5,
            top_level: false,
        }];

        let skill = render_cluster_skill("repo", &cluster, &syms, &[]);

        assert!(!skill.contains("## Public API"));
        assert!(skill.contains("## Internal symbols"));
        assert!(skill.contains("- `helper`"));
    }
}
