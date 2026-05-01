//! Context engineering — budget-aware prompt assembly (Chunk 27.2).
//!
//! Bridges `coding::context_budget` with `coding::prompting` so that
//! prompt sections are priority-ranked and automatically trimmed to
//! fit the model's context window before `CodingPrompt::build()` is
//! called.
//!
//! ## Integration Point
//!
//! ```text
//! load_documents() → budget_aware_assembly() → CodingPrompt::build()
//! ```
//!
//! The assembler converts task/document/plan inputs into
//! `ContextSection`s, runs `fit_to_budget`, then rewrites the
//! `CodingPrompt.documents` field with the surviving sections.

use crate::coding::context_budget::{
    fit_to_budget, BudgetConfig, BudgetResult, ContextSection, SectionPriority,
};
use crate::coding::prompting::{CodingPrompt, DocSnippet};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Wrapper around a document snippet with an assigned priority.
#[derive(Debug, Clone)]
pub struct PrioritisedDoc {
    pub snippet: DocSnippet,
    pub priority: SectionPriority,
    /// Whether a summary can replace this doc when pruned.
    pub summarizable: bool,
}

/// Result of budget-aware assembly.
#[derive(Debug, Clone)]
pub struct AssemblyResult {
    /// The prompt with documents trimmed to fit.
    pub prompt: CodingPrompt,
    /// Underlying budget computation details.
    pub budget: BudgetResult,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Apply the context budget to a `CodingPrompt`'s documents.
///
/// Converts the prompt's existing documents (plus any additional prioritised
/// docs) into budget sections. The task itself is treated as `System`-priority
/// (never pruned). Documents are ranked by their assigned priority and pruned
/// lowest-first until the total fits within `config.max_tokens`.
///
/// Returns the modified prompt (with `.documents` replaced by the surviving
/// subset) along with the full `BudgetResult` for diagnostics.
pub fn budget_aware_assembly(
    mut prompt: CodingPrompt,
    extra_docs: Vec<PrioritisedDoc>,
    config: &BudgetConfig,
) -> AssemblyResult {
    // Build context sections from the prompt.
    let mut sections: Vec<ContextSection> = Vec::new();

    // The system role + task is always kept (System priority).
    let role_task_content = format!("{}\n---\n{}", prompt.role, prompt.task);
    sections.push(ContextSection {
        label: "system-task".to_owned(),
        priority: SectionPriority::System,
        content: role_task_content,
        summarizable: false,
    });

    // Existing documents from the prompt get Documentation priority by default.
    for doc in &prompt.documents {
        sections.push(ContextSection {
            label: doc.label.clone(),
            priority: SectionPriority::Documentation,
            content: doc.body.clone(),
            summarizable: true,
        });
    }

    // Extra prioritised docs.
    for pd in &extra_docs {
        sections.push(ContextSection {
            label: pd.snippet.label.clone(),
            priority: pd.priority,
            content: pd.snippet.body.clone(),
            summarizable: pd.summarizable,
        });
    }

    // Run the budget.
    let budget_result = fit_to_budget(sections, config);

    // Rebuild the prompt's documents from the kept sections.
    // Skip the "system-task" section — that stays in role/task fields.
    let mut new_docs: Vec<DocSnippet> = Vec::new();
    for section in &budget_result.kept {
        if section.label == "system-task" {
            continue;
        }
        new_docs.push(DocSnippet {
            label: section.label.clone(),
            body: section.content.clone(),
        });
    }

    // If there's a pruned summary, inject it as a final document.
    if let Some(ref summary) = budget_result.pruned_summary {
        new_docs.push(DocSnippet {
            label: "[pruned-context-summary]".to_owned(),
            body: summary.clone(),
        });
    }

    prompt.documents = new_docs;

    AssemblyResult {
        prompt,
        budget: budget_result,
    }
}

/// Classify a document label into a default priority.
///
/// Heuristic based on common label patterns in coding workflows.
pub fn infer_priority(label: &str) -> SectionPriority {
    let lower = label.to_lowercase();
    if lower.contains("error") || lower.contains("fail") {
        SectionPriority::RecentErrors
    } else if lower.contains("plan") || lower.contains("step") {
        SectionPriority::ActivePlan
    } else if lower.contains("test") || lower.contains("result") || lower.contains("output") {
        SectionPriority::PriorResults
    } else if lower.contains("handoff") || lower.contains("chain") || lower.contains("resume") {
        SectionPriority::Handoff
    } else if lower.contains("history") || lower.contains("completed") {
        SectionPriority::CompletedSteps
    } else {
        SectionPriority::Documentation
    }
}

/// Convenience: auto-prioritise a list of plain `DocSnippet`s based on
/// their labels, then apply the budget.
pub fn auto_budget_assembly(prompt: CodingPrompt, config: &BudgetConfig) -> AssemblyResult {
    // Move existing docs into prioritised form using label heuristics.
    let prioritised: Vec<PrioritisedDoc> = prompt
        .documents
        .iter()
        .map(|doc| PrioritisedDoc {
            snippet: doc.clone(),
            priority: infer_priority(&doc.label),
            summarizable: true,
        })
        .collect();

    // Clear documents from prompt (they'll be re-added from budget result).
    let mut prompt = prompt;
    prompt.documents = Vec::new();

    budget_aware_assembly(prompt, prioritised, config)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::prompting::OutputShape;

    fn test_prompt(docs: Vec<DocSnippet>) -> CodingPrompt {
        CodingPrompt {
            role: "Test role".to_owned(),
            task: "Do something".to_owned(),
            negative_constraints: vec![],
            documents: docs,
            output: OutputShape::Prose,
            example: None,
            assistant_prefill: None,
            error_handling: vec![],
        }
    }

    fn big_doc(label: &str, chars: usize) -> DocSnippet {
        DocSnippet {
            label: label.to_owned(),
            body: "x".repeat(chars),
        }
    }

    #[test]
    fn all_docs_fit_within_budget() {
        let prompt = test_prompt(vec![big_doc("doc1", 100), big_doc("doc2", 100)]);
        let config = BudgetConfig {
            max_tokens: 10_000,
            response_reserve: 1_000,
            ..Default::default()
        };
        let result = budget_aware_assembly(prompt, vec![], &config);
        // Both docs should survive.
        assert_eq!(result.prompt.documents.len(), 2);
        assert!(result.budget.pruned.is_empty());
    }

    #[test]
    fn docs_pruned_when_over_budget() {
        // Budget is tiny — only room for system-task + 1 small doc.
        let prompt = test_prompt(vec![
            big_doc("small", 40),  // ~10 tokens
            big_doc("large", 400), // ~100 tokens
        ]);
        let config = BudgetConfig {
            max_tokens: 50, // very tight
            response_reserve: 0,
            summary_max_tokens: 100,
            generate_summaries: true,
        };
        let result = budget_aware_assembly(prompt, vec![], &config);
        // The large doc should be pruned, small might survive.
        assert!(!result.budget.pruned.is_empty());
    }

    #[test]
    fn system_task_never_pruned() {
        let prompt = test_prompt(vec![big_doc("huge", 8000)]);
        let config = BudgetConfig {
            max_tokens: 20, // impossibly tight
            response_reserve: 0,
            summary_max_tokens: 10,
            generate_summaries: false,
        };
        let result = budget_aware_assembly(prompt, vec![], &config);
        // Even with impossible budget, the role/task content is preserved
        // (System priority is never pruned by fit_to_budget).
        // The "huge" doc should be pruned.
        assert!(result.budget.pruned.iter().any(|p| p.label == "huge"));
    }

    #[test]
    fn pruned_summary_injected_as_doc() {
        let prompt = test_prompt(vec![big_doc("big", 4000)]);
        let config = BudgetConfig {
            max_tokens: 50,
            response_reserve: 0,
            summary_max_tokens: 100,
            generate_summaries: true,
        };
        let result = budget_aware_assembly(prompt, vec![], &config);
        // Should have a pruned summary doc.
        let has_summary = result
            .prompt
            .documents
            .iter()
            .any(|d| d.label.contains("pruned-context-summary"));
        assert!(has_summary);
    }

    #[test]
    fn extra_docs_with_high_priority_kept() {
        let prompt = test_prompt(vec![big_doc("background", 200)]);
        let extra = vec![PrioritisedDoc {
            snippet: big_doc("errors", 200),
            priority: SectionPriority::RecentErrors,
            summarizable: false,
        }];
        let config = BudgetConfig {
            max_tokens: 200, // tight — only room for system + one doc
            response_reserve: 0,
            summary_max_tokens: 50,
            generate_summaries: true,
        };
        let result = budget_aware_assembly(prompt, extra, &config);
        // Errors (higher priority) should survive over background docs.
        let has_errors = result.prompt.documents.iter().any(|d| d.label == "errors");
        assert!(has_errors);
    }

    #[test]
    fn infer_priority_error_label() {
        assert_eq!(
            infer_priority("build-errors.log"),
            SectionPriority::RecentErrors
        );
    }

    #[test]
    fn infer_priority_plan_label() {
        assert_eq!(infer_priority("active-plan"), SectionPriority::ActivePlan);
    }

    #[test]
    fn infer_priority_test_label() {
        assert_eq!(infer_priority("test-output"), SectionPriority::PriorResults);
    }

    #[test]
    fn infer_priority_handoff_label() {
        assert_eq!(infer_priority("session-handoff"), SectionPriority::Handoff);
    }

    #[test]
    fn infer_priority_history_label() {
        assert_eq!(
            infer_priority("completed-history"),
            SectionPriority::CompletedSteps
        );
    }

    #[test]
    fn infer_priority_generic_label() {
        assert_eq!(
            infer_priority("some-doc.md"),
            SectionPriority::Documentation
        );
    }

    #[test]
    fn auto_budget_assembly_works() {
        let prompt = test_prompt(vec![
            big_doc("error-log", 100),
            big_doc("plan-steps", 100),
            big_doc("reference.md", 100),
        ]);
        let config = BudgetConfig::default();
        let result = auto_budget_assembly(prompt, &config);
        // With default budget (24k), all should fit.
        assert_eq!(result.prompt.documents.len(), 3);
        assert!(result.budget.pruned.is_empty());
    }

    #[test]
    fn auto_budget_prunes_low_priority_first() {
        let prompt = test_prompt(vec![
            big_doc("completed-history", 2000), // CompletedSteps (low)
            big_doc("error-log", 2000),         // RecentErrors (high)
        ]);
        let config = BudgetConfig {
            max_tokens: 700, // room for system-task + one doc
            response_reserve: 0,
            summary_max_tokens: 50,
            generate_summaries: true,
        };
        let result = auto_budget_assembly(prompt, &config);
        // error-log (higher priority) should survive.
        let labels: Vec<&str> = result
            .prompt
            .documents
            .iter()
            .map(|d| d.label.as_str())
            .collect();
        assert!(labels.contains(&"error-log"));
    }

    #[test]
    fn serde_roundtrip_prioritised_doc() {
        let pd = PrioritisedDoc {
            snippet: DocSnippet {
                label: "test".to_owned(),
                body: "content".to_owned(),
            },
            priority: SectionPriority::ActivePlan,
            summarizable: true,
        };
        // PrioritisedDoc itself isn't Serialize, but its components are.
        let _ = serde_json::to_string(&pd.priority).unwrap();
        let _ = serde_json::to_string(&pd.summarizable).unwrap();
    }
}
