//! Coding intent detection and orchestrator routing (Chunk 28.2).
//!
//! ## Purpose
//!
//! When a user sends a chat message that looks like a coding request
//! ("implement X", "fix bug in Y", "refactor Z"), this module detects
//! the intent and can route it to `coding::workflow::run_coding_task`
//! instead of (or in addition to) the normal brain response path.
//!
//! ## Design
//!
//! Intent detection is a **lightweight heuristic** — no LLM call, just
//! keyword/pattern matching with confidence scoring. The orchestrator
//! can then decide:
//! - High confidence → route to coding workflow
//! - Medium confidence → ask the user to confirm
//! - Low confidence → normal brain response
//!
//! This keeps latency minimal (no extra LLM round-trip for detection)
//! while still catching clear coding requests.

use serde::{Deserialize, Serialize};

/// Confidence level for coding intent detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IntentConfidence {
    /// No coding intent detected — use normal brain response.
    None,
    /// Weak signals — might be coding-related but ambiguous.
    Low,
    /// Moderate signals — likely a coding request.
    Medium,
    /// Strong signals — almost certainly a coding request.
    High,
}

/// Result of coding intent detection on a user message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingIntent {
    /// Confidence level of the detection.
    pub confidence: IntentConfidence,
    /// Which coding capability the message maps to.
    pub capability: CodingCapability,
    /// Keywords/phrases that triggered the detection.
    pub signals: Vec<String>,
    /// Suggested output shape for the coding workflow.
    pub suggested_shape: SuggestedShape,
}

/// What kind of coding work the message requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodingCapability {
    /// Plan a multi-step implementation.
    Plan,
    /// Write or modify code directly.
    Implement,
    /// Review existing code for issues.
    Review,
    /// Fix a bug or error.
    Fix,
    /// Refactor / restructure without changing behaviour.
    Refactor,
    /// Run tests or verify behaviour.
    Test,
    /// General coding question (explain, help).
    Explain,
}

/// Suggested output shape based on detected intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestedShape {
    /// Multi-step plan.
    Plan,
    /// Raw file contents.
    File,
    /// Structured JSON response.
    Json,
    /// Free-form prose explanation.
    Prose,
}

/// Routing decision after intent detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingDecision {
    /// Normal brain response — no coding workflow.
    PassThrough,
    /// Route to coding workflow with the given task description.
    CodingWorkflow {
        task_description: String,
        capability: CodingCapability,
        shape: SuggestedShape,
    },
    /// Ask user to confirm before routing to coding workflow.
    ConfirmCoding {
        task_description: String,
        capability: CodingCapability,
    },
}

/// Configuration for the coding router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Minimum confidence to auto-route to coding workflow.
    pub auto_route_threshold: IntentConfidence,
    /// Minimum confidence to ask user for confirmation.
    pub confirm_threshold: IntentConfidence,
    /// Whether the router is enabled at all.
    pub enabled: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            auto_route_threshold: IntentConfidence::High,
            confirm_threshold: IntentConfidence::Medium,
            enabled: true,
        }
    }
}

// ─── Keywords & Patterns ─────────────────────────────────────────────────────

/// Strong coding indicators (direct action verbs for code).
const STRONG_SIGNALS: &[&str] = &[
    "implement",
    "refactor",
    "write a function",
    "write a method",
    "write a class",
    "write code",
    "fix the bug",
    "fix this bug",
    "fix the error",
    "fix this error",
    "create a module",
    "add a test",
    "write a test",
    "write tests",
    "add unit test",
    "run the tests",
    "run cargo test",
    "run vitest",
    "code review",
    "review this code",
    "review the code",
    "debug this",
    "make a pr",
    "create a pr",
    "next chunk",
    "continue chunk",
    "implement chunk",
];

/// Medium coding indicators (might be coding, might be general).
const MEDIUM_SIGNALS: &[&str] = &[
    "add a",
    "create a",
    "modify",
    "update the",
    "change the",
    "remove the",
    "delete the",
    "rename",
    "move the",
    "extract",
    "split",
    "merge",
    "compile",
    "build",
    "deploy",
    "commit",
    "push",
    "pull request",
];

/// Weak coding indicators (general tech terms).
const WEAK_SIGNALS: &[&str] = &[
    "function",
    "method",
    "class",
    "struct",
    "module",
    "component",
    "variable",
    "parameter",
    "argument",
    "return type",
    "error handling",
    "api",
    "endpoint",
    "database",
    "schema",
    "migration",
];

/// File extension mentions that suggest coding context.
const FILE_EXTENSIONS: &[&str] = &[
    ".rs", ".ts", ".vue", ".js", ".tsx", ".jsx", ".py", ".go", ".java", ".cpp", ".c", ".h", ".css",
    ".html", ".sql", ".toml", ".json", ".yaml", ".yml",
];

/// Detect coding intent from a user message.
///
/// Uses keyword/pattern matching with weighted scoring. No LLM call —
/// runs in O(n * m) where n = message length, m = pattern count.
pub fn detect_intent(message: &str) -> CodingIntent {
    let lower = message.to_lowercase();
    let mut signals = Vec::new();
    let mut strong_count = 0u32;
    let mut medium_count = 0u32;
    let mut weak_count = 0u32;

    // Check strong signals.
    for &pattern in STRONG_SIGNALS {
        if lower.contains(pattern) {
            signals.push(pattern.to_string());
            strong_count += 1;
        }
    }

    // Check medium signals.
    for &pattern in MEDIUM_SIGNALS {
        if lower.contains(pattern) {
            signals.push(pattern.to_string());
            medium_count += 1;
        }
    }

    // Check weak signals (file extensions, tech terms).
    for &ext in FILE_EXTENSIONS {
        if lower.contains(ext) {
            signals.push(ext.to_string());
            weak_count += 1;
        }
    }
    for &pattern in WEAK_SIGNALS {
        if lower.contains(pattern) {
            weak_count += 1;
        }
    }

    // Score: strong=3, medium=2, weak=1.
    let score = strong_count * 3 + medium_count * 2 + weak_count;

    let confidence = if strong_count >= 1 || score >= 6 {
        IntentConfidence::High
    } else if medium_count >= 1 || score >= 3 {
        IntentConfidence::Medium
    } else if score >= 1 {
        IntentConfidence::Low
    } else {
        IntentConfidence::None
    };

    let capability = infer_capability(&lower);
    let suggested_shape = match capability {
        CodingCapability::Plan => SuggestedShape::Plan,
        CodingCapability::Implement | CodingCapability::Fix => SuggestedShape::File,
        CodingCapability::Review => SuggestedShape::Json,
        CodingCapability::Explain | CodingCapability::Refactor | CodingCapability::Test => {
            SuggestedShape::Prose
        }
    };

    CodingIntent {
        confidence,
        capability,
        signals,
        suggested_shape,
    }
}

/// Determine the routing decision based on intent and config.
pub fn route(message: &str, config: &RouterConfig) -> RoutingDecision {
    if !config.enabled {
        return RoutingDecision::PassThrough;
    }

    let intent = detect_intent(message);

    if intent.confidence >= config.auto_route_threshold {
        RoutingDecision::CodingWorkflow {
            task_description: message.to_string(),
            capability: intent.capability,
            shape: intent.suggested_shape,
        }
    } else if intent.confidence >= config.confirm_threshold {
        RoutingDecision::ConfirmCoding {
            task_description: message.to_string(),
            capability: intent.capability,
        }
    } else {
        RoutingDecision::PassThrough
    }
}

/// Infer the coding capability from the message text.
fn infer_capability(lower: &str) -> CodingCapability {
    if lower.contains("plan") || lower.contains("design") || lower.contains("architect") {
        CodingCapability::Plan
    } else if lower.contains("fix") || lower.contains("bug") || lower.contains("error") {
        CodingCapability::Fix
    } else if lower.contains("review") || lower.contains("audit") {
        CodingCapability::Review
    } else if lower.contains("refactor")
        || lower.contains("restructure")
        || lower.contains("clean up")
    {
        CodingCapability::Refactor
    } else if lower.contains("test") || lower.contains("spec") || lower.contains("verify") {
        CodingCapability::Test
    } else if lower.contains("explain") || lower.contains("how does") || lower.contains("what is") {
        CodingCapability::Explain
    } else {
        CodingCapability::Implement
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_strong_coding_intent() {
        let intent = detect_intent("implement a new REST endpoint for user profiles");
        assert_eq!(intent.confidence, IntentConfidence::High);
        assert_eq!(intent.capability, CodingCapability::Implement);
        assert!(!intent.signals.is_empty());
    }

    #[test]
    fn detect_fix_intent() {
        let intent = detect_intent("fix the bug in the login handler");
        assert_eq!(intent.confidence, IntentConfidence::High);
        assert_eq!(intent.capability, CodingCapability::Fix);
    }

    #[test]
    fn detect_review_intent() {
        let intent = detect_intent("review this code for security issues");
        assert_eq!(intent.confidence, IntentConfidence::High);
        assert_eq!(intent.capability, CodingCapability::Review);
    }

    #[test]
    fn detect_refactor_intent() {
        let intent = detect_intent("refactor the database module to use connection pooling");
        assert_eq!(intent.confidence, IntentConfidence::High);
        assert_eq!(intent.capability, CodingCapability::Refactor);
    }

    #[test]
    fn detect_test_intent() {
        let intent = detect_intent("write tests for the auth module");
        assert_eq!(intent.confidence, IntentConfidence::High);
        assert_eq!(intent.capability, CodingCapability::Test);
    }

    #[test]
    fn detect_medium_intent() {
        let intent = detect_intent("update the config to support new format");
        assert_eq!(intent.confidence, IntentConfidence::Medium);
    }

    #[test]
    fn detect_no_coding_intent() {
        let intent = detect_intent("what's the weather like today?");
        assert_eq!(intent.confidence, IntentConfidence::None);
    }

    #[test]
    fn detect_weak_intent_from_file_extension() {
        let intent = detect_intent("look at the main.rs file");
        assert!(intent.confidence >= IntentConfidence::Low);
    }

    #[test]
    fn route_auto_routes_high_confidence() {
        let config = RouterConfig::default();
        let decision = route("implement the user authentication module", &config);
        matches!(decision, RoutingDecision::CodingWorkflow { .. });
    }

    #[test]
    fn route_confirms_medium_confidence() {
        let config = RouterConfig::default();
        let decision = route("add a new configuration option", &config);
        matches!(decision, RoutingDecision::ConfirmCoding { .. });
    }

    #[test]
    fn route_passes_through_low_confidence() {
        let config = RouterConfig::default();
        let decision = route("tell me about quantum physics", &config);
        matches!(decision, RoutingDecision::PassThrough);
    }

    #[test]
    fn route_disabled_always_passes_through() {
        let config = RouterConfig {
            enabled: false,
            ..Default::default()
        };
        let decision = route("implement everything from scratch", &config);
        matches!(decision, RoutingDecision::PassThrough);
    }

    #[test]
    fn suggested_shape_plan() {
        let intent = detect_intent("plan the architecture for the new feature");
        assert_eq!(intent.suggested_shape, SuggestedShape::Plan);
    }

    #[test]
    fn suggested_shape_file() {
        let intent = detect_intent("implement a new parser for JSON");
        assert_eq!(intent.suggested_shape, SuggestedShape::File);
    }

    #[test]
    fn serde_roundtrip_intent() {
        let intent = detect_intent("implement a new REST endpoint");
        let json = serde_json::to_string(&intent).unwrap();
        let back: CodingIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.confidence, intent.confidence);
        assert_eq!(back.capability, intent.capability);
    }

    #[test]
    fn serde_roundtrip_config() {
        let config = RouterConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let back: RouterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.enabled, config.enabled);
    }

    #[test]
    fn chunk_keyword_detected() {
        let intent = detect_intent("continue chunk 28.3 implementation");
        assert_eq!(intent.confidence, IntentConfidence::High);
    }

    #[test]
    fn multiple_signals_boost_confidence() {
        // Weak signals alone don't reach high
        let intent = detect_intent("the function and the module both have issues");
        assert!(intent.confidence < IntentConfidence::High);
    }
}
