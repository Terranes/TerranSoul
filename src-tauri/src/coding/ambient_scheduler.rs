//! Rate-limit-aware adaptive scheduler for the ambient agent (Chunk 43.11).
//!
//! Reads `x-ratelimit-*` headers from LLM provider responses, reserves
//! 20% of the budget for user requests, and applies exponential backoff
//! when a 429 is received.
//!
//! Pure decision module — the caller owns I/O and the clock.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Rate-limit budget
// ---------------------------------------------------------------------------

/// Parsed rate-limit headers from an LLM provider response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Requests remaining in the current window.
    pub remaining: Option<u64>,
    /// Total requests allowed in the window.
    pub limit: Option<u64>,
    /// Seconds until the window resets.
    pub reset_after_secs: Option<u64>,
}

impl RateLimitInfo {
    /// Parse from an HTTP header map. Looks for standard
    /// `x-ratelimit-remaining`, `x-ratelimit-limit`, and
    /// `x-ratelimit-reset` headers.
    pub fn from_headers(headers: &[(String, String)]) -> Self {
        let mut info = Self::default();
        for (key, value) in headers {
            let k = key.to_ascii_lowercase();
            if k == "x-ratelimit-remaining" {
                info.remaining = value.trim().parse().ok();
            } else if k == "x-ratelimit-limit" {
                info.limit = value.trim().parse().ok();
            } else if k == "x-ratelimit-reset" {
                info.reset_after_secs = value.trim().parse().ok();
            }
        }
        info
    }

    /// Fraction of the budget that has been consumed (0.0 – 1.0).
    /// Returns `None` if limit info is unavailable.
    pub fn usage_fraction(&self) -> Option<f64> {
        match (self.remaining, self.limit) {
            (Some(rem), Some(lim)) if lim > 0 => Some(1.0 - (rem as f64 / lim as f64)),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Backoff state
// ---------------------------------------------------------------------------

/// Exponential backoff state for 429 responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackoffState {
    /// Current backoff delay in milliseconds.
    pub current_delay_ms: u64,
    /// Number of consecutive 429 responses.
    pub consecutive_429s: u32,
    /// Base delay before any backoff (milliseconds).
    pub base_delay_ms: u64,
    /// Maximum backoff cap (milliseconds).
    pub max_delay_ms: u64,
}

impl Default for BackoffState {
    fn default() -> Self {
        Self {
            current_delay_ms: 0,
            consecutive_429s: 0,
            base_delay_ms: 1_000,
            max_delay_ms: 300_000, // 5 minutes
        }
    }
}

impl BackoffState {
    /// Record a 429 response, computing the next backoff delay.
    pub fn record_429(&mut self) {
        self.consecutive_429s += 1;
        let exp = 2u64.saturating_pow(self.consecutive_429s.min(20));
        self.current_delay_ms = (self.base_delay_ms.saturating_mul(exp)).min(self.max_delay_ms);
    }

    /// Record a successful response, resetting backoff.
    pub fn record_success(&mut self) {
        self.consecutive_429s = 0;
        self.current_delay_ms = 0;
    }

    /// Whether the scheduler is currently in backoff.
    pub fn is_backing_off(&self) -> bool {
        self.current_delay_ms > 0
    }
}

// ---------------------------------------------------------------------------
// Scheduling decision
// ---------------------------------------------------------------------------

/// The scheduler's decision for when the next cycle should run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleDecision {
    /// Delay until the next cycle in milliseconds.
    pub delay_ms: u64,
    /// Reason for the chosen delay.
    pub reason: String,
}

/// Compute when the next ambient cycle should run.
///
/// # Arguments
/// - `base_delay_ms`: The configured cycle delay.
/// - `user_headroom`: Fraction of budget reserved for user (0.0 – 1.0).
/// - `rate_info`: Most recent rate-limit headers (if available).
/// - `backoff`: Current 429 backoff state.
pub fn schedule_next(
    base_delay_ms: u64,
    user_headroom: f64,
    rate_info: &RateLimitInfo,
    backoff: &BackoffState,
) -> ScheduleDecision {
    // 1. If in backoff, respect it.
    if backoff.is_backing_off() {
        return ScheduleDecision {
            delay_ms: backoff.current_delay_ms,
            reason: format!(
                "429 backoff (attempt {}): {}ms",
                backoff.consecutive_429s, backoff.current_delay_ms
            ),
        };
    }

    // 2. If rate limit info available, throttle to stay within user headroom.
    if let Some(usage) = rate_info.usage_fraction() {
        let max_ambient_usage = 1.0 - user_headroom;
        if usage >= max_ambient_usage {
            // Budget exhausted for ambient — wait for reset.
            let wait = rate_info
                .reset_after_secs
                .map(|s| s * 1000)
                .unwrap_or(base_delay_ms * 5);
            return ScheduleDecision {
                delay_ms: wait,
                reason: format!(
                    "rate limit: {:.0}% used (headroom {:.0}%), waiting {}ms for reset",
                    usage * 100.0,
                    user_headroom * 100.0,
                    wait
                ),
            };
        }
        // Scale delay inversely with remaining budget.
        let scale = 1.0 + (usage / max_ambient_usage).powi(2);
        let adjusted = (base_delay_ms as f64 * scale) as u64;
        return ScheduleDecision {
            delay_ms: adjusted,
            reason: format!(
                "rate-aware: {:.0}% used, delay scaled to {}ms",
                usage * 100.0,
                adjusted
            ),
        };
    }

    // 3. Default: use base delay.
    ScheduleDecision {
        delay_ms: base_delay_ms,
        reason: "no rate-limit info, using base delay".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rate_limit_headers() {
        let headers = vec![
            ("X-RateLimit-Remaining".to_string(), "45".to_string()),
            ("X-RateLimit-Limit".to_string(), "100".to_string()),
            ("X-RateLimit-Reset".to_string(), "30".to_string()),
        ];
        let info = RateLimitInfo::from_headers(&headers);
        assert_eq!(info.remaining, Some(45));
        assert_eq!(info.limit, Some(100));
        assert_eq!(info.reset_after_secs, Some(30));
    }

    #[test]
    fn usage_fraction_calculation() {
        let info = RateLimitInfo {
            remaining: Some(20),
            limit: Some(100),
            reset_after_secs: None,
        };
        let frac = info.usage_fraction().unwrap();
        assert!((frac - 0.8).abs() < 0.001);
    }

    #[test]
    fn usage_fraction_none_when_missing() {
        let info = RateLimitInfo::default();
        assert!(info.usage_fraction().is_none());
    }

    #[test]
    fn backoff_exponential() {
        let mut b = BackoffState::default();
        b.record_429();
        assert_eq!(b.consecutive_429s, 1);
        assert_eq!(b.current_delay_ms, 2_000); // 1000 * 2^1
        b.record_429();
        assert_eq!(b.consecutive_429s, 2);
        assert_eq!(b.current_delay_ms, 4_000); // 1000 * 2^2
    }

    #[test]
    fn backoff_caps_at_max() {
        let mut b = BackoffState {
            max_delay_ms: 10_000,
            ..Default::default()
        };
        for _ in 0..20 {
            b.record_429();
        }
        assert_eq!(b.current_delay_ms, 10_000);
    }

    #[test]
    fn backoff_resets_on_success() {
        let mut b = BackoffState::default();
        b.record_429();
        b.record_429();
        assert!(b.is_backing_off());
        b.record_success();
        assert!(!b.is_backing_off());
        assert_eq!(b.consecutive_429s, 0);
    }

    #[test]
    fn schedule_respects_backoff() {
        let backoff = BackoffState {
            current_delay_ms: 8_000,
            consecutive_429s: 3,
            ..Default::default()
        };
        let info = RateLimitInfo::default();
        let decision = schedule_next(60_000, 0.2, &info, &backoff);
        assert_eq!(decision.delay_ms, 8_000);
        assert!(decision.reason.contains("429 backoff"));
    }

    #[test]
    fn schedule_throttles_on_high_usage() {
        let info = RateLimitInfo {
            remaining: Some(5),
            limit: Some(100),
            reset_after_secs: Some(60),
        };
        let backoff = BackoffState::default();
        // 95% used, headroom 20% → max ambient 80% → over budget
        let decision = schedule_next(60_000, 0.2, &info, &backoff);
        assert_eq!(decision.delay_ms, 60_000); // reset_after_secs * 1000
        assert!(decision.reason.contains("rate limit"));
    }

    #[test]
    fn schedule_scales_delay_with_moderate_usage() {
        let info = RateLimitInfo {
            remaining: Some(60),
            limit: Some(100),
            reset_after_secs: None,
        };
        let backoff = BackoffState::default();
        // 40% used, headroom 20%, max ambient 80% → not over budget
        let decision = schedule_next(60_000, 0.2, &info, &backoff);
        // Scale = 1 + (0.4/0.8)^2 = 1 + 0.25 = 1.25 → 75_000
        assert_eq!(decision.delay_ms, 75_000);
        assert!(decision.reason.contains("rate-aware"));
    }

    #[test]
    fn schedule_uses_base_delay_without_info() {
        let info = RateLimitInfo::default();
        let backoff = BackoffState::default();
        let decision = schedule_next(60_000, 0.2, &info, &backoff);
        assert_eq!(decision.delay_ms, 60_000);
        assert!(decision.reason.contains("base delay"));
    }
}
