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

Pick the next `not-started` item from the tables below or from `rules/backlog.md`.

---

## Active Chunks

### Phase 12 — Brain Advanced Design (Documentation & QA)

| # | Chunk | Status | Notes |
|---|---|---|---|
| 1.1 | Brain Advanced Design — QA screenshots | in-progress | All agent work done; waiting on user to capture scenario-specific screenshots on a real Tauri build with Vietnamese content loaded. |

---

### Phase 14 — Persona, Self-Learning Animation & Master-Mirror

| # | Chunk | Status | Notes |
|---|---|---|---|
| 14.13 | **Hunyuan-Motion / MimicMotion offline polish** — research chunk, feature-flagged. ML pass smooths recorded motion clips. | not-started | Deferred until 3+ user requests. |
| 14.14 | **MoMask full-body retarget from sparse keypoints** — research chunk. SMPL-X reconstruction from 33 PoseLandmarker keypoints. | not-started | Research, off by default. |
| 14.15 | **MotionGPT motion token generation** — research chunk. Brain generates motion tokens → `LearnedMotionPlayer` playback. | not-started | Furthest-out research chunk. |

---

### Phase 15 — AI Coding Integrations (MCP + gRPC brain gateway)

| # | Chunk | Status | Notes |
|---|---|---|---|
| 15.1 | **MCP server** — stdio + HTTP/SSE on `127.0.0.1:7421`, bearer-token auth, 8 ops. Prefer `rmcp` crate. | not-started | ~600 LOC + tests. |
| 15.2 | **gRPC server** — `tonic` + mTLS on `127.0.0.1:7422`, `brain.v1.proto`, streaming search. | not-started | ~700 LOC + tests. |
| 15.4 | **Control Panel** — `AICodingIntegrationsView.vue` + `ai-integrations.ts` store. Server status, clients, auto-setup buttons, LAN toggle. | not-started | ~500 LOC + tests. |
| 15.5 | **Voice / chat intents** — `ai_integrations` capability + intents in routing.rs. "ai-bridge" skill activation gate. | not-started | ~300 LOC + tests. |
| 15.6 | **Auto-setup writers** — GitHub Copilot, Claude Desktop, Codex/ChatGPT. Pure functions, idempotent config writes, `terransoul-mcp` stdio shim sidecar. | not-started | ~600 LOC + tests. |
| 15.7 | **VS Code Copilot incremental-indexing QA** — e2e test: cold/warm calls, fingerprint cache, file-watcher invalidation. | not-started | Depends on 15.1 + 15.3 + 15.6. |
| 15.8 | **Doc finalisation** — replace all "Planned" sections in `docs/AI-coding-integrations.md` with as-built reality. | not-started | Final QA gate for Phase 15. |

---

### Phase 16 — Modern RAG

| # | Chunk | Status | Notes |
|---|---|---|---|
| 16.3 | **Late chunking** — long-context embed → mean-pool per-chunk windows. `memory::late_chunking` module. | not-started | Needs long-context embedding model in Ollama catalogue. |
| 16.4 | **Self-RAG iterative refinement** — orchestrator loop with `<Retrieve>`/`<Relevant>`/`<Supported>`/`<Useful>` reflection tokens, max 3 iterations. | not-started | Reuses `StreamTagParser`. |
| 16.5 | **Corrective RAG (CRAG)** — LLM evaluator classifies recall as Correct/Ambiguous/Incorrect; rewrite or web-search fallback. | not-started | Web-search only with crawl capability. |
| 16.6 | **GraphRAG / LightRAG community summaries** — Leiden community detection over `memory_edges`, LLM summaries, dual-level retrieval + RRF. | not-started | Heavy chunk; background workflow job. |
| 16.7 | **Sleep-time consolidation** — idle-triggered workflow: compress short→working, link related, promote high-access to long. | not-started | Reuses `TaskManager` for resume. |
| 16.8 | **Matryoshka embeddings** — two-stage search: fast 256-dim pass → re-rank at 768-dim. | not-started | Pairs with ANN index (16.10, shipped). |

---

### Phase 17 — Brain Phase-5 Intelligence

| # | Chunk | Status | Notes |
|---|---|---|---|
| 17.5 | **Cross-device memory merge via CRDT sync** — wire `MemoryStore` into Soul Link. LWW-Map CRDT keyed on `(content_hash, source_url)`. | not-started | Hardest chunk — may split into 17.5a (schema) + 17.5b (delta sync). |
| 17.7 | **Bidirectional Obsidian sync** — extend one-way export to bidirectional via `notify` file-watcher. LWW conflict resolution. | not-started | Depends on 18.5 (shipped). |

---

### Phase 19 — Pre-release schema cleanup

| # | Chunk | Status | Notes |
|---|---|---|---|
| 19.1 | **Collapse migration history → canonical schema; delete migration runner.** Single `create_canonical_schema` block, remove `migrations.rs` + 600 test lines. | not-started | **MUST land last** — after all schema-changing chunks (16.6, 17.5, etc.). |
