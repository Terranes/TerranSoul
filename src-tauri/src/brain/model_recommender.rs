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
///
/// Model sizes sourced from Ollama library pages (download size + ~2 GB overhead):
/// - gemma4   https://ollama.com/library/gemma4
/// - gemma3   https://ollama.com/library/gemma3
/// - gemma2   https://ollama.com/library/gemma2
/// - phi4-mini https://ollama.com/library/phi4-mini
/// - tinyllama https://ollama.com/library/tinyllama
///
/// Note: GLM-5.1 (744B total, 40B active MoE) is cloud-only on Ollama
/// (`glm-5.1:cloud`). Even the 2-bit GGUF needs ~236 GB RAM.
/// Not viable for consumer hardware.
///   GitHub:    https://github.com/zai-org/GLM-5
///   HF:       https://huggingface.co/zai-org/GLM-5.1
///   Unsloth:  https://unsloth.ai/docs/models/glm-5.1
///   GGUF:     https://huggingface.co/unsloth/GLM-5.1-GGUF
pub fn recommend(total_ram_mb: u64) -> Vec<ModelRecommendation> {
    let tier = RamTier::from_mb(total_ram_mb);

    // Ollama download sizes (Q4 quantised) and required RAM estimates:
    //   gemma4:31b  — 20 GB download  → ~24 GB RAM
    //   gemma4:26b  — 18 GB download  → ~22 GB RAM (MoE, 3.8B active)
    //   gemma3:27b  — 17 GB download  → ~20 GB RAM
    //   gemma4:e4b  — 9.6 GB download → ~12 GB RAM
    //   gemma4:e2b  — 7.2 GB download → ~8 GB RAM
    //   gemma3:4b   — 3.3 GB download → ~6 GB RAM
    //   phi4-mini   — 2.5 GB download → ~4 GB RAM (3.8B params)
    //   gemma3:1b   — 815 MB download → ~2 GB RAM
    //   gemma2:2b   — 1.6 GB download → ~4 GB RAM
    //   tinyllama   — 638 MB download → ~2 GB RAM
    let all: Vec<ModelRecommendation> = vec![
        ModelRecommendation {
            model_tag: "gemma4:31b".to_string(),
            display_name: "Gemma 4 31B".to_string(),
            description: "Google's dense 30.7B flagship. State-of-the-art reasoning, coding, and 256K context. 20 GB download.".to_string(),
            required_ram_mb: 24_576,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma4:26b".to_string(),
            display_name: "Gemma 4 26B MoE".to_string(),
            description: "MoE with 25.2B total / 3.8B active params. Fast inference with 256K context. 18 GB download.".to_string(),
            required_ram_mb: 22_528,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma3:27b".to_string(),
            display_name: "Gemma 3 27B".to_string(),
            description: "Previous-gen flagship. Excellent reasoning, vision, and 128K context. 17 GB download.".to_string(),
            required_ram_mb: 20_480,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma4:e4b".to_string(),
            display_name: "Gemma 4 E4B".to_string(),
            description: "4.5B effective params optimised for edge. 128K context, vision + audio. 9.6 GB download.".to_string(),
            required_ram_mb: 12_288,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma4:e2b".to_string(),
            display_name: "Gemma 4 E2B".to_string(),
            description: "2.3B effective params for edge devices. 128K context, vision + audio. 7.2 GB download.".to_string(),
            required_ram_mb: 8_192,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma3:4b".to_string(),
            display_name: "Gemma 3 4B".to_string(),
            description: "Compact multimodal model. 128K context, great for everyday chat. 3.3 GB download.".to_string(),
            required_ram_mb: 6_144,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "phi4-mini".to_string(),
            display_name: "Phi-4 Mini 3.8B".to_string(),
            description: "Microsoft's compact reasoner. 128K context, strong math/logic. 2.5 GB download.".to_string(),
            required_ram_mb: 4_096,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma3:1b".to_string(),
            display_name: "Gemma 3 1B".to_string(),
            description: "Ultra-lightweight. 32K context, text-only. 815 MB download.".to_string(),
            required_ram_mb: 2_048,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "gemma2:2b".to_string(),
            display_name: "Gemma 2 2B".to_string(),
            description: "Compact Gemma 2. 8K context, solid for simple tasks. 1.6 GB download.".to_string(),
            required_ram_mb: 4_096,
            is_top_pick: false,
        },
        ModelRecommendation {
            model_tag: "tinyllama".to_string(),
            display_name: "TinyLlama 1.1B".to_string(),
            description: "Minimal 1.1B model. 2K context. Works on very limited hardware. 638 MB download.".to_string(),
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
        RamTier::VeryHigh => "gemma4:31b",
        RamTier::High => "gemma4:e4b",
        RamTier::Medium => "gemma4:e2b",
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
    fn recommend_very_high_ram_top_pick_is_gemma4_31b() {
        let recs = recommend(65_536);
        assert!(!recs.is_empty());
        assert_eq!(recs[0].model_tag, "gemma4:31b");
        assert!(recs[0].is_top_pick);
    }

    #[test]
    fn recommend_high_ram_top_pick_is_gemma4_e4b() {
        let recs = recommend(24_000);
        let top = recs.iter().find(|m| m.is_top_pick).unwrap();
        assert_eq!(top.model_tag, "gemma4:e4b");
    }

    #[test]
    fn recommend_medium_ram_top_pick_is_gemma4_e2b() {
        let recs = recommend(12_000);
        let top = recs.iter().find(|m| m.is_top_pick).unwrap();
        assert_eq!(top.model_tag, "gemma4:e2b");
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
        // 8 GB — should not include gemma4:e4b (12 GB) or gemma4:31b (24 GB)
        let recs = recommend(8_192);
        assert!(!recs.iter().any(|m| m.model_tag == "gemma4:e4b"));
        assert!(!recs.iter().any(|m| m.model_tag == "gemma4:31b"));
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
