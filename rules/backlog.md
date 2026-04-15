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

## Phase 7 — VRM Model Security (Anti-Exploit & Asset Protection)

> **Goal:** Prevent VRM model files from being publicly exposed, downloaded, or
> extracted — whether from the GitHub repository, the local filesystem after install,
> or the app's runtime network traffic. VRM creators' work must be protected.
>
> ⛔ **Do not start these chunks until the user explicitly says so.**

### Problem Statement

Currently, all default VRM models (~68 MB total) are committed to `public/models/default/`
in plaintext `.vrm` files. This creates **three exploit vectors**:

1. **GitHub repo exposure** — Anyone can clone/download the repo and get the raw `.vrm` files.
2. **Local source code / installed app extraction** — After `npm run build`, VRM files are
   copied as-is to `dist/models/default/` and then bundled into the Tauri app. Users or
   attackers can browse the app resources folder and copy the files.
3. **Network/DevTools interception** — In dev mode the `.vrm` files are served over HTTP.
   Even in production, the WebView loads them from the local filesystem with predictable paths.

### Milestone Chunks

| Chunk | Description | Status |
|-------|-------------|--------|
| 100 | **Remove VRM from Git & .gitignore** — Add `*.vrm` to `.gitignore`. Remove tracked VRM files from index with `git rm --cached`. Update CI to download models from a private source (GitHub Release asset, private S3 bucket, or Git LFS with access control). Document the private model storage location. | `not-started` |
| 101 | **Build-Time Encryption Pipeline** — Create `scripts/encrypt-models.ts`. AES-256-GCM encryption of each `.vrm` to `.vrm.enc`. Key generated per-release, stored as CI secret. Build script downloads raw VRM from private source → encrypts → outputs to `public/models/default/`. Update `npm run build` to call encryption step. Add `.vrm.enc` files to the repo (or download at CI). | `not-started` |
| 102 | **Rust Decryption Command** — New Tauri command `load_vrm_secure(model_id: String) → Vec<u8>`. Reads `.vrm.enc` from Tauri resource dir, decrypts with compiled-in key (injected via `build.rs` env var). Returns raw bytes. Use `aes-gcm` crate. Add `zeroize` crate for key cleanup. Unit tests with a test-encrypted VRM. | `not-started` |
| 103 | **Frontend Secure Loading Path** — Update `CharacterViewport.vue` and `vrm-loader.ts` to call `invoke('load_vrm_secure', { modelId })` for default models. Receive `ArrayBuffer`, create `Blob` URL, pass to `GLTFLoader`. User-imported models continue using direct file path. Update `default-models.ts` to flag built-in vs user models. Vitest tests for both paths. | `not-started` |
| 104 | **CSP, DevTools & Scope Lockdown** — Set strict CSP in `tauri.conf.json`. Disable devtools in production. Configure Tauri resource/asset scope to deny raw `.vrm` access. Add integrity hashes for `.vrm.enc` files verified at startup. | `not-started` |
| 105 | **Obfuscation & Anti-Tamper** — Add `vite-plugin-obfuscator` to production build. Rust SHA-256 integrity check of `.vrm.enc` files at load time. `zeroize` sensitive buffers after use. Optional: platform-specific anti-debug checks behind feature flag. | `not-started` |

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
