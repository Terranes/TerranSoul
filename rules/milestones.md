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

Chunk 29.2 — Browser brain transport hardening.

---

## Active Chunks

| Chunk | Status | Goal | Maps to |
|---|---|---|---|
| 29.2 | not-started | Browser brain transport hardening: define the browser-safe path for free cloud chat, paid API configuration, and optional RemoteHost pairing so browser mode does not imply local Rust memory/LLM capabilities that are unavailable without a host. | `src/stores/brain.ts`, `src/stores/conversation.ts`, `src/transport/`, `docs/brain-advanced-design.md` |
| 29.3 | not-started | Browser app-window UX hardening: refine the small in-page window controls, keyboard/focus behavior, accessibility labels, and mode switching between pet preview, 3D, and chatbox layouts. | `src/App.vue`, `src/views/BrowserLandingView.vue`, `src/views/ChatView.vue` |
| 29.4 | not-started | glib/GTK modernization tracker: periodically retry the Tauri/wry/gtk-rs dependency path, remove the `glib 0.18` advisory note only when the Linux stack can actually resolve to `glib >=0.20`, and avoid adding duplicate direct glib dependencies that leave gtk3 transitives in place. | `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, Tauri/wry Linux dependency graph |
| 29.5 | not-started | Sitting-prop lifecycle regression coverage: add targeted frontend/renderer tests or lightweight harness coverage proving the chair is absent by default, appears only for sitting animations, and is disposed/hidden after sitting ends. | `src/components/CharacterViewport.vue`, `src/renderer/props.ts`, `src/renderer/vrma-manager.ts` |
