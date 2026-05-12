# Coding Standards

> All code must satisfy the Quality Pillars defined in `rules/quality-pillars.md`.
> AI-routing logic must additionally satisfy `rules/llm-decision-rules.md`
> (no regex / `.includes` / keyword arrays for LLM-routing decisions —
> route through the brain classifier with a user toggle in
> `src/stores/ai-decision-policy.ts`).

---

## Languages and Frameworks

| Layer | Language / Framework | Version |
|-------|---------------------|---------|
| Desktop/Mobile shell | Tauri | 2.x |
| Backend | Rust | stable (MSRV 1.80+) |
| Frontend | Vue 3 + TypeScript | Vue 3.5 / TS 5.x |
| **UI components** | **PrimeVue** (MIT, Vue 3 native) | **4.x** |
| Utility CSS | Tailwind CSS v4 | 4.x |
| 3D rendering | Three.js | 0.175+ |
| VRM loading | @pixiv/three-vrm | 3.x |
| State management | Pinia | 2.x |
| Bundler | Vite | 6.x |

---

## Production-Readiness Mandate

- **No pretend code.** Every function has a real, working implementation with proper error handling.
- **No demo or toy code.** No educational stubs. If it cannot run in production, it does not belong here.
- **No speculative comments.** Do not write `TODO`, `placeholder`, `future`, `will be`, or `subsequent chunks` in code. Implement it or leave it out.
- **No empty trait implementations.** Every trait `impl` has working method bodies.
- **Every committed file must compile and function.** No non-functional scaffolding.

---

## Multi-Agent Instruction Sync

> **ENFORCEMENT RULE — Agent instructions must work for ALL AI coding agents,
> not just GitHub Copilot.**

TerranSoul is developed by multiple AI coding agents: GitHub Copilot, Claude
Code, Codex CLI, Cursor, Continue.dev, Aider, and others. Each agent has its
own auto-loaded instruction file format:

| Agent | Auto-loaded file |
|---|---|
| GitHub Copilot | `.github/copilot-instructions.md` |
| Claude Code | `CLAUDE.md` (repo root) |
| Codex CLI / OpenAI agents | `AGENTS.md` (repo root) |
| Cursor | `.cursorrules` (repo root) |
| Hermes Agent (NousResearch) | `AGENTS.md` (repo root) — auto-loaded by Hermes per its docs |

**Canonical source:** `.github/copilot-instructions.md` is the single source
of truth for project-wide instructions. It contains the full architecture,
coding standards, session protocol, and operational rules.

**Satellite files:** `CLAUDE.md`, `AGENTS.md`, and `.cursorrules` are thin
pointers that:
1. Declare the canonical source (`.github/copilot-instructions.md`)
2. List the files to read for full context (in priority order)
3. Provide a minimal "Quick Reference" so the agent has enough context to
   start without reading every file

**Rules:**

1. **Never edit satellite files directly.** All substantive content lives in
   `.github/copilot-instructions.md`. Satellite files only point to it.
2. **When you add a new rule or change instructions** in
   `.github/copilot-instructions.md`, verify the satellite files' Quick
   Reference still matches. If the change is significant (new section,
   renamed command, removed pattern), update the Quick Reference in all
   satellite files in the same commit.
3. **When a new AI agent becomes popular** and has its own instruction file
   convention, add a satellite file for it and update this table.
4. **MCP bootstrap is agent-agnostic.** The `rules/agent-mcp-bootstrap.md`
   file is already addressed to all agents. It must never contain
   Copilot-specific language or assume VS Code is the editor.
5. **The `CodingWorkflowConfig`** (in `src-tauri/src/coding/mod.rs`) already
   includes `AGENTS.md` in its default `include_files` — this means the
   self-improve engine and coding workflows also see the instructions.

---

## Third-Party Naming & Licensing Hygiene

- Do not name modules, files, commands, Tauri IPC identifiers, types, seed IDs,
  persisted directories, UI labels, docs, milestones, or comments after third-party
  creators, channels, projects, products, mascots, or branded demos unless the name
  is required by an imported dependency or public API contract.
- Use neutral, descriptive names for researched feature patterns (for example,
  `teachable_capabilities`, `wake_word`, or `reference_voice_tts`) and keep any
  attribution or license notes in dedicated research/licensing documentation, not
  runtime identifiers or user-facing feature names.
- Never copy protected assets, transcripts, prompts, branding, or distinctive
  character identity from external projects. Implement only original, configurable
  TerranSoul behavior.

### Always Credit Authors, Open Source, and Reverse-Engineered References

- Maintain a single top-level [CREDITS.md](../CREDITS.md) as the source of
  truth for every external author, project, dataset, paper, blog post,
  video/channel, social post, docs page, tutorial, or artefact whose
  ideas, code, schemas, prompts, file formats, behavior, product lessons,
  or design insights we reference, port, adapt, learn from, compare
  against, or reverse-engineer.
- Any change that pulls in a new dependency, copies a non-trivial
  algorithm, ports a data shape, follows a published technique, cites a
  creator/video/channel for insight, uses an external project to generate
  roadmap/product/architecture decisions, or reverse-engineers an external
  project's behavior **must update `CREDITS.md` in the same PR**. New
  entries include: project / author / creator name, upstream URL, license
  or terms when known, the TerranSoul files or features it influences, and
  one respectful sentence describing what we learned or used.
- No-code influence still counts. If a source informed a persona design,
  UI flow, prompt shape, feature catalogue, comparison matrix, rejected
  decision, or backlog/milestone insight, credit it even when no code,
  text, media, transcript, asset, or schema was copied.
- Removing or replacing a referenced source must also update
  `CREDITS.md` so the file stays accurate (do not leave dangling
  attributions for code that is no longer in the tree).
- Keep attribution out of runtime identifiers and user-facing labels —
  the third-party-naming rule above still applies. `CREDITS.md`,
  `docs/licensing-audit.md`, and dedicated research docs are the right
  homes for names, links, and license details.
- Permissive-licensed dependencies that already appear in
  `docs/licensing-audit.md` should still be reflected in `CREDITS.md`
  so the credits file stands on its own as a complete attribution
  manifest.
- `CREDITS.md` is a public thanks page, not an enforcement page. Keep the
  hard rule text here in `rules/coding-standards.md`; keep the credits file
  appreciative, concrete, and human.

### DeepWiki First for GitHub Reverse Engineering

- When reverse-engineering any GitHub repository, first check
  `https://deepwiki.org/<owner>/<repo>` when network access allows. Use it as
  the high-level map for architecture, module boundaries, and feature
  inventory before reading upstream files directly.
- Always cross-check DeepWiki observations against the upstream repository,
  license, README, docs, and code before turning them into TerranSoul design
  decisions. DeepWiki is an aid, not the source of truth.
- If DeepWiki is unreachable or blocked, record the blocker in the session
  report and continue with direct upstream research instead of silently
  skipping the rule.
- Any durable lesson from reverse-engineering must be credited in
  `CREDITS.md` and synced into MCP self-improve knowledge in
  `mcp-data/shared/**` so future agents can retrieve it with `brain_search`.

### Deep Analysis Before Action

- Every non-trivial decision (adopting an idea from another project, adding a
  dependency, refactoring a subsystem, accepting or rejecting a research
  finding) must complete the deep-analysis protocol in
  [`rules/deep-analysis-rule.md`](deep-analysis-rule.md): MCP-prior-art
  check, source-of-truth read, gap analysis, verdict, and a durable
  write-back appended to `mcp-data/shared/memory-seed.sql`.
- No partial scans. Reading only a top-level README, searching only one MCP
  keyword, or reading only one source file is a violation. If a required
  step cannot complete, stop and report the blocker.
- The cost of a deep analysis is paid once: the verdict goes into the
  shared memory seed so future agents retrieve it via `brain_search` instead
  of re-scanning the same upstream repo or the same TerranSoul subsystem.

### MCP Markdown Memory Boundary

- Do not treat Markdown as TerranSoul MCP memory. Markdown files may describe
  instructions, design, lessons, and projections for humans, but the MCP
  memory source of truth is the SQLite schema seeded from
  `mcp-data/shared/memory-seed.sql` plus `memory_edges`.
- If a PR adds or updates Markdown that contains durable project knowledge
  meant for future agents, sync the same knowledge into
  `mcp-data/shared/memory-seed.sql` in the same PR and connect it with
  `memory_edges` where it has a relationship to existing rules, docs, or
  architecture facts.
- Markdown-only knowledge is incomplete for MCP self-improve. Future agents
  must be able to retrieve the rule/fact through `brain_search` and traverse
  its relationships through the knowledge graph, not by bulk-loading `.md`
  files into context.

### No Mocks in Production

> **It is either a chunk in `rules/milestones.md` OR a real working version with the highest QA — never a half-done mock shipped to users.**

A "mock in production" is anything that pretends to deliver a feature while
actually returning canned, sentinel, or placeholder data on a code path a
real user can hit. Examples:

- HTTP / IPC handlers that return `b"PLACEHOLDER_BINARY"`, hard-coded
  `"Mock response"`, or empty-but-misleading payloads.
- UI components with `// TODO: Implement props usage` and commented-out
  `defineProps<…>()` blocks (the prop contract is fake).
- Renderers / engines named `*Stub*` that satisfy a trait/interface but
  do nothing (e.g. a renderer whose `update()` is empty when the type
  exists in `RendererType`).
- LLM / agent providers that return canned text when the user expects a
  real reply.

**Rule.** When you discover or are about to introduce such code:

1. **Either** implement it for real, with full QA (unit + integration
   tests, error handling, security review, parity with the existing
   tests); **or**
2. **Move it out of the production path** by:
   - Adding a chunk to `rules/milestones.md` (or `rules/backlog.md` if
     unscheduled) describing the proper implementation, AND
   - Removing or hiding the half-done code path so users cannot hit it
     (e.g. delete the catalog entry, remove the variant from the union,
     gate behind `#[cfg(test)]` if it is a test fixture, etc.).

**Allowed exceptions.** Code is *not* a "mock in production" when:

- It is gated behind `#[cfg(test)]` / lives in `*.test.ts` / is only
  reachable from test harnesses.
- It is an explicitly-labeled **fallback** that is never reached in normal
  user flow because auto-configuration immediately replaces it (e.g.
  `StubAgent` only fires when no LLM is configured, and desktop auto-
  config sets up Free API on first launch — it is the "no brain"
  diagnostic, not a pretend brain).
- It is a typed `BuiltIn` variant whose contract is "this path skips the
  download / the binary lives in-process" — i.e. the empty payload is a
  real signal, not a pretend value, and the consumer special-cases it.

When in doubt: write a chunk in `milestones.md` and remove the half-done
path. A clearly-tracked "not yet implemented" is always better than a
hidden mock.

---

## Rust Standards

### Naming
- `snake_case` for functions, variables, modules, and file names
- `PascalCase` for types, enums, and traits
- `SCREAMING_SNAKE_CASE` for constants and statics
- `I`-prefix on traits is **not** used (Rust convention)

### Code Style
- `#[derive(Debug, Serialize, Deserialize, Clone)]` on all public data types
- Prefer `?` operator for error propagation; never `.unwrap()` in library code
- Use `thiserror` for typed error enums; `anyhow` for application-level error chains
- All Tauri commands are `async fn` and return `Result<T, String>`
- Use `tokio::sync::Mutex` for state shared across async tasks; `std::sync::Mutex` for sync-only state

### Error Handling
- Never silently swallow errors — always propagate or log
- Return structured error messages from Tauri commands: `"module: context: cause"`
- Log errors with `tracing::error!` before returning them

### Module Structure
```
src-tauri/src/
  main.rs          — entry point, calls lib::run()
  lib.rs           — Tauri builder, plugin registration, AppState
  commands/
    mod.rs         — pub use re-exports
    chat.rs        — send_message, get_conversation
    agent.rs       — list_agents, get_agent_status
    character.rs   — load_vrm, set_character
  orchestrator/
    mod.rs
    agent_orchestrator.rs
  agent/
    mod.rs         — AgentProvider trait
    stub_agent.rs  — stub implementation
  link/            — (Phase 2) device pairing + sync
  package_manager/ — (Phase 3) install/update/remove agents
```

---

## File Size Budget

Single source files become hard to read, search, and review once they
balloon past a few hundred lines. To keep modules focused and reviewable,
TerranSoul enforces size budgets through the **standard ecosystem
linters** (no custom scripts):

| Language    | Tool                          | Rule                                | Threshold |
|-------------|-------------------------------|-------------------------------------|-----------|
| TypeScript  | ESLint v9 (flat config)       | `max-lines`                         | 1000 / file |
| Vue SFC     | ESLint + `eslint-plugin-vue`  | `max-lines`                         | 800 / file  |
| Rust        | clippy (`src-tauri/clippy.toml`) | `clippy::too_many_lines` (per-fn) | 250 / fn    |

Run from the repo root:

```bash
npm run lint        # report issues (CI gate)
npm run lint:fix    # auto-fix the auto-fixable ones
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

ESLint config: [`eslint.config.js`](../eslint.config.js).
Clippy config: [`src-tauri/clippy.toml`](../src-tauri/clippy.toml).

### Pre-existing oversized files

A small allowlist of pre-existing oversized Vue/TS files lives at the
bottom of `eslint.config.js` (under the comment "Pre-existing oversized
files (allowlist)"). Each entry is a temporary exception — the
long-term goal is for the list to shrink to zero through targeted
refactors (extract sub-components, move long quest-data blocks to a
data file, split state machines into separate stores, etc.).

**Do NOT widen this list.** Adding a new entry requires PR justification
and a tracked follow-up issue for the future split.

For Rust, individual oversized functions can be silenced with
`#[allow(clippy::too_many_lines)]` and a comment linking the follow-up
refactor. The `too_many_lines` lint is currently advisory (not enforced
by CI) — see `src-tauri/clippy.toml` for the rationale.

### Refactoring an oversized file below threshold

When a Vue/TS file is brought back under its budget, **delete its entry
from the allowlist block in `eslint.config.js`** in the same PR so future
regressions are caught at the threshold.

---

## TypeScript / Vue Standards

### Naming
- `camelCase` for variables, functions, and Pinia store actions
- `PascalCase` for Vue components, TypeScript interfaces, and type aliases
- `UPPER_SNAKE_CASE` for top-level constants
- Vue SFC filenames: `PascalCase.vue`
- Store filenames: `camelCase.ts`

### Vue Composition API
- Always use `<script setup lang="ts">` — no Options API
- Prefer `defineProps<{...}>()` and `defineEmits<{...}>()` — no runtime prop validators
- Reactive state: `ref()` for primitives, `reactive()` for objects, `computed()` for derived values
- Side effects: `watch()` for explicit reactions, `watchEffect()` for implicit dependencies

### TypeScript
- `strict: true` enabled — no `any`, no `@ts-ignore` unless absolutely necessary with a comment
- Export all shared types from `src/types/index.ts`
- Never use `!` non-null assertion on Tauri `invoke()` results — always handle the Promise rejection

### Error Handling (Frontend)
- All `invoke()` calls are wrapped in `try/catch`; errors are surfaced in the UI (store error state)
- Never swallow rejected promises silently

### Style
- Single-file components only (template + script + style in one `.vue` file)
- `scoped` styles on all components unless explicitly overriding global styles
- CSS custom properties (`--var-name`) for all theme colors and spacings

---

## UI Framework — No CSS Hacking

> **PrimeVue v4 is the project-mandated Vue 3 component framework.** All new
> UI surfaces — buttons, menus, dropdowns, popovers, dialogs, drawers,
> toolbars, toasts, tags/chips, tooltips, form inputs, data tables, file
> uploaders — must be built from PrimeVue components, not from hand-rolled
> `<button>` + bespoke CSS.

### Why this rule exists

Hand-rolled UI primitives (custom dropdown, custom popover, custom dialog,
custom positioning) cause recurring problems we have shipped multiple times:

- **Overlap from stacked absolute siblings.** Two independently positioned
  elements with hand-tuned `top: 12px` / `top: 56px` overlap as soon as a
  sibling resizes, the viewport shrinks, or padding/font changes.
- **Broken keyboard navigation, focus traps, and ARIA roles.** Real component
  libraries solve these once; our home-grown ones forget them.
- **Inconsistent spacing, motion, and elevation.** Each ad-hoc component
  picks its own paddings and shadows, drifting the design system over time.

PrimeVue solves all of these out of the box: accessible focus management,
collision-aware popovers, real focus trapping in dialogs, consistent design
tokens, and Tailwind-friendly unstyled mode that composes with our
`--ts-*` design tokens.

### What to use instead of CSS hacking

| Instead of … | Use PrimeVue |
|---|---|
| `<button>` + custom hover/focus CSS | `<Button>` |
| Hand-rolled dropdown anchored with `top: 42px; right: 0` | `<Popover>` or `<Menu>` |
| Two stacked absolutely-positioned siblings with `top: 12px` / `top: 56px` | A single positioned wrapper with `display: flex; flex-direction: column; gap` (or `<Toolbar>` for horizontal groups) |
| Hand-rolled modal with backdrop + Esc handling | `<Dialog>` |
| Hand-rolled slide-out panel | `<Drawer>` |
| Bespoke chip/badge with custom CSS variants | `<Tag>` or `<Chip>` |
| `setTimeout`-based toast notifications | `<Toast>` + `useToast()` |
| Custom `<input>` with manual validation styling | `<InputText>`, `<Select>`, `<Checkbox>`, etc. |
| Custom file picker `<input type="file">` styling | `<FileUpload>` |
| Hand-rolled tooltip on `mouseenter` | `v-tooltip` directive |

### Allowed exceptions

These domains are genuinely outside PrimeVue's scope and may continue to
use bespoke Vue components and CSS:

- **3D viewport overlays** — `CharacterViewport.vue`, VRM controls, subtitle
  overlays anchored to character coordinates.
- **Animated 3D background scene** — `BackgroundScene.vue` and its per-theme
  decoration in `style.css`.
- **Quest / RPG-specific surfaces with bespoke gamification visuals** —
  `QuestRewardCeremony.vue`, skill-tree constellation, etc., where the
  visual design *is* the product.
- **Tauri-window-specific chrome** — pet mode overlay, drag regions.

For everything else, **search PrimeVue's component catalogue first** and
only fall back to a custom component when no PrimeVue primitive fits.

### Stacking layout — never hand-tune `top` / `right`

If two elements must sit in the same screen corner, they go in the
**same flex/grid container**:

- ✅ One absolutely-positioned wrapper with `display: flex; flex-direction:
  column; gap: 8px; align-items: flex-end` containing both children.
- ❌ Two siblings each `position: absolute; top: 12px / 56px`.

Hand-tuned `top: 56px` / `right: 130px` magic numbers are a code-review
blocker — they break the moment a font or padding changes.

---

## Tauri IPC Conventions

- Command names: `snake_case` (matches Rust function name)
- Payload types: mirrored exactly between Rust `#[derive(Serialize, Deserialize)]` structs and TypeScript interfaces
- Events emitted from Rust: `kebab-case` (e.g., `character-state-changed`)
- All Tauri capabilities declared in `src-tauri/capabilities/` — no wildcard permissions

---

## Testing

### Rust
- Use the built-in `#[cfg(test)]` module with `#[test]` and `#[tokio::test]`
- Test naming: `test_method_name_scenario_expected_result`
- Use `assert_eq!`, `assert!`, `assert_matches!`
- Mock external I/O with trait-based injection (pass a mock `AgentProvider`)

### TypeScript / Vue
- Vitest for unit and component tests
- `@vue/test-utils` for Vue component testing
- Test naming: `describe('ComponentName') > it('should ...')`
- Test files colocated: `src/components/ChatInput.test.ts` next to `ChatInput.vue`

---

## Documentation

- Rust: `///` doc comments on all public functions, types, and traits
- TypeScript: JSDoc `/** */` on all exported interfaces and store actions
- Architecture Decision Records (ADRs) in `docs/adr/` for significant decisions
- Update `rules/milestones.md` after each chunk is completed

### Correctness Confirmation -> Self-Improve Write-Back (Mandatory)

- When an agent confirms a solution is correct (for example: tests pass,
  bug reproduction is gone, CI gate is green, or the user explicitly accepts
  the fix), the agent **must trigger self-improve write-back** in the same
  chunk before marking work done.
- "Self-improve write-back" means:
  1. Capture the durable lesson/rule that prevented or fixed the issue.
  2. Persist it to MCP shared knowledge by updating
     `mcp-data/shared/memory-seed.sql` in the same PR/commit.
  3. If MCP is healthy, verify retrievability with `brain_search` or
     `brain_suggest_context`; if MCP is blocked, record the exact blocker in
     progress/final output.
- Do not defer confirmed lessons to a later chunk. If correctness is proven
  now, knowledge sync happens now.
- Chunk completion is not finished until both are true:
  1. Code/docs changes are validated.
  2. Durable self-improve knowledge is written back.

---

## Tutorial Screenshots (Mandatory)

Every tutorial in `tutorials/` **must** have an accompanying screenshot for
each numbered section. Screenshots are stored under
`tutorials/screenshots/<tutorial-name>/NN-step-description.png`.

### Rules

1. **No placeholder references.** If a tutorial has an `![alt](screenshots/…)`
   reference, the actual image file **must exist**. A tutorial with broken
   image links is considered incomplete.
2. **Agent-captured.** When an agent creates or updates a tutorial section, it
   must also capture the screenshot itself using the browser tools
   (`open_browser_page` → navigate to the relevant UI state →
   `screenshot_page` → save the PNG to the correct path). Do NOT leave
   screenshot capture for the user.
3. **Capture workflow:**
   - Start the dev server (`npm run dev` or the existing Vite terminal).
   - Open `http://localhost:1420` in the integrated browser.
   - Navigate to the relevant view/panel (e.g. `#/memory` for the graph,
     `#/settings` for configuration panels, `#/chat` for chat UI).
   - Use `screenshot_page` with an appropriate `selector` or full viewport.
   - Save the captured image to `tutorials/screenshots/<name>/NN-step.png`.
4. **Meaningful content.** Screenshots should show realistic UI states — not
   empty/loading screens. If the app needs seed data (memories, personas,
   etc.), use the MCP brain or Tauri commands to populate state first.
5. **Consistent dimensions.** Target 1280×800 viewport. Crop to relevant
   panels when a full-page shot is too wide.
6. **Update on UI change.** When a PR modifies a UI component that appears in
   a tutorial screenshot, re-capture the affected screenshots in the same PR.
7. **Alt text.** The `![alt text]` must concisely describe what the screenshot
   shows (for accessibility). No generic text like "screenshot" or "image".

### Directory convention

```
tutorials/screenshots/
  quick-start/
    01-install.png
    02-first-launch.png
    ...
  voice-setup/
    01-settings-panel.png
    ...
```

---

## Use Existing Libraries — Don't Reinvent the Wheel

Before writing any non-trivial functionality from scratch, **search for a well-maintained open-source crate (Rust) or npm package (frontend)** that already solves the problem. Only write custom code when no suitable library exists, or when the library would introduce unacceptable bloat or licensing issues.

### Decision checklist

1. **Search first** — check crates.io / npm / GitHub for existing solutions before coding.
2. **Prefer battle-tested** — choose libraries with active maintenance, >100 GitHub stars, and recent releases.
3. **Prefer permissive licenses** — MIT, Apache-2.0, BSD, ISC, MPL-2.0 are acceptable. Avoid GPL/AGPL in library dependencies.
4. **Prefer zero/low-dependency** — between two equal libraries, pick the one with fewer transitive dependencies.
5. **Wrap, don't fork** — if a library needs slight customization, write a thin wrapper. Don't copy-paste its source.

### Rust crate preferences

| Need | Use | Don't reinvent |
|---|---|---|
| HTTP client | `reqwest` | Custom TCP/TLS |
| JSON | `serde_json` | Manual parsing |
| Async runtime | `tokio` | Custom thread pools |
| Error handling | `thiserror` / `anyhow` | String-based errors |
| Logging | `tracing` | `println!` debugging |
| UUID | `uuid` | Custom ID schemes |
| Date/time | `chrono` or `time` | Manual epoch math |
| HTML parsing | `scraper` | Regex on HTML |
| URL parsing | `url` | Manual string splits |
| Embeddings / ANN | `usearch` or HNSW crate | Custom brute-force search |
| SQLite | `rusqlite` (bundled) | Raw FFI |
| Regex | `regex` | Hand-rolled matchers |
| Base64 / hex | `base64` / `hex` | Manual encode/decode |
| Crypto | `ring` / `ed25519-dalek` | Custom crypto primitives |

### Frontend (npm) preferences

| Need | Use | Don't reinvent |
|---|---|---|
| State management | `pinia` | Custom reactive stores |
| 3D rendering | `three` + `@pixiv/three-vrm` | WebGL from scratch |
| Markdown | `marked` or `markdown-it` | Regex-based markdown |
| Date formatting | `Intl.DateTimeFormat` (built-in) | Custom formatters |
| Utilities | `@vueuse/core` | Manual browser API wrappers |
| Chart/graph viz | `cytoscape` / `d3` | Canvas drawing from scratch |
| E2E testing | `@playwright/test` | Custom browser automation |
| Unit testing | `vitest` + `@vue/test-utils` | Custom test harness |

### When custom code is acceptable

- The feature is truly domain-specific (quest skill tree, VRM emotion pipeline, stream tag parser).
- No library exists after a genuine search.
- The library adds >5 MB to bundle size for a trivial feature.
- The library is unmaintained (no commits in 2+ years, unpatched CVEs).
- Licensing conflict with the project.

---

## Version Control

- Conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`
- One logical change per commit
- PR description must reference the chunk ID (e.g., `Implements Chunk 003`)
