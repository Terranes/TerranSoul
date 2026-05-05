-- Migration 008: Coding workflow research (Roo Code, VoltAgent agent-skills, Sentrux)
-- Lessons learned from reverse-engineering leading AI coding tools and skill ecosystems.

-- ─── Roo Code architecture insights ─────────────────────────────────────────

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'Roo Code uses a Mode system (Code, Architect, Ask, Debug, Custom) where each mode changes the system prompt and available tools. CustomModesManager loads mode-specific rules from .roo/modes/ directory. SYSTEM_PROMPT() dynamically incorporates mode rules, workspace context, available tools, and token budget constraints. Key insight: modes are just prompt-template + tool-filter combinations, not separate codepaths.',
  'roo-code,coding-workflow,modes,system-prompt,architecture',
  8, 'fact', strftime('%s','now') * 1000, 'long', 1.0, 85
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'Roo Code MCP integration: McpServerManager singleton coordinates McpHub instances. McpHub manages server lifecycles (stdio/SSE/streamable-HTTP transports). Configuration lives in ~/.vscode/settings/mcp.json (global) and .roo/mcp.json (project, higher priority). File watchers auto-reload config with 500ms debounce. Tools have alwaysAllow and disabledTools lists for fine-grained permission control.',
  'roo-code,mcp,architecture,configuration,permissions',
  8, 'fact', strftime('%s','now') * 1000, 'long', 1.0, 72
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'Roo Code provider system: buildApiHandler() factory creates provider-specific ApiHandler from ApiConfiguration. 30+ providers supported (OpenRouter, OpenAI, Anthropic, Gemini, Ollama, LM Studio, etc). Each handler implements createMessage(), completePrompt(), getModel(). ProviderSettingsManager validates configs. Supports prompt caching (Anthropic/OpenRouter), reasoning tokens, vision, and function calling.',
  'roo-code,llm-providers,factory-pattern,architecture',
  7, 'fact', strftime('%s','now') * 1000, 'long', 1.0, 68
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'Roo Code Task orchestration: Task class manages execution loops with startTask() → executeLoop() → tool parse → execute → ask/say cycle. CheckpointTracker saves state for rollback. DiffViewProvider shows changes. Tool approval flow: check alwaysAllowMcp → check tool.alwaysAllow → ask user → execute. Reference counting tracks active clients for proper cleanup.',
  'roo-code,task-orchestration,execution-loop,checkpoints',
  7, 'fact', strftime('%s','now') * 1000, 'long', 1.0, 65
);

-- ─── VoltAgent Awesome Agent Skills ecosystem ───────────────────────────────

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'Agent Skills standard format: YAML frontmatter (name, description) + Markdown sections (overview, when-to-use, step-by-step instructions, validation criteria, optional resources). Compatible with 8+ assistants: Copilot (.github/skills/), Claude Code (.claude/skills/), Cursor (.cursor/skills/), Gemini CLI (.gemini/skills/), Codex (.codex/skills/), OpenCode (.opencode/skills/), Windsurf (.windsurf/skills/). Both project-scoped and user-scoped (~/) paths supported.',
  'agent-skills,standard-format,compatibility,copilot,claude,cursor',
  9, 'fact', strftime('%s','now') * 1000, 'long', 1.0, 90
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'Context Engineering skills (muratcankoylan): 8 skills covering context-fundamentals (anatomy, systems), context-degradation (lost-in-middle, poisoning), context-compression (long-running sessions), context-optimization (compaction, masking, caching), multi-agent-patterns (orchestrator, P2P, hierarchical), memory-systems (short/long-term, graph-based), tool-design (architectural reduction), evaluation (agent system frameworks). Directly relevant to TerranSoul RAG/memory architecture.',
  'agent-skills,context-engineering,memory-systems,rag,multi-agent',
  9, 'fact', strftime('%s','now') * 1000, 'long', 1.0, 82
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'Key agent skills relevant to TerranSoul coding workflow: obra/superpowers (brainstorming, writing-plans, executing-plans, dispatching-parallel-agents, test-driven-development, subagent-driven-development, systematic-debugging, root-cause-tracing, using-git-worktrees, finishing-a-development-branch, requesting/receiving-code-review, defense-in-depth). Trail of Bits (22 security skills: audit-context-building, differential-review, static-analysis, variant-analysis, semgrep-rule-creator). Playwright skill for browser automation testing.',
  'agent-skills,development,testing,security,git-workflow,debugging',
  8, 'fact', strftime('%s','now') * 1000, 'long', 1.0, 95
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'VoltAgent ecosystem stats (2026): 180+ curated skills across 16 official teams + 100+ community contributors. Quality bar: community-adopted skills only, public repo with docs, proven real-world usage, max 10-word description. MIT licensed. Decentralized content (skills live in external repos), centralized discovery (README.md as index). Key orgs: Anthropic (15), Trail of Bits (22 security), Vercel (8 React/Next), Cloudflare (7 edge), Hugging Face (8 ML), Sentry (7 code review).',
  'agent-skills,ecosystem,voltagent,statistics,quality-curation',
  7, 'fact', strftime('%s','now') * 1000, 'long', 1.0, 78
);

-- ─── Sentrux architecture insights ──────────────────────────────────────────

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'Sentrux is a structural quality analysis engine — the "missing feedback loop" for AI-assisted development. Sensor/Spec/Actuator model: Scanner produces Quality Signal (0-10000 score from 5 root-cause metrics), Rules Engine (.sentrux/rules.toml) defines good architecture, AI Agent uses signal to correct drift. Key insight: measures actual architecture (dependency graph, complexity, coupling) rather than just text diffs. Prevents structural degradation during high-speed AI code generation.',
  'sentrux,code-quality,architecture-analysis,feedback-loop,quality-gate',
  8, 'fact', strftime('%s','now') * 1000, 'long', 1.0, 88
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'Sentrux MCP server (stdio transport): ToolRegistry with tier-gating (Free/Pro/Team), McpState caches Snapshot+HealthReport+ArchReport for low-latency responses. Tools: scan/rescan (file discovery + dependency graph), health/check_rules (quality signals + violations), evolution/test_gaps (git history + untested complexity), whatif (Pro: predict impact of moves/deletes). Telemetry records mcp_calls for improving tool schemas. Invalidates-evolution flag clears cached data before re-analysis.',
  'sentrux,mcp-server,tool-registry,tier-gating,quality-tools',
  8, 'fact', strftime('%s','now') * 1000, 'long', 1.0, 82
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'Sentrux analysis capabilities for TerranSoul inspiration: Multi-language support (52+ via Tree-sitter), real-time treemap visualization with dependency edges (egui renderer), CLI quality gate (sentrux check) to prevent regressions during AI sessions, evolutionary/git-based metrics (churn rate, change coupling), test gap analysis (identify untested high-complexity areas), what-if simulation (predict refactoring impact). Two crates: sentrux-core (analysis+rendering) and sentrux-bin (CLI+entrypoint).',
  'sentrux,analysis-engine,tree-sitter,quality-gate,git-metrics,test-gaps',
  7, 'fact', strftime('%s','now') * 1000, 'long', 1.0, 85
);

-- ─── Actionable improvements for TerranSoul ─────────────────────────────────

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'TerranSoul coding workflow improvements inspired by research: (1) Add quality-gate MCP tool (like Sentrux check) that runs clippy+tests+vue-tsc before allowing chunk completion. (2) Support agent-skills format (.github/skills/) for portable skill definitions that work across Copilot/Claude/Cursor. (3) Add mode system to MCP (research/implement/review modes with different tool+prompt sets). (4) Add context-compression to session memory (compact long conversations). (5) Add checkpoint/rollback for multi-step tasks.',
  'terransoul,improvements,quality-gate,agent-skills,modes,context-compression',
  9, 'decision', strftime('%s','now') * 1000, 'long', 1.0, 92
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
VALUES (
  'MCP brain config sharing rule: When dev or release mode configures brain/LLM settings, sync the config to mcp-data/shared/brain_config.json so MCP mode inherits the same provider without re-setup. auto_configure_mcp_brain() now rejects free_api (clears stale config), auto-installs Ollama if missing, and only accepts local_ollama/paid_api/local_lm_studio as valid MCP providers. The connecting coding agent IS the LLM for MCP operations that need reasoning.',
  'terransoul,mcp,brain-config,sharing,auto-install,provider-policy',
  9, 'decision', strftime('%s','now') * 1000, 'long', 1.0, 78
);

-- ─── Knowledge graph edges ──────────────────────────────────────────────────

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT m1.id, m2.id, 'informs', 0.9, 'seed', strftime('%s','now') * 1000
FROM memories m1, memories m2
WHERE m1.content LIKE '%Agent Skills standard format%'
  AND m2.content LIKE '%TerranSoul coding workflow improvements%'
  AND m1.id != m2.id;

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT m1.id, m2.id, 'informs', 0.9, 'seed', strftime('%s','now') * 1000
FROM memories m1, memories m2
WHERE m1.content LIKE '%Sentrux is a structural quality%'
  AND m2.content LIKE '%TerranSoul coding workflow improvements%'
  AND m1.id != m2.id;

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT m1.id, m2.id, 'informs', 0.9, 'seed', strftime('%s','now') * 1000
FROM memories m1, memories m2
WHERE m1.content LIKE '%Roo Code uses a Mode system%'
  AND m2.content LIKE '%TerranSoul coding workflow improvements%'
  AND m1.id != m2.id;

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT m1.id, m2.id, 'relates_to', 0.8, 'seed', strftime('%s','now') * 1000
FROM memories m1, memories m2
WHERE m1.content LIKE '%Context Engineering skills%'
  AND m2.content LIKE '%MCP brain config sharing rule%'
  AND m1.id != m2.id;
