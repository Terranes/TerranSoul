#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // CLI flag: `terransoul --mcp-stdio` runs as an MCP stdio server
    // (newline-delimited JSON-RPC over stdin/stdout) instead of
    // launching the GUI. See Chunk 15.9 in `rules/milestones.md`.
    if std::env::args().any(|a| a == "--mcp-stdio") {
        if let Err(e) = terransoul_lib::run_stdio() {
            eprintln!("[mcp-stdio] fatal: {e}");
            std::process::exit(1);
        }
        return;
    }

    // CLI flag: `terransoul --mcp-http` runs the headless MCP HTTP
    // server (a.k.a. "MCP pet mode") used by `npm run mcp` so external
    // AI coding agents (Copilot, Codex, Claude Code, Clawcode, …) can
    // attach to the brain/RAG/memory surface without launching Tauri
    // or colliding with `npm run dev` / a release build.
    if std::env::args().any(|a| a == "--mcp-http") {
        if let Err(e) = terransoul_lib::run_http_server() {
            eprintln!("[mcp-http] fatal: {e}");
            std::process::exit(1);
        }
        return;
    }

    terransoul_lib::run()
}
