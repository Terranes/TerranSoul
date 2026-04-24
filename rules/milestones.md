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
> Configuration), Chunks 1.2 / 1.3 / 1.4 / 1.5 / 1.6 / 1.7 / 1.8 / 1.9 / 1.10 / 1.11,
> the Phase 9 Learned-Features batch, and all Post-Phase polish are recorded
> there in reverse-chronological order.
>
> Phase 7 — VRM Model Security: ❌ removed (2026-04-24). Encryption-based
> VRM asset protection is not feasible for an open-source desktop app
> (any decryption key compiled into the binary is extractable, reducing
> the scheme to obfuscation rather than real DRM). Chunks 100–105 will
> not be re-promoted.

---

## Next Chunk

**Chunk 2.1 — GitNexus sidecar agent (Tier 1 of GitNexus integration).**
Add a sidecar agent (`InstallMethod::Sidecar`) that spawns
`npx gitnexus mcp` over stdio and exposes the four core read-only
tools (`gitnexus_query`, `gitnexus_context`, `gitnexus_impact`,
`gitnexus_detect_changes`) as Tauri commands behind a
`code-intelligence` orchestrator capability gate. **Strictly
out-of-process** — GitNexus's PolyForm-Noncommercial-1.0.0 license
prevents bundling, so the user installs it under their own license
terms via the marketplace. See chat plan from this session for the
full reverse-engineering analysis and the four-tier roadmap (2.1
sidecar → 2.2 RAG fusion → 2.3 KG mirror → 2.4 BrainView panel).

---

## Active Chunks

### Phase 12 — Brain Advanced Design (Documentation & QA)

| # | Chunk | Status | Owner | Notes |
|---|---|---|---|---|
| 1.1 | Brain Advanced Design — Validation, Docs Rewrite, QA Walkthrough | in-progress | agent + user (screenshots) | Source tracking + cross-framework comparison table done; user-captured screenshots remain |

### Phase 13 — GitNexus Code-Intelligence Integration

> Reverse-engineering and four-tier integration plan derived from GitNexus
> v1.6.x (`abhigyanpatwari/GitNexus`, PolyForm-Noncommercial-1.0.0).
> Strategy: out-of-process sidecar — never bundle GitNexus binaries.
> Each chunk ships independently and may be reordered by user demand.

| # | Chunk | Status | Owner | Notes |
|---|---|---|---|---|
| 2.1 | GitNexus sidecar agent (stdio MCP bridge for `query` / `context` / `impact` / `detect_changes`) | not-started | agent | Tier 1 of integration plan; reuses Chunk 1.5 agent-roster + sidecar `InstallMethod` infra; ~400 LOC + tests |
| 2.2 | Code-RAG fusion in `rerank_search_memories` (recall stage also queries GitNexus when an active repo is configured) | not-started | agent | Tier 2; depends on 2.1; ~150 LOC; uses existing `memory::fusion::reciprocal_rank_fuse` |
| 2.3 | Knowledge-graph mirror — V7 schema adds `edge_source` column; `gitnexus_sync` / `gitnexus_unmirror` Tauri commands; map `CONTAINS`/`CALLS`/`IMPORTS`/`EXTENDS`/`HANDLES_ROUTE` to the existing 17-relation taxonomy | not-started | agent | Tier 3; opt-in only; never auto-syncs at startup; ~500 LOC + integration test |
| 2.4 | BrainView "Code knowledge" panel — list indexed repos, last-sync time, blast-radius pre-flight indicator | not-started | agent | Tier 4; pure frontend; depends on 2.1 |

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

---
