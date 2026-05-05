-- ============================================================================
-- Migration 002 — brain-os-plugin reverse-engineering + Deep-Analysis Rule
-- ============================================================================
-- Date: 2026-05-05
-- Trigger: User request to reverse-engineer https://github.com/sonthanh/brain-os-plugin
--          and to write a rule preventing random / partial decisions.
--
-- DeepWiki status: https://deepwiki.org/sonthanh/brain-os-plugin redirects to
-- deepwiki.com but the repo is "Not Indexed" (per rules/research-reverse-
-- engineering.md, blocker recorded; we proceeded with direct upstream
-- README/skill-spec.md/CLAUDE.md/brain-os.config.md research).
--
-- Schema columns: content, tags, importance, memory_type, created_at, tier,
--                 decay_score, category, cognitive_kind
-- ============================================================================

-- ----------------------------------------------------------------------------
-- The deep-analysis rule itself (highest importance — non-negotiable).
-- The full text lives in rules/deep-analysis-rule.md (human-readable
-- projection); the durable durable knowledge is here.
-- ----------------------------------------------------------------------------
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'DEEP-ANALYSIS-BEFORE-ACTION RULE: never decide, propose, or apply a change to TerranSoul on a whim. Before any non-trivial action (adopting an idea from another project, adding a dependency, refactoring a subsystem, accepting or rejecting a research finding) the agent MUST consult both (1) the TerranSoul MCP brain via brain_search/brain_suggest_context/brain_get_entry/brain_kg_neighbors AND (2) the actual source under src/, src-tauri/src/, rules/, docs/, mcp-data/shared/migrations/. External research (DeepWiki, GitHub, papers, videos) is input, never a substitute. Required outputs per decision: (a) MCP-prior-art check with at least three queries against topic+synonyms, (b) source-of-truth read of the affected modules (NOT just READMEs), (c) explicit gap analysis stating what TerranSoul already does vs what is missing vs what was intentionally rejected, (d) verdict in {adopt, partial-adopt, defer, reject} with a grounded reason, (e) self-improve write-back as a numbered SQL migration in mcp-data/shared/migrations/. Partial scans are violations: reading only a top-level README, searching only one MCP keyword, reading only one source file, or skipping any required step. If a required step cannot complete, stop and report the blocker. The full rule lives at rules/deep-analysis-rule.md.',
  'rules,deep-analysis,non-negotiable,decision-protocol,mcp,source-of-truth,self-improve',
  10, 'procedure', 1746489600000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'NO PARTIAL SCAN RULE: a "partial scan" is any of: reading only the top-level README of an upstream repo and concluding about architecture; searching one MCP keyword and declaring "no prior art"; reading one Rust/TS file and inferring a whole subsystem; citing an upstream feature without checking whether TerranSoul already implements it under a different name; skipping the source-code check because "the rule probably says X". Each of these is a process bug. Mandatory minimum for any reverse-engineering or adoption decision: DeepWiki check (or recorded blocker), README + at least two source/spec files upstream, MCP queries against topic + 2 synonyms, and reads of the corresponding TerranSoul module tree, not a single guess path. See rules/deep-analysis-rule.md.',
  'rules,partial-scan,reverse-engineering,non-negotiable,deep-analysis',
  10, 'procedure', 1746489600000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'SELF-IMPROVE WRITE-BACK CONTRACT: every deep analysis ends with a numbered SQL migration appended under mcp-data/shared/migrations/ (002_*.sql, 003_*.sql, ...). Append-only — never edit a shipped migration. Each migration uses INSERT OR IGNORE so re-runs are safe, records RESEARCH:/LESSON:/VERDICT:/RULE: rows with importance >= 7, cognitive_kind="procedural" for rules and "episodic" for one-time research, and adds memory_edges connecting new rows to existing rules/modules. Migrations must be registered in compiled_migrations() inside src-tauri/src/memory/seed_migrations.rs so release builds pick them up without the on-disk folder. Cost of analysis is paid once; future agents retrieve verdicts via brain_search for free. Re-scanning the same upstream repo across sessions is a process bug, not a feature.',
  'rules,self-improve,write-back,migrations,seed,non-negotiable',
  10, 'procedure', 1746489600000, 'long', 1.0, 'self-improve', 'procedural'
);

-- ----------------------------------------------------------------------------
-- brain-os-plugin — what it is (one canonical row + per-component rows)
-- ----------------------------------------------------------------------------
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'BRAIN-OS-PLUGIN OVERVIEW (sonthanh/brain-os-plugin, MIT, ~v2.0.216 as of 2026-05-05): Claude Code plugin (TypeScript 87% / Shell 9% / Python 4%) that turns an Obsidian vault into a "second brain" for the Claude Code CLI. Architecture: skills are pointers (each skills/<name>/SKILL.md), the Obsidian vault is the SSOT for knowledge, GitHub issues are the SSOT for tasks (labels: status:*, priority:p1-p4, weight:heavy/quick, owner:human/bot, type:*), and Claude Code hooks (hooks/hooks.json) fire on SessionStart / PostToolUse / SessionEnd / UserPromptSubmit. 24+ skills across Learn (study, self-learn, research, ingest, absorb, verify, transcribe-video), Think (think, grill, grill-fast, audit), Act (status, aha, journal, handover, pickup, slice, impl, tdd, debug, refactor, vault-lint, eval, gmail, gmail-bootstrap, improve, airtable-knowledge-extract). knowledge/graph/ is currently a .gitkeep placeholder with one "interaction-edge emitter" commit — graph is bootstrapping, not yet substantive. License: MIT. Author: sonthanh + claude (bot). Researched 2026-05-05 from upstream README + skill-spec.md + CLAUDE.md + brain-os.config.md (DeepWiki not indexed).',
  'research,reverse-engineering,brain-os-plugin,claude-code,obsidian,external-project',
  8, 'fact', 1746489600000, 'long', 1.0, 'research', 'episodic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'BRAIN-OS-PLUGIN §11 OUTCOME LOGGING: every skill appends one pipe-delimited line per run to {vault}/daily/skill-outcomes/{skill}.log. Format: {date}|{skill}|{action}|{source_repo}|{output_path}|commit:{hash}|{result} with optional trailing key=value pairs (corrections=N, args="...", score=N.N, interrupt="...", triaged=N, encoded=N, deleted=N, ambiguous=N, expired=N). result in {pass, partial, fail}. Append-only, never rotate. Logged AFTER skill completes (not before). Skills with no artifact use output_path=N/A. These logs feed the /improve skill for pattern detection and skill auto-evolution. Useful pattern for TerranSoul to consider for chunk 34.2 (coding workflow gate telemetry): structured per-run outcome events with normalized result tag and optional severity key=value tail.',
  'research,brain-os-plugin,outcome-logging,telemetry,workflow-gate,chunk-34.2',
  7, 'fact', 1746489600000, 'long', 1.0, 'research', 'episodic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'BRAIN-OS-PLUGIN §12 SKILL TRACE CAPTURE: trace-capture.sh PostToolUse hook (registered in hooks/hooks.json, fires on all tools) writes one JSONL line per tool call to {vault}/daily/skill-traces/{skill}.jsonl. Two event types (skill_start, tool_call), common fields {ts, event, skill, session}, tool_call adds {tool, input} where input is a minimal summary (Read/Edit/Write -> {file}, Glob -> {path}, Grep -> {pattern,glob,type}, Bash -> {command first 120 chars}, WebFetch/WebSearch -> {url}/{query first 120 chars}). Never records tool_response. Boundary tracked via state file ${TMPDIR}/claude-trace/{session_id}.skill (cleared on session-end). Hook ALWAYS exits 0 — silent drop on failure better than blocked tool. Append-only, gitignored, local telemetry only. Pattern useful as inspiration for TerranSoul coding-workflow gate events but not directly portable: TerranSoul is not a Claude Code plugin and has no PostToolUse hook surface. We can record analogous events directly in our memories table with cognitive_kind="workflow_event".',
  'research,brain-os-plugin,trace-capture,telemetry,jsonl,workflow-gate',
  7, 'fact', 1746489600000, 'long', 1.0, 'research', 'episodic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'BRAIN-OS-PLUGIN §13 DESCRIPTION HYGIENE: every enabled skill''s frontmatter description is loaded into Claude Code session-start context, so bloat compounds across many skills * many sessions. Convention: WHAT + WHEN + DIFFERENTIATOR only. Bloat patterns (each fails the gate): >300 chars, 2+ numbered workflow steps, 2+ line breaks, 2+ distinct flag mentions, 2+ distinct technical thresholds. Single mentions are fine when they are the trigger keyword itself. Detection integrated into per-skill /improve flow Phase 1 (scripts/scan-skill-descriptions.py --describe-stdin). Phase 4 generates "description trim" mutation strategy candidates, gated by a semantic-preservation judge (Gate 2) that verifies every routing keyword survives the trim (revert via standard Phase 4 protocol on FAIL). No standalone /improve descriptions sub-mode. Daily /improve cron picks bloated skills automatically.',
  'research,brain-os-plugin,description-hygiene,prompt-budget,context-window',
  6, 'fact', 1746489600000, 'long', 1.0, 'research', 'episodic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'BRAIN-OS-PLUGIN /STUDY + /SELF-LEARN: marquee skill that reads an entire book end-to-end, extracts atomic notes, validates each one against NotebookLM (claimed 159/159 checks, avg 97/100 accuracy), and connects insights into the Obsidian vault — fully autonomous. Dependencies: Python 3.12+, notebooklm-py, ebooklib, beautifulsoup4. /self-learn is the validation core that /study wraps. This validation-loop pattern (extract -> third-party verifier -> accept/revise) is interesting but TerranSoul already has stronger primitives: contextualize.rs (Anthropic Contextual Retrieval), conflicts.rs (LLM contradiction resolution), reranker.rs (LLM-as-judge), and the chunked late_chunking.rs pipeline. We do not need an external NotebookLM dependency — the equivalent role is filled by our cross-encoder rerank threshold (default 0.55).',
  'research,brain-os-plugin,study,self-learn,validation,notebooklm,already-have-equivalent',
  6, 'fact', 1746489600000, 'long', 1.0, 'research', 'episodic'
);

-- ----------------------------------------------------------------------------
-- VERDICT — what to adopt / partial-adopt / defer / reject
-- ----------------------------------------------------------------------------
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'VERDICT brain-os-plugin (2026-05-05): TerranSoul does NOT need to adopt brain-os-plugin''s architecture. Reason: the two systems target different runtimes (TerranSoul is a Tauri 2 + Rust + Vue 3 desktop app with an embedded SQLite+HNSW+KG+RAG brain; brain-os-plugin is a Claude Code CLI plugin whose "brain" is a Markdown vault + Claude with no embeddings, no semantic search, and no graph beyond a .gitkeep). On every brain dimension TerranSoul already does more: real vector ANN (usearch HNSW) vs none, RRF-fused hybrid 6-signal search vs Markdown grep, typed/directional KG edges via memory_edges vs empty knowledge/graph/, multi-provider LLM routing with capability gates vs Claude-only, MCP server exposing the brain to other AI tools vs no external surface, CRDT sync + Ed25519 device identity vs no sync layer, versioned migration-based seed system vs install.sh symlinks. The vault structure (context/, business/, personal/, thinking/, knowledge/, daily/, private/) is a UX layer for human Obsidian users, not an architecture TerranSoul should mirror. GitHub-issues-as-tasks is replaced by our skill-tree quest system + rules/milestones.md. Hooks are replaced by our maintenance_runtime.rs scheduler.',
  'verdict,brain-os-plugin,reject,architecture,2026-05-05',
  9, 'fact', 1746489600000, 'long', 1.0, 'verdict', 'episodic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'VERDICT brain-os-plugin partial-adopt for chunk 34.2 (coding workflow gate telemetry): borrow the IDEA of structured per-run outcome events (§11 outcome logging) and per-tool-call trace events (§12 trace capture) when designing TerranSoul''s workflow-gate telemetry. Specifically: emit one structured event per gate (context-load, plan, edit, validate, archive, PR) with normalized fields {ts, gate, session, action, result in {pass,partial,fail}, key=value tail for severity/score/corrections}. Persist as memory rows with cognitive_kind="workflow_event" — NOT as JSONL on disk. Do NOT borrow: trace-capture.sh hook surface (we have no Claude Code hook layer), the Obsidian vault path scheme, the GitHub-issues task pipeline. Reject reason: those couple us to Claude Code and Obsidian, both incompatible with TerranSoul''s Tauri runtime.',
  'verdict,brain-os-plugin,partial-adopt,chunk-34.2,workflow-gate,telemetry,2026-05-05',
  8, 'fact', 1746489600000, 'long', 1.0, 'verdict', 'episodic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'VERDICT brain-os-plugin defer description-hygiene gate (§13): TerranSoul''s analogue would be a quest/skill description size gate. Defer reason: our quests are defined in TypeScript (src/stores/skill-tree.ts) and rendered in-app, NOT loaded into LLM context every session, so the "bloat compounds across N skills * M sessions" problem does not apply. Re-evaluate IF and WHEN we start injecting quest descriptions into the system prompt. Until then it is solving a problem we do not have.',
  'verdict,brain-os-plugin,defer,description-hygiene,quests,2026-05-05',
  6, 'fact', 1746489600000, 'long', 1.0, 'verdict', 'episodic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'VERDICT brain-os-plugin reject /improve auto-evolving prompts: the /improve flow generates mutation candidates for skill descriptions and gates them via an LLM judge. Reject reason for TerranSoul: (a) high token cost per mutation cycle, (b) requires a stable behavioral eval harness we do not have for our 30+ skill-tree skills, (c) our skill-tree quest descriptions are user-facing UX strings, not LLM-routing surface, so mutating them changes copy not behavior, (d) we already have brain/maintenance_runtime.rs jobs (Decay/GC/PromoteTier/EdgeExtract/ObsidianExport) that handle our durable evolution needs. Re-evaluate only if/when we add LLM-routed skill invocation that uses descriptions as the dispatch surface.',
  'verdict,brain-os-plugin,reject,improve-loop,auto-evolution,2026-05-05',
  6, 'fact', 1746489600000, 'long', 1.0, 'verdict', 'episodic'
);

-- ----------------------------------------------------------------------------
-- DeepWiki blocker (so future sessions don''t retry)
-- ----------------------------------------------------------------------------
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'DEEPWIKI BLOCKER 2026-05-05: https://deepwiki.org/sonthanh/brain-os-plugin redirects to deepwiki.com but the repo is "Not Indexed" with an "Index Repository" button (manual indexing, 2-10 min). Per rules/research-reverse-engineering.md, when DeepWiki is unavailable we record the blocker and proceed with direct upstream research. Sources used instead: README.md, skill-spec.md, CLAUDE.md, brain-os.config.md, hooks/, skills/, knowledge/graph/, references/ tree listings on github.com.',
  'research,deepwiki,blocker,brain-os-plugin,reverse-engineering',
  5, 'fact', 1746489600000, 'long', 1.0, 'research', 'episodic'
);

-- ============================================================================
-- Knowledge graph edges
-- ============================================================================

-- The new rule supports the existing MCP/preflight/reality enforcement layers
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'DEEP-ANALYSIS-BEFORE-ACTION RULE:%'
  AND (
       d.content LIKE 'MCP PREFLIGHT ENFORCEMENT:%'
    OR d.content LIKE 'MCP SELF-LEARNING RULE:%'
    OR d.content LIKE 'DEEPWIKI REVERSE-ENGINEERING RULE:%'
    OR d.content LIKE 'NO PRETEND CODE RULE:%'
    OR d.content LIKE 'VALIDATION AND REALITY RULE:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE d.content LIKE 'DEEP-ANALYSIS-BEFORE-ACTION RULE:%'
  AND (
       s.content LIKE 'NO PARTIAL SCAN RULE:%'
    OR s.content LIKE 'SELF-IMPROVE WRITE-BACK CONTRACT:%'
  );

-- All brain-os-plugin research rows hang off the overview row
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE d.content LIKE 'BRAIN-OS-PLUGIN OVERVIEW%'
  AND (
       s.content LIKE 'BRAIN-OS-PLUGIN §11 OUTCOME LOGGING:%'
    OR s.content LIKE 'BRAIN-OS-PLUGIN §12 SKILL TRACE CAPTURE:%'
    OR s.content LIKE 'BRAIN-OS-PLUGIN §13 DESCRIPTION HYGIENE:%'
    OR s.content LIKE 'BRAIN-OS-PLUGIN /STUDY + /SELF-LEARN:%'
  );

-- Verdicts cite the overview as their evidence source
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'derived_from', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE d.content LIKE 'BRAIN-OS-PLUGIN OVERVIEW%'
  AND s.content LIKE 'VERDICT brain-os-plugin%';

-- The deep-analysis rule was triggered by this research session
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'DEEP-ANALYSIS-BEFORE-ACTION RULE:%'
  AND d.content LIKE 'BRAIN-OS-PLUGIN OVERVIEW%';

-- The chunk-34.2 verdict supports the (future) workflow-gate telemetry work
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 0.9, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'VERDICT brain-os-plugin partial-adopt for chunk 34.2%'
  AND d.content LIKE 'BRAIN-OS-PLUGIN §11 OUTCOME LOGGING:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 0.9, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'VERDICT brain-os-plugin partial-adopt for chunk 34.2%'
  AND d.content LIKE 'BRAIN-OS-PLUGIN §12 SKILL TRACE CAPTURE:%';

-- The DeepWiki blocker is a context note for the overview
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'DEEPWIKI BLOCKER 2026-05-05:%'
  AND d.content LIKE 'BRAIN-OS-PLUGIN OVERVIEW%';
