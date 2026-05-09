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

**Phase 46.1 — Agent-session lesson capture for self-improve.**
See chunk row below.

---

## Phase 46 — Agent-session knowledge ingestion (closes self-improve gap discovered 2026-05-10)

| Chunk | Title | Status | Goal |
|---|---|---|---|
| 46.1 | Agent-session lesson detector + `brain_ingest_lesson` MCP tool | not-started | Add `src-tauri/src/coding/agent_session_lessons.rs` with `detect_lesson(message, role, prior_messages) -> Option<LessonChunk>` recognising user-corrective ("you should X instead of Y", "stop doing X") and agent-authored ("I learned X", "lesson:") patterns. Add `brain_ingest_lesson{content,tags,importance,category}` MCP tool that writes to `memories` table via the gateway AND appends an `INSERT` row to `mcp-data/shared/memory-seed.sql` so lessons survive DB reseed. Extend `coding/conversation_learning.rs` `DetectionReply` schema with a `lesson` category that routes to the new tool instead of `milestones.md`. Add CI check: when `mcp-data/shared/migrations/NNN_*.sql` is added, `lessons-learned.md` must change in the same commit. Tests: detection unit tests for both pattern families, MCP tool round-trip via `gateway` mock, CI check via shell script. Reference: migration `019_self_improve_agent_session_gap_lessons.sql`. |
| 46.2 | Manual tutorial screenshot QA — sweep all 21 tutorials | not-started | Walk every tutorial step-by-step (one screenshot at a time), opening the exact target view, dismissing quest overlays, confirming 3D mode state, capturing, and visually verifying the resulting PNG before moving on. Fix any UI defect found and recapture immediately. Tutorials: `quick-start`, `brain-rag-setup`, `brain-rag-local-lm`, `advanced-memory-rag`, `knowledge-wiki`, `folder-to-knowledge-graph`, `context-folder-conversion`, `skill-tree-quests`, `voice-setup`, `charisma-teaching`, `teaching-animations-expressions-persona`, `device-sync-hive`, `hive-relay`, `lan-mcp-sharing`, `browser-mobile`, `mcp-coding-agents`, `multi-agent-workflows`, `openclaw-plugin`, `packages-plugins`, `self-improve-to-pr`, `mcp-server-integration-guide`. Will span multiple sessions; track progress via `/memories/session/tutorial-qa-progress.md`. **No batch scripts** — each capture must be human-verified per migration `018`. |

---

