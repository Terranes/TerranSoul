//! Vendor / asset detection + tiered indexing (Chunk 45.4).
//!
//! Classifies repository files into tiers:
//! - **App** — first-party application code, fully indexed (symbols + edges).
//! - **Vendor** — third-party library code, symbols-only indexing (no edges).
//! - **Asset** — non-code assets (images, fonts, audio), skipped entirely.
//! - **Generated** — auto-generated files (lock files, build artifacts), configurable.
//!
//! Classification uses:
//! 1. `.codeignore` (gitignore syntax) at repo root — user overrides.
//! 2. Auto-detected presets per build manifest.
//! 3. Path/extension heuristics.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

// ─── File Tier ──────────────────────────────────────────────────────────────

/// The classification tier for a file in the code graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileTier {
    /// First-party application code — full indexing (symbols + edges).
    App,
    /// Third-party library code — symbols-only (no edges).
    Vendor,
    /// Non-code assets — skipped entirely.
    Asset,
    /// Auto-generated files — configurable (default: symbols-only like vendor).
    Generated,
}

impl FileTier {
    /// Whether this tier should have edges extracted.
    pub fn index_edges(self) -> bool {
        matches!(self, Self::App)
    }

    /// Whether this tier should have symbols extracted.
    pub fn index_symbols(self) -> bool {
        !matches!(self, Self::Asset)
    }

    /// Whether this tier should be completely skipped during indexing.
    pub fn skip(self) -> bool {
        matches!(self, Self::Asset)
    }
}

// ─── Vendor Detector ────────────────────────────────────────────────────────

/// Classifies files into tiers based on `.codeignore`, manifest presets, and heuristics.
#[derive(Debug, Clone)]
pub struct VendorDetector {
    /// User-defined ignore patterns from `.codeignore` (gitignore syntax).
    codeignore_patterns: Vec<IgnorePattern>,
    /// Auto-detected vendor directories based on manifests found at repo root.
    vendor_dirs: Vec<String>,
    /// Auto-detected generated file patterns.
    generated_patterns: Vec<String>,
}

/// A single pattern from `.codeignore` with a tier annotation.
#[derive(Debug, Clone)]
struct IgnorePattern {
    /// The glob pattern (gitignore syntax, relative to repo root).
    pattern: String,
    /// What tier files matching this pattern belong to.
    tier: FileTier,
    /// If true, this is a negation pattern (un-ignores).
    negate: bool,
}

impl VendorDetector {
    /// Create a new detector by scanning the repo root for manifests and `.codeignore`.
    pub fn new(repo_root: &Path) -> Self {
        let codeignore_patterns = load_codeignore(repo_root);
        let vendor_dirs = detect_vendor_dirs(repo_root);
        let generated_patterns = detect_generated_patterns(repo_root);

        Self {
            codeignore_patterns,
            vendor_dirs,
            generated_patterns,
        }
    }

    /// Create a detector with no patterns (everything is `App`).
    pub fn empty() -> Self {
        Self {
            codeignore_patterns: Vec::new(),
            vendor_dirs: Vec::new(),
            generated_patterns: Vec::new(),
        }
    }

    /// Classify a file by its relative path (forward-slash separated).
    pub fn classify(&self, rel_path: &str) -> FileTier {
        // 1. Check .codeignore patterns (last match wins, like gitignore).
        let mut tier_from_ignore: Option<FileTier> = None;
        for pat in &self.codeignore_patterns {
            if matches_pattern(&pat.pattern, rel_path) {
                if pat.negate {
                    tier_from_ignore = None; // un-ignore
                } else {
                    tier_from_ignore = Some(pat.tier);
                }
            }
        }
        if let Some(t) = tier_from_ignore {
            return t;
        }

        // 2. Check auto-detected vendor directories.
        for dir in &self.vendor_dirs {
            if rel_path.starts_with(dir.as_str()) {
                return FileTier::Vendor;
            }
        }

        // 3. Check generated patterns.
        for pat in &self.generated_patterns {
            if matches_pattern(pat, rel_path) {
                return FileTier::Generated;
            }
        }

        // 4. Extension-based heuristics.
        if is_asset_extension(rel_path) {
            return FileTier::Asset;
        }

        // 5. Path-based heuristics for vendor.
        if is_vendor_path(rel_path) {
            return FileTier::Vendor;
        }

        FileTier::App
    }

    /// Classify all files and return a map of tier → file paths.
    pub fn classify_all<'a>(&self, rel_paths: &[&'a str]) -> HashMap<FileTier, Vec<&'a str>> {
        let mut map: HashMap<FileTier, Vec<&str>> = HashMap::new();
        for &path in rel_paths {
            let tier = self.classify(path);
            map.entry(tier).or_default().push(path);
        }
        map
    }
}

// ─── .codeignore parsing ────────────────────────────────────────────────────

/// Load and parse `.codeignore` from the repo root.
///
/// Format (gitignore-like with optional tier annotations):
/// ```text
/// # comment
/// vendor/         # → vendor tier (default for directory patterns)
/// *.min.js        # → vendor tier (default for minified)
/// [asset] *.png   # → asset tier
/// [generated] *.lock  # → generated tier
/// !src/vendor/important.rs  # negation: treat as app
/// ```
fn load_codeignore(repo_root: &Path) -> Vec<IgnorePattern> {
    let codeignore_path = repo_root.join(".codeignore");
    let content = match std::fs::read_to_string(&codeignore_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    parse_codeignore(&content)
}

/// Parse `.codeignore` content into patterns.
fn parse_codeignore(content: &str) -> Vec<IgnorePattern> {
    let mut patterns = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments.
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Check for negation.
        let (negate, line) = if let Some(rest) = line.strip_prefix('!') {
            (true, rest.trim())
        } else {
            (false, line)
        };

        // Check for tier annotation: [asset], [vendor], [generated], [app].
        let (tier, pattern) = if let Some(rest) = line.strip_prefix("[asset]") {
            (FileTier::Asset, rest.trim())
        } else if let Some(rest) = line.strip_prefix("[vendor]") {
            (FileTier::Vendor, rest.trim())
        } else if let Some(rest) = line.strip_prefix("[generated]") {
            (FileTier::Generated, rest.trim())
        } else if let Some(rest) = line.strip_prefix("[app]") {
            (FileTier::App, rest.trim())
        } else {
            // Default: patterns without tier annotation are vendor.
            (FileTier::Vendor, line)
        };

        // Strip inline comments.
        let pattern = pattern.split('#').next().unwrap_or(pattern).trim();

        if !pattern.is_empty() {
            patterns.push(IgnorePattern {
                pattern: pattern.to_string(),
                tier,
                negate,
            });
        }
    }

    patterns
}

// ─── Manifest-based auto-detection ─────────────────────────────────────────

/// Detect vendor directories based on manifest files at repo root.
fn detect_vendor_dirs(repo_root: &Path) -> Vec<String> {
    let mut dirs = Vec::new();

    // Node.js
    if repo_root.join("package.json").exists() {
        dirs.push("node_modules/".to_string());
        dirs.push(".npm/".to_string());
    }

    // Rust
    if repo_root.join("Cargo.toml").exists() {
        dirs.push("target/".to_string());
    }

    // Go
    if repo_root.join("go.mod").exists() {
        dirs.push("vendor/".to_string());
    }

    // Python
    if repo_root.join("requirements.txt").exists()
        || repo_root.join("pyproject.toml").exists()
        || repo_root.join("setup.py").exists()
    {
        dirs.push(".venv/".to_string());
        dirs.push("venv/".to_string());
        dirs.push("__pycache__/".to_string());
        dirs.push(".eggs/".to_string());
    }

    // Ruby
    if repo_root.join("Gemfile").exists() {
        dirs.push("vendor/bundle/".to_string());
    }

    // iOS/macOS
    if repo_root.join("Podfile").exists() {
        dirs.push("Pods/".to_string());
    }

    // Java/Maven
    if repo_root.join("pom.xml").exists() {
        dirs.push("target/".to_string());
        dirs.push(".m2/".to_string());
    }

    // Java/Gradle
    if repo_root.join("build.gradle").exists() || repo_root.join("build.gradle.kts").exists() {
        dirs.push("build/".to_string());
        dirs.push(".gradle/".to_string());
    }

    // .NET
    if repo_root.join("*.csproj").exists() || repo_root.join("*.sln").exists() {
        dirs.push("bin/".to_string());
        dirs.push("obj/".to_string());
        dirs.push("packages/".to_string());
    }

    // Deduplicate.
    dirs.sort();
    dirs.dedup();
    dirs
}

/// Detect generated-file patterns based on repo content.
fn detect_generated_patterns(repo_root: &Path) -> Vec<String> {
    let mut patterns = vec![
        // Lock files (all ecosystems).
        "package-lock.json".to_string(),
        "yarn.lock".to_string(),
        "pnpm-lock.yaml".to_string(),
        "Cargo.lock".to_string(),
        "Gemfile.lock".to_string(),
        "poetry.lock".to_string(),
        "composer.lock".to_string(),
        "go.sum".to_string(),
        // Source maps.
        "*.map".to_string(),
    ];

    // Common generated dirs.
    if repo_root.join("package.json").exists() {
        patterns.push("dist/".to_string());
        patterns.push(".next/".to_string());
        patterns.push(".nuxt/".to_string());
    }

    patterns
}

// ─── Pattern matching (simplified gitignore) ────────────────────────────────

/// Match a gitignore-style pattern against a relative path.
///
/// Supports:
/// - `*` matches anything except `/`
/// - `**` matches everything including `/`
/// - `?` matches single char
/// - Trailing `/` matches only directories (we treat any prefix match as dir match)
/// - Leading `/` anchors to repo root
fn matches_pattern(pattern: &str, rel_path: &str) -> bool {
    let (pattern, anchored) = if let Some(p) = pattern.strip_prefix('/') {
        (p, true)
    } else {
        (pattern, false)
    };

    // Directory pattern: trailing slash means match as prefix.
    let (pattern, dir_only) = if let Some(p) = pattern.strip_suffix('/') {
        (p, true)
    } else {
        (pattern, false)
    };

    if dir_only {
        // Match if the path starts with this directory.
        if anchored {
            return rel_path.starts_with(pattern)
                && (rel_path.len() == pattern.len()
                    || rel_path.as_bytes().get(pattern.len()) == Some(&b'/'));
        }
        // Non-anchored: match any path component.
        return rel_path.starts_with(pattern)
            || rel_path.contains(&format!("/{pattern}/"))
            || rel_path.starts_with(&format!("{pattern}/"));
    }

    // Exact or glob match.
    if pattern.contains("**") {
        glob_match_double_star(pattern, rel_path, anchored)
    } else if pattern.contains('*') || pattern.contains('?') {
        glob_match_simple(pattern, rel_path, anchored)
    } else {
        // Literal match.
        if anchored {
            rel_path == pattern
        } else {
            rel_path == pattern
                || rel_path.ends_with(pattern)
                || rel_path.ends_with(&format!("/{pattern}"))
        }
    }
}

/// Simple glob matching (* and ?).
fn glob_match_simple(pattern: &str, text: &str, anchored: bool) -> bool {
    if anchored {
        glob_match_segment(pattern, text)
    } else {
        // Try matching against the full path or any suffix.
        if glob_match_segment(pattern, text) {
            return true;
        }
        // Try matching against the filename only.
        if let Some(filename) = text.rsplit('/').next() {
            if glob_match_segment(pattern, filename) {
                return true;
            }
        }
        false
    }
}

/// Match a single glob segment (no `**`).
fn glob_match_segment(pattern: &str, text: &str) -> bool {
    let p_chars: Vec<char> = pattern.chars().collect();
    let t_chars: Vec<char> = text.chars().collect();
    glob_dp(&p_chars, &t_chars)
}

/// DP-based glob matching for * and ?.
fn glob_dp(pattern: &[char], text: &[char]) -> bool {
    let m = pattern.len();
    let n = text.len();
    let mut dp = vec![vec![false; n + 1]; m + 1];
    dp[0][0] = true;

    // Handle leading * patterns.
    for i in 1..=m {
        if pattern[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        } else {
            break;
        }
    }

    for i in 1..=m {
        for j in 1..=n {
            if pattern[i - 1] == '*' {
                // * cannot match '/' (single-star in gitignore).
                dp[i][j] = dp[i - 1][j] || (text[j - 1] != '/' && dp[i][j - 1]);
            } else if pattern[i - 1] == '?' {
                dp[i][j] = text[j - 1] != '/' && dp[i - 1][j - 1];
            } else {
                dp[i][j] = pattern[i - 1] == text[j - 1] && dp[i - 1][j - 1];
            }
        }
    }

    dp[m][n]
}

/// Double-star glob matching (** matches everything including /).
fn glob_match_double_star(pattern: &str, text: &str, anchored: bool) -> bool {
    // Split on ** and match segments.
    let parts: Vec<&str> = pattern.split("**").collect();
    if parts.len() == 2 {
        let prefix = parts[0];
        let suffix = parts[1].strip_prefix('/').unwrap_or(parts[1]);

        let prefix_ok = if prefix.is_empty() {
            true
        } else if anchored {
            text.starts_with(prefix)
        } else {
            text.contains(prefix)
        };

        let suffix_ok = if suffix.is_empty() {
            true
        } else {
            // Match suffix against filename or tail.
            text.ends_with(suffix)
                || text
                    .rsplit('/')
                    .next()
                    .is_some_and(|f| glob_match_segment(suffix, f))
        };

        return prefix_ok && suffix_ok;
    }

    // Fallback: treat as simple pattern.
    glob_match_simple(&pattern.replace("**", "*"), text, anchored)
}

// ─── Extension-based asset detection ────────────────────────────────────────

/// Common asset file extensions that should never be indexed.
const ASSET_EXTENSIONS: &[&str] = &[
    // Images
    "png", "jpg", "jpeg", "gif", "webp", "svg", "ico", "bmp", "tiff", "tif", "avif",
    // Fonts
    "woff", "woff2", "ttf", "otf", "eot",
    // Audio
    "mp3", "wav", "ogg", "flac", "aac", "m4a",
    // Video
    "mp4", "webm", "avi", "mov", "mkv",
    // Archives
    "zip", "tar", "gz", "bz2", "xz", "7z", "rar",
    // Binary data
    "bin", "dat", "db", "sqlite", "sqlite3",
    // Documents (non-code)
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
    // 3D models / VRM
    "vrm", "glb", "gltf", "fbx", "obj", "vrma",
    // Other binary
    "exe", "dll", "so", "dylib", "wasm",
];

fn is_asset_extension(rel_path: &str) -> bool {
    let ext = rel_path.rsplit('.').next().unwrap_or("");
    ASSET_EXTENSIONS.contains(&ext.to_lowercase().as_str())
}

// ─── Path-based vendor detection ────────────────────────────────────────────

/// Path prefixes/components that indicate vendor/third-party code.
const VENDOR_PATH_MARKERS: &[&str] = &[
    "node_modules/",
    "vendor/",
    "third_party/",
    "third-party/",
    "external/",
    "deps/",
    ".cargo/registry/",
    "Pods/",
    "__pycache__/",
    ".venv/",
    "venv/",
    "site-packages/",
    ".gradle/",
    "target/debug/",
    "target/release/",
];

fn is_vendor_path(rel_path: &str) -> bool {
    for marker in VENDOR_PATH_MARKERS {
        if rel_path.starts_with(marker) || rel_path.contains(&format!("/{marker}")) {
            return true;
        }
    }

    // Minified files.
    if rel_path.contains(".min.") {
        return true;
    }

    false
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn empty_detector_classifies_everything_as_app() {
        let det = VendorDetector::empty();
        assert_eq!(det.classify("src/main.rs"), FileTier::App);
        assert_eq!(det.classify("lib/utils.ts"), FileTier::App);
    }

    #[test]
    fn asset_extensions_detected() {
        let det = VendorDetector::empty();
        assert_eq!(det.classify("images/logo.png"), FileTier::Asset);
        assert_eq!(det.classify("fonts/inter.woff2"), FileTier::Asset);
        assert_eq!(det.classify("audio/click.mp3"), FileTier::Asset);
        assert_eq!(det.classify("models/avatar.vrm"), FileTier::Asset);
        assert_eq!(det.classify("build/app.wasm"), FileTier::Asset);
    }

    #[test]
    fn vendor_paths_detected() {
        let det = VendorDetector::empty();
        assert_eq!(det.classify("node_modules/lodash/index.js"), FileTier::Vendor);
        assert_eq!(det.classify("vendor/autoload.php"), FileTier::Vendor);
        assert_eq!(det.classify("third_party/lib.c"), FileTier::Vendor);
        assert_eq!(det.classify("src/utils.min.js"), FileTier::Vendor);
    }

    #[test]
    fn manifest_based_detection() {
        let dir = TempDir::new().unwrap();
        // Create package.json to trigger node_modules detection.
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        // Create Cargo.toml to trigger target/ detection.
        std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();

        let det = VendorDetector::new(dir.path());
        assert_eq!(det.classify("node_modules/react/index.js"), FileTier::Vendor);
        assert_eq!(det.classify("target/debug/build/foo.rs"), FileTier::Vendor);
        assert_eq!(det.classify("src/main.rs"), FileTier::App);
    }

    #[test]
    fn generated_patterns_detected() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();

        let det = VendorDetector::new(dir.path());
        assert_eq!(det.classify("package-lock.json"), FileTier::Generated);
        assert_eq!(det.classify("Cargo.lock"), FileTier::Generated);
        assert_eq!(det.classify("src/app.js.map"), FileTier::Generated);
    }

    #[test]
    fn codeignore_overrides_heuristics() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".codeignore"),
            "[app] vendor/important/\n[asset] docs/\n",
        )
        .unwrap();

        let det = VendorDetector::new(dir.path());
        // .codeignore says vendor/important/ is app.
        assert_eq!(det.classify("vendor/important/lib.rs"), FileTier::App);
        // .codeignore says docs/ is asset.
        assert_eq!(det.classify("docs/readme.md"), FileTier::Asset);
    }

    #[test]
    fn codeignore_negation() {
        let content = "[asset] *.png\n!src/icons/*.png\n";
        let patterns = parse_codeignore(content);

        let det = VendorDetector {
            codeignore_patterns: patterns,
            vendor_dirs: Vec::new(),
            generated_patterns: Vec::new(),
        };

        // *.png → asset, but src/icons/*.png is negated → falls through to heuristics.
        assert_eq!(det.classify("images/bg.png"), FileTier::Asset);
        // Negated pattern un-ignores: falls to extension heuristic which still says Asset.
        // (This is correct — negation removes the codeignore rule, but the extension still matches.)
        // To truly override, you'd use `[app] src/icons/*.png`.
    }

    #[test]
    fn codeignore_parsing() {
        let content = r#"
# This is a comment
node_modules/
[asset] *.png
[generated] *.lock
!important.lock

"#;
        let patterns = parse_codeignore(content);
        assert_eq!(patterns.len(), 4);
        assert_eq!(patterns[0].pattern, "node_modules/");
        assert_eq!(patterns[0].tier, FileTier::Vendor);
        assert!(!patterns[0].negate);
        assert_eq!(patterns[1].pattern, "*.png");
        assert_eq!(patterns[1].tier, FileTier::Asset);
        assert_eq!(patterns[2].pattern, "*.lock");
        assert_eq!(patterns[2].tier, FileTier::Generated);
        assert_eq!(patterns[3].pattern, "important.lock");
        assert!(patterns[3].negate);
    }

    #[test]
    fn file_tier_indexing_rules() {
        assert!(FileTier::App.index_symbols());
        assert!(FileTier::App.index_edges());

        assert!(FileTier::Vendor.index_symbols());
        assert!(!FileTier::Vendor.index_edges());

        assert!(!FileTier::Asset.index_symbols());
        assert!(!FileTier::Asset.index_edges());
        assert!(FileTier::Asset.skip());

        assert!(FileTier::Generated.index_symbols());
        assert!(!FileTier::Generated.index_edges());
    }

    #[test]
    fn classify_all_groups_correctly() {
        let det = VendorDetector::empty();
        let paths = &[
            "src/main.rs",
            "node_modules/foo/index.js",
            "images/logo.png",
        ];
        let map = det.classify_all(paths);

        assert_eq!(map[&FileTier::App], vec!["src/main.rs"]);
        assert_eq!(map[&FileTier::Vendor], vec!["node_modules/foo/index.js"]);
        assert_eq!(map[&FileTier::Asset], vec!["images/logo.png"]);
    }

    #[test]
    fn glob_star_matching() {
        assert!(matches_pattern("*.rs", "src/main.rs"));
        assert!(matches_pattern("*.rs", "main.rs"));
        assert!(!matches_pattern("*.rs", "src/main.ts"));
    }

    #[test]
    fn glob_double_star_matching() {
        assert!(matches_pattern("src/**/*.rs", "src/lib/utils.rs"));
        assert!(matches_pattern("**/*.min.js", "dist/app.min.js"));
        assert!(matches_pattern("**/test/**", "src/test/foo.rs"));
    }

    #[test]
    fn directory_pattern_matching() {
        assert!(matches_pattern("vendor/", "vendor/lib.rs"));
        assert!(matches_pattern("vendor/", "vendor/sub/deep.rs"));
        assert!(!matches_pattern("vendor/", "src/vendor_code.rs"));
    }

    #[test]
    fn anchored_pattern_matching() {
        assert!(matches_pattern("/src/main.rs", "src/main.rs"));
        assert!(!matches_pattern("/src/main.rs", "lib/src/main.rs"));
    }
}
