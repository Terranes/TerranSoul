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

---

## Next Chunk

_Phase 13 (2.1 → 2.4) shipped 2026-04-24. Phase 14 main-chain
(14.1 + 14.2) shipped 2026-04-24. Chunk 14.7 (persona pack export /
import) shipped 2026-04-24._ Remaining Phase 14 work is the
camera-driven side chain (14.3 / 14.4) and one optional polish chunk
(14.5 VRMA baking). Pick the next item
from the active table below or from `rules/backlog.md`.

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
| 15.3 | **`BrainGateway` trait + shared op surface** (`src-tauri/src/ai_integrations/gateway.rs`). All 8 ops route through one trait so MCP and gRPC can't drift. Composes existing `commands::memory::hybrid_search_memories_rrf`, `hyde_search_memories`, `memory::edges`, `brain::ollama_agent::summarize`, `commands::brain::get_brain_selection`. New op `brain.suggest_context` (the editor-flagship call) composes the three above into a delta-stable context pack. | not-started | agent | Land **first** so 15.1 and 15.2 just wire transports onto a finished surface. Capability-gate writes through the orchestrator (`brain.write` off by default per client). ~400 LOC + tests. |
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

