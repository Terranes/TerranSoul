//! AI-integrations intent matcher — Chunk 15.5.
//!
//! Deterministic phrase-based matcher for the small surface of voice /
//! chat commands that drive the AI-integrations control plane:
//!
//! - Start/stop/status the MCP server.
//! - Open a project in VS Code (with optional explicit path).
//! - List known VS Code windows.
//! - Run the auto-setup writers for Copilot / Claude Desktop / Codex
//!   (HTTP or stdio transport).
//!
//! Why phrase-based and not the existing LLM `intent_classifier`?
//! These intents are short, exact, and high-stakes (they spawn
//! processes and rewrite editor configs). LLM classification would
//! add latency, costs, and a non-zero false-positive rate. A
//! deterministic regex/keyword matcher is fast, free, and trivially
//! auditable — and falls through to normal chat (or to the LLM
//! classifier) on anything it doesn't recognise.
//!
//! The frontend pattern: call [`match_intent`] (via the
//! `match_ai_integration_intent` Tauri command) on every chat turn
//! *before* sending to the LLM. If `Some(intent)`, execute the
//! matching Tauri command and surface the result through the chat
//! pipeline. If `None`, proceed with a normal LLM turn.

use serde::{Deserialize, Serialize};

/// Transport variant requested by an autosetup intent. Matches the
/// shape of the `setup_*_mcp` (HTTP) and `setup_*_mcp_stdio` Tauri
/// commands (Chunks 15.6 + 15.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    Http,
    Stdio,
}

/// All AI-integrations intents recognised by the phrase matcher.
///
/// Variant order ≠ stable wire format — the JSON tag is `kind` so
/// new variants can be added without breaking older clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AiIntegrationIntent {
    /// "start the MCP server", "turn MCP on".
    McpStart,
    /// "stop the MCP server", "turn MCP off".
    McpStop,
    /// "is MCP running?", "MCP status".
    McpStatus,
    /// "open this project in VS Code", "let me code on TerranSoul".
    /// `target` is `None` when the user wants the inferred project root,
    /// or `Some(path)` when an explicit path was uttered.
    VscodeOpenProject { target: Option<String> },
    /// "which VS Code windows do you know about?"
    VscodeListKnown,
    /// "set up Copilot", "set up VS Code". Defaults to stdio transport
    /// since 15.9 made it the canonical choice.
    AutosetupCopilot { transport: McpTransport },
    /// "set up Claude Desktop".
    AutosetupClaude { transport: McpTransport },
    /// "set up Codex" / "set up ChatGPT desktop".
    AutosetupCodex { transport: McpTransport },
}

/// Match a user utterance against the AI-integrations phrase set.
///
/// Returns `None` when nothing recognisable was said — the caller
/// should fall through to normal chat.
///
/// Matching rules:
/// - Case-insensitive.
/// - Punctuation tolerant (commas, periods, question marks).
/// - First match wins; specific phrases are tested before generic
///   ones (e.g. "set up Copilot via stdio" beats "set up Copilot").
pub fn match_intent(text: &str) -> Option<AiIntegrationIntent> {
    let normalised = normalise(text);
    if normalised.is_empty() {
        return None;
    }

    // Order matters: more-specific patterns first.

    // ── VS Code workspace surfacing (Chunk 15.10) ──────────────────
    if let Some(intent) = match_vscode_open(&normalised) {
        return Some(intent);
    }
    if matches_any(&normalised, VSCODE_LIST_PHRASES) {
        return Some(AiIntegrationIntent::VscodeListKnown);
    }

    // ── Auto-setup writers (Chunks 15.6 + 15.9) ────────────────────
    if matches_any(&normalised, COPILOT_PHRASES) {
        return Some(AiIntegrationIntent::AutosetupCopilot {
            transport: detect_transport(&normalised),
        });
    }
    if matches_any(&normalised, CLAUDE_PHRASES) {
        return Some(AiIntegrationIntent::AutosetupClaude {
            transport: detect_transport(&normalised),
        });
    }
    if matches_any(&normalised, CODEX_PHRASES) {
        return Some(AiIntegrationIntent::AutosetupCodex {
            transport: detect_transport(&normalised),
        });
    }

    // ── MCP server control ─────────────────────────────────────────
    if matches_any(&normalised, MCP_STATUS_PHRASES) {
        return Some(AiIntegrationIntent::McpStatus);
    }
    if matches_any(&normalised, MCP_START_PHRASES) {
        return Some(AiIntegrationIntent::McpStart);
    }
    if matches_any(&normalised, MCP_STOP_PHRASES) {
        return Some(AiIntegrationIntent::McpStop);
    }

    None
}

// ─── Phrase tables ──────────────────────────────────────────────────

const MCP_START_PHRASES: &[&str] = &[
    "start the mcp server",
    "start mcp server",
    "start mcp",
    "turn mcp on",
    "turn on mcp",
    "enable mcp",
    "boot mcp",
];

const MCP_STOP_PHRASES: &[&str] = &[
    "stop the mcp server",
    "stop mcp server",
    "stop mcp",
    "turn mcp off",
    "turn off mcp",
    "disable mcp",
    "kill mcp",
    "shutdown mcp",
    "shut down mcp",
];

const MCP_STATUS_PHRASES: &[&str] = &[
    "is mcp running",
    "is the mcp server running",
    "mcp status",
    "mcp server status",
    "show mcp status",
    "what is mcp doing",
    "what's mcp doing",
];

const VSCODE_LIST_PHRASES: &[&str] = &[
    "which vs code windows",
    "which vscode windows",
    "list vs code windows",
    "list vscode windows",
    "what vs code windows do you know",
    "what vscode windows do you know",
    "show me known vs code windows",
    "show me known vscode windows",
];

const COPILOT_PHRASES: &[&str] = &[
    "set up copilot",
    "setup copilot",
    "configure copilot",
    "wire up copilot",
    "let vs code talk to you",
    "let vscode talk to you",
];

const CLAUDE_PHRASES: &[&str] = &[
    "set up claude desktop",
    "setup claude desktop",
    "configure claude desktop",
    "set up claude",
    "setup claude",
    "wire up claude",
];

const CODEX_PHRASES: &[&str] = &[
    "set up codex",
    "setup codex",
    "configure codex",
    "set up chatgpt desktop",
    "setup chatgpt desktop",
];

// ─── VS Code "open project" with optional path ──────────────────────

const VSCODE_OPEN_PREFIXES: &[&str] = &[
    "open this project in vs code",
    "open this project in vscode",
    "open the project in vs code",
    "open the project in vscode",
    "open project in vs code",
    "open project in vscode",
    "let me code on terransoul",
    "let me code in terransoul",
    "show me the code",
    "show me my code",
    "open my code",
];

const VSCODE_OPEN_WITH_PATH: &[&str] = &[
    "open ",          // "open <path> in vs code"
    "code on ",       // "let me code on <path>"
];

fn match_vscode_open(input: &str) -> Option<AiIntegrationIntent> {
    // Plain "open this project in VS Code" with no explicit path.
    if matches_any(input, VSCODE_OPEN_PREFIXES) {
        return Some(AiIntegrationIntent::VscodeOpenProject { target: None });
    }

    // "open <path> in vs code" / "open <path> in vscode"
    for marker in [" in vs code", " in vscode"] {
        if let Some(stripped) = input.strip_suffix(marker) {
            for prefix in VSCODE_OPEN_WITH_PATH {
                if let Some(rest) = stripped.strip_prefix(prefix) {
                    let target = rest.trim();
                    if !target.is_empty() && looks_like_path(target) {
                        return Some(AiIntegrationIntent::VscodeOpenProject {
                            target: Some(target.to_string()),
                        });
                    }
                }
            }
        }
    }

    None
}

/// Normalise input for case-insensitive matching: lowercase, trim,
/// collapse whitespace, strip trailing punctuation.
fn normalise(s: &str) -> String {
    let lower = s.to_lowercase();
    let trimmed = lower.trim_matches(|c: char| {
        c.is_whitespace() || matches!(c, '.' | '?' | '!' | ',' | ';' | ':')
    });
    trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn matches_any(input: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|p| input == *p || input.starts_with(p))
}

/// Detect explicit transport hints in the utterance. Defaults to
/// **stdio** because 15.9 made it the canonical MCP transport.
fn detect_transport(input: &str) -> McpTransport {
    if input.contains(" via http") || input.contains(" over http") || input.contains("http transport") {
        McpTransport::Http
    } else {
        McpTransport::Stdio
    }
}

/// Quick sanity check on a candidate path string. Rejects empty
/// strings and obvious non-paths ("the", "everything", etc.).
fn looks_like_path(candidate: &str) -> bool {
    if candidate.is_empty() || candidate.len() > 1024 {
        return false;
    }
    // A path must contain `/`, `\`, look like `~/...`, or look like a
    // Windows drive letter (`d:\...`).
    candidate.contains('/')
        || candidate.contains('\\')
        || candidate.starts_with('~')
        || (candidate.len() >= 3 && candidate.as_bytes().get(1) == Some(&b':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_match(text: &str, expected: AiIntegrationIntent) {
        match match_intent(text) {
            Some(actual) => assert_eq!(
                actual, expected,
                "input {text:?} matched but produced wrong intent"
            ),
            None => panic!("input {text:?} did not match any intent"),
        }
    }

    fn assert_no_match(text: &str) {
        assert!(
            match_intent(text).is_none(),
            "input {text:?} unexpectedly matched"
        );
    }

    // ── MCP server control ────────────────────────────────────────

    #[test]
    fn mcp_start_recognised() {
        assert_match("start the mcp server", AiIntegrationIntent::McpStart);
        assert_match("Start MCP", AiIntegrationIntent::McpStart);
        assert_match("turn on MCP", AiIntegrationIntent::McpStart);
    }

    #[test]
    fn mcp_stop_recognised() {
        assert_match("stop MCP", AiIntegrationIntent::McpStop);
        assert_match("Turn MCP off.", AiIntegrationIntent::McpStop);
        assert_match("disable mcp", AiIntegrationIntent::McpStop);
    }

    #[test]
    fn mcp_status_recognised() {
        assert_match("is MCP running?", AiIntegrationIntent::McpStatus);
        assert_match("MCP status", AiIntegrationIntent::McpStatus);
    }

    // ── VS Code surfacing (Chunk 15.10) ───────────────────────────

    #[test]
    fn vscode_open_no_path() {
        assert_match(
            "open this project in VS Code",
            AiIntegrationIntent::VscodeOpenProject { target: None },
        );
        assert_match(
            "let me code on TerranSoul",
            AiIntegrationIntent::VscodeOpenProject { target: None },
        );
        assert_match(
            "show me the code",
            AiIntegrationIntent::VscodeOpenProject { target: None },
        );
    }

    #[test]
    fn vscode_open_with_unix_path() {
        assert_match(
            "open /home/user/proj in vs code",
            AiIntegrationIntent::VscodeOpenProject {
                target: Some("/home/user/proj".into()),
            },
        );
    }

    #[test]
    fn vscode_open_with_windows_path() {
        assert_match(
            "open D:\\Git\\TerranSoul in vscode",
            AiIntegrationIntent::VscodeOpenProject {
                target: Some("d:\\git\\terransoul".into()),
            },
        );
    }

    #[test]
    fn vscode_list_recognised() {
        assert_match(
            "which vs code windows do you know about?",
            AiIntegrationIntent::VscodeListKnown,
        );
        assert_match(
            "list vscode windows",
            AiIntegrationIntent::VscodeListKnown,
        );
    }

    // ── Auto-setup writers ────────────────────────────────────────

    #[test]
    fn autosetup_copilot_defaults_to_stdio() {
        assert_match(
            "set up Copilot",
            AiIntegrationIntent::AutosetupCopilot {
                transport: McpTransport::Stdio,
            },
        );
        assert_match(
            "configure Copilot",
            AiIntegrationIntent::AutosetupCopilot {
                transport: McpTransport::Stdio,
            },
        );
    }

    #[test]
    fn autosetup_copilot_with_explicit_http_transport() {
        assert_match(
            "set up Copilot via HTTP",
            AiIntegrationIntent::AutosetupCopilot {
                transport: McpTransport::Http,
            },
        );
    }

    #[test]
    fn autosetup_claude_recognised() {
        assert_match(
            "set up Claude Desktop",
            AiIntegrationIntent::AutosetupClaude {
                transport: McpTransport::Stdio,
            },
        );
    }

    #[test]
    fn autosetup_codex_recognised() {
        assert_match(
            "set up Codex",
            AiIntegrationIntent::AutosetupCodex {
                transport: McpTransport::Stdio,
            },
        );
        assert_match(
            "set up ChatGPT desktop",
            AiIntegrationIntent::AutosetupCodex {
                transport: McpTransport::Stdio,
            },
        );
    }

    // ── Negative cases / fall-through ─────────────────────────────

    #[test]
    fn empty_input_returns_none() {
        assert_no_match("");
        assert_no_match("   ");
    }

    #[test]
    fn unrelated_chat_returns_none() {
        assert_no_match("how are you today?");
        assert_no_match("what's the weather like?");
        assert_no_match("tell me a joke");
        assert_no_match("hello there");
    }

    #[test]
    fn naked_open_command_does_not_match() {
        // "open" alone is far too ambiguous — must have the qualifier.
        assert_no_match("open");
        assert_no_match("open files");
    }

    #[test]
    fn open_with_non_path_target_does_not_match() {
        // "open the door in vs code" is gibberish; the path-shape
        // check rejects "the door".
        assert_no_match("open the door in vs code");
    }

    // ── Robustness ────────────────────────────────────────────────

    #[test]
    fn punctuation_does_not_break_matching() {
        assert_match("Stop MCP!", AiIntegrationIntent::McpStop);
        assert_match("Stop MCP.", AiIntegrationIntent::McpStop);
        assert_match("Stop MCP???", AiIntegrationIntent::McpStop);
    }

    #[test]
    fn extra_whitespace_collapses() {
        assert_match("  start    mcp  ", AiIntegrationIntent::McpStart);
    }

    #[test]
    fn intent_serialises_to_tagged_json() {
        // Frontend depends on the `kind` discriminator + snake_case.
        let intent = AiIntegrationIntent::VscodeOpenProject { target: None };
        let json = serde_json::to_string(&intent).unwrap();
        assert!(json.contains("\"kind\":\"vscode_open_project\""));
    }

    #[test]
    fn transport_serialises_lowercase() {
        let intent = AiIntegrationIntent::AutosetupCopilot {
            transport: McpTransport::Stdio,
        };
        let json = serde_json::to_string(&intent).unwrap();
        assert!(json.contains("\"transport\":\"stdio\""));
    }
}
