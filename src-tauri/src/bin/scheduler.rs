//! Standalone maintenance scheduler daemon (Chunk 33B.10).
//!
//! Runs the TerranSoul brain maintenance loop (decay, GC, tier promotion,
//! edge extraction, Obsidian export) as a long-running process without
//! requiring the Tauri GUI or MCP server.
//!
//! Intended for headless/server deployments where the companion memory
//! should stay healthy even when no desktop app is running.
//!
//! # Usage
//!
//! ```sh
//! # Use default platform data directory:
//! terransoul-scheduler
//!
//! # Override data directory:
//! TERRANSOUL_SCHEDULER_DATA_DIR=/path/to/data terransoul-scheduler
//!
//! # Custom tick interval (default 3600 seconds = 1 hour):
//! TERRANSOUL_SCHEDULER_INTERVAL_SECS=1800 terransoul-scheduler
//! ```
//!
//! Exits cleanly on SIGTERM / Ctrl+C.

fn main() {
    if let Err(e) = terransoul_lib::run_scheduler() {
        eprintln!("[scheduler] fatal: {e}");
        std::process::exit(1);
    }
}
