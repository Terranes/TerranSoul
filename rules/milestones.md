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

Next up: **Chunk 117 — CI/research containerization support**

---

## Phase 9 — Learned Features (From Reference Projects)

> Promoted from `rules/backlog.md` on 2026-05-11 by explicit user request.
> Scope is constrained to CI/research/service containers; do **not** make the
> Tauri desktop runtime depend on Docker.

| Chunk | Status | Title | Notes |
|---|---|---|---|
| 117 | not-started | CI/research containerization support | Keep desktop install native. Improve documented/containerized support for CI/research services and MCP/headless workflows using existing `Dockerfile.mcp`, `docker-compose.mcp.yml`, hive-relay compose files, and container-tooling conventions. Add tests/docs where practical; do not introduce a mandatory app runtime container. |

---



 



