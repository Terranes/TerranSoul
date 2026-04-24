# Prompting Rules — TerranSoul

## Environment Prerequisites

Before implementing any chunk, verify the development environment:

1. **Rust (stable, MSRV 1.80+)** — check with `rustc --version`
   - Install: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. **Node.js 20+** — check with `node --version`
   - Install via `nvm`, `fnm`, or <https://nodejs.org>
3. **Tauri CLI 2.x** — check with `cargo tauri --version`
   - Install: `cargo install tauri-cli --version "^2"`
4. **System dependencies for Tauri** — platform-specific:
   - Linux: `webkit2gtk`, `libssl-dev`, `build-essential`
   - macOS: Xcode Command Line Tools
   - Windows: Visual Studio Build Tools + WebView2

See `docs/developer-setup.md` for full setup instructions.

---

## Resumption Protocol

To continue development from where the last session ended, use:

```
Continue
```

The AI agent must:

1. Read `rules/milestones.md`
2. Identify the next chunk with status `not-started`
3. Implement **only that chunk** — no skipping ahead
4. Update the chunk status to `done` in `milestones.md`
5. Log completion details in `rules/completion-log.md`
6. Ensure the repository is in a buildable, runnable state before finishing

---

## Chunk Implementation Rules

- Implement **one chunk** at a time
- Do not skip ahead to future chunks
- Do not modify completed chunks unless fixing a confirmed bug
- Set chunk status to `in-progress` when starting
- Set chunk status to `done` when complete and verified
- Log details (files created/modified, test counts, notes) in `rules/completion-log.md`

---

## ENFORCEMENT RULE — Never Start Chunks from Backlog

> **`rules/backlog.md` is a holding area only — never start work on chunks listed there.**
>
> All chunk implementation **must** begin from `rules/milestones.md`.
>
> If `milestones.md` has no `not-started` chunks remaining, the AI agent must:
> 1. **Stop** and inform the user that all milestone chunks are complete.
> 2. Ask the user which backlog chunk(s) to promote to `milestones.md`.
> 3. **Only after the user confirms**, move the selected chunk(s) from `backlog.md` to `milestones.md`.
> 4. Then proceed with the normal "Continue" workflow from `milestones.md`.
>
> **Rationale:** The backlog is unscheduled, unprioritized work. The user decides
> what gets promoted to milestones. AI agents must not autonomously pick backlog
> items to implement.

---

## Code Generation Rules

- Generate **working, compilable code** — no stubs, no skeletons
- Follow all rules in `rules/coding-standards.md`
- Follow all rules in `rules/architecture-rules.md`
- Satisfy the relevant Quality Pillars in `rules/quality-pillars.md`
- Apply the Reality Filter from `rules/reality-filter.md`
- Add Rust `///` doc comments on all public functions and types
- Add JSDoc `/** */` on all exported TypeScript interfaces and store actions
- Write unit tests for all new non-trivial logic

---

## Build Verification

After every chunk, verify:

```bash
# Frontend
cd /path/to/TerranSoul
npm run build          # vue-tsc + vite build; must emit dist/ with zero errors

# Rust backend
cd src-tauri
cargo check            # must compile with zero errors
cargo clippy -- -D warnings  # must produce zero warnings
```

**Do not mark a chunk `done` if either build step fails.**

---

## Documentation Rules

- Update `rules/milestones.md` after every completed chunk:
  - Mark the chunk row as `done`
  - Add a one-line summary under the chunk
  - Update the `Next Chunk` section
- Log full completion details in `rules/completion-log.md`
- Create an ADR in `docs/adr/` for any significant architectural decision

---

## ENFORCEMENT RULE — Completed Chunks Must Be Archived

When a chunk is marked `done`:

1. Log full details (date, goal, files created/modified, test counts) in `rules/completion-log.md`
2. **Remove the done row from `milestones.md`** — replace with a one-line `✅ Chunk NNN — done` note
3. If an entire phase has no remaining rows, replace the table with: `✅ Phase N complete — see completion-log.md`
4. Update the `Next Chunk` section to point to the next `not-started` chunk

This rule is mandatory for every AI agent session.

---

## ENFORCEMENT RULE — Completion-Log File Size Cap

> **`rules/completion-log.md` and any rolled-over file must not exceed 10,000 lines.**

The completion log grows monotonically with every shipped chunk. To keep
any single file readable, greppable, and cheap to load into agent
context, the log is **rotated when it would otherwise exceed 10,000 lines**.

### Rotation procedure

When an AI agent is about to append a new completion entry and the
existing `rules/completion-log.md` already has, or would after the
append exceed, **10,000 lines**:

1. **Rename** the current `rules/completion-log.md` to
   `rules/completion-log-{YYYY-MM-DD}.md`, where `{YYYY-MM-DD}` is the
   **creation date of the file being rotated** (i.e. the date of its
   *first* entry — read from the `**Date:**` field of the oldest
   entry, or from the file's earliest git history if no date field is
   present). This is the file's "creation date" for archival purposes.
2. **Create a fresh `rules/completion-log.md`** — copy only the
   following from the rotated file:
   - The top banner / "purpose" paragraph
   - The `## Table of Contents` header (with an empty table body)
   - A new `> Previous entries archived in:` block listing every
     historical `completion-log-{YYYY-MM-DD}.md` file in
     reverse-chronological order so future readers can find old chunks.
3. **Append the new chunk entry** to the fresh file as usual.
4. Commit both the rotated file and the new file in the same commit
   with message `chore(completion-log): rotate at 10,000 lines`.

### Why 10,000 lines?

- A 10k-line markdown file is ~400-500 KB — large but still loadable
  by every common editor, `view` tool, and AI agent context window.
- Rotation by **calendar date** (not by chunk number) makes archived
  files self-describing: `completion-log-2026-04-24.md` is obviously
  the log that *started* on 2026-04-24.
- Archived files are **never edited again** — they are an immutable
  historical record. Only the current `rules/completion-log.md`
  receives new entries.

### What the agent must NOT do

- Do **not** split a single chunk entry across two files.
- Do **not** delete or summarize archived entries to save space —
  rotate instead.
- Do **not** rotate based on byte size, KB, or chunk count — only the
  10,000-line threshold applies.
- Do **not** rotate eagerly when the file is well under the cap — only
  when the next append would cross 10,000 lines.
