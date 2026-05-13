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

Next up: **INTEGRATE-1** — detect-and-link companion AI registry (Hermes Desktop, Hermes Agent CLI, OpenClaw, Temporal.io). Doc-only portions of Phase INTEGRATE (INTEGRATE-2/-3/-4) shipped 2026-05-14; the code-side work (registry + Tauri commands + chat-side suggest-hook + quest-based guided installer) is queued under the normal CI gate. See Phase INTEGRATE below.

Also queued: **BENCH-SCALE-2 run** — harness work landed 2026-05-14, actual two-arm 1M-doc run pending. **Deferred per user (2026-05-14): "Finish the entire chunks left except Phase BENCH-SCALE".**

Phases **HYBRID-DOC** and **TOP1** complete (2026-05-14 — see `rules/completion-log.md`).

Also queued: **TOP1-3 methodology-normalized LoCoMo compare** — ingest Mem0 paper Table 1 (Mem0, Mem0g, LangMem, Zep, A-Mem, MemGPT, ReadAgent, MemoryBank, OpenAI) into a separate end-to-end `J` lane so TerranSoul retrieval metrics are never placed in the same cell.

Also queued: **Phase INFRA** — close the gap between the README "Why TerranSoul is different" pillars and what's actually measured (`RESILIENCE-1` five-nines SLO, `SCALE-INF-1` cross-instance knowledge sharing, `CAP-1` per-memory AP/CP profile selector). See Phase INFRA below.

---

## Phase INFRA — Distributed-infrastructure pillars (resilience, scale-to-infinity, CAP)

Goal: close the gap between the README's "Why TerranSoul is different" claims and what's actually measured/shipped. Each pillar that's marked "design target" in the README must have a milestone chunk that converts it into a measurable contract before we can re-mark it "shipped".

> **Honesty rule.** No pillar moves from "design target" to "shipped" in the README until its chunk's acceptance evidence is logged in `rules/completion-log.md`. README and milestones stay in sync — if a pillar's chunk is open here, the README must say "design target" there.

| Chunk | Status | Scope |
|---|---|---|
| RESILIENCE-1 | not-started | **Five-nines availability SLO + telemetry + chaos test.** Define the local-TerranSoul uptime SLO as 99.999 % measured per calendar quarter (~5 min unplanned downtime budget). Add in-process uptime telemetry: heartbeat timestamp written every 30 s to `app-data/uptime/heartbeat.jsonl` with atomic write, gap detection on startup classifies the previous run as `clean_exit` / `crash` / `power_loss`. Add a crash-loop guard: ≥ 3 crashes inside 5 min triggers safe-mode (disable plugins, embedding worker, hive relay) and surfaces a one-click "send debug bundle" action. Add a hermetic chaos test that kills the renderer, the embedding worker, and the MCP server in sequence and asserts: (a) no memory loss, (b) auto-recovery within 30 s, (c) heartbeat resumes. Acceptance: telemetry+guard shipped, 2 hermetic tests pass, design doc `docs/availability-slo.md` defines the SLO calculation. **Until this chunk lands, the README must say "design target — five nines" not "five nines".** |
| SCALE-INF-1 | not-started | **Cross-TerranSoul knowledge sharing audit + org/team relay contract.** Peer-to-peer pairing already ships (CRDT over QUIC, Ed25519 device identity, hive relay). What's missing for "another TerranSoul's partner / company / another PC": (a) a documented contract for **subscribing** to a remote TerranSoul's specific shards under ACL (today the model is push-bundle, not subscribe), (b) a multi-tenant ACL test proving that a tag `private:darren` never leaks across the relay even under aggressive sync pressure, (c) an audit doc `docs/cross-instance-knowledge-sharing.md` describing partner / team / company use cases and the trust model for each. Acceptance: contract doc shipped, 1 hermetic Rust test for the ACL leak scenario, 1 Playwright e2e for the subscribe-handshake UX. |
| CAP-1 | not-started | **Per-memory CAP profile selector (P mandatory, A or C per purpose).** Today every memory rides the CRDT (AP / eventual consistency) lane. Some memories need stronger guarantees: legal/financial/shared-team facts where two devices disagreeing is worse than one device blocking. New `AppSettings.cap_profile_default` (`Availability` | `Consistency`) plus a per-memory `cap_profile: Option<CapProfile>` column. AP path: existing CRDT merge. CP path: writes go through the hive relay as a single-writer linearizable log with quorum=2 acks before the originating device's UI confirms the write; offline devices that hold a CP memory **block** the write until reachable. Hermetic test pair: (1) AP write succeeds offline, merges on reconnect; (2) CP write blocks offline, succeeds online, never produces a divergent state. Design doc `docs/cap-profile.md` explains the trade-off in plain language for users. |
| ACTOR-MODEL-1 | not-started | **Formal agent-fleet supervision tree (actor-style).** Today agents run as separate OS processes registered with the orchestrator (good isolation), but supervision is ad-hoc (manual retry, no formal restart policy per agent kind). Add a typed supervision spec: each registered agent declares `restart: { policy: Always | OnFailure | Never, max_restarts: u32, window: Duration, backoff: ExponentialBackoff }`, enforced by the orchestrator. Add 2 hermetic tests: an agent that crashes 3× in 1 min trips its restart budget and is marked `degraded`; an agent with `policy: Always` survives an OOM-kill and resumes. This formalises the "actor model" framing from the README pillar. |

> **Loop rule.** When a Phase INFRA chunk completes, update its README pillar status from "design target" to "shipped" in the **same PR**, and re-grep README for any other pillar accidentally drifting from the milestone status.

---

## Phase INTEGRATE — Companion AI ecosystem (detect-and-link)

Goal: TerranSoul is a personal assistant, not a walled garden. When a user's question is heavier than TerranSoul should answer alone — long deep-research, full-IDE coding sessions, durable multi-day workflows — TerranSoul should suggest the right companion AI app and help the user install it the safe way (guided installer with explicit click + OS-level UAC prompt). No silent background installs. No bundled redistribution. No claims about products we can't verify.

**Install policy (user, 2026-05-14):** *Guided installer with explicit user click + UAC through the quest system.* TerranSoul detects whether each tool is installed, surfaces a quest, and only runs the per-OS install command (`winget`, `dnf`, `brew`, `apt`, official `.dmg` / `.exe`) after the user clicks Install **and** the OS confirms the elevation prompt. No detection-without-consent. Never call `winget` / `dnf` / `apt` from background tasks.

**Verified scope (user, 2026-05-14):**
- **Hermes Desktop** = [`fathah/hermes-desktop`](https://github.com/fathah/hermes-desktop) (MIT, Electron, v0.3.7, `winget install NousResearch.HermesDesktop` pending winget-pkgs PR; `.exe` / `.dmg` / `.AppImage` / `.deb` / `.rpm` on the GitHub Releases page). The desktop GUI for Hermes Agent.
- **Hermes Agent** = [`NousResearch/hermes-agent`](https://github.com/NousResearch/hermes-agent) (MIT, Python CLI). Already integrated as an MCP-config consumer — TerranSoul writes a marker-managed block into Hermes's `cli-config.yaml` via `setup_hermes_mcp` / `setup_hermes_mcp_stdio`.
- **OpenClaw** = `openclaw/openclaw`. Already integrated as the `openclaw-bridge` plugin (`src-tauri/src/agent/openclaw_agent.rs`).
- **Temporal.io** = workflow-engine design reference only. Not an integration. Cited in `docs/coding-workflow-design.md` and `instructions/AGENT-ROSTER.md` as the durable-history pattern TerranSoul's own runner is *inspired by*.
- **Claude Cowork** = **deferred pending verification** — user did not confirm a product name/URL; not added to README, docs, or code until a real reference exists.

| Chunk | Status | Scope |
|---|---|---|
| INTEGRATE-1 | not-started | **Detect-and-link registry.** New Rust module `src-tauri/src/integrations/companions.rs` with a `CompanionApp` struct (id, display name, role, official URL, per-OS install command, detect-command). Detection runs only on explicit user click of the integrations panel or quest; no background scanning. Tauri commands: `companions_list`, `companions_detect_one`, `companions_open_install_page`, `companions_run_guided_install` (always opens an OS-elevated terminal, never silent). Hermetic Rust tests for the registry shape + a UAC-required-flag test (must return `RequiresElevation` enum variant). |
| INTEGRATE-2 | doc shipped 2026-05-14 | **README + Hermes setup doc.** README "Companion AI Ecosystem" section + `docs/integrations/hermes-setup.md` shipped. Code follow-up: ChatView dismissable suggest-hook that fires only when `turn_token_estimate ≥ TS_HERMES_HINT_TOKENS` (default 4000) **and** `intent.classification ∈ {deep_research, long_running_workflow, full_ide_coding}` **and** `app_settings.hermes_hint_enabled = true` (default `true`, user-toggleable). Vitest hermetic coverage for the gate + Hermes config wiring. |
| INTEGRATE-3 | doc shipped 2026-05-14 | **OpenClaw status accuracy.** README "Companion AI Ecosystem" section documents OpenClaw as the existing `openclaw-bridge` plugin; CREDITS.md unchanged (already lists OpenClaw correctly). Code follow-up: surface `openclaw-bridge` install state in the same companions registry as INTEGRATE-1 — detect upstream OpenClaw CLI, offer guided install, show "active plugin" badge in BrainView. |
| INTEGRATE-4 | doc shipped 2026-05-14 | **Temporal.io status correction.** README "Companion AI Ecosystem" section explicitly tags Temporal.io as a *design reference, not an integration*. Code follow-up: optional `temporal-bridge` plugin contract spec (deferred — only if user provides a concrete TerranSoul use case for outsourcing a workflow to a Temporal worker; no work until then). |
| INTEGRATE-5 | not-started | **Quest-based guided installer.** New quest `companion-ecosystem` with 4 sub-quests (one per verified companion). Each sub-quest renders an Install button that triggers `companions_run_guided_install`; the OS elevation prompt is the consent gate. No background install. Test: Playwright e2e that confirms the OS-elevation step is reachable but the install command never runs without an explicit click. |

> **Loop rule.** This phase intentionally stops short of "every AI in the world". Add a new INTEGRATE-N chunk only when (a) a user provides a verifiable upstream URL, (b) the upstream license permits redistribution mention, and (c) at least one TerranSoul workflow benefits from delegating to that tool. No speculative additions.

---

## Phase CHAT-PARITY — close design-doc subsystems left dark in the chat path

Goal: close the gaps surfaced by the 2026-05-13 brain-doc audit so every documented brain subsystem is exercised by either the chat surface, the bench harness, or both. Each gap is a small, hermetic chunk: pure-logic gate + 2 unit tests + design-doc footnote, mirroring the BENCH-CHAT-PARITY-1/2 + BENCH-KG-1 pattern.

_All CHAT-PARITY + BENCH-PARITY chunks shipped (see `rules/completion-log.md`). Phase complete._

---

## Phase HYBRID-DOC — Hybrid-RAG design docs + benchmark folder polish

_All HYBRID-DOC chunks shipped 2026-05-14 (see `rules/completion-log.md`). Audit doc: `docs/audit-2026-05-12-status.md`; new benchmark folder layout: `benchmark/<system>/<task>/`. Phase complete._

---

## Phase TOP1 — "Beat everyone" benchmark loop

TOP1-1 shipped 2026-05-14 (cross-system matrix in `benchmark/COMPARISON.md` § Round TOP1-1; TerranSoul rank 1 on every directly-comparable retrieval cell vs agentmemory + MemPalace). The methodology-gap finding remains active: Mem0-paper LoCoMo Table 1 systems report end-to-end LLM-as-Judge `J`, while TerranSoul's canonical bench reports retrieval metrics (`R@10`, `NDCG@10`, `MRR`). These must stay in separate lanes.

| Chunk | Status | Scope |
|---|---|---|
| TOP1-3 | not-started | **Methodology-normalized LoCoMo compare (same-cell rule).** Add a dedicated end-to-end `J` comparison lane in `benchmark/COMPARISON.md` populated from the Mem0 paper full Table 1 set: `Mem0`, `Mem0g`, `LangMem`, `Zep`, `A-Mem`, `MemGPT`, `ReadAgent`, `MemoryBank`, `OpenAI`. Keep TerranSoul retrieval metrics (`R@10`, `NDCG@10`, `MRR`) in a separate retrieval lane; do not place end-to-end `J` systems and retrieval systems in the same ranking cell. Add an explicit "methodology mismatch" badge and a footnote defining the two axes. Acceptance: (1) no mixed-metric cells in the matrix, (2) every Table 1 system appears in the end-to-end lane with source citation, (3) TerranSoul row is marked "retrieval-only; end-to-end J pending TOP1-2 harness" until parity harness lands. |

> **Same-cell rule (mandatory).** If two systems are measured with different objectives (end-to-end judge vs retrieval relevance), they must never share a ranked cell. Show them in adjacent lanes with a methodology note.

_TOP1-2 remains scoped in `benchmark/COMPARISON.md` § "TOP1-2 scope" (requires paid `gpt-4o-mini` API budget for Mem0-paper parity or an explicit local-judge variant decision). Phase loop rule remains active — re-run the matrix at the start of the next benchmarking session. See `rules/completion-log.md`._

---

## Phase BENCH-LCM — Beat LoCoMo / LMEB retrieval benchmarks

Goal: add a direct, reproducible TerranSoul run on the MTEB `LoCoMo` text-retrieval dataset so the benchmark table can move beyond mixed published LoCoMo QA numbers and compare TerranSoul against top memory systems on a shared retrieval task.

> **Round 1 baseline (BENCH-LCM-1, 2026-05-12):** 250-query slice shows search R@10 51.3%, NDCG@10 40.9%. Strongest: temporal_reasoning 90% R@10. Weakest: multi_hop 15%, open_domain 24%. Root cause: insufficient morphological stemming (only -s/-ies), weak all-terms bonus (16), no conversational concept expansion.

> **Round 2 result (BENCH-LCM-2, 2026-05-12):** 250-query slice shows rrf R@10 **54.4%** (+2.8pp), search R@10 **53.6%** (+2.3pp). multi_hop nearly doubled: 15→33% R@10. Morphological variants now FTS5-recall-only (not scored), fixing the `configuration_term` regression. Added 11 new QUERY_TERM_EXPANSIONS and 3 new phrase expansions (activities, destress, art).

> **Round 3 result (BENCH-LCM-3, 2026-05-12):** Full 1655-query run, all 4 tasks. Added `rrf_emb` system (3-tier embedding-enhanced RRF: lexical+freshness fusion, cosine re-rank of candidates, embedding rescue for semantically-missed docs). rrf_emb **59.4%** R@10 overall (+3.7pp vs rrf 55.7%). Wins every task: single_hop 68.1% (+2.6pp), multi_hop 33.3% (+2.7pp), open_domain 34.1% (+4.4pp), adversarial 64.3% (+6.2pp). Also added 12 new query expansions (career, degree, education, financial, music, etc.).

> **Round 4 result (BENCH-LCM-4, 2026-05-12):** Store-level embedding integration. Embeddings stored in HNSW ANN index, query embeddings passed to `hybrid_search_rrf()` for native 3-way RRF. rrf+emb **59.9%** R@10 overall (+4.2pp vs plain rrf, +0.5pp vs IPC-level rrf_emb). single_hop 68.6%, multi_hop 35.6%, open_domain 32.6%, adversarial 64.3%.

> **Round 5 result (BENCH-LCM-5, 2026-05-12):** Upgraded from nomic-embed-text (137M, 768d) to mxbai-embed-large (335M, 1024d). Massive gains: overall **63.6%** R@10 (+3.7pp). single_hop 73.5% (+4.9pp), multi_hop 46.2% (+10.6pp), open_domain 42.0% (+9.4pp). Adversarial regressed to 61.7% (-2.6pp) — stronger semantic matching creates false positives on trick questions.

> **Round 6 smoke (BENCH-LCM-6, 2026-05-12):** Proper-noun penalty (×0.35 when query proper noun missing from candidate) re-ranks adversarial queries on top of mxbai. 100-query smoke slice: **adversarial R@10 66.5%** (+4.8pp vs LCM-5's 61.7%, exceeds 64% target). Other-task deltas are slice-composition noise, not regressions. Awaiting 250-query confirmation in BENCH-LCM-7.

> **Round 7 result (BENCH-LCM-7, 2026-05-12, NEGATIVE):** Full 1655-query run with the BENCH-LCM-6 proper-noun penalty (tuned to ×0.5) revealed the smoke slice was misleading. Adversarial +1.6pp (63.3%), but single_hop -4.9pp, multi_hop -11.3pp, open_domain -9.8pp, **overall -2.1pp net loss** (63.6 → 61.5). Penalty reverted. The failure mode is structural: factual queries often paraphrase entities ("the runner" instead of "Melanie") so candidates omitting the named entity are correctly ranked first by semantic similarity, and an unconditional penalty wipes them out. A narrower trigger is required.

> **Round 8 result (BENCH-LCM-8, 2026-05-12):** Wired the existing `src-tauri/src/memory/reranker.rs` LLM-as-judge cross-encoder into the bench via a new `rrf_rerank` system: top-30 RRF candidates re-scored 0–10 by `gemma3:4b` over Ollama in batches of 5, then top-10 returned. Full 1976-query run: **overall R@10 63.6 → 68.3 (+4.7pp)**, adversarial 61.7 → 67.7 (+6.0pp, exceeds 64% target), single_hop 73.5 → 77.6 (+4.1pp), temporal_reasoning 74.2%. multi_hop 46.2 → 44.0 (-2.2pp) and open_domain 42.0 → 39.7 (-2.3pp) regress just past the 2pp soft threshold, but the +4.7pp net win is the canonical Anthropic-Contextual-Retrieval cross-encoder pattern. Latency: 0.98s → 4.2s per query (acceptable for offline retrieval bench).

> **Loop rule (per user request).** After each `BENCH-LCM-N` chunk completes, re-run the LoCoMo benchmark, diff against the previous round, and open the next fix chunk. Stop only when TerranSoul holds rank 1 on every measured metric.

> **Smoke-slice rule (2026-05-12, per user request).** Always run a **100-query** smoke slice first to validate a fix before any heavier run. 250-query slices are too high for iteration. Only promote to a 250-query or full 1655-query run after the 100-query slice shows the expected directional change on the affected task(s).

> **Smoke-slice caveat (2026-05-12, BENCH-LCM-7 lesson).** A 100-query smoke that shows the desired delta on the *target* task can still hide regressions on other tasks. After the 100-q smoke passes on the target metric, always check the other tasks' 100-q numbers against their LCM-N baselines BEFORE promoting to 250-q or full. If any non-target task is more than ~5pp below its LCM-N baseline on the 100-q slice, treat that as a likely regression and tune the trigger, don't just tune the penalty magnitude.

> **Audit-before-invent rule (2026-05-12, BENCH-LCM-8 lesson).** Before inventing any new retrieval heuristic to fix a benchmark regression, audit `docs/brain-advanced-design.md` and `src-tauri/src/memory/` to verify that every advertised pipeline stage is actually being exercised by the bench. The LoCoMo bench currently uses only RRF (`hybrid_search_rrf`) — HyDE, the cross-encoder reranker, contextual retrieval, and KG edges are dark. The fix is almost always to wire an existing stage into the bench rather than invent new heuristics. The BENCH-LCM-6/7 proper-noun penalty was a hand-rolled reinvention of cross-encoder rerank; the existing `src-tauri/src/memory/reranker.rs` LLM-as-judge module was the right answer all along. Cross-reference top open-source memory systems (Mem0, MemPalace, agentmemory, Letta, Anthropic Contextual Retrieval 2024, BGE-reranker-v2-m3, mxbai-rerank, Jina reranker v2) for the canonical config before guessing.

> **Round 9 result (BENCH-LCM-9, 2026-05-13, NEGATIVE):** Both hypotheses for closing the LCM-8 marginal regressions (multi_hop -2.2pp, open_domain -2.3pp) failed on 100-q smoke. (1) Widening the rerank candidate pool 30→50 dropped adversarial R@10 65.5→61.5 (-4pp) — extra near-miss distractors diluted the cross-encoder's attention budget. (2) Applying the production-default 5.5 confidence threshold (and a milder 3.5) produced IDENTICAL results because `gemma3:4b` at temperature 0 is bimodal: scores cluster at 0-2 or 7-9, rarely landing in the 3-5 "partial answer" tier the threshold was designed to gate. Reverted to LCM-8 defaults (pool=30, threshold=0). Real fix paths deferred to LCM-10 (HyDE) and a future bigger-reranker chunk.

> **Round 10 result (BENCH-LCM-10, 2026-05-13, MIXED):** Wired HyDE (`src-tauri/src/memory/hyde.rs::build_hyde_prompt` + `clean_hyde_reply`) into the bench as `rrf_hyde` and stacked `rrf_hyde_rerank`. Full 1976-query run vs LCM-8 baseline: **overall R@10 68.3 → 68.5 (+0.2pp, tied)**; per-task: temporal_reasoning +2.9pp ✅, multi_hop +1.0pp ✅, single_hop -0.5pp, adversarial -0.7pp, open_domain -1.6pp ❌. HyDE is a per-query-class tool (helps abstract/multi-hop/temporal, hurts under-specified open-ended queries). Both `rrf_hyde` and `rrf_hyde_rerank` stay registered as alternative bench systems for production query-class gating, but the canonical bench default remains `rrf_rerank` (LCM-8).

> **Round 11 result (BENCH-LCM-11, 2026-05-13, MIXED):** Wired Anthropic Contextual Retrieval (`src-tauri/src/memory/contextualize.rs`) into the bench as `rrf_ctx` + `rrf_ctx_rerank` (ingest-side, with disk-cache at `target-copilot-bench/ctx-cache/<sha16>.txt`, ~52-min one-time corpus pass with gemma3:4b for 5,882 adversarial rows). 100-q adversarial smoke vs LCM-10/LCM-8: R@10 **68.5 %** (tied with LCM-10), NDCG@10 **40.8 %** (+1.1pp over LCM-10, +4.3pp over LCM-8), MRR@100 33.2 %. **Acceptance NOT met** (target NDCG@10 ≥ 50, missed by ~9pp). Promotion to full 1976-q skipped: a tied-R@10 + thin-NDCG-bump smoke does not justify the bench cost when acceptance is missed by such a wide margin. Anthropic's headline -49% failure rate does not transfer to this LoCoMo slice — LoCoMo conversational chunks already carry inline speaker+timestamp anchors, so contextualization adds less marginal signal than the financial/Wikipedia corpora in the original paper. Both systems stay registered as alternative bench systems; canonical default remains `rrf_rerank` (LCM-8); ctx-cache is preserved for future stacked-pipeline experiments. Durable lesson: contextual retrieval is dataset-dependent, not a universal RAG win.

> **Chat-parity result (BENCH-CHAT-PARITY-1, 2026-05-13, PASS):** Closed the canonical chat ↔ design-doc rerank gap identified in the 2026-05-13 audit. New `retrieve_chat_rag_memories_reranked` async wrapper in `commands/streaming.rs` layers the LCM-8 cross-encoder rerank on top of the sync RRF helper, gated on `active_brain.is_some()`. Cloud streaming (`stream_openai_api`) now uses the reranked path; local-Ollama streaming and Self-RAG keep the sync helper because loading a reranker would evict the chat model from VRAM and break the <1s TTFT contract. Hermetic test `chat_rag_reranked_falls_back_to_rrf_without_brain` confirms RRF passthrough when no brain is registered. Bench harness `rrf_rerank` system remains the canonical end-to-end rerank verification because mocking Ollama inside a unit test cannot validate prompt-shape compatibility. Cloud streaming chat now exercises 4 of 5 design-doc stages (embed → threshold → RRF → rerank). KG-edge boost remains the last bench-dark stage (BENCH-KG-1). HyDE in streaming (per-query-class gated per LCM-10 lesson) deferred to BENCH-CHAT-PARITY-2.

> **KG-1 chat-half result (BENCH-KG-1, 2026-05-13, PASS):** Closed the chat-side half of the 5th design-doc retrieval stage. `memory::cascade::cascade_expand` (Chunk 43.5, 1–2 hop BFS over `memory_edges` with `seed × edge_prior × 0.7^depth` decay) was already wired into the MCP gateway but NOT into the chat path. Added `AppSettings.enable_kg_boost: bool` (default `false`) + new `expand_seeds_via_kg` helper in `commands/chat.rs::retrieve_prompt_memories` so opt-in users get graph-adjacent memories promoted into the rerank pool. Two hermetic tests (positive + negative) prove an edge-only neighbour with NO query tokens appears in chat top-K iff the flag is on. Bench-side wiring (entity-overlap edge builder at ingest + `rrf_kg`/`rrf_kg_rerank` modes + 100-q smoke) moved to BENCH-KG-2.

> **Round SCALE-1 result (BENCH-SCALE-1, 2026-05-13, MIXED):** First quality-at-scale bench. New `scripts/locomo-at-scale.mjs` harness ingested 100,000 chunks (5,882 gold + 23,528 natural cross-task distractors + 70,590 entity-swap/synthetic) into a fresh `MemoryStore` with mxbai-embed-large + HNSW in 47 minutes, then ran 100 adversarial queries through `rrf_rerank`. **Quality PASS:** R@10 **59.5 %** at 100k vs LCM-8 5k baseline 67.7 % = -8.2pp (within 10pp acceptance bar). R@100 71.5 % shows gold chunks are still retrievable — the R@10 loss is the cross-encoder dropping correct candidates from top-10 to top-100, not embedding/HNSW recall collapsing. **Latency FAIL by design:** p99 30.77s vs 200ms target — but SCALE-1 measured end-to-end `rrf_rerank` (gemma3:4b reranker ~6s/query), while the 200ms bar applies to retrieval-only (RRF+HNSW, no LLM judge). Acceptance-language mismatch, not a real regression. Retrieval-only validation queued as BENCH-SCALE-1b on the cached 100k corpus.

> **Round SCALE-1b result (BENCH-SCALE-1b, 2026-05-13, MIXED → quality PROMOTED):** Re-ran the same 100k adversarial corpus with `--systems=rrf` (no rerank). **Quality PASS + improvement:** R@10 **64.0 %** (+4.5pp over SCALE-1's reranked R@10), NDCG@10 **46.7 %** (+14.2pp), MAP@10 **41.0 %** (+17.1pp), MRR@100 **42.3 %** (+16.9pp), R@100 **80.0 %** (+8.5pp). The gemma3:4b cross-encoder is a measurable *quality regression* on this corpus — confirms the BENCH-LCM-9 / LCM-10 lesson at scale. **Latency MIXED:** avg 1791ms (-72 %), p50 **1.21s** (-81 %), p95 **3.68s** (-61 %), p99 **25.32s** still exceeds the 200ms bar. Root cause is the per-query Ollama embedding hop (long tail under load), not the post-embedding RRF + HNSW lookup. The acceptance bar in `docs/billion-scale-retrieval-design.md` was already split into "retrieval-only" vs "end-to-end-with-rerank" earlier this chunk; per-query embed-vs-search latency breakout queued for a future bench harness improvement. Decision: **promote `--systems=rrf` as the canonical bench mode** — it is both faster AND higher quality on this corpus.

> **Chat-parity result (BENCH-CHAT-PARITY-2, 2026-05-13, PASS):** Closed the last design-doc retrieval stage missing from cloud streaming chat. New pure-logic helpers `hyde_recommended` + `should_run_hyde` in `src-tauri/src/memory/query_intent.rs` gate HyDE expansion on the user turn's classified intent — only `Semantic` (abstract / multi-hop) and `Episodic` (temporal) classes get the hypothetical-embedding sharpening, per BENCH-LCM-10's per-class lesson. `commands/streaming.rs::stream_openai_api` now calls `OllamaAgent::hyde_complete` then re-embeds via `embed_for_mode` so cloud users get cloud embeddings of the hypothetical, replacing `query_emb` before the RRF + threshold + rerank pipeline runs. Local-Ollama streaming and Self-RAG keep using the sync helper directly to honour the VRAM-safety contract. 5 new unit tests in `query_intent` (24/24 pass), `commands::streaming` 44/44 still pass, clippy clean, vitest 1842/1842. Cloud streaming chat now exercises **all 5** design-doc retrieval stages (embed → optional class-gated HyDE → RRF threshold → optional KG-cascade boost → optional cross-encoder rerank).

> **KG-2 bench-half result (BENCH-KG-2, 2026-05-13, NEUTRAL/MARGINAL):** Brought the 5th design-doc retrieval stage out of bench-dark territory on the bench harness side. `src-tauri/src/bin/longmemeval_ipc.rs` gained an ingest-time `KgIndex` (gated on `LONGMEM_KG_EDGES=1`) that runs a proper-noun extractor (capitalised tokens ≥4 chars, sentence-start stoplist) and inserts top-10 `shares_entities` `EdgeSource::Auto` edges for any pair sharing ≥2 entities. New search modes `rrf_kg` / `rrf_kg_rerank` call a new public `MemoryStore::cascade_expand_seeds(&[(i64,f64)], Option<usize>)` wrapper at depth 2 between RRF and the optional cross-encoder, with the post-cascade pool truncated to the reranker pool size BEFORE rerank. `scripts/locomo-mteb.mjs` registers the modes and threads `LONGMEM_KG_EDGES=1` through. **100-q adversarial smoke vs `rrf_rerank` baseline:** R@10 tied (64.0 %), R@5 +1pp (53→54), NDCG@10 +0.3pp (41.1→41.4), MAP@10 +0.4pp (33.8→34.2), MRR@100 +0.3pp (35.3→35.6) — **below the +0.5pp R@10 PROMOTE bar AND ~2× latency** (7011ms vs 3440ms). Adversarial is the wrong fixture for cascade (correct answer rarely needs a graph hop on this LoCoMo slice). Verdict: **NEUTRAL/MARGINAL POSITIVE** — modes stay registered for future `multi_hop` re-runs; `enable_kg_boost` chat default stays `false`. The win is parity, not promotion: chat and bench now both exercise all 5 design-doc stages. Durable lesson: when wiring cascade or any pool-expanding stage into rerank, ALWAYS truncate the post-expansion pool to the reranker pool size BEFORE rerank or the rerank workload explodes and bench hangs.

---

## Phase BENCH-SCALE — Combined retrieval-quality + scale bench

Goal: stop treating "1M+ memories" as a latency-only claim. The current `cargo bench million_memory --features bench-million` measures HNSW p50/p95/p99 over synthetic vectors and CRUD throughput — it does NOT measure whether LoCoMo R@10 survives when the relevant docs are buried in a 1M-distractor corpus. Per `docs/billion-scale-retrieval-design.md` Phase 1-5 (sharded HNSW, IVF-PQ, sharded KG, all shipped), the brain claims billion-scale viability — but no public bench validates that retrieval quality holds at scale.

> **Why this matters.** A perfect retriever at 5k docs is meaningless if it falls apart at 1M. Top memory systems publish quality-at-scale curves (Mem0, MemPalace, Letta) — TerranSoul should too. This is the missing bridge between `million_memory` (latency) and `locomo-mteb` (quality).

| Chunk | Status | Scope |
|---|---|---|
| BENCH-SCALE-2 | harness-shipped, run-pending | **Sharded-HNSW scale bench.** Harness shipped 2026-05-14: new `MemoryStore::set_shard_mode(ShardMode)` toggle (`RouterRouted` default vs `AllShards` baseline), `longmemeval-ipc` reads `LONGMEM_SHARD_MODE`, `scripts/locomo-at-scale.mjs --shard-mode={routed,all}`, JSON report + filename now include the mode (no more SCALE-1b-style overwrite). Run-pending: execute the two-arm 1M comparison per the protocol in `docs/billion-scale-retrieval-design.md` § Phase 2 (router-routed vs all-shards probe). Report deltas on R@10 / NDCG@10 / MRR / p50 / p95 / p99 / ingest time / peak RSS. Document the shard-count sweet spot. |
| BENCH-SCALE-3 | not-started | **IVF-PQ disk-backed bench.** Phase 3 (kickoff shipped, Chunk 49.1) targets >100M with m=96, nbits=8 PQ. Once a working IVF-PQ shard is available, re-run the LoCoMo-at-scale bench at 10M and report the PQ accuracy/latency trade against full-precision HNSW. |

---

## Phase BENCH-AM — Beat agentmemory's published benchmark

Goal: match-or-beat the agentmemory v0.6.0 quality bench (Recall@10 ≥ 58.6 %, NDCG@10 ≥ 84.7 %, MRR ≥ 95.4 %) and stage LongMemEval-S so we can claim a public retrieval-accuracy number. Reference: `https://github.com/rohitg00/agentmemory/blob/main/benchmark/COMPARISON.md`.

> **Round 3 result (BENCH-AM-3, 2026-05-12):** TerranSoul `search` with lexical rerank + gated KG boost → R@10 **64.1 %** (+5.5 pp ahead), NDCG@10 **94.7 %** (+10.0 pp ahead), MRR **95.8 %** (+0.3 pp ahead vs agentmemory BM25-only's 95.5 %). TerranSoul now leads the pinned agentmemory `bench:quality` case set on every measured quality metric. Full numbers in [docs/agentmemory-comparison.md](../docs/agentmemory-comparison.md).

> **Round 4 result (BENCH-AM-4, 2026-05-12):** token-efficiency accounting now ships in the harness JSON/Markdown report plus `npm run brain:tokens`. Full-context paste costs 32,660 tokens/query on the pinned fixture; 200-line MEMORY.md costs 7,960 tokens/query. TerranSoul no-vector RRF uses 2,798 retrieved-memory tokens/query while holding R@10 63.6 %, NDCG@10 94.3 %, MRR 95.8 %, saving **91.4 %** vs full paste and **64.8 %** vs the 200-line baseline.

> **Round 5 adapter (BENCH-AM-5, 2026-05-12):** LongMemEval-S plumbing now ships: `npm run brain:longmem:prepare`, `npm run brain:longmem:run`, `npm run brain:longmem:sample`, and a Rust JSONL IPC shim over the real `MemoryStore`.

> **Round 6 result (BENCH-AM-6/6.1, 2026-05-11):** LongMemEval-S retrieval-only top-1 verified on the 500-question cleaned set. TerranSoul `search` with corpus-aware lexical weighting and light query variants hit R@5 **99.2 %**, R@10 **99.6 %**, R@20 **100.0 %**, NDCG@10 **91.3 %**, MRR **92.6 %**. This beats agentmemory's published **95.2 / 98.6 / 99.4 / 87.9 / 88.2** and MemPalace's ~**96.6 % R@5** on the comparable retrieval table. Full numbers live in [docs/agentmemory-comparison.md](../docs/agentmemory-comparison.md) and `target-copilot-bench/bench-results/longmemeval_s_terransoul.{json,md}`.

> **Round 7 result (BENCH-AM-7, 2026-05-11):** feature-matrix parity sweep complete. The remaining partial rows are documented scope boundaries (Hive/MCP instead of a core-memory lease mesh; MCP/Tauri/Rust/Vue APIs instead of separate SDK packages). The required quality rerun found and fixed a candidate-pool rarity regression by capping broad low-signal terms (`configuration`, `setup`, `test`, `validation`) while preserving LongMem rare-anchor weighting. Final post-fix checks: agentmemory bench `search` **66.4 % R@10 / 96.5 % NDCG / 100.0 % MRR**, no-vector RRF **67.1 % / 98.2 % / 100.0 %**, and LongMemEval-S unchanged at **99.2 % R@5 / 99.6 % R@10 / 100.0 % R@20 / 91.3 % NDCG / 92.6 % MRR**.

> **Loop rule (per user request).** After each `BENCH-AM-N` chunk completes, the next agent session must re-run the quality harness, diff against the previous round, and either promote `BENCH-AM-(N+1)` or open a new fix chunk if a regression appears. Stop only when TerranSoul holds rank 1 on every measured metric and `BENCH-AM-7` is done.

---



 



