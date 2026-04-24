# Coding Standards

> All code must satisfy the Quality Pillars defined in `rules/quality-pillars.md`.

---

## Languages and Frameworks

| Layer | Language / Framework | Version |
|-------|---------------------|---------|
| Desktop/Mobile shell | Tauri | 2.x |
| Backend | Rust | stable (MSRV 1.80+) |
| Frontend | Vue 3 + TypeScript | Vue 3.5 / TS 5.x |
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
TerranSoul enforces a **per-file line-count budget**:

| Language | Path scope                | Max lines |
|----------|---------------------------|-----------|
| Rust     | `src-tauri/src/**/*.rs`   | 1000      |
| Vue SFC  | `src/**/*.vue`            | 800       |

The check is implemented by `scripts/check-file-sizes.mjs` and runs via:

```bash
npm run check:file-sizes
```

It scans every Rust and Vue file under the configured roots, prints the
top 5 largest, and **fails (exit 1)** if any non-allowlisted file exceeds
its threshold OR if an allowlisted file has grown beyond its pinned size.

### Allowlist

`scripts/file-size-allowlist.json` pins the recorded line count of files
that already exceed the threshold at the time the rule was introduced.
Allowlisted files are tolerated **at or below** their pinned size — they
**must not grow**. The long-term goal is for this allowlist to shrink
to zero entries through targeted refactors (extract submodules,
extract sub-components, move tests to a `tests/` folder, etc.).

### Adding to the allowlist

Adding new entries is a **last resort**. Prefer splitting the file
first. If you genuinely need to extend the allowlist:

1. Open a PR that runs `node scripts/check-file-sizes.mjs --update`
   (which rewrites the allowlist with the *current* file sizes).
2. In the PR description, justify per-file why splitting is impractical
   and link to a follow-up issue tracking the future split.

### Refactoring an oversized file below threshold

When a file is brought back under its budget, **delete its entry from
`scripts/file-size-allowlist.json`** in the same PR. Future regressions
will then be caught at the threshold instead of the (now obsolete)
larger pinned size.

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
