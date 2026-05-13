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
