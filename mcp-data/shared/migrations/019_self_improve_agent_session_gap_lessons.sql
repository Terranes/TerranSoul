-- 019_self_improve_agent_session_gap_lessons.sql
-- Why TerranSoul's self-improve subsystem failed to autonomously capture the
-- 2026-05-10 "manual screenshot QA, not batch scripts" procedural lesson, and
-- what code changes would close the gap.

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'GAP: TerranSoul self-improve has no ingestion path for lessons learned by an EXTERNAL coding agent (Copilot/Claude Code/Codex) operating in the main checkout. coding/conversation_learning.rs only classifies user chat messages as feature/bugfix/improvement -> appends to rules/milestones.md. coding/engine.rs runs the autonomous loop in an isolated worktree against the configured Coding LLM. Neither path observes when an interactive agent in the user-facing chat session discovers a procedural rule (e.g. "do this manually, not with a batch script"). Result: the agent must hand-write SQL migrations into mcp-data/shared/ to durably store the lesson. The user correctly identified this as a self-improve regression, not a normal feature gap.',
  'gap,self-improve,agent-session,conversation-learning,mcp,knowledge-ingestion',
  10, 'analysis', 1746921600000, 'long', 1.0, 'self-improve', 'analytical'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'GAP: TerranSoul self-improve has no ingestion path for lessons learned by an EXTERNAL coding agent%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'PROPOSAL: Close the agent-session lesson gap with three additions. (1) New module src-tauri/src/coding/agent_session_lessons.rs with detect_lesson(message, role, prior_messages) -> Option<LessonChunk> that recognises user-corrective patterns ("you should X instead of Y", "stop doing X", "instead of using a script, do X manually") and agent-authored patterns ("I learned X", "lesson:"). (2) New MCP tool brain_ingest_lesson{content, tags, importance, category} that writes to memories table via the gateway AND appends an INSERT row to mcp-data/shared/memory-seed.sql so the lesson survives memory.db reset/reseed. (3) Extend coding/conversation_learning.rs DetectionReply schema to include category="lesson" branch that routes to brain_ingest_lesson instead of milestones.md. CI hook: when a new mcp-data/shared/migrations/NNN_*.sql is added, lessons-learned.md must be updated in the same PR.',
  'proposal,self-improve,agent-session,brain-ingest-lesson,mcp-tool,roadmap',
  10, 'proposal', 1746921600000, 'long', 1.0, 'self-improve', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'PROPOSAL: Close the agent-session lesson gap with three additions%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'RULE: When an agent learns a procedural lesson during an interactive coding session (the user corrects a workflow, multiple bugs trace to the same anti-pattern, a screenshot QA reveals systemic issues), the agent MUST: (a) write the lesson into mcp-data/shared/memory-seed.sql via a numbered migration under mcp-data/shared/migrations/, (b) update mcp-data/shared/lessons-learned.md with the same lesson, (c) verify retrievability with brain_search before declaring the task complete, and (d) send a visible MCP receipt naming the lesson topic. Hand-written migrations are the current durable path until brain_ingest_lesson MCP tool ships. Skipping (a)+(b) means the lesson lives only in chat and is lost when context is summarised.',
  'rule,self-improve,agent-session,lesson-capture,markdown-not-memory,migration',
  10, 'rule', 1746921600000, 'long', 1.0, 'self-improve', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'RULE: When an agent learns a procedural lesson during an interactive coding session%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Native HTML <select> dropdown contrast in TerranSoul broke on light themes (Corporate, Pastel, Adventurer-light variants) because src/views/ChatView.vue .reasoning-effort-select used non-existent CSS tokens var(--ts-text, #e2e8f0) and var(--ts-bg-base, #0f172a). --ts-text was never defined; the fallback dark hex always won, producing light-on-light text on light themes. Fix: use var(--ts-text-primary) and var(--ts-bg-surface) which are defined in every html[data-theme=*] block. Also add color-scheme: inherit + accent-color: var(--ts-accent) so the native popup follows the active theme. Audit rule: any custom token reference must match a definition that exists in every theme block of src/style.css.',
  'lesson,frontend,themes,css-tokens,contrast,native-select,accessibility',
  10, 'procedure', 1746921600000, 'long', 1.0, 'frontend', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Native HTML <select> dropdown contrast in TerranSoul broke on light themes%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Chat input must be a multi-line auto-grow textarea, never a single-line <input type="text">. Long messages scroll horizontally in a single-line input so the user cannot see what they typed; on small screens the issue is amplified because the visible window is narrow. Fix in src/components/ChatInput.vue: replace input with textarea rows="1", set CSS resize:none, line-height:1.4, max-height:calc(1.4em*6 + 18px), overflow-y:auto, and call autoResize() on @input that sets el.style.height=auto then min(scrollHeight, lineHeight*MAX_ROWS+padding). Submit on Enter without Shift; Shift+Enter inserts a newline. Reset textarea height when inputText clears (use a watcher that calls autoResize via nextTick).',
  'lesson,frontend,chat-input,textarea,auto-grow,responsive,ux',
  10, 'procedure', 1746921600000, 'long', 1.0, 'frontend', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Chat input must be a multi-line auto-grow textarea%'
);
