//! XML-tag-structured prompt builder for all coding workflows.
//!
//! Applies the ten Anthropic-style prompt-engineering principles uniformly
//! to every coding task TerranSoul runs (self-improve planner, ad-hoc chat
//! coding tasks, conflict resolution, conversation learning):
//!
//! 1. Use XML tags for structure — not markdown, not JSON.
//! 2. Roles read like a job description (skills + tools + priorities).
//! 3. Force the model to think before answering (`<analysis>` before `<output>`).
//! 4. Few-shot examples include reasoning, not only input/output pairs.
//! 5. State explicitly what the model MUST NOT do.
//! 6. Pre-write the assistant's opening words to skip "I'd be happy to…".
//! 7. Define the output shape exactly (sections, length, format).
//! 8. Wrap every supplied document in an indexed `<document>` tag.
//! 9. Build error-handling into the prompt (missing data, conflicts,
//!    "say I don't know").
//! 10. Treat prompts as versioned product code (constants, tests, schemas).
//!
//! See `rules/prompting-rules.md` for the human-facing version of these
//! rules.

use crate::brain::openai_client::OpenAiMessage;

/// Schema version of the prompt builder. Bump when changing the role
/// description or output contract so callers can pin a known revision.
pub const PROMPT_SCHEMA_VERSION: &str = "v1";

/// A single supplementary document to inject into the prompt.
#[derive(Debug, Clone)]
pub struct DocSnippet {
    /// Short label shown to the model (e.g. `"rules/prompting-rules.md"`).
    pub label: String,
    /// Plain-text contents of the snippet. Will be truncated by the
    /// builder if it exceeds [`MAX_DOC_CHARS`].
    pub body: String,
}

/// Output-shape requirement for the model's reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputShape {
    /// Numbered markdown list of at most `max_steps` steps. The reply is
    /// expected inside a `<plan>` tag.
    NumberedPlan { max_steps: u8 },
    /// Strict JSON object matching the supplied schema description.
    /// The reply is expected inside a `<json>` tag.
    StrictJson { schema_description: String },
    /// Bare file contents (no markdown fence, no commentary). Reply
    /// inside a `<file>` tag.
    BareFileContents,
    /// Free-form prose — no enclosing tag required. Use sparingly; the
    /// other shapes are preferred for parser stability.
    Prose,
}

/// All inputs needed to assemble a coding-task prompt.
#[derive(Debug, Clone)]
pub struct CodingPrompt {
    /// Job-description-style role (rule 2). Should name skills, tools,
    /// and concrete priorities — never `"You are an expert"`.
    pub role: String,
    /// Concrete, bounded task statement.
    pub task: String,
    /// Things the model MUST NOT do (rule 5). Each entry becomes one
    /// `<dont>…</dont>` line.
    pub negative_constraints: Vec<String>,
    /// Optional reference documents that the model may cite by index
    /// (rule 8). Each becomes `<document index="N" label="…">…</document>`.
    pub documents: Vec<DocSnippet>,
    /// Output contract (rule 7).
    pub output: OutputShape,
    /// Optional one-shot worked example (rule 4) showing input + reasoning
    /// + output. Already-formatted XML; the builder inserts it verbatim
    ///   inside `<example>…</example>`.
    pub example: Option<String>,
    /// Pre-written opening for the assistant reply (rule 6). When
    /// present, the builder appends an `assistant` message containing
    /// this string so the model continues from there.
    pub assistant_prefill: Option<String>,
    /// Recoverable error guidance (rule 9). Each entry becomes one
    /// `<on_error>…</on_error>` line.
    pub error_handling: Vec<String>,
}

/// Maximum characters per supplied document before truncation.
pub const MAX_DOC_CHARS: usize = 6_000;

impl CodingPrompt {
    /// Build the OpenAI-compatible message list for this prompt.
    ///
    /// Layout (rules 1, 3, 7, 8, 9, 10):
    /// ```text
    /// <system>
    ///   <schema_version>v1</schema_version>
    ///   <role>…job description…</role>
    ///   <constraints>
    ///     <dont>…</dont>
    ///   </constraints>
    ///   <error_handling>
    ///     <on_error>…</on_error>
    ///   </error_handling>
    ///   <thinking_protocol>
    ///     Before answering, write your analysis inside <analysis>…</analysis>.
    ///     Then write the final answer inside the required output tag.
    ///   </thinking_protocol>
    ///   <output_contract>…</output_contract>
    /// </system>
    /// <user>
    ///   <task>…</task>
    ///   <documents>
    ///     <document index="1" label="…">…</document>
    ///   </documents>
    ///   <example>…</example>   (optional)
    /// </user>
    /// (optional <assistant>prefill</assistant>)
    /// ```
    pub fn build(&self) -> Vec<OpenAiMessage> {
        let mut system = String::new();
        system.push_str("<system>\n");
        system.push_str(&format!(
            "  <schema_version>{}</schema_version>\n",
            PROMPT_SCHEMA_VERSION
        ));
        system.push_str("  <role>\n");
        system.push_str(&indent(self.role.trim(), 4));
        system.push_str("\n  </role>\n");

        if !self.negative_constraints.is_empty() {
            system.push_str("  <constraints>\n");
            for c in &self.negative_constraints {
                system.push_str(&format!("    <dont>{}</dont>\n", escape_xml(c.trim())));
            }
            system.push_str("  </constraints>\n");
        }

        if !self.error_handling.is_empty() {
            system.push_str("  <error_handling>\n");
            for e in &self.error_handling {
                system.push_str(&format!(
                    "    <on_error>{}</on_error>\n",
                    escape_xml(e.trim())
                ));
            }
            system.push_str("  </error_handling>\n");
        }

        // Rule 3 — separate thinking from output.
        system.push_str(
            "  <thinking_protocol>\n    \
             Before producing the final answer, write your reasoning inside \
             <analysis>…</analysis>. Then produce the final answer inside \
             the required output tag described in <output_contract>. Never \
             interleave analysis and output.\n  \
             </thinking_protocol>\n",
        );

        // Rule 7 — exact output contract.
        system.push_str("  <output_contract>\n");
        system.push_str(&indent(&self.output_contract_text(), 4));
        system.push_str("\n  </output_contract>\n");

        system.push_str("</system>");

        // ── user message ─────────────────────────────────────────────
        let mut user = String::new();
        user.push_str("<task>\n");
        user.push_str(&indent(self.task.trim(), 2));
        user.push_str("\n</task>");

        if !self.documents.is_empty() {
            user.push_str("\n<documents>\n");
            for (i, d) in self.documents.iter().enumerate() {
                let truncated = truncate(&d.body, MAX_DOC_CHARS);
                user.push_str(&format!(
                    "  <document index=\"{}\" label=\"{}\">\n",
                    i + 1,
                    escape_xml_attr(&d.label),
                ));
                user.push_str(&indent(&truncated, 4));
                user.push_str("\n  </document>\n");
            }
            user.push_str("</documents>");
        }

        if let Some(ex) = &self.example {
            user.push_str("\n<example>\n");
            user.push_str(&indent(ex.trim(), 2));
            user.push_str("\n</example>");
        }

        let mut msgs = vec![
            OpenAiMessage {
                role: "system".to_string(),
                content: system,
            },
            OpenAiMessage {
                role: "user".to_string(),
                content: user,
            },
        ];

        // Rule 6 — pre-write the assistant's opening if requested.
        if let Some(prefill) = &self.assistant_prefill {
            msgs.push(OpenAiMessage {
                role: "assistant".to_string(),
                content: prefill.clone(),
            });
        }

        msgs
    }

    fn output_contract_text(&self) -> String {
        match &self.output {
            OutputShape::NumberedPlan { max_steps } => format!(
                "Reply with one <plan> tag containing a numbered markdown list of at most {} steps. \
                 Each step is one line: \"N. <action>\". No prose outside the tag.",
                max_steps
            ),
            OutputShape::StrictJson { schema_description } => format!(
                "Reply with exactly one <json> tag containing a single JSON object. Schema: {}. \
                 No prose outside the tag, no markdown fences, no trailing commas.",
                schema_description
            ),
            OutputShape::BareFileContents => {
                "Reply with exactly one <file> tag containing the bare file contents. \
                 No markdown fences, no commentary, no conflict markers."
                    .to_string()
            }
            OutputShape::Prose => "Reply with prose only.".to_string(),
        }
    }
}

// ── helpers ──────────────────────────────────────────────────────────

fn indent(text: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    text.lines()
        .map(|l| format!("{}{}", pad, l))
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_xml_attr(s: &str) -> String {
    escape_xml(s).replace('"', "&quot;")
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let head: String = s.chars().take(max_chars).collect();
    format!("{head}\n… [truncated to {max_chars} chars]")
}

/// Extract the contents of a single XML tag from the model's reply.
///
/// Returns the body of the first `<tag>…</tag>` pair found, or `None` if
/// the tag is missing or unbalanced. Useful for parsing the structured
/// replies the builder asks the model to produce.
pub fn extract_tag<'a>(reply: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = reply.find(&open)? + open.len();
    let end = reply[start..].find(&close)? + start;
    Some(reply[start..end].trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_prompt() -> CodingPrompt {
        CodingPrompt {
            role: "Senior backend engineer specialised in Rust + Tauri 2.x. \
                   Tools: cargo, clippy, rustfmt. Priorities: correctness > \
                   readability > brevity."
                .to_string(),
            task: "Add a function that returns the sum of two integers.".to_string(),
            negative_constraints: vec![
                "Do not include unsafe blocks.".to_string(),
                "Do not modify Cargo.toml.".to_string(),
            ],
            documents: vec![DocSnippet {
                label: "rules/coding-standards.md".to_string(),
                body: "All public Rust functions must include a doc comment.".to_string(),
            }],
            output: OutputShape::NumberedPlan { max_steps: 5 },
            example: None,
            assistant_prefill: Some("<analysis>".to_string()),
            error_handling: vec![
                "If the task is ambiguous, ask one clarifying question and stop.".to_string(),
            ],
        }
    }

    #[test]
    fn build_emits_three_messages_when_prefilled() {
        let msgs = sample_prompt().build();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].role, "system");
        assert_eq!(msgs[1].role, "user");
        assert_eq!(msgs[2].role, "assistant");
        assert_eq!(msgs[2].content, "<analysis>");
    }

    #[test]
    fn build_omits_assistant_when_no_prefill() {
        let mut p = sample_prompt();
        p.assistant_prefill = None;
        let msgs = p.build();
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn system_includes_schema_version_and_role() {
        let msgs = sample_prompt().build();
        let sys = &msgs[0].content;
        assert!(sys.contains("<schema_version>v1</schema_version>"));
        assert!(sys.contains("<role>"));
        assert!(sys.contains("Senior backend engineer"));
    }

    #[test]
    fn system_includes_constraints_and_error_handling() {
        let msgs = sample_prompt().build();
        let sys = &msgs[0].content;
        assert!(sys.contains("<dont>Do not include unsafe blocks.</dont>"));
        assert!(sys.contains("<dont>Do not modify Cargo.toml.</dont>"));
        assert!(sys.contains("<on_error>"));
    }

    #[test]
    fn system_includes_thinking_protocol() {
        let msgs = sample_prompt().build();
        let sys = &msgs[0].content;
        assert!(sys.contains("<thinking_protocol>"));
        assert!(sys.contains("<analysis>"));
    }

    #[test]
    fn user_wraps_documents_with_indices() {
        let msgs = sample_prompt().build();
        let user = &msgs[1].content;
        assert!(user.contains("<document index=\"1\" label=\"rules/coding-standards.md\">"));
        assert!(user.contains("All public Rust functions"));
    }

    #[test]
    fn output_contract_for_plan_mentions_max_steps() {
        let msgs = sample_prompt().build();
        let sys = &msgs[0].content;
        assert!(sys.contains("at most 5 steps"));
        assert!(sys.contains("<plan>"));
    }

    #[test]
    fn output_contract_for_strict_json_includes_schema() {
        let mut p = sample_prompt();
        p.output = OutputShape::StrictJson {
            schema_description: "{\"ok\": bool}".to_string(),
        };
        let msgs = p.build();
        assert!(msgs[0].content.contains("{\"ok\": bool}"));
        assert!(msgs[0].content.contains("<json>"));
    }

    #[test]
    fn output_contract_for_bare_file_forbids_fences() {
        let mut p = sample_prompt();
        p.output = OutputShape::BareFileContents;
        let msgs = p.build();
        assert!(msgs[0].content.contains("No markdown fences"));
        assert!(msgs[0].content.contains("<file>"));
    }

    #[test]
    fn xml_metacharacters_are_escaped_in_constraints() {
        let mut p = sample_prompt();
        p.negative_constraints = vec!["Do not output <script> tags.".to_string()];
        let msgs = p.build();
        assert!(msgs[0].content.contains("&lt;script&gt;"));
    }

    #[test]
    fn long_documents_are_truncated() {
        let mut p = sample_prompt();
        let huge = "x".repeat(MAX_DOC_CHARS + 100);
        p.documents = vec![DocSnippet {
            label: "huge.md".to_string(),
            body: huge,
        }];
        let msgs = p.build();
        assert!(msgs[1].content.contains("[truncated to"));
    }

    #[test]
    fn extract_tag_reads_first_match() {
        let reply = "noise <plan>\n1. step one\n</plan> trailing";
        assert_eq!(extract_tag(reply, "plan"), Some("1. step one"));
    }

    #[test]
    fn extract_tag_returns_none_when_missing() {
        assert!(extract_tag("nothing here", "plan").is_none());
    }

    #[test]
    fn extract_tag_returns_none_when_unclosed() {
        assert!(extract_tag("<plan> oops", "plan").is_none());
    }

    #[test]
    fn example_is_wrapped_in_example_tag() {
        let mut p = sample_prompt();
        p.example = Some("<analysis>test</analysis>\n<plan>1. do it</plan>".to_string());
        let msgs = p.build();
        assert!(msgs[1].content.contains("<example>"));
        assert!(msgs[1].content.contains("<plan>1. do it</plan>"));
    }
}
