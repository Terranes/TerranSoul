---
description: "Check and install all TerranSoul prerequisites: Node.js, Rust, Tauri dependencies, WebView2, and VBScript"
agent: "agent"
---
Check and install all prerequisites needed for TerranSoul development on the current OS.

## Prerequisites to verify

1. **Node.js** ≥ 20 — check with `node -v`
2. **Rust** (latest stable) — check with `rustc --version`
3. **Tauri CLI** — check with `npx tauri --version` or `cargo tauri --version`
4. **WebView2** (Windows only) — check registry key `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`
5. **VBScript** (Windows only) — check CBS package registry for `Microsoft-Windows-VBSCRIPT-FoD-Package-Wrapper` with `CurrentState = 0x70`

## Workflow

For each prerequisite:

1. **Check** if it is already installed and meets the version requirement.
2. **Report** the status (installed version or missing).
3. **Install** anything that is missing:
   - Node.js → suggest `winget install OpenJS.NodeJS.LTS` (Windows) or link to https://nodejs.org/
   - Rust → suggest `winget install Rustlang.Rustup` (Windows) or `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` (macOS/Linux)
   - Tauri CLI → `cargo install tauri-cli` or `npm install -g @tauri-apps/cli`
   - WebView2 → link to https://developer.microsoft.com/en-us/microsoft-edge/webview2/ or `winget install Microsoft.EdgeWebView2Runtime`
   - VBScript → `Enable-WindowsOptionalFeature -Online -FeatureName 'VBSCRIPT'` (requires admin)
4. **Re-verify** after installation to confirm success.

Finish with a summary table showing each prerequisite, its status, and installed version.
