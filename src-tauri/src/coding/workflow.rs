//! Reusable coding-task workflow.
//!
//! All TerranSoul coding flows (self-improve planner, ad-hoc chat coding
//! tasks, future agent-driven refactors) share **one** entry point:
//! [`run_coding_task`]. This module:
//!
//! * Loads project rules + instructions + docs from disk so every task
//!   is anchored to the same source of truth (the bundled `rules/`,
//!   `instructions/`, and `docs/` directories — see
//!   `rules/prompting-rules.md` for the rationale).
//! * Builds an XML-structured prompt via
//!   [`super::prompting::CodingPrompt`] applying all ten Anthropic
//!   prompt-engineering principles uniformly.
//! * Calls the configured Coding LLM via the existing OpenAI-compatible
//!   client.
//! * Extracts the structured reply from the model's enclosing XML tag
//!   so callers receive a clean payload (no fences, no preamble).
//!
//! Callers (the self-improve engine, the chat path, future agents) do
//! not need to construct prompts themselves; they describe the task
//! and choose an [`OutputShape`].

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::client::client_from;
use super::handoff::{
    build_handoff_block, emit_handoff_seed_instruction, parse_handoff_reply, HandoffState,
};
use super::prompting::{extract_tag, CodingPrompt, DocSnippet, OutputShape};
use super::{CodingLlmConfig, CodingWorkflowConfig};

/// Maximum total characters of context loaded from `rules/`, `instructions/`,
/// and `docs/` per task. Caps the prompt size on huge repos.
///
/// This is the **default** when no [`CodingWorkflowConfig`] is supplied
/// — most callers should pass an explicit config so the cap is
/// user-configurable.
pub const MAX_CONTEXT_CHARS: usize = 30_000;

/// Per-file character cap when auto-loading rules / instructions / docs.
///
/// This is the **default** when no [`CodingWorkflowConfig`] is supplied.
pub const MAX_FILE_CHARS: usize = 4_000;

/// A single coding task to run through the shared workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingTask {
    /// One-line task identifier used for logging and progress events.
    pub id: String,
    /// Human-readable task description that becomes the `<task>` body.
    pub description: String,
    /// Optional repository root used to auto-load rules + instructions
    /// + docs as supplementary `<document>` blocks. When `None`, no
    ///   context is loaded.
    pub repo_root: Option<PathBuf>,
    /// Whether to load `rules/*.md` as context (recommended).
    #[serde(default = "default_true")]
    pub include_rules: bool,
    /// Whether to load `instructions/*.md` as context (recommended).
    #[serde(default = "default_true")]
    pub include_instructions: bool,
    /// Whether to load `docs/*.md` as context (recommended for design /
    /// brain-related tasks; can be disabled when the task is purely
    /// mechanical).
    #[serde(default = "default_true")]
    pub include_docs: bool,
    /// Output contract for the model's reply.
    pub output_kind: TaskOutputKind,
    /// Optional extra documents (e.g. failing test output, source
    /// snippets, commit messages) injected after the auto-loaded ones.
    #[serde(default)]
    pub extra_documents: Vec<TaskDocument>,
    /// Optional in-memory handoff state to inject as a `[RESUMING
    /// SESSION]` block at the head of the prompt. The Tauri command
    /// layer loads this from disk via
    /// [`super::handoff_store::load_handoff`] when the caller passes a
    /// `handoffSessionId`; pure callers can populate it themselves.
    ///
    /// When `Some`, the task description is also suffixed with the
    /// seed-emission instruction so the model is asked to produce a
    /// `<next_session_seed>` JSON payload for the next call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_handoff: Option<HandoffState>,
}

fn default_true() -> bool {
    true
}

/// Wire-format mirror of [`DocSnippet`] for Tauri command transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDocument {
    pub label: String,
    pub body: String,
}

impl From<TaskDocument> for DocSnippet {
    fn from(d: TaskDocument) -> Self {
        DocSnippet {
            label: d.label,
            body: d.body,
        }
    }
}

/// Serialisable mirror of [`OutputShape`] for Tauri transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskOutputKind {
    NumberedPlan { max_steps: u8 },
    StrictJson { schema_description: String },
    BareFileContents,
    Prose,
}

impl From<TaskOutputKind> for OutputShape {
    fn from(k: TaskOutputKind) -> Self {
        match k {
            TaskOutputKind::NumberedPlan { max_steps } => OutputShape::NumberedPlan { max_steps },
            TaskOutputKind::StrictJson { schema_description } => {
                OutputShape::StrictJson { schema_description }
            }
            TaskOutputKind::BareFileContents => OutputShape::BareFileContents,
            TaskOutputKind::Prose => OutputShape::Prose,
        }
    }
}

/// Result of running a coding task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingTaskResult {
    /// Echoed task id for caller correlation.
    pub task_id: String,
    /// The full raw reply from the model (useful for debugging).
    pub raw_reply: String,
    /// Extracted payload from inside the contracted output tag, or the
    /// raw reply when [`TaskOutputKind::Prose`] was requested.
    pub payload: String,
    /// Whether the model returned a properly-tagged reply matching the
    /// requested output contract.
    pub well_formed: bool,
    /// Number of context documents auto-loaded into the prompt.
    pub context_doc_count: usize,
    /// Parsed `<next_session_seed>` from the model's reply, if the task
    /// asked for one and the model produced a well-formed payload.
    /// Persistence to disk is the caller's responsibility (the Tauri
    /// command does it via `handoff_store::save_handoff`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_handoff: Option<HandoffState>,
}

/// Run a single coding task through the shared workflow.
///
/// `workflow_config` controls which files are auto-loaded into the
/// `<documents>` block. Pass `None` to use [`CodingWorkflowConfig::default`].
pub async fn run_coding_task(
    config: &CodingLlmConfig,
    task: &CodingTask,
    workflow_config: Option<&CodingWorkflowConfig>,
) -> Result<CodingTaskResult, String> {
    let default_cfg = CodingWorkflowConfig::default();
    let workflow_cfg = workflow_config.unwrap_or(&default_cfg);

    // Auto-load context.
    let mut documents: Vec<DocSnippet> = Vec::new();

    // Handoff block goes FIRST so the model re-grounds before reading
    // any other documents. It's small (≤4 KiB) and deterministic.
    if let Some(prior) = &task.prior_handoff {
        documents.push(DocSnippet {
            label: "resuming_session".to_string(),
            body: build_handoff_block(prior),
        });
    }

    if let Some(root) = &task.repo_root {
        let auto = load_workflow_context(
            root,
            workflow_cfg,
            task.include_rules,
            task.include_instructions,
            task.include_docs,
        );
        documents.extend(auto);
    }

    // Append caller-supplied documents.
    documents.extend(task.extra_documents.iter().cloned().map(DocSnippet::from));
    let context_doc_count = documents.len();

    // Build the prompt.
    let output: OutputShape = task.output_kind.clone().into();
    let prefill = match &output {
        OutputShape::Prose => None,
        _ => Some("<analysis>".to_string()),
    };

    // When a prior handoff is in play we ask the model to emit a fresh
    // seed at the end of its reply. The instruction is appended to the
    // task description so it lands inside the user message — easier to
    // keep within token budgets than mutating the role.
    let task_description = if task.prior_handoff.is_some() {
        format!(
            "{}\n\n{}",
            task.description.trim_end(),
            emit_handoff_seed_instruction().trim()
        )
    } else {
        task.description.clone()
    };

    let prompt = CodingPrompt {
        role: default_coding_role(),
        task: task_description,
        negative_constraints: default_negative_constraints(),
        documents,
        output: output.clone(),
        example: None,
        assistant_prefill: prefill,
        error_handling: default_error_handling(),
    };
    let messages = prompt.build();

    // Call the LLM.
    let client = client_from(config);
    let raw_reply = client.chat(messages).await?;

    // Extract the contracted output tag.
    let (payload, well_formed) = match &output {
        OutputShape::NumberedPlan { .. } => extract_or_raw(&raw_reply, "plan"),
        OutputShape::StrictJson { .. } => extract_or_raw(&raw_reply, "json"),
        OutputShape::BareFileContents => extract_or_raw(&raw_reply, "file"),
        OutputShape::Prose => (raw_reply.trim().to_string(), true),
    };

    let next_handoff = if task.prior_handoff.is_some() {
        parse_handoff_reply(&raw_reply)
    } else {
        None
    };

    Ok(CodingTaskResult {
        task_id: task.id.clone(),
        raw_reply,
        payload,
        well_formed,
        context_doc_count,
        next_handoff,
    })
}

/// Public, provider-agnostic context loader used by every coding
/// workflow (the `run_coding_task` runner, the self-improve planner,
/// and any future agent).
///
/// Walks the directories listed in `cfg.include_dirs` (when the
/// matching `include_*` flag is true) and reads each `*.md` file,
/// skipping anything in `cfg.exclude_paths`. Then loads the explicit
/// files listed in `cfg.include_files`. Files are truncated at
/// `cfg.max_file_chars` and the loader stops once `cfg.max_total_chars`
/// is reached.
///
/// `include_rules` / `include_instructions` / `include_docs` map to the
/// matching directory names in `cfg.include_dirs` (case-sensitive). A
/// directory not listed in `cfg.include_dirs` is never loaded — the
/// flags only control which of the configured directories participate
/// in *this particular* task.
pub fn load_workflow_context(
    repo_root: &Path,
    cfg: &CodingWorkflowConfig,
    include_rules: bool,
    include_instructions: bool,
    include_docs: bool,
) -> Vec<DocSnippet> {
    let mut documents: Vec<DocSnippet> = Vec::new();
    let mut total_chars = 0usize;

    for dir_name in &cfg.include_dirs {
        let allow = match dir_name.as_str() {
            "rules" => include_rules,
            "instructions" => include_instructions,
            "docs" => include_docs,
            _ => true, // Other configured dirs are always on.
        };
        if !allow {
            continue;
        }
        load_dir(
            &repo_root.join(dir_name),
            cfg,
            &mut documents,
            &mut total_chars,
        );
        if total_chars >= cfg.max_total_chars {
            break;
        }
    }

    // Explicit files (e.g. README.md, AGENTS.md) loaded last so they
    // are visible even when a directory loader hits the cap.
    for rel in &cfg.include_files {
        if total_chars >= cfg.max_total_chars {
            break;
        }
        if is_excluded(rel, &cfg.exclude_paths) {
            continue;
        }
        let path = repo_root.join(rel);
        if let Some(snippet) = read_truncated(&path, rel, cfg) {
            total_chars += snippet.body.chars().count();
            documents.push(snippet);
        }
    }

    documents
}

fn extract_or_raw(reply: &str, tag: &str) -> (String, bool) {
    match extract_tag(reply, tag) {
        Some(body) => (body.to_string(), true),
        None => (reply.trim().to_string(), false),
    }
}

/// Default job-description-style role applied to every coding task
/// (Anthropic principle 2). Calls out the tools and priorities the
/// model should optimise for.
pub fn default_coding_role() -> String {
    "Senior software engineer working on TerranSoul, a Vue 3 + Tauri 2.x \
     desktop AI companion with a Rust backend. Specialised in Rust (stable, \
     MSRV 1.80+), TypeScript 5.x, and Pinia state management. Tools \
     available: cargo, clippy, rustfmt, vitest, vue-tsc, playwright, \
     git. Priorities, in order: (1) correctness and test coverage, \
     (2) adherence to the project's rules in `rules/*.md` and \
     `instructions/*.md`, (3) using existing crates / npm packages over \
     reinventing wheels, (4) readability over cleverness."
        .to_string()
}

/// Default negative constraints applied to every coding task
/// (Anthropic principle 5). Project-wide invariants that no task may
/// violate.
pub fn default_negative_constraints() -> Vec<String> {
    vec![
        "Do not invent file paths. Only suggest edits to paths visible in the supplied documents or rooted at src/, src-tauri/src/, rules/, docs/, instructions/.".to_string(),
        "Do not produce TODO, placeholder, or pretend code. Every line must compile and function.".to_string(),
        "Do not use .unwrap() or .expect() in library Rust code.".to_string(),
        "Do not introduce regex-based AI routing or keyword arrays driving behaviour (see rules/llm-decision-rules.md).".to_string(),
        "Do not bypass safety checks (no --no-verify, no force-push, no destructive shortcuts).".to_string(),
        "Do not duplicate functionality available in an existing crate or npm package.".to_string(),
    ]
}

/// Default error-handling guidance (Anthropic principle 9).
pub fn default_error_handling() -> Vec<String> {
    vec![
        "If the supplied documents conflict, cite both by their <document index> and choose the rule from rules/*.md.".to_string(),
        "If the task is ambiguous, write your best interpretation in <analysis>, then proceed.".to_string(),
        "If a required input is missing, say so explicitly inside <analysis> and stop.".to_string(),
        "If you would need to break a negative constraint to complete the task, refuse and explain in <analysis>.".to_string(),
    ]
}

/// Load all `*.md` files under a directory (non-recursive) into the
/// document list, respecting per-file and total character caps from
/// `cfg`, and skipping anything in `cfg.exclude_paths`.
fn load_dir(
    dir: &Path,
    cfg: &CodingWorkflowConfig,
    documents: &mut Vec<DocSnippet>,
    total_chars: &mut usize,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|x| x.to_str()) == Some("md"))
        .collect();
    // Sort for deterministic prompts.
    paths.sort();

    for path in paths {
        if *total_chars >= cfg.max_total_chars {
            break;
        }
        let label = path
            .strip_prefix(dir.parent().unwrap_or(dir))
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });
        if is_excluded(&label, &cfg.exclude_paths) {
            continue;
        }
        if let Some(snippet) = read_truncated(&path, &label, cfg) {
            *total_chars += snippet.body.chars().count();
            documents.push(snippet);
        }
    }
}

/// Read a single file and return a truncated [`DocSnippet`], or `None`
/// when the path cannot be read.
fn read_truncated(path: &Path, label: &str, cfg: &CodingWorkflowConfig) -> Option<DocSnippet> {
    let body = std::fs::read_to_string(path).ok()?;
    let truncated = if body.chars().count() > cfg.max_file_chars {
        let head: String = body.chars().take(cfg.max_file_chars).collect();
        format!(
            "{head}\n… [file truncated to {} chars]",
            cfg.max_file_chars
        )
    } else {
        body
    };
    Some(DocSnippet {
        label: label.to_string(),
        body: truncated,
    })
}

/// Returns true when `path` (which may be an absolute label, a
/// repo-relative path, or a bare file name) matches any entry in
/// `exclude`. Matching is case-sensitive and uses normalised forward
/// slashes.
fn is_excluded(path: &str, exclude: &[String]) -> bool {
    let normalised = path.replace('\\', "/");
    let basename = normalised.rsplit('/').next().unwrap_or(&normalised);
    exclude.iter().any(|pat| {
        let pat_norm = pat.replace('\\', "/");
        normalised == pat_norm
            || normalised.ends_with(&format!("/{pat_norm}"))
            || basename == pat_norm
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, name: &str, body: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn load_dir_picks_up_markdown_only() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("rules"), "a.md", "rule a");
        write(&tmp.path().join("rules"), "b.md", "rule b");
        write(&tmp.path().join("rules"), "c.txt", "ignored");

        let cfg = CodingWorkflowConfig::default();
        let mut docs = Vec::new();
        let mut total = 0usize;
        load_dir(&tmp.path().join("rules"), &cfg, &mut docs, &mut total);
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().any(|d| d.label.ends_with("a.md")));
        assert!(docs.iter().any(|d| d.label.ends_with("b.md")));
    }

    #[test]
    fn load_dir_truncates_oversized_files() {
        let tmp = TempDir::new().unwrap();
        let huge = "x".repeat(MAX_FILE_CHARS + 200);
        write(&tmp.path().join("rules"), "huge.md", &huge);

        let cfg = CodingWorkflowConfig::default();
        let mut docs = Vec::new();
        let mut total = 0usize;
        load_dir(&tmp.path().join("rules"), &cfg, &mut docs, &mut total);
        assert_eq!(docs.len(), 1);
        assert!(docs[0].body.contains("[file truncated"));
    }

    #[test]
    fn load_dir_respects_total_cap() {
        let tmp = TempDir::new().unwrap();
        // Each file just under MAX_FILE_CHARS so total grows quickly.
        let body = "y".repeat(MAX_FILE_CHARS - 1);
        for i in 0..20 {
            write(&tmp.path().join("rules"), &format!("f{i}.md"), &body);
        }
        let cfg = CodingWorkflowConfig::default();
        let mut docs = Vec::new();
        let mut total = 0usize;
        load_dir(&tmp.path().join("rules"), &cfg, &mut docs, &mut total);
        // Should stop before reading all 20 files (~ 20 * 4000 = 80_000 > 30_000).
        assert!(total <= MAX_CONTEXT_CHARS + MAX_FILE_CHARS);
        assert!(docs.len() < 20);
    }

    #[test]
    fn load_dir_is_no_op_for_missing_path() {
        let tmp = TempDir::new().unwrap();
        let cfg = CodingWorkflowConfig::default();
        let mut docs = Vec::new();
        let mut total = 0usize;
        load_dir(&tmp.path().join("missing"), &cfg, &mut docs, &mut total);
        assert!(docs.is_empty());
        assert_eq!(total, 0);
    }

    #[test]
    fn load_dir_skips_excluded_paths_by_basename() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("rules"), "keep.md", "keep me");
        write(&tmp.path().join("rules"), "skip.md", "skip me");
        let cfg = CodingWorkflowConfig {
            exclude_paths: vec!["skip.md".to_string()],
            ..CodingWorkflowConfig::default()
        };
        let mut docs = Vec::new();
        let mut total = 0usize;
        load_dir(&tmp.path().join("rules"), &cfg, &mut docs, &mut total);
        assert_eq!(docs.len(), 1);
        assert!(docs[0].label.ends_with("keep.md"));
    }

    #[test]
    fn load_workflow_context_loads_all_three_dirs_and_explicit_files() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("rules"), "r.md", "r");
        write(&tmp.path().join("instructions"), "i.md", "i");
        write(&tmp.path().join("docs"), "d.md", "d");
        std::fs::write(tmp.path().join("README.md"), "readme").unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "agents").unwrap();

        let cfg = CodingWorkflowConfig::default();
        let docs = load_workflow_context(tmp.path(), &cfg, true, true, true);
        let labels: Vec<&str> = docs.iter().map(|d| d.label.as_str()).collect();
        assert!(labels.iter().any(|l| l.ends_with("r.md")));
        assert!(labels.iter().any(|l| l.ends_with("i.md")));
        assert!(labels.iter().any(|l| l.ends_with("d.md")));
        assert!(labels.iter().any(|l| l == &"README.md"));
        assert!(labels.iter().any(|l| l == &"AGENTS.md"));
    }

    #[test]
    fn load_workflow_context_respects_per_task_dir_flags() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("rules"), "r.md", "r");
        write(&tmp.path().join("instructions"), "i.md", "i");
        write(&tmp.path().join("docs"), "d.md", "d");

        let cfg = CodingWorkflowConfig {
            include_files: Vec::new(),
            ..CodingWorkflowConfig::default()
        };
        let docs = load_workflow_context(tmp.path(), &cfg, true, false, false);
        assert_eq!(docs.len(), 1);
        assert!(docs[0].label.ends_with("r.md"));
    }

    #[test]
    fn load_workflow_context_supports_custom_dirs() {
        let tmp = TempDir::new().unwrap();
        write(&tmp.path().join("playbooks"), "p.md", "playbook");

        let cfg = CodingWorkflowConfig {
            include_dirs: vec!["playbooks".to_string()],
            include_files: Vec::new(),
            ..CodingWorkflowConfig::default()
        };
        let docs = load_workflow_context(tmp.path(), &cfg, true, true, true);
        assert_eq!(docs.len(), 1);
        assert!(docs[0].label.ends_with("p.md"));
    }

    #[test]
    fn is_excluded_matches_basename_and_full_path() {
        assert!(is_excluded("rules/foo.md", &["foo.md".to_string()]));
        assert!(is_excluded("foo.md", &["foo.md".to_string()]));
        assert!(is_excluded(
            "rules/foo.md",
            &["rules/foo.md".to_string()]
        ));
        assert!(!is_excluded("rules/bar.md", &["foo.md".to_string()]));
    }

    #[test]
    fn extract_or_raw_returns_well_formed_when_tag_present() {
        let (payload, ok) = extract_or_raw("noise <plan>1. step</plan> tail", "plan");
        assert!(ok);
        assert_eq!(payload, "1. step");
    }

    #[test]
    fn extract_or_raw_returns_raw_when_tag_missing() {
        let (payload, ok) = extract_or_raw("just text", "plan");
        assert!(!ok);
        assert_eq!(payload, "just text");
    }

    #[test]
    fn default_role_mentions_tooling() {
        let role = default_coding_role();
        assert!(role.contains("cargo"));
        assert!(role.contains("vitest"));
        assert!(role.contains("Rust"));
    }

    #[test]
    fn default_constraints_forbid_unwrap_and_placeholders() {
        let constraints = default_negative_constraints();
        assert!(constraints.iter().any(|c| c.contains("unwrap")));
        assert!(constraints.iter().any(|c| c.contains("placeholder")));
    }

    #[test]
    fn task_output_kind_round_trips_to_output_shape() {
        let kind = TaskOutputKind::NumberedPlan { max_steps: 7 };
        let shape: OutputShape = kind.into();
        match shape {
            OutputShape::NumberedPlan { max_steps } => assert_eq!(max_steps, 7),
            _ => panic!("wrong shape"),
        }
    }
}
