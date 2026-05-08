---
description: "Prepare TerranSoul MCP prerequisites to avoid startup/build failures"
agent: "agent"
---
Set up TerranSoul MCP prerequisites before starting implementation work.

## Goal

Bring the environment to a state where MCP preflight (`brain_health` or `/health`) can succeed without repeated dependency failures.

## Steps

1. **Install project dependencies**
   - Run: `npm ci`

2. **Install Linux Tauri/MCP system libraries (Linux only)**
   - Run:
     - `sudo apt-get update`
     - `sudo apt-get install -y libglib2.0-dev libgtk-3-dev libsoup-3.0-dev libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev`

3. **Verify required pkg-config entries (Linux only)**
   - Confirm these resolve:
     - `pkg-config --modversion glib-2.0`
     - `pkg-config --modversion gobject-2.0`
     - `pkg-config --modversion gio-2.0`
     - `pkg-config --modversion gdk-3.0`

4. **Start MCP for coding sessions**
   - Preferred: `node scripts/copilot-start-mcp.mjs`
   - Fallback: `npm run mcp`

5. **Health check**
   - Check in order:
     - `curl -sf http://127.0.0.1:7423/health`
     - `curl -sf http://127.0.0.1:7422/health`
     - `curl -sf http://127.0.0.1:7421/health`

6. **If MCP still fails**
   - Capture and report exact blocker:
     - `tail -n 120 /home/runner/work/TerranSoul/TerranSoul/mcp-data/self_improve_mcp_process.log`

Finish by reporting which endpoint succeeded and which setup steps were required.
