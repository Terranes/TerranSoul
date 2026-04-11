use serde::Serialize;

use super::system_info::RamTier;

/// A recommended local AI model for the user's hardware.
#[derive(Debug, Clone, Serialize)]
pub struct ModelRecommendation {
    /// Ollama model tag (e.g. "gemma3:4b").
    pub model_tag: String,
    /// Human-readable display name (e.g. "Gemma 3 4B").
    pub display_name: String,
    /// Brief description of the model's strengths.
    pub description: String,
    /// Approximate minimum RAM needed in MB.
    pub required_ram_mb: u64,
    /// True for the top pick for this hardware tier.
    pub is_top_pick: bool,
}

/// Build a ranked list of model recommendations for the given RAM amount.
///
/// The list is ordered from most-to-least recommended for the hardware.
/// Models that exceed available RAM are excluded.
pub fn recommend(total_ram_mb: u64) -> Vec<ModelRecommendation> {
    let tier = RamTier::from_mb(total_ram_mb);

    let all: Vec<ModelRecommendation> = vec![
        ModelRecommendation {
            model_tag: "gemma4:27b".to_string(),
            display_name: "Gemma 4 27B".to_string(),
            description: "Google's latest flagship open model. State-of-the-art reasoning, coding, and conversation. Requires 32 GB+ RAM.".to_string(),
            required_ram_mb: 32_768,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma4:12b".to_string(),
            display_name: "Gemma 4 12B".to_string(),
            description: "Excellent quality-to-speed ratio. Strong instruction-following and multilingual support. Requires 16 GB+ RAM.".to_string(),
            required_ram_mb: 16_384,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma4:4b".to_string(),
            display_name: "Gemma 4 4B".to_string(),
            description: "Fast and capable. Great for everyday chat and recommendations. Requires 8 GB+ RAM.".to_string(),
            required_ram_mb: 8_192,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma3:27b".to_string(),
            display_name: "Gemma 3 27B".to_string(),
            description: "Previous-gen flagship. Still excellent reasoning and coding. Requires 32 GB+ RAM.".to_string(),
            required_ram_mb: 32_768,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma3:12b".to_string(),
            display_name: "Gemma 3 12B".to_string(),
            description: "Excellent balance of quality and speed. Strong instruction-following. Requires 16 GB+ RAM.".to_string(),
            required_ram_mb: 16_384,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma3:4b".to_string(),
            display_name: "Gemma 3 4B".to_string(),
            description: "Fast and capable. Great for everyday chat and software recommendations. Requires 8 GB+ RAM.".to_string(),
            required_ram_mb: 8_192,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "phi4-mini".to_string(),
            display_name: "Phi-4 Mini".to_string(),
            description: "Microsoft's compact model. Efficient reasoning in a small footprint. Requires 8 GB+ RAM.".to_string(),
            required_ram_mb: 8_192,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma3:1b".to_string(),
            display_name: "Gemma 3 1B".to_string(),
            description: "Ultra-lightweight Gemma. Runs on almost any machine. Basic conversation quality. Requires 4 GB+ RAM.".to_string(),
            required_ram_mb: 4_096,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma2:2b".to_string(),
            display_name: "Gemma 2 2B".to_string(),
            description: "Compact Gemma 2 model. Solid for simple tasks on lower-end hardware. Requires 4 GB+ RAM.".to_string(),
            required_ram_mb: 4_096,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "tinyllama".to_string(),
            display_name: "TinyLlama".to_string(),
            description: "Minimal 1.1B model. Works even on very limited hardware. Requires 2 GB+ RAM.".to_string(),
            required_ram_mb: 2_048,
            is_top_pick: false,
        },
    ];

    // Filter to models that fit in available RAM.
    // Use total_ram_mb directly; model sizes are already conservative estimates.
    let mut candidates: Vec<ModelRecommendation> = all
        .into_iter()
        .filter(|m| m.required_ram_mb <= total_ram_mb)
        .collect();

    // If everything was filtered out (very little RAM), include TinyLlama regardless.
    if candidates.is_empty() {
        candidates.push(ModelRecommendation {
            model_tag: "tinyllama".to_string(),
            display_name: "TinyLlama".to_string(),
            description: "Minimal 1.1B model. The only option for very limited hardware.".to_string(),
            required_ram_mb: 2_048,
            is_top_pick: true,
        });
        return candidates;
    }

    // Mark the top pick for this tier.
    let top_tag = match tier {
        RamTier::VeryHigh => "gemma4:27b",
        RamTier::High => "gemma4:12b",
        RamTier::Medium => "gemma4:4b",
        RamTier::Low => "gemma3:1b",
        RamTier::VeryLow => "tinyllama",
    };

    for m in &mut candidates {
        if m.model_tag == top_tag {
            m.is_top_pick = true;
        }
    }

    // Sort: top pick first, then by required_ram_mb descending.
    candidates.sort_by(|a, b| {
        b.is_top_pick
            .cmp(&a.is_top_pick)
            .then(b.required_ram_mb.cmp(&a.required_ram_mb))
    });

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommend_very_high_ram_top_pick_is_gemma4_27b() {
        let recs = recommend(65_536);
        assert!(!recs.is_empty());
        assert_eq!(recs[0].model_tag, "gemma4:27b");
        assert!(recs[0].is_top_pick);
    }

    #[test]
    fn recommend_high_ram_top_pick_is_gemma4_12b() {
        let recs = recommend(24_000);
        let top = recs.iter().find(|m| m.is_top_pick).unwrap();
        assert_eq!(top.model_tag, "gemma4:12b");
    }

    #[test]
    fn recommend_medium_ram_top_pick_is_gemma4_4b() {
        let recs = recommend(12_000);
        let top = recs.iter().find(|m| m.is_top_pick).unwrap();
        assert_eq!(top.model_tag, "gemma4:4b");
    }

    #[test]
    fn recommend_low_ram_top_pick_is_gemma3_1b() {
        let recs = recommend(6_000);
        let top = recs.iter().find(|m| m.is_top_pick).unwrap();
        assert_eq!(top.model_tag, "gemma3:1b");
    }

    #[test]
    fn recommend_very_low_ram_top_pick_is_tinyllama() {
        let recs = recommend(2_048);
        assert!(!recs.is_empty());
        assert!(recs.iter().any(|m| m.model_tag == "tinyllama"));
    }

    #[test]
    fn recommend_extreme_low_ram_always_returns_tinyllama() {
        let recs = recommend(0);
        assert!(!recs.is_empty());
        assert!(recs.iter().any(|m| m.model_tag == "tinyllama"));
    }

    #[test]
    fn recommend_does_not_include_models_exceeding_ram() {
        // 8 GB — should not include gemma3:12b (16 GB) or gemma3:27b (32 GB)
        let recs = recommend(8_192);
        assert!(!recs.iter().any(|m| m.model_tag == "gemma3:12b"));
        assert!(!recs.iter().any(|m| m.model_tag == "gemma3:27b"));
    }

    #[test]
    fn recommend_all_items_have_non_empty_fields() {
        for ram_mb in [4_096u64, 8_192, 16_384, 32_768] {
            for m in recommend(ram_mb) {
                assert!(!m.model_tag.is_empty());
                assert!(!m.display_name.is_empty());
                assert!(!m.description.is_empty());
            }
        }
    }

    #[test]
    fn recommend_exactly_one_top_pick() {
        for ram_mb in [4_096u64, 8_192, 16_384, 32_768, 65_536] {
            let recs = recommend(ram_mb);
            let top_count = recs.iter().filter(|m| m.is_top_pick).count();
            assert_eq!(top_count, 1, "Expected exactly one top pick for {} MB", ram_mb);
        }
    }
}
