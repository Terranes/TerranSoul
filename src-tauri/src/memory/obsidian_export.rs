//! Obsidian vault export (one-way) — Chunk 18.5.
//!
//! Writes one Markdown file per long-tier memory under
//! `<vault_dir>/TerranSoul/<id>-<slug>.md` with YAML frontmatter
//! containing id, tags, importance, source_url, and created_at.
//! Idempotent: file mtime drives the "should I rewrite?" decision.
//!
//! Optionally uses PARA layout (chunk 33B.9) to organise into
//! Projects / Areas / Resources / Archive subfolders.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::store::MemoryEntry;
use crate::settings::ObsidianLayout;

/// Maximum slug length (bytes). Prevents absurdly long filenames.
const MAX_SLUG_LEN: usize = 60;

/// Convert a memory's content into a filesystem-safe slug.
pub fn slugify(content: &str) -> String {
    let slug: String = content
        .chars()
        .take(MAX_SLUG_LEN * 2) // take more chars, then truncate bytes
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == ' ' || c == '-' || c == '_' {
                Some('-')
            } else {
                None
            }
        })
        .collect();
    // Trim trailing dashes and truncate to MAX_SLUG_LEN bytes
    let trimmed = slug.trim_matches('-');
    if trimmed.len() > MAX_SLUG_LEN {
        // Find a clean break
        let truncated = &trimmed[..MAX_SLUG_LEN];
        truncated.trim_end_matches('-').to_string()
    } else {
        trimmed.to_string()
    }
}

/// Build the filename for a memory: `<id>-<slug>.md`.
pub fn filename_for(entry: &MemoryEntry) -> String {
    let slug = slugify(&entry.content);
    if slug.is_empty() {
        format!("{}.md", entry.id)
    } else {
        format!("{}-{}.md", entry.id, slug)
    }
}

/// Format a Unix-millisecond timestamp as ISO 8601 (UTC).
fn format_iso(ms: i64) -> String {
    let total_secs = ms / 1000;
    let secs_per_min = 60i64;
    let secs_per_hour = 3600i64;
    let secs_per_day = 86400i64;

    // Days since epoch → year/month/day via a direct algorithm.
    let time_of_day = total_secs.rem_euclid(secs_per_day);
    let mut day_count = (total_secs - time_of_day) / secs_per_day;

    let hours = time_of_day / secs_per_hour;
    let minutes = (time_of_day % secs_per_hour) / secs_per_min;
    let seconds = time_of_day % secs_per_min;

    // Civil date from day count (algorithm from Howard Hinnant).
    day_count += 719_468;
    let era = if day_count >= 0 {
        day_count / 146_097
    } else {
        (day_count - 146_096) / 146_097
    };
    let doe = (day_count - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, m, d, hours, minutes, seconds
    )
}

/// Render YAML frontmatter + body for a single memory.
pub fn render_markdown(entry: &MemoryEntry) -> String {
    let mut fm = String::from("---\n");
    fm.push_str(&format!("id: {}\n", entry.id));
    fm.push_str(&format!(
        "created_at: \"{}\"\n",
        format_iso(entry.created_at)
    ));
    fm.push_str(&format!("importance: {}\n", entry.importance));
    fm.push_str(&format!(
        "memory_type: \"{}\"\n",
        entry.memory_type.as_str()
    ));
    fm.push_str(&format!("tier: \"{}\"\n", entry.tier.as_str()));

    if !entry.tags.is_empty() {
        fm.push_str("tags:\n");
        for tag in entry.tags.split(',') {
            let t = tag.trim();
            if !t.is_empty() {
                fm.push_str(&format!("  - \"{}\"\n", t));
            }
        }
    }

    if let Some(ref url) = entry.source_url {
        fm.push_str(&format!("source_url: \"{}\"\n", url));
    }

    if let Some(ref hash) = entry.source_hash {
        fm.push_str(&format!("source_hash: \"{}\"\n", hash));
    }

    fm.push_str("---\n\n");
    fm.push_str(&entry.content);
    fm.push('\n');
    fm
}

/// Result of a single export run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportReport {
    /// Number of files written (new or updated).
    pub written: usize,
    /// Number of files skipped (unchanged).
    pub skipped: usize,
    /// Total long-tier memories considered.
    pub total: usize,
    /// Output directory used.
    pub output_dir: String,
}

/// PARA category for a memory entry (chunk 33B.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParaCategory {
    /// Active goal-oriented work (tag prefix `project:`)
    Projects,
    /// Ongoing responsibilities (tag prefix `personal:`)
    Areas,
    /// Reference material (tag prefixes `code:`, `domain:`, or semantic/procedural type)
    Resources,
    /// Low-importance or decayed content
    Archive,
}

impl ParaCategory {
    /// Subdirectory name for this category.
    pub fn folder_name(self) -> &'static str {
        match self {
            Self::Projects => "Projects",
            Self::Areas => "Areas",
            Self::Resources => "Resources",
            Self::Archive => "Archive",
        }
    }
}

/// Classify a memory entry into a PARA category based on its tags,
/// type, importance, and decay score.
///
/// Priority order:
/// 1. Low importance (< 2) or heavy decay (< 0.3) → Archive
/// 2. Any tag starting with `project:` → Projects
/// 3. Any tag starting with `personal:` → Areas
/// 4. Any tag starting with `code:` or `domain:`, or memory_type is
///    Fact/Procedural → Resources
/// 5. Fallback → Resources
pub fn classify_para(entry: &MemoryEntry) -> ParaCategory {
    // Archive: decayed or unimportant
    if entry.importance < 2 || entry.decay_score < 0.3 {
        return ParaCategory::Archive;
    }

    let tags_lower = entry.tags.to_lowercase();
    let tag_list: Vec<&str> = tags_lower.split(',').map(|t| t.trim()).collect();

    // Projects: active goal-oriented
    if tag_list.iter().any(|t| t.starts_with("project:")) {
        return ParaCategory::Projects;
    }

    // Areas: personal/ongoing
    if tag_list.iter().any(|t| t.starts_with("personal:")) {
        return ParaCategory::Areas;
    }

    // Resources: reference material
    if tag_list
        .iter()
        .any(|t| t.starts_with("code:") || t.starts_with("domain:"))
    {
        return ParaCategory::Resources;
    }

    // Default: Resources (reference is the safe bucket)
    ParaCategory::Resources
}

/// Determine the output subdirectory for an entry based on layout.
fn output_dir_for(base: &Path, entry: &MemoryEntry, layout: ObsidianLayout) -> PathBuf {
    match layout {
        ObsidianLayout::Flat => base.to_path_buf(),
        ObsidianLayout::Para => {
            let category = classify_para(entry);
            base.join(category.folder_name())
        }
    }
}

/// Export all long-tier memories to a vault directory.
///
/// Creates `<vault_dir>/TerranSoul/` if it doesn't exist. For each long-tier
/// memory, writes `<id>-<slug>.md` with YAML frontmatter. Skips files whose
/// mtime is >= the memory's `last_accessed` (or `created_at` if never accessed).
pub fn export_to_vault(vault_dir: &Path, entries: &[MemoryEntry]) -> Result<ExportReport, String> {
    export_to_vault_with_layout(vault_dir, entries, ObsidianLayout::Flat)
}

/// Export all long-tier memories with an explicit layout choice.
pub fn export_to_vault_with_layout(
    vault_dir: &Path,
    entries: &[MemoryEntry],
    layout: ObsidianLayout,
) -> Result<ExportReport, String> {
    let base_dir = vault_dir.join("TerranSoul");
    fs::create_dir_all(&base_dir).map_err(|e| format!("Failed to create output dir: {e}"))?;

    let long_entries: Vec<&MemoryEntry> = entries
        .iter()
        .filter(|e| e.tier == super::store::MemoryTier::Long)
        .collect();
    let total = long_entries.len();
    let mut written = 0usize;
    let mut skipped = 0usize;

    for entry in &long_entries {
        let entry_dir = output_dir_for(&base_dir, entry, layout);
        fs::create_dir_all(&entry_dir)
            .map_err(|e| format!("Failed to create dir {}: {e}", entry_dir.display()))?;
        let fname = filename_for(entry);
        let fpath = entry_dir.join(&fname);
        let content = render_markdown(entry);

        // Decide whether to skip: if file exists and its mtime is newer than
        // the memory's last modification timestamp, skip it.
        let memory_updated_ms = entry.last_accessed.unwrap_or(entry.created_at);
        if fpath.exists() {
            if let Ok(meta) = fs::metadata(&fpath) {
                if let Ok(mtime) = meta.modified() {
                    let mtime_ms = mtime
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64;
                    if mtime_ms >= memory_updated_ms {
                        skipped += 1;
                        continue;
                    }
                }
            }
        }

        fs::write(&fpath, content)
            .map_err(|e| format!("Failed to write {}: {e}", fpath.display()))?;

        // Touch the file mtime to the memory's timestamp so future runs can
        // correctly compare. Use the current time (SystemTime::now) since we
        // just wrote — this is fine because the comparison is >= memory_updated_ms.
        // The file was just written, so its mtime is now >= memory_updated_ms.
        written += 1;
    }

    Ok(ExportReport {
        written,
        skipped,
        total,
        output_dir: base_dir.to_string_lossy().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::{MemoryTier, MemoryType};
    use std::fs;
    use tempfile::TempDir;

    fn make_entry(id: i64, content: &str, tags: &str, tier: MemoryTier) -> MemoryEntry {
        MemoryEntry {
            id,
            content: content.to_string(),
            tags: tags.to_string(),
            importance: 3,
            memory_type: MemoryType::Fact,
            created_at: 1_714_000_000_000,
            last_accessed: None,
            access_count: 0,
            embedding: None,
            tier,
            decay_score: 1.0,
            session_id: None,
            parent_id: None,
            token_count: 10,
            source_url: Some("https://example.com".to_string()),
            source_hash: Some("abc123".to_string()),
            expires_at: None,
            valid_to: None,
            obsidian_path: None,
            last_exported: None,
            updated_at: None,
            origin_device: None,
            hlc_counter: None,
            confidence: 1.0,
        }
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World!"), "hello-world");
    }

    #[test]
    fn slugify_empty() {
        assert_eq!(slugify("!!!"), "");
    }

    #[test]
    fn slugify_long_content() {
        let long = "a".repeat(200);
        let slug = slugify(&long);
        assert!(slug.len() <= MAX_SLUG_LEN);
    }

    #[test]
    fn slugify_unicode_stripped() {
        assert_eq!(slugify("café résumé"), "caf-rsum");
    }

    #[test]
    fn filename_for_normal() {
        let e = make_entry(42, "User prefers Rust", "", MemoryTier::Long);
        assert_eq!(filename_for(&e), "42-user-prefers-rust.md");
    }

    #[test]
    fn filename_for_empty_content() {
        let e = make_entry(7, "!!!", "", MemoryTier::Long);
        assert_eq!(filename_for(&e), "7.md");
    }

    #[test]
    fn format_iso_epoch() {
        assert_eq!(format_iso(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn format_iso_known_date() {
        // 2024-04-25T12:00:00Z = 1714046400000 ms
        assert_eq!(format_iso(1_714_046_400_000), "2024-04-25T12:00:00Z");
    }

    #[test]
    fn render_markdown_includes_frontmatter() {
        let e = make_entry(
            1,
            "User's name is Alice",
            "personal:name,domain:law",
            MemoryTier::Long,
        );
        let md = render_markdown(&e);
        assert!(md.starts_with("---\n"));
        assert!(md.contains("id: 1"));
        assert!(md.contains("importance: 3"));
        assert!(md.contains("\"personal:name\""));
        assert!(md.contains("\"domain:law\""));
        assert!(md.contains("source_url: \"https://example.com\""));
        assert!(md.contains("User's name is Alice"));
    }

    #[test]
    fn render_markdown_no_tags() {
        let e = make_entry(2, "Some fact", "", MemoryTier::Long);
        let md = render_markdown(&e);
        assert!(!md.contains("tags:"));
    }

    #[test]
    fn export_creates_dir_and_writes_files() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![
            make_entry(1, "Fact one", "personal:name", MemoryTier::Long),
            make_entry(2, "Fact two", "domain:law", MemoryTier::Long),
            make_entry(3, "Short term note", "", MemoryTier::Short),
        ];
        let report = export_to_vault(tmp.path(), &entries).unwrap();
        assert_eq!(report.total, 2); // only long-tier
        assert_eq!(report.written, 2);
        assert_eq!(report.skipped, 0);

        let out_dir = tmp.path().join("TerranSoul");
        assert!(out_dir.exists());
        let files: Vec<_> = fs::read_dir(&out_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn export_skips_unchanged() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![make_entry(1, "Fact one", "", MemoryTier::Long)];

        // First export writes
        let r1 = export_to_vault(tmp.path(), &entries).unwrap();
        assert_eq!(r1.written, 1);

        // Second export skips (mtime >= created_at)
        let r2 = export_to_vault(tmp.path(), &entries).unwrap();
        assert_eq!(r2.skipped, 1);
        assert_eq!(r2.written, 0);
    }

    #[test]
    fn export_rewrites_when_accessed() {
        let tmp = TempDir::new().unwrap();
        let mut entry = make_entry(1, "Fact one", "", MemoryTier::Long);

        // First export
        let _ = export_to_vault(tmp.path(), &[entry.clone()]).unwrap();

        // Simulate an access in the future — set last_accessed far ahead
        entry.last_accessed = Some(entry.created_at + 999_999_999_000);
        let r = export_to_vault(tmp.path(), &[entry]).unwrap();
        assert_eq!(r.written, 1);
    }

    #[test]
    fn export_working_tier_excluded() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![make_entry(1, "Working note", "", MemoryTier::Working)];
        let report = export_to_vault(tmp.path(), &entries).unwrap();
        assert_eq!(report.total, 0);
        assert_eq!(report.written, 0);
    }

    // ── PARA layout tests (chunk 33B.9) ────────────────────────────

    #[test]
    fn classify_para_project_tag() {
        let e = make_entry(1, "Sprint goal", "project:terransoul", MemoryTier::Long);
        assert_eq!(classify_para(&e), ParaCategory::Projects);
    }

    #[test]
    fn classify_para_personal_tag() {
        let e = make_entry(
            2,
            "User likes coffee",
            "personal:preferences",
            MemoryTier::Long,
        );
        assert_eq!(classify_para(&e), ParaCategory::Areas);
    }

    #[test]
    fn classify_para_code_tag() {
        let e = make_entry(3, "Rust borrow checker", "code:rust", MemoryTier::Long);
        assert_eq!(classify_para(&e), ParaCategory::Resources);
    }

    #[test]
    fn classify_para_domain_tag() {
        let e = make_entry(4, "ML embeddings", "domain:ml", MemoryTier::Long);
        assert_eq!(classify_para(&e), ParaCategory::Resources);
    }

    #[test]
    fn classify_para_no_tag_defaults_resources() {
        let e = make_entry(5, "Some fact", "", MemoryTier::Long);
        assert_eq!(classify_para(&e), ParaCategory::Resources);
    }

    #[test]
    fn classify_para_low_importance_archive() {
        let mut e = make_entry(6, "Old note", "project:done", MemoryTier::Long);
        e.importance = 1;
        assert_eq!(classify_para(&e), ParaCategory::Archive);
    }

    #[test]
    fn classify_para_decayed_archive() {
        let mut e = make_entry(7, "Faded fact", "project:active", MemoryTier::Long);
        e.decay_score = 0.1;
        assert_eq!(classify_para(&e), ParaCategory::Archive);
    }

    #[test]
    fn classify_para_priority_project_over_personal() {
        // Project tag wins when both are present
        let e = make_entry(
            8,
            "Mixed",
            "project:terransoul,personal:preferences",
            MemoryTier::Long,
        );
        assert_eq!(classify_para(&e), ParaCategory::Projects);
    }

    #[test]
    fn export_para_routes_to_subfolders() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![
            make_entry(1, "Project note", "project:alpha", MemoryTier::Long),
            make_entry(2, "Area note", "personal:health", MemoryTier::Long),
            make_entry(3, "Resource note", "code:rust", MemoryTier::Long),
            {
                let mut e = make_entry(4, "Archive note", "project:legacy", MemoryTier::Long);
                e.importance = 1;
                e
            },
        ];
        let report =
            export_to_vault_with_layout(tmp.path(), &entries, ObsidianLayout::Para).unwrap();
        assert_eq!(report.total, 4);
        assert_eq!(report.written, 4);

        let base = tmp.path().join("TerranSoul");
        assert!(base.join("Projects").join("1-project-note.md").exists());
        assert!(base.join("Areas").join("2-area-note.md").exists());
        assert!(base.join("Resources").join("3-resource-note.md").exists());
        assert!(base.join("Archive").join("4-archive-note.md").exists());
    }

    #[test]
    fn export_flat_layout_unchanged() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![make_entry(
            1,
            "Project note",
            "project:alpha",
            MemoryTier::Long,
        )];
        let report =
            export_to_vault_with_layout(tmp.path(), &entries, ObsidianLayout::Flat).unwrap();
        assert_eq!(report.written, 1);
        // Flat layout: file is in TerranSoul/, not TerranSoul/Projects/
        assert!(tmp
            .path()
            .join("TerranSoul")
            .join("1-project-note.md")
            .exists());
        assert!(!tmp.path().join("TerranSoul").join("Projects").exists());
    }
}
