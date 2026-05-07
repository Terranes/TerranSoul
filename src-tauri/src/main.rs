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

    // CLI flag: `terransoul --mcp-setup` detects AI editor config
    // directories and writes MCP server entries for the headless runner.
    if std::env::args().any(|a| a == "--mcp-setup") {
        if let Err(e) = terransoul_lib::run_mcp_setup() {
            eprintln!("[mcp-setup] fatal: {e}");
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

    // CLI flag: `terransoul --mcp-tray` starts the **full Tauri/Vue UI** in
    // MCP mode with the main window initially hidden in the system tray.
    // The tray menu exposes "Show UI" so users can reopen the complete app
    // shell for brain config, MCP config, memory, and graph panels while
    // keeping the MCP HTTP server alive for coding agents.
    if std::env::args().any(|a| a == "--mcp-tray") {
        // SAFETY: set before any other thread spawns.
        unsafe {
            std::env::set_var("TERRANSOUL_MCP_APP_MODE", "1");
            std::env::set_var("TERRANSOUL_MCP_TRAY_MODE", "1");
        }
    }

    // CLI flag: `terransoul --resume <name>` tells the headless MCP to
    // resume a named session from the session registry instead of
    // creating a new one.
    {
        let args: Vec<String> = std::env::args().collect();
        if let Some(pos) = args.iter().position(|a| a == "--resume") {
            if let Some(name) = args.get(pos + 1) {
                // SAFETY: set before any other thread spawns.
                unsafe {
                    std::env::set_var("TERRANSOUL_MCP_RESUME", name);
                }
            }
        }
    }

    terransoul_lib::run()
}
