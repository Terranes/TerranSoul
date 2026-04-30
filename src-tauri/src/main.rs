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

    terransoul_lib::run()
}
