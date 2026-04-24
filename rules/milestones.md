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

> **Completed work lives in [`rules/completion-log.md`](completion-log.md).**
> Do not re-list done chunks here. Phases 0–11 (Foundation through RPG Brain
> Configuration), Chunks 1.2 / 1.3 / 1.4 / 1.5 / 1.6 / 1.7 / 1.8, the
> Phase 9 Learned-Features batch, and all Post-Phase polish are recorded
> there in reverse-chronological order.
>
> Phase 7 — VRM Model Security: ❌ removed (2026-04-24). Encryption-based
> VRM asset protection is not feasible for an open-source desktop app
> (any decryption key compiled into the binary is extractable, reducing
> the scheme to obfuscation rather than real DRM). Chunks 100–105 will
> not be re-promoted.

---

## Next Chunk

**Chunk 1.10 — Cross-encoder reranker pass.** Use BGE-reranker-v2-m3
(or any reranker model) via Ollama to re-score the top-k candidates
returned by `hybrid_search_rrf` for higher precision.

---

## Active Chunks

### Phase 12 — Brain Advanced Design (Documentation & QA)

| # | Chunk | Status | Owner | Notes |
|---|---|---|---|---|
| 1.1 | Brain Advanced Design — Validation, Docs Rewrite, QA Walkthrough | in-progress | agent + user (screenshots) | Source tracking + cross-framework comparison table done; user-captured screenshots remain |
| 1.10 | Cross-encoder reranker pass (BGE-reranker-v2-m3 via Ollama) | not-started | agent | §19.2 row 10 (🔵→✅), §16 Phase 6 |
| 1.11 | Temporal KG edges — V6 schema with `valid_from` / `valid_to` | not-started | agent | §19.2 row 13 (🔵→✅), §16 Phase 6 |

#### Chunk 1.1 — Brain Advanced Design — Validation, Docs Rewrite, QA Walkthrough

**Goal.** Validate `docs/brain-advanced-design.md` against best-in-class
open-source references (Obsidian, SiYuan, RAGFlow), confirm the Phase-1
implementation in `src-tauri/src/memory/` + `src-tauri/src/commands/ingest.rs`
matches the design, and rewrite the user-facing docs around a single
end-to-end scenario (Vietnamese law portal `http://thuvienphapluat.vn/` +
internal-firm-rules PDF) so a fresh user can reproduce it step-by-step.

**Done in prior PRs (agent).**
- [x] Read `docs/brain-advanced-design.md` end-to-end and audited current code.
- [x] Wrote design-validation summary (Obsidian / SiYuan / RAGFlow comparison
      table) inside the rewritten walkthrough.
- [x] Replaced `instructions/BRAIN-COMPLEX-EXAMPLE.md` with a focused
      walkthrough of the thuvienphapluat.vn + PDF scenario.
- [x] Replaced `instructions/BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md` with a
      concise quick-reference.
- [x] Source tracking pipeline — Extended `NewMemory` + `MemoryEntry`
      with `source_url`, `source_hash`, `expires_at`; full hash-based
      dedup + staleness in `run_ingest_task`.
- [x] Cross-framework comparison table — Added a single consolidated
      table to `docs/brain-advanced-design.md` §13 contrasting TerranSoul
      against LangChain, Odyssey, RAGFlow, SiYuan and GitNexus across 14
      dimensions (purpose, distribution, storage, retrieval, graph, etc.).

**Remaining (user environment / follow-up).**
- [ ] Capture **scenario-specific** screenshots on a real Tauri build with
      Vietnamese content loaded.

#### Chunk 1.10 — Cross-encoder reranker pass

**Goal.** After hybrid retrieval returns top-k candidates, re-score them
with a cross-encoder model (BGE-reranker-v2-m3 via Ollama) for higher
precision, then return the top-N reranked subset. Ollama is queried via
the existing `OllamaAgent` wrapper; reranker model is configurable.

**Acceptance.** Unit tests on the reranker request/response parser and
the score-merge function; integration with `hybrid_search_rrf` (Chunk
1.8); documented in §19.2 row 10 status update.

#### Chunk 1.11 — Temporal KG edges (V6 schema)

**Goal.** Add `valid_from` / `valid_to` columns to `memory_edges` (V6
migration), expose `valid_at: i64` parameter on edge queries so
graph traversal can answer "what was true on date X?". Supports
contradicting-fact resolution (Zep / Graphiti pattern, 2024).

**Acceptance.** V6 migration up/down + tests; `MemoryEdge` struct
extended; `list_memory_edges` and graph traversal accept optional
`valid_at`; backward-compat default keeps current behaviour for callers
that don't pass `valid_at`; §19.2 row 13 status update.

---
