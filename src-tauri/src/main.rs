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
    // server in a container or background process. No GUI, no Tauri
    // window — just axum on port 7423 with state in `mcp-data/`.
    // Used by `npm run mcp` (Docker) for full isolation from dev/release.
    if std::env::args().any(|a| a == "--mcp-http") {
        if let Err(e) = terransoul_lib::run_http_server() {
            eprintln!("[mcp-http] fatal: {e}");
            std::process::exit(1);
        }
        return;
    }

    // CLI flag: `terransoul --mcp-app` runs the **full Tauri app** in
    // MCP mode so developers can visually observe the brain/RAG/memory
    // surface live. Same UI as the normal app, but the bottom-left badge
    // reads "MCP", state lives in `<repo>/mcp-data/`, and the MCP HTTP
    // server auto-binds to port 7423.
    if std::env::args().any(|a| a == "--mcp-app") {
        // SAFETY: set before any other thread spawns. We are still the
        // sole thread on `main`. The flag is read at Tauri setup time.
        unsafe { std::env::set_var("TERRANSOUL_MCP_APP_MODE", "1") };
    }

    terransoul_lib::run()
}
