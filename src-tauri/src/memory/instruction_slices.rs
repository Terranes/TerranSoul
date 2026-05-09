//! Embedding-indexed instruction slices (Chunk 43.9).
//!
//! Chunks `rules/`, `instructions/`, and `docs/` markdown files by heading,
//! indexes each slice as `category=rule, cognitive_kind=instruction` in the
//! memory store, then retrieves relevant slices per-task via embedding
//! similarity instead of bulk-injecting the entire file.

use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::store::{MemoryStore, MemoryType, NewMemory};

/// A markdown section extracted from a rules/docs file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionSlice {
    /// Source file relative path (e.g. "rules/coding-standards.md").
    pub source_file: String,
    /// Heading text (e.g. "## Use Existing Libraries First").
    pub heading: String,
    /// Full section content including the heading line.
    pub content: String,
    /// Heading level (1 = `#`, 2 = `##`, etc.).
    pub level: u8,
}

/// Result of indexing instruction files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    pub files_processed: usize,
    pub slices_created: usize,
    pub slices_skipped: usize,
}

/// Chunk a markdown file into sections by heading.
///
/// Each section contains the heading and all content until the next heading
/// of equal or higher level (lower number).
pub fn chunk_by_heading(source_file: &str, text: &str) -> Vec<InstructionSlice> {
    let mut slices = Vec::new();
    let mut current_heading = String::new();
    let mut current_level: u8 = 0;
    let mut current_content = String::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(level) = heading_level(trimmed) {
            // Flush previous section.
            if !current_content.is_empty() && current_level > 0 {
                slices.push(InstructionSlice {
                    source_file: source_file.to_string(),
                    heading: current_heading.clone(),
                    content: current_content.trim().to_string(),
                    level: current_level,
                });
            }
            current_heading = trimmed.trim_start_matches('#').trim().to_string();
            current_level = level;
            current_content = format!("{line}\n");
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Flush last section.
    if !current_content.is_empty() && current_level > 0 {
        slices.push(InstructionSlice {
            source_file: source_file.to_string(),
            heading: current_heading,
            content: current_content.trim().to_string(),
            level: current_level,
        });
    }

    slices
}

/// Detect markdown heading level. Returns `None` for non-heading lines.
fn heading_level(line: &str) -> Option<u8> {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hashes) && line.len() > hashes && line.as_bytes()[hashes] == b' ' {
        Some(hashes as u8)
    } else {
        None
    }
}

/// Index all markdown files from a directory into the memory store.
///
/// Each section becomes a memory with:
/// - `category = "rule"`
/// - `tags = "instruction,rule,<source_file>"`
/// - `memory_type = Fact`
///
/// Idempotent: sections whose content already exists (by exact match with
/// matching source tag) are skipped.
pub fn index_directory(
    store: &MemoryStore,
    dir: &Path,
    relative_prefix: &str,
) -> Result<IndexResult, String> {
    let mut result = IndexResult {
        files_processed: 0,
        slices_created: 0,
        slices_skipped: 0,
    };

    if !dir.is_dir() {
        return Ok(result);
    }

    let entries = std::fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let relative = if relative_prefix.is_empty() {
            filename.to_string()
        } else {
            format!("{relative_prefix}/{filename}")
        };

        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let slices = chunk_by_heading(&relative, &text);
        result.files_processed += 1;

        for slice in &slices {
            // Skip very small sections (likely just a heading with no content).
            if slice.content.len() < 20 {
                result.slices_skipped += 1;
                continue;
            }

            // Idempotency check: see if content already exists with this source tag.
            let tag = format!("instruction,rule,{}", slice.source_file);
            let exists: bool = store
                .conn
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM memories
                        WHERE content = ?1
                          AND tags LIKE ?2
                          AND valid_to IS NULL
                    )",
                    params![slice.content, format!("%{}%", slice.source_file)],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            if exists {
                result.slices_skipped += 1;
                continue;
            }

            store
                .add(NewMemory {
                    content: slice.content.clone(),
                    tags: tag,
                    importance: 3,
                    memory_type: MemoryType::Fact,
                    ..Default::default()
                })
                .map_err(|e| e.to_string())?;

            // Set cognitive_kind to 'instruction' (not a standard CognitiveKind variant
            // but stored as a string for retrieval filtering).
            // We'll store it via the category field instead to avoid schema changes.
            store
                .conn
                .execute(
                    "UPDATE memories SET category = 'rule' WHERE id = (
                        SELECT MAX(id) FROM memories WHERE content = ?1
                    )",
                    params![slice.content],
                )
                .map_err(|e| e.to_string())?;

            result.slices_created += 1;
        }
    }

    Ok(result)
}

/// Index the standard TerranSoul directories: `rules/`, `instructions/`, `docs/`.
pub fn index_repo_instructions(
    store: &MemoryStore,
    repo_root: &Path,
) -> Result<IndexResult, String> {
    let mut total = IndexResult {
        files_processed: 0,
        slices_created: 0,
        slices_skipped: 0,
    };

    for (dir_name, prefix) in [
        ("rules", "rules"),
        ("instructions", "instructions"),
        ("docs", "docs"),
    ] {
        let dir = repo_root.join(dir_name);
        if dir.is_dir() {
            let r = index_directory(store, &dir, prefix)?;
            total.files_processed += r.files_processed;
            total.slices_created += r.slices_created;
            total.slices_skipped += r.slices_skipped;
        }
    }

    Ok(total)
}

/// Retrieve the top-K instruction slices most relevant to a query.
///
/// Uses the memory store's hybrid search with a `category = 'rule'` filter.
/// Falls back to keyword matching when embeddings are unavailable.
pub fn retrieve_relevant_slices(
    store: &MemoryStore,
    query: &str,
    query_embedding: Option<&[f32]>,
    limit: usize,
) -> Result<Vec<InstructionSlice>, String> {
    // Use hybrid_search_rrf and filter by category afterwards.
    let entries = store
        .hybrid_search_rrf(query, query_embedding, limit * 3)
        .map_err(|e| e.to_string())?;

    let slices: Vec<InstructionSlice> = entries
        .into_iter()
        .filter(|e| e.tags.contains("instruction") && e.tags.contains("rule"))
        .take(limit)
        .map(|e| {
            let source_file = e
                .tags
                .split(',')
                .find(|t| t.contains('/') || t.ends_with(".md"))
                .unwrap_or("unknown")
                .to_string();
            InstructionSlice {
                source_file,
                heading: e.content.lines().next().unwrap_or("").to_string(),
                content: e.content,
                level: 2,
            }
        })
        .collect();

    Ok(slices)
}

/// Format retrieved instruction slices as a prompt block.
pub fn format_instruction_block(slices: &[InstructionSlice]) -> String {
    if slices.is_empty() {
        return String::new();
    }

    let mut block = String::from("[RELEVANT RULES & INSTRUCTIONS]\n");

    // Group by source file for TOC.
    let mut by_file: std::collections::BTreeMap<&str, Vec<&InstructionSlice>> =
        std::collections::BTreeMap::new();
    for s in slices {
        by_file.entry(&s.source_file).or_default().push(s);
    }

    // TOC.
    block.push_str("Sources:\n");
    for file in by_file.keys() {
        block.push_str(&format!("- {file}\n"));
    }
    block.push('\n');

    // Content.
    for (file, file_slices) in &by_file {
        block.push_str(&format!("--- {file} ---\n"));
        for s in file_slices {
            block.push_str(&s.content);
            block.push_str("\n\n");
        }
    }

    block
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MD: &str = "# Top Level\n\nIntro paragraph.\n\n## Section One\n\nContent for section one with enough characters to pass the minimum.\n\n## Section Two\n\nContent for section two with enough characters to pass the minimum.\n\n### Subsection\n\nSubsection content with enough characters to pass the minimum threshold.\n";

    #[test]
    fn chunk_by_heading_splits_correctly() {
        let slices = chunk_by_heading("test.md", SAMPLE_MD);
        assert_eq!(slices.len(), 4);
        assert_eq!(slices[0].heading, "Top Level");
        assert_eq!(slices[0].level, 1);
        assert_eq!(slices[1].heading, "Section One");
        assert_eq!(slices[1].level, 2);
        assert_eq!(slices[2].heading, "Section Two");
        assert_eq!(slices[3].heading, "Subsection");
        assert_eq!(slices[3].level, 3);
    }

    #[test]
    fn chunk_preserves_source_file() {
        let slices = chunk_by_heading("rules/coding-standards.md", SAMPLE_MD);
        for s in &slices {
            assert_eq!(s.source_file, "rules/coding-standards.md");
        }
    }

    #[test]
    fn heading_level_detection() {
        assert_eq!(heading_level("# H1"), Some(1));
        assert_eq!(heading_level("## H2"), Some(2));
        assert_eq!(heading_level("### H3"), Some(3));
        assert_eq!(heading_level("Not a heading"), None);
        assert_eq!(heading_level("#nospace"), None);
        assert_eq!(heading_level(""), None);
    }

    #[test]
    fn empty_text_returns_no_slices() {
        let slices = chunk_by_heading("test.md", "");
        assert!(slices.is_empty());
    }

    #[test]
    fn text_without_headings_returns_no_slices() {
        let slices = chunk_by_heading("test.md", "Just some text\nNo headings here.\n");
        assert!(slices.is_empty());
    }

    #[test]
    fn format_instruction_block_empty() {
        assert!(format_instruction_block(&[]).is_empty());
    }

    #[test]
    fn format_instruction_block_includes_toc() {
        let slices = vec![
            InstructionSlice {
                source_file: "rules/a.md".to_string(),
                heading: "Rule A".to_string(),
                content: "## Rule A\n\nDo this thing.".to_string(),
                level: 2,
            },
            InstructionSlice {
                source_file: "rules/b.md".to_string(),
                heading: "Rule B".to_string(),
                content: "## Rule B\n\nDo that thing.".to_string(),
                level: 2,
            },
        ];
        let block = format_instruction_block(&slices);
        assert!(block.contains("[RELEVANT RULES & INSTRUCTIONS]"));
        assert!(block.contains("rules/a.md"));
        assert!(block.contains("rules/b.md"));
        assert!(block.contains("Do this thing"));
    }

    #[test]
    fn index_creates_memories() {
        let store = MemoryStore::in_memory();
        let dir = std::env::temp_dir().join("ts_test_instruction_slices");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(
            dir.join("test-rules.md"),
            "# Test Rule\n\nThis is a test rule with enough content to pass the minimum length check for indexing.\n\n## Sub Rule\n\nAnother rule with enough content to pass the minimum length check for indexing.\n",
        )
        .unwrap();

        let result = index_directory(&store, &dir, "rules").unwrap();
        assert_eq!(result.files_processed, 1);
        assert!(result.slices_created >= 1);

        // Cleanup.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn index_is_idempotent() {
        let store = MemoryStore::in_memory();
        let dir = std::env::temp_dir().join("ts_test_instruction_slices_idem");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(
            dir.join("idem.md"),
            "# Rule\n\nThis is a rule with enough content to pass the minimum length check.\n",
        )
        .unwrap();

        let r1 = index_directory(&store, &dir, "rules").unwrap();
        let r2 = index_directory(&store, &dir, "rules").unwrap();
        assert_eq!(r2.slices_created, 0);
        assert_eq!(r2.slices_skipped, r1.slices_created);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
