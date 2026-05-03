# Platform Support — Windows, macOS, Linux, iOS

> **Status (2026-05):** Windows is the primary manual-test platform. Linux is
> the full CI gate for frontend, Rust, and Playwright. iOS has a Tauri 2
> platform overlay, shared Vue frontend shell, Stronghold-secured pairing
> storage, and a macOS smoke check; generating or building the Xcode project
> still requires macOS with Xcode and signing configured.

TerranSoul uses one Rust + Vue 3 codebase across desktop and mobile. There is
no platform-specific Rust core or Vue UI fork; platform differences live in
Tauri config overlays, thin `cfg(target_os = "...")` blocks for OS APIs, and
CSS safe-area variables for mobile WebViews.

## Install per platform

### Windows
- **Recommended:** download the MSI / NSIS installer from the GitHub
  Releases page.
- WebView2 ships with Windows 10/11; no extra install required.

### macOS
- Open the `.dmg` from Releases and drag TerranSoul into `Applications`.
- macOS Gatekeeper: the app is currently signed but not notarised yet
  (tracked in the Phase-12 release notes). On first launch you may need to
  right-click → Open.
- Apple Silicon (`aarch64-apple-darwin`) and Intel (`x86_64-apple-darwin`)
  are both built from the same source.

### Linux
- Pick the bundle that matches your distribution from Releases:
  - `*.deb` — Debian / Ubuntu / Mint
  - `*.rpm` — Fedora / RHEL
  - `*.AppImage` — distro-agnostic, no install required
- Required runtime libraries (already declared by the bundle metadata):
  - `webkit2gtk-4.1` (or `webkit2gtk-4.0` on older distros)
  - `libgtk-3` / `libsoup-3.0`
  - `libayatana-appindicator3-1` (system-tray icon)

For source builds on Linux you also need the dev headers — install with:

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
                 libjavascriptcoregtk-4.1-dev libappindicator3-dev \
                 librsvg2-dev patchelf
```

### iOS
- Requires macOS, Xcode, the iOS simulator toolchain, Rust stable, Node.js,
  and Tauri's npm CLI dependency from `npm ci`.
- `src-tauri/tauri.ios.conf.json` is the supported Tauri 2 platform config
  overlay. It sets the iOS minimum system version, an opaque full-screen main
  window, disabled WKWebView input accessory view, and disabled link previews.
- Pairing credentials use `@tauri-apps/plugin-stronghold` and
  `tauri-plugin-stronghold`; the frontend wrapper lives in
  `src/utils/secure-pairing-store.ts` and requires a caller-supplied vault
  password.
- The shared Vue shell uses `viewport-fit=cover` and `env(safe-area-inset-*)`
  tokens so the bottom navigation clears the iPhone home indicator.

```bash
npm ci
npm run tauri:ios:check  # validates config + macOS/Xcode tooling
npm run tauri:ios:init   # runs npx tauri ios init on macOS
```

For signed device builds, set `APPLE_DEVELOPMENT_TEAM` or add a local iOS
development-team value in a non-committed config override. The CI smoke job
does not run `tauri ios build` yet because signing and simulator selection are
not configured in the repository.

## Build from source

```bash
# All platforms
git clone https://github.com/Terranes/TerranSoul.git
cd TerranSoul
npm ci
npm run build           # type-check + Vite bundle
cd src-tauri
cargo tauri build       # produces installers in src-tauri/target/release/bundle/
```

The `tauri.conf.json` already sets `bundle.targets = "all"` so the bundler
emits every artifact your host OS supports (e.g. `cargo tauri build` on
macOS produces `.dmg` + `.app`).

For iOS, use the npm guard scripts from a macOS host. `npm run tauri:ios:check`
validates the Tauri iOS overlay and host tooling without mutating the tree;
`npm run tauri:ios:init` generates the Xcode project under `src-tauri/gen/apple`.

## CI matrix

The current `.github/workflows/terransoul-ci.yml` workflow has four jobs:

- `frontend` on Ubuntu: ESLint, Vue type-check/build, and Vitest.
- `rust` on Ubuntu: Tauri/WebKit system deps, `cargo clippy --all-targets`,
  and `cargo test --all-targets`.
- `playwright-e2e` on Ubuntu after frontend passes.
- `ios-smoke` on macOS: `npm ci` plus `npm run tauri:ios:check` to validate
  the Tauri iOS scaffold and Xcode host tools without signing or building.

macOS / Windows full bundle smoke-tests will be added once signing certs are
configured.

## Platform-specific code map

| File | What it does per OS |
|---|---|
| `src-tauri/src/brain/docker_ollama.rs` | Locates Docker Desktop install path on Windows / macOS / Linux; uses `taskkill` / `osascript` / `systemctl` to stop it. |
| `src-tauri/src/container/mod.rs` | Detects Podman alongside Docker (Linux native daemon-less, macOS/Windows uses `podman machine`). |
| `src-tauri/src/commands/window.rs` | Click-through pet mode uses Win32 `SetWindowLongPtrW` on Windows; relies on Tauri's cross-platform window API elsewhere. |
| `src-tauri/src/commands/user_models.rs` | Stores imported VRMs under the OS-specific `app_data_dir` (`%APPDATA%`, `~/Library/Application Support`, `~/.local/share`). |
| `src-tauri/tauri.ios.conf.json` | iOS-only Tauri config overlay for minimum iOS version, full-screen opaque WKWebView, and mobile WebView behavior. |
| `src/utils/secure-pairing-store.ts` | Stronghold-backed secure storage wrapper for mobile pairing certificate bundles. |
| `scripts/tauri-ios-check.mjs` | Guarded macOS/Xcode/iOS scaffold checker and `tauri ios init` launcher. |

## Known platform gaps

- macOS notarisation is not yet automated.
- Linux `.deb` / `.rpm` packages are not yet published to apt / dnf
  repositories.
- iOS Xcode project generation and device/simulator builds still require a
  macOS developer machine; CI currently performs a scaffold smoke check only.
- Android remains a follow-up mobile target.
