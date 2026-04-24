# TerranSoul — Backlog

> **Never-started work lives here.** Only move chunks from this file to
> `milestones.md` when the user explicitly says so. This file is the holding
> area for planned but unscheduled work.
>
> ⛔ **RULE: Never start chunks from this file.**
> All chunk implementation must begin from `rules/milestones.md`.
> If milestones.md has no `not-started` chunks, ask the user which backlog items to promote.
> See `rules/prompting-rules.md` for full enforcement rule.

---

## Phase 9 — Learned Features (From Reference Projects)

> **Source repos:** Open-LLM-VTuber, AI4Animation-js, VibeVoice, aituber-kit
> **Analysis:** See `rules/research-reverse-engineering.md` §9.
> **Principle:** Integrate proven patterns; don't reinvent.

### High Priority

📦 Promoted to `rules/milestones.md` — chunks 106–109.

### Medium Priority

📦 Promoted to `rules/milestones.md` — chunks 094–098.

### Lower Priority

📦 Promoted to `rules/milestones.md` — chunks 115–119 (renumbered from 110–114 to avoid conflict with Chunk 110 BGM).

### Demoted from Milestones

| Chunk | Description | Status | Notes |
|-------|-------------|--------|-------|

---

## Phase 10 — Developer Experience & Copilot Integration

> **Goal:** Streamline the AI-assisted development loop so Copilot (and other
> coding agents) can run long autonomous sessions without manual babysitting.

| Chunk | Description | Status | Notes |
|-------|-------------|--------|-------|
| 10.1 | **Auto-accept Copilot permissions & prompts.** Add a VS Code workspace setting (`/.vscode/settings.json`) and/or `copilot-instructions.md` directive that pre-approves tool-use confirmations (terminal commands, file edits, browser actions) so the agent never stalls waiting for a click. Document which `github.copilot.chat.*` settings to toggle and ship a one-click "Enable Autonomous Mode" task in `.vscode/tasks.json`. | not-started | Scope: `.vscode/settings.json`, `copilot-instructions.md`, `docs/AI-coding-integrations.md`. |
| 10.2 | **Auto-retrigger agent tasks until completion.** Create a lightweight VS Code task / script (`scripts/copilot-loop.mjs`) that monitors the Copilot chat session output, detects when the agent stops mid-task (context-limit hit, timeout, permission gate), and automatically re-prompts with a "Continue" message plus the last conversation summary. Include a configurable max-retries guard and a stop-on-error flag. | not-started | Depends on 10.1. Could use VS Code extension API or a terminal-based wrapper. Research Open-LLM-VTuber's agent-loop pattern for inspiration. |
| 10.3 | **Long-running service health gate.** Add a `scripts/wait-for-service.mjs` utility that polls a local endpoint (Ollama, dev server, Tauri) and only triggers the next Copilot task step once the service is confirmed healthy. Wire into `.vscode/tasks.json` as a `dependsOn` pre-task so Copilot commands that need a running backend don't race against cold starts. | not-started | Pairs with existing `scripts/check-port.cjs`. |
