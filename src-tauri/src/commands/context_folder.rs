//! Tauri commands for user-defined context folders.
//!
//! Context folders let users point TerranSoul at local directories of
//! documents. The brain scans them recursively for supported file types
//! and ingests the content into long-term memory — similar to how
//! Copilot / Claude index a workspace, but opt-in and explicit.
//!
//! **Not recommended for large trees**: scanning is brute-force and can
//! be slow for folders with thousands of files.

use crate::commands::ingest::ingest_document_silent;
use crate::settings::{config_store, ContextFolder};
use crate::AppState;
use tauri::{AppHandle, Emitter, State};

/// File extensions accepted for context-folder ingestion.
/// Must match the set supported by `commands::ingest::read_local_file`.
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "md", "markdown", "txt", "csv", "json", "xml", "html", "htm", "log", "rst", "adoc", "pdf",
];

/// Result of scanning a folder (preview before ingest).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextFolderScanResult {
    pub path: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub files: Vec<String>,
    pub warning: Option<String>,
}

/// Progress event emitted during context-folder sync.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextFolderSyncProgress {
    pub folder_path: String,
    pub current_file: String,
    pub processed: usize,
    pub total: usize,
}

/// Result of a full sync across all enabled context folders.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextFolderSyncResult {
    pub folders_synced: usize,
    pub files_ingested: usize,
    pub errors: Vec<String>,
}

/// Scan a folder and return a preview of the files that would be ingested.
/// Does NOT ingest anything — use `sync_context_folders` for that.
#[tauri::command(rename_all = "camelCase")]
pub async fn scan_context_folder(
    folder_path: String,
) -> Result<ContextFolderScanResult, String> {
    let path = std::path::Path::new(&folder_path);
    if !path.exists() {
        return Err(format!("Folder not found: {folder_path}"));
    }
    if !path.is_dir() {
        return Err(format!("Not a directory: {folder_path}"));
    }

    let files = collect_supported_files(path)?;
    let total_bytes: u64 = files
        .iter()
        .filter_map(|f| std::fs::metadata(f).ok().map(|m| m.len()))
        .sum();

    let warning = if files.len() > 500 {
        Some(format!(
            "This folder contains {} files ({:.1} MB). Ingestion will be slow \
             and consume significant brain resources. Consider using a smaller, \
             curated folder instead.",
            files.len(),
            total_bytes as f64 / 1_048_576.0,
        ))
    } else {
        None
    };

    Ok(ContextFolderScanResult {
        path: folder_path,
        file_count: files.len(),
        total_bytes,
        files,
        warning,
    })
}

/// Add a context folder to settings. Returns the scan preview.
#[tauri::command(rename_all = "camelCase")]
pub async fn add_context_folder(
    folder_path: String,
    label: Option<String>,
    state: State<'_, AppState>,
) -> Result<ContextFolderScanResult, String> {
    let path = std::path::Path::new(&folder_path);
    if !path.exists() {
        return Err(format!("Folder not found: {folder_path}"));
    }
    if !path.is_dir() {
        return Err(format!("Not a directory: {folder_path}"));
    }

    // Canonicalize for consistent comparison
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Cannot resolve path: {e}"))?
        .to_string_lossy()
        .to_string();

    let label = label.unwrap_or_else(|| {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string()
    });

    // Check for duplicates
    {
        let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        if settings
            .context_folders
            .iter()
            .any(|f| normalize_path(&f.path) == normalize_path(&canonical))
        {
            return Err(format!("Folder already added: {canonical}"));
        }
    }

    let folder = ContextFolder {
        path: canonical.clone(),
        label,
        enabled: true,
        last_synced_at: 0,
        last_file_count: 0,
    };

    {
        let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        settings.context_folders.push(folder);
        config_store::save(&state.data_dir, &settings)?;
    }

    scan_context_folder(canonical).await
}

/// Remove a context folder from settings by path.
#[tauri::command(rename_all = "camelCase")]
pub async fn remove_context_folder(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    let before = settings.context_folders.len();
    settings
        .context_folders
        .retain(|f| normalize_path(&f.path) != normalize_path(&folder_path));
    if settings.context_folders.len() == before {
        return Err(format!("Folder not found in settings: {folder_path}"));
    }
    config_store::save(&state.data_dir, &settings)?;
    Ok(())
}

/// Toggle a context folder's enabled state.
#[tauri::command(rename_all = "camelCase")]
pub async fn toggle_context_folder(
    folder_path: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    let folder = settings
        .context_folders
        .iter_mut()
        .find(|f| normalize_path(&f.path) == normalize_path(&folder_path))
        .ok_or_else(|| format!("Folder not found: {folder_path}"))?;
    folder.enabled = enabled;
    config_store::save(&state.data_dir, &settings)?;
    Ok(())
}

/// List all configured context folders.
#[tauri::command]
pub async fn list_context_folders(
    state: State<'_, AppState>,
) -> Result<Vec<ContextFolder>, String> {
    let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.context_folders.clone())
}

/// Sync all enabled context folders — scan each folder and ingest new/changed
/// files into the brain's long-term memory. Emits `context-folder-progress`
/// events for UI feedback.
///
/// Each file is ingested via the existing `ingest_document_silent` pipeline
/// (chunking, embedding, dedup by SHA-256 hash). Files that haven't changed
/// since the last sync are automatically skipped by the dedup logic.
#[tauri::command(rename_all = "camelCase")]
pub async fn sync_context_folders(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<ContextFolderSyncResult, String> {
    let folders: Vec<ContextFolder> = {
        let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        settings
            .context_folders
            .iter()
            .filter(|f| f.enabled)
            .cloned()
            .collect()
    };

    if folders.is_empty() {
        return Ok(ContextFolderSyncResult {
            folders_synced: 0,
            files_ingested: 0,
            errors: vec![],
        });
    }

    let mut total_ingested = 0usize;
    let mut total_errors = Vec::new();
    let mut folders_synced = 0usize;

    for folder in &folders {
        let path = std::path::Path::new(&folder.path);
        if !path.exists() || !path.is_dir() {
            total_errors.push(format!("Folder missing: {}", folder.path));
            continue;
        }

        let files = match collect_supported_files(path) {
            Ok(f) => f,
            Err(e) => {
                total_errors.push(format!("{}: {e}", folder.path));
                continue;
            }
        };

        let total = files.len();
        for (i, file_path) in files.iter().enumerate() {
            let _ = app_handle.emit(
                "context-folder-progress",
                ContextFolderSyncProgress {
                    folder_path: folder.path.clone(),
                    current_file: file_path.clone(),
                    processed: i,
                    total,
                },
            );

            let tags = format!("context-folder,{}", folder.label);
            match ingest_document_silent(
                file_path.clone(),
                Some(tags),
                Some(3), // moderate importance — context folders are reference, not critical
                state.inner().clone(),
            )
            .await
            {
                Ok(_) => total_ingested += 1,
                Err(e) => {
                    total_errors.push(format!("{file_path}: {e}"));
                }
            }
        }

        // Update last-synced metadata
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
            if let Some(f) = settings
                .context_folders
                .iter_mut()
                .find(|f| normalize_path(&f.path) == normalize_path(&folder.path))
            {
                f.last_synced_at = now;
                f.last_file_count = total;
            }
            let _ = config_store::save(&state.data_dir, &settings);
        }

        folders_synced += 1;
    }

    Ok(ContextFolderSyncResult {
        folders_synced,
        files_ingested: total_ingested,
        errors: total_errors,
    })
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Recursively collect supported files from a directory.
fn collect_supported_files(dir: &std::path::Path) -> Result<Vec<String>, String> {
    let mut results = Vec::new();
    collect_recursive(dir, &mut results, 0)?;
    results.sort();
    Ok(results)
}

fn collect_recursive(
    dir: &std::path::Path,
    out: &mut Vec<String>,
    depth: usize,
) -> Result<(), String> {
    // Safety: limit recursion depth to prevent runaway on symlink loops
    if depth > 20 {
        return Ok(());
    }

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read {}: {e}", dir.display()))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();

        // Skip hidden files/dirs (start with '.')
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }

        // Skip common large/irrelevant directories
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if matches!(
                name,
                "node_modules" | "target" | ".git" | "__pycache__" | "dist" | "build" | ".venv"
                    | "venv" | ".next" | ".nuxt"
            ) {
                continue;
            }
            collect_recursive(&path, out, depth + 1)?;
            continue;
        }

        if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                out.push(path.to_string_lossy().to_string());
            }
        }
    }
    Ok(())
}

/// Normalize path separators for consistent comparison across platforms.
fn normalize_path(p: &str) -> String {
    p.replace('\\', "/").to_lowercase()
}

// ── Context ↔ Knowledge Conversion ────────────────────────────────────────────

/// Summary of memories originating from context folders.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextFolderMemoryInfo {
    pub total_memories: usize,
    pub total_tokens: i64,
    pub by_folder: Vec<FolderMemoryCount>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FolderMemoryCount {
    pub label: String,
    pub count: usize,
}

/// Return stats about memories that originated from context-folder ingestion.
#[tauri::command]
pub async fn list_context_folder_memories(
    state: State<'_, AppState>,
) -> Result<ContextFolderMemoryInfo, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let entries = store
        .search("context-folder")
        .map_err(|e| format!("search: {e}"))?;

    let ctx_entries: Vec<_> = entries
        .iter()
        .filter(|e| e.tags.contains("context-folder"))
        .collect();

    let total_tokens: i64 = ctx_entries.iter().map(|e| e.token_count).sum();

    // Group by folder label (second tag after "context-folder")
    let mut folder_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for entry in &ctx_entries {
        let label = entry
            .tags
            .split(',')
            .map(|t| t.trim())
            .find(|t| !t.is_empty() && *t != "context-folder")
            .unwrap_or("unknown")
            .to_string();
        *folder_counts.entry(label).or_insert(0) += 1;
    }

    let mut by_folder: Vec<FolderMemoryCount> = folder_counts
        .into_iter()
        .map(|(label, count)| FolderMemoryCount { label, count })
        .collect();
    by_folder.sort_by_key(|b| std::cmp::Reverse(b.count));

    Ok(ContextFolderMemoryInfo {
        total_memories: ctx_entries.len(),
        total_tokens,
        by_folder,
    })
}

/// Result of exporting knowledge to a folder.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KnowledgeExportResult {
    pub files_written: usize,
    pub output_dir: String,
}

/// Export brain memories back to a folder as plain-text files.
///
/// Each memory becomes a separate `.md` file with YAML frontmatter
/// (id, tags, importance, created_at) and body content. This is the
/// reverse of context-folder ingestion: knowledge → portable files.
///
/// Filter options:
/// - `tag_filter`: only export memories matching this tag substring
///   (e.g. "context-folder" for just context-folder content, or empty
///   for all memories).
/// - `min_importance`: only export memories at or above this importance.
#[tauri::command(rename_all = "camelCase")]
pub async fn export_knowledge_to_folder(
    output_dir: String,
    tag_filter: Option<String>,
    min_importance: Option<i64>,
    state: State<'_, AppState>,
) -> Result<KnowledgeExportResult, String> {
    let out_path = std::path::Path::new(&output_dir);
    std::fs::create_dir_all(out_path)
        .map_err(|e| format!("Cannot create output dir: {e}"))?;

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let all_entries = if let Some(ref filter) = tag_filter {
        if filter.trim().is_empty() {
            store.get_all().map_err(|e| format!("get_all: {e}"))?
        } else {
            store
                .search(filter)
                .map_err(|e| format!("search: {e}"))?
                .into_iter()
                .filter(|e| e.tags.to_lowercase().contains(&filter.to_lowercase()))
                .collect()
        }
    } else {
        store.get_all().map_err(|e| format!("get_all: {e}"))?
    };

    let min_imp = min_importance.unwrap_or(1);
    let filtered: Vec<_> = all_entries
        .iter()
        .filter(|e| e.importance >= min_imp)
        .collect();

    let mut written = 0usize;
    for entry in &filtered {
        let slug = crate::memory::obsidian_export::slugify(&entry.content);
        let filename = format!("{:04}-{}.md", entry.id, slug);
        let file_path = out_path.join(&filename);

        let md = crate::memory::obsidian_export::render_markdown(entry);
        std::fs::write(&file_path, md)
            .map_err(|e| format!("write {}: {e}", file_path.display()))?;
        written += 1;
    }

    Ok(KnowledgeExportResult {
        files_written: written,
        output_dir,
    })
}

/// Result of converting raw context chunks into consolidated knowledge.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextConversionResult {
    pub source_chunks: usize,
    pub knowledge_entries_created: usize,
    pub summary: String,
}

/// Convert raw context-folder chunks into consolidated knowledge entries.
///
/// Groups context-folder memories by source file, concatenates chunks from the
/// same file, and stores a consolidated summary as a new high-importance
/// knowledge memory tagged `knowledge,converted,<original-label>`. The original
/// context-folder chunks remain untouched (they are low-importance reference
/// material; the converted knowledge entries are the optimised form).
///
/// This is the recommended workflow: ingest a context folder for raw reference,
/// then convert to produce compact, high-quality knowledge entries that score
/// higher in RAG retrieval.
#[tauri::command(rename_all = "camelCase")]
pub async fn convert_context_to_knowledge(
    folder_label: Option<String>,
    state: State<'_, AppState>,
) -> Result<ContextConversionResult, String> {
    use crate::memory::{MemoryType, NewMemory};

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let entries = store
        .search("context-folder")
        .map_err(|e| format!("search: {e}"))?;

    let ctx_entries: Vec<_> = entries
        .into_iter()
        .filter(|e| {
            if !e.tags.contains("context-folder") {
                return false;
            }
            if let Some(ref label) = folder_label {
                e.tags.to_lowercase().contains(&label.to_lowercase())
            } else {
                true
            }
        })
        .collect();

    if ctx_entries.is_empty() {
        return Ok(ContextConversionResult {
            source_chunks: 0,
            knowledge_entries_created: 0,
            summary: "No context-folder memories found to convert.".to_string(),
        });
    }

    // Group by source_url (file path). Chunks from the same file are grouped.
    let mut by_source: std::collections::HashMap<String, Vec<&crate::memory::store::MemoryEntry>> =
        std::collections::HashMap::new();
    for entry in &ctx_entries {
        let key = entry
            .source_url
            .as_deref()
            .unwrap_or("unknown-source")
            .to_string();
        by_source.entry(key).or_default().push(entry);
    }

    let source_chunks = ctx_entries.len();
    let mut created = 0usize;
    let mut summaries = Vec::new();

    for (source, chunks) in &by_source {
        // Sort by id (creation order) to preserve document flow
        let mut sorted_chunks = chunks.clone();
        sorted_chunks.sort_by_key(|c| c.id);

        // Concatenate all chunks from this file
        let combined: String = sorted_chunks
            .iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        // Create a compact summary — truncate to first 2000 chars as the
        // consolidated knowledge entry. Full LLM summarisation could be added
        // but would require an async brain call; this deterministic approach
        // works offline and is fast.
        let max_len = 2000;
        let consolidated = if combined.len() > max_len {
            // Take first and last portions for context
            let head = &combined[..max_len / 2];
            let tail_start = combined.len().saturating_sub(max_len / 2);
            let tail = &combined[tail_start..];
            format!(
                "{head}\n\n[… {} chars omitted …]\n\n{tail}",
                combined.len() - max_len
            )
        } else {
            combined.clone()
        };

        // Extract the file basename for the knowledge title
        let basename = std::path::Path::new(source)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(source);

        let content = format!(
            "# Knowledge: {basename}\n\nSource: {source}\nChunks consolidated: {}\n\n{consolidated}",
            chunks.len()
        );

        // Derive label from the original context-folder tag
        let label = sorted_chunks
            .first()
            .and_then(|c| {
                c.tags
                    .split(',')
                    .map(|t| t.trim())
                    .find(|t| !t.is_empty() && *t != "context-folder")
            })
            .unwrap_or("converted");

        let tags = format!("knowledge,converted,{label}");

        let new_mem = NewMemory {
            content,
            tags,
            importance: 4, // high importance — consolidated knowledge
            memory_type: MemoryType::Fact,
            source_url: Some(source.clone()),
            source_hash: None,
            expires_at: None,
        };

        match store.add(new_mem) {
            Ok(_) => {
                created += 1;
                summaries.push(format!("{basename}: {} chunks → 1 knowledge entry", chunks.len()));
            }
            Err(e) => {
                summaries.push(format!("{basename}: error — {e}"));
            }
        }
    }

    Ok(ContextConversionResult {
        source_chunks,
        knowledge_entries_created: created,
        summary: format!(
            "Converted {} chunks from {} sources into {} knowledge entries.\n{}",
            source_chunks,
            by_source.len(),
            created,
            summaries.join("\n")
        ),
    })
}

// ── Knowledge Graph ↔ Context Files ───────────────────────────────────────────

/// A node in the exported KG subtree, including its edges to other nodes
/// that are also inside the subtree.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KgExportNode {
    pub id: i64,
    pub content: String,
    pub tags: String,
    pub importance: i64,
    pub memory_type: String,
    pub edges: Vec<KgExportEdge>,
}

/// A directed edge between two nodes in an exported KG subtree.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KgExportEdge {
    pub src_id: i64,
    pub dst_id: i64,
    pub rel_type: String,
    pub confidence: f64,
}

/// Result of exporting a KG subtree to context files.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KgSubtreeExportResult {
    pub root_ids: Vec<i64>,
    pub nodes_exported: usize,
    pub edges_exported: usize,
    pub files_written: usize,
    pub output_dir: String,
}

/// Export a knowledge-graph subtree rooted at `root_ids` to a directory of
/// Markdown context files with a `_graph.json` manifest.
///
/// For each node in the subtree (BFS up to `max_hops` from each root):
/// - One `.md` file with YAML frontmatter + body (same format as Obsidian
///   export) plus a `## Graph Edges` section listing connected nodes
/// - A `_graph.json` manifest with the full node+edge structure so the
///   subtree can be re-imported via `import_file_to_knowledge_graph`
///
/// This is the KG-aware counterpart of `export_knowledge_to_folder`: it
/// preserves graph structure, not just flat memories.
#[tauri::command(rename_all = "camelCase")]
pub async fn export_kg_subtree(
    root_ids: Vec<i64>,
    output_dir: String,
    max_hops: Option<usize>,
    state: State<'_, AppState>,
) -> Result<KgSubtreeExportResult, String> {
    use crate::memory::edges::EdgeDirection;

    if root_ids.is_empty() {
        return Err("root_ids must not be empty".into());
    }
    let out_path = std::path::Path::new(&output_dir);
    std::fs::create_dir_all(out_path)
        .map_err(|e| format!("Cannot create output dir: {e}"))?;

    let hops = max_hops.unwrap_or(2).clamp(0, 5);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;

    // 1. BFS from each root to collect all node IDs in the subtree
    let mut node_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
    for &root in &root_ids {
        node_ids.insert(root);
        if hops > 0 {
            let reachable = store
                .traverse_from(root, hops, None)
                .map_err(|e| format!("traverse: {e}"))?;
            for (nid, _) in &reachable {
                node_ids.insert(*nid);
            }
        }
    }

    // 2. Load all node entries and their internal edges
    let mut export_nodes: Vec<KgExportNode> = Vec::new();
    let mut all_edges: Vec<KgExportEdge> = Vec::new();

    for &nid in &node_ids {
        let entry = store
            .get_by_id(nid)
            .map_err(|e| format!("get_by_id({nid}): {e}"))?;

        let edges = store
            .get_edges_for(nid, EdgeDirection::Both)
            .map_err(|e| format!("edges for {nid}: {e}"))?;

        // Only keep edges where BOTH endpoints are in the subtree
        let internal_edges: Vec<KgExportEdge> = edges
            .iter()
            .filter(|e| node_ids.contains(&e.src_id) && node_ids.contains(&e.dst_id))
            .map(|e| KgExportEdge {
                src_id: e.src_id,
                dst_id: e.dst_id,
                rel_type: e.rel_type.clone(),
                confidence: e.confidence,
            })
            .collect();

        export_nodes.push(KgExportNode {
            id: entry.id,
            content: entry.content.clone(),
            tags: entry.tags.clone(),
            importance: entry.importance,
            memory_type: entry.memory_type.as_str().to_string(),
            edges: internal_edges.clone(),
        });

        for edge in &internal_edges {
            // Avoid duplicating undirected edges
            if edge.src_id == nid {
                all_edges.push(edge.clone());
            }
        }
    }

    // 3. Write individual .md files
    let mut files_written = 0usize;
    for node in &export_nodes {
        let entry = store.get_by_id(node.id).map_err(|e| e.to_string())?;
        let mut md = crate::memory::obsidian_export::render_markdown(&entry);

        // Append graph-edges section
        if !node.edges.is_empty() {
            md.push_str("\n## Graph Edges\n\n");
            for edge in &node.edges {
                let other = if edge.src_id == node.id {
                    edge.dst_id
                } else {
                    edge.src_id
                };
                let direction = if edge.src_id == node.id {
                    "→"
                } else {
                    "←"
                };
                md.push_str(&format!(
                    "- {} `{}` (id={}, confidence={:.2})\n",
                    direction, edge.rel_type, other, edge.confidence
                ));
            }
        }

        let slug = crate::memory::obsidian_export::slugify(&entry.content);
        let filename = if slug.is_empty() {
            format!("{}.md", node.id)
        } else {
            format!("{}-{}.md", node.id, slug)
        };
        std::fs::write(out_path.join(&filename), md)
            .map_err(|e| format!("write {filename}: {e}"))?;
        files_written += 1;
    }

    // 4. Write _graph.json manifest
    let manifest = serde_json::json!({
        "version": 1,
        "root_ids": root_ids,
        "max_hops": hops,
        "nodes": export_nodes.iter().map(|n| serde_json::json!({
            "id": n.id,
            "tags": n.tags,
            "importance": n.importance,
            "memory_type": n.memory_type,
            "content_file": format!("{}-{}.md",
                n.id,
                crate::memory::obsidian_export::slugify(&n.content)
            ),
        })).collect::<Vec<_>>(),
        "edges": all_edges.iter().map(|e| serde_json::json!({
            "src_id": e.src_id,
            "dst_id": e.dst_id,
            "rel_type": e.rel_type,
            "confidence": e.confidence,
        })).collect::<Vec<_>>(),
    });
    std::fs::write(
        out_path.join("_graph.json"),
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("write _graph.json: {e}"))?;

    Ok(KgSubtreeExportResult {
        root_ids,
        nodes_exported: export_nodes.len(),
        edges_exported: all_edges.len(),
        files_written,
        output_dir,
    })
}

/// Result of importing a text file into the knowledge graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileToKgResult {
    pub file_path: String,
    pub chunks_created: usize,
    pub edges_created: usize,
    pub root_id: Option<i64>,
    pub summary: String,
}

/// Import a text file into the knowledge graph following the
/// brain-advanced-design.md pipeline:
///
/// 1. Read the file and split into semantic chunks (markdown-aware or
///    plain text depending on extension)
/// 2. Store each chunk as a `MemoryType::Fact` entry at `importance` 3
///    (configurable), tagged `kg-import,<label>`
/// 3. Create `follows` edges between consecutive chunks (preserving
///    document order)
/// 4. Create a root summary node linking to all chunks via `contains`
///    edges — the document becomes a small subgraph in the KG
///
/// This is the KG-aware counterpart of context-folder ingestion: instead
/// of flat low-importance chunks, you get a structured graph with typed
/// edges and a summary hub node.
#[tauri::command(rename_all = "camelCase")]
pub async fn import_file_to_knowledge_graph(
    file_path: String,
    label: Option<String>,
    importance: Option<i64>,
    state: State<'_, AppState>,
) -> Result<FileToKgResult, String> {
    use crate::memory::chunking::{split_markdown, split_text, DEFAULT_CHUNK_CHARS};
    use crate::memory::edges::{EdgeSource, NewMemoryEdge};
    use crate::memory::{MemoryType, NewMemory};

    let path_buf = std::path::PathBuf::from(&file_path);
    if !path_buf.exists() || !path_buf.is_file() {
        return Err(format!("File not found: {file_path}"));
    }

    let content = std::fs::read_to_string(&path_buf)
        .map_err(|e| format!("read {file_path}: {e}"))?;
    if content.trim().is_empty() {
        return Err("File is empty".into());
    }

    let basename = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("import");
    let label = label.unwrap_or_else(|| {
        path_buf.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("import")
            .to_string()
    });
    let imp = importance.unwrap_or(3).clamp(1, 5);
    let tags = format!("kg-import,{label}");

    // Choose chunker based on extension
    let ext = path_buf
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let chunks = if ext == "md" || ext == "markdown" {
        split_markdown(&content, DEFAULT_CHUNK_CHARS)
    } else {
        split_text(&content, DEFAULT_CHUNK_CHARS)
    };

    if chunks.is_empty() {
        return Err("No chunks produced from file content".into());
    }

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;

    // Create a root summary node for the document
    let summary_content = {
        let preview: String = content.chars().take(300).collect();
        format!(
            "# Document: {basename}\n\nSource: {file_path}\nChunks: {}\n\n{preview}{}",
            chunks.len(),
            if content.len() > 300 { "…" } else { "" }
        )
    };
    let root_entry = store
        .add(NewMemory {
            content: summary_content,
            tags: format!("{tags},document-root"),
            importance: (imp + 1).min(5),
            memory_type: MemoryType::Summary,
            source_url: Some(file_path.clone()),
            source_hash: None,
            ..Default::default()
        })
        .map_err(|e| format!("add root: {e}"))?;
    let root_id = root_entry.id;

    // Store each chunk and track IDs
    let mut chunk_ids: Vec<i64> = Vec::with_capacity(chunks.len());
    for chunk in &chunks {
        let chunk_tags = if let Some(ref heading) = chunk.heading {
            format!("{tags},{heading}")
        } else {
            tags.clone()
        };
        let entry = store
            .add(NewMemory {
                content: chunk.text.clone(),
                tags: chunk_tags,
                importance: imp,
                memory_type: MemoryType::Fact,
                source_url: Some(file_path.clone()),
                source_hash: Some(chunk.hash.clone()),
                ..Default::default()
            })
            .map_err(|e| format!("add chunk {}: {e}", chunk.index))?;
        chunk_ids.push(entry.id);
    }

    // Create edges
    let mut new_edges: Vec<NewMemoryEdge> = Vec::new();

    // root --contains--> each chunk
    for &cid in &chunk_ids {
        new_edges.push(NewMemoryEdge {
            src_id: root_id,
            dst_id: cid,
            rel_type: "contains".to_string(),
            confidence: 1.0,
            source: EdgeSource::Auto,
            valid_from: None,
            valid_to: None,
            edge_source: Some(format!("file-import:{}", normalize_path(&file_path))),
        });
    }

    // chunk[i] --follows--> chunk[i+1]  (document order)
    for pair in chunk_ids.windows(2) {
        new_edges.push(NewMemoryEdge {
            src_id: pair[0],
            dst_id: pair[1],
            rel_type: "follows".to_string(),
            confidence: 1.0,
            source: EdgeSource::Auto,
            valid_from: None,
            valid_to: None,
            edge_source: Some(format!("file-import:{}", normalize_path(&file_path))),
        });
    }

    let edges_created = store
        .add_edges_batch(&new_edges)
        .map_err(|e| format!("add edges: {e}"))?;

    Ok(FileToKgResult {
        file_path,
        chunks_created: chunk_ids.len(),
        edges_created,
        root_id: Some(root_id),
        summary: format!(
            "Imported '{basename}' → 1 root + {} chunks, {} edges (root id={})",
            chunk_ids.len(),
            edges_created,
            root_id
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_supported_files_from_temp_dir() {
        let tmp = std::env::temp_dir().join("ts_ctx_folder_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("sub")).unwrap();
        std::fs::write(tmp.join("readme.md"), "# Hello").unwrap();
        std::fs::write(tmp.join("data.csv"), "a,b,c").unwrap();
        std::fs::write(tmp.join("binary.exe"), [0u8; 10]).unwrap();
        std::fs::write(tmp.join("sub/notes.txt"), "notes").unwrap();
        std::fs::write(tmp.join(".hidden.md"), "secret").unwrap();

        let files = collect_supported_files(&tmp).unwrap();
        assert!(files.iter().any(|f| f.contains("readme.md")));
        assert!(files.iter().any(|f| f.contains("data.csv")));
        assert!(files.iter().any(|f| f.contains("notes.txt")));
        // .exe should be excluded
        assert!(!files.iter().any(|f| f.contains("binary.exe")));
        // hidden files should be excluded
        assert!(!files.iter().any(|f| f.contains(".hidden.md")));
        assert_eq!(files.len(), 3);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn collect_skips_node_modules() {
        let tmp = std::env::temp_dir().join("ts_ctx_folder_nm_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("node_modules/pkg")).unwrap();
        std::fs::write(tmp.join("node_modules/pkg/readme.md"), "# Dep").unwrap();
        std::fs::write(tmp.join("real.md"), "content").unwrap();

        let files = collect_supported_files(&tmp).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].contains("real.md"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn normalize_path_works() {
        assert_eq!(
            normalize_path("D:\\Docs\\Notes"),
            normalize_path("d:/docs/notes")
        );
    }

    #[test]
    fn supported_extensions_list() {
        // Ensure common doc types are covered
        for ext in &["md", "txt", "csv", "json", "xml", "html", "pdf", "rst"] {
            assert!(
                SUPPORTED_EXTENSIONS.contains(ext),
                "Missing extension: {ext}"
            );
        }
    }
}
