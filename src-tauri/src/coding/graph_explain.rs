//! Persona-adaptive graph explanations.
//!
//! Generates natural-language explanations of code-graph elements
//! (symbols, clusters, call graphs) tailored to different audience levels.
//! Uses the active brain mode for LLM summarization when available.

use serde::{Deserialize, Serialize};

/// The intended audience for a code explanation.
///
/// Each variant produces a different depth, vocabulary, and focus
/// in the generated explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Audience {
    /// New to the codebase — needs high-level "what" and "why",
    /// minimal jargon, analogies welcome.
    Newcomer,
    /// Active contributor — wants implementation details, edge cases,
    /// and how this fits the broader architecture.
    Maintainer,
    /// Non-technical stakeholder — needs business impact, risk,
    /// and plain-English summaries.
    ProjectManager,
    /// Expert user — wants dense technical detail, performance
    /// characteristics, and internal invariants.
    PowerUser,
}

impl Audience {
    /// Parse from a string (case-insensitive, underscore or hyphen).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().replace('-', "_").as_str() {
            "newcomer" | "new" | "beginner" => Some(Self::Newcomer),
            "maintainer" | "contributor" | "dev" => Some(Self::Maintainer),
            "project_manager" | "pm" | "manager" | "stakeholder" => Some(Self::ProjectManager),
            "power_user" | "expert" | "advanced" => Some(Self::PowerUser),
            _ => None,
        }
    }

    /// Human-readable label for this audience.
    pub fn label(self) -> &'static str {
        match self {
            Self::Newcomer => "newcomer",
            Self::Maintainer => "maintainer",
            Self::ProjectManager => "project manager",
            Self::PowerUser => "power user",
        }
    }

    /// Build a system prompt tailored to this audience.
    pub fn system_prompt(self) -> &'static str {
        match self {
            Self::Newcomer => concat!(
                "You are a friendly code guide explaining a software component to someone ",
                "who has never seen this codebase before. Use simple language, avoid jargon, ",
                "use analogies where helpful, and focus on WHAT it does and WHY it exists. ",
                "Keep it concise (3-5 sentences)."
            ),
            Self::Maintainer => concat!(
                "You are a senior engineer explaining a code component to a fellow contributor. ",
                "Focus on implementation details, design decisions, edge cases, dependencies, ",
                "and how this fits the broader architecture. Be specific and technical. ",
                "Keep it concise (3-5 sentences)."
            ),
            Self::ProjectManager => concat!(
                "You are explaining a code component to a non-technical project manager. ",
                "Focus on business impact, what user-facing features this enables, risk level ",
                "if it breaks, and dependencies on other teams/services. Avoid code terminology. ",
                "Keep it concise (2-3 sentences)."
            ),
            Self::PowerUser => concat!(
                "You are writing a dense technical summary for an expert. Include performance ",
                "characteristics, invariants, thread-safety, error propagation paths, and ",
                "internal data structures. Assume the reader understands all programming ",
                "concepts. Keep it concise but information-dense (3-5 sentences)."
            ),
        }
    }
}

/// A code-graph explanation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExplanation {
    /// The subject being explained (symbol name, cluster label, etc.).
    pub subject: String,
    /// Which audience the explanation targets.
    pub audience: String,
    /// The generated explanation text (empty if brain unavailable).
    pub explanation: String,
    /// Context provided to the LLM (symbol list, edges, etc.).
    pub context_summary: String,
}

/// Build a user-prompt describing a symbol and its relationships for LLM explanation.
pub fn build_symbol_context(
    symbol_name: &str,
    symbol_kind: &str,
    file_path: &str,
    incoming: &[(String, String)], // (caller_name, edge_kind)
    outgoing: &[(String, String)], // (callee_name, edge_kind)
) -> String {
    let mut ctx = format!("Symbol: `{symbol_name}` (kind: {symbol_kind})\nFile: {file_path}\n");
    if !incoming.is_empty() {
        ctx.push_str("\nCalled by:\n");
        for (name, kind) in incoming.iter().take(15) {
            ctx.push_str(&format!("  - {name} ({kind})\n"));
        }
        if incoming.len() > 15 {
            ctx.push_str(&format!("  ... and {} more\n", incoming.len() - 15));
        }
    }
    if !outgoing.is_empty() {
        ctx.push_str("\nCalls:\n");
        for (name, kind) in outgoing.iter().take(15) {
            ctx.push_str(&format!("  - {name} ({kind})\n"));
        }
        if outgoing.len() > 15 {
            ctx.push_str(&format!("  ... and {} more\n", outgoing.len() - 15));
        }
    }
    if incoming.is_empty() && outgoing.is_empty() {
        ctx.push_str("\nNo known callers or callees in the indexed graph.\n");
    }
    ctx
}

/// Build a user-prompt describing a cluster for LLM explanation.
pub fn build_cluster_context(
    cluster_label: &str,
    symbols: &[(String, String)], // (name, kind)
    edge_count: usize,
) -> String {
    let mut ctx = format!(
        "Cluster: \"{cluster_label}\"\nSymbols ({}):\n",
        symbols.len()
    );
    for (name, kind) in symbols.iter().take(20) {
        ctx.push_str(&format!("  - {name} ({kind})\n"));
    }
    if symbols.len() > 20 {
        ctx.push_str(&format!("  ... and {} more\n", symbols.len() - 20));
    }
    ctx.push_str(&format!("\nInternal edges: {edge_count}\n"));
    ctx
}

/// Generate an explanation for a symbol using the given audience.
///
/// This is a pure prompt-builder — the caller is responsible for
/// sending the result to an LLM via `complete_via_mode` or similar.
pub fn explain_symbol_prompt(
    audience: Audience,
    symbol_name: &str,
    symbol_kind: &str,
    file_path: &str,
    incoming: &[(String, String)],
    outgoing: &[(String, String)],
) -> (String, String) {
    let system = audience.system_prompt().to_owned();
    let user = format!(
        "Explain this code component:\n\n{}",
        build_symbol_context(symbol_name, symbol_kind, file_path, incoming, outgoing)
    );
    (system, user)
}

/// Generate an explanation for a cluster using the given audience.
pub fn explain_cluster_prompt(
    audience: Audience,
    cluster_label: &str,
    symbols: &[(String, String)],
    edge_count: usize,
) -> (String, String) {
    let system = audience.system_prompt().to_owned();
    let user = format!(
        "Explain this code cluster:\n\n{}",
        build_cluster_context(cluster_label, symbols, edge_count)
    );
    (system, user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audience_parse_variants() {
        assert_eq!(Audience::parse("newcomer"), Some(Audience::Newcomer));
        assert_eq!(Audience::parse("Maintainer"), Some(Audience::Maintainer));
        assert_eq!(Audience::parse("PM"), Some(Audience::ProjectManager));
        assert_eq!(Audience::parse("power-user"), Some(Audience::PowerUser));
        assert_eq!(Audience::parse("expert"), Some(Audience::PowerUser));
        assert_eq!(Audience::parse("beginner"), Some(Audience::Newcomer));
        assert_eq!(Audience::parse("unknown"), None);
    }

    #[test]
    fn system_prompts_differ_by_audience() {
        let prompts: Vec<&str> = [
            Audience::Newcomer,
            Audience::Maintainer,
            Audience::ProjectManager,
            Audience::PowerUser,
        ]
        .iter()
        .map(|a| a.system_prompt())
        .collect();

        // All prompts should be unique
        for (i, p1) in prompts.iter().enumerate() {
            for (j, p2) in prompts.iter().enumerate() {
                if i != j {
                    assert_ne!(p1, p2, "Audiences {i} and {j} have same prompt");
                }
            }
        }
    }

    #[test]
    fn build_symbol_context_includes_edges() {
        let incoming = vec![("caller_a".to_string(), "calls".to_string())];
        let outgoing = vec![
            ("callee_b".to_string(), "calls".to_string()),
            ("callee_c".to_string(), "imports".to_string()),
        ];
        let ctx = build_symbol_context("my_fn", "function", "src/main.rs", &incoming, &outgoing);
        assert!(ctx.contains("my_fn"));
        assert!(ctx.contains("caller_a"));
        assert!(ctx.contains("callee_b"));
        assert!(ctx.contains("callee_c"));
    }

    #[test]
    fn build_symbol_context_truncates_long_lists() {
        let many: Vec<(String, String)> = (0..20)
            .map(|i| (format!("sym_{i}"), "calls".to_string()))
            .collect();
        let ctx = build_symbol_context("target", "function", "a.rs", &many, &[]);
        assert!(ctx.contains("... and 5 more"));
    }

    #[test]
    fn build_cluster_context_basic() {
        let symbols = vec![
            ("init".to_string(), "function".to_string()),
            ("Config".to_string(), "struct".to_string()),
        ];
        let ctx = build_cluster_context("Startup", &symbols, 3);
        assert!(ctx.contains("Startup"));
        assert!(ctx.contains("init"));
        assert!(ctx.contains("Config"));
        assert!(ctx.contains("Internal edges: 3"));
    }

    #[test]
    fn explain_symbol_prompt_returns_pair() {
        let (system, user) = explain_symbol_prompt(
            Audience::Newcomer,
            "handle_request",
            "function",
            "src/server.rs",
            &[],
            &[("parse_body".to_string(), "calls".to_string())],
        );
        assert!(system.contains("never seen this codebase"));
        assert!(user.contains("handle_request"));
        assert!(user.contains("parse_body"));
    }

    #[test]
    fn explain_cluster_prompt_returns_pair() {
        let symbols = vec![("run".to_string(), "function".to_string())];
        let (system, user) =
            explain_cluster_prompt(Audience::ProjectManager, "Core Engine", &symbols, 5);
        assert!(system.contains("non-technical"));
        assert!(user.contains("Core Engine"));
    }
}
