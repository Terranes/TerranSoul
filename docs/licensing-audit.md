# Commercial-Licence Audit — 2026-04-24

> Triggered by: agent task requirement *"check to make sure all package
> or integrations or libraries meet the commercial usage."*

This document records the commercial-use review of every third-party
package, integration, and library shipped in the TerranSoul desktop app.
The bar is **strict commercial use**: a TerranSoul build distributed as
a paid commercial product (or as part of one) must not require any
extra licence purchase, must not violate any upstream Terms of Service,
and must not silently send user data to any third-party endpoint.

The audit is repeated whenever a new dependency is added (see
`rules/coding-standards.md` *"Use Existing Libraries First"* — the
checklist already includes a licence step). When in doubt, prefer
permissive (MIT / Apache-2.0 / BSD / ISC / MPL-2.0) and prefer
zero-network libraries over SaaS integrations.

## ✅ Cleared for commercial use

### npm dependencies

| Package | Licence | Notes |
|---|---|---|
| `vue`, `pinia`, `@vue/test-utils`, `vue-tsc` | MIT | Core UI framework |
| `@pixiv/three-vrm`, `@pixiv/three-vrm-animation`, `three` | MIT | Avatar rendering (VRM is the only supported avatar format — Live2D is permanently rejected, see avatar-rendering memory) |
| `@tauri-apps/api`, `@tauri-apps/cli`, `@tauri-apps/plugin-shell` | MIT / Apache-2.0 | Desktop shell |
| `cytoscape`, `@types/cytoscape` | MIT | Knowledge-graph mini-viz |
| `pdfkit` | MIT | Persona/memory PDF export |
| `better-sqlite3` | MIT | Embedded DB |
| `@ricky0123/vad-web` | MIT | Voice-activity detection (ONNX, runs in-browser via WebAssembly) |
| `vite`, `@vitejs/plugin-vue` | MIT | Bundler |
| ESLint / TypeScript-ESLint / typescript / globals | MIT / Apache-2.0 | Tooling |
| `vitest`, `jsdom`, `@playwright/test`, `cypress` | MIT | Testing |

### Rust crates

| Crate | Licence | Notes |
|---|---|---|
| `tauri*`, `tauri-plugin-shell` | MIT / Apache-2.0 | Desktop shell |
| `tokio*`, `tokio-tungstenite`, `tokio-util`, `futures-util`, `async-trait` | MIT | Async runtime |
| `axum` | MIT | Local HTTP server (registry / IPC) |
| `reqwest`, `rustls`, `rustls-pemfile`, `quinn`, `rcgen` | MIT / Apache-2.0 / ISC | Networking + TLS |
| `serde`, `serde_json`, `thiserror` | MIT / Apache-2.0 | Core utilities |
| `sqlx`, `rusqlite`, `tiberius`, `scylla`, `postgres` | MIT / Apache-2.0 | StorageBackend implementations |
| `wasmtime` | Apache-2.0 (with WASM exception) | WASM agent runtime |
| `ed25519-dalek`, `ring`, `sha2`, `base64`, `hex`, `rand`, `rand_core` | BSD-3 / MIT / Apache-2.0 | Crypto primitives + manifest signing |
| `scraper`, `url`, `uuid`, `qrcode`, `sysinfo`, `tempfile` | MIT / Apache-2.0 / ISC | General-purpose utilities |

### GitHub advisory database

`gh-advisory-database` scan (2026-04-24): **no vulnerabilities** in the
spot-checked subset. Re-run the scan whenever a dependency version
changes.

## 🚫 Removed (commercial blockers)

The following integrations were present in earlier builds but removed
on 2026-04-24 because they fail the strict commercial-use bar:

### `msedge-tts` (Rust crate, was used in `src-tauri/src/voice/edge_tts.rs`)

- **Crate licence:** MIT (commercial-OK as code).
- **Blocker:** the crate calls Microsoft Edge's undocumented
  `speech.platform.bing.com` *"Read Aloud"* WebSocket endpoint. That
  endpoint is **not** part of any public Microsoft API; the Microsoft
  Services Agreement directs commercial users to **paid Azure
  Cognitive Services — Text to Speech** instead. Programmatic
  third-party use of the Edge endpoint is not sanctioned by Microsoft
  and has historically been rate-limited or blocked when abuse is
  detected.
- **Replacement:** new `web-speech` TTS provider (browser
  `SpeechSynthesis` API). The backend's `synthesize_tts` command
  returns `Vec::new()` for `web-speech`, and the existing
  `useTtsPlayback` composable already falls back to
  `speechSynthesis.speak()` whenever the WAV payload is empty. Browser
  TTS is built into every Tauri-supported platform, has no API key, no
  network round-trip, no telemetry, and no third-party ToS to worry
  about. If higher-quality cloud voices are desired, the
  user-supplied **OpenAI TTS** provider remains available with an
  explicit API key.

### `@vercel/analytics` + `@vercel/speed-insights` (npm, was mounted in `src/App.vue`)

- **Library licence:** MPL-2.0 (commercial-OK as code).
- **Blocker 1:** runtime telemetry is sent to Vercel's analytics
  servers without a user-visible privacy contract. This conflicts with
  TerranSoul's local-first privacy posture (camera frames never leave
  the device, persona suggestions are computed locally, brain memories
  live in SQLite, etc.).
- **Blocker 2:** Vercel's free Web Analytics tier is restricted to
  *non-commercial / personal* projects. Commercial use requires a paid
  Vercel plan and is a poor fit for a desktop binary that doesn't run
  on Vercel.
- **Replacement:** none. A privacy-first desktop app should not phone
  home for usage analytics. If a future opt-in telemetry channel is
  desired, it must be built as a first-party service with explicit
  user consent and a privacy policy.
- **Bonus removal:** `vue-router` was only included to satisfy the
  Vercel libraries' unconditional `useRoute()` calls. With Vercel
  removed, the router was deleted too.

## Process

1. New dependencies are evaluated against the table above before the
   PR that adds them is opened — see the
   *"Use Existing Libraries First"* checklist in
   `rules/coding-standards.md` (steps 3 & 4).
2. The `gh-advisory-database` MCP tool must be invoked for every new
   dependency on a supported ecosystem (npm, rust, pip, …).
3. Any integration that calls a SaaS endpoint must be reviewed for
   ToS compliance, **even if** the client library itself is
   permissively licensed.
4. Telemetry / analytics integrations are forbidden in the desktop
   binary. A future opt-in telemetry surface, if any, must be a
   first-party service with explicit user consent.

## Related documents

- [`rules/coding-standards.md`](../rules/coding-standards.md) — *"Use Existing Libraries First"* checklist
- [`docs/brain-advanced-design.md`](./brain-advanced-design.md) — privacy contract for memory & RAG
- [`docs/persona-design.md`](./persona-design.md) — privacy contract for camera & audio-prosody features
