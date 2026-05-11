//! Rolling-window summarization hook for coding sessions (chunk 47.4).
//!
//! This hook trims old context once prompt-token usage crosses a threshold.
//! Older messages are marked `exclude_from_context` and replaced with a
//! synthetic summary message.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::runtime_hooks::{
    AgentHook, AgentMessage, AgentState, BeforeModelNext, ModelRequest, RunContext,
};

pub const DEFAULT_SUMMARIZATION_THRESHOLD: u32 = 100_000;
const SHARED_SUMMARIZATION_CONFIG: &str = "shared/coding_summarization.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummarizationSettings {
    pub threshold: u32,
}

impl Default for SummarizationSettings {
    fn default() -> Self {
        Self {
            threshold: DEFAULT_SUMMARIZATION_THRESHOLD,
        }
    }
}

pub fn resolve_summarization_threshold(data_dir: &Path, session_override: Option<u32>) -> u32 {
    session_override
        .or_else(|| load_shared_summarization_settings(data_dir).map(|s| s.threshold))
        .unwrap_or(DEFAULT_SUMMARIZATION_THRESHOLD)
}

pub fn load_shared_summarization_settings(data_dir: &Path) -> Option<SummarizationSettings> {
    let path = data_dir.join(SHARED_SUMMARIZATION_CONFIG);
    let raw = fs::read_to_string(path).ok()?;
    let threshold = parse_threshold_from_toml(&raw)?;
    Some(SummarizationSettings { threshold })
}

fn parse_threshold_from_toml(raw: &str) -> Option<u32> {
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if key.trim() != "threshold" {
                continue;
            }
            let value = value.trim().trim_matches('"');
            if let Ok(parsed) = value.parse::<u32>() {
                return Some(parsed);
            }
        }
    }
    None
}

type SummarizeFn = dyn Fn(&[AgentMessage]) -> Result<String, String> + Send + Sync;

pub struct SummarizationHook {
    threshold: u32,
    keep_tail: usize,
    summarize: Arc<SummarizeFn>,
}

impl SummarizationHook {
    pub fn new(threshold: u32, summarize: Arc<SummarizeFn>) -> Self {
        Self {
            threshold,
            keep_tail: 8,
            summarize,
        }
    }

    #[cfg(test)]
    fn with_keep_tail(mut self, keep_tail: usize) -> Self {
        self.keep_tail = keep_tail;
        self
    }
}

impl AgentHook for SummarizationHook {
    fn hook_name(&self) -> &'static str {
        "rolling_window_summarization"
    }

    fn before_model(
        &self,
        ctx: &RunContext,
        state: &mut AgentState,
        request: ModelRequest,
        next: &BeforeModelNext<'_>,
    ) -> Result<ModelRequest, String> {
        if state.usage.last_prompt_tokens < self.threshold {
            return next(ctx, state, request);
        }

        let candidate_indexes: Vec<usize> = state
            .messages
            .iter()
            .enumerate()
            .filter_map(|(i, m)| {
                if m.exclude_from_context || m.kind == "summary" {
                    None
                } else {
                    Some(i)
                }
            })
            .collect();

        if candidate_indexes.len() <= self.keep_tail {
            return next(ctx, state, request);
        }

        let summarize_count = candidate_indexes.len() - self.keep_tail;
        let summarize_indexes = &candidate_indexes[..summarize_count];
        let to_summarize: Vec<AgentMessage> = summarize_indexes
            .iter()
            .map(|idx| state.messages[*idx].clone())
            .collect();

        if to_summarize.is_empty() {
            return next(ctx, state, request);
        }

        let summary = (self.summarize)(&to_summarize)?;
        if summary.trim().is_empty() {
            return next(ctx, state, request);
        }

        for idx in summarize_indexes {
            if let Some(message) = state.messages.get_mut(*idx) {
                message.exclude_from_context = true;
            }
        }
        state.messages.push(AgentMessage::summary(summary));

        let rebuilt = request.with_messages(state.active_messages());
        next(ctx, state, rebuilt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::coding::runtime_hooks::{run_before_model_hooks, AgentHook};

    #[test]
    fn hook_is_noop_below_threshold() {
        let ctx = RunContext::new("s", "agent", PathBuf::from("."));
        let mut state = AgentState::default();
        state.messages = vec![
            AgentMessage::new("user", "u1"),
            AgentMessage::new("assistant", "a1"),
        ];
        state.usage.last_prompt_tokens = 99_999;

        let request = ModelRequest::new(vec!["prompt".to_string()]);
        let hook = SummarizationHook::new(100_000, Arc::new(|_| Ok("should not run".to_string())));
        let hooks: Vec<Arc<dyn AgentHook>> = vec![Arc::new(hook)];

        let rebuilt = run_before_model_hooks(&hooks, &ctx, &mut state, request).unwrap();
        assert_eq!(rebuilt.messages.len(), 1);
        assert_eq!(state.messages.len(), 2);
        assert!(state.messages.iter().all(|m| !m.exclude_from_context));
    }

    #[test]
    fn hook_summarizes_once_above_threshold() {
        let ctx = RunContext::new("s", "agent", PathBuf::from("."));
        let mut state = AgentState::default();
        state.messages = vec![
            AgentMessage::new("user", "u1"),
            AgentMessage::new("assistant", "a1"),
            AgentMessage::new("user", "u2"),
            AgentMessage::new("assistant", "a2"),
            AgentMessage::new("user", "u3"),
        ];
        state.usage.last_prompt_tokens = 120_000;

        let request = ModelRequest::new(vec!["prompt".to_string()]);
        let hook = SummarizationHook::new(
            100_000,
            Arc::new(|msgs| Ok(format!("summary:{}", msgs.len()))),
        )
        .with_keep_tail(2);
        let hooks: Vec<Arc<dyn AgentHook>> = vec![Arc::new(hook)];

        let rebuilt = run_before_model_hooks(&hooks, &ctx, &mut state, request).unwrap();
        assert!(state.messages.iter().any(|m| m.kind == "summary"));
        assert_eq!(
            state
                .messages
                .iter()
                .filter(|m| m.exclude_from_context)
                .count(),
            3
        );
        assert_eq!(
            rebuilt
                .messages
                .iter()
                .filter(|m| m.kind == "summary")
                .count(),
            1
        );
    }

    #[test]
    fn threshold_cascade_prefers_session_then_file_then_default() {
        let root = std::env::temp_dir().join(format!(
            "ts-summarization-{}-{}",
            std::process::id(),
            crate::coding::session_chat::MAX_MESSAGE_BYTES
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("shared")).unwrap();
        fs::write(
            root.join("shared/coding_summarization.toml"),
            "threshold = 77777\n",
        )
        .unwrap();

        assert_eq!(resolve_summarization_threshold(&root, Some(55555)), 55555);
        assert_eq!(resolve_summarization_threshold(&root, None), 77777);

        let empty_root = root.join("missing");
        fs::create_dir_all(&empty_root).unwrap();
        assert_eq!(
            resolve_summarization_threshold(&empty_root, None),
            DEFAULT_SUMMARIZATION_THRESHOLD
        );
    }
}
