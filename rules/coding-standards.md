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

## Version Control

- Conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`
- One logical change per commit
- PR description must reference the chunk ID (e.g., `Implements Chunk 003`)
