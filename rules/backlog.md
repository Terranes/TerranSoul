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
| 117 | Docker Containerization | Demoted | Containers are not useful for the Tauri desktop runtime by default; keep Docker/Aspire-style work in research/CI only until explicitly promoted. |

---

## Phase 33B — Claudia Adoption Catalogue (Reverse-Engineered from `kbanc85/claudia`)

> **Source:** [`mcp-data/shared/claudia-research.md`](../mcp-data/shared/claudia-research.md)
> — full reverse-engineering analysis with module mapping, license boundary
> (PolyForm-NC 1.0.0 → patterns/ideas only, **no code copy**), and adoption
> rationale. Each row below is a single promotable chunk; promote to
> `rules/milestones.md` Phase 33 (or a successor) when the user says so.
> Sequence is roughly highest → lowest leverage.

| Chunk | Title | Goal |
|---|---|---|
| 33B.1 | Persisted judgment-rules artefact | New `cognitive_kind='judgment'` rows + Tauri commands `judgment_add` / `judgment_list` / `judgment_apply`; inject top-N matching judgments into `commands/chat.rs` system prompt. Adapts Claudia's `judgment.yaml`. |
| 33B.2 | `/reflect` slash command for session reflection | User-invocable wrapper around the existing session-memory absorption pipeline (Chunks 30.2 + 30.6); writes a `session_reflection` memory with `derived_from` edges to the turns it summarises. Adapts Claudia's `/meditate`. |
| 33B.3 | `quest_daily_brief` skill-tree quest | Once-per-day quest that runs `hybrid_search_rrf("overdue OR upcoming OR commitment", since=now-1d)` via `memory/temporal.rs` and surfaces results in the existing skill-tree UI. Adapts Claudia's `/morning-brief`. |
| 33B.4 | Memory-audit provenance view | New brain-panel tab that joins `memories ⨝ memory_versions ⨝ memory_edges` and renders a provenance tree per entry. Adapts Claudia's `/memory-audit`. |
| 33B.5 | `BrainGraphViewport.vue` 3-D KG visualiser | Three.js + d3-force-3d component consuming `memory_edges` + `memories`; node colour = `cognitive_kind`, edge colour = `rel_type`. Adapts Claudia's brain visualiser concept. |
| 33B.6 | Agent-roster capability tags + tag-based routing | Extend `agents/roster.rs` with capability tags (`{code,web,memory,vision,…}`); `coding/coding_router.rs` selects by tag instead of name. Adapts Claudia's two-tier agent team architecture. |
| 33B.7 | Per-workspace `data_root` setting | Allow `app_settings.json` to override the SQLite + HNSW + Obsidian-export root per workspace, so multiple projects can share one TerranSoul install without colliding state. |
| 33B.8 | Stdio MCP transport adapter on top of `BrainGateway` | Add an alternate transport (alongside HTTP) that speaks JSON-RPC over stdio for editors that only support stdio MCP. Reuses the existing `BrainGateway` trait — no new business logic. |
| 33B.9 | PARA opt-in template for `obsidian_export.rs` | Optional Project / Area / Resource / Archive folder layout for the one-way Obsidian export, behind a setting. Read-only projection only — vault is never the source of truth (`memory-philosophy.md` rule 1). |
| 33B.10 | Standalone scheduler daemon for non-GUI runs | If Phase 33 chunk 33.6 lands, harden the maintenance scheduler into a dedicated `terransoul-scheduler` binary so power users can run brain maintenance on a server without the desktop app. |

---

## Phase 10 — Developer Experience & Copilot Integration

> **Goal:** Streamline the AI-assisted development loop so Copilot (and other
> coding agents) can run long autonomous sessions without manual babysitting.

📦 Promoted to `rules/milestones.md` — chunks 10.1–10.3.
