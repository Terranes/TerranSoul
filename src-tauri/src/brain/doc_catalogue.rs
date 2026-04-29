//! Parse the recommended LLM catalogue from `docs/brain-advanced-design.md`.
//!
//! The markdown contains two machine-readable tables between HTML-comment
//! markers (`<!-- BEGIN MODEL_CATALOGUE -->` / `<!-- END MODEL_CATALOGUE -->`
//! and `<!-- BEGIN TOP_PICKS -->` / `<!-- END TOP_PICKS -->`).  This module
//! extracts those tables and produces the same [`ModelRecommendation`] shape
//! used by the hardcoded fallback in [`super::model_recommender`].

use std::collections::HashMap;

use super::model_recommender::ModelRecommendation;
use super::system_info::RamTier;

/// Parsed result from the design-doc catalogue.
#[derive(Debug, Clone)]
pub struct ParsedCatalogue {
    /// Local models (not cloud).
    pub local_models: Vec<ModelRecommendation>,
    /// Cloud-only models.
    pub cloud_models: Vec<ModelRecommendation>,
    /// RAM tier → preferred model_tag.
    pub top_picks: HashMap<String, String>,
}

/// Extract the model catalogue + top-picks from a markdown string.
///
/// Returns `None` if the required markers are missing or the tables are
/// unparseable (caller should fall back to the hardcoded catalogue).
pub fn parse_catalogue(markdown: &str) -> Option<ParsedCatalogue> {
    let models_block = extract_between(markdown, "<!-- BEGIN MODEL_CATALOGUE -->", "<!-- END MODEL_CATALOGUE -->")?;
    let picks_block = extract_between(markdown, "<!-- BEGIN TOP_PICKS -->", "<!-- END TOP_PICKS -->")?;

    let mut local_models = Vec::new();
    let mut cloud_models = Vec::new();

    for row in parse_table_rows(models_block) {
        // Expect columns: model_tag | display_name | description | required_ram_mb | is_cloud
        if row.len() < 5 {
            continue;
        }
        let model_tag = row[0].to_string();
        let display_name = row[1].to_string();
        let description = row[2].to_string();
        let required_ram_mb: u64 = row[3].parse().ok()?;
        let is_cloud = row[4].eq_ignore_ascii_case("true");

        let rec = ModelRecommendation {
            model_tag,
            display_name,
            description,
            required_ram_mb,
            is_top_pick: false,
            is_cloud,
        };

        if is_cloud {
            cloud_models.push(rec);
        } else {
            local_models.push(rec);
        }
    }

    if local_models.is_empty() {
        return None;
    }

    let mut top_picks = HashMap::new();
    for row in parse_table_rows(picks_block) {
        // Expect columns: tier | model_tag
        if row.len() < 2 {
            continue;
        }
        top_picks.insert(row[0].to_string(), row[1].to_string());
    }

    Some(ParsedCatalogue {
        local_models,
        cloud_models,
        top_picks,
    })
}

/// Build a ranked recommendation list from a parsed catalogue,
/// applying the same RAM-tier filtering and sorting logic as the
/// hardcoded [`super::model_recommender::recommend`].
pub fn recommend_from_catalogue(
    total_ram_mb: u64,
    catalogue: &ParsedCatalogue,
) -> Vec<ModelRecommendation> {
    let tier = RamTier::from_mb(total_ram_mb);

    let mut candidates: Vec<ModelRecommendation> = catalogue
        .local_models
        .iter()
        .filter(|m| m.required_ram_mb <= total_ram_mb)
        .cloned()
        .collect();

    let mut cloud = catalogue.cloud_models.clone();

    // If nothing fits, include the smallest model + cloud.
    if candidates.is_empty() {
        if let Some(smallest) = catalogue
            .local_models
            .iter()
            .min_by_key(|m| m.required_ram_mb)
        {
            let mut fallback = smallest.clone();
            fallback.is_top_pick = true;
            candidates.push(fallback);
        }
        candidates.append(&mut cloud);
        return candidates;
    }

    // Resolve the top-pick for this tier.
    let tier_name = match tier {
        RamTier::VeryHigh => "VeryHigh",
        RamTier::High => "High",
        RamTier::Medium => "Medium",
        RamTier::Low => "Low",
        RamTier::VeryLow => "VeryLow",
    };

    if let Some(top_tag) = catalogue.top_picks.get(tier_name) {
        for m in &mut candidates {
            if m.model_tag == *top_tag {
                m.is_top_pick = true;
            }
        }
    }

    // Ensure exactly one top pick.
    if !candidates.iter().any(|m| m.is_top_pick) {
        if let Some(first) = candidates.first_mut() {
            first.is_top_pick = true;
        }
    }

    // Sort: top pick first, then by required_ram_mb descending.
    candidates.sort_by(|a, b| {
        b.is_top_pick
            .cmp(&a.is_top_pick)
            .then(b.required_ram_mb.cmp(&a.required_ram_mb))
    });

    candidates.append(&mut cloud);
    candidates
}

// ── helpers ──────────────────────────────────────────────────────────

/// Extract the text between two marker strings (exclusive).
fn extract_between<'a>(text: &'a str, begin: &str, end: &str) -> Option<&'a str> {
    let start = text.find(begin)? + begin.len();
    let stop = text[start..].find(end)? + start;
    Some(text[start..stop].trim())
}

/// Parse markdown table rows, skipping the header and separator rows.
/// Returns a vec of rows, each row being a vec of trimmed cell strings.
fn parse_table_rows(table_block: &str) -> Vec<Vec<&str>> {
    let mut rows = Vec::new();
    let mut lines = table_block.lines();

    // Skip the header row.
    let _header = lines.next();
    // Skip the separator row (|---|---|...).
    let _sep = lines.next();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = trimmed
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if !cells.is_empty() {
            rows.push(cells);
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MD: &str = r#"
# Some heading

<!-- BEGIN MODEL_CATALOGUE -->
| model_tag | display_name | description | required_ram_mb | is_cloud |
|---|---|---|---|---|
| gemma4:31b | Gemma 4 31B | Dense 30.7B flagship. | 24576 | false |
| gemma4:e4b | Gemma 4 E4B | Edge 4.5B. | 12288 | false |
| gemma4:e2b | Gemma 4 E2B | Edge 2.3B. | 8192 | false |
| gemma3:1b | Gemma 3 1B | Ultra-lightweight. | 2048 | false |
| tinyllama | TinyLlama 1.1B | Minimal model. | 2048 | false |
| kimi-k2.6:cloud | Kimi K2.6 (Cloud) | Cloud MoE. | 0 | true |
<!-- END MODEL_CATALOGUE -->

<!-- BEGIN TOP_PICKS -->
| tier | model_tag |
|---|---|
| VeryHigh | gemma4:31b |
| High | gemma4:e4b |
| Medium | gemma4:e2b |
| Low | gemma3:1b |
| VeryLow | tinyllama |
<!-- END TOP_PICKS -->
"#;

    #[test]
    fn parse_catalogue_extracts_models() {
        let cat = parse_catalogue(SAMPLE_MD).unwrap();
        assert_eq!(cat.local_models.len(), 5);
        assert_eq!(cat.cloud_models.len(), 1);
        assert_eq!(cat.cloud_models[0].model_tag, "kimi-k2.6:cloud");
        assert!(cat.cloud_models[0].is_cloud);
    }

    #[test]
    fn parse_catalogue_extracts_top_picks() {
        let cat = parse_catalogue(SAMPLE_MD).unwrap();
        assert_eq!(cat.top_picks.len(), 5);
        assert_eq!(cat.top_picks["VeryHigh"], "gemma4:31b");
        assert_eq!(cat.top_picks["Low"], "gemma3:1b");
    }

    #[test]
    fn recommend_from_catalogue_high_ram() {
        let cat = parse_catalogue(SAMPLE_MD).unwrap();
        let recs = recommend_from_catalogue(65_536, &cat);
        assert!(!recs.is_empty());
        assert_eq!(recs[0].model_tag, "gemma4:31b");
        assert!(recs[0].is_top_pick);
        // Cloud model should be at the end.
        assert!(recs.last().unwrap().is_cloud);
    }

    #[test]
    fn recommend_from_catalogue_medium_ram() {
        let cat = parse_catalogue(SAMPLE_MD).unwrap();
        let recs = recommend_from_catalogue(12_000, &cat);
        let top = recs.iter().find(|m| m.is_top_pick).unwrap();
        assert_eq!(top.model_tag, "gemma4:e2b");
    }

    #[test]
    fn recommend_from_catalogue_very_low_ram() {
        let cat = parse_catalogue(SAMPLE_MD).unwrap();
        let recs = recommend_from_catalogue(1_024, &cat);
        assert!(!recs.is_empty());
        // Should include the smallest model as fallback.
        assert!(recs.iter().any(|m| m.model_tag == "tinyllama" || m.model_tag == "gemma3:1b"));
    }

    #[test]
    fn parse_catalogue_returns_none_on_missing_markers() {
        assert!(parse_catalogue("no markers here").is_none());
    }

    #[test]
    fn parse_catalogue_returns_none_on_empty_table() {
        let md = r#"
<!-- BEGIN MODEL_CATALOGUE -->
| model_tag | display_name | description | required_ram_mb | is_cloud |
|---|---|---|---|---|
<!-- END MODEL_CATALOGUE -->
<!-- BEGIN TOP_PICKS -->
| tier | model_tag |
|---|---|
<!-- END TOP_PICKS -->
"#;
        assert!(parse_catalogue(md).is_none());
    }

    #[test]
    fn recommend_from_catalogue_exactly_one_top_pick() {
        let cat = parse_catalogue(SAMPLE_MD).unwrap();
        for ram in [4_096u64, 8_192, 16_384, 32_768, 65_536] {
            let recs = recommend_from_catalogue(ram, &cat);
            let top_count = recs.iter().filter(|m| m.is_top_pick).count();
            assert_eq!(top_count, 1, "Expected 1 top pick for {} MB, got {}", ram, top_count);
        }
    }

    #[test]
    fn extract_between_basic() {
        let text = "before <!-- START --> middle <!-- END --> after";
        assert_eq!(
            extract_between(text, "<!-- START -->", "<!-- END -->"),
            Some("middle")
        );
    }
}
