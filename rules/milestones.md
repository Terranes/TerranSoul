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
> See `rules/prompting-rules.md` → "ENFORCEMENT RULE — Clean Up Reverse-Engineering Research on Chunk Completion".

> **Completed work lives in [`rules/completion-log.md`](completion-log.md).**
> Do not re-list done chunks here. Phases 0–11 (Foundation through RPG Brain
> Configuration), Chunks 1.2 / 1.3 / 1.4 / 1.5 / 1.6 / 1.7 / 1.8 / 1.9 / 1.10 / 1.11,
> the Phase 9 Learned-Features batch, and all Post-Phase polish are recorded
> there in reverse-chronological order.

---

## Next Chunk

_Phase 13 (2.1 → 2.4) shipped 2026-04-24. Phase 14 main-chain
(14.1 + 14.2) shipped 2026-04-24. Chunk 14.7 (persona pack export /
import) shipped 2026-04-24._ Remaining Phase 14 work is the
camera-driven side chain (14.3 / 14.4 / 14.5), the persona-storage +
self-learning chunks (14.8 → 14.11) and the optional offline-polish
research chunks (14.12 → 14.15). **Phase 15** (AI Coding Integrations
— MCP + gRPC) is in progress (15.3 landed; 15.1 / 15.2 / 15.4–15.8
pending). **Phases 16 / 17** land the remaining items from
`docs/brain-advanced-design.md` § 16 (Modern RAG, Phase-5
Intelligence). **Phase 18** is fully complete (18.1–18.5 all shipped).
**Chunk 17.3** (temporal reasoning queries) shipped 2026-04-25.
**Chunk 16.2** (Contextual Retrieval) and **Chunk 16.12** (Memory
versioning / V8 schema) shipped 2026-04-25. **Chunk 17.4** (importance
auto-adjustment) and **Chunk 16.11** (semantic chunking pipeline)
shipped 2026-04-26. Pick the next item from
the active tables below or from `rules/backlog.md`.

---

## Active Chunks

### Phase 12 — Brain Advanced Design (Documentation & QA)

| # | Chunk | Status | Owner | Notes |
|---|---|---|---|---|
| 1.1 | Brain Advanced Design — Validation, Docs Rewrite, QA Walkthrough | in-progress | agent + user (screenshots) | Source tracking + cross-framework comparison table done; user-captured screenshots remain |

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
### Phase 14 — Persona, Self-Learning Animation & Master-Mirror

> Architectural reference: **[`docs/persona-design.md`](../docs/persona-design.md)**.
> Privacy contract: per-session/per-chat camera consent only; webcam
> frames never cross the IPC boundary.

| # | Chunk | Status | Notes |
|---|---|---|---|
| 14.3 | Persona-side camera quest **`expressions-pack`** — `useCameraCapture.ts` per-session consent composable + `face-mirror.ts` (lazy-loaded `@mediapipe/tasks-vision` FaceLandmarker → ARKit-blendshape → VRM expression mapper) + `PersonaTeacher.vue` "Teach an expression" panel. Activation gate already wired via `persona.learnedExpressions.length > 0`. Must ship `<add @mediapipe/tasks-vision>` dependency, the consent dialog, the always-visible "Camera live" badge, the idle-timeout/chat-change auto-stop, and unit tests on the pure mapper. | not-started | Camera permission MUST be per-session; no on-disk "always on" flag. |
| 14.4 | Persona-side camera quest **`motion-capture`** — `pose-mirror.ts` PoseLandmarker wrapper (33-keypoint → VRM humanoid bone retargeting; pure retargeter is the unit-tested seam) + record-and-name-clip UI in `PersonaTeacher.vue`. Activation gate already wired via `persona.learnedMotions.length > 0`. | not-started | Reuses the same per-session consent flow as 14.3. |
| 14.5 | VRMA baking — convert a recorded learned-motion clip into a VRMA file so the avatar can replay it through the existing `VrmaManager` instead of always streaming landmarks. | not-started | Reduces per-frame cost and unlocks sharing learned motions between devices via the existing Soul Link sync surface. |
| 14.8 | **Persona drift detection** — `auto_learn` evaluator gains a `persona_drift_check` step: every N (default 25) memorised facts, the existing extractor compares the latest `personal:*` cluster against the active `PersonaTraits`, and surfaces a chat-side suggestion ("Echo noticed you've shifted toward …; update persona?"). Pure additive on top of `memory::auto_learn::evaluate` — no new background loop. Maps to `docs/persona-design.md` § 15.1 row 143. | done | Reuses the auto-learn cadence shipped in chunk 14.2; the only new state is a `last_drift_check_turn: i64` field on `AutoLearnState`. |
| 14.9 | **Save / load learned expression presets** — promotes the `expressions-pack` activation gate from a stub to real on-disk persistence under `${app_data}/persona/expressions/<id>.json`. New Tauri commands `persona_list_expressions`, `persona_save_expression`, `persona_load_expression`, `persona_delete_expression` + a `LearnedExpressionStore` Rust module mirroring the persona pack envelope (§ 11.3). Maps to `docs/persona-design.md` § 15.2 row 147. | not-started | Storage chunk — depends on 14.3 landing the runtime expression mapper; delete is hard-delete (no soft-delete tier). |
| 14.10 | **Save / load learned motion clips + `LearnedMotionPlayer`** — same shape as 14.9 for `${app_data}/persona/motions/<id>.json` (33-keypoint × N-frame timeline). New `LearnedMotionPlayer.ts` Three.js helper drives the VRM humanoid bones from a saved clip (consumed by Quest panels and Soul Mirror replay). Maps to `docs/persona-design.md` § 15.2 row 149. | not-started | Promotes `motion-capture` activation gate from stub to real. Depends on 14.4 landing the live retargeter. |
| 14.11 | **Persona side-chain bundle** — extend the `.terransoul-persona` exporter (chunk 14.7) with an optional `assets/` folder containing the learned expressions (14.9) + motion clips (14.10). Importer handles the new optional fields back-compat (older bundles still import cleanly). Bumps the persona-pack envelope to schema v2. Maps to `docs/persona-design.md` § 15.2 row 151. | not-started | Depends on 14.7 + 14.9 + 14.10. Strict back-compat: v1 importer survives. |
| 14.12 | **Phoneme-aware viseme model** — replace the current text-driven mouth-shape heuristic with a phoneme-extracted viseme stream from the active TTS provider (web-speech / future paid TTS). Pure mapping table viseme → ARKit jawOpen + mouthFunnel + …; LLM not involved. Maps to `docs/persona-design.md` § 14.2 row 11 + § 15.2 row 152. | not-started | Sources: FaceFormer / EMOTalk literature in § 14.4. Web-Speech API exposes phoneme `boundary` events on most browsers. |
| 14.13 | **Hunyuan-Motion / MimicMotion offline polish pass (opt-in, deferred)** — research chunk, gated behind a feature flag. Bake-time ML pass that smooths a recorded motion clip (14.10) using a pretrained motion-prior model run via Ollama-like local inference. Off by default; documented in BrainView "Persona advanced" panel. Maps to `docs/persona-design.md` § 14.2 rows 4 + 6 + § 15.2 row 153. | not-started | Defer until at least three real users request it. Heavy-weight; license check required (see `docs/licensing-audit.md`). |
| 14.14 | **MoMask reconstruction for full-body retarget from sparse keypoints** — research chunk. PoseLandmarker only returns 33 keypoints; MoMask reconstructs full SMPL-X body params from sparse signal so the VRM humanoid avoids the "broken elbow" artefacts of pure IK. Maps to `docs/persona-design.md` § 14.2 row 5 + § 15.2 row 154. | not-started | Same gating as 14.13 — research chunk, off by default. |
| 14.15 | **MotionGPT — let the brain *generate* motion tokens directly** — research chunk: introduce a `motion-gpt` brain capability that returns a stream of motion tokens which `LearnedMotionPlayer` (14.10) plays back. Quest-gated behind brain configured + 14.10 active. Maps to `docs/persona-design.md` § 14.2 row 8 + § 15.2 row 155. | not-started | Furthest-out research chunk. Likely uses an Ollama-hosted model when one exists; else opt-in cloud. |

> Camera quests (14.3 / 14.4) are explicitly **side chain** and ship
> *after* the main chain (14.1 + 14.2 — both shipped). Chunk 14.7
> (persona pack export / import) shipped 2026-04-24.

---

### Phase 15 — AI Coding Integrations (MCP + gRPC brain gateway)

> Architectural reference: **[`docs/AI-coding-integrations.md`](../docs/AI-coding-integrations.md)**.
> Goal: expose TerranSoul's brain to other AI coding assistants
> (GitHub Copilot, Claude Desktop, OpenAI Codex / ChatGPT desktop, Cursor,
> Continue, Aider, …) over **two transports**: MCP (plug-and-play) and
> **gRPC with mTLS — the recommended/default for always-on connections**.
> A single Brain-tab Control Panel manages servers, tokens, certs,
> connected clients, and one-click auto-setup.
> Brain-doc-sync rule applies: each chunk must update both
> `docs/AI-coding-integrations.md` and `README.md`'s brain-system listing.

| # | Chunk | Status | Owner | Notes |
|---|---|---|---|---|
| 15.1 | **MCP server** (`src-tauri/src/ai_integrations/mcp/`) — stdio + HTTP/SSE bound to `127.0.0.1:7421`, bearer-token auth (`0600` token file in app config dir), implements the 8 ops in `docs/AI-coding-integrations.md § Surface`. Prefer the `rmcp` crate; fall back to a thin `axum` JSON-RPC 2.0 router if rmcp's stdio+HTTP combo isn't stable yet. Tauri commands: `mcp_server_start` / `_stop` / `_status` / `_regenerate_token`. | not-started | agent | Loopback-only by default; the LAN toggle ships in 15.4. ~600 LOC + tests. Use existing `gh-advisory-database` check before adding `rmcp`. |
| 15.2 | **gRPC server** (`src-tauri/src/ai_integrations/grpc/`) using `tonic` + `prost` + `tonic-build` — bound to `127.0.0.1:7422` with **mTLS by default** (server + client cert pair generated on first start, exported as `.p12`). Schema lives in `proto/terransoul/brain.v1.proto`, versioned. Streaming `brain.search` RPC. Bearer-token-over-TLS fallback for clients that can't do mTLS. | not-started | agent | This is the **recommended/default** transport per the user's request for "more secure". Reuse the existing `rustls` 0.23 dep; cert generation via `rcgen` (already in deps). ~700 LOC + tests. |
| 15.4 | **Control Panel** — `src/views/AICodingIntegrationsView.vue` mounted under the existing **Brain** tab, plus `src/stores/ai-integrations.ts`. Sections: server status (start/stop, ports, token preview, regenerate buttons), connected-clients list with kick-switch, three big auto-setup buttons (15.6), recent-calls log (rolling 200), and a single red **Allow LAN** toggle (off by default, requires re-typing `EXPOSE`). Use `var(--ts-*)` design tokens; no hardcoded hex. | not-started | agent | Adds a sub-route under Brain; doesn't touch the top-level tabs array. Vitest coverage for the store. ~500 LOC frontend + tests. |
| 15.5 | **Voice / chat intents** — extend `src-tauri/src/commands/routing.rs` with an `ai_integrations` capability and the intents listed in `docs/AI-coding-integrations.md § Voice / chat operability` (mcp.start / grpc.stop / autosetup.* / clients.list / clients.disconnect). Each intent calls the matching Tauri command and replies through the assistant-message pipeline. Activation gate for the new "ai-bridge" skill in `src/stores/skill-tree.ts` (skill activates when either server is running). | not-started | agent | Reuses the existing intent-routing surface; no new infra. Update `rules/coding-standards.md` skill-list if needed. ~300 LOC + tests. |
| 15.6 | **Auto-setup writers** for **GitHub Copilot (VS Code)**, **Claude Desktop**, and **Codex / ChatGPT desktop**. Pure functions of `(transport, bind, token, cert_path)` so they're unit-testable. Per-OS config paths per the doc; atomic temp-file + rename writes; idempotent (never duplicate `terransoul-brain` entry); reads existing config to preserve other servers; one-click "Remove from <client>" undo. Bundle a tiny `terransoul-mcp` stdio shim binary (preferred over long-lived HTTP for editor connections). | not-started | agent | The shim ships as a Tauri-sidecar binary — no separate crate. Frontend exposes the three buttons in 15.4. ~600 LOC + tests across all three writers. |
| 15.7 | **VS Code Copilot incremental-indexing QA** — `e2e/ai-integrations/copilot.spec.ts` covering: (a) cold call ingests + answers, (b) warm call hits the fingerprint cache and returns in <50 ms, (c) editing one file invalidates only that file's slice. Implement the `brain.fingerprint` value (hash of indexed set + active brain config) and a `notify`-based file-watcher that updates the fingerprint only on real content changes. Run in CI on the existing Mac/Linux/Windows matrix. | not-started | agent | This is the chunk that makes the user's "don't re-scan everything every turn" requirement testable and enforced. Depends on 15.1 + 15.3 + 15.6. Use the existing `@playwright/test` harness. |
| 15.8 | **Doc finalisation** — replace every "Planned" section in `docs/AI-coding-integrations.md` with as-built reality (paths, ports, exact CLI commands, screenshots from a real Tauri build, verified version matrix). Cross off the chunk rows in the doc. Update `README.md`'s brain-system / component listings to mention the AI Coding Integrations surface and link to the doc. | not-started | agent | Per the brain-doc-sync rule (architecture-rules.md rule 11). Final QA gate before declaring Phase 15 done. |

> **Why two transports?** MCP gives plug-and-play coverage of every editor
> agent that already speaks the protocol (Claude Desktop, Cursor, Continue,
> Codex desktop, Copilot Chat 1.92+). gRPC + mTLS is the **recommended**
> path for any always-on / IDE-plugin scenario because of the strict typed
> schema, encrypted-by-default loopback, first-class streaming, and lower
> per-call overhead. Both terminate at the same `BrainGateway` trait
> (15.3) so the surface can never drift between transports.

---

### Phase 16 — Modern RAG (April 2026 research absorption)

> Architectural reference: **[`docs/brain-advanced-design.md`](../docs/brain-advanced-design.md)** §16 Phase 6 + §19.2.
> Each chunk maps 1:1 to a 🔵 row in §19.2. Brain-doc-sync rule applies
> (architecture-rules.md rule 11): every chunk that lands here must
> update both `docs/brain-advanced-design.md` (flip the 🔵 to ✅, add an
> as-built section) and `README.md`'s brain-system listing.
> Chunks are independently shippable; suggested order tracks the §19.2
> impact ranking — start with the cheapest / highest-recall items.

| # | Chunk | Status | Notes |
|---|---|---|---|
| 16.3 | **Late chunking (Jina AI 2024)** — embed the *whole* document with a long-context embedding model (e.g. `jina-embeddings-v3` or `nomic-embed-text-v2-moe`), then mean-pool per-chunk token windows so each chunk embedding carries cross-chunk context. New `memory::late_chunking` module + ingest-pipeline integration. Maps to §19.2 row 9. | not-started | Requires a long-context embedding model selectable via Ollama. Add to `model_recommender::EmbeddingModel` catalogue. |
| 16.4 | **Self-RAG iterative refinement** (Asai et al., 2023) — orchestrator-level loop where the brain emits `<Retrieve>` / `<Relevant>` / `<Supported>` / `<Useful>` reflection tokens, and the loop iteratively re-retrieves until `<Useful>` is reached or a max-iteration cap is hit (default 3). Lives under `src-tauri/src/orchestrator/self_rag.rs`. Maps to §19.2 row 5. | not-started | Reuses the existing `StreamTagParser` for tag detection. Capped to prevent runaway loops. |
| 16.5 | **Corrective RAG (CRAG)** (Yan et al., 2024) — lightweight LLM evaluator classifies the recall set as `Correct` / `Ambiguous` / `Incorrect`. `Ambiguous` triggers a query-rewrite + re-search; `Incorrect` triggers a web-search fallback (gated behind the existing crawl capability). New `memory::crag::evaluate_recall` + integration into `rerank_search_memories`. Pairs naturally with 16.1's relevance threshold. Maps to §19.2 row 6. | not-started | Web-search fallback only when the user has crawl capability granted; otherwise CRAG just rewrites the query and re-ranks. |
| 16.6 | **GraphRAG / LightRAG community summaries** (Microsoft 2024 + HKU 2024) — Leiden community detection over `memory_edges` (V5+ schema), one LLM-written summary per community, then dual-level retrieval (low-level entity + high-level community theme) fused via existing RRF utility. New `memory::community::detect_communities` + `summarise_community`. Maps to §19.2 rows 7 + 8. | not-started | Heavy chunk; depends on at least 100 edges existing. Background workflow job (use `workflows::engine` for incremental updates). |
| 16.7 | **Sleep-time consolidation** (Letta 2024) — durable workflow job (`workflows::engine`) that runs during user-idle (default >15 min no input). Compresses ageing short→working entries, links related working memories, promotes high-access working entries to long. Writable structured memory blocks per the Letta spec. Maps to §19.2 row 12 + §16 Phase 6. | not-started | Idle detection lives in the existing pet-cursor poll loop; reuses `tasks::manager::TaskManager` for resume on restart. |
| 16.8 | **Matryoshka embeddings** (Kusupati et al., 2022; widely adopted 2024) — switch the active embedding model to a Matryoshka-trained one (e.g. `nomic-embed-text-v1.5`, truncatable to 256 / 512 / 768 dim). Cheap fast first pass at 256-dim → re-rank survivors at 768-dim. New `memory::matryoshka::truncate(emb, target_dim)` + two-stage hybrid_search. Maps to §19.2 row 11 + §16 Phase 6. | not-started | Pairs naturally with the Phase-4 ANN index (16.10). |
| 16.9 | **Cloud embedding API for free / paid modes** — extend `brain::OllamaAgent::embed_text` to also dispatch to OpenAI / Cohere / Voyage when the active brain mode is `FreeApi { provider_id: "..." }` or `PaidApi { ... }`. Allows free-tier users to get real RAG quality without local Ollama. Maps to §16 Phase 4. | not-started | Reuses `provider_rotator::ProviderRotator` for rate-limit handling. |
| 16.10 | **ANN index (`usearch` crate)** — replace the brute-force cosine pass in `MemoryStore::vector_search` with an HNSW ANN index that scales to 1M+ entries while keeping <10 ms p99. Index lives next to the SQLite file (`vectors.usearch`); rebuilt incrementally on insert. Maps to §16 Phase 4. | not-started | Hard dependency: `usearch = "2"` (run `gh-advisory-database` check before adding). Falls back to brute-force when index file is corrupt or missing. |


---

### Phase 17 — Brain Phase-5 Intelligence

> Architectural reference: **[`docs/brain-advanced-design.md`](../docs/brain-advanced-design.md)** §16 Phase 5.
> These chunks turn the brain from a passive store into an active reasoner —
> resolving contradictions, promoting hot memories, reasoning over time,
> and merging across devices via the existing Soul Link CRDT surface.

| # | Chunk | Status | Notes |
|---|---|---|---|
| 17.2 | **Contradiction resolution (LLM picks winner)** — when `add_memory` finds a near-duplicate (existing dedup-by-cosine path) whose content semantically *contradicts* the new one (LLM "do these contradict?" check), opens a `MemoryConflict` row that the BrainView surfaces as a "resolve" prompt. User picks winner; loser is closed via `valid_to` (V9 schema) — never deleted. Maps to §16 Phase 5. | done | V9 migration (valid_to + memory_conflicts table), `conflicts.rs` (prompt/parse + CRUD), `add_memory` wired, 4 new Tauri commands, 12 new Rust tests. |
| 17.5 | **Cross-device memory merge via CRDT sync** — wire `MemoryStore` into the existing Soul Link sync engine (`src-tauri/src/sync/`). LWW-Map CRDT keyed on `(content_hash, source_url)`; conflicts resolved by `last_accessed` then `device_id` lexicographic tiebreak. Maps to §16 Phase 5. | not-started | Reuses Soul Link's QUIC + WebSocket transport. The hardest chunk in this phase — likely splits into 17.5a (schema + handshake) and 17.5b (delta sync). |
| 17.6 | **Conflict detection between connected memories** — Phase 3 leftover. Daily LLM-as-judge pass over `memory_edges` looking for `EdgeRelType::CONTRADICTS` between entries that previously had `SUPPORTS` / `IMPLIES`. Surfaces conflicts in BrainView. Maps to §16 Phase 3 row "Conflict detection between connected memories". | not-started | Composes naturally with 17.2 — both feed the same MemoryConflict surface. |
| 17.7 | **Bidirectional Obsidian sync** — extends the one-way export (18.5) into a bidirectional sync. Watches the configured Obsidian vault dir via `notify`; new / edited markdown files become memories; deleted files close the corresponding memories via `valid_to`. Conflict resolution mirrors 17.5 (LWW). Maps to §16 Phase 4 row "Bidirectional Obsidian sync". | not-started | Depends on 18.5 (one-way export) landing first. |

---


### Phase 19 — Pre-release schema cleanup

> **Why this phase exists.** TerranSoul has not had a public release yet,
> so there is **no installed-base SQLite database in the wild that needs
> to be migrated forward**. The current `src-tauri/src/memory/migrations.rs`
> module (V1 → V7+ schema-first migration runner with up/down SQL blocks
> and per-version test fixtures) is dead weight before v1.0 — it pays a
> compile-time + maintenance cost on every PR and forces every new schema
> field to ship as a numbered migration block instead of just being added
> to the canonical `CREATE TABLE` statement. After Phase 16 / 17 / 18 land,
> we collapse the entire migration history into one canonical schema and
> delete the migration runner. The next time we need a schema change
> *after* the public v1 release, we re-introduce a versioned migration
> (starting cleanly at V1 again, against the v1 baseline).

| # | Chunk | Status | Notes |
|---|---|---|---|
| 19.1 | **Collapse migration history into a canonical schema; delete the migration runner.** Lands as the very last chunk before the v1 cut. Steps: (1) inline the final state of every `CREATE TABLE` / `CREATE INDEX` from `migrations.rs` into a single `MemoryStore::create_canonical_schema(&Connection)` block (or a `schema.sql` constant); (2) remove `src-tauri/src/memory/migrations.rs` and its 600+ test lines; (3) remove the `pub mod migrations;` entry in `src-tauri/src/memory/mod.rs`; (4) remove every `migrations::migrate_to_latest` / `migrations::get_version` / `migrations::downgrade_to` / `MIGRATIONS` / `TARGET_VERSION` reference (`store.rs`, `edges.rs`, `commands::memory::get_schema_status`, `postgres.rs::run_migrations`); (5) drop the `schema_migrations` SQLite table from the canonical schema (no longer needed); (6) drop the `get_schema_status` Tauri command + its frontend caller in `BrainView`. After this chunk, `MemoryStore::open` is just `Connection::open + create_canonical_schema`. Maps to: this phase. | not-started | **MUST land last** — every other in-flight chunk that adds a column (16.6, 16.10, 16.12, 17.2, 17.5) currently does so via a numbered migration block; if 19.1 lands first they would ship as plain canonical-schema additions instead. Concretely: the chunk owner of 19.1 must rebase after 16.x / 17.x / 18.x merge and fold their additions into the canonical block. ~200 LOC removed, ~50 LOC added; 600+ test lines removed (the per-version migration tests are obsolete once there is only one schema). |
