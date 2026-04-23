//! OpenClaw bridge agent — example provider.
//!
//! This module is the **reference implementation** for integrating an external
//! agent platform (OpenClaw) with TerranSoul's [`AgentProvider`] trait. It is
//! installed via the Agent Marketplace as `openclaw-bridge` and demonstrates:
//!
//! 1. **Capability gating** — chat / filesystem / network are declared in the
//!    manifest (see [`crate::registry_server::catalog`]) and the user must
//!    grant them via the consent dialog before any tool call may run.
//! 2. **Tool-call wiring** — the bridge inspects the user message for
//!    OpenClaw-style tool directives (`/openclaw read <path>`, `/openclaw fetch <url>`)
//!    and dispatches them through guarded handlers. Real OpenClaw deployments
//!    would forward to the OpenClaw runtime over JSON-RPC; this example keeps
//!    everything in-process so the integration shape is testable end-to-end
//!    without a network dependency.
//! 3. **Sentiment passthrough** — the bridge returns a [`Sentiment::Neutral`]
//!    by default and switches to [`Sentiment::Happy`] when a tool succeeds, so
//!    the VRM character's expression reflects the bridge's outcome — exactly
//!    like any first-party agent.
//!
//! See `instructions/OPENCLAW-EXAMPLE.md` for the end-to-end walkthrough.

use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::stub_agent::Sentiment;
use super::AgentProvider;

/// Registered tool name used by [`OpenClawAgent::handle_command`] dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenClawTool {
    /// `/openclaw read <relative-path>` — read a file via the filesystem capability.
    Read,
    /// `/openclaw fetch <url>` — fetch a URL via the network capability.
    Fetch,
    /// `/openclaw chat <prompt>` — forward to the configured brain.
    Chat,
}

impl OpenClawTool {
    /// Map the textual tool name from a `/openclaw <tool> ...` directive.
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "read" => Some(Self::Read),
            "fetch" => Some(Self::Fetch),
            "chat" => Some(Self::Chat),
            _ => None,
        }
    }

    /// Capability that must be granted before this tool may run.
    pub fn required_capability(&self) -> &'static str {
        match self {
            Self::Read => "file_read",
            Self::Fetch => "network",
            Self::Chat => "chat",
        }
    }
}

/// Result of parsing a user message that may carry an OpenClaw directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedMessage<'a> {
    /// A free-form chat message (no `/openclaw` directive).
    Chat(&'a str),
    /// A directive — `(tool, argument)`.
    Directive(OpenClawTool, &'a str),
    /// A `/openclaw` line that did not match any known tool.
    UnknownDirective(&'a str),
}

/// Parse a single message line. Pure function — no side effects.
pub fn parse(message: &str) -> ParsedMessage<'_> {
    let trimmed = message.trim();
    if let Some(rest) = trimmed.strip_prefix("/openclaw ") {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let tool = parts.next().unwrap_or("").trim();
        let arg = parts.next().unwrap_or("").trim();
        match OpenClawTool::from_token(tool) {
            Some(t) => ParsedMessage::Directive(t, arg),
            None => ParsedMessage::UnknownDirective(tool),
        }
    } else {
        ParsedMessage::Chat(trimmed)
    }
}

/// Bridge to OpenClaw. The bridge takes a snapshot of the granted capability
/// set at construction time so it can authoritatively reject tool calls that
/// the user hasn't consented to — independent of what the orchestrator thinks.
pub struct OpenClawAgent {
    id: String,
    granted_capabilities: Arc<Mutex<HashSet<String>>>,
}

impl OpenClawAgent {
    /// Create a new bridge with a starting capability set.
    pub fn new(granted: impl IntoIterator<Item = String>) -> Self {
        Self {
            id: "openclaw-bridge".to_string(),
            granted_capabilities: Arc::new(Mutex::new(granted.into_iter().collect())),
        }
    }

    /// Update the granted-capability set (called when consent changes).
    pub async fn update_capabilities(&self, granted: impl IntoIterator<Item = String>) {
        let mut guard = self.granted_capabilities.lock().await;
        *guard = granted.into_iter().collect();
    }

    /// Returns true iff the given capability is currently granted.
    pub async fn has_capability(&self, capability: &str) -> bool {
        self.granted_capabilities.lock().await.contains(capability)
    }

    /// Dispatch a parsed directive. In real deployments this would forward to
    /// the OpenClaw runtime; here we keep the implementation minimal and pure
    /// so the integration shape remains testable.
    pub async fn handle_command(
        &self,
        tool: OpenClawTool,
        argument: &str,
    ) -> Result<(String, Sentiment), String> {
        let cap = tool.required_capability();
        if !self.has_capability(cap).await {
            return Err(format!(
                "openclaw: capability `{cap}` not granted — install via the Agent Marketplace and approve in the consent dialog"
            ));
        }
        if argument.is_empty() {
            return Err(format!(
                "openclaw: tool `{}` requires an argument",
                match tool {
                    OpenClawTool::Read => "read",
                    OpenClawTool::Fetch => "fetch",
                    OpenClawTool::Chat => "chat",
                }
            ));
        }
        // Demo-grade implementations. Real bridges would JSON-RPC out to the
        // OpenClaw runtime here — the boundary is intentional so that future
        // changes only need to replace the body of this match arm.
        let response = match tool {
            OpenClawTool::Read => format!(
                "[openclaw/read] would read `{argument}` (capability granted; real bridge sends JSON-RPC `fs.read` to OpenClaw runtime)"
            ),
            OpenClawTool::Fetch => format!(
                "[openclaw/fetch] would fetch `{argument}` (capability granted; real bridge sends JSON-RPC `net.fetch` to OpenClaw runtime)"
            ),
            OpenClawTool::Chat => format!(
                "[openclaw/chat] forwarded prompt to the OpenClaw chat tool: {argument}"
            ),
        };
        Ok((response, Sentiment::Happy))
    }
}

#[async_trait]
impl AgentProvider for OpenClawAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "OpenClaw"
    }

    async fn respond(&self, message: &str) -> (String, Sentiment) {
        match parse(message) {
            ParsedMessage::Chat(text) => (
                format!(
                    "OpenClaw bridge ready. Type `/openclaw read <path>`, `/openclaw fetch <url>`, or `/openclaw chat <prompt>`. (You said: {text})"
                ),
                Sentiment::Neutral,
            ),
            ParsedMessage::Directive(tool, arg) => match self.handle_command(tool, arg).await {
                Ok((text, sentiment)) => (text, sentiment),
                Err(e) => (e, Sentiment::Sad),
            },
            ParsedMessage::UnknownDirective(name) => (
                format!(
                    "openclaw: unknown tool `{name}` — supported tools: read, fetch, chat"
                ),
                Sentiment::Sad,
            ),
        }
    }

    async fn health_check(&self) -> bool {
        // The in-process bridge is always healthy. Real deployments would ping
        // the OpenClaw runtime here.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_chat_returns_chat_variant() {
        assert_eq!(parse("hello"), ParsedMessage::Chat("hello"));
    }

    #[test]
    fn parse_directive_returns_tool_and_arg() {
        assert_eq!(
            parse("/openclaw read README.md"),
            ParsedMessage::Directive(OpenClawTool::Read, "README.md"),
        );
        assert_eq!(
            parse("/openclaw fetch https://example.com"),
            ParsedMessage::Directive(OpenClawTool::Fetch, "https://example.com"),
        );
        assert_eq!(
            parse("/openclaw chat tell me a joke"),
            ParsedMessage::Directive(OpenClawTool::Chat, "tell me a joke"),
        );
    }

    #[test]
    fn parse_unknown_directive_returns_unknown_variant() {
        assert_eq!(
            parse("/openclaw delete /etc/passwd"),
            ParsedMessage::UnknownDirective("delete"),
        );
    }

    #[test]
    fn parse_trims_leading_and_trailing_whitespace() {
        assert_eq!(
            parse("  /openclaw read foo.txt  "),
            ParsedMessage::Directive(OpenClawTool::Read, "foo.txt"),
        );
    }

    #[tokio::test]
    async fn directive_without_capability_is_rejected() {
        let bridge = OpenClawAgent::new(std::iter::empty());
        let err = bridge
            .handle_command(OpenClawTool::Read, "Cargo.toml")
            .await
            .unwrap_err();
        assert!(err.contains("file_read"));
        assert!(err.contains("not granted"));
    }

    #[tokio::test]
    async fn directive_without_argument_is_rejected() {
        let bridge = OpenClawAgent::new(["file_read".to_string()]);
        let err = bridge
            .handle_command(OpenClawTool::Read, "")
            .await
            .unwrap_err();
        assert!(err.contains("requires an argument"));
    }

    #[tokio::test]
    async fn directive_with_capability_returns_response_and_happy_sentiment() {
        let bridge = OpenClawAgent::new(["file_read".to_string()]);
        let (text, sentiment) = bridge
            .handle_command(OpenClawTool::Read, "Cargo.toml")
            .await
            .unwrap();
        assert!(text.contains("Cargo.toml"));
        assert!(text.contains("openclaw/read"));
        assert_eq!(sentiment, Sentiment::Happy);
    }

    #[tokio::test]
    async fn fetch_directive_requires_network_capability() {
        let bridge = OpenClawAgent::new(["file_read".to_string()]);
        let err = bridge
            .handle_command(OpenClawTool::Fetch, "https://example.com")
            .await
            .unwrap_err();
        assert!(err.contains("network"));
    }

    #[tokio::test]
    async fn update_capabilities_swaps_grant_set() {
        let bridge = OpenClawAgent::new(std::iter::empty());
        assert!(!bridge.has_capability("network").await);
        bridge.update_capabilities(["network".to_string()]).await;
        assert!(bridge.has_capability("network").await);
    }

    #[tokio::test]
    async fn respond_chat_returns_help_text_and_neutral_sentiment() {
        let bridge = OpenClawAgent::new(std::iter::empty());
        let (text, sentiment) = bridge.respond("hello").await;
        assert!(text.contains("OpenClaw bridge ready"));
        assert!(text.contains("/openclaw read"));
        assert_eq!(sentiment, Sentiment::Neutral);
    }

    #[tokio::test]
    async fn respond_directive_dispatches_to_handler() {
        let bridge = OpenClawAgent::new(["file_read".to_string()]);
        let (text, sentiment) = bridge.respond("/openclaw read notes.md").await;
        assert!(text.contains("notes.md"));
        assert_eq!(sentiment, Sentiment::Happy);
    }

    #[tokio::test]
    async fn respond_unknown_directive_returns_error_and_sad_sentiment() {
        let bridge = OpenClawAgent::new(std::iter::empty());
        let (text, sentiment) = bridge.respond("/openclaw nuke").await;
        assert!(text.contains("unknown tool"));
        assert!(text.contains("nuke"));
        assert_eq!(sentiment, Sentiment::Sad);
    }

    #[test]
    fn openclaw_tool_required_capability_matches_manifest() {
        // These must stay in sync with `registry_server::catalog::all_entries`
        // for openclaw-bridge. If the manifest grows new capabilities, add a
        // matching `OpenClawTool` variant.
        assert_eq!(OpenClawTool::Read.required_capability(), "file_read");
        assert_eq!(OpenClawTool::Fetch.required_capability(), "network");
        assert_eq!(OpenClawTool::Chat.required_capability(), "chat");
    }

    #[tokio::test]
    async fn agent_provider_metadata() {
        let bridge = OpenClawAgent::new(std::iter::empty());
        assert_eq!(bridge.id(), "openclaw-bridge");
        assert_eq!(bridge.name(), "OpenClaw");
        assert!(bridge.health_check().await);
    }
}
