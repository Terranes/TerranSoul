-- 020_openagentd_audit_lessons.sql
-- Durable lessons distilled from the 2026-05-10 reverse-engineering audit of
-- lthoangg/OpenAgentd (Apache-2.0). Studied via the upstream documents/docs/
-- tree (architecture, agent loop, hooks, tools, teams, summarization,
-- memory) plus the README; DeepWiki was reachable but not yet indexed.
-- Full audit lives in docs/openagentd-audit.md. Mirrored to
-- mcp-data/shared/lessons-learned.md and chunked into rules/milestones.md
-- as Phase 47.
--
-- Clean-room: no source, prompts, asset names, or wire formats are copied.
-- Apache-2.0 attribution is recorded in CREDITS.md.

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Long tool results must spill to disk, not into the conversation. OpenAgentd ToolResultOffloadHook writes any result above ~40 KB / ~10 K tokens to {workspace}/{agent_name}/.tool_results/{tool_call_id}.txt and replaces the in-context value with a head/tail preview plus the file path. Write failure logs a warning and returns the original — execution is never broken by an offload failure. TerranSoul mapping: add the same step in coding::engine run_tool_with_hooks and commands::chat tool-call handling, spilling to <worktree>/.terransoul/tool_results/<id>.txt so the agent can read the full content via its existing file tools.',
  'lesson,openagentd,coding,tool-offload,context-window,worktree,phase-47',
  9, 'lesson', 1746921600000, 'long', 1.0, 'coding-workflow', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Long tool results must spill to disk%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Long shell output must spill, and child processes must die. OpenAgentd shell tool streams output, spills past ~128 KB to .shell_output/<call_id>.txt, returns only the last 200 lines inline plus a reference, and starts subprocesses with start_new_session=True so timeout/cancel can os.killpg() the entire process group (e.g. node under npm run dev). It also appends a <shell_metadata> advisory on timeout suggesting a higher timeout_seconds. TerranSoul mapping: coding::test_runner and coding::processes need a ring-buffer + spill-to-file pattern, process-group kill on cancel/timeout, and a structured timeout advisory.',
  'lesson,openagentd,coding,shell,process-group,timeout,phase-47',
  9, 'lesson', 1746921600000, 'long', 1.0, 'coding-workflow', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Long shell output must spill%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Cancellation must reach in-flight tool calls, not only DAG boundaries. OpenAgentd checks an asyncio.Event interrupt at three points (during LLM streaming, before tool dispatch, during tool execution via asyncio.wait FIRST_COMPLETED racing tool tasks vs the event). Tools that finish in the same tick keep their real result; still-running tools are cancelled and return Cancelled by user. TerranSoul mapping: coding::engine has cancel: Arc<AtomicBool> but only checks between DAG nodes; add a tokio::sync::watch::Sender<bool> per run and tokio::select!{ tool, _ = watch.changed() } around each in-flight task.',
  'lesson,openagentd,coding,interrupt,cancellation,tokio-select,phase-47',
  9, 'lesson', 1746921600000, 'long', 1.0, 'coding-workflow', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Cancellation must reach in-flight tool calls%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Heal orphaned tool calls before the next user turn. OpenAgentd heal_orphaned_tool_calls runs in the same DB transaction as the new user message: it inspects the latest assistant row and inserts a synthetic ToolMessage("Tool execution was interrupted before a result could be recorded.") for every tool_call_id without a reply, anchored to last_assistant.created_at + 1µs * (i+1) so input order stays assistant -> tool -> ... -> user. A second guard in _deserialize_messages drops tool calls with truncated arguments JSON (mid-stream interrupt). Without this, the next OpenAI Responses turn 400s with No tool output found for function call. TerranSoul mapping: add heal-on-load in commands::chat::load_session and coding::session_chat::resume.',
  'lesson,openagentd,coding,heal,orphan,tool-call,resume,phase-47',
  9, 'lesson', 1746921600000, 'long', 1.0, 'coding-workflow', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Heal orphaned tool calls before the next user turn%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Agent loop hooks must use chain-of-responsibility, not branches. OpenAgentd BaseAgentHook exposes before_agent / before_model / wrap_model_call / on_model_delta / after_model / wrap_tool_call / after_agent. wrap_tool_call is a chain (Hook0 -> Hook1 -> ... -> execute_fn), so tool-result offload, OTel, audit, and team-protocol injection are composable units instead of branches in the loop body. All invocations are wrapped in _safe_invoke_hooks() so a buggy hook never crashes the run. TerranSoul mapping: coding/hooks.rs is for git hooks only; introduce a runtime AgentHook trait in coding/runtime_hooks.rs with the same lifecycle, route hook errors through tracing::warn! with the hook name, and migrate the new tool-offload step (lesson above) to be the first hook on the chain.',
  'lesson,openagentd,coding,hooks,chain-of-responsibility,trait,phase-47',
  9, 'lesson', 1746921600000, 'long', 1.0, 'coding-workflow', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Agent loop hooks must use chain-of-responsibility%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: ModelRequest.messages is a frozen tuple; hooks that mutate state must return a new request. OpenAgentd before_model snapshot is built before hooks fire, so SummarizationHook and TeamInboxHook must call request.override(messages=tuple(state.messages_for_llm)) — mutating state.messages alone leaves the LLM seeing stale data. TerranSoul mapping: any future runtime hook that injects context (RAG block, inbox message, summary) must rebuild the outgoing request, not just mutate the in-memory state. Encode this as a doctest on the AgentHook trait.',
  'lesson,openagentd,coding,hooks,immutable-request,gotcha,phase-47',
  9, 'lesson', 1746921600000, 'long', 1.0, 'coding-workflow', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: ModelRequest.messages is a frozen tuple%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Rolling-window summarization needs cross-request token seeding. OpenAgentd SummarizationHook is a pure state transform that fires when state.usage.last_prompt_tokens >= threshold, but that counter resets to 0 on every Agent.run(). The fix lives in the Checkpointer: mark_loaded() scans history for the last assistant extra.usage.input and seed_state() restores it before the loop, so turn N+1 of a multi-HTTP-request session still triggers summarization. Settings cascade per-agent frontmatter -> .openagentd/config/summarization.md -> module defaults. TerranSoul mapping: add a SummarizationHook on the new hook framework and seed last_prompt_tokens from coding::session_chat persisted usage so long resumed coding sessions actually compress.',
  'lesson,openagentd,coding,summarization,token-seeding,context,phase-47',
  8, 'lesson', 1746921600000, 'long', 1.0, 'coding-workflow', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Rolling-window summarization needs cross-request token seeding%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Subscribe-before-read + commit-after-persist prevents replay duplication. OpenAgentd stream_store.attach() registers the subscriber asyncio.Queue BEFORE replaying state to close the producer/consumer gap, and commit_agent_content(session_id, agent_name) drops the just-persisted content[agent], thinking[agent], and tool_calls entries from the replay blob immediately after each successful checkpointer.sync(). Without commit-after-persist, a mid-turn UI refresh between sync and the team-wide mark_done renders the same assistant text twice (once from DB, once from replay). TerranSoul mapping: deferred until we actually have a Tauri replay surface (multi-window pet-mode chat mirror, LAN-MCP-shared web UI); adopt the contract from day one when that work begins.',
  'lesson,openagentd,streaming,replay,sse,subscribe-before-read,deferred',
  7, 'lesson', 1746921600000, 'long', 1.0, 'streaming', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Subscribe-before-read + commit-after-persist%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: A scheduled dream agent turns ephemeral notes into durable topics. OpenAgentd wiki has three tiers — USER.md (always injected), topics/{slug}.md (BM25-searchable on demand via wiki_search), and notes/{date}.md (append-only via the note tool). A cron-driven dream agent reads unprocessed sessions and notes, synthesises new topics, and updates USER.md and INDEX.md. Empty sessions auto-skip so they do not consume a batch slot. TerranSoul mapping: TerranSoul has memory::wiki, memory::brain_memory, and obsidian_export, but no scheduled-promotion loop turning ephemeral chat notes into BM25-searchable wiki topics. Reuse the brain LLM provider and embedding worker. Deferred until the runtime hook framework lands so the dream agent stays composable.',
  'lesson,openagentd,memory,wiki,dream,scheduled,bm25,deferred',
  7, 'lesson', 1746921600000, 'long', 1.0, 'memory', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: A scheduled dream agent turns ephemeral notes%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Sandbox needs a secrets denylist plus a tokenised shell pre-flight. OpenAgentd filesystem sandbox uses a denylist (data/state/cache dirs) plus user-defined globs from sandbox.yaml (e.g. **/.env, **/secrets/**). The same denylist also covers shell commands via a best-effort shlex tokenisation: path-like tokens (containing /, leading ~ or .) are resolved against the workspace and matched. Best-effort only — $VAR, $(...), backticks, and base64 are not evaluated, so OS-level user permissions remain the last line of defence. TerranSoul mapping: TerranSoul coding worktree gives directory isolation; add a first-class secrets_denylist (default **/.env, **/secrets/**, **/*.pem, **/id_rsa*) checked by file tools, plus a tokenised shell pre-flight that rejects path-like tokens resolving into a denied root.',
  'lesson,openagentd,sandbox,secrets,denylist,shlex,shell,phase-47',
  8, 'lesson', 1746921600000, 'long', 1.0, 'security', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Sandbox needs a secrets denylist%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Provider tool schemas must be sanitised per provider. OpenAgentd _resolve_refs() inlines $ref and drops $defs so providers that reject $ref (Gemini, Vertex AI) get flat self-contained schemas. GeminiProviderBase._sanitize_schema() recursively strips discriminator, const, exclusiveMinimum/Maximum, additionalProperties, $schema, $id, $ref, contentEncoding, contentMediaType before the request — Gemini returns 400 INVALID_ARGUMENT otherwise. TerranSoul mapping: when free-mode tool calls flow through Gemini, add brain::providers::sanitize_tool_schema_for_gemini and call it in the free-mode tool-call adapter; otherwise some MCP tools silently 400.',
  'lesson,openagentd,brain,providers,gemini,schema-sanitisation,phase-47',
  8, 'lesson', 1746921600000, 'long', 1.0, 'brain', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Provider tool schemas must be sanitised per provider%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Tool-call SSE is three-phase, and parallel ids must be preserved. OpenAgentd emits tool_call (from the streaming model delta the moment a name appears, no args yet -> spinner card), tool_start (from wrap_tool_call before execution with assembled args), and tool_end (after execution with the result). All three carry the same tool_call_id so the frontend matches them even for parallel calls. Critical: tool_end must use the id registered at tool_call time — some providers mis-index parallel calls (Gemini snapshots the parts array every chunk; both Gemini providers now assign a stream-scoped tool_idx_by_id.setdefault(fc_id, len(tool_idx_by_id)) before building each ToolCallDelta). TerranSoul mapping: verify StreamTagParser preserves the contract for parallel calls and consider adopting the ToolIdResolver pattern.',
  'lesson,openagentd,streaming,tool-call,three-phase,parallel-id',
  7, 'lesson', 1746921600000, 'long', 1.0, 'streaming', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Tool-call SSE is three-phase%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Live config drift detection avoids restart-on-edit. OpenAgentd team members stamp the mtimes of their .md, mcp.json, and every referenced SKILL.md in agent.config_stamp at build time; end-of-turn the wrapper detects drift and the next activation re-parses and swaps the underlying agent in place (model, tools, prompt, MCP). Wrapper, mailbox binding, and session_id are preserved. Parse failures keep the previous agent and re-stamp to avoid looping. TerranSoul mapping: TerranSoul coding skills/personas live as files on disk; add coding::skills::stamp_and_refresh_if_dirty so editing a skill file is reflected on the next ambient-mode wake without restarting the engine.',
  'lesson,openagentd,coding,skills,drift-detection,hot-reload',
  7, 'lesson', 1746921600000, 'long', 1.0, 'coding-workflow', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Live config drift detection avoids restart-on-edit%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Lead-as-translator team protocol with verify-before-claim is the right multi-agent default. OpenAgentd members describe needs in plain language ("I need to write files to disk") rather than registry names ("grant me write"); the lead translates and calls team_manage(member, "add", "tool", "write"). Validation runs before any disk write. Critical robustness: members must read the result of every tool call before claiming success, and after mutating state (file write) the protocol asks for a cheap follow-up read (ls, read) before reporting completion. The lead must sanity-check a member done claim with a cheap read when feasible. TerranSoul mapping: codify verify-before-claim in coding::multi_agent prompts and rules/prompting-rules.md so it is enforced consistently.',
  'lesson,openagentd,coding,multi-agent,verify-before-claim,prompts,phase-47',
  8, 'lesson', 1746921600000, 'long', 1.0, 'coding-workflow', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Lead-as-translator team protocol%'
);
