567# TerranSoul — Milestones

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
> Do not re-list done chunks here. Chunks are recorded there in reverse-chronological order.

---

## Next Chunk

The next active chunk is **THEME-COCKPIT-1c** — audit and spread the
cockpit pattern across Memory, Settings, Chat root, and the splash.
**THEME-COCKPIT-1b** landed 2026-05-16 (mood-driven palette accent on
`.bp-shell[data-accent]`; see completion log). After 1c the queue
continues with **GRAPHRAG-1a** (hierarchical Leiden community summaries).
**BENCH-SCALE-3** remains code-done / run-deferred (IVF-PQ 10M-doc
runner + ~40h wall-clock pass).

---

## Phase THEME-COCKPIT — sci-fi HUD cockpit aesthetic

Goal: match the visual quality of the user-authored `Brain Panel.html`
reference (deep navy cards with layered cyan glow, corner reticles,
numbered tracked-caps section labels, breadcrumb header, brain-orb hero).

| Chunk | Status | Scope |
|---|---|---|
| THEME-COCKPIT-1a | done 2026-05-16 | **Tokens + utility primitives landed.** App-wide `--ts-shadow-glow` / `--ts-shadow-inset` strengthened to reference values. New tokens `--ts-glow-cyan{,-soft,-strong}`, `--ts-shadow-cockpit{,-hover,-selected}`, `--ts-cockpit-bg`, `--ts-cockpit-reticle`. New utility classes `.ts-cockpit-card` (with `::before` corner reticles, `::after` cyan halo blob, hover + `[data-selected]/.is-active` states, compact variant, light-theme overrides for corporate/pastel), `.ts-cockpit-label`, `.ts-cockpit-crumb`. CSS-only; vue-tsc clean. See completion log. |
| THEME-COCKPIT-1b | done 2026-05-16 | **Brain panel view port — completed via existing port + mood-driven palette swap.** Discovery found the brain panel already uses `.bp-*` classes (`.bp-shell`, `.bp-cockpit`, `.bp-module`, `.bp-prov`, etc.) backed by `src/styles/brain-panel.css` (1249 lines, aliased to `--ts-*` tokens). Cockpit hero already has the layered radial halo + corner-reticle `::before`. Gap that remained was mood-driven palette: ported the reference's `[data-accent="violet|green|amber"]` shell variants (plus `pink`) and `[data-backdrop="false"]` opt-out into `brain-panel.css`. Added `accentKey` computed in `BrainView.vue` that maps `moodKey` → `green` (free) / `violet` (paid) / `amber` (local) / `''` (none), bound on `.bp-shell[:data-accent="accentKey"]` so every cockpit border/glow/active-state cascades to the active brain mode without component edits. vue-tsc clean. |
| THEME-COCKPIT-1c | not-started | **Audit + spread the pattern.** Walk Knowledge Graphs, Settings, Memory, Chat root and the splash. For each large container, decide: adopt `.ts-cockpit-card`, leave as bare panel, or apply the lighter `--compact` variant. Goal is a consistent HUD feel without losing density. Tests: vitest snapshot updates as needed. |

---

## Phase GRAPHRAG — Cross-system brain comparison

Goal: study `microsoft/graphrag` (community-summary GraphRAG, global vs
local search) and adopt the top 1–3 concrete improvements that fit
TerranSoul's local-first, single-user constraints. **GRAPHRAG-1
research + comparison doc landed 2026-05-16 — see
[`docs/graphrag-comparison.md`](../docs/graphrag-comparison.md) and the
completion-log entry. Sub-chunks below carry the adoptions.**

| Chunk | Status | Scope |
|---|---|---|
| GRAPHRAG-1a | not-started | **Hierarchical community summaries.** Recurse `memory::graph_rag::detect_communities` so `memory_communities.level` carries levels 0..N (N capped at 4). Generate per-level LLM summaries through the active brain provider. New Tauri command `graph_rag_build_hierarchy`. Extend `graph_rag_search` to accept an optional `level` parameter and fetch community hits at that depth. Tests: multi-level detection, summary generation idempotency, level filter in search. See [`docs/graphrag-comparison.md`](../docs/graphrag-comparison.md) §5 row 1. |
| GRAPHRAG-1b | not-started | **Structured entity / relationship extraction at ingest.** New `memory::extraction::extract_entities_relationships(text, kind)` step in the ingest pipeline that calls the brain with a typed JSON-schema prompt (entity name + type + description, relationship src/dst/type/description), then materialises results as `memories` rows (cognitive_kind=`semantic`) + `memory_edges` rows. Behind a `BrainConfig.graph_extract_enabled` toggle (default off for offline-only sessions). Tests: schema validation, deduplication against existing entities, edge confidence assignment. See [`docs/graphrag-comparison.md`](../docs/graphrag-comparison.md) §5 row 2. |
| GRAPHRAG-1c | not-started | **Global vs Local query routing.** Extend the existing query-intent classifier (Chunk 16.6b) with a `scope ∈ {global, local, mixed}` axis. Route `global` queries to top-level community summaries (requires GRAPHRAG-1a), `local` queries to entity-walk + hybrid_search_rrf, and `mixed` queries to the current dual-level RRF fusion. Tests: scope classification accuracy on a fixture set, retrieval shape per scope. See [`docs/graphrag-comparison.md`](../docs/graphrag-comparison.md) §5 row 3. Depends on GRAPHRAG-1a. |

---

## Phase BENCH-SCALE — Combined retrieval-quality + scale bench

Goal: validate that LoCoMo R@10 survives when relevant docs are buried in a 1M-distractor corpus.

| Chunk | Status | Scope |
|---|---|---|
| BENCH-SCALE-3 | code-done, run-deferred | **IVF-PQ disk-backed bench.** Phase 3 code complete (codebook training + IVF-PQ build + ADC search path + `build_ivf_pq_indexes` Tauri command). Remaining: write a 10M-doc bench runner (none exists yet — `locomo-at-scale.mjs` uses HNSW via `longmemeval-ipc`, not IVF-PQ) and run it (~40h+ wall clock). |

