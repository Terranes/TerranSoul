# Platform Support — Windows, macOS, Linux

> **Status (2026-04):** Windows is the primary tested platform. macOS and Linux
> are supported by Tauri and verified on every push by the
> `cross-platform-rust` CI job, but full installer testing on those OSes is
> still being expanded (Chunk 1.2).

TerranSoul ships a single Tauri 2.x binary that targets Windows, macOS, and
Linux from the same Rust + Vue 3 codebase. There is no platform-specific
Rust core or Vue UI fork — only thin `cfg(target_os = "...")` blocks for OS
APIs the desktop quest layer needs (window placement, container runtime
auto-start, file-system paths).

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

## CI matrix

The `cross-platform-rust` job in `.github/workflows/terransoul-ci.yml`
runs `cargo check --all-targets` and `cargo test --lib` on
`macos-latest` and `windows-latest` for every push. The Linux full
build / clippy / vitest job remains the gating job. macOS / Windows full
bundle smoke-tests will be added once we have signing certs configured.

## Platform-specific code map

| File | What it does per OS |
|---|---|
| `src-tauri/src/brain/docker_ollama.rs` | Locates Docker Desktop install path on Windows / macOS / Linux; uses `taskkill` / `osascript` / `systemctl` to stop it. |
| `src-tauri/src/container/mod.rs` | Detects Podman alongside Docker (Linux native daemon-less, macOS/Windows uses `podman machine`). |
| `src-tauri/src/commands/window.rs` | Click-through pet mode uses Win32 `SetWindowLongPtrW` on Windows; relies on Tauri's cross-platform window API elsewhere. |
| `src-tauri/src/commands/user_models.rs` | Stores imported VRMs under the OS-specific `app_data_dir` (`%APPDATA%`, `~/Library/Application Support`, `~/.local/share`). |

## Known platform gaps (tracked in Chunk 1.2)

- macOS notarisation is not yet automated.
- Linux `.deb` / `.rpm` packages are not yet published to apt / dnf
  repositories.
- iOS / Android Tauri targets are scaffolded but not yet shipped.
