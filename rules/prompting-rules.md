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
npm run build               # vue-tsc + vite build; must emit dist/ with zero errors
npm run lint                # ESLint (max-lines, vue/recommended, ts-eslint/recommended)

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

> **`rules/completion-log.md` always contains the LATEST entries and must not exceed 10,000 lines.**
> When the cap is reached, the OLDEST entries are moved to a dated archive file
> named `completion-log-{YYYY-MM-DD}.md` (the date is the **archive date**, i.e.
> the date the rotation is performed). `rules/completion-log.md` itself is
> never renamed — it is the single, stable filename that always points at the
> newest history.

The completion log grows monotonically with every shipped chunk. To keep
the active file readable, greppable, and cheap to load into agent
context, the log is **rotated when it would otherwise exceed 10,000 lines**.

### Rotation procedure

When an AI agent is about to append a new completion entry and the
existing `rules/completion-log.md` already has, or would after the
append exceed, **10,000 lines**:

1. **Decide the split point.** Walk the existing entries (newest first
   at the top) and pick the boundary so that:
   - All entries kept in `completion-log.md` together with the new
     incoming entry stay strictly **under 10,000 lines** (including the
     banner, the Table of Contents, and the archive index block).
   - The **oldest** entries — and only the oldest — are the ones that
     move out. Never split an individual chunk entry across two files.
2. **Create the archive file** at
   `rules/completion-log-{YYYY-MM-DD}.md`, where `{YYYY-MM-DD}` is the
   **archive date** (the UTC date the rotation is performed, e.g.
   `completion-log-2026-04-24.md`). The archive file contains:
   - The same top banner / "purpose" paragraph
   - A short `> Archived on {YYYY-MM-DD}. Newer entries live in
     [completion-log.md](completion-log.md).` note
   - A `## Table of Contents` rebuilt for only the archived entries
   - The full body of every archived entry (oldest at the bottom,
     newest at the top — reverse-chronological, same convention as the
     active log).
3. **Edit `rules/completion-log.md` in place** so it now contains:
   - The same top banner / "purpose" paragraph (unchanged filename, so
     all existing links keep working).
   - A `> 📦 Older entries archived in:` block listing every historical
     `completion-log-{YYYY-MM-DD}.md` file in reverse-chronological
     order so future readers can find old chunks.
   - A `## Table of Contents` rebuilt for only the entries that
     remain in this file (plus the new incoming entry).
   - The full body of those remaining entries, with the new entry at
     the top.
4. Commit both files in the same commit with message
   `chore(completion-log): rotate — archive {N} oldest entries to completion-log-{YYYY-MM-DD}.md`.

### Why 10,000 lines?

- A 10k-line markdown file is ~400-500 KB — large but still loadable
  by every common editor, `view` tool, and AI agent context window.
- Rotation by **archive date** (not by chunk number, not by creation
  date) makes the chronology obvious: any
  `completion-log-{YYYY-MM-DD}.md` file is the snapshot of older
  history as of that date.
- Archived files are **never edited again** — they are an immutable
  historical record. Only the active `rules/completion-log.md` ever
  receives new entries, and `rules/completion-log.md` always contains
  the latest history within the 10k-line budget.

### What the agent must NOT do

- Do **not** split a single chunk entry across two files.
- Do **not** delete or summarize archived entries to save space —
  rotate instead.
- Do **not** rotate based on byte size, KB, or chunk count — only the
  10,000-line threshold applies.
- Do **not** rotate eagerly when the file is well under the cap — only
  when the next append would cross 10,000 lines.
- Do **not** rename `rules/completion-log.md` itself — its filename is
  stable so external links and tooling never break. Older entries are
  what move out, not the active file.

---

## ENFORCEMENT RULE — Clean Up Temporary Files After Each Session

> **The agent must leave the repository working tree free of any temporary
> files it created (or inherited) before ending the session.**

Temporary files are anything that is *not* part of the long-lived source
tree, *not* part of the established build / test output already covered
by `.gitignore`, and *not* a deliberate code change for the task. They
include — but are not limited to:

- ad-hoc test logs (`test-output.txt`, `*.log`, captured CI logs),
- scratch JSON / dumps used during debugging,
- one-off helper scripts written under the repo root,
- editor backup files (`*.tmp`, `*~`, `*.bak`, `*.orig`),
- any `/tmp-agent`, `.scratch`, or similar throwaway folders.

### Required end-of-session checklist

1. **Run `git status`** before reporting completion.
2. For every untracked or modified file that is *not* part of the
   actual code change, decide:
   - **delete it** (preferred — `rm <file>`), or
   - if it must stay, add an explicit gitignore entry **and** justify
     it in the PR description.
3. **Never commit** scratch logs or debug dumps. If one slips through,
   remove it in the same PR with `git rm <file>` and add the pattern
   to `.gitignore` so the mistake cannot recur.
4. Prefer creating temporary work under `/tmp/` (outside the repo) so
   it can never be staged by accident — see the existing
   "tips_and_tricks" guidance in `.github/copilot-instructions.md`.

### Patterns already pinned in `.gitignore`

`test-output.txt`, `*.log`, `*.tmp`, `.scratch/`, `/tmp-agent/`. If a
new class of temp file appears, add the pattern to `.gitignore` in the
**same** PR that removes the file — never in a follow-up.
