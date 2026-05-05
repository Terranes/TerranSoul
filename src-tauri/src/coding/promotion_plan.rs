//! Shared building blocks for "promote a user-tuned artefact to a
//! source-code default" workflow plans.
//!
//! Both [`crate::persona::charisma`] and
//! [`crate::teachable_capabilities::registry`] need
//! exactly the same 4-step DAG (Researcher → Coder *requires_approval* →
//! Tester → Reviewer *requires_approval*). Keeping the construction in
//! one place avoids drift and means future tweaks (different default
//! models, an extra Linter step, etc.) only need a single edit.
//!
//! See `docs/charisma-teaching-tutorial.md` and
//! the teachable-capabilities guide for the user-facing flow.

use crate::coding::multi_agent::{
    AgentRole, StepOutputFormat, StepStatus, WorkflowKind, WorkflowPlan, WorkflowPlanStatus,
    WorkflowStep,
};

/// Maturity tier shared by every "user-taught artefact that can be
/// promoted to source" surface (Charisma stats, teachable capabilities,
/// future ones).
///
/// * `Untested` — never used since taught / never enabled.
/// * `Learning` — used at least once but not yet meeting the bar.
/// * `Proven`   — ≥ 10 uses **and** average rating ≥ 4.0.
/// * `Canon`    — already promoted to a source-code default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Maturity {
    Untested,
    Learning,
    Proven,
    Canon,
}

/// Threshold constants — single source of truth so the panel and the
/// backend can never disagree.
pub const PROVEN_MIN_USES: u32 = 10;
pub const PROVEN_MIN_AVG_RATING: f32 = 4.0;

/// Inputs for [`build_promotion_plan`]. Charisma and teachable capabilities wrap
/// their domain-specific data into this shape before delegating.
pub struct PromotionPlanSpec<'a> {
    /// Stable plan id (caller must allocate via `multi_agent::new_plan_id`).
    pub plan_id: String,
    /// Now (ms epoch).
    pub now_ms: u64,
    /// Plan title shown in the Workflows panel.
    pub title: String,
    /// Markdown body shown to the user as the plan's request.
    pub user_request: String,
    /// Researcher step description ("locate the right file to edit").
    pub research_description: String,
    /// Coder step description (must produce `<file>` blocks).
    pub code_description: String,
    /// Tester step description (which test slice to run).
    pub test_description: String,
    /// Reviewer step description (security + style audit).
    pub review_description: String,
    /// Tags for the workflow (e.g. `["charisma", "promotion"]`).
    pub tags: Vec<String>,
    /// Hints for the Coder — list of source files the Researcher should
    /// expect to need to inspect / edit.
    pub target_files: &'a [String],
}

/// Build the 4-step coding workflow plan that promotes a user-tuned
/// artefact (Charisma stat or teachable capability config) into a bundled
/// source-code default. Two human approval gates: the Coder step (before
/// the `<file>` block is written via `apply_file`) and the Reviewer step
/// (before the change is considered accepted).
pub fn build_promotion_plan(spec: PromotionPlanSpec<'_>) -> WorkflowPlan {
    let target_hint = if spec.target_files.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nLikely target file(s):\n{}",
            spec.target_files
                .iter()
                .map(|p| format!("- {p}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    };

    let researcher = WorkflowStep {
        id: "research".to_string(),
        agent: AgentRole::Researcher,
        llm_provider: "ollama".to_string(),
        llm_model: "llama3.1:8b".to_string(),
        description: format!("{}{}", spec.research_description, target_hint),
        depends_on: vec![],
        output_format: StepOutputFormat::Prose,
        status: StepStatus::Pending,
        output: None,
        error: None,
        duration_ms: 0,
        requires_approval: false,
    };

    let coder = WorkflowStep {
        id: "code".to_string(),
        agent: AgentRole::Coder,
        llm_provider: "anthropic".to_string(),
        llm_model: "claude-sonnet-4-5".to_string(),
        description: spec.code_description,
        depends_on: vec!["research".to_string()],
        output_format: StepOutputFormat::Code,
        status: StepStatus::Pending,
        output: None,
        error: None,
        duration_ms: 0,
        requires_approval: true,
    };

    let tester = WorkflowStep {
        id: "test".to_string(),
        agent: AgentRole::Tester,
        llm_provider: "ollama".to_string(),
        llm_model: "qwen2.5-coder:7b".to_string(),
        description: spec.test_description,
        depends_on: vec!["code".to_string()],
        output_format: StepOutputFormat::TestResults,
        status: StepStatus::Pending,
        output: None,
        error: None,
        duration_ms: 0,
        requires_approval: false,
    };

    let reviewer = WorkflowStep {
        id: "review".to_string(),
        agent: AgentRole::Reviewer,
        llm_provider: "anthropic".to_string(),
        llm_model: "claude-opus-4".to_string(),
        description: spec.review_description,
        depends_on: vec!["test".to_string()],
        output_format: StepOutputFormat::Verdict,
        status: StepStatus::Pending,
        output: None,
        error: None,
        duration_ms: 0,
        requires_approval: true,
    };

    WorkflowPlan {
        id: spec.plan_id,
        title: spec.title,
        kind: WorkflowKind::Coding,
        status: WorkflowPlanStatus::PendingReview,
        user_request: spec.user_request,
        steps: vec![researcher, coder, tester, reviewer],
        agent_llm_overrides: Default::default(),
        created_at: spec.now_ms,
        updated_at: spec.now_ms,
        tags: spec.tags,
        schedule: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_has_two_approval_gates() {
        let targets = vec!["src/foo.rs".to_string()];
        let plan = build_promotion_plan(PromotionPlanSpec {
            plan_id: "p_x".into(),
            now_ms: 1,
            title: "t".into(),
            user_request: "r".into(),
            research_description: "research".into(),
            code_description: "code".into(),
            test_description: "test".into(),
            review_description: "review".into(),
            tags: vec!["promotion".into()],
            target_files: &targets,
        });
        assert_eq!(plan.steps.len(), 4);
        assert!(!plan.steps[0].requires_approval);
        assert!(
            plan.steps[1].requires_approval,
            "Coder must require approval"
        );
        assert!(!plan.steps[2].requires_approval);
        assert!(
            plan.steps[3].requires_approval,
            "Reviewer must require approval"
        );
        assert_eq!(plan.kind, WorkflowKind::Coding);
        assert_eq!(plan.status, WorkflowPlanStatus::PendingReview);
        assert!(plan.steps[0].description.contains("src/foo.rs"));
    }

    #[test]
    fn maturity_thresholds_are_stable() {
        assert_eq!(PROVEN_MIN_USES, 10);
        assert!((PROVEN_MIN_AVG_RATING - 4.0).abs() < f32::EPSILON);
    }
}
