//! Self-RAG iterative refinement — Chunk 16.4.
//!
//! Implements the **reflection-token** half of Self-RAG (Asai et al., 2023):
//! the LLM is prompted to emit special tokens grading its own retrieval
//! and generation, and a small state machine decides whether to retrieve
//! again, accept the answer, or give up.
//!
//! ## Reflection token vocabulary
//!
//! The Self-RAG paper defines four token families. We adopt them as
//! XML-style inline tags so they're easy to spot in raw LLM output and
//! survive intact through any markdown rendering:
//!
//! | Tag | Values | Meaning |
//! |---|---|---|
//! | `<Retrieve>` | `YES`, `NO`, `CONTINUE` | Should we retrieve more passages? |
//! | `<Relevant>` | `RELEVANT`, `IRRELEVANT` | Was the previous retrieval useful? |
//! | `<Supported>` | `FULLY`, `PARTIALLY`, `NO` | Is the answer grounded in retrieved evidence? |
//! | `<Useful>` | integer 1–5 | Self-graded usefulness of the answer to the user. |
//!
//! Tags are case-insensitive on the value; the tag name itself is
//! also case-insensitive (`<retrieve>` and `<Retrieve>` both work).
//!
//! ## Iteration loop
//!
//! [`SelfRagController::next_step`] takes the latest LLM response (which
//! should contain reflection tokens) and returns a [`Decision`] for
//! the caller to act on:
//!
//! - [`Decision::Retrieve`] — issue another retrieval round; the
//!   controller has already incremented its iteration counter.
//! - [`Decision::Accept { answer }`] — answer is good enough; return
//!   it to the user (with reflection tokens stripped).
//! - [`Decision::Reject { reason }`] — answer is unsupported and we've
//!   exhausted retries; surface a graceful "I don't know" to the user.
//!
//! The hard cap is configurable via [`SelfRagController::with_max_iterations`]
//! and defaults to **3** per the milestone spec — matching the Self-RAG
//! paper's typical setting and keeping latency bounded.
//!
//! ## What's *not* in this module
//!
//! This module is the pure decision logic. It deliberately does **not**:
//! - Call any LLM (caller passes raw responses in).
//! - Touch the memory store (caller does retrieval).
//! - Build the prompt (caller is responsible for instructing the LLM
//!   to emit reflection tokens — see [`SELF_RAG_SYSTEM_PROMPT`]).
//!
//! That separation lets the controller be 100 % synchronous and 100 %
//! testable without any tokio runtime, mock LLM, or fixture DB.

use serde::{Deserialize, Serialize};

/// Hard cap on retrieval iterations to keep wall-clock bounded.
pub const DEFAULT_MAX_ITERATIONS: u8 = 3;

/// Minimum self-rated `<Useful>` score required to auto-accept an
/// answer when retrieval has been deemed unnecessary or irrelevant.
/// Below this, the controller will keep iterating up to the cap.
pub const MIN_ACCEPTABLE_USEFULNESS: u8 = 3;

/// System-prompt addendum that instructs an LLM to emit reflection
/// tokens in the Self-RAG vocabulary. Caller prepends this to whatever
/// task-specific instructions they already have.
pub const SELF_RAG_SYSTEM_PROMPT: &str = r#"After answering, append four reflection tokens on their own lines, each on a single line, in this exact form:

<Retrieve>YES|NO|CONTINUE</Retrieve>
<Relevant>RELEVANT|IRRELEVANT</Relevant>
<Supported>FULLY|PARTIALLY|NO</Supported>
<Useful>1|2|3|4|5</Useful>

Use `<Retrieve>YES` if you need additional passages to answer correctly. Use `<Retrieve>NO` if your answer is well-supported by what's already in context. Use `<Retrieve>CONTINUE` only if the user is asking a follow-up that genuinely needs new evidence.

Use `<Supported>FULLY` only when every claim in your answer is directly supported by the retrieved passages. Be honest — `<Supported>NO` is better than fabricating a false `FULLY`."#;

// ─── Token enums ────────────────────────────────────────────────────

/// Should we retrieve more passages?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RetrieveToken {
    Yes,
    No,
    Continue,
}

/// Was the previous retrieval round useful?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RelevantToken {
    Relevant,
    Irrelevant,
}

/// Is the answer grounded in retrieved evidence?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SupportedToken {
    Fully,
    Partially,
    No,
}

/// All four reflection tokens parsed out of an LLM response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reflection {
    pub retrieve: Option<RetrieveToken>,
    pub relevant: Option<RelevantToken>,
    pub supported: Option<SupportedToken>,
    /// Self-rated usefulness, 1..=5. `None` when missing or unparseable.
    pub useful: Option<u8>,
}

impl Reflection {
    /// True when **all four** tokens were present and parseable.
    /// Useful for caller-side telemetry — a missing token isn't fatal
    /// (the controller falls back to conservative defaults) but it's
    /// a sign the LLM is drifting from the prompt format.
    pub fn is_complete(&self) -> bool {
        self.retrieve.is_some()
            && self.relevant.is_some()
            && self.supported.is_some()
            && self.useful.is_some()
    }
}

// ─── Parsing ────────────────────────────────────────────────────────

/// Parse all four reflection tokens out of an LLM response. Missing
/// tokens are silently `None` so a chatty model that emits extra prose
/// doesn't crash the controller. Multiple occurrences: **first wins**
/// (defensive — if the LLM repeats itself, take its initial answer).
pub fn parse_reflection(response: &str) -> Reflection {
    Reflection {
        retrieve: parse_tag::<RetrieveToken>(response, "Retrieve"),
        relevant: parse_tag::<RelevantToken>(response, "Relevant"),
        supported: parse_tag::<SupportedToken>(response, "Supported"),
        useful: parse_useful(response),
    }
}

/// Strip every reflection token from `response` so the user-visible
/// answer doesn't contain them. Returns trimmed text.
pub fn strip_reflection_tokens(response: &str) -> String {
    let mut out = String::with_capacity(response.len());
    let mut remaining = response;
    let tag_names = ["Retrieve", "Relevant", "Supported", "Useful"];

    'outer: while !remaining.is_empty() {
        // Find the earliest opening tag of any known name.
        let mut earliest: Option<(usize, &str)> = None;
        for name in &tag_names {
            if let Some(pos) = find_open_tag_ci(remaining, name) {
                if earliest.is_none_or(|(p, _)| pos < p) {
                    earliest = Some((pos, *name));
                }
            }
        }
        let Some((pos, name)) = earliest else {
            out.push_str(remaining);
            break;
        };

        out.push_str(&remaining[..pos]);
        // Skip past the open tag.
        let after_open = pos + name.len() + 2; // "<" + name + ">"
        remaining = &remaining[after_open..];

        // Find the closing tag.
        let close = format!("</{name}>");
        if let Some(close_pos) = find_close_tag_ci(remaining, &close) {
            remaining = &remaining[close_pos + close.len()..];
        } else {
            // Malformed (no close tag) — drop the rest of the line, keep going.
            if let Some(nl) = remaining.find('\n') {
                remaining = &remaining[nl + 1..];
            } else {
                break 'outer;
            }
        }
    }

    // Collapse runs of blank lines that the strip created.
    let mut cleaned = String::with_capacity(out.len());
    let mut prev_blank = false;
    for line in out.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        cleaned.push_str(line);
        cleaned.push('\n');
        prev_blank = is_blank;
    }
    cleaned.trim().to_string()
}

// ─── Internal: tag finder (case-insensitive ASCII only) ─────────────

fn find_open_tag_ci(haystack: &str, name: &str) -> Option<usize> {
    let needle_lower = format!("<{}>", name.to_ascii_lowercase());
    let lower = haystack.to_ascii_lowercase();
    lower.find(&needle_lower)
}

fn find_close_tag_ci(haystack: &str, close: &str) -> Option<usize> {
    let lower_haystack = haystack.to_ascii_lowercase();
    let lower_close = close.to_ascii_lowercase();
    lower_haystack.find(&lower_close)
}

/// Pull the inner text of `<Name>VALUE</Name>` and run it through
/// FromStrCi.
fn parse_tag<T: FromStrCi>(response: &str, tag_name: &str) -> Option<T> {
    let open = find_open_tag_ci(response, tag_name)?;
    let after_open = open + tag_name.len() + 2;
    let rest = response.get(after_open..)?;
    let close_str = format!("</{tag_name}>");
    let close = find_close_tag_ci(rest, &close_str)?;
    let inner = &rest[..close];
    T::from_str_ci(inner.trim())
}

fn parse_useful(response: &str) -> Option<u8> {
    let open = find_open_tag_ci(response, "Useful")?;
    let after_open = open + "Useful".len() + 2;
    let rest = response.get(after_open..)?;
    let close = find_close_tag_ci(rest, "</Useful>")?;
    let inner = rest[..close].trim();
    let n: u8 = inner.parse().ok()?;
    if (1..=5).contains(&n) { Some(n) } else { None }
}

trait FromStrCi: Sized {
    fn from_str_ci(s: &str) -> Option<Self>;
}

impl FromStrCi for RetrieveToken {
    fn from_str_ci(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "YES" => Some(Self::Yes),
            "NO" => Some(Self::No),
            "CONTINUE" => Some(Self::Continue),
            _ => None,
        }
    }
}

impl FromStrCi for RelevantToken {
    fn from_str_ci(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "RELEVANT" => Some(Self::Relevant),
            "IRRELEVANT" => Some(Self::Irrelevant),
            _ => None,
        }
    }
}

impl FromStrCi for SupportedToken {
    fn from_str_ci(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "FULLY" => Some(Self::Fully),
            "PARTIALLY" => Some(Self::Partially),
            "NO" => Some(Self::No),
            _ => None,
        }
    }
}

// ─── Decision state machine ─────────────────────────────────────────

/// The controller's verdict for the current iteration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Run another retrieval round. Caller should fetch new
    /// candidates (typically via a query rewrite or HyDE pass) and
    /// re-prompt the LLM with the augmented context.
    Retrieve,
    /// Answer is good enough — return it to the user. The string is
    /// the response with reflection tokens stripped.
    Accept { answer: String },
    /// Answer cannot be supported and we've exhausted retries. Caller
    /// should surface a graceful refusal rather than a hallucination.
    Reject { reason: RejectReason },
}

/// Why the controller gave up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// Hit the iteration cap without a `<Supported>FULLY` answer.
    MaxIterationsExceeded,
    /// The LLM explicitly said the answer is not supported.
    Unsupported,
}

/// Iteration controller. Stateful — holds the running iteration
/// counter so callers don't have to track it themselves.
#[derive(Debug, Clone)]
pub struct SelfRagController {
    iteration: u8,
    max_iterations: u8,
}

impl Default for SelfRagController {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfRagController {
    pub fn new() -> Self {
        Self {
            iteration: 0,
            max_iterations: DEFAULT_MAX_ITERATIONS,
        }
    }

    /// Override the default cap (3). Capped at 10 to avoid runaway
    /// loops if a caller passes an absurd value. A floor of 1 is
    /// enforced so the controller always runs at least once.
    pub fn with_max_iterations(mut self, max: u8) -> Self {
        self.max_iterations = max.clamp(1, 10);
        self
    }

    /// Current iteration count (0 = nothing run yet).
    pub fn iteration(&self) -> u8 {
        self.iteration
    }

    /// Take the latest LLM response and decide what to do next.
    /// Increments the iteration counter on every call so the cap is
    /// enforced even if the LLM emits malformed reflection tokens.
    pub fn next_step(&mut self, response: &str) -> Decision {
        self.iteration = self.iteration.saturating_add(1);
        let reflection = parse_reflection(response);

        // Hard rule: explicit `<Supported>NO` means we never accept
        // this answer, even at iteration cap. Better refuse than
        // hallucinate.
        if matches!(reflection.supported, Some(SupportedToken::No))
            && self.iteration >= self.max_iterations
        {
            return Decision::Reject {
                reason: RejectReason::Unsupported,
            };
        }

        // At the cap: either accept (if at all supported) or reject.
        if self.iteration >= self.max_iterations {
            return match reflection.supported {
                Some(SupportedToken::Fully) | Some(SupportedToken::Partially) => {
                    Decision::Accept {
                        answer: strip_reflection_tokens(response),
                    }
                }
                _ => Decision::Reject {
                    reason: RejectReason::MaxIterationsExceeded,
                },
            };
        }

        // Below the cap: classic Self-RAG branching.
        match reflection.retrieve {
            // Explicit ask for more retrieval — go round again.
            Some(RetrieveToken::Yes) | Some(RetrieveToken::Continue) => Decision::Retrieve,

            // LLM says no more retrieval needed. Accept iff supported
            // *and* useful enough; otherwise loop once more to give
            // it a chance with refreshed context.
            Some(RetrieveToken::No) => match (reflection.supported, reflection.useful) {
                (Some(SupportedToken::Fully), _) => Decision::Accept {
                    answer: strip_reflection_tokens(response),
                },
                (Some(SupportedToken::Partially), Some(u)) if u >= MIN_ACCEPTABLE_USEFULNESS => {
                    Decision::Accept {
                        answer: strip_reflection_tokens(response),
                    }
                }
                (Some(SupportedToken::No), _) => Decision::Retrieve,
                _ => Decision::Retrieve,
            },

            // No retrieval token at all → assume the LLM is done. Same
            // accept/reject logic as the `No` branch.
            None => match reflection.supported {
                Some(SupportedToken::Fully) => Decision::Accept {
                    answer: strip_reflection_tokens(response),
                },
                _ => Decision::Retrieve,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_response(retrieve: &str, relevant: &str, supported: &str, useful: u8) -> String {
        format!(
            "Here is the answer.\n<Retrieve>{retrieve}</Retrieve>\n<Relevant>{relevant}</Relevant>\n<Supported>{supported}</Supported>\n<Useful>{useful}</Useful>"
        )
    }

    #[test]
    fn parser_handles_complete_response() {
        let r = parse_reflection(&full_response("NO", "RELEVANT", "FULLY", 5));
        assert_eq!(r.retrieve, Some(RetrieveToken::No));
        assert_eq!(r.relevant, Some(RelevantToken::Relevant));
        assert_eq!(r.supported, Some(SupportedToken::Fully));
        assert_eq!(r.useful, Some(5));
        assert!(r.is_complete());
    }

    #[test]
    fn parser_is_case_insensitive() {
        let r = parse_reflection(
            "answer\n<retrieve>yes</retrieve>\n<RELEVANT>relevant</RELEVANT>\n<Supported>fully</Supported>\n<useful>4</useful>",
        );
        assert_eq!(r.retrieve, Some(RetrieveToken::Yes));
        assert_eq!(r.relevant, Some(RelevantToken::Relevant));
        assert_eq!(r.supported, Some(SupportedToken::Fully));
        assert_eq!(r.useful, Some(4));
    }

    #[test]
    fn parser_handles_missing_tokens() {
        let r = parse_reflection("Just an answer with no tags.");
        assert!(r.retrieve.is_none());
        assert!(r.relevant.is_none());
        assert!(r.supported.is_none());
        assert!(r.useful.is_none());
        assert!(!r.is_complete());
    }

    #[test]
    fn parser_rejects_useful_out_of_range() {
        let r = parse_reflection("ans <Useful>0</Useful> <Useful>9</Useful>");
        // First match wins, value 0 is out of range → None
        assert!(r.useful.is_none());
    }

    #[test]
    fn parser_rejects_useful_garbage() {
        let r = parse_reflection("ans <Useful>five</Useful>");
        assert!(r.useful.is_none());
    }

    #[test]
    fn parser_rejects_unknown_enum_values() {
        let r = parse_reflection("ans <Retrieve>MAYBE</Retrieve>");
        assert!(r.retrieve.is_none());
    }

    #[test]
    fn strip_removes_all_tokens() {
        let raw = full_response("NO", "RELEVANT", "FULLY", 5);
        let stripped = strip_reflection_tokens(&raw);
        assert_eq!(stripped, "Here is the answer.");
    }

    #[test]
    fn strip_preserves_text_with_no_tokens() {
        let raw = "Hello\n\nworld.";
        assert_eq!(strip_reflection_tokens(raw), "Hello\n\nworld.");
    }

    #[test]
    fn strip_handles_malformed_tokens() {
        // Open tag with no close — strip should not panic, and should
        // drop only the malformed line.
        let raw = "Answer.\n<Retrieve>YES (oops, no close)\nMore text.";
        let stripped = strip_reflection_tokens(raw);
        assert!(stripped.contains("Answer."));
        assert!(stripped.contains("More text."));
        assert!(!stripped.contains("<Retrieve>"));
    }

    // ── Controller decisions ─────────────────────────────────────

    #[test]
    fn accept_when_fully_supported_and_no_retrieve() {
        let mut c = SelfRagController::new();
        let r = full_response("NO", "RELEVANT", "FULLY", 5);
        match c.next_step(&r) {
            Decision::Accept { answer } => assert!(answer.contains("Here is the answer")),
            other => panic!("expected Accept, got {other:?}"),
        }
        assert_eq!(c.iteration(), 1);
    }

    #[test]
    fn retrieve_when_explicit_yes() {
        let mut c = SelfRagController::new();
        let r = full_response("YES", "IRRELEVANT", "NO", 2);
        assert_eq!(c.next_step(&r), Decision::Retrieve);
    }

    #[test]
    fn retrieve_when_no_token_is_unsupported() {
        let mut c = SelfRagController::new();
        let r = full_response("NO", "RELEVANT", "NO", 4);
        assert_eq!(c.next_step(&r), Decision::Retrieve);
    }

    #[test]
    fn accept_partial_when_useful_enough() {
        let mut c = SelfRagController::new();
        let r = full_response("NO", "RELEVANT", "PARTIALLY", 4);
        assert!(matches!(c.next_step(&r), Decision::Accept { .. }));
    }

    #[test]
    fn retrieve_partial_when_not_useful_enough() {
        let mut c = SelfRagController::new();
        let r = full_response("NO", "RELEVANT", "PARTIALLY", 1);
        assert_eq!(c.next_step(&r), Decision::Retrieve);
    }

    #[test]
    fn reject_at_max_iterations_when_unsupported() {
        let mut c = SelfRagController::new().with_max_iterations(2);
        let bad = full_response("YES", "IRRELEVANT", "NO", 1);
        assert_eq!(c.next_step(&bad), Decision::Retrieve);
        assert_eq!(
            c.next_step(&bad),
            Decision::Reject {
                reason: RejectReason::Unsupported,
            }
        );
    }

    #[test]
    fn accept_at_max_iterations_when_partial() {
        let mut c = SelfRagController::new().with_max_iterations(2);
        let r1 = full_response("YES", "IRRELEVANT", "NO", 1);
        let r2 = full_response("NO", "RELEVANT", "PARTIALLY", 3);
        assert_eq!(c.next_step(&r1), Decision::Retrieve);
        assert!(matches!(c.next_step(&r2), Decision::Accept { .. }));
    }

    #[test]
    fn reject_at_max_iterations_when_no_supported_token() {
        let mut c = SelfRagController::new().with_max_iterations(1);
        // No reflection at all → at the cap → reject for max-iter.
        let r = "Just text, no tokens.";
        assert_eq!(
            c.next_step(r),
            Decision::Reject {
                reason: RejectReason::MaxIterationsExceeded,
            }
        );
    }

    #[test]
    fn iteration_counter_advances() {
        let mut c = SelfRagController::new().with_max_iterations(5);
        for _ in 0..3 {
            let _ = c.next_step("");
        }
        assert_eq!(c.iteration(), 3);
    }

    #[test]
    fn max_iterations_clamped_to_safe_range() {
        let c1 = SelfRagController::new().with_max_iterations(0);
        // We can't read max_iterations directly; check by behaviour —
        // first call should not be at-cap.
        let mut c1 = c1.clone();
        // 0 was clamped up to 1 → first call IS at cap.
        // Empty response → no `Supported` token → reject for max-iter.
        assert_eq!(
            c1.next_step("nothing"),
            Decision::Reject {
                reason: RejectReason::MaxIterationsExceeded,
            }
        );
    }

    #[test]
    fn system_prompt_mentions_all_four_tags() {
        for tag in ["<Retrieve>", "<Relevant>", "<Supported>", "<Useful>"] {
            assert!(
                SELF_RAG_SYSTEM_PROMPT.contains(tag),
                "system prompt missing {tag}"
            );
        }
    }
}
