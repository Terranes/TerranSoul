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
