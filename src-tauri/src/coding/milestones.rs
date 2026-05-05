//! Tiny parser for `rules/milestones.md` chunk tables.
//!
//! The file is mostly free-form prose, but each phase contains a markdown
//! table whose rows look like:
//!
//! ```text
//! | 25.4 | **Autonomous coding loop MVP** ... | not-started | Notes ... |
//! ```
//!
//! We only need three columns — id, title, status — and we tolerate any
//! extras so the parser keeps working when columns are added later.

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ChunkRow {
    pub id: String,
    pub title: String,
    pub status: String,
}

/// Parse all chunk rows from a milestones.md document.
pub fn parse_chunks(markdown: &str) -> Vec<ChunkRow> {
    let mut out = Vec::new();
    let mut columns = MilestoneColumns::default();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        // Strip the leading + trailing pipes, then split.
        let inner = trimmed.trim_start_matches('|').trim_end_matches('|');
        let cols: Vec<&str> = inner.split('|').map(|c| c.trim()).collect();
        if cols.len() < 3 {
            continue;
        }
        if let Some(next_columns) = MilestoneColumns::from_header(&cols) {
            columns = next_columns;
            continue;
        }
        // Skip separator rows: `---|---|---|---`.
        let Some(id) = cols.get(columns.id).copied() else {
            continue;
        };
        if id.is_empty() || id == "#" || id.chars().all(|c| c == '-' || c == ':') {
            continue;
        }
        // The id column is supposed to be a chunk number like `25.4` or
        // `1.11`. Skip rows whose first col is plainly not an id.
        if !id.chars().any(|c| c.is_ascii_digit()) {
            continue;
        }
        out.push(ChunkRow {
            id: id.to_string(),
            title: cols.get(columns.title).unwrap_or(&"").to_string(),
            status: cols.get(columns.status).unwrap_or(&"").to_string(),
        });
    }
    out
}

#[derive(Debug, Clone, Copy)]
struct MilestoneColumns {
    id: usize,
    title: usize,
    status: usize,
}

impl Default for MilestoneColumns {
    fn default() -> Self {
        Self {
            id: 0,
            title: 1,
            status: 2,
        }
    }
}

impl MilestoneColumns {
    fn from_header(cols: &[&str]) -> Option<Self> {
        let normalized: Vec<String> = cols.iter().map(|col| normalize_header(col)).collect();
        let id = normalized
            .iter()
            .position(|col| matches!(col.as_str(), "id" | "#"))?;
        let title = normalized
            .iter()
            .position(|col| matches!(col.as_str(), "title" | "chunk"))?;
        let status = normalized.iter().position(|col| col == "status")?;
        Some(Self { id, title, status })
    }
}

fn normalize_header(value: &str) -> String {
    value.trim_matches('*').trim().to_ascii_lowercase()
}

/// First chunk whose status is `not-started`.
pub fn next_not_started(rows: &[ChunkRow]) -> Option<&ChunkRow> {
    rows.iter()
        .find(|r| r.status.eq_ignore_ascii_case("not-started"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
### Phase 25 — Self-Improve Autonomous Coding System

| # | Chunk | Status | Notes |
|---|---|---|---|
| 25.2 | **GitHub repo binding** | not-started | Use git2 |
| 25.3 | **Coding LLM client** | in-progress | reqwest based |
| 25.4 | **Autonomous loop MVP** | not-started | Subagent-style |
"#;

    const CURRENT_ORDER_SAMPLE: &str = r#"
| ID | Status | Title | Goal |
|---|---|---|---|
| 32.3 | not-started | Self-improve chunk completion + retry | Archive successful chunks. |
| 32.4 | in-progress | Self-improve isolated patch auto-merge | Apply isolated patches. |
"#;

    #[test]
    fn parses_three_chunk_rows() {
        let rows = parse_chunks(SAMPLE);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].id, "25.2");
        assert_eq!(rows[0].title, "**GitHub repo binding**");
        assert_eq!(rows[0].status, "not-started");
        assert_eq!(rows[1].status, "in-progress");
    }

    #[test]
    fn parses_current_id_status_title_order() {
        let rows = parse_chunks(CURRENT_ORDER_SAMPLE);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "32.3");
        assert_eq!(rows[0].title, "Self-improve chunk completion + retry");
        assert_eq!(rows[0].status, "not-started");
        assert_eq!(next_not_started(&rows).unwrap().id, "32.3");
    }

    #[test]
    fn next_not_started_skips_in_progress() {
        let rows = parse_chunks(SAMPLE);
        let next = next_not_started(&rows).unwrap();
        assert_eq!(next.id, "25.2");
    }

    #[test]
    fn skips_header_and_separator_rows() {
        let rows = parse_chunks(SAMPLE);
        for r in &rows {
            assert!(!r.id.is_empty());
            assert!(!r.id.starts_with('-'));
            assert_ne!(r.id, "#");
        }
    }

    #[test]
    fn empty_input_returns_empty_vec() {
        assert!(parse_chunks("").is_empty());
        assert!(parse_chunks("just prose, no table").is_empty());
    }
}
