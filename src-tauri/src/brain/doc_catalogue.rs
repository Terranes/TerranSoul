//! Parse the recommended LLM catalogue from `docs/brain-advanced-design.md`.
//!
//! The markdown contains two machine-readable tables between HTML-comment
//! markers (`<!-- BEGIN MODEL_CATALOGUE -->` / `<!-- END MODEL_CATALOGUE -->`
//! and `<!-- BEGIN TOP_PICKS -->` / `<!-- END TOP_PICKS -->`).  This module
//! extracts those tables and produces the same [`ModelRecommendation`] shape
//! used by the hardcoded fallback in [`super::model_recommender`].

use std::collections::HashMap;
use std::path::Path;

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

/// Estimate download size in MB from the description text or required RAM.
///
/// Tries to parse "X GB download" or "X MB download" from the description
/// first, then falls back to `required_ram_mb * 0.8`.
fn estimate_download_size(description: &str, required_ram_mb: u64) -> u64 {
    let lower = description.to_lowercase();
    // Look for patterns like "20 GB download", "815 MB download", "9.6 GB download"
    for window in lower.split_whitespace().collect::<Vec<_>>().windows(3) {
        if window[2].starts_with("download") {
            if let Ok(val) = window[0].parse::<f64>() {
                if window[1] == "gb" {
                    return (val * 1024.0) as u64;
                } else if window[1] == "mb" {
                    return val as u64;
                }
            }
        }
    }
    // Fallback: ~80% of required RAM is a decent estimate for Q4 quantised models.
    required_ram_mb * 4 / 5
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
            required_ram_mb,
            download_size_mb: estimate_download_size(&description, required_ram_mb),
            description,
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

// ── Online catalogue refresh ─────────────────────────────────────────

/// Build a reusable HTTP client for all online-catalogue sources.
fn build_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("TerranSoul/1.0 (model catalogue refresh)")
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))
}

/// Known Ollama model tags → approximate RAM required (MB, Q4 quant + OS overhead).
const KNOWN_RAM_TABLE: &[(&str, u64)] = &[
    ("tinyllama",          2_048),
    ("gemma3:1b",          2_048),
    ("llama3.2:1b",        2_048),
    ("gemma2:2b",          4_096),
    ("llama3.2:3b",        4_096),
    ("phi4-mini",          4_096),
    ("phi3:mini",          4_096),
    ("gemma3:4b",          6_144),
    ("deepseek-r1:1.5b",   3_072),
    ("deepseek-r1:7b",     8_192),
    ("deepseek-r1:8b",     8_192),
    ("deepseek-r1:14b",   16_384),
    ("deepseek-r1:32b",   32_768),
    ("deepseek-r1:70b",   49_152),
    ("deepseek-r1:671b", 393_216),
    ("mistral:7b",         8_192),
    ("mistral-nemo",      14_336),
    ("llama3.1:8b",       10_240),
    ("llama3.3:70b",      49_152),
    ("qwen2.5:7b",         8_192),
    ("qwen2.5:14b",       14_336),
    ("qwen2.5:32b",       32_768),
    ("qwen2.5:72b",       49_152),
    ("qwen3:8b",          10_240),
    ("qwen3:14b",         14_336),
    ("qwen3:32b",         32_768),
    ("gemma4:e2b",         8_192),
    ("gemma4:e4b",        12_288),
    ("gemma4:26b",        22_528),
    ("gemma4:31b",        24_576),
    ("gemma3:12b",        14_336),
    ("gemma3:27b",        20_480),
    ("command-r",         16_384),
    ("command-r-plus",    49_152),
];

/// Estimate RAM (MB) for an Ollama model by name.
/// Uses the lookup table first, then heuristics from the parameter count.
fn estimate_ram_mb_for_name(name: &str) -> u64 {
    let lower = name.to_lowercase();
    // Exact or prefix match in the known table.
    for (key, ram) in KNOWN_RAM_TABLE {
        if lower == *key || lower.starts_with(&format!("{key}-")) || lower.starts_with(&format!("{key}:")) {
            return *ram;
        }
    }
    // Heuristic: Q4 quant ≈ 0.55 bytes/param + 20 % runtime overhead, round to 512 MB.
    if let Some(b) = extract_param_billions(name) {
        let raw = (b * 1_000.0 * 0.55 * 1.2) as u64;
        return raw.div_ceil(512) * 512;
    }
    // Default: assume a 7 B-class model.
    8_192
}

/// Extract the parameter count in billions from strings like "7b", "3.8b", "llama3.2:3b".
fn extract_param_billions(s: &str) -> Option<f64> {
    let lower = s.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            // Must be followed by 'b' and then a non-alpha character (or end of string).
            if i < chars.len() && chars[i] == 'b' {
                let after = i + 1;
                let boundary = after >= chars.len()
                    || !chars[after].is_ascii_alphabetic()
                    || chars[after] == '-'
                    || chars[after] == '_';
                if boundary {
                    let num_str: String = chars[start..i].iter().collect();
                    if let Ok(v) = num_str.parse::<f64>() {
                        if (0.1..=2_000.0).contains(&v) {
                            return Some(v);
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Convert a model name / Ollama tag to a human-readable display name.
fn format_display_name(tag: &str) -> String {
    let base = tag.split(':').next().unwrap_or(tag);
    base.split(['-', '_'])
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ── Source: Ollama Library ────────────────────────────────────────────

/// Scrape the Ollama model library for all pull-able model tags.
async fn fetch_from_ollama_library(
    client: &reqwest::Client,
) -> Result<Vec<ModelRecommendation>, String> {
    let html = client
        .get("https://ollama.com/library")
        .send()
        .await
        .map_err(|e| format!("Ollama library request: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Ollama library body: {e}"))?;

    let doc = scraper::Html::parse_document(&html);
    // Model cards on ollama.com/library are anchors that link to /library/<name>.
    let sel = scraper::Selector::parse("a[href^='/library/']")
        .map_err(|_| "CSS selector parse error".to_string())?;

    let mut models: Vec<ModelRecommendation> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for link in doc.select(&sel) {
        let href = link.value().attr("href").unwrap_or("");
        let name = href.trim_start_matches("/library/");
        // Skip empty, nested paths, and duplicates.
        if name.is_empty() || name.contains('/') || !seen.insert(name.to_string()) {
            continue;
        }
        let inner_text: String = link
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        // Use inner text as description, stripping the model name prefix if present.
        let desc = inner_text
            .trim_start_matches(name)
            .trim_start_matches(':')
            .trim()
            .to_string();
        let desc = if desc.len() < 10 {
            format!("Available on Ollama library — `ollama pull {name}`")
        } else {
            desc.chars().take(200).collect()
        };

        let ram = estimate_ram_mb_for_name(name);
        models.push(ModelRecommendation {
            model_tag:       name.to_string(),
            display_name:    format_display_name(name),
            download_size_mb: estimate_download_size(&desc, ram),
            description:     desc,
            required_ram_mb: ram,
            is_top_pick:     false,
            is_cloud:        false,
        });
    }

    if models.is_empty() {
        return Err("no models found on Ollama library page".to_string());
    }
    Ok(models)
}

// ── Source: HuggingFace Trending ─────────────────────────────────────

/// Map a HuggingFace model ID to an Ollama-compatible tag, or return `None`.
fn hf_id_to_ollama_tag(hf_id: &str) -> Option<String> {
    const MAPPINGS: &[(&str, &str)] = &[
        ("google/gemma-4",             "gemma4"),
        ("google/gemma-3",             "gemma3"),
        ("google/gemma-2",             "gemma2"),
        ("google/gemma",               "gemma"),
        ("meta-llama/llama-3.3",       "llama3.3"),
        ("meta-llama/llama-3.2",       "llama3.2"),
        ("meta-llama/llama-3.1",       "llama3.1"),
        ("meta-llama/meta-llama-3",    "llama3"),
        ("microsoft/phi-4-mini",       "phi4-mini"),
        ("microsoft/phi-4",            "phi4"),
        ("microsoft/phi-3",            "phi3"),
        ("qwen/qwen3",                 "qwen3"),
        ("qwen/qwen2.5",               "qwen2.5"),
        ("deepseek-ai/deepseek-r1",    "deepseek-r1"),
        ("deepseek-ai/deepseek-v3",    "deepseek-v3"),
        ("mistralai/mistral-7b",       "mistral"),
        ("mistralai/mixtral",          "mixtral"),
        ("mistralai/mistral-nemo",     "mistral-nemo"),
        ("cohere",                     "command-r"),
        ("01-ai/yi",                   "yi"),
    ];
    let lower = hf_id.to_lowercase();
    for (prefix, base) in MAPPINGS {
        if lower.starts_with(prefix) {
            if let Some(b) = extract_param_billions(hf_id) {
                let suffix = if b.fract() == 0.0 {
                    format!("{}b", b as u64)
                } else {
                    format!("{b:.1}b")
                };
                return Some(format!("{base}:{suffix}"));
            }
        }
    }
    None
}

/// Fetch trending text-generation models from the HuggingFace public API.
async fn fetch_from_huggingface(
    client: &reqwest::Client,
) -> Result<Vec<ModelRecommendation>, String> {
    #[derive(serde::Deserialize)]
    struct HfModel {
        id: String,
        #[serde(default)]
        downloads: u64,
        #[serde(default)]
        tags: Vec<String>,
    }

    let url = "https://huggingface.co/api/models\
               ?pipeline_tag=text-generation&sort=trending&limit=50&full=false";

    let models: Vec<HfModel> = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("HuggingFace request: {e}"))?
        .json()
        .await
        .map_err(|e| format!("HuggingFace JSON: {e}"))?;

    let mut result = Vec::new();
    for m in &models {
        let Some(ollama_tag) = hf_id_to_ollama_tag(&m.id) else { continue };
        let is_gguf = m.tags.iter().any(|t| t == "gguf");
        let desc = format!(
            "Trending on HuggingFace{}. {} downloads.",
            if is_gguf { " · GGUF available" } else { "" },
            m.downloads,
        );
        let ram = estimate_ram_mb_for_name(&ollama_tag);
        result.push(ModelRecommendation {
            model_tag:       ollama_tag.clone(),
            display_name:    format_display_name(&ollama_tag),
            description:     desc.clone(),
            required_ram_mb: ram,
            download_size_mb: estimate_download_size(&desc, ram),
            is_top_pick:     false,
            is_cloud:        false,
        });
    }
    Ok(result)
}

// ── Source: LM Studio Model Catalog ──────────────────────────────────

/// Fetch curated models from the LM Studio public model catalog.
async fn fetch_from_lm_studio(
    client: &reqwest::Client,
) -> Result<Vec<ModelRecommendation>, String> {
    #[derive(serde::Deserialize)]
    struct LmsModel {
        #[serde(alias = "modelId", alias = "id", default)]
        id: String,
        #[serde(alias = "name", default)]
        name: String,
        #[serde(default)]
        description: String,
    }

    // LM Studio may expose a JSON catalog; this is best-effort.
    let url = "https://lmstudio.ai/api/catalog/featured";
    let resp = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("LM Studio request: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("LM Studio HTTP {}", resp.status()));
    }

    let models: Vec<LmsModel> = resp
        .json()
        .await
        .map_err(|e| format!("LM Studio JSON: {e}"))?;

    let mut result = Vec::new();
    for m in &models {
        let key = if m.id.is_empty() { &m.name } else { &m.id };
        let Some(ollama_tag) = hf_id_to_ollama_tag(key) else { continue };
        let desc = if m.description.is_empty() {
            "Curated on LM Studio model catalog.".to_string()
        } else {
            m.description.chars().take(200).collect()
        };
        let ram = estimate_ram_mb_for_name(&ollama_tag);
        result.push(ModelRecommendation {
            model_tag:       ollama_tag.clone(),
            display_name:    format_display_name(&ollama_tag),
            description:     desc.clone(),
            required_ram_mb: ram,
            download_size_mb: estimate_download_size(&desc, ram),
            is_top_pick:     false,
            is_cloud:        false,
        });
    }
    Ok(result)
}

// ── Merge & cache ─────────────────────────────────────────────────────

/// Assign the best top-pick model for each RAM tier from a sorted list.
fn build_top_picks(local_models: &[ModelRecommendation]) -> HashMap<String, String> {
    // For each tier, the budget is the FLOOR of that tier's RAM range
    // (i.e. the minimum RAM any user in that tier can have).  This ensures
    // the recommended model can actually run on the weakest machine in the
    // tier — matching the logic in `RamTier::from_mb` and `recommend()`.
    //
    // VeryLow is special: floor is 0 but we use 4,095 (the tier's upper
    // bound) so we always produce a pick for the smallest machines.
    const TIERS: &[(&str, u64)] = &[
        ("VeryLow",  4_095),   // tier spans 0–4095; use ceiling so *something* fits
        ("Low",      4_096),   // tier floor — gemma3:1b (2048) fits
        ("Medium",   8_192),   // tier floor — gemma4:e2b (8192) fits
        ("High",    16_384),   // tier floor — gemma4:e4b (12288) fits
        ("VeryHigh", 32_768),  // tier floor — gemma4:31b (24576) fits
    ];
    let mut picks = HashMap::new();
    for (tier, budget) in TIERS {
        if let Some(best) = local_models
            .iter()
            .filter(|m| m.required_ram_mb <= *budget)
            .max_by_key(|m| m.required_ram_mb)
        {
            picks.insert(tier.to_string(), best.model_tag.clone());
        }
    }
    picks
}

/// Serialise a `ParsedCatalogue` to marker-delimited markdown for caching.
fn catalogue_to_markdown(cat: &ParsedCatalogue) -> String {
    let mut md = String::from(
        "<!-- Auto-generated by TerranSoul online catalogue refresh -->\n\n\
         <!-- BEGIN MODEL_CATALOGUE -->\n\
         | model_tag | display_name | description | required_ram_mb | is_cloud |\n\
         |---|---|---|---|---|\n",
    );
    for m in &cat.local_models {
        md.push_str(&format!(
            "| {} | {} | {} | {} | false |\n",
            m.model_tag, m.display_name, m.description, m.required_ram_mb,
        ));
    }
    for m in &cat.cloud_models {
        md.push_str(&format!(
            "| {} | {} | {} | {} | true |\n",
            m.model_tag, m.display_name, m.description, m.required_ram_mb,
        ));
    }
    md.push_str("<!-- END MODEL_CATALOGUE -->\n\n");
    md.push_str("<!-- BEGIN TOP_PICKS -->\n| tier | model_tag |\n|---|---|\n");
    for (tier, tag) in &cat.top_picks {
        md.push_str(&format!("| {tier} | {tag} |\n"));
    }
    md.push_str("<!-- END TOP_PICKS -->\n");
    md
}

/// Fetch the latest model catalogue from multiple well-known public sources
/// **concurrently**, merge the results, and cache them locally.
///
/// Sources queried:
/// 1. **Ollama Model Library** (`ollama.com/library`) — definitive list of
///    all models available via `ollama pull`.
/// 2. **HuggingFace Trending API** — broad ecosystem quality signal;
///    only models that map to a known Ollama tag are included.
/// 3. **LM Studio Model Catalog** — curated, quantised models for
///    consumer hardware; mapped to Ollama tags where possible.
///
/// Individual source failures are silently ignored. Returns an error only
/// when all sources fail to return any models.
/// The merged catalogue is cached to `<cache_dir>/model-catalogue.md`.
pub async fn fetch_online_catalogue(
    cache_dir: &Path,
) -> Result<ParsedCatalogue, String> {
    let client = build_http_client()?;

    // Fetch all three sources concurrently; individual failures are tolerated.
    let (ollama_res, hf_res, lms_res) = tokio::join!(
        fetch_from_ollama_library(&client),
        fetch_from_huggingface(&client),
        fetch_from_lm_studio(&client),
    );

    // Merge results, deduplicating by model_tag.
    // Ollama source takes precedence (inserted first), others fill in gaps.
    let mut merged: Vec<ModelRecommendation> = Vec::new();
    let mut seen: std::collections::HashSet<String> = Default::default();

    for batch in [ollama_res.ok(), hf_res.ok(), lms_res.ok()]
        .into_iter()
        .flatten()
    {
        for model in batch {
            if seen.insert(model.model_tag.clone()) {
                merged.push(model);
            }
        }
    }

    if merged.is_empty() {
        return Err("all online sources (Ollama, HuggingFace, LM Studio) returned no models".to_string());
    }

    // Always include our curated models from the hardcoded catalogue.
    // The hardcoded list has precise RAM estimates and download sizes that
    // online scrapers cannot reliably provide.  Online models *supplement*
    // but never *replace* the curated entries.
    let curated = super::model_recommender::recommend(u64::MAX);
    for m in curated {
        if seen.insert(m.model_tag.clone()) {
            merged.push(m);
        }
    }

    // Sort by required RAM descending so most-capable models come first.
    merged.sort_by_key(|m| std::cmp::Reverse(m.required_ram_mb));

    let (local, cloud): (Vec<_>, Vec<_>) = merged.into_iter().partition(|m| !m.is_cloud);
    let top_picks = build_top_picks(&local);

    let catalogue = ParsedCatalogue { local_models: local, cloud_models: cloud, top_picks };

    // Cache as marker-delimited markdown so `load_cached_catalogue` can read it.
    std::fs::create_dir_all(cache_dir).ok();
    std::fs::write(cache_dir.join("model-catalogue.md"), catalogue_to_markdown(&catalogue)).ok();

    Ok(catalogue)
}

/// Load a previously-cached catalogue from disk.
pub fn load_cached_catalogue(cache_dir: &Path) -> Option<ParsedCatalogue> {
    let markdown = std::fs::read_to_string(cache_dir.join("model-catalogue.md")).ok()?;
    parse_catalogue(&markdown)
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
    fn recommend_high_tier_picks_e4b_not_31b() {
        let cat = parse_catalogue(SAMPLE_MD).unwrap();
        // 16 GB user is "High" tier — should get gemma4:e4b (12 GB),
        // NOT gemma4:31b (24 GB) which won't fit.
        let recs = recommend_from_catalogue(16_384, &cat);
        let top = recs.iter().find(|m| m.is_top_pick).unwrap();
        assert_eq!(top.model_tag, "gemma4:e4b");
        // gemma4:31b should NOT be in the list at all (24,576 > 16,384).
        assert!(!recs.iter().any(|m| m.model_tag == "gemma4:31b"));
    }

    #[test]
    fn build_top_picks_matches_hardcoded_recommend() {
        // Online catalogue top-picks must agree with the hardcoded recommend().
        let cat = parse_catalogue(SAMPLE_MD).unwrap();
        assert_eq!(cat.top_picks["VeryHigh"], "gemma4:31b");
        assert_eq!(cat.top_picks["High"], "gemma4:e4b");
        assert_eq!(cat.top_picks["Medium"], "gemma4:e2b");
        assert_eq!(cat.top_picks["Low"], "gemma3:1b");
        assert_eq!(cat.top_picks["VeryLow"], "tinyllama");
    }

    #[test]
    fn build_top_picks_online_picks_correct_models() {
        // Simulate the online catalogue path where build_top_picks is called
        // (no explicit TOP_PICKS markers — the function must compute picks).
        let models = vec![
            ModelRecommendation {
                model_tag: "gemma4:31b".to_string(), display_name: String::new(),
                description: String::new(), required_ram_mb: 24_576,
                download_size_mb: 0, is_top_pick: false, is_cloud: false,
            },
            ModelRecommendation {
                model_tag: "gemma4:e4b".to_string(), display_name: String::new(),
                description: String::new(), required_ram_mb: 12_288,
                download_size_mb: 0, is_top_pick: false, is_cloud: false,
            },
            ModelRecommendation {
                model_tag: "gemma4:e2b".to_string(), display_name: String::new(),
                description: String::new(), required_ram_mb: 8_192,
                download_size_mb: 0, is_top_pick: false, is_cloud: false,
            },
            ModelRecommendation {
                model_tag: "gemma3:1b".to_string(), display_name: String::new(),
                description: String::new(), required_ram_mb: 2_048,
                download_size_mb: 0, is_top_pick: false, is_cloud: false,
            },
            ModelRecommendation {
                model_tag: "tinyllama".to_string(), display_name: String::new(),
                description: String::new(), required_ram_mb: 2_048,
                download_size_mb: 0, is_top_pick: false, is_cloud: false,
            },
        ];
        let picks = build_top_picks(&models);
        // High tier user (16–32 GB) must get gemma4:e4b, NOT gemma4:31b
        assert_eq!(picks["High"], "gemma4:e4b");
        assert_eq!(picks["VeryHigh"], "gemma4:31b");
        assert_eq!(picks["Medium"], "gemma4:e2b");
    }

    #[test]
    fn extract_between_basic() {
        let text = "before <!-- START --> middle <!-- END --> after";
        assert_eq!(
            extract_between(text, "<!-- START -->", "<!-- END -->"),
            Some("middle")
        );
    }

    #[test]
    fn dev_path_reads_actual_design_doc() {
        // This test verifies the CARGO_MANIFEST_DIR dev fallback path
        // actually finds and parses the real brain-advanced-design.md.
        let dev_doc = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("docs")
            .join("brain-advanced-design.md");
        let markdown = std::fs::read_to_string(&dev_doc)
            .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", dev_doc, e));
        let catalogue = parse_catalogue(&markdown)
            .expect("Failed to parse catalogue from brain-advanced-design.md");
        // Must have local models
        assert!(!catalogue.local_models.is_empty(), "No local models parsed");
        // Must have top picks for all tiers
        assert!(catalogue.top_picks.contains_key("VeryHigh"));
        assert!(catalogue.top_picks.contains_key("High"));
        assert!(catalogue.top_picks.contains_key("Medium"));
        assert!(catalogue.top_picks.contains_key("Low"));
        assert!(catalogue.top_picks.contains_key("VeryLow"));
        // Verify recommend_from_catalogue returns results for a typical system
        let recs = recommend_from_catalogue(16_384, &catalogue);
        assert!(!recs.is_empty(), "No recommendations for 16 GB RAM");
        assert!(recs[0].is_top_pick, "First result should be top pick");
    }
}
