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
| CTX-OFFLOAD-1b | **done 2026-05-17** | New `src-tauri/src/coding/offload_tool_output_hook.rs` — `OffloadToolOutputHook { store: Arc<Mutex<MemoryStore>>, threshold_chars }` (default 40_000 from `coding/offload::OFFLOAD_CHAR_THRESHOLD`). On `wrap_tool_call`, if result content exceeds threshold: adds a `MemoryType::Context` row (tags `tool_output,offloaded,call:<id>`, importance 2), stores raw bytes via `add_offload_payload(entry.id, bytes, "text/plain")`, replaces in-context content with `{"kind":"tool_output_ref","id":<i>,"summary":<head…tail>,"byte_count":<n>}`. Failure-safe: returns original result on any DB error. 3 unit tests (small pass-through, large round-trip via `get_offload_payload`, summary head/tail marker). New Tauri command `memory_offload_payload_total_bytes` + memory store ref `offloadPayloadBytes` (refreshed in `fetchAll`). Skill-tree quest `context-compression` (tier=advanced, requires=`rag-knowledge`, category=brain) auto-activates when `offloadPayloadBytes > 0`. All vue-tsc clean, 76/76 skill-tree tests pass, clippy clean. |
| MEM-SCENARIO-1 | **done 2026-05-17** | **Per-task scenario aggregation (L2) tier.** Design decision: added nullable `scenario_id INTEGER REFERENCES memories(id) ON DELETE SET NULL` column on `memories` instead of extending `MemoryType` (avoids ripple into every `add_many` call-site). Schema V24 (`ensure_v24_scenario_id` + canonical CREATE + index `idx_memories_scenario_id ... WHERE scenario_id IS NOT NULL`). New `src-tauri/src/memory/scenario.rs` (~340 lines) — `ScenarioSummary { scenario_id, content, tags, importance, created_at, member_count }`, methods `create_scenario(head, member_ids)`, `set_scenario_id`, `assign_members_to_scenario` (transactional via `conn.unchecked_transaction()`), `get_scenario_id`, `list_scenario_members` (excludes head, chronological), `list_scenarios(limit)` (heads with members only), `scenario_total_count` (distinct heads). 9 unit tests including `deleting_scenario_head_nulls_member_pointers_not_member_rows` (ON DELETE SET NULL contract) and `assign_members_is_transactional_and_idempotent`. 5 Tauri commands (`memory_create_scenario`, `memory_list_scenarios`, `memory_list_scenario_members`, `memory_set_scenario_id`, `memory_scenario_total_count`). Frontend store `scenarioCount` ref + `refreshScenarioCount()` wired into `fetchAll`. New skill-tree quest `scenario-aggregation` (tier=advanced, requires=`rag-knowledge`, category=brain, icon=🎬) auto-activates when `scenarioCount > 0`. `store::row_to_entry` promoted to `pub(crate)` for cross-module reuse. cargo test/clippy/vue-tsc all clean; vitest 76/76 skill-tree pass; full vitest 1969/1969 pass after a 1-line test update for the new best-effort telemetry calls in `fetchAll`. See `rules/completion-log.md` "Chunk MEM-SCENARIO-1". |
| CLAIM-VERIFY-1/2/3 | **done 2026-05-17** | **Contradiction subsystem — user-visible feature.** (1) New `pub(crate) embed_and_detect_contradiction` helper in `src-tauri/src/commands/memory.rs` collapses the embed → persist → find_duplicate(0.85) → check_contradiction → record_contradiction_if pipeline into one call; both `add_memory_inner` and the chat-extract auto-detect block now use it. (2) New Tauri command `memory_scan_recent_for_conflicts(limit=50, max=500)` returns count of newly-opened conflicts; backfills users who toggled `auto_detect_conflicts` ON after the fact. (3) New `MemoryStore::contested_memory_ids() -> HashSet<i64>` (single UNION query, empty-set fast path). (4) `hybrid_search_rrf` + `hybrid_search_rrf_with_intent` apply a `0.7×` score penalty to ids in the contested set during the per-kind decay step. New unit test `hybrid_search_rrf_penalizes_contested_memories` proves an uncontested peer ranks above a contested one of equal relevance. (5) Frontend: new `memory.ts` `openConflictCount` ref + `refreshOpenConflictCount()` (fired in `fetchAll`) + `scanRecentForConflicts(limit)` method. (6) New skill-tree quest `claim-verification` (tier=advanced, requires=rag-knowledge, category=brain, icon=⚖️) auto-activates when `openConflictCount > 0`. (7) MemoryView Contradictions section: side-by-side A/B with "Keep A"/"Keep B"/"Dismiss" buttons + Refresh + "Scan recent (50)" buttons. Bonus: fixed pre-existing canonical-schema test by removing `idx_memories_scenario_id` from inline CANONICAL_SCHEMA_SQL (lives in `ensure_v24_scenario_id` only). Validation: cargo build/clippy clean, cargo test --lib 3044/3044 pass, vue-tsc clean, vitest 1969/1969 pass. See `rules/completion-log.md` "Chunk CLAIM-VERIFY-1/2/3". |
