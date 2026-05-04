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
> Do not re-list done chunks here. Phases 0–28 and all previously tracked
> chunks are recorded there in reverse-chronological order.

---

## Next Chunk

**Chunk 32.6 — MCP seed verification + status enrichment.**

---

## Phase 32 — MCP Agent-Ready, Self-Improve Autonomy, Animation Wiring & Hardening

> Close the remaining gaps so any agent (Copilot, Codex, Cursor, Claude Code)
> can connect to `npm run mcp` with zero manual setup, the self-improve loop
> actually completes chunks end-to-end, animation pose streaming works live,
> and documentation covers the full contributor onboarding flow.

| ID | Status | Title | Goal |
|---|---|---|---|
| 32.6 | not-started | MCP seed verification + status enrichment | Add `seed_loaded: bool` and `actual_port: u16` to `/status` response. Verify seed was applied by checking memory count > 0 on startup. Test: assert `/status` includes both fields. |
| 32.7 | not-started | vue-tsc + clippy hardening pass | Fix any remaining type errors across all `.vue`/`.ts` files. Ensure `cargo clippy -- -D warnings` has zero suppressed lints. Add `#![deny(unused_must_use)]` to lib.rs. Run full CI gate. |
| 32.8 | not-started | Animation emotion intensity pipeline | Wire avatar-state emotion scores (from streaming text analysis) into `EmotionPoseBias.setEmotion(emotion, intensity)`. Map LLM stream tag `<emotion:happy:0.8>` to intensity. Test: unit test emotion tag parsing + bias application. |

---
