---
description: "Check and install all TerranSoul prerequisites: Node.js, Rust, Tauri CLI, WebView2 (Windows), and npm deps"
---
# Setup Prerequisites

Check and install all prerequisites for TerranSoul development on the current OS.

## Steps

1. Run the prerequisite checker script:
   ```bash
   node scripts/setup-prerequisites.mjs
   ```

2. If anything is missing, auto-install:
   ```bash
   node scripts/setup-prerequisites.mjs --auto
   ```

3. After installation completes, re-verify:
   ```bash
   node scripts/setup-prerequisites.mjs
   ```

4. Once all green, install npm dependencies if not already done:
   ```bash
   npm install
   ```

## Prerequisites

| Requirement | Min Version | Install (Windows) | Install (macOS/Linux) |
|---|---|---|---|
| Node.js | ≥ 20 | `winget install OpenJS.NodeJS.LTS` | `brew install node@20` / nodesource |
| Rust | stable | `winget install Rustlang.Rustup` | `curl https://sh.rustup.rs -sSf \| sh` |
| Tauri CLI | latest | `cargo install tauri-cli` | `cargo install tauri-cli` |
| WebView2 | any (Win only) | `winget install Microsoft.EdgeWebView2Runtime` | N/A |
| npm deps | — | `npm install` | `npm install` |

## Recommended Setup

After prerequisites are installed:
```bash
npm install              # Install frontend dependencies
npm run mcp             # Start the MCP brain (optional, for AI agents)
cargo tauri dev         # Launch the full app with hot-reload
```
