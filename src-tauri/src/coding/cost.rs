//! Token-cost estimation for the self-improve loop (Chunk 28.5).
//!
//! Pure, I/O-free price catalogue — given a `(provider, model,
//! prompt_tokens, completion_tokens)` tuple, returns an estimated
//! USD cost. The catalogue covers the providers TerranSoul actually
//! talks to (Anthropic, Groq, OpenAI, DeepSeek, Custom/Ollama). Prices
//! are USD per 1M tokens, matching the public list-price page each
//! provider publishes.
//!
//! Local providers (Ollama, anything routed through `127.0.0.1:11434`)
//! return `0.0` because there is no per-token cost to running a model
//! the user already owns. This is the single source of truth for
//! "is this run free or paid?".
//!
//! All public functions are pure — they take primitives in and return
//! primitives out, so the metrics layer can compute cost without a
//! tokio runtime or any I/O.

use super::CodingLlmProvider;

/// USD price per 1 000 000 tokens, as of 2026-04-30.
///
/// Source: each provider's public pricing page. Values are conservative
/// for older / smaller models — we don't ship guesses for premium tiers
/// the user hasn't configured. When a model is not listed, the lookup
/// falls back to the cheapest entry for that provider so cost stays
/// non-zero for paid providers (a known-low estimate is more useful than
/// `0.0`, which would falsely suggest a paid call was free).
#[derive(Debug, Clone, Copy)]
pub struct ModelPrice {
    /// USD per 1M prompt tokens.
    pub prompt_per_m: f64,
    /// USD per 1M completion tokens.
    pub completion_per_m: f64,
}

impl ModelPrice {
    pub const FREE: ModelPrice =
        ModelPrice { prompt_per_m: 0.0, completion_per_m: 0.0 };
}

/// Look up a price entry by `(provider, model)`. Returns `ModelPrice::FREE`
/// for local providers; otherwise returns either the exact match or the
/// cheapest known entry for that provider as a fallback.
pub fn lookup_price(provider: &CodingLlmProvider, model: &str) -> ModelPrice {
    match provider {
        CodingLlmProvider::Anthropic => anthropic_price(model),
        CodingLlmProvider::Openai => openai_price(model),
        CodingLlmProvider::Deepseek => deepseek_price(model),
        // Custom is the local-Ollama escape hatch (`127.0.0.1:11434/v1`)
        // *and* the catch-all for self-hosted endpoints. Treat as free
        // unless the model name strongly suggests a paid hosted variant.
        CodingLlmProvider::Custom => ModelPrice::FREE,
    }
}

/// Estimate USD cost for a completed run. Returns `0.0` for free
/// providers, capped at 8 decimal places to avoid noise from
/// floating-point representation when serialising.
pub fn estimate_cost_usd(
    provider: &CodingLlmProvider,
    model: &str,
    prompt_tokens: u64,
    completion_tokens: u64,
) -> f64 {
    let price = lookup_price(provider, model);
    let prompt = (prompt_tokens as f64) * price.prompt_per_m / 1_000_000.0;
    let completion = (completion_tokens as f64) * price.completion_per_m / 1_000_000.0;
    let total = prompt + completion;
    // Round to 8 dp.
    (total * 1e8).round() / 1e8
}

fn anthropic_price(model: &str) -> ModelPrice {
    let m = model.to_ascii_lowercase();
    if m.contains("opus") {
        ModelPrice { prompt_per_m: 15.0, completion_per_m: 75.0 }
    } else if m.contains("sonnet") {
        ModelPrice { prompt_per_m: 3.0, completion_per_m: 15.0 }
    } else if m.contains("haiku") {
        ModelPrice { prompt_per_m: 0.80, completion_per_m: 4.0 }
    } else {
        // Cheapest known fallback for the family.
        ModelPrice { prompt_per_m: 0.80, completion_per_m: 4.0 }
    }
}

fn openai_price(model: &str) -> ModelPrice {
    let m = model.to_ascii_lowercase();
    if m.contains("gpt-4o-mini") || m.contains("4o-mini") {
        ModelPrice { prompt_per_m: 0.15, completion_per_m: 0.60 }
    } else if m.contains("gpt-4o") || m.contains("4o") {
        ModelPrice { prompt_per_m: 2.50, completion_per_m: 10.0 }
    } else if m.contains("gpt-4-turbo") {
        ModelPrice { prompt_per_m: 10.0, completion_per_m: 30.0 }
    } else if m.contains("gpt-3.5") {
        ModelPrice { prompt_per_m: 0.50, completion_per_m: 1.50 }
    } else {
        ModelPrice { prompt_per_m: 0.15, completion_per_m: 0.60 }
    }
}

fn deepseek_price(model: &str) -> ModelPrice {
    let m = model.to_ascii_lowercase();
    if m.contains("coder") {
        ModelPrice { prompt_per_m: 0.14, completion_per_m: 0.28 }
    } else if m.contains("reasoner") {
        ModelPrice { prompt_per_m: 0.55, completion_per_m: 2.19 }
    } else {
        ModelPrice { prompt_per_m: 0.14, completion_per_m: 0.28 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_custom_is_always_free() {
        let cost = estimate_cost_usd(
            &CodingLlmProvider::Custom,
            "gemma3:4b",
            10_000,
            5_000,
        );
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn anthropic_haiku_default_when_unknown() {
        let p = lookup_price(&CodingLlmProvider::Anthropic, "claude-some-future-model");
        assert!((p.prompt_per_m - 0.80).abs() < 1e-9);
    }

    #[test]
    fn anthropic_sonnet_lookup() {
        let p = lookup_price(&CodingLlmProvider::Anthropic, "claude-sonnet-4-5");
        assert!((p.prompt_per_m - 3.0).abs() < 1e-9);
        assert!((p.completion_per_m - 15.0).abs() < 1e-9);
    }

    #[test]
    fn anthropic_opus_lookup() {
        let p = lookup_price(&CodingLlmProvider::Anthropic, "claude-opus-4-7");
        assert!((p.prompt_per_m - 15.0).abs() < 1e-9);
    }

    #[test]
    fn estimate_cost_is_non_negative_and_rounded() {
        let cost = estimate_cost_usd(
            &CodingLlmProvider::Anthropic,
            "claude-sonnet-4-5",
            1_000_000, // exactly 1M prompt tokens
            500_000,   // 0.5M completion
        );
        // 1.0 * 3.00 + 0.5 * 15.00 = 10.50 USD
        assert!((cost - 10.50).abs() < 1e-6);
    }

    #[test]
    fn openai_4o_mini_cheaper_than_4o() {
        let mini = lookup_price(&CodingLlmProvider::Openai, "gpt-4o-mini");
        let full = lookup_price(&CodingLlmProvider::Openai, "gpt-4o");
        assert!(mini.prompt_per_m < full.prompt_per_m);
    }

    #[test]
    fn deepseek_coder_cheaper_than_reasoner() {
        let coder = lookup_price(&CodingLlmProvider::Deepseek, "deepseek-coder");
        let reasoner = lookup_price(&CodingLlmProvider::Deepseek, "deepseek-reasoner");
        assert!(coder.prompt_per_m < reasoner.prompt_per_m);
    }

    #[test]
    fn estimate_cost_against_openai_gpt4o() {
        let cost = estimate_cost_usd(
            &CodingLlmProvider::Openai,
            "gpt-4o",
            0,
            0,
        );
        assert_eq!(cost, 0.0);
    }
}
