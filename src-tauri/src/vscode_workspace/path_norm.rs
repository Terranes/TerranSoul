//! Path canonicalisation + case-folding for cross-platform prefix matching.
//!
//! Windows filesystems are case-insensitive but `Path::starts_with`
//! is case-sensitive, so we lowercase paths on Windows before
//! comparison. macOS HFS+ / APFS is *usually* case-insensitive (same
//! treatment). Linux is case-sensitive both ways — no fold needed.

use std::path::{Path, PathBuf};

/// Canonicalise `p` (resolve `..`, follow symlinks). On Windows this
/// also strips the verbatim UNC prefix (`\\?\`) that
/// `std::fs::canonicalize` adds, because user-facing paths and editor
/// command lines almost never include it.
pub fn canonicalise(p: &Path) -> std::io::Result<PathBuf> {
    let canonical = std::fs::canonicalize(p)?;
    Ok(strip_verbatim(canonical))
}

/// On Windows, `\\?\C:\foo` → `C:\foo`. Pass-through everywhere else.
fn strip_verbatim(p: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy().into_owned();
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            // \\?\UNC\server\share → \\server\share
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    p
}

/// Returns `true` if `target` equals `root` after platform-appropriate
/// case folding.
pub fn paths_equal(root: &Path, target: &Path) -> bool {
    fold(root) == fold(target)
}

/// Returns `true` if `target` lives inside `root` (or equals it),
/// after platform-appropriate case folding.
pub fn target_inside_root(root: &Path, target: &Path) -> bool {
    let r = fold(root);
    let t = fold(target);
    PathBuf::from(&t).starts_with(&r)
}

/// Number of path components in `target` *below* `root`. Returns 0
/// when they are equal, `usize::MAX` when `target` isn't inside `root`.
pub fn depth_below(root: &Path, target: &Path) -> usize {
    if !target_inside_root(root, target) {
        return usize::MAX;
    }
    let r_components = root.components().count();
    let t_components = target.components().count();
    t_components.saturating_sub(r_components)
}

/// Number of path components in `root` (used as a tie-breaker — deeper
/// roots win when two registry windows both contain the target).
pub fn root_specificity(root: &Path) -> usize {
    root.components().count()
}

fn fold(p: &Path) -> String {
    let s = p.to_string_lossy().into_owned();
    if cfg!(any(windows, target_os = "macos")) {
        s.to_lowercase()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_paths_match() {
        assert!(paths_equal(Path::new("/a/b/c"), Path::new("/a/b/c")));
    }

    #[test]
    #[cfg(windows)]
    fn windows_case_insensitive_equality() {
        assert!(paths_equal(
            Path::new(r"D:\Git\TerranSoul"),
            Path::new(r"d:\git\terransoul")
        ));
    }

    #[test]
    fn ancestor_matches_child() {
        assert!(target_inside_root(Path::new("/a/b"), Path::new("/a/b/c/d")));
    }

    #[test]
    fn unrelated_paths_dont_match() {
        assert!(!target_inside_root(Path::new("/a/b"), Path::new("/x/y")));
    }

    #[test]
    fn equal_paths_dont_match_as_ancestor_of_child() {
        // target_inside_root accepts equality (subset semantics)
        assert!(target_inside_root(Path::new("/a/b"), Path::new("/a/b")));
    }

    #[test]
    fn depth_below_basic() {
        assert_eq!(depth_below(Path::new("/a/b"), Path::new("/a/b/c/d")), 2);
        assert_eq!(depth_below(Path::new("/a/b"), Path::new("/a/b")), 0);
        assert_eq!(
            depth_below(Path::new("/a/b"), Path::new("/x/y")),
            usize::MAX
        );
    }

    #[test]
    fn root_specificity_counts_components() {
        let s_short = root_specificity(Path::new("/a"));
        let s_long = root_specificity(Path::new("/a/b/c"));
        assert!(s_long > s_short);
    }

    #[test]
    #[cfg(windows)]
    fn windows_ancestor_case_insensitive() {
        assert!(target_inside_root(
            Path::new(r"D:\Git"),
            Path::new(r"d:\git\terransoul\src")
        ));
    }
}
