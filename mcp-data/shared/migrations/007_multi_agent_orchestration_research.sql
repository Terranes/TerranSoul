-- ============================================================================
-- Migration 007 — Multi-agent orchestration research and Windows loader lesson
-- ============================================================================
-- Date: 2026-05-05
-- Trigger: User requested deep reverse-engineering of a public multi-agent
--          workflow dashboard and a full backend/system/UI comparison through
--          2026-05, then asked to fix the Windows DLL root cause and continue.
-- Sources: DeepWiki + upstream public repository/docs for the studied project;
--          public docs for LangGraph, CrewAI, AutoGen, Semantic Kernel,
--          OpenAI Agents SDK, Google ADK, LlamaIndex Workflows, Pydantic AI,
--          Haystack Agents, Agno, Mastra; Microsoft Windows manifest docs.
-- Verdict: partial-adopt patterns, not the external stack or branding.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MULTI-AGENT ORCHESTRATION RESEARCH SOURCE CHECK (2026-05-05): for public GitHub multi-agent projects, first check DeepWiki when reachable, then cross-check upstream repo/docs/license and TerranSoul source before proposing adoption. The 2026-05 survey covered a self-hosted multi-agent dashboard plus LangGraph, CrewAI, AutoGen, Semantic Kernel, OpenAI Agents SDK, Google ADK, LlamaIndex Workflows, Pydantic AI, Haystack Agents, Agno, and Mastra. Credit all sources in CREDITS.md and keep TerranSoul names neutral.',
  'research,multi-agent,reverse-engineering,deepwiki,credits,self-improve',
  9, 'procedure', 1746489600000, 'long', 1.0, 'research', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MULTI-AGENT ORCHESTRATION VERDICT (2026-05-05): partial-adopt. TerranSoul should absorb durable lineage, per-session tool bundles, bounded swarm joins, task queue recovery, quality gates, trace/eval hooks, and dense operations UI patterns. Do not import a Next.js/Node dashboard stack, hosted workflow service, or another memory store. Keep orchestration local-first in Rust/Tauri and reuse TerranSoul memory/RAG, provider policy, MCP caps, and LAN sharing.',
  'multi-agent,orchestration,verdict,partial-adopt,local-first,rust-tauri',
  10, 'fact', 1746489600000, 'long', 1.0, 'architecture', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'TERRANSOUL MULTI-AGENT BASELINE (2026-05-05): current equivalents include coding/multi_agent.rs workflow plans and recurrence, coding/dag_runner.rs dependency layers and parallel execution, coding/task_queue.rs persistent SQLite retry queue, workflows/engine.rs durable SQLite event log, workflows/resilience.rs retry/timeout/circuit-breaker/watchdog, coding/engine.rs self-improve gates, coding/gate_telemetry.rs gate metrics, workflow-plans.ts plan UI store, agent-roster.ts handoff profiles, MCP brain exposure, and LAN MCP brain sharing. Next work is integration, not a rewrite.',
  'multi-agent,terransoul-baseline,coding-workflow,dag-runner,task-queue,workflow-engine',
  9, 'fact', 1746489600000, 'long', 1.0, 'architecture', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MULTI-AGENT BACKEND ADOPTION PLAN (2026-05-05): add durable run lineage with parent/child ids, agent role, model, status, tool-bundle hash, cancellation flag, timestamps, and verdict; emit typed sub-agent events; build session tool bundles from capability policy, provider policy, MCP transport caps, plan role, and approval state; layer a bounded swarm pool over dag_runner with all/any/quorum/best joins; extend task_queue with dead-letter, stalled recovery, quality gate rows, and budget deferral; add trace ids to every LLM/tool/approval/retry/gate event.',
  'multi-agent,backend-plan,lineage,tool-bundles,swarm-runtime,task-queue,tracing',
  10, 'procedure', 1746489600000, 'long', 1.0, 'self-improve', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MULTI-AGENT UI UX ADOPTION PLAN (2026-05-05): build a compact operations workbench, not a landing page. First screen should answer what is running, which agent owns it, what tools it can touch, what is blocked, and what changed. Use active-run rail, agent lanes, backlog/queued/running/blocked/review/failed/done task cards, parent-child run graph, transcript bubbles for delegation/tool/approval/validation/verdict events, repair controls, and LAN peer brain status panels with source host and token state.',
  'multi-agent,ui-ux,operations-workbench,run-graph,agent-lanes,lan-sharing',
  9, 'procedure', 1746489600000, 'long', 1.0, 'ui-ux', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'WINDOWS COMMON CONTROLS DLL LESSON (2026-05-05): Rust lib test harnesses on Windows can fail before Rust code runs with STATUS_ENTRYPOINT_NOT_FOUND when native dependencies import comctl32.dll!TaskDialogIndirect without a Common Controls v6 activation manifest. Root fix: embed one canonical Windows app manifest that declares Microsoft.Windows.Common-Controls version 6.0.0.0, compatibility, UTF-8 code page, DPI, longPathAware, and asInvoker privileges; disable Tauri duplicate default manifest via WindowsAttributes::new_without_app_manifest(). Validate with cargo test, not only cargo check.',
  'windows,manifest,comctl32,TaskDialogIndirect,dll-loader,tauri,testing',
  10, 'procedure', 1746489600000, 'long', 1.0, 'development', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'ANN AND WASM FEATURE-GATING LESSON (2026-05-05): native-heavy dependencies should be opt-in when not required for normal MCP/test flows. TerranSoul default builds now use a pure-Rust linear cosine AnnIndex fallback and a WASM runner stub that returns a clear disabled message. The native-ann feature enables persisted usearch HNSW vectors.usearch for large stores; the wasm-sandbox feature enables Wasmtime plugin execution. README and docs/brain-advanced-design.md must describe default vs feature-enabled behavior together.',
  'memory,ann,usearch,wasmtime,feature-gating,rag,brain-docs,self-improve',
  10, 'procedure', 1746489600000, 'long', 1.0, 'brain', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MULTI-AGENT ORCHESTRATION VERDICT:%'
  AND d.content LIKE 'TERRANSOUL MULTI-AGENT BASELINE:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'informs', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MULTI-AGENT BACKEND ADOPTION PLAN:%'
  AND d.content LIKE 'MULTI-AGENT ORCHESTRATION VERDICT:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'informs', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MULTI-AGENT UI UX ADOPTION PLAN:%'
  AND d.content LIKE 'MULTI-AGENT ORCHESTRATION VERDICT:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MULTI-AGENT ORCHESTRATION RESEARCH SOURCE CHECK:%'
  AND (
       d.content LIKE 'DEEPWIKI REVERSE-ENGINEERING RULE:%'
    OR d.content LIKE 'MCP SELF-LEARNING RULE:%'
    OR d.content LIKE 'DEEP ANALYSIS RULE:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 0.9, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'ANN AND WASM FEATURE-GATING LESSON:%'
  AND (
       d.content LIKE 'RAG pipeline:%'
    OR d.content LIKE 'Vector support:%'
    OR d.content LIKE 'Memory store uses SQLite%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 0.9, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'WINDOWS COMMON CONTROLS DLL LESSON:%'
  AND (
       d.content LIKE 'CI gate command:%'
    OR d.content LIKE 'MCP/app dependency bootstrap rule:%'
    OR d.content LIKE 'Self-improve with MCP mode:%'
  );
