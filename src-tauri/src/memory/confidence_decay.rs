//! Per-cognitive-kind confidence decay (Chunk 43.3).
//!
//! Different memory kinds age at different rates:
//!
//! | Kind       | Half-life | Rationale                                    |
//! |------------|-----------|----------------------------------------------|
//! | Judgment   | 365 days  | Corrections/rules are very durable           |
//! | Semantic   | 90 days   | General knowledge, stable preferences        |
//! | Procedural | 60 days   | How-to steps change as tools evolve           |
//! | Episodic   | 7 days    | Time-anchored experiences lose value quickly  |
//!
//! The decay factor is multiplied into RRF scores (post-fusion) so that
//! stale memories rank lower without needing a background maintenance pass.

use super::cognitive_kind::CognitiveKind;

/// Per-kind half-life configuration (in days).
#[derive(Debug, Clone, Copy)]
pub struct ConfidenceDecayConfig {
    pub judgment_days: f64,
    pub semantic_days: f64,
    pub procedural_days: f64,
    pub episodic_days: f64,
}

impl Default for ConfidenceDecayConfig {
    fn default() -> Self {
        Self {
            judgment_days: 365.0,
            semantic_days: 90.0,
            procedural_days: 60.0,
            episodic_days: 7.0,
        }
    }
}

impl ConfidenceDecayConfig {
    /// Half-life in days for a given cognitive kind.
    pub fn half_life_days(&self, kind: CognitiveKind) -> f64 {
        match kind {
            CognitiveKind::Judgment | CognitiveKind::Negative => self.judgment_days,
            CognitiveKind::Semantic => self.semantic_days,
            CognitiveKind::Procedural => self.procedural_days,
            CognitiveKind::Episodic => self.episodic_days,
        }
    }
}

/// Compute a multiplicative decay factor in `(0.0, 1.0]` for the given
/// cognitive kind and memory age.
///
/// Formula: `confidence × 0.5^(age_days / half_life_days)`.
///
/// `confidence` is the stored column value (default 1.0).
/// `age_ms` is `now_ms - created_at` (milliseconds).
/// If `kind` is `None`, `CognitiveKind::Semantic` (the default) is used.
pub fn confidence_factor(
    config: &ConfidenceDecayConfig,
    kind: Option<CognitiveKind>,
    confidence: f64,
    age_ms: i64,
) -> f64 {
    if age_ms <= 0 {
        return confidence.clamp(0.0, 1.0);
    }
    let kind = kind.unwrap_or_default();
    let half_life = config.half_life_days(kind);
    // Guard against misconfiguration
    if half_life <= 0.0 {
        return 0.0;
    }
    let age_days = age_ms as f64 / 86_400_000.0;
    let decay = 0.5_f64.powf(age_days / half_life);
    (confidence * decay).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: ConfidenceDecayConfig = ConfidenceDecayConfig {
        judgment_days: 365.0,
        semantic_days: 90.0,
        procedural_days: 60.0,
        episodic_days: 7.0,
    };

    const DAY_MS: i64 = 86_400_000;

    #[test]
    fn brand_new_memory_has_full_confidence() {
        let f = confidence_factor(&CFG, Some(CognitiveKind::Semantic), 1.0, 0);
        assert!((f - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn at_half_life_factor_is_half() {
        // Episodic half-life = 7 days
        let f = confidence_factor(&CFG, Some(CognitiveKind::Episodic), 1.0, 7 * DAY_MS);
        assert!((f - 0.5).abs() < 0.001, "expected ~0.5, got {f}");
    }

    #[test]
    fn judgment_decays_slowly() {
        // After 90 days, Judgment (half-life 365d) should still be ~0.83
        let f = confidence_factor(&CFG, Some(CognitiveKind::Judgment), 1.0, 90 * DAY_MS);
        assert!(f > 0.8, "expected >0.8 for Judgment after 90d, got {f}");
    }

    #[test]
    fn episodic_decays_fast() {
        // After 30 days, Episodic (half-life 7d) should be very low (~0.051)
        let f = confidence_factor(&CFG, Some(CognitiveKind::Episodic), 1.0, 30 * DAY_MS);
        assert!(f < 0.06, "expected <0.06 for Episodic after 30d, got {f}");
    }

    #[test]
    fn none_kind_uses_semantic_default() {
        let with_none = confidence_factor(&CFG, None, 1.0, 90 * DAY_MS);
        let with_sem = confidence_factor(&CFG, Some(CognitiveKind::Semantic), 1.0, 90 * DAY_MS);
        assert!((with_none - with_sem).abs() < f64::EPSILON);
    }

    #[test]
    fn stored_confidence_scales_result() {
        // A memory with confidence=0.5 starts lower
        let full = confidence_factor(&CFG, Some(CognitiveKind::Semantic), 1.0, 45 * DAY_MS);
        let half = confidence_factor(&CFG, Some(CognitiveKind::Semantic), 0.5, 45 * DAY_MS);
        assert!((half - full * 0.5).abs() < 0.001);
    }

    #[test]
    fn negative_age_returns_clamped_confidence() {
        let f = confidence_factor(&CFG, Some(CognitiveKind::Semantic), 1.0, -1000);
        assert!((f - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn result_never_exceeds_one() {
        // Even with confidence > 1.0 (shouldn't happen but guard it)
        let f = confidence_factor(&CFG, Some(CognitiveKind::Semantic), 2.0, 0);
        assert!(f <= 1.0);
    }
}
