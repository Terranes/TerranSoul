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

✅ Completed 2026-05-11 — chunk 117, constrained to CI/research/service containers only, is archived in `rules/completion-log.md`.

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

✅ Reconciled 2026-05-11 — chunks 43.1–43.13 are implemented and recorded in `rules/completion-log.md`; no active backlog rows remain for this phase.

---

## Phase 48 — TencentDB Drill-Down + Symbolic Offload (Reverse-Engineered from `Tencent/TencentDB-Agent-Memory`)

> **Source:** [`Tencent/TencentDB-Agent-Memory`](https://github.com/Tencent/TencentDB-Agent-Memory)
> (MIT, v0.3.4, 2.2k stars), [DeepWiki](https://deepwiki.com/Tencent/TencentDB-Agent-Memory)
> (indexed 2026-05-14, commit `285896f8`). Full analysis in
> [docs/tencentdb-agent-memory-research.md](../docs/tencentdb-agent-memory-research.md).
> Lesson seeded as `seed:lesson-tencentdb-agent-memory-drilldown-2026-05-17`.
> Studied public README + DeepWiki only; no source, prompts, schema column
> names, asset names, branded identity, or `tdai_*` tool surface copied.
> Promote rows only when the user explicitly asks for these ideas to move
> into `rules/milestones.md`.

| Chunk | Status | Scope |
|---|---|---|
| MEM-DRILLDOWN-1 | **done 2026-05-17** | Implemented in commit-prep batch — see `rules/completion-log.md` "Chunk MEM-DRILLDOWN-1". `MemoryStore::source_chain`, `BrainGateway::drilldown`, MCP `brain_drilldown`, Tauri `memory_drilldown`. Follow-ups deferred: wiring `brain_memory::summarize`/`extract` to emit `derived_from` edges automatically (today's consolidation already does), and UI affordance in `MemoryView`. |
| CTX-OFFLOAD-1a | **done 2026-05-17** | Storage primitive + gateway + MCP/Tauri surfaces. V23 schema adds `memory_offload_payloads(memory_id PK FK->memories ON DELETE CASCADE, payload BLOB, byte_count, mime_type DEFAULT 'text/plain', created_at)`. New `src-tauri/src/memory/offload_payload.rs` provides `MemoryStore::{add,get,get_info,delete}_offload_payload` + `offload_payload_total_bytes` + 7 unit tests. New `BrainGateway::drilldown_payload`, MCP tool `brain_drilldown_payload`, Tauri command `memory_drilldown_payload`. Bundled fixes: gix-hash 0.25 `sha1` feature unification + rustls 0.23 default crypto provider install (3 quic tests now pass). See `rules/completion-log.md` "Chunk CTX-OFFLOAD-1a". |
| CTX-OFFLOAD-1b | not-started | **Coding runtime offload hook + quest.** Wire CTX-OFFLOAD-1a into `coding/runtime_hooks` with a new `OffloadToolOutputHook` modelled on `SummarizationHook`: when a tool result exceeds `offload_threshold_chars` (reconcile with existing `coding/offload.rs::OFFLOAD_CHAR_THRESHOLD = 40_000`; backlog originally suggested 4_000), create a `memory` row carrying a summary, store raw bytes via `add_offload_payload`, replace the in-context message with `{kind: "tool_output_ref", id, summary, byte_count}`. Skill-tree quest "Context Compression" activates when at least one offloaded payload exists. Tests: hook offload round-trip, summary fidelity, agent re-inflation via `brain_drilldown_payload`. |
| MEM-SCENARIO-1 | not-started, deferred | **Per-task scenario aggregation tier.** Equivalent of L2 Scenario blocks. Requires design review: extend `MemoryType` (which ripples into every `add_many` call-site) vs. add a `scenario_id` column on `memories` and use existing kinds. Re-evaluate after MEM-DRILLDOWN-1 lands so we can decide based on actual drill-down UX. |
