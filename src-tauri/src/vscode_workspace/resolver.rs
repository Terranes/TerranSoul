//! Pure resolver: pick the best VS Code window for a given target.
//!
//! Algorithm (matches the milestones spec for Chunk 15.10):
//!
//! 1. For every registered window, classify against `target`:
//!    - `Exact` if `window.root == target`
//!    - `Ancestor { depth }` if `target` lives inside `window.root`
//!    - skip otherwise
//! 2. PID-liveness filter — drop candidates whose PID is dead.
//! 3. Prefer `Exact`. If multiple, pick the most-recently launched.
//! 4. Else pick the `Ancestor` with the deepest root (most components,
//!    "most-children-near-target" per the spec). Ties broken by
//!    most-recent launch.
//! 5. Else return `None`.

use std::path::Path;

use super::path_norm;
use super::registry::VsCodeWindow;

/// What the resolver decided about a target path.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum WindowChoice {
    /// Existing window opened on exactly the same folder as `target`.
    Exact { pid: u32 },
    /// Existing window opened on an ancestor folder containing `target`.
    /// `depth` = number of components `target` is below the window root.
    Ancestor { pid: u32, depth: usize },
    /// No registered window can serve `target` — caller should launch new.
    None,
}

/// Pure picker. `pid_alive` is injected so we can unit-test the
/// liveness filter without spawning processes.
///
/// `target` and every `window.root` should already be canonicalised
/// by the caller (via `path_norm::canonicalise`).
pub fn pick_window(
    target: &Path,
    windows: &[VsCodeWindow],
    pid_alive: impl Fn(u32) -> bool,
) -> WindowChoice {
    let mut exact: Option<&VsCodeWindow> = None;
    let mut best_ancestor: Option<(&VsCodeWindow, usize)> = None;

    for w in windows {
        if !pid_alive(w.pid) {
            continue;
        }

        if path_norm::paths_equal(&w.root, target) {
            // Most-recent wins on duplicate exact match.
            match exact {
                Some(prev) if prev.launched_at_ms >= w.launched_at_ms => {}
                _ => exact = Some(w),
            }
        } else if path_norm::target_inside_root(&w.root, target) {
            let depth = path_norm::depth_below(&w.root, target);
            let specificity = path_norm::root_specificity(&w.root);
            let candidate = (w, depth);

            best_ancestor = match best_ancestor {
                None => Some(candidate),
                Some((prev_w, _prev_depth)) => {
                    let prev_specificity = path_norm::root_specificity(&prev_w.root);
                    // Deeper root wins (more children near target).
                    // Tie-broken by most-recent launch.
                    if specificity > prev_specificity
                        || (specificity == prev_specificity
                            && w.launched_at_ms > prev_w.launched_at_ms)
                    {
                        Some(candidate)
                    } else {
                        Some((prev_w, _prev_depth))
                    }
                }
            };
        }
    }

    if let Some(w) = exact {
        return WindowChoice::Exact { pid: w.pid };
    }
    if let Some((w, depth)) = best_ancestor {
        return WindowChoice::Ancestor { pid: w.pid, depth };
    }
    WindowChoice::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn win(pid: u32, root: &str, launched_at_ms: i64) -> VsCodeWindow {
        VsCodeWindow {
            pid,
            root: PathBuf::from(root),
            launched_at_ms,
            launched_by: super::super::registry::LaunchSource::SelfLaunched,
        }
    }

    fn always_alive(_pid: u32) -> bool {
        true
    }

    fn always_dead(_pid: u32) -> bool {
        false
    }

    #[cfg(not(windows))]
    const ROOT_TS: &str = "/d/git/terransoul";
    #[cfg(not(windows))]
    const ROOT_GIT: &str = "/d/git";
    #[cfg(not(windows))]
    const ROOT_DRIVE: &str = "/d";
    #[cfg(not(windows))]
    const TARGET_TS_SRC: &str = "/d/git/terransoul/src";
    #[cfg(not(windows))]
    const TARGET_TS_DEEP: &str = "/d/git/terransoul/src/components";
    #[cfg(not(windows))]
    const TARGET_TS: &str = "/d/git/terransoul";
    #[cfg(not(windows))]
    const TARGET_OTHER: &str = "/x/y";

    #[cfg(windows)]
    const ROOT_TS: &str = r"D:\Git\TerranSoul";
    #[cfg(windows)]
    const ROOT_GIT: &str = r"D:\Git";
    #[cfg(windows)]
    const ROOT_DRIVE: &str = r"D:\";
    #[cfg(windows)]
    const TARGET_TS_SRC: &str = r"D:\Git\TerranSoul\src";
    #[cfg(windows)]
    const TARGET_TS_DEEP: &str = r"D:\Git\TerranSoul\src\components";
    #[cfg(windows)]
    const TARGET_TS: &str = r"D:\Git\TerranSoul";
    #[cfg(windows)]
    const TARGET_OTHER: &str = r"X:\y";

    #[test]
    fn empty_registry_returns_none() {
        let choice = pick_window(Path::new(TARGET_TS), &[], always_alive);
        assert_eq!(choice, WindowChoice::None);
    }

    #[test]
    fn exact_match_wins() {
        let windows = vec![win(123, ROOT_TS, 1000)];
        let choice = pick_window(Path::new(TARGET_TS), &windows, always_alive);
        assert_eq!(choice, WindowChoice::Exact { pid: 123 });
    }

    #[test]
    fn ancestor_match_with_depth() {
        let windows = vec![win(99, ROOT_GIT, 1000)];
        let choice = pick_window(Path::new(TARGET_TS_SRC), &windows, always_alive);
        match choice {
            WindowChoice::Ancestor { pid, depth } => {
                assert_eq!(pid, 99);
                assert!(depth >= 1, "depth was {depth}");
            }
            other => panic!("expected Ancestor, got {other:?}"),
        }
    }

    #[test]
    fn deepest_ancestor_wins() {
        let windows = vec![win(99, ROOT_GIT, 1000), win(42, ROOT_TS, 500)];
        let choice = pick_window(Path::new(TARGET_TS_SRC), &windows, always_alive);
        match choice {
            WindowChoice::Ancestor { pid, .. } => assert_eq!(pid, 42),
            other => panic!("expected Ancestor pid=42, got {other:?}"),
        }
    }

    #[test]
    fn three_window_chain_picks_deepest() {
        let windows = vec![
            win(1, ROOT_DRIVE, 100),
            win(2, ROOT_GIT, 200),
            win(3, ROOT_TS, 300),
        ];
        let choice = pick_window(Path::new(TARGET_TS_DEEP), &windows, always_alive);
        match choice {
            WindowChoice::Ancestor { pid, .. } => assert_eq!(pid, 3),
            other => panic!("expected pid=3, got {other:?}"),
        }
    }

    #[test]
    fn dead_pids_are_filtered() {
        let windows = vec![win(42, ROOT_TS, 1000)];
        let choice = pick_window(Path::new(TARGET_TS), &windows, always_dead);
        assert_eq!(choice, WindowChoice::None);
    }

    #[test]
    fn dead_exact_falls_through_to_live_ancestor() {
        let windows = vec![
            win(42, ROOT_TS, 1000), // dead exact
            win(99, ROOT_GIT, 500), // live ancestor
        ];
        let choice = pick_window(Path::new(TARGET_TS), &windows, |pid| pid == 99);
        match choice {
            WindowChoice::Ancestor { pid, .. } => assert_eq!(pid, 99),
            other => panic!("expected ancestor pid=99, got {other:?}"),
        }
    }

    #[test]
    fn unrelated_target_returns_none() {
        let windows = vec![win(42, ROOT_TS, 1000)];
        let choice = pick_window(Path::new(TARGET_OTHER), &windows, always_alive);
        assert_eq!(choice, WindowChoice::None);
    }

    #[test]
    fn duplicate_exact_picks_more_recent() {
        let windows = vec![win(1, ROOT_TS, 100), win(2, ROOT_TS, 200)];
        let choice = pick_window(Path::new(TARGET_TS), &windows, always_alive);
        assert_eq!(choice, WindowChoice::Exact { pid: 2 });
    }

    #[test]
    fn tie_in_specificity_picks_more_recent_ancestor() {
        let windows = vec![win(1, ROOT_GIT, 100), win(2, ROOT_GIT, 200)];
        let choice = pick_window(Path::new(TARGET_TS_SRC), &windows, always_alive);
        match choice {
            WindowChoice::Ancestor { pid, .. } => assert_eq!(pid, 2),
            other => panic!("expected pid=2, got {other:?}"),
        }
    }

    #[test]
    fn exact_beats_ancestor_even_when_ancestor_is_more_recent() {
        let windows = vec![
            win(1, ROOT_TS, 100),   // exact, older
            win(2, ROOT_GIT, 9999), // ancestor, newer
        ];
        let choice = pick_window(Path::new(TARGET_TS), &windows, always_alive);
        assert_eq!(choice, WindowChoice::Exact { pid: 1 });
    }
}
