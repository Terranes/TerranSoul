//! Tauri commands for the multi-agent workflow system.
//!
//! Surfaces:
//! - List, load, save, delete workflow plans (YAML-backed).
//! - Create plan from user request (uses Planner agent via configured LLM).
//! - Run plans, update step status, override per-agent LLMs.
//! - Calendar projection (occurrences in date range, MS Teams-style).
//!
//! Pure inner helpers (`*_inner`) take `data_dir: &Path` and are unit-tested.
//! Tauri-facing wrappers extract `data_dir` from `AppState`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::coding::{
    self, multi_agent, AgentLlmConfig, AgentRole, CalendarEvent, RecurrencePattern,
    StepOutputFormat, StepStatus, Weekday, WorkflowKind, WorkflowPlan, WorkflowPlanStatus,
    WorkflowPlanSummary, WorkflowSchedule, WorkflowStep,
};
use crate::AppState;

// ---------------------------------------------------------------------------
// Pure helpers (unit-tested)
// ---------------------------------------------------------------------------

fn list_inner(data_dir: &Path) -> Result<Vec<WorkflowPlanSummary>, String> {
    multi_agent::list_plans(data_dir)
}

fn load_inner(data_dir: &Path, plan_id: &str) -> Result<WorkflowPlan, String> {
    multi_agent::load_plan(data_dir, plan_id)
}

fn save_inner(data_dir: &Path, mut plan: WorkflowPlan) -> Result<WorkflowPlan, String> {
    plan.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    if let Err(errors) = multi_agent::validate_plan(&plan) {
        return Err(format!("invalid plan: {}", errors.join("; ")));
    }
    multi_agent::save_plan(data_dir, &plan)?;
    Ok(plan)
}

fn delete_inner(data_dir: &Path, plan_id: &str) -> Result<(), String> {
    multi_agent::delete_plan(data_dir, plan_id)
}

fn create_blank_inner(user_request: &str, kind: WorkflowKind) -> WorkflowPlan {
    multi_agent::create_blank_plan(user_request, kind)
}

fn calendar_events_inner(
    data_dir: &Path,
    from_ms: u64,
    to_ms: u64,
) -> Result<Vec<CalendarEvent>, String> {
    let summaries = multi_agent::list_plans(data_dir)?;
    let mut plans = Vec::new();
    for summary in summaries {
        if let Ok(plan) = multi_agent::load_plan(data_dir, &summary.id) {
            plans.push(plan);
        }
    }
    Ok(multi_agent::project_calendar_events(&plans, from_ms, to_ms))
}

fn update_step_inner(
    data_dir: &Path,
    plan_id: &str,
    step_id: &str,
    status: StepStatus,
    output: Option<String>,
    error: Option<String>,
) -> Result<WorkflowPlan, String> {
    let mut plan = multi_agent::load_plan(data_dir, plan_id)?;
    let step = plan
        .steps
        .iter_mut()
        .find(|s| s.id == step_id)
        .ok_or_else(|| format!("step '{step_id}' not found"))?;
    step.status = status;
    if output.is_some() {
        step.output = output;
    }
    if error.is_some() {
        step.error = error;
    }
    save_inner(data_dir, plan)
}

fn override_llm_inner(
    data_dir: &Path,
    plan_id: &str,
    agent: AgentRole,
    config: AgentLlmConfig,
) -> Result<WorkflowPlan, String> {
    let mut plan = multi_agent::load_plan(data_dir, plan_id)?;
    plan.agent_llm_overrides.insert(agent, config.clone());
    // Apply to existing steps for this agent
    for step in plan.steps.iter_mut() {
        if step.agent == agent {
            step.llm_model = config.model.clone();
            step.llm_provider = config.provider.clone();
        }
    }
    save_inner(data_dir, plan)
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn workflow_plan_list(
    state: State<'_, AppState>,
) -> Result<Vec<WorkflowPlanSummary>, String> {
    list_inner(&state.data_dir)
}

#[tauri::command]
pub async fn workflow_plan_load(
    state: State<'_, AppState>,
    plan_id: String,
) -> Result<WorkflowPlan, String> {
    load_inner(&state.data_dir, &plan_id)
}

#[tauri::command]
pub async fn workflow_plan_save(
    state: State<'_, AppState>,
    plan: WorkflowPlan,
) -> Result<WorkflowPlan, String> {
    save_inner(&state.data_dir, plan)
}

#[tauri::command]
pub async fn workflow_plan_delete(
    state: State<'_, AppState>,
    plan_id: String,
) -> Result<(), String> {
    delete_inner(&state.data_dir, &plan_id)
}

#[derive(Debug, Deserialize)]
pub struct CreateBlankPlanArgs {
    pub user_request: String,
    pub kind: WorkflowKind,
}

#[tauri::command]
pub async fn workflow_plan_create_blank(args: CreateBlankPlanArgs) -> Result<WorkflowPlan, String> {
    Ok(create_blank_inner(&args.user_request, args.kind))
}

#[tauri::command]
pub async fn workflow_plan_validate(plan: WorkflowPlan) -> Result<Vec<String>, String> {
    match multi_agent::validate_plan(&plan) {
        Ok(()) => Ok(Vec::new()),
        Err(errs) => Ok(errs),
    }
}

#[tauri::command]
pub async fn workflow_plan_update_step(
    state: State<'_, AppState>,
    plan_id: String,
    step_id: String,
    status: StepStatus,
    output: Option<String>,
    error: Option<String>,
) -> Result<WorkflowPlan, String> {
    update_step_inner(&state.data_dir, &plan_id, &step_id, status, output, error)
}

#[tauri::command]
pub async fn workflow_plan_override_llm(
    state: State<'_, AppState>,
    plan_id: String,
    agent: AgentRole,
    config: AgentLlmConfig,
) -> Result<WorkflowPlan, String> {
    override_llm_inner(&state.data_dir, &plan_id, agent, config)
}

#[tauri::command]
pub async fn workflow_calendar_events(
    state: State<'_, AppState>,
    from_ms: u64,
    to_ms: u64,
) -> Result<Vec<CalendarEvent>, String> {
    calendar_events_inner(&state.data_dir, from_ms, to_ms)
}

/// Per-role LLM recommendation surfaced to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct RoleRecommendations {
    pub role: AgentRole,
    pub display_name: String,
    pub recommendations: Vec<RecommendationDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecommendationDto {
    pub model: String,
    pub provider: String,
    pub tier: String,
    pub reason: String,
}

#[tauri::command]
pub async fn workflow_agent_recommendations() -> Result<Vec<RoleRecommendations>, String> {
    Ok(AgentRole::all()
        .iter()
        .map(|&role| RoleRecommendations {
            role,
            display_name: role.display_name().to_string(),
            recommendations: role
                .recommended_llms()
                .iter()
                .map(|r| RecommendationDto {
                    model: r.model.to_string(),
                    provider: r.provider.to_string(),
                    tier: format!("{:?}", r.tier).to_lowercase(),
                    reason: r.reason.to_string(),
                })
                .collect(),
        })
        .collect())
}

// Allow the unused import warning when compiled without all the types in scope.
#[allow(dead_code)]
fn _silence(
    _: WorkflowSchedule,
    _: RecurrencePattern,
    _: Weekday,
    _: WorkflowPlanStatus,
    _: StepOutputFormat,
    _: WorkflowStep,
    _: coding::CodingTask,
) {
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn sample_plan() -> WorkflowPlan {
        let mut plan = create_blank_inner("Implement feature X", WorkflowKind::Coding);
        plan.steps.push(WorkflowStep {
            id: "research".to_string(),
            agent: AgentRole::Researcher,
            llm_model: "gemma-3:4b".to_string(),
            llm_provider: "ollama".to_string(),
            description: "Research existing patterns".to_string(),
            depends_on: vec![],
            output_format: StepOutputFormat::Prose,
            status: StepStatus::Pending,
            output: None,
            error: None,
            duration_ms: 0,
            requires_approval: false,
        });
        plan.steps.push(WorkflowStep {
            id: "code".to_string(),
            agent: AgentRole::Coder,
            llm_model: "qwen2.5-coder:7b".to_string(),
            llm_provider: "ollama".to_string(),
            description: "Implement the feature".to_string(),
            depends_on: vec!["research".to_string()],
            output_format: StepOutputFormat::Code,
            status: StepStatus::Pending,
            output: None,
            error: None,
            duration_ms: 0,
            requires_approval: false,
        });
        plan
    }

    #[test]
    fn test_save_load_delete_cycle() {
        let tmp = temp_dir();
        let plan = sample_plan();
        let plan_id = plan.id.clone();
        let saved = save_inner(tmp.path(), plan).unwrap();
        assert_eq!(saved.id, plan_id);
        assert!(saved.updated_at > 0);

        let loaded = load_inner(tmp.path(), &plan_id).unwrap();
        assert_eq!(loaded.steps.len(), 2);

        let summaries = list_inner(tmp.path()).unwrap();
        assert_eq!(summaries.len(), 1);

        delete_inner(tmp.path(), &plan_id).unwrap();
        assert_eq!(list_inner(tmp.path()).unwrap().len(), 0);
    }

    #[test]
    fn test_save_invalid_plan_rejected() {
        let tmp = temp_dir();
        let mut plan = sample_plan();
        plan.steps[1].depends_on.push("missing-step".to_string());
        let err = save_inner(tmp.path(), plan).unwrap_err();
        assert!(err.contains("missing-step"));
    }

    #[test]
    fn test_create_blank() {
        let plan = create_blank_inner("Daily standup summary", WorkflowKind::Daily);
        assert_eq!(plan.kind, WorkflowKind::Daily);
        assert_eq!(plan.status, WorkflowPlanStatus::PendingReview);
        assert!(plan.steps.is_empty());
        assert!(plan.schedule.is_none());
    }

    #[test]
    fn test_update_step_status() {
        let tmp = temp_dir();
        let plan = sample_plan();
        let plan_id = plan.id.clone();
        save_inner(tmp.path(), plan).unwrap();

        let updated = update_step_inner(
            tmp.path(),
            &plan_id,
            "research",
            StepStatus::Completed,
            Some("Found 3 relevant patterns".to_string()),
            None,
        )
        .unwrap();
        let step = updated.steps.iter().find(|s| s.id == "research").unwrap();
        assert_eq!(step.status, StepStatus::Completed);
        assert_eq!(step.output.as_deref(), Some("Found 3 relevant patterns"));
    }

    #[test]
    fn test_update_step_unknown_id() {
        let tmp = temp_dir();
        let plan = sample_plan();
        let plan_id = plan.id.clone();
        save_inner(tmp.path(), plan).unwrap();

        let err = update_step_inner(
            tmp.path(),
            &plan_id,
            "nonexistent",
            StepStatus::Completed,
            None,
            None,
        )
        .unwrap_err();
        assert!(err.contains("nonexistent"));
    }

    #[test]
    fn test_override_llm_propagates_to_steps() {
        let tmp = temp_dir();
        let plan = sample_plan();
        let plan_id = plan.id.clone();
        save_inner(tmp.path(), plan).unwrap();

        let cfg = AgentLlmConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            provider: "anthropic".to_string(),
            api_key: None,
            base_url: None,
        };
        let updated =
            override_llm_inner(tmp.path(), &plan_id, AgentRole::Coder, cfg.clone()).unwrap();

        assert_eq!(
            updated
                .agent_llm_overrides
                .get(&AgentRole::Coder)
                .unwrap()
                .model,
            "claude-sonnet-4-20250514"
        );
        let coder_step = updated.steps.iter().find(|s| s.id == "code").unwrap();
        assert_eq!(coder_step.llm_model, "claude-sonnet-4-20250514");
        assert_eq!(coder_step.llm_provider, "anthropic");
        // Researcher step untouched
        let research_step = updated.steps.iter().find(|s| s.id == "research").unwrap();
        assert_eq!(research_step.llm_model, "gemma-3:4b");
    }

    #[test]
    fn test_calendar_events_excludes_unscheduled() {
        let tmp = temp_dir();
        let plan = sample_plan(); // no schedule
        save_inner(tmp.path(), plan).unwrap();

        let events = calendar_events_inner(tmp.path(), 0, u64::MAX / 2).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_calendar_events_includes_scheduled() {
        let tmp = temp_dir();
        let mut plan = sample_plan();
        plan.schedule = Some(WorkflowSchedule {
            start_at: 1_000_000_000_000, // 2001-09-09
            end_at: None,
            duration_minutes: 30,
            recurrence: RecurrencePattern::Daily { interval: 1 },
            timezone: "UTC".to_string(),
            last_fired_at: None,
        });
        save_inner(tmp.path(), plan).unwrap();

        // 7-day window starting at start_at
        let events = calendar_events_inner(
            tmp.path(),
            1_000_000_000_000,
            1_000_000_000_000 + 7 * 86_400_000,
        )
        .unwrap();
        assert_eq!(events.len(), 7);
        assert!(events[0].recurring);
    }

    #[test]
    fn test_summary_sorted_by_recency() {
        let tmp = temp_dir();
        let mut plan_a = sample_plan();
        plan_a.title = "Older".to_string();
        save_inner(tmp.path(), plan_a).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut plan_b = sample_plan();
        plan_b.title = "Newer".to_string();
        save_inner(tmp.path(), plan_b).unwrap();

        let summaries = list_inner(tmp.path()).unwrap();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].title, "Newer");
    }

    #[test]
    fn test_agent_recommendations_shape() {
        // Run the inner function directly (no Tauri context)
        let recs: Vec<_> = AgentRole::all()
            .iter()
            .map(|&role| {
                (
                    role,
                    role.recommended_llms()
                        .iter()
                        .map(|r| r.model)
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(recs.len(), 6);
        // Each role has at least one recommendation
        for (_, models) in &recs {
            assert!(!models.is_empty());
        }
    }

    #[test]
    fn test_blank_plan_with_overrides_serializes() {
        let mut plan = create_blank_inner("Test", WorkflowKind::OneTime);
        let mut overrides = HashMap::new();
        overrides.insert(
            AgentRole::Reviewer,
            AgentLlmConfig {
                model: "x".to_string(),
                provider: "y".to_string(),
                api_key: None,
                base_url: None,
            },
        );
        plan.agent_llm_overrides = overrides;
        let yaml = serde_yaml::to_string(&plan).unwrap();
        assert!(yaml.contains("reviewer"));
    }
}
