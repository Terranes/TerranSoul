#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

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

    let is_mcp_app = std::env::args().any(|a| a == "--mcp-app");
    let is_mcp_tray = std::env::args().any(|a| a == "--mcp-tray");

    // ── Single-instance enforcement for MCP modes ─────────────────────────
    // Prevent duplicate tray icons / port conflicts by checking the PID
    // file written by the launch script. If another instance is alive,
    // exit immediately instead of spawning a duplicate.
    if is_mcp_app || is_mcp_tray {
        let mcp_data = resolve_mcp_data_dir();
        let pid_path = mcp_data.join("self_improve_mcp_process.pid");
        if pid_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&pid_path) {
                if let Ok(pid) = contents.trim().parse::<u32>() {
                    if pid != std::process::id() && is_process_running(pid) {
                        eprintln!(
                            "[mcp] another MCP instance is already running (pid {pid}); exiting."
                        );
                        std::process::exit(0);
                    }
                }
            }
        }
    }

    // CLI flag: `terransoul --mcp-app` runs the **full Tauri app** in
    // MCP mode so developers can visually observe the brain/RAG/memory
    // surface live. Same UI as the normal app, but the bottom-left badge
    // reads "MCP", state lives in `<repo>/mcp-data/`, and the MCP HTTP
    // server auto-binds to port 7423.
    if is_mcp_app {
        // SAFETY: set before any other thread spawns. We are still the
        // sole thread on `main`. The flag is read at Tauri setup time.
        unsafe { std::env::set_var("TERRANSOUL_MCP_APP_MODE", "1") };
    }

    // CLI flag: `terransoul --mcp-tray` starts the **full Tauri/Vue UI** in
    // MCP mode with the main window initially hidden in the system tray.
    // The tray menu exposes "Show UI" so users can reopen the complete app
    // shell for brain config, MCP config, memory, and graph panels while
    // keeping the MCP HTTP server alive for coding agents.
    if is_mcp_tray {
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

/// Resolve the MCP data directory (mirrors lib.rs logic).
fn resolve_mcp_data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("TERRANSOUL_MCP_DATA_DIR") {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("mcp-data")
}

/// Check whether a process with the given PID is still running.
fn is_process_running(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // kill -0 checks liveness without sending a signal.
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .is_ok_and(|o| o.status.success())
    }
}
