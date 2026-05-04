//! Multi-agent workflow system for TerranSoul.
//!
//! Defines specialized agent roles (Planner, Coder, Reviewer, Tester,
//! Researcher, Orchestrator) that collaborate through YAML-serialised
//! workflow plans. Each step in a plan specifies which agent role
//! performs the work, which LLM to use, dependencies on other steps,
//! and expected output shape.
//!
//! ## Design Principles (absorbed from Anthropic, CrewAI, AutoGen)
//!
//! 1. **Orchestrator-workers** — A planner decomposes the user request
//!    into a DAG of steps; specialized agents execute each step.
//! 2. **Evaluator-optimizer** — Reviewer agents provide feedback loops.
//! 3. **Parallelization** — Independent steps execute concurrently.
//! 4. **Human-in-the-loop** — Users can review, edit, re-assign LLMs,
//!    and approve/reject before execution proceeds.
//! 5. **Transparency** — All agent reasoning is logged and visible.
//!
//! ## Integration Points
//!
//! - **Brain/RAG** — Researcher agent uses `brain_search` + `brain_ingest`.
//! - **MCP** — External AI tools can enqueue workflows via MCP `brain_ingest`.
//! - **gRPC** — Phone surface can list/monitor/cancel workflows.
//! - **Self-Improve** — Coding workflows reuse `run_coding_task`.
//! - **Task Queue** — Durable persistence via existing `task_queue.rs`.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Agent Roles
// ---------------------------------------------------------------------------

/// Specialized agent roles in the multi-agent system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Decomposes requests into step-by-step plans.
    Planner,
    /// Writes/modifies code.
    Coder,
    /// Reviews code for quality, bugs, security.
    Reviewer,
    /// Runs tests and validates correctness.
    Tester,
    /// Gathers information, searches docs/web/brain.
    Researcher,
    /// Coordinates flow between agents; handles routing.
    Orchestrator,
}

impl AgentRole {
    /// Human-readable display name.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Planner => "Planner",
            Self::Coder => "Coder",
            Self::Reviewer => "Reviewer",
            Self::Tester => "Tester",
            Self::Researcher => "Researcher",
            Self::Orchestrator => "Orchestrator",
        }
    }

    /// Default system prompt preamble for this role.
    pub fn system_preamble(self) -> &'static str {
        match self {
            Self::Planner => "You are a meticulous task planner. Break complex requests into clear, actionable steps. Output a structured YAML plan. Consider dependencies between steps and which specialist agent should handle each one.",
            Self::Coder => "You are an expert software engineer. Write clean, idiomatic, well-tested code. Follow project conventions. Never leave placeholder or TODO code.",
            Self::Reviewer => "You are a thorough code reviewer. Check for bugs, security issues, performance problems, and style violations. Provide specific, actionable feedback with file and line references.",
            Self::Tester => "You are a testing specialist. Design comprehensive test cases, run test suites, and report results clearly. Cover edge cases and failure modes.",
            Self::Researcher => "You are a skilled researcher. Gather relevant information from documentation, codebases, and knowledge bases. Synthesize findings into concise, actionable summaries.",
            Self::Orchestrator => "You are a workflow orchestrator. Coordinate between specialist agents, decide when to escalate to the user, and ensure the overall goal is achieved efficiently.",
        }
    }

    /// Recommended LLM tiers for each role (from cheap/fast to powerful).
    pub fn recommended_llms(self) -> &'static [LlmRecommendation] {
        match self {
            Self::Planner => &[
                LlmRecommendation {
                    model: "gemma-3:4b",
                    provider: "ollama",
                    tier: LlmTier::Fast,
                    reason: "Lightweight planning for simple tasks",
                },
                LlmRecommendation {
                    model: "qwen3:8b",
                    provider: "ollama",
                    tier: LlmTier::Balanced,
                    reason: "Good reasoning for complex decomposition",
                },
                LlmRecommendation {
                    model: "claude-sonnet-4-20250514",
                    provider: "anthropic",
                    tier: LlmTier::Premium,
                    reason: "Best-in-class planning and decomposition",
                },
            ],
            Self::Coder => &[
                LlmRecommendation {
                    model: "qwen2.5-coder:7b",
                    provider: "ollama",
                    tier: LlmTier::Fast,
                    reason: "Fast local coding for simple changes",
                },
                LlmRecommendation {
                    model: "deepseek-coder-v2:16b",
                    provider: "ollama",
                    tier: LlmTier::Balanced,
                    reason: "Strong coding with good context",
                },
                LlmRecommendation {
                    model: "claude-sonnet-4-20250514",
                    provider: "anthropic",
                    tier: LlmTier::Premium,
                    reason: "Excellent for complex multi-file changes",
                },
            ],
            Self::Reviewer => &[
                LlmRecommendation {
                    model: "gemma-3:4b",
                    provider: "ollama",
                    tier: LlmTier::Fast,
                    reason: "Quick lint-level review",
                },
                LlmRecommendation {
                    model: "qwen3:8b",
                    provider: "ollama",
                    tier: LlmTier::Balanced,
                    reason: "Solid review with reasoning",
                },
                LlmRecommendation {
                    model: "claude-sonnet-4-20250514",
                    provider: "anthropic",
                    tier: LlmTier::Premium,
                    reason: "Deep architectural review",
                },
            ],
            Self::Tester => &[
                LlmRecommendation {
                    model: "gemma-3:4b",
                    provider: "ollama",
                    tier: LlmTier::Fast,
                    reason: "Parse test output quickly",
                },
                LlmRecommendation {
                    model: "qwen2.5-coder:7b",
                    provider: "ollama",
                    tier: LlmTier::Balanced,
                    reason: "Generate test cases",
                },
                LlmRecommendation {
                    model: "claude-sonnet-4-20250514",
                    provider: "anthropic",
                    tier: LlmTier::Premium,
                    reason: "Complex test design",
                },
            ],
            Self::Researcher => &[
                LlmRecommendation {
                    model: "gemma-3:4b",
                    provider: "ollama",
                    tier: LlmTier::Fast,
                    reason: "Quick document summarization",
                },
                LlmRecommendation {
                    model: "qwen3:8b",
                    provider: "ollama",
                    tier: LlmTier::Balanced,
                    reason: "Research synthesis",
                },
                LlmRecommendation {
                    model: "claude-sonnet-4-20250514",
                    provider: "anthropic",
                    tier: LlmTier::Premium,
                    reason: "Deep analysis and reasoning",
                },
            ],
            Self::Orchestrator => &[
                LlmRecommendation {
                    model: "gemma-3:4b",
                    provider: "ollama",
                    tier: LlmTier::Fast,
                    reason: "Simple routing decisions",
                },
                LlmRecommendation {
                    model: "qwen3:8b",
                    provider: "ollama",
                    tier: LlmTier::Balanced,
                    reason: "Complex orchestration logic",
                },
                LlmRecommendation {
                    model: "claude-sonnet-4-20250514",
                    provider: "anthropic",
                    tier: LlmTier::Premium,
                    reason: "Nuanced multi-step coordination",
                },
            ],
        }
    }

    /// All roles in definition order.
    pub fn all() -> &'static [AgentRole] {
        &[
            Self::Planner,
            Self::Coder,
            Self::Reviewer,
            Self::Tester,
            Self::Researcher,
            Self::Orchestrator,
        ]
    }
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

// ---------------------------------------------------------------------------
// LLM Recommendation
// ---------------------------------------------------------------------------

/// Cost/speed tier for LLM selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmTier {
    /// Fast, cheap, local. Good for simple tasks.
    Fast,
    /// Balanced speed/quality. Good default.
    Balanced,
    /// Highest quality, may be slower/costlier.
    Premium,
}

/// A recommended LLM for a specific agent role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRecommendation {
    pub model: &'static str,
    pub provider: &'static str,
    pub tier: LlmTier,
    pub reason: &'static str,
}

// ---------------------------------------------------------------------------
// Workflow Plan (YAML-serialisable)
// ---------------------------------------------------------------------------

/// The kind of workflow being executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowKind {
    /// A coding/development task (writes code, runs tests).
    Coding,
    /// A recurring daily task (check email summaries, review PRs, etc.).
    Daily,
    /// A one-time task (research, write a doc, etc.).
    OneTime,
}

/// Status of a workflow plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowPlanStatus {
    /// Plan generated, awaiting user review.
    PendingReview,
    /// User approved, ready to execute.
    Approved,
    /// Currently executing.
    Running,
    /// Paused by user or waiting for input.
    Paused,
    /// All steps completed successfully.
    Completed,
    /// Failed with errors.
    Failed,
    /// Cancelled by user.
    Cancelled,
}

/// A single step in the workflow plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Unique step ID (e.g. "step-1", "research-api-docs").
    pub id: String,
    /// Which agent role executes this step.
    pub agent: AgentRole,
    /// LLM model to use for this step (user can override).
    pub llm_model: String,
    /// LLM provider (e.g. "ollama", "anthropic", "openai").
    pub llm_provider: String,
    /// Human-readable description of what this step does.
    pub description: String,
    /// IDs of steps that must complete before this one starts.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Expected output format.
    #[serde(default = "default_output_format")]
    pub output_format: StepOutputFormat,
    /// Current execution status.
    #[serde(default)]
    pub status: StepStatus,
    /// Output produced by this step (populated after execution).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Error message if step failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Duration in milliseconds (0 if not yet run).
    #[serde(default)]
    pub duration_ms: u64,
    /// Whether user must approve this step's output before continuing.
    #[serde(default)]
    pub requires_approval: bool,
}

fn default_output_format() -> StepOutputFormat {
    StepOutputFormat::Prose
}

/// Expected output format for a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepOutputFormat {
    /// Free-form text.
    Prose,
    /// Code (files to write/modify).
    Code,
    /// Structured JSON.
    Json,
    /// A sub-plan (YAML).
    Plan,
    /// Test results.
    TestResults,
    /// Boolean pass/fail verdict.
    Verdict,
}

/// Execution status of a single step.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Not yet started.
    #[default]
    Pending,
    /// Waiting for user approval.
    AwaitingApproval,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed.
    Failed,
    /// Skipped (dependency failed or user skipped).
    Skipped,
}

/// The complete workflow plan — the primary data structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPlan {
    /// Unique workflow ID.
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// What kind of workflow this is.
    pub kind: WorkflowKind,
    /// Overall status.
    pub status: WorkflowPlanStatus,
    /// The original user request that triggered this workflow.
    pub user_request: String,
    /// Ordered list of steps.
    pub steps: Vec<WorkflowStep>,
    /// Per-agent LLM overrides (user can change any agent's LLM).
    #[serde(default)]
    pub agent_llm_overrides: HashMap<AgentRole, AgentLlmConfig>,
    /// When the plan was created (Unix epoch ms).
    pub created_at: u64,
    /// Last modification timestamp (Unix epoch ms).
    pub updated_at: u64,
    /// Optional tags for filtering.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional schedule for recurring or future-dated workflows.
    /// `None` means run-on-demand (immediately when approved).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<WorkflowSchedule>,
}

/// Per-agent LLM configuration that the user can set/override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLlmConfig {
    pub model: String,
    pub provider: String,
    /// Optional API key override (for cloud providers).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Optional base URL override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

// ---------------------------------------------------------------------------
// Workflow Plan Store (disk persistence)
// ---------------------------------------------------------------------------

/// Lightweight summary for listing workflows without loading full plans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPlanSummary {
    pub id: String,
    pub title: String,
    pub kind: WorkflowKind,
    pub status: WorkflowPlanStatus,
    pub step_count: usize,
    pub completed_steps: usize,
    pub created_at: u64,
    pub updated_at: u64,
    pub tags: Vec<String>,
    /// Schedule summary (next occurrence ms, or None for on-demand).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_occurrence: Option<u64>,
    /// Whether this workflow has any schedule attached.
    #[serde(default)]
    pub recurring: bool,
}

impl From<&WorkflowPlan> for WorkflowPlanSummary {
    fn from(plan: &WorkflowPlan) -> Self {
        let completed_steps = plan
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count();
        let next_occurrence = plan
            .schedule
            .as_ref()
            .and_then(|s| s.next_occurrence_after(now_ms()));
        let recurring = plan
            .schedule
            .as_ref()
            .is_some_and(|s| !matches!(s.recurrence, RecurrencePattern::Once));
        Self {
            id: plan.id.clone(),
            title: plan.title.clone(),
            kind: plan.kind,
            status: plan.status,
            step_count: plan.steps.len(),
            completed_steps,
            created_at: plan.created_at,
            updated_at: plan.updated_at,
            tags: plan.tags.clone(),
            next_occurrence,
            recurring,
        }
    }
}

/// Storage directory: `<data_dir>/workflow_plans/`
pub fn plans_dir(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("workflow_plans")
}

/// Save a workflow plan to disk as YAML.
pub fn save_plan(data_dir: &Path, plan: &WorkflowPlan) -> Result<(), String> {
    let dir = plans_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    let path = dir.join(format!("{}.yaml", plan.id));
    let yaml = serde_yaml::to_string(plan).map_err(|e| format!("serialize: {e}"))?;

    // Atomic write via tmp file
    let tmp = dir.join(format!(".{}.tmp", plan.id));
    std::fs::write(&tmp, yaml.as_bytes()).map_err(|e| format!("write tmp: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("rename: {e}"))?;
    Ok(())
}

/// Load a workflow plan from disk.
pub fn load_plan(data_dir: &Path, plan_id: &str) -> Result<WorkflowPlan, String> {
    let path = plans_dir(data_dir).join(format!("{plan_id}.yaml"));
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("read plan {plan_id}: {e}"))?;
    serde_yaml::from_str(&content).map_err(|e| format!("parse plan {plan_id}: {e}"))
}

/// List all workflow plans (summaries only).
pub fn list_plans(data_dir: &Path) -> Result<Vec<WorkflowPlanSummary>, String> {
    let dir = plans_dir(data_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut summaries = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("read dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("entry: {e}"))?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "yaml")
            && !path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .starts_with('.')
        {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(plan) = serde_yaml::from_str::<WorkflowPlan>(&content) {
                    summaries.push(WorkflowPlanSummary::from(&plan));
                }
            }
        }
    }
    summaries.sort_by_key(|s| std::cmp::Reverse(s.updated_at));
    Ok(summaries)
}

/// Delete a workflow plan from disk.
pub fn delete_plan(data_dir: &Path, plan_id: &str) -> Result<(), String> {
    let path = plans_dir(data_dir).join(format!("{plan_id}.yaml"));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("delete: {e}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Plan Generation (Planner Agent)
// ---------------------------------------------------------------------------

/// The prompt template for the Planner agent to decompose a user request.
pub const PLANNER_SYSTEM_PROMPT: &str = r#"You are a workflow planner for TerranSoul, a personal AI assistant.

Given a user's request, decompose it into a YAML workflow plan with clear steps.

RULES:
1. Each step must specify: id, agent role, description, dependencies, and output_format.
2. Agent roles: planner, coder, reviewer, tester, researcher, orchestrator.
3. Steps can run in parallel if they don't depend on each other.
4. For coding tasks: always include a reviewer step after coder, and a tester step after reviewer approves.
5. For research tasks: researcher gathers info, then planner synthesizes.
6. Mark critical steps with requires_approval: true so the user can review.
7. Keep plans concise — prefer fewer, well-scoped steps over many tiny ones.
8. Consider the workflow kind: coding (writes code), daily (recurring), one_time (one-off).

OUTPUT FORMAT:
Respond with ONLY a YAML document (no markdown fences) matching this schema:

title: "Short descriptive title"
kind: coding | daily | one_time
tags: [optional, tags]
steps:
  - id: "step-id"
    agent: planner | coder | reviewer | tester | researcher | orchestrator
    description: "What this step does"
    depends_on: []
    output_format: prose | code | json | plan | test_results | verdict
    requires_approval: false
"#;

/// Generate a new workflow plan ID.
pub fn new_plan_id() -> String {
    Uuid::new_v4().to_string()
}

/// Create a blank workflow plan from a user request (before LLM planning).
pub fn create_blank_plan(user_request: &str, kind: WorkflowKind) -> WorkflowPlan {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    WorkflowPlan {
        id: new_plan_id(),
        title: truncate_title(user_request, 80),
        kind,
        status: WorkflowPlanStatus::PendingReview,
        user_request: user_request.to_string(),
        steps: Vec::new(),
        agent_llm_overrides: HashMap::new(),
        created_at: now,
        updated_at: now,
        tags: Vec::new(),
        schedule: None,
    }
}

/// Parse a YAML plan response from the Planner LLM into steps.
pub fn parse_planner_response(yaml_str: &str, plan: &mut WorkflowPlan) -> Result<(), String> {
    // Strip potential markdown fences
    let cleaned = yaml_str
        .trim()
        .trim_start_matches("```yaml")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    #[derive(Deserialize)]
    struct PlannerOutput {
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        kind: Option<WorkflowKind>,
        #[serde(default)]
        tags: Vec<String>,
        steps: Vec<PlannerStep>,
    }

    #[derive(Deserialize)]
    struct PlannerStep {
        id: String,
        agent: AgentRole,
        description: String,
        #[serde(default)]
        depends_on: Vec<String>,
        #[serde(default = "default_output_format")]
        output_format: StepOutputFormat,
        #[serde(default)]
        requires_approval: bool,
    }

    let output: PlannerOutput =
        serde_yaml::from_str(cleaned).map_err(|e| format!("invalid planner YAML: {e}"))?;

    if let Some(title) = output.title {
        plan.title = title;
    }
    if let Some(kind) = output.kind {
        plan.kind = kind;
    }
    if !output.tags.is_empty() {
        plan.tags = output.tags;
    }

    plan.steps = output
        .steps
        .into_iter()
        .map(|s| WorkflowStep {
            id: s.id,
            agent: s.agent,
            llm_model: default_model_for_role(s.agent),
            llm_provider: default_provider_for_role(s.agent),
            description: s.description,
            depends_on: s.depends_on,
            output_format: s.output_format,
            status: StepStatus::Pending,
            output: None,
            error: None,
            duration_ms: 0,
            requires_approval: s.requires_approval,
        })
        .collect();

    Ok(())
}

/// Default model for a role (picks the "balanced" tier).
fn default_model_for_role(role: AgentRole) -> String {
    role.recommended_llms()
        .iter()
        .find(|r| r.tier == LlmTier::Balanced)
        .or_else(|| role.recommended_llms().first())
        .map(|r| r.model.to_string())
        .unwrap_or_else(|| "gemma-3:4b".to_string())
}

/// Default provider for a role.
fn default_provider_for_role(role: AgentRole) -> String {
    role.recommended_llms()
        .iter()
        .find(|r| r.tier == LlmTier::Balanced)
        .or_else(|| role.recommended_llms().first())
        .map(|r| r.provider.to_string())
        .unwrap_or_else(|| "ollama".to_string())
}

/// Truncate a string to max chars, adding "..." if needed.
fn truncate_title(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

// ---------------------------------------------------------------------------
// Plan Validation
// ---------------------------------------------------------------------------

/// Validate a workflow plan for structural correctness.
pub fn validate_plan(plan: &WorkflowPlan) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if plan.steps.is_empty() {
        errors.push("Plan has no steps".to_string());
    }

    // Check for duplicate IDs
    let mut seen_ids = HashSet::new();
    for step in &plan.steps {
        if !seen_ids.insert(&step.id) {
            errors.push(format!("Duplicate step ID: {}", step.id));
        }
    }

    // Check dependency references
    let all_ids: HashSet<&str> = plan.steps.iter().map(|s| s.id.as_str()).collect();
    for step in &plan.steps {
        for dep in &step.depends_on {
            if !all_ids.contains(dep.as_str()) {
                errors.push(format!(
                    "Step '{}' depends on unknown step '{}'",
                    step.id, dep
                ));
            }
        }
    }

    // Check for cycles (simple topological sort attempt)
    if has_cycle(plan) {
        errors.push("Plan contains a dependency cycle".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Simple cycle detection via Kahn's algorithm.
fn has_cycle(plan: &WorkflowPlan) -> bool {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

    for step in &plan.steps {
        in_degree.entry(step.id.as_str()).or_insert(0);
        adj.entry(step.id.as_str()).or_default();
    }
    for step in &plan.steps {
        for dep in &step.depends_on {
            adj.entry(dep.as_str()).or_default().push(step.id.as_str());
            *in_degree.entry(step.id.as_str()).or_insert(0) += 1;
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut visited = 0usize;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        if let Some(neighbors) = adj.get(node) {
            for &next in neighbors {
                if let Some(deg) = in_degree.get_mut(next) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(next);
                    }
                }
            }
        }
    }

    visited < plan.steps.len()
}

// ---------------------------------------------------------------------------
// Scheduling / Calendar (Microsoft Teams-inspired recurrence)
// ---------------------------------------------------------------------------

/// Days of the week (Sunday=0 to match `chrono::Weekday::num_days_from_sunday`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weekday {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl Weekday {
    /// Days since Sunday (Sun=0, Mon=1, ..., Sat=6).
    pub fn num_from_sunday(self) -> u32 {
        match self {
            Self::Sunday => 0,
            Self::Monday => 1,
            Self::Tuesday => 2,
            Self::Wednesday => 3,
            Self::Thursday => 4,
            Self::Friday => 5,
            Self::Saturday => 6,
        }
    }

    pub fn from_num_from_sunday(n: u32) -> Option<Self> {
        match n % 7 {
            0 => Some(Self::Sunday),
            1 => Some(Self::Monday),
            2 => Some(Self::Tuesday),
            3 => Some(Self::Wednesday),
            4 => Some(Self::Thursday),
            5 => Some(Self::Friday),
            6 => Some(Self::Saturday),
            _ => None,
        }
    }
}

/// Recurrence pattern for a workflow schedule.
///
/// Modelled on Microsoft Teams calendar recurrence options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecurrencePattern {
    /// Run once at `start_at`, no repetition.
    Once,
    /// Daily, every N days.
    Daily {
        /// Repeat interval in days (e.g. 1=every day, 2=every other day).
        interval: u32,
    },
    /// Weekly on specific weekdays.
    Weekly {
        /// Repeat interval in weeks.
        interval: u32,
        /// Days of week to fire on.
        weekdays: Vec<Weekday>,
    },
    /// Monthly on a specific day of the month.
    Monthly {
        /// Repeat interval in months.
        interval: u32,
        /// Day of month (1–31). Snaps to last day if month is shorter.
        day_of_month: u32,
    },
}

/// Schedule attached to a workflow plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSchedule {
    /// Start time in Unix epoch milliseconds (UTC).
    pub start_at: u64,
    /// End time / cutoff in Unix epoch ms. `None` = no end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_at: Option<u64>,
    /// Estimated duration in minutes (for calendar display).
    #[serde(default = "default_duration_minutes")]
    pub duration_minutes: u32,
    /// Recurrence pattern.
    pub recurrence: RecurrencePattern,
    /// IANA timezone name (e.g. "America/New_York"). For display only;
    /// all stored times are UTC milliseconds.
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// Last fired occurrence in epoch ms (None if never run).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_fired_at: Option<u64>,
}

fn default_duration_minutes() -> u32 {
    30
}

fn default_timezone() -> String {
    "UTC".to_string()
}

/// Current time in epoch ms (testable seam).
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

const DAY_MS: u64 = 24 * 60 * 60 * 1000;
#[allow(dead_code)]
const WEEK_MS: u64 = 7 * DAY_MS;

impl WorkflowSchedule {
    /// Compute the next occurrence strictly after the given timestamp (ms).
    /// Returns `None` if the schedule has expired or has no future occurrence.
    pub fn next_occurrence_after(&self, after_ms: u64) -> Option<u64> {
        let mut candidate = self.start_at;
        match &self.recurrence {
            RecurrencePattern::Once => {
                if candidate > after_ms {
                    Some(candidate)
                } else {
                    None
                }
            }
            RecurrencePattern::Daily { interval } => {
                let interval_ms = (*interval as u64).max(1) * DAY_MS;
                if candidate > after_ms {
                    return self.respect_end(Some(candidate));
                }
                let elapsed = after_ms - candidate;
                let n = elapsed / interval_ms + 1;
                candidate += n * interval_ms;
                self.respect_end(Some(candidate))
            }
            RecurrencePattern::Weekly { interval, weekdays } => {
                if weekdays.is_empty() {
                    return None;
                }
                let interval_weeks = (*interval as u64).max(1);
                // Walk forward day by day from max(start, after) until we hit a
                // matching weekday inside an active week.
                let start_day = candidate / DAY_MS;
                let begin_day = (after_ms / DAY_MS).max(start_day);
                // Cap search at 366 days × interval to avoid infinite loops.
                let cap = 366 * interval_weeks;
                for day in begin_day..begin_day + cap {
                    let days_since_start = day.saturating_sub(start_day);
                    let week_index = days_since_start / 7;
                    if week_index.is_multiple_of(interval_weeks) {
                        // Day-of-week (Unix epoch day 0 = Thursday, so add 4 for Sunday-based).
                        let dow = ((day as u32) + 4) % 7;
                        if let Some(w) = Weekday::from_num_from_sunday(dow) {
                            if weekdays.contains(&w) {
                                let ts = day * DAY_MS + (self.start_at % DAY_MS);
                                if ts > after_ms {
                                    return self.respect_end(Some(ts));
                                }
                            }
                        }
                    }
                }
                None
            }
            RecurrencePattern::Monthly {
                interval,
                day_of_month,
            } => {
                // Approximate month as 30.44 days. Sufficient for personal-assistant
                // recurrence at the milestone scope; for exact calendar arithmetic,
                // upgrade later to `chrono`.
                let interval = (*interval as u64).max(1);
                let approx_month_ms = ((30.44 * DAY_MS as f64) as u64) * interval;
                if candidate > after_ms {
                    return self.respect_end(Some(self.snap_to_dom(candidate, *day_of_month)));
                }
                let elapsed = after_ms - candidate;
                let n = elapsed / approx_month_ms + 1;
                candidate += n * approx_month_ms;
                Some(self.snap_to_dom(candidate, *day_of_month))
                    .and_then(|ts| self.respect_end(Some(ts)))
            }
        }
    }

    fn snap_to_dom(&self, ts: u64, _dom: u32) -> u64 {
        // For now keep the day-of-month from start_at. Full month arithmetic
        // requires `chrono`; the user-facing UI displays the actual time.
        ts
    }

    fn respect_end(&self, ts: Option<u64>) -> Option<u64> {
        match (ts, self.end_at) {
            (Some(t), Some(end)) if t > end => None,
            (t, _) => t,
        }
    }

    /// All occurrences in `[from_ms, to_ms)` for calendar rendering.
    /// Caps at 100 occurrences to bound work.
    pub fn occurrences_in_range(&self, from_ms: u64, to_ms: u64) -> Vec<u64> {
        let mut out = Vec::new();
        // Seed cursor so an occurrence that lands exactly on `from_ms` is
        // included (next_occurrence_after is strictly-after, so we ask
        // about `from_ms - 1` whenever from_ms > 0).
        let mut cursor = from_ms.saturating_sub(1);
        // Special case: `from_ms == 0` and `start_at == 0` would otherwise be
        // skipped because `0.saturating_sub(1) == 0` and the function is
        // strictly-after. Force-include the start when it falls in range.
        if from_ms == 0 && self.start_at == 0 && self.start_at < to_ms {
            out.push(0);
            cursor = 0;
        }
        while let Some(next) = self.next_occurrence_after(cursor) {
            if next >= to_ms || out.len() >= 100 {
                break;
            }
            if next < from_ms {
                cursor = next;
                continue;
            }
            out.push(next);
            cursor = next;
        }
        out
    }
}

/// A single calendar event projected from a workflow + schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub workflow_id: String,
    pub title: String,
    pub kind: WorkflowKind,
    /// Start time (ms).
    pub start_at: u64,
    /// End time (ms), derived from `start_at + duration_minutes * 60_000`.
    pub end_at: u64,
    /// Whether this is a recurring event (vs. one-time).
    pub recurring: bool,
    /// Status of the parent workflow.
    pub status: WorkflowPlanStatus,
}

/// Project all scheduled workflows in `plans` into calendar events for
/// `[from_ms, to_ms)`. Plans without a schedule are excluded.
pub fn project_calendar_events(
    plans: &[WorkflowPlan],
    from_ms: u64,
    to_ms: u64,
) -> Vec<CalendarEvent> {
    let mut events = Vec::new();
    for plan in plans {
        let Some(schedule) = &plan.schedule else {
            continue;
        };
        let recurring = !matches!(schedule.recurrence, RecurrencePattern::Once);
        let duration_ms = (schedule.duration_minutes as u64) * 60_000;
        for occ in schedule.occurrences_in_range(from_ms, to_ms) {
            events.push(CalendarEvent {
                workflow_id: plan.id.clone(),
                title: plan.title.clone(),
                kind: plan.kind,
                start_at: occ,
                end_at: occ + duration_ms,
                recurring,
                status: plan.status,
            });
        }
    }
    events.sort_by_key(|e| e.start_at);
    events
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_display() {
        assert_eq!(AgentRole::Planner.display_name(), "Planner");
        assert_eq!(AgentRole::Coder.display_name(), "Coder");
        assert_eq!(AgentRole::Reviewer.display_name(), "Reviewer");
        assert_eq!(AgentRole::Tester.display_name(), "Tester");
        assert_eq!(AgentRole::Researcher.display_name(), "Researcher");
        assert_eq!(AgentRole::Orchestrator.display_name(), "Orchestrator");
    }

    #[test]
    fn test_agent_role_recommendations() {
        for role in AgentRole::all() {
            let recs = role.recommended_llms();
            assert!(!recs.is_empty(), "{role} should have recommendations");
            // Should have at least one balanced tier
            assert!(
                recs.iter().any(|r| r.tier == LlmTier::Balanced),
                "{role} should have a balanced recommendation"
            );
        }
    }

    #[test]
    fn test_create_blank_plan() {
        let plan = create_blank_plan("Refactor the auth module", WorkflowKind::Coding);
        assert_eq!(plan.title, "Refactor the auth module");
        assert_eq!(plan.kind, WorkflowKind::Coding);
        assert_eq!(plan.status, WorkflowPlanStatus::PendingReview);
        assert!(plan.steps.is_empty());
        assert!(!plan.id.is_empty());
    }

    #[test]
    fn test_parse_planner_response() {
        let yaml = r#"
title: "Add user authentication"
kind: coding
tags: [auth, security]
steps:
  - id: research
    agent: researcher
    description: "Review existing auth patterns in codebase"
    depends_on: []
    output_format: prose
  - id: implement
    agent: coder
    description: "Implement OAuth2 login flow"
    depends_on: [research]
    output_format: code
  - id: review
    agent: reviewer
    description: "Review implementation for security issues"
    depends_on: [implement]
    output_format: verdict
    requires_approval: true
  - id: test
    agent: tester
    description: "Write and run integration tests"
    depends_on: [review]
    output_format: test_results
"#;
        let mut plan = create_blank_plan("Add auth", WorkflowKind::Coding);
        parse_planner_response(yaml, &mut plan).unwrap();

        assert_eq!(plan.title, "Add user authentication");
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(plan.steps[0].agent, AgentRole::Researcher);
        assert_eq!(plan.steps[1].agent, AgentRole::Coder);
        assert_eq!(plan.steps[1].depends_on, vec!["research"]);
        assert_eq!(plan.steps[2].agent, AgentRole::Reviewer);
        assert!(plan.steps[2].requires_approval);
        assert_eq!(plan.steps[3].output_format, StepOutputFormat::TestResults);
        assert_eq!(plan.tags, vec!["auth", "security"]);
    }

    #[test]
    fn test_validate_plan_ok() {
        let plan = WorkflowPlan {
            id: "test".to_string(),
            title: "Test".to_string(),
            kind: WorkflowKind::Coding,
            status: WorkflowPlanStatus::PendingReview,
            user_request: "test".to_string(),
            steps: vec![
                WorkflowStep {
                    id: "a".to_string(),
                    agent: AgentRole::Researcher,
                    llm_model: "gemma-3:4b".to_string(),
                    llm_provider: "ollama".to_string(),
                    description: "Research".to_string(),
                    depends_on: vec![],
                    output_format: StepOutputFormat::Prose,
                    status: StepStatus::Pending,
                    output: None,
                    error: None,
                    duration_ms: 0,
                    requires_approval: false,
                },
                WorkflowStep {
                    id: "b".to_string(),
                    agent: AgentRole::Coder,
                    llm_model: "qwen2.5-coder:7b".to_string(),
                    llm_provider: "ollama".to_string(),
                    description: "Code".to_string(),
                    depends_on: vec!["a".to_string()],
                    output_format: StepOutputFormat::Code,
                    status: StepStatus::Pending,
                    output: None,
                    error: None,
                    duration_ms: 0,
                    requires_approval: false,
                },
            ],
            agent_llm_overrides: HashMap::new(),
            created_at: 0,
            updated_at: 0,
            tags: vec![],
            schedule: None,
        };
        assert!(validate_plan(&plan).is_ok());
    }

    #[test]
    fn test_validate_plan_cycle() {
        let plan = WorkflowPlan {
            id: "test".to_string(),
            title: "Test".to_string(),
            kind: WorkflowKind::Coding,
            status: WorkflowPlanStatus::PendingReview,
            user_request: "test".to_string(),
            steps: vec![
                WorkflowStep {
                    id: "a".to_string(),
                    agent: AgentRole::Coder,
                    llm_model: "m".to_string(),
                    llm_provider: "ollama".to_string(),
                    description: "A".to_string(),
                    depends_on: vec!["b".to_string()],
                    output_format: StepOutputFormat::Code,
                    status: StepStatus::Pending,
                    output: None,
                    error: None,
                    duration_ms: 0,
                    requires_approval: false,
                },
                WorkflowStep {
                    id: "b".to_string(),
                    agent: AgentRole::Reviewer,
                    llm_model: "m".to_string(),
                    llm_provider: "ollama".to_string(),
                    description: "B".to_string(),
                    depends_on: vec!["a".to_string()],
                    output_format: StepOutputFormat::Verdict,
                    status: StepStatus::Pending,
                    output: None,
                    error: None,
                    duration_ms: 0,
                    requires_approval: false,
                },
            ],
            agent_llm_overrides: HashMap::new(),
            created_at: 0,
            updated_at: 0,
            tags: vec![],
            schedule: None,
        };
        let err = validate_plan(&plan).unwrap_err();
        assert!(err.iter().any(|e| e.contains("cycle")));
    }

    #[test]
    fn test_validate_plan_unknown_dep() {
        let plan = WorkflowPlan {
            id: "test".to_string(),
            title: "Test".to_string(),
            kind: WorkflowKind::Coding,
            status: WorkflowPlanStatus::PendingReview,
            user_request: "test".to_string(),
            steps: vec![WorkflowStep {
                id: "a".to_string(),
                agent: AgentRole::Coder,
                llm_model: "m".to_string(),
                llm_provider: "ollama".to_string(),
                description: "A".to_string(),
                depends_on: vec!["nonexistent".to_string()],
                output_format: StepOutputFormat::Code,
                status: StepStatus::Pending,
                output: None,
                error: None,
                duration_ms: 0,
                requires_approval: false,
            }],
            agent_llm_overrides: HashMap::new(),
            created_at: 0,
            updated_at: 0,
            tags: vec![],
            schedule: None,
        };
        let err = validate_plan(&plan).unwrap_err();
        assert!(err.iter().any(|e| e.contains("nonexistent")));
    }

    #[test]
    fn test_plan_summary() {
        let mut plan = create_blank_plan("Test", WorkflowKind::Daily);
        plan.steps.push(WorkflowStep {
            id: "s1".to_string(),
            agent: AgentRole::Researcher,
            llm_model: "m".to_string(),
            llm_provider: "ollama".to_string(),
            description: "d".to_string(),
            depends_on: vec![],
            output_format: StepOutputFormat::Prose,
            status: StepStatus::Completed,
            output: Some("done".to_string()),
            error: None,
            duration_ms: 100,
            requires_approval: false,
        });
        plan.steps.push(WorkflowStep {
            id: "s2".to_string(),
            agent: AgentRole::Coder,
            llm_model: "m".to_string(),
            llm_provider: "ollama".to_string(),
            description: "d".to_string(),
            depends_on: vec!["s1".to_string()],
            output_format: StepOutputFormat::Code,
            status: StepStatus::Pending,
            output: None,
            error: None,
            duration_ms: 0,
            requires_approval: false,
        });

        let summary = WorkflowPlanSummary::from(&plan);
        assert_eq!(summary.step_count, 2);
        assert_eq!(summary.completed_steps, 1);
        assert_eq!(summary.kind, WorkflowKind::Daily);
    }

    #[test]
    fn test_truncate_title() {
        assert_eq!(truncate_title("short", 80), "short");
        let long = "a".repeat(100);
        let result = truncate_title(&long, 20);
        assert!(result.len() <= 20);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_plan_serialization_roundtrip() {
        let mut plan = create_blank_plan("Test roundtrip", WorkflowKind::OneTime);
        plan.steps.push(WorkflowStep {
            id: "s1".to_string(),
            agent: AgentRole::Researcher,
            llm_model: "gemma-3:4b".to_string(),
            llm_provider: "ollama".to_string(),
            description: "Research the topic".to_string(),
            depends_on: vec![],
            output_format: StepOutputFormat::Prose,
            status: StepStatus::Pending,
            output: None,
            error: None,
            duration_ms: 0,
            requires_approval: false,
        });

        let yaml = serde_yaml::to_string(&plan).unwrap();
        let restored: WorkflowPlan = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(restored.id, plan.id);
        assert_eq!(restored.steps.len(), 1);
        assert_eq!(restored.steps[0].agent, AgentRole::Researcher);
    }

    #[test]
    fn test_disk_persistence() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path();

        let mut plan = create_blank_plan("Persist test", WorkflowKind::Coding);
        plan.steps.push(WorkflowStep {
            id: "s1".to_string(),
            agent: AgentRole::Coder,
            llm_model: "m".to_string(),
            llm_provider: "ollama".to_string(),
            description: "Write code".to_string(),
            depends_on: vec![],
            output_format: StepOutputFormat::Code,
            status: StepStatus::Pending,
            output: None,
            error: None,
            duration_ms: 0,
            requires_approval: false,
        });

        save_plan(data_dir, &plan).unwrap();
        let loaded = load_plan(data_dir, &plan.id).unwrap();
        assert_eq!(loaded.title, "Persist test");
        assert_eq!(loaded.steps.len(), 1);

        let summaries = list_plans(data_dir).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, plan.id);

        delete_plan(data_dir, &plan.id).unwrap();
        let summaries = list_plans(data_dir).unwrap();
        assert_eq!(summaries.len(), 0);
    }

    // ---- Scheduling tests ----

    fn schedule_plan(schedule: WorkflowSchedule) -> WorkflowPlan {
        let mut plan = create_blank_plan("Scheduled", WorkflowKind::Daily);
        plan.schedule = Some(schedule);
        plan
    }

    #[test]
    fn test_schedule_once_future() {
        let schedule = WorkflowSchedule {
            start_at: 1_000_000,
            end_at: None,
            duration_minutes: 30,
            recurrence: RecurrencePattern::Once,
            timezone: "UTC".to_string(),
            last_fired_at: None,
        };
        assert_eq!(schedule.next_occurrence_after(0), Some(1_000_000));
        assert_eq!(schedule.next_occurrence_after(1_000_000), None);
        assert_eq!(schedule.next_occurrence_after(2_000_000), None);
    }

    #[test]
    fn test_schedule_daily_interval_2() {
        // start at day 0, every 2 days
        let schedule = WorkflowSchedule {
            start_at: 0,
            end_at: None,
            duration_minutes: 30,
            recurrence: RecurrencePattern::Daily { interval: 2 },
            timezone: "UTC".to_string(),
            last_fired_at: None,
        };
        // After day 0, next is day 2
        assert_eq!(schedule.next_occurrence_after(0), Some(2 * DAY_MS));
        // After day 1, next is day 2
        assert_eq!(schedule.next_occurrence_after(DAY_MS), Some(2 * DAY_MS));
        // After day 2, next is day 4
        assert_eq!(schedule.next_occurrence_after(2 * DAY_MS), Some(4 * DAY_MS));
    }

    #[test]
    fn test_schedule_weekly_mwf() {
        // Unix epoch day 0 = Thursday. Day 1 = Friday. Day 4 = Monday.
        // start_at: epoch ms 0 (Thursday)
        let schedule = WorkflowSchedule {
            start_at: 0,
            end_at: None,
            duration_minutes: 30,
            recurrence: RecurrencePattern::Weekly {
                interval: 1,
                weekdays: vec![Weekday::Monday, Weekday::Wednesday, Weekday::Friday],
            },
            timezone: "UTC".to_string(),
            last_fired_at: None,
        };
        // First occurrence after t=0 should be day 1 (Friday).
        let next = schedule.next_occurrence_after(0).unwrap();
        assert_eq!(next, DAY_MS);
        // Then day 4 (Monday).
        let next2 = schedule.next_occurrence_after(next).unwrap();
        assert_eq!(next2, 4 * DAY_MS);
        // Then day 6 (Wednesday).
        let next3 = schedule.next_occurrence_after(next2).unwrap();
        assert_eq!(next3, 6 * DAY_MS);
    }

    #[test]
    fn test_schedule_weekly_empty_weekdays() {
        let schedule = WorkflowSchedule {
            start_at: 0,
            end_at: None,
            duration_minutes: 30,
            recurrence: RecurrencePattern::Weekly {
                interval: 1,
                weekdays: vec![],
            },
            timezone: "UTC".to_string(),
            last_fired_at: None,
        };
        assert_eq!(schedule.next_occurrence_after(0), None);
    }

    #[test]
    fn test_schedule_respects_end_at() {
        let schedule = WorkflowSchedule {
            start_at: 0,
            end_at: Some(3 * DAY_MS),
            duration_minutes: 30,
            recurrence: RecurrencePattern::Daily { interval: 1 },
            timezone: "UTC".to_string(),
            last_fired_at: None,
        };
        assert_eq!(schedule.next_occurrence_after(0), Some(DAY_MS));
        assert_eq!(schedule.next_occurrence_after(2 * DAY_MS), Some(3 * DAY_MS));
        assert_eq!(schedule.next_occurrence_after(3 * DAY_MS), None);
    }

    #[test]
    fn test_occurrences_in_range() {
        let schedule = WorkflowSchedule {
            start_at: 0,
            end_at: None,
            duration_minutes: 30,
            recurrence: RecurrencePattern::Daily { interval: 1 },
            timezone: "UTC".to_string(),
            last_fired_at: None,
        };
        let occs = schedule.occurrences_in_range(0, 5 * DAY_MS);
        // Should be 5 occurrences: day 0, 1, 2, 3, 4
        assert_eq!(occs.len(), 5);
        assert_eq!(occs[0], 0);
        assert_eq!(occs[4], 4 * DAY_MS);
    }

    #[test]
    fn test_project_calendar_events() {
        let plan_a = schedule_plan(WorkflowSchedule {
            start_at: DAY_MS, // Friday at midnight UTC
            end_at: None,
            duration_minutes: 60,
            recurrence: RecurrencePattern::Daily { interval: 1 },
            timezone: "UTC".to_string(),
            last_fired_at: None,
        });
        let mut plan_b = schedule_plan(WorkflowSchedule {
            start_at: 2 * DAY_MS,
            end_at: None,
            duration_minutes: 30,
            recurrence: RecurrencePattern::Once,
            timezone: "UTC".to_string(),
            last_fired_at: None,
        });
        plan_b.title = "One-shot".to_string();

        let events = project_calendar_events(&[plan_a, plan_b], 0, 4 * DAY_MS);
        // plan_a fires on days 1, 2, 3 (3 events). plan_b fires on day 2 (1 event).
        assert_eq!(events.len(), 4);
        // Sorted by start_at
        assert_eq!(events[0].start_at, DAY_MS);
        // 60min duration -> end = start + 60*60_000
        assert_eq!(events[0].end_at, DAY_MS + 60 * 60_000);
        assert!(events.iter().any(|e| e.title == "One-shot" && !e.recurring));
    }

    #[test]
    fn test_weekday_roundtrip() {
        for n in 0..7 {
            let w = Weekday::from_num_from_sunday(n).unwrap();
            assert_eq!(w.num_from_sunday(), n);
        }
    }

    #[test]
    fn test_summary_includes_schedule_metadata() {
        let plan = schedule_plan(WorkflowSchedule {
            start_at: now_ms() + 60_000_000, // far future
            end_at: None,
            duration_minutes: 30,
            recurrence: RecurrencePattern::Weekly {
                interval: 1,
                weekdays: vec![Weekday::Monday],
            },
            timezone: "UTC".to_string(),
            last_fired_at: None,
        });
        let summary = WorkflowPlanSummary::from(&plan);
        assert!(summary.recurring);
        assert!(summary.next_occurrence.is_some());
    }
}
