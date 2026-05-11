# TerranSoul — Backlog

> **Never-started work lives here.** Only move chunks from this file to
> `milestones.md` when the user explicitly says so. This file is the holding
> area for planned but unscheduled work.
>
> ⛔ **RULE: Never start chunks from this file.**
> All chunk implementation must begin from `rules/milestones.md`.
> If milestones.md has no `not-started` chunks, ask the user which backlog items to promote.
> See `rules/prompting-rules.md` for full enforcement rule.

---

## Phase 9 — Learned Features (From Reference Projects)

> **Source repos:** Open-LLM-VTuber, AI4Animation-js, VibeVoice, aituber-kit
> **Analysis:** See `rules/research-reverse-engineering.md` §9.
> **Principle:** Integrate proven patterns; don't reinvent.

### High Priority

📦 Promoted to `rules/milestones.md` — chunks 106–109.

### Medium Priority

✅ Reconciled 2026-05-02 — chunks 094–098 are implemented and backfilled in `rules/completion-log.md`; no active milestone rows remain.

### Lower Priority

✅ Reconciled 2026-05-02 — chunks 116, 118, and 119 are implemented and backfilled in `rules/completion-log.md`; chunks 115 and 117 are closed/demoted below.

### Demoted from Milestones

| Chunk | Description | Status | Notes |
|-------|-------------|--------|-------|
| 115 | Live2D Support | Closed / no-action | VRM remains the sole supported avatar format; Live2D is rejected for licensing/runtime fit in `docs/persona-design.md` and `docs/licensing-audit.md`. |

📦 Promoted to `rules/milestones.md` on 2026-05-11 — chunk 117, constrained to CI/research/service containers only.

---

## Phase 33B — Claudia Adoption Catalogue (Reverse-Engineered from `kbanc85/claudia`)

> **Source:** [`mcp-data/shared/claudia-research.md`](../mcp-data/shared/claudia-research.md)
> — full reverse-engineering analysis with module mapping, license boundary
> (PolyForm-NC 1.0.0 → patterns/ideas only, **no code copy**), and adoption
> rationale. Each row below is a single promotable chunk; promote to
> `rules/milestones.md` Phase 33 (or a successor) when the user says so.
> Sequence is roughly highest → lowest leverage.

✅ Reconciled 2026-05-06 — chunks 33B.1–33B.10 are implemented and recorded in `rules/completion-log.md`; no additional 33B backlog rows remain.

---

## Phase 36B — Understand-Anything Adoption Catalogue (Reverse-Engineered from `Lum1104/Understand-Anything`)

> **Source:** [`Lum1104/Understand-Anything`](https://github.com/Lum1104/Understand-Anything)
> (MIT). Studied public README, plugin layout, agent catalogue, and license only;
> no source, prompts, assets, or branding copied. Promote rows only when the user
> explicitly asks for these ideas to move into `rules/milestones.md`.

✅ Reconciled 2026-05-06 — chunks 36B.1–36B.4 are implemented and recorded in `rules/completion-log.md`; no active backlog rows remain for this phase.

---

## Phase 10 — Developer Experience & Copilot Integration

> **Goal:** Streamline the AI-assisted development loop so Copilot (and other
> coding agents) can run long autonomous sessions without manual babysitting.

📦 Promoted to `rules/milestones.md` — chunks 10.1–10.3.

---

## Phase 43 — Coding-Workflow Redesign (Reference Specs)

> **Status:** In progress (2026-05-07). All chunks promoted to `rules/milestones.md`.
> Detailed specs preserved here for implementation reference.
> **Goal:** Adopt strongest patterns from jcode reverse-engineering study into 
> TerranSoul's coding workflow under neutral names.

### 43.1 — Memorable session names + idle timeout for headless MCP

Two-word `adjective + animal` session naming in `mcp-data/sessions.json`. Add 
`--resume <name>` to headless runner + `coding_session_resume` Tauri command. 
Configurable idle timeout (default 5 min for `npm run mcp`, disabled for app MCP). 
Ensure `coding/coding_sessions.rs` is single source of truth. Tests: collision 
avoidance, case-insensitive lookup. Success: `npm run mcp -- --resume blazing-fox` 
reattaches without losing context.

### 43.2 — V20 schema migration

Single migration adds:
- `memories.confidence REAL NOT NULL DEFAULT 1.0`
- `memory_reinforcements(memory_id, session_id, message_index, ts, PK(...))`
- `memory_trigger_patterns(memory_id, pattern, kind)` 
- `memory_gaps(id, query_embedding BLOB, context_snippet, session_id, ts)`
- `safety_decisions(id, action, decision, decided_at, decided_via)`

Backfill existing rows with `confidence=1.0`. Tests: forward migration + round-trip.

### 43.3 — Per-category confidence decay

Apply per-`cognitive_kind` half-lives in `memory/maintenance_runtime.rs`:
Correction 365d, Preference 90d, Procedure 60d, Fact 30d, Inferred 7d. 
Multiplies final score in `hybrid_search_rrf` + `brain_search`. Benchmark: 
O(N) on 1M rows, < 50ms. Configurable via `coding/config.toml::[memory.decay]`.

### 43.4 — Reinforcement provenance

Hook `memory_reinforcements` inserts at: `memory/reranker.rs` (LLM-as-judge ≥ threshold) 
+ `commands/streaming.rs` (post-turn judge). `brain_get_entry` returns last 10 
reinforcements. Surface in `MemoryDetailPanel.vue`. Tests: insert idempotency on PK.

### 43.5 — Cascade retrieval through memory_edges

New helper post-`hybrid_search_rrf` top-K: BFS depth ≤ 2, edge-weighted decay 
`weight * 0.7^depth`. Edge priors: Supersedes 0.9, HasTag 0.8, RelatesTo (confidence), 
InCluster 0.6, Contradicts/DerivedFrom 0.3. Behind `cascade=true` flag; default for 
`brain_suggest_context`. A/B bench: recall@10 hold ±1% vs flat RRF.

### 43.6 — Post-retrieval maintenance background task

New `memory/post_retrieval.rs` triggered post-LLM-judge verdict. Async via `tokio::spawn`: 
strengthen RelatesTo edges between co-relevant pairs, bump confidence (+0.05 cap 1.0) on 
verified, decay (-0.02) on rejected, log gap when no hits, refresh tag inference every N. 
Configurable in `coding/config.toml`.

### 43.7 — Negative memories + trigger patterns

Extend `cognitive_kind` with `negative`. New `memory/negative.rs` scans new context 
(file, user message, diff) against active triggers, prepends matching ones as 
`[NEGATIVE — DO NOT DO THIS]`. Backfill: `code_extract_negatives` imports rules from 
`rules/coding-standards.md`. Surface `MemoryNegativeBadge.vue`.

### 43.8 — Gap detection

When `top_score < 0.3 && embedding_norm > 0.7` post-`hybrid_search_rrf`, persist 
`memory_gaps` row. New `review_gaps` MCP tool. UI: `MemoryGapsPanel.vue` in 
brain-debug view shows recent gaps + context snippet.

### 43.9 — Embedding-indexed instruction slices

Replace bulk-XML rules injection with per-section retrieval. Chunk 
`rules/instructions/docs` by markdown heading, index as `category=rule, 
cognitive_kind=instruction`. Embed task, pull top-K (default 10). Prepend TOC 
per file. Guarantee hit on rule topic matching task. Behind `--bulk-rules` escape. 
Success: ≥ 5k token context shrink on regression suite without recall drop.

### 43.10 — Tier 1 / Tier 2 safety classifier

New `coding/safety.rs` + `commands/safety.rs`. `Action` enum: read, write, run-tests, 
create-branch, push-remote, open-pr, merge-pr, run-shell, send-email, install-package, …. 
Static table + per-project override in `coding/config.toml`. Persistent 
`safety_decisions` rows. Propose Tier 1 promotion after 14 consecutive approvals. 
Centralize all gate sites through one `request_permission` entry point.

### 43.11 — Background-maintenance agent skeleton

New `coding/ambient.rs` + `coding/ambient_scheduler.rs` wrapping 
`memory/maintenance_scheduler.rs`. Tools: garden, extract_from_session, verify_fact, 
scout_recent_sessions, request_permission, schedule_next, end_cycle (mandatory). 
Adaptive scheduler reads `x-ratelimit-*` headers, reserves 20% for user, exponential 
backoff on 429. PID guard in `mcp-data/`. Default: `enabled=false`, 
`proactive_work=false` until 20 decision-history cycles exist.

### 43.12 — Cross-harness session import

New `coding/session_import.rs` + `code_import_session` command. Detect transcripts: 
`~/.claude/sessions/`, `~/.codex/sessions/`, `~/.opencode/sessions/`, 
`~/.cursor/transcripts/`, `~/.config/github-copilot/cli/`. Format: JSON/JSONL/SQLite. 
Extract turns + tools, run through `memory/brain_memory.rs::extract_memories`, tag 
`imported_from=<harness>`. Reuse redaction helper. No replay mode yet.

### 43.13 — Post-completion comparative review

After all remaining Phase 43 chunks complete, produce neutral comparison: TerranSoul 
vs jcode + ≥4 active agents (Claude Code, Codex, Cursor, Copilot). Focus on workflow 
outcomes: session continuity, memory quality, context efficiency, safety, orchestration, 
self-improve throughput. Deliverables: `docs/coding-workflow-comparison-2026.md` + 
follow-up milestone proposal + MCP memory sync in `mcp-data/shared/memory-seed.sql`.
