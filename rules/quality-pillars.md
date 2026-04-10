# Quality Pillars — TerranSoul

> These pillars are the **highest-level** guiding principles for TerranSoul.
> Every chunk, every design decision, and every line of code must satisfy the relevant pillars.
> Pillars are delivered incrementally across chunks — each chunk must advance at least one pillar.

---

## Pillar 1 — Reliability

**TerranSoul must work correctly across all supported platforms at all times.**

| Concern | Approach |
|---------|----------|
| Cross-platform parity | One Rust backend + one Vue frontend via Tauri 2.0; platform-specific code isolated in `platform/` |
| Conversation persistence | Conversations saved to local SQLite/JSON before any acknowledgement; no in-memory-only state |
| Agent fault tolerance | Orchestrator handles agent timeout, crash, and non-response; marks agent as `stopped`; never blocks the UI |
| Idempotent message delivery | Each message has a UUID; re-sending the same UUID is a no-op |
| Graceful degradation | If no agent is available, display a clear error message; 3D character remains functional |

---

## Pillar 2 — Security

**Protect user data, device access, and inter-device communication at every layer.**

| Concern | Approach |
|---------|----------|
| Device identity | Each device holds a long-term Ed25519 key pair; private key never leaves the device |
| Pairing | New device pairing via QR code establishes mutual trust; no central authority |
| Link encryption | All TerranSoul Link traffic is end-to-end encrypted; no plaintext transport |
| Agent sandboxing | Agents run in separate processes; WASM sandbox (Phase 3) for community plugins |
| Capability permissions | Sensitive agent capabilities (filesystem, clipboard, email, remote exec) require explicit user consent |
| No secrets in code | API keys and credentials via user settings only; never committed to the repository |
| Input validation | All user input validated before passing to agents or file system APIs |
| Tauri CSP | Content-Security-Policy configured to block arbitrary resource loads |

---

## Pillar 3 — Scalability

**TerranSoul scales from a single-device personal assistant to a multi-device fleet.**

| Concern | Approach |
|---------|----------|
| Multi-device sync | CRDT-based sync (TerranSoul Link) scales to N devices without a central server |
| Agent count | Orchestrator designed as a registry; adding the 100th agent requires no core changes |
| Conversation history | Conversation storage designed for pagination from day one |
| 3D performance | Per-platform pixel ratio cap; LOD strategy for mobile; background render throttling when app is not focused |
| Agent processes | Each agent is a separate OS process; heavy agents run on "Primary" devices; lightweight agents on "Secondary" |

---

## Pillar 4 — Maintainability

**Many contributors must be able to work on TerranSoul productively and safely.**

| Concern | Approach |
|---------|----------|
| Chunk-based delivery | All work is broken into self-contained, reviewable chunks with clear acceptance criteria |
| Consistent standards | Enforced via `rules/coding-standards.md`, `clippy`, `eslint`, and CI |
| Visible architecture | Architecture is explicit in `rules/architecture-rules.md`; module dependency rules are enforced |
| Documentation-first | ADRs for significant decisions; all public APIs documented |
| Resumable sessions | Any AI agent or developer can resume from `rules/milestones.md` + `rules/completion-log.md` |
| Conventional commits | Every commit is traceable to a chunk and a rationale |

---

## Pillar 5 — Availability

**TerranSoul must be available whenever the user needs it.**

| Concern | Approach |
|---------|----------|
| Offline-first | All Phase 1 features work without network access |
| System tray | Desktop app stays accessible from the system tray; never force-quits |
| Mobile background | Background sync (Phase 2) via push notifications; app resumes instantly |
| App startup time | App must be interactive within 2 seconds on desktop; 4 seconds on mobile (Phase 1 target) |
| Auto-restart agents | Orchestrator auto-restarts crashed agents up to 3 times with exponential backoff |

---

## Pillar 6 — Resilience

**TerranSoul recovers gracefully from failures without losing user data.**

| Concern | Approach |
|---------|----------|
| Agent crash recovery | Orchestrator detects agent crash; marks it stopped; queues retry |
| Network interruption | TerranSoul Link reconnects automatically; CRDT merge on reconnect |
| Partial sync | CRDT ensures partial syncs never corrupt state |
| Renderer failure | Three.js errors are caught and surfaced as a friendly fallback message |
| IPC timeout | Tauri commands have a configurable timeout; stalled commands return an error |

---

## Pillar 7 — Supportability

**TerranSoul must be diagnosable by developers and users alike.**

| Concern | Approach |
|---------|----------|
| Structured logging | Rust `tracing` crate with structured fields; log level configurable at runtime |
| Frontend errors | Vue error boundary at `App.vue` level; unhandled errors shown as an in-app notification |
| Agent status | UI shows per-agent status (running / stopped / installing / error) in real time |
| Debug devtools | Tauri devtools open automatically in debug builds |
| Diagnostic export | Users can export a sanitized log bundle for bug reports (Phase 2) |

---

## Pillar 8 — Observability

**Know what is happening inside TerranSoul at all times during development and production.**

| Concern | Approach |
|---------|----------|
| Logging | Rust `tracing-subscriber` (structured JSON in prod, pretty-print in dev) |
| Character state | Every state transition logged at `debug` level with timestamp and trigger |
| IPC tracing | All Tauri command invocations logged with command name, duration, and result |
| Agent health | Periodic health ping to each running agent; status surfaced in UI |
| Performance metrics | Three.js `renderer.info` exposed in debug overlay (draw calls, triangles, textures) |

---

## Pillar 9 — Operational Excellence

**TerranSoul is a joy to install, configure, and operate.**

| Concern | Approach |
|---------|----------|
| One-click install | Bundled via Tauri (MSI/NSIS for Windows, DMG for macOS, AppImage/deb for Linux) |
| In-app updates | Tauri updater plugin for seamless auto-updates |
| Agent marketplace | Phase 3 registry with one-command install: `terransoul install <agent>` |
| Configuration UI | All settings editable via the in-app Settings view; no manual config file editing required |
| Automation rule | If a setup step takes more than a minute and repeats, it must be automated |

---

## Pillar 10 — Testability

**Prove TerranSoul works correctly at every level.**

| Concern | Approach |
|---------|----------|
| Unit tests | Rust: `#[test]` + `#[tokio::test]`; TypeScript: Vitest |
| Component tests | `@vue/test-utils` + Vitest for Vue component behavior |
| Integration tests | Tauri mock-IPC layer for testing frontend ↔ backend contracts |
| E2E tests | Playwright (Phase 2) for full app flows |
| CI gate | All tests must pass on every PR; no merges with failing tests |
| Naming | Rust: `test_<method>_<scenario>_<expected>`; TypeScript: `it('should <behavior>')` |

---

## Pillar 11 — Performance

**TerranSoul feels fast and fluid on all supported devices.**

| Concern | Approach |
|---------|----------|
| 3D render budget | Target 60fps on desktop; 30fps on mobile; pixel ratio capped at 2 |
| IPC latency | Tauri commands should resolve in < 50ms for local operations |
| Agent response time | Stub agent < 1s; real agents stream tokens to the UI to minimize perceived latency |
| Bundle size | Vite tree-shaking; Three.js `three/examples` imports are explicit (no wildcard) |
| Memory | VRM textures released when character is switched; Three.js dispose() called on scene teardown |
| Startup | Lazy-load the Three.js renderer after the chat UI is interactive |

---

## Pillar Coverage by Chunk

| Chunk | Primary Pillars |
|-------|----------------|
| 001 — Project scaffold | 4 (Maintainability) |
| 002 — Chat UI components | 7 (Supportability), 10 (Testability) |
| 003 — Three.js scene setup | 11 (Performance), 5 (Availability) |
| 004 — VRM model loading | 11 (Performance), 1 (Reliability) |
| 005 — Character state machine | 8 (Observability), 1 (Reliability) |
| 006 — Rust chat commands | 1 (Reliability), 10 (Testability) |
| 007 — Agent orchestrator + stub | 1 (Reliability), 6 (Resilience) |
| 008 — Tauri IPC bridge wiring | 4 (Maintainability), 10 (Testability) |
| 009 — Character reactions | 8 (Observability), 11 (Performance) |
| 010 — VRM import + selection UI | 2 (Security), 9 (Operational Excellence) |
| 020 — Device pairing | 2 (Security), 3 (Scalability) |
| 021 — Link transport (QUIC/WS) | 5 (Availability), 6 (Resilience) |
| 022 — CRDT sync engine | 1 (Reliability), 6 (Resilience) |
| 023 — Remote command routing | 2 (Security), 3 (Scalability) |
| 030 — Package manifest format | 4 (Maintainability), 9 (Operational Excellence) |
| 031 — Install/update/remove | 9 (Operational Excellence), 2 (Security) |
| 032 — Agent registry | 3 (Scalability), 9 (Operational Excellence) |
| 033 — Agent sandboxing | 2 (Security), 6 (Resilience) |
