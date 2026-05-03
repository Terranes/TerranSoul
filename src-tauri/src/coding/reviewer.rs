//! Reviewer sub-agent — validates LLM-produced diffs before application.
//!
//! The reviewer runs a second LLM pass over the generated code changes,
//! requesting a strict-JSON verdict with structured issue reporting.
//! Callers use [`ReviewVerdict`] to decide whether to apply or reject
//! the diff.
//!
//! This module is **pure logic + types** — it builds prompts and parses
//! results but does not call the LLM itself. The caller (orchestrator or
//! workflow engine) is responsible for the actual LLM invocation via
//! `coding::workflow::run_coding_task`.

use serde::{Deserialize, Serialize};

use super::prompting::{extract_tag, CodingPrompt, DocSnippet, OutputShape};
use super::workflow::{CodingTask, TaskDocument, TaskOutputKind};

// ─── Types ───────────────────────────────────────────────────────────────────

/// Severity level for a review issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A single issue found during code review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewIssue {
    pub severity: Severity,
    pub file: String,
    pub line: u32,
    pub msg: String,
}

/// Structured review result parsed from the model's JSON reply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewResult {
    pub ok: bool,
    pub issues: Vec<ReviewIssue>,
}

/// Final verdict: accept or reject (with reasons).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewVerdict {
    /// The diff is acceptable — no blocking issues.
    Accept,
    /// The diff must be rejected.
    Reject {
        /// Human-readable summary of why.
        reason: String,
        /// The error-level issues that caused rejection.
        blocking_issues: Vec<ReviewIssue>,
    },
}

/// Configuration for the reviewer sub-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewerConfig {
    /// Whether warnings alone (without errors) should cause rejection.
    /// Default: `false` — only `Severity::Error` issues block.
    #[serde(default)]
    pub reject_on_warnings: bool,
    /// Maximum number of issues to include in the rejection reason.
    /// Default: 10.
    #[serde(default = "default_max_issues")]
    pub max_issues_in_reason: usize,
}

fn default_max_issues() -> usize {
    10
}

impl Default for ReviewerConfig {
    fn default() -> Self {
        Self {
            reject_on_warnings: false,
            max_issues_in_reason: default_max_issues(),
        }
    }
}

// ─── Schema Description ──────────────────────────────────────────────────────

/// The JSON schema description passed to the LLM as part of the
/// `StrictJson` output shape contract.
pub const REVIEW_SCHEMA_DESCRIPTION: &str = r#"{ "ok": bool, "issues": [{ "severity": "error"|"warning"|"info", "file": "path/to/file.rs", "line": 42, "msg": "description of the issue" }] }"#;

// ─── Prompt Builder ──────────────────────────────────────────────────────────

/// Build a [`CodingTask`] that asks the LLM to review the given diff.
///
/// The returned task uses `OutputShape::StrictJson` with the review schema.
/// The caller should pass this to `run_coding_task` and then feed the
/// result's `payload` into [`parse_review_result`].
pub fn build_review_task(task_id: &str, diff: &str, context_docs: Vec<TaskDocument>) -> CodingTask {
    let description = format!(
        "You are a senior code reviewer. Analyse the following diff for correctness, \
         security vulnerabilities, performance issues, and adherence to project conventions.\n\n\
         Evaluate EVERY file in the diff. For each issue found, record it with the appropriate \
         severity:\n\
         - \"error\" — must be fixed before merge (bugs, security holes, compilation failures)\n\
         - \"warning\" — should be fixed but not blocking (style, minor perf, naming)\n\
         - \"info\" — optional suggestion (refactoring ideas, alternative approaches)\n\n\
         If the diff is correct and ready to merge, set \"ok\": true and \"issues\": [].\n\
         If there are any error-severity issues, set \"ok\": false.\n\n\
         THE DIFF TO REVIEW:\n```diff\n{diff}\n```"
    );

    CodingTask {
        id: task_id.to_string(),
        description,
        repo_root: None,
        include_rules: false,
        include_instructions: false,
        include_docs: false,
        output_kind: TaskOutputKind::StrictJson {
            schema_description: REVIEW_SCHEMA_DESCRIPTION.to_string(),
        },
        extra_documents: context_docs,
        target_paths: Vec::new(),
        prior_handoff: None,
    }
}

/// Build a raw [`CodingPrompt`] for the reviewer (useful when the caller
/// wants to drive the LLM directly without the full workflow).
pub fn build_review_prompt(diff: &str, extra_docs: &[DocSnippet]) -> CodingPrompt {
    let task = format!(
        "You are a senior code reviewer. Analyse the following diff for correctness, \
         security vulnerabilities, performance issues, and adherence to project conventions.\n\n\
         Evaluate EVERY file in the diff. For each issue found, record it with the appropriate \
         severity:\n\
         - \"error\" — must be fixed before merge (bugs, security holes, compilation failures)\n\
         - \"warning\" — should be fixed but not blocking (style, minor perf, naming)\n\
         - \"info\" — optional suggestion (refactoring ideas, alternative approaches)\n\n\
         If the diff is correct and ready to merge, set \"ok\": true and \"issues\": [].\n\
         If there are any error-severity issues, set \"ok\": false.\n\n\
         THE DIFF TO REVIEW:\n```diff\n{diff}\n```"
    );

    CodingPrompt {
        role: "Expert code reviewer with deep knowledge of Rust, TypeScript, \
              security best practices, and the OWASP Top 10. You are thorough, \
              precise, and only flag real issues — no false positives."
            .to_string(),
        task,
        negative_constraints: vec![
            "Do not suggest style-only changes unless they affect readability.".to_string(),
            "Do not flag TODO/FIXME comments as issues.".to_string(),
            "Do not hallucinate line numbers — use 0 if unsure.".to_string(),
        ],
        documents: extra_docs.to_vec(),
        output: OutputShape::StrictJson {
            schema_description: REVIEW_SCHEMA_DESCRIPTION.to_string(),
        },
        example: Some(
            r#"<json>{"ok": false, "issues": [{"severity": "error", "file": "src/main.rs", "line": 42, "msg": "division by zero when denominator is 0"}]}</json>"#
                .to_string(),
        ),
        assistant_prefill: Some("<json>".to_string()),
        error_handling: vec![],
    }
}

// ─── Parser ──────────────────────────────────────────────────────────────────

/// Parse the model's JSON reply (from inside a `<json>` tag) into a
/// [`ReviewResult`].
///
/// Returns `None` if the payload is not valid JSON matching the schema.
/// Lenient: accepts the raw JSON even without the enclosing `<json>` tag
/// (handles both `run_coding_task` payload extraction and raw replies).
pub fn parse_review_result(payload: &str) -> Option<ReviewResult> {
    // First try: payload is already the extracted content (from CodingTaskResult.payload)
    if let Ok(result) = serde_json::from_str::<ReviewResult>(payload) {
        return Some(result);
    }
    // Second try: payload still has the <json> wrapper
    if let Some(inner) = extract_tag(payload, "json") {
        if let Ok(result) = serde_json::from_str::<ReviewResult>(inner) {
            return Some(result);
        }
    }
    None
}

// ─── Verdict ─────────────────────────────────────────────────────────────────

/// Decide whether to accept or reject the diff based on the review result.
pub fn decide(result: &ReviewResult, config: &ReviewerConfig) -> ReviewVerdict {
    // Explicit rejection by the model
    if !result.ok {
        let blocking: Vec<ReviewIssue> = result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .take(config.max_issues_in_reason)
            .cloned()
            .collect();

        let reason = if blocking.is_empty() {
            "Reviewer marked ok=false but did not specify error-level issues.".to_string()
        } else {
            format!(
                "{} error(s) found: {}",
                blocking.len(),
                blocking
                    .iter()
                    .map(|i| format!("{}:{} — {}", i.file, i.line, i.msg))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        };

        return ReviewVerdict::Reject {
            reason,
            blocking_issues: blocking,
        };
    }

    // Check for error-level issues even if model said ok=true (safety net)
    let errors: Vec<ReviewIssue> = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .take(config.max_issues_in_reason)
        .cloned()
        .collect();

    if !errors.is_empty() {
        let reason = format!(
            "{} error(s) found despite ok=true: {}",
            errors.len(),
            errors
                .iter()
                .map(|i| format!("{}:{} — {}", i.file, i.line, i.msg))
                .collect::<Vec<_>>()
                .join("; ")
        );
        return ReviewVerdict::Reject {
            reason,
            blocking_issues: errors,
        };
    }

    // Optionally reject on warnings
    if config.reject_on_warnings {
        let warnings: Vec<ReviewIssue> = result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .take(config.max_issues_in_reason)
            .cloned()
            .collect();

        if !warnings.is_empty() {
            let reason = format!(
                "{} warning(s) found (reject_on_warnings=true): {}",
                warnings.len(),
                warnings
                    .iter()
                    .map(|i| format!("{}:{} — {}", i.file, i.line, i.msg))
                    .collect::<Vec<_>>()
                    .join("; ")
            );
            return ReviewVerdict::Reject {
                reason,
                blocking_issues: warnings,
            };
        }
    }

    ReviewVerdict::Accept
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_accept() {
        let json = r#"{"ok": true, "issues": []}"#;
        let result = parse_review_result(json).unwrap();
        assert!(result.ok);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn parse_valid_reject_with_errors() {
        let json = r#"{"ok": false, "issues": [{"severity": "error", "file": "src/main.rs", "line": 10, "msg": "null deref"}]}"#;
        let result = parse_review_result(json).unwrap();
        assert!(!result.ok);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].severity, Severity::Error);
        assert_eq!(result.issues[0].file, "src/main.rs");
        assert_eq!(result.issues[0].line, 10);
        assert_eq!(result.issues[0].msg, "null deref");
    }

    #[test]
    fn parse_with_json_tag_wrapper() {
        let raw = r#"Some preamble text <json>{"ok": true, "issues": []}</json> trailing"#;
        let result = parse_review_result(raw).unwrap();
        assert!(result.ok);
    }

    #[test]
    fn parse_invalid_json_returns_none() {
        assert!(parse_review_result("not json at all").is_none());
        assert!(parse_review_result("{\"ok\": true}").is_none()); // missing issues
    }

    #[test]
    fn parse_mixed_severities() {
        let json = r#"{"ok": false, "issues": [
            {"severity": "error", "file": "a.rs", "line": 1, "msg": "bug"},
            {"severity": "warning", "file": "b.rs", "line": 2, "msg": "style"},
            {"severity": "info", "file": "c.rs", "line": 3, "msg": "suggestion"}
        ]}"#;
        let result = parse_review_result(json).unwrap();
        assert_eq!(result.issues.len(), 3);
        assert_eq!(result.issues[0].severity, Severity::Error);
        assert_eq!(result.issues[1].severity, Severity::Warning);
        assert_eq!(result.issues[2].severity, Severity::Info);
    }

    #[test]
    fn decide_accept_when_ok_no_issues() {
        let result = ReviewResult {
            ok: true,
            issues: vec![],
        };
        let verdict = decide(&result, &ReviewerConfig::default());
        assert_eq!(verdict, ReviewVerdict::Accept);
    }

    #[test]
    fn decide_accept_with_info_only() {
        let result = ReviewResult {
            ok: true,
            issues: vec![ReviewIssue {
                severity: Severity::Info,
                file: "x.rs".to_string(),
                line: 1,
                msg: "consider refactoring".to_string(),
            }],
        };
        let verdict = decide(&result, &ReviewerConfig::default());
        assert_eq!(verdict, ReviewVerdict::Accept);
    }

    #[test]
    fn decide_accept_with_warnings_default_config() {
        let result = ReviewResult {
            ok: true,
            issues: vec![ReviewIssue {
                severity: Severity::Warning,
                file: "x.rs".to_string(),
                line: 5,
                msg: "naming convention".to_string(),
            }],
        };
        let verdict = decide(&result, &ReviewerConfig::default());
        assert_eq!(verdict, ReviewVerdict::Accept);
    }

    #[test]
    fn decide_reject_when_ok_false() {
        let result = ReviewResult {
            ok: false,
            issues: vec![ReviewIssue {
                severity: Severity::Error,
                file: "lib.rs".to_string(),
                line: 42,
                msg: "use after free".to_string(),
            }],
        };
        let verdict = decide(&result, &ReviewerConfig::default());
        match verdict {
            ReviewVerdict::Reject {
                reason,
                blocking_issues,
            } => {
                assert!(reason.contains("1 error(s) found"));
                assert_eq!(blocking_issues.len(), 1);
                assert_eq!(blocking_issues[0].msg, "use after free");
            }
            ReviewVerdict::Accept => panic!("expected rejection"),
        }
    }

    #[test]
    fn decide_reject_on_error_even_if_ok_true() {
        // Safety net: model says ok=true but has error issues
        let result = ReviewResult {
            ok: true,
            issues: vec![ReviewIssue {
                severity: Severity::Error,
                file: "main.rs".to_string(),
                line: 1,
                msg: "panic in production path".to_string(),
            }],
        };
        let verdict = decide(&result, &ReviewerConfig::default());
        match verdict {
            ReviewVerdict::Reject {
                reason,
                blocking_issues,
            } => {
                assert!(reason.contains("despite ok=true"));
                assert_eq!(blocking_issues.len(), 1);
            }
            ReviewVerdict::Accept => panic!("expected rejection"),
        }
    }

    #[test]
    fn decide_reject_on_warnings_when_configured() {
        let config = ReviewerConfig {
            reject_on_warnings: true,
            max_issues_in_reason: 10,
        };
        let result = ReviewResult {
            ok: true,
            issues: vec![ReviewIssue {
                severity: Severity::Warning,
                file: "utils.rs".to_string(),
                line: 77,
                msg: "variable shadowing".to_string(),
            }],
        };
        let verdict = decide(&result, &config);
        match verdict {
            ReviewVerdict::Reject { reason, .. } => {
                assert!(reason.contains("reject_on_warnings=true"));
            }
            ReviewVerdict::Accept => panic!("expected rejection"),
        }
    }

    #[test]
    fn decide_reject_ok_false_no_error_issues() {
        // Model says ok=false but only has warnings — still reject
        let result = ReviewResult {
            ok: false,
            issues: vec![ReviewIssue {
                severity: Severity::Warning,
                file: "x.rs".to_string(),
                line: 1,
                msg: "bad".to_string(),
            }],
        };
        let verdict = decide(&result, &ReviewerConfig::default());
        match verdict {
            ReviewVerdict::Reject { reason, .. } => {
                assert!(reason.contains("did not specify error-level issues"));
            }
            ReviewVerdict::Accept => panic!("expected rejection"),
        }
    }

    #[test]
    fn build_review_task_shape() {
        let task = build_review_task("rev-001", "+ fn foo() {}\n- fn bar() {}", vec![]);
        assert_eq!(task.id, "rev-001");
        assert!(task.description.contains("THE DIFF TO REVIEW"));
        assert!(task.description.contains("fn foo()"));
        match task.output_kind {
            TaskOutputKind::StrictJson {
                ref schema_description,
            } => {
                assert!(schema_description.contains("ok"));
                assert!(schema_description.contains("issues"));
            }
            _ => panic!("expected StrictJson output kind"),
        }
    }

    #[test]
    fn build_review_prompt_shape() {
        let prompt = build_review_prompt("diff content here", &[]);
        assert!(prompt.task.contains("diff content here"));
        assert!(prompt.role.contains("code reviewer"));
        assert_eq!(
            prompt.output,
            OutputShape::StrictJson {
                schema_description: REVIEW_SCHEMA_DESCRIPTION.to_string()
            }
        );
        assert!(prompt.example.is_some());
        assert_eq!(prompt.assistant_prefill, Some("<json>".to_string()));
    }

    #[test]
    fn config_default_values() {
        let cfg = ReviewerConfig::default();
        assert!(!cfg.reject_on_warnings);
        assert_eq!(cfg.max_issues_in_reason, 10);
    }

    #[test]
    fn max_issues_in_reason_caps_output() {
        let config = ReviewerConfig {
            reject_on_warnings: false,
            max_issues_in_reason: 2,
        };
        let result = ReviewResult {
            ok: false,
            issues: (0..5)
                .map(|i| ReviewIssue {
                    severity: Severity::Error,
                    file: format!("f{i}.rs"),
                    line: i,
                    msg: format!("err {i}"),
                })
                .collect(),
        };
        let verdict = decide(&result, &config);
        match verdict {
            ReviewVerdict::Reject {
                blocking_issues, ..
            } => {
                assert_eq!(blocking_issues.len(), 2);
            }
            ReviewVerdict::Accept => panic!("expected rejection"),
        }
    }

    #[test]
    fn severity_serde_roundtrip() {
        let json = serde_json::to_string(&Severity::Error).unwrap();
        assert_eq!(json, "\"error\"");
        let back: Severity = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Severity::Error);

        let json = serde_json::to_string(&Severity::Warning).unwrap();
        assert_eq!(json, "\"warning\"");

        let json = serde_json::to_string(&Severity::Info).unwrap();
        assert_eq!(json, "\"info\"");
    }

    #[test]
    fn review_result_serde_roundtrip() {
        let result = ReviewResult {
            ok: false,
            issues: vec![ReviewIssue {
                severity: Severity::Error,
                file: "test.rs".to_string(),
                line: 99,
                msg: "overflow".to_string(),
            }],
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: ReviewResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, back);
    }
}
