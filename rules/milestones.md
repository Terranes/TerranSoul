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
> See `rules/prompting-rules.md` -> "ENFORCEMENT RULE — Clean Up Reverse-Engineering Research on Chunk Completion".

> **Completed work lives in [`rules/completion-log.md`](completion-log.md).**
> Do not re-list done chunks here. Chunks are recorded there in reverse-chronological order.

---

## Next Chunk

The next `not-started` chunk is **44.1 — RAG latency optimization**.
Implement chunks in order; each one must keep the Full CI Gate green before moving on.

---

## Phase 44 — Retrieval Quality & Onboarding (May 2026)

> **Goal.** Address the top findings from the Phase 43 comparison review:
> optimize RAG hot paths, smooth out new-user onboarding, validate the
> ambient maintenance agent in production-like conditions, unlock
> cross-harness replay, and support embedding model switching. Five chunks.

| ID | Title |
| -- | ----- |
| 44.1 | RAG latency optimization — benchmark end-to-end retrieval, add query result caching |
| 44.2 | First-run setup wizard — guided Ollama install + model pull + embedding warmup |
| 44.3 | Ambient agent validation — run 50+ simulated garden cycles, measure quality metrics |
| 44.4 | Cross-harness replay mode — replay imported sessions to extract/verify context |
| 44.5 | Embedding model registry — switch models with automatic re-embedding migration |

> Detailed specs for each chunk live in [`rules/backlog.md`](backlog.md) § Phase 43.
