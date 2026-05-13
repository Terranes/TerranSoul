//! Runtime hook chain for coding workflows.
//!
//! This module is the reusable hook skeleton for the agent runtime.
//! It is intentionally small and generic: hooks can observe or wrap the
//! model-request path, the tool-call path, and streaming chunks without
//! hard-coding branching logic into the engine.
//!
//! # Before-model rebuild contract
//!
//! Hooks that mutate `AgentState.messages` must return a rebuilt
//! `ModelRequest` snapshot. Mutating state alone is not enough because the
//! model only sees the request object passed into the call.
//!
//! ```
//! use std::path::PathBuf;
//! use std::sync::Arc;
//! use terransoul_lib::coding::runtime_hooks::{run_before_model_hooks, AgentHook, AgentMessage, AgentState, ModelRequest, RunContext};
//!
//! struct InjectHook;
//! impl AgentHook for InjectHook {
//!     fn hook_name(&self) -> &'static str { "inject" }
//!     fn before_model(
//!         &self,
//!         _ctx: &RunContext,
//!         state: &mut AgentState,
//!         request: ModelRequest,
//!         next: &terransoul_lib::coding::runtime_hooks::BeforeModelNext<'_>,
//!     ) -> Result<ModelRequest, String> {
//!         state.messages.push(AgentMessage::new("system", "extra context"));
//!         next(_ctx, state, request.with_messages(state.messages.clone()))
//!     }
//! }
//!
//! let ctx = RunContext::new("session-1", "agent-1", PathBuf::from("."));
//! let mut state = AgentState::default();
//! let request = ModelRequest::new(vec!["user prompt".to_string()]);
//! let rebuilt = run_before_model_hooks(&[Arc::new(InjectHook)], &ctx, &mut state, request).unwrap();
//! assert!(rebuilt.messages.iter().any(|m| m.content == "extra context"));
//! ```

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::Arc;

use crate::coding::offload::maybe_offload_tool_result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunContext {
    pub session_id: String,
    pub agent_name: String,
    pub worktree: PathBuf,
}

impl RunContext {
    pub fn new(
        session_id: impl Into<String>,
        agent_name: impl Into<String>,
        worktree: PathBuf,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            agent_name: agent_name.into(),
            worktree,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentUsage {
    pub last_prompt_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
    pub kind: String,
    pub exclude_from_context: bool,
}

impl AgentMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            kind: String::new(),
            exclude_from_context: false,
        }
    }

    pub fn summary(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            kind: "summary".to_string(),
            exclude_from_context: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentState {
    pub messages: Vec<AgentMessage>,
    pub metadata: HashMap<String, String>,
    pub usage: AgentUsage,
}

impl AgentState {
    pub fn active_messages(&self) -> Vec<AgentMessage> {
        self.messages
            .iter()
            .filter(|m| !m.exclude_from_context)
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRequest {
    pub messages: Vec<AgentMessage>,
    pub metadata: HashMap<String, String>,
}

impl ModelRequest {
    pub fn new(messages: Vec<String>) -> Self {
        let messages = messages
            .into_iter()
            .map(|content| AgentMessage::new("user", content))
            .collect();
        Self {
            messages,
            metadata: HashMap::new(),
        }
    }

    pub fn with_messages(mut self, messages: Vec<AgentMessage>) -> Self {
        self.messages = messages;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl ToolCall {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments: arguments.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallResult {
    pub content: String,
}

impl ToolCallResult {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

pub type BeforeModelNext<'a> =
    dyn Fn(&RunContext, &mut AgentState, ModelRequest) -> Result<ModelRequest, String> + 'a;
pub type AfterModelNext<'a> =
    dyn Fn(&RunContext, &mut AgentState, ModelRequest) -> Result<ModelRequest, String> + 'a;
pub type ToolCallNext<'a> =
    dyn Fn(&RunContext, &mut AgentState, &ToolCall) -> Result<ToolCallResult, String> + 'a;

/// Hook interface for TerranSoul's coding runtime.
pub trait AgentHook: Send + Sync {
    fn hook_name(&self) -> &'static str {
        "agent_hook"
    }

    fn before_model(
        &self,
        ctx: &RunContext,
        state: &mut AgentState,
        request: ModelRequest,
        next: &BeforeModelNext<'_>,
    ) -> Result<ModelRequest, String> {
        next(ctx, state, request)
    }

    fn after_model(
        &self,
        ctx: &RunContext,
        state: &mut AgentState,
        response: ModelRequest,
        next: &AfterModelNext<'_>,
    ) -> Result<ModelRequest, String> {
        next(ctx, state, response)
    }

    fn wrap_tool_call(
        &self,
        ctx: &RunContext,
        state: &mut AgentState,
        call: &ToolCall,
        next: &ToolCallNext<'_>,
    ) -> Result<ToolCallResult, String> {
        next(ctx, state, call)
    }

    fn on_chunk(&self, _ctx: &RunContext, _state: &mut AgentState, _chunk: &str) {}
}

#[derive(Debug, Clone, Default)]
pub struct OffloadHook;

impl AgentHook for OffloadHook {
    fn hook_name(&self) -> &'static str {
        "tool_result_offload"
    }

    fn wrap_tool_call(
        &self,
        ctx: &RunContext,
        state: &mut AgentState,
        call: &ToolCall,
        next: &ToolCallNext<'_>,
    ) -> Result<ToolCallResult, String> {
        let result = next(ctx, state, call)?;
        let offloaded = maybe_offload_tool_result(&ctx.worktree, &call.id, result.content);
        Ok(ToolCallResult::new(offloaded))
    }
}

pub fn run_before_model_hooks(
    hooks: &[Arc<dyn AgentHook>],
    ctx: &RunContext,
    state: &mut AgentState,
    request: ModelRequest,
) -> Result<ModelRequest, String> {
    run_before_model_chain(hooks, 0, ctx, state, request)
}

pub fn run_after_model_hooks(
    hooks: &[Arc<dyn AgentHook>],
    ctx: &RunContext,
    state: &mut AgentState,
    response: ModelRequest,
) -> Result<ModelRequest, String> {
    run_after_model_chain(hooks, 0, ctx, state, response)
}

pub fn run_tool_call_hooks(
    hooks: &[Arc<dyn AgentHook>],
    ctx: &RunContext,
    state: &mut AgentState,
    call: &ToolCall,
    executor: &ToolCallNext<'_>,
) -> Result<ToolCallResult, String> {
    run_tool_call_chain(hooks, 0, ctx, state, call, executor)
}

pub fn run_on_chunk_hooks(
    hooks: &[Arc<dyn AgentHook>],
    ctx: &RunContext,
    state: &mut AgentState,
    chunk: &str,
) {
    for hook in hooks {
        let _ = safe_invoke(hook.hook_name(), "on_chunk", || {
            hook.on_chunk(ctx, state, chunk);
            Ok(())
        });
    }
}

fn run_before_model_chain(
    hooks: &[Arc<dyn AgentHook>],
    index: usize,
    ctx: &RunContext,
    state: &mut AgentState,
    request: ModelRequest,
) -> Result<ModelRequest, String> {
    if index >= hooks.len() {
        return Ok(request);
    }
    let hook = Arc::clone(&hooks[index]);
    let next = |ctx: &RunContext, state: &mut AgentState, request: ModelRequest| {
        run_before_model_chain(hooks, index + 1, ctx, state, request)
    };
    safe_invoke(hook.hook_name(), "before_model", || {
        hook.before_model(ctx, state, request, &next)
    })
}

fn run_after_model_chain(
    hooks: &[Arc<dyn AgentHook>],
    index: usize,
    ctx: &RunContext,
    state: &mut AgentState,
    response: ModelRequest,
) -> Result<ModelRequest, String> {
    if index >= hooks.len() {
        return Ok(response);
    }
    let hook = Arc::clone(&hooks[index]);
    let next = |ctx: &RunContext, state: &mut AgentState, response: ModelRequest| {
        run_after_model_chain(hooks, index + 1, ctx, state, response)
    };
    safe_invoke(hook.hook_name(), "after_model", || {
        hook.after_model(ctx, state, response, &next)
    })
}

fn run_tool_call_chain(
    hooks: &[Arc<dyn AgentHook>],
    index: usize,
    ctx: &RunContext,
    state: &mut AgentState,
    call: &ToolCall,
    executor: &ToolCallNext<'_>,
) -> Result<ToolCallResult, String> {
    if index >= hooks.len() {
        return executor(ctx, state, call);
    }
    let hook = Arc::clone(&hooks[index]);
    let next = |ctx: &RunContext, state: &mut AgentState, call: &ToolCall| {
        run_tool_call_chain(hooks, index + 1, ctx, state, call, executor)
    };
    safe_invoke(hook.hook_name(), "wrap_tool_call", || {
        hook.wrap_tool_call(ctx, state, call, &next)
    })
}

fn safe_invoke<T>(
    hook_name: &str,
    phase: &str,
    f: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(_) => {
            eprintln!("warning: hook {hook_name} panicked during {phase}");
            Err(format!("{hook_name} panicked during {phase}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    struct InjectHook;

    impl AgentHook for InjectHook {
        fn hook_name(&self) -> &'static str {
            "inject"
        }

        fn before_model(
            &self,
            ctx: &RunContext,
            state: &mut AgentState,
            request: ModelRequest,
            next: &BeforeModelNext<'_>,
        ) -> Result<ModelRequest, String> {
            state.messages.push(AgentMessage::new(
                "system",
                format!("ctx:{}", ctx.session_id),
            ));
            next(ctx, state, request.with_messages(state.messages.clone()))
        }
    }

    struct PanicHook;

    impl AgentHook for PanicHook {
        fn hook_name(&self) -> &'static str {
            "panic_hook"
        }

        fn wrap_tool_call(
            &self,
            _ctx: &RunContext,
            _state: &mut AgentState,
            _call: &ToolCall,
            _next: &ToolCallNext<'_>,
        ) -> Result<ToolCallResult, String> {
            panic!("boom")
        }
    }

    #[test]
    fn before_model_chain_rebuilds_request() {
        let ctx = RunContext::new("session-1", "agent", PathBuf::from("."));
        let mut state = AgentState::default();
        let request = ModelRequest::new(vec!["user prompt".to_string()]);
        let hooks: Vec<Arc<dyn AgentHook>> = vec![Arc::new(InjectHook)];

        let rebuilt = run_before_model_hooks(&hooks, &ctx, &mut state, request).unwrap();
        assert_eq!(rebuilt.messages[0].content, "ctx:session-1");
        assert_eq!(state.messages[0].content, "ctx:session-1");
    }

    #[test]
    fn panicking_hook_is_caught() {
        let ctx = RunContext::new("session-1", "agent", PathBuf::from("."));
        let mut state = AgentState::default();
        let call = ToolCall::new("call-1", "tool", "{}");
        let hooks: Vec<Arc<dyn AgentHook>> = vec![Arc::new(PanicHook)];

        let err = run_tool_call_hooks(&hooks, &ctx, &mut state, &call, &|_, _, _| {
            Ok(ToolCallResult::new("ok"))
        })
        .unwrap_err();

        assert!(err.contains("panicked"));
    }

    #[test]
    fn offload_hook_spills_large_results() {
        let dir = tempdir().unwrap();
        let ctx = RunContext::new("session-1", "agent", dir.path().to_path_buf());
        let mut state = AgentState::default();
        let call = ToolCall::new("call-1", "tool", "{}");
        let hooks: Vec<Arc<dyn AgentHook>> = vec![Arc::new(OffloadHook)];
        let big = "x".repeat(crate::coding::offload::OFFLOAD_CHAR_THRESHOLD + 1);

        let result = run_tool_call_hooks(&hooks, &ctx, &mut state, &call, &|_, _, _| {
            Ok(ToolCallResult::new(big.clone()))
        })
        .unwrap();

        assert!(result.content.contains("Tool result offloaded"));
        assert!(result.content.contains("Preview (first"));
        assert!(crate::coding::offload::tool_result_path(&ctx.worktree, &call.id).exists());
    }
}
