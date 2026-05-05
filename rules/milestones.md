# TerranSoul — Milestones

> **To continue development, tell the AI agent:**
>
> ```
> Continue
> ```
>
> The agent will read this file, find the next chunk with status `not-started`,
> implement it, update the status to `done`, **remove the row from this file**,
> and log details in `rules/completion-log.md`.

> **ENFORCEMENT RULE — Completed chunks must be archived.**
>
> When a chunk is marked `done`:
> 1. Log full details (date, goal, architecture, files created/modified, test counts) in `rules/completion-log.md`.
> 2. **Remove the done row from this file.** `milestones.md` contains only `not-started` and `in-progress` chunks.
> 3. If an entire phase has no remaining rows, drop the phase heading too.
> 4. Update `Next Chunk` (below) to point to the next `not-started` chunk.
>
> This rule is mandatory for every AI agent session. Never leave done rows
> in milestones.md — the full historical record lives in `completion-log.md`.
>
> **Additional:** If the chunk was derived from reverse-engineering research,
> also clean up `rules/research-reverse-engineering.md` and `rules/backlog.md`.
> See `rules/prompting-rules.md` -> "ENFORCEMENT RULE — Clean Up Reverse-Engineering Research on Chunk Completion".

> **Completed work lives in [`rules/completion-log.md`](completion-log.md).**
> Do not re-list done chunks here. Phases 0–28 and all previously tracked
> chunks are recorded there in reverse-chronological order.

---

## Next Chunk

**Chunk 33.4 — Auto-edge extraction on memory ingest** (Phase 33). See the Phase 33 table below.

---

## Phase 32 — MCP Agent-Ready, Self-Improve Autonomy, Animation Wiring & Hardening

> Close the remaining gaps so any agent (Copilot, Codex, Cursor, Claude Code)
> can connect to `npm run mcp` with zero manual setup, the self-improve loop
> actually completes chunks end-to-end, animation pose streaming works live,
> and documentation covers the full contributor onboarding flow.

| ID | Status | Title | Goal |
|---|---|---|---|

---

## Phase 33 — MCP Memory Stack Full-Stack Optimization (SQLite + HNSW + KG-edges + RRF + HyDE + Reranker)

> **Why this phase exists:** the seed in `mcp-data/shared/memory-seed.sql` now
> populates rows + typed edges, but the **vector / HNSW / reranker** layers of
> the brain are not exercised on a fresh `npm run mcp` run because no embedding
> backfill is triggered post-seed and the headless runner has no LLM provider
> by default. Result: RRF degrades to 5 of 6 signals on the canonical dataset
> until an end-user starts the GUI app. This phase closes that gap so every
> retrieval layer (SQLite schema · FTS5 · HNSW vectors · KG edges · RRF fusion ·
> HyDE expansion · LLM-as-judge reranker) is **fully active on the seeded
> dataset out of the box**.
>
> **Source docs (durable analysis already absorbed into the brain):**
> [`mcp-data/shared/memory-philosophy.md`](../mcp-data/shared/memory-philosophy.md)
> · [`mcp-data/shared/claudia-research.md`](../mcp-data/shared/claudia-research.md)
> · [`docs/brain-advanced-design.md`](../docs/brain-advanced-design.md).
> Stay-out-of-scope items captured in `rules/backlog.md` Phase 33B.

| ID | Status | Title | Goal |
|---|---|---|---|
| 33.4 | not-started | Auto-edge extraction on memory ingest | After every `memory_ingest` (and the post-seed pass from 33.1), schedule `parse_llm_edges` → `add_edges_batch` so new memories join the KG without manual seeding. Reuse the rate-limit + cost-cap from `auto_promote_memories`. Verify with a Vitest store mock + Rust test. |
| 33.5 | not-started | Reranker default-on for RRF + relevance threshold pruning | Flip `BrainConfig.rerank` default to `true` for `SearchMode::Rrf`; add a configurable `rerank_threshold` (default 0.55) that drops candidates before they land in `[LONG-TERM MEMORY]`. Wire the threshold to `commands/chat.rs` system-prompt assembly. Tests: mock reranker, assert pruning. |
| 33.6 | not-started | Maintenance scheduler in headless MCP runner | Today `auto_promote_memories`, `edge_conflict_scan`, `consolidate_duplicates`, and `backfill_embeddings` only run inside the GUI Tauri app's tick loop. Hoist the scheduler into a shared `memory::maintenance::spawn` task started by both `lib.rs::run` and the headless `mcp-http` binary, with intervals from `app_settings.json`. |

---

## Phase 34 — Self-Improve Operations Dashboard & Coding Workflow UX

> Make self-improve understandable to humans while the autonomous loop works:
> show what finished, what is currently running, what is queued/backlogged, and
> which coding workflow gate is active before PR review.

| ID | Status | Title | Goal |
|---|---|---|---|
| 34.1 | not-started | Persisted self-improve workboard | Promote the panel's finished / working / backlog lanes from derived frontend state into a persisted backend workboard sourced from `rules/milestones.md`, the run log, and completion-log archival state. |
| 34.2 | not-started | Coding workflow gate telemetry | Emit structured workflow-gate events for context load, plan, edit, validate, archive, and PR so the panel can show the exact active gate and last successful gate per session. |
| 34.3 | not-started | Backlog promotion controls | Add safe UI controls for converting failed runs, research ideas, and deferred self-improve items into scoped milestone chunks without editing markdown by hand. |

---
