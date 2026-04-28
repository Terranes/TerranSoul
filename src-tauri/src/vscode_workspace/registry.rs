//! Self-launched VS Code window registry, persisted as JSON on disk.
//!
//! Stores `(pid, root)` tuples for every VS Code window TerranSoul has
//! itself launched. PID-liveness is verified on every read; dead
//! entries are filtered out and the file is rewritten. This avoids
//! ever returning a stale entry across an OS reboot (PIDs reset).
//!
//! Format (`<data_dir>/vscode-windows.json`):
//! ```json
//! {
//!   "version": 1,
//!   "windows": [
//!     { "pid": 47588, "root": "D:\\Git\\TerranSoul",
//!       "launched_at_ms": 1714050000000, "launched_by": "SelfLaunched" }
//!   ]
//! }
//! ```

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Source of a registered window. v1 only ever uses `SelfLaunched`;
/// `DiscoveredViaScanner` is reserved for the workspace-storage
/// scanner described in the milestones "out of scope" notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaunchSource {
    SelfLaunched,
    #[allow(dead_code)]
    DiscoveredViaScanner,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VsCodeWindow {
    pub pid: u32,
    pub root: PathBuf,
    pub launched_at_ms: i64,
    pub launched_by: LaunchSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OnDisk {
    version: u32,
    windows: Vec<VsCodeWindow>,
}

const FORMAT_VERSION: u32 = 1;
const REGISTRY_FILE: &str = "vscode-windows.json";

/// JSON-on-disk + in-memory cache of launched VS Code windows.
///
/// All public methods take `&mut self` — the registry is owned by the
/// caller and serialised internally; concurrent access from multiple
/// Tauri commands must be guarded by a `Mutex` outside this module.
#[derive(Debug, Clone)]
pub struct SelfLaunchedRegistry {
    path: PathBuf,
    windows: Vec<VsCodeWindow>,
}

impl SelfLaunchedRegistry {
    /// Load the registry from `<data_dir>/vscode-windows.json`. A
    /// missing or corrupt file yields an empty registry — corruption
    /// is non-fatal because the worst case is "we forget some
    /// windows and launch fresh ones".
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join(REGISTRY_FILE);
        let windows = Self::read_file(&path).unwrap_or_default();
        Self { path, windows }
    }

    fn read_file(path: &Path) -> Option<Vec<VsCodeWindow>> {
        if !path.exists() {
            return None;
        }
        let raw = std::fs::read_to_string(path).ok()?;
        let parsed: OnDisk = serde_json::from_str(&raw).ok()?;
        if parsed.version != FORMAT_VERSION {
            return None;
        }
        Some(parsed.windows)
    }

    /// All known windows, including any whose PID may already be dead.
    /// Use [`Self::live_windows`] when you need a liveness-filtered view.
    pub fn windows(&self) -> &[VsCodeWindow] {
        &self.windows
    }

    /// Append a freshly-launched window. The caller is responsible for
    /// invoking [`Self::save`] afterwards.
    pub fn append(&mut self, w: VsCodeWindow) {
        self.windows.push(w);
    }

    /// Remove every entry whose `pid` matches.
    pub fn forget(&mut self, pid: u32) {
        self.windows.retain(|w| w.pid != pid);
    }

    /// Drop entries for PIDs that no longer exist.
    /// Returns `true` if any entry was removed.
    pub fn prune_dead(&mut self, pid_alive: impl Fn(u32) -> bool) -> bool {
        let before = self.windows.len();
        self.windows.retain(|w| pid_alive(w.pid));
        before != self.windows.len()
    }

    /// Liveness-filtered snapshot of the current registry.
    pub fn live_windows(&self, pid_alive: impl Fn(u32) -> bool) -> Vec<VsCodeWindow> {
        self.windows
            .iter()
            .filter(|w| pid_alive(w.pid))
            .cloned()
            .collect()
    }

    /// Atomically write the registry to disk (temp file + rename).
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let on_disk = OnDisk {
            version: FORMAT_VERSION,
            windows: self.windows.clone(),
        };
        let pretty = serde_json::to_string_pretty(&on_disk)
            .map_err(std::io::Error::other)?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, pretty.as_bytes())?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// Best-effort PID liveness check using `sysinfo`. Returns `true` if
/// the OS still has a process with that ID. Refreshes only the single
/// process, not the whole system, for speed.
pub fn pid_alive(pid: u32) -> bool {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let mut sys = System::new();
    let pid = Pid::from_u32(pid);
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::new(),
    );
    sys.process(pid).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(pid: u32, root: &str) -> VsCodeWindow {
        VsCodeWindow {
            pid,
            root: PathBuf::from(root),
            launched_at_ms: 1_700_000_000_000,
            launched_by: LaunchSource::SelfLaunched,
        }
    }

    #[test]
    fn load_missing_file_yields_empty_registry() {
        let dir = tempfile::tempdir().unwrap();
        let reg = SelfLaunchedRegistry::load(dir.path());
        assert!(reg.windows().is_empty());
    }

    #[test]
    fn append_then_save_then_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = SelfLaunchedRegistry::load(dir.path());
        reg.append(fixture(123, "/some/path"));
        reg.save().unwrap();

        let loaded = SelfLaunchedRegistry::load(dir.path());
        assert_eq!(loaded.windows().len(), 1);
        assert_eq!(loaded.windows()[0].pid, 123);
    }

    #[test]
    fn forget_removes_matching_pid() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = SelfLaunchedRegistry::load(dir.path());
        reg.append(fixture(1, "/a"));
        reg.append(fixture(2, "/b"));
        reg.append(fixture(1, "/c")); // duplicate pid
        reg.forget(1);
        assert_eq!(reg.windows().len(), 1);
        assert_eq!(reg.windows()[0].pid, 2);
    }

    #[test]
    fn prune_dead_removes_only_dead_entries() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = SelfLaunchedRegistry::load(dir.path());
        reg.append(fixture(1, "/a"));
        reg.append(fixture(2, "/b"));
        reg.append(fixture(3, "/c"));
        let pruned = reg.prune_dead(|pid| pid != 2);
        assert!(pruned);
        let pids: Vec<u32> = reg.windows().iter().map(|w| w.pid).collect();
        assert_eq!(pids, vec![1, 3]);
    }

    #[test]
    fn prune_dead_returns_false_when_all_alive() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = SelfLaunchedRegistry::load(dir.path());
        reg.append(fixture(1, "/a"));
        let pruned = reg.prune_dead(|_| true);
        assert!(!pruned);
        assert_eq!(reg.windows().len(), 1);
    }

    #[test]
    fn corrupt_file_yields_empty_registry() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(REGISTRY_FILE), "not json")
            .unwrap();
        let reg = SelfLaunchedRegistry::load(dir.path());
        assert!(reg.windows().is_empty());
    }

    #[test]
    fn version_mismatch_yields_empty_registry() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(REGISTRY_FILE),
            r#"{"version":99,"windows":[]}"#,
        )
        .unwrap();
        let reg = SelfLaunchedRegistry::load(dir.path());
        assert!(reg.windows().is_empty());
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        let mut reg = SelfLaunchedRegistry::load(&nested);
        reg.append(fixture(1, "/x"));
        reg.save().unwrap();
        assert!(nested.join(REGISTRY_FILE).exists());
    }

    #[test]
    fn live_windows_filters_dead() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = SelfLaunchedRegistry::load(dir.path());
        reg.append(fixture(1, "/a"));
        reg.append(fixture(2, "/b"));
        let live = reg.live_windows(|pid| pid == 1);
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].pid, 1);
    }

    #[test]
    fn pid_alive_true_for_self() {
        // The current process must be alive.
        let me = std::process::id();
        assert!(pid_alive(me));
    }

    #[test]
    fn pid_alive_false_for_unlikely_pid() {
        // 0 is reserved on Windows, "swapper" on Linux, "kernel_task"
        // on macOS — but `sysinfo` won't surface it in the per-PID
        // refresh on most platforms. We just test a clearly-bogus
        // value; sysinfo returns None.
        assert!(!pid_alive(u32::MAX));
    }
}
