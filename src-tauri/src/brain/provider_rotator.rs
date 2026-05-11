use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::brain::circuit_breaker::{CircuitBreaker, CircuitState};
use crate::brain::free_api::{free_provider_catalogue, FreeProvider};

// ---------------------------------------------------------------------------
// Failover types (Chunk 35.2)
// ---------------------------------------------------------------------------

/// Why a particular provider was skipped during selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailoverReason {
    /// Provider returned HTTP 429 or remaining-requests reached 0.
    RateLimit,
    /// Provider failed health check (network error, timeout).
    Unhealthy,
    /// Request's estimated token count exceeds the provider's max context.
    ContextOverflow {
        estimated_tokens: u32,
        provider_max: u32,
    },
    /// Token budget configured in the provider policy would be exceeded.
    TokenCapExceeded { cap: u32, estimated: u32 },
    /// Local-first/privacy constraint prevents failover to a cloud provider.
    PrivacyConstraint,
    /// All free-tier providers in the catalogue are exhausted.
    FreeTierExhausted,
    /// Provider circuit breaker is OPEN (repeated request failures).
    CircuitBreakerOpen,
}

impl FailoverReason {
    /// Human-readable short label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::RateLimit => "rate-limited",
            Self::Unhealthy => "unhealthy",
            Self::ContextOverflow { .. } => "context-overflow",
            Self::TokenCapExceeded { .. } => "token-cap-exceeded",
            Self::PrivacyConstraint => "privacy-constraint",
            Self::FreeTierExhausted => "free-tier-exhausted",
            Self::CircuitBreakerOpen => "circuit-breaker-open",
        }
    }
}

/// Records a single failover decision during provider selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverEvent {
    /// Provider that was considered.
    pub provider_id: String,
    /// Why it was skipped.
    pub reason: FailoverReason,
    /// Unix epoch milliseconds when this event was recorded.
    pub timestamp_ms: u64,
}

/// Constraints passed to [`ProviderRotator::select_provider`] to enable
/// context-aware failover decisions.
#[derive(Debug, Clone, Default)]
pub struct SelectionConstraints {
    /// Estimated total tokens (input + output) for the current request.
    /// When set, providers whose max context is smaller are skipped.
    pub estimated_tokens: Option<u32>,
    /// Max token budget from the provider policy override. Providers are
    /// skipped if `estimated_tokens > token_cap`.
    pub token_cap: Option<u32>,
    /// When `true`, only local providers are acceptable (Ollama, LM Studio).
    /// Cloud providers are skipped with `FailoverReason::PrivacyConstraint`.
    pub local_only: bool,
}

/// Summary of the rotator's recent failover history for UI consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverSummary {
    /// Number of providers currently healthy and available.
    pub healthy_count: usize,
    /// Number of providers currently rate-limited.
    pub rate_limited_count: usize,
    /// Number of providers currently unhealthy.
    pub unhealthy_count: usize,
    /// Whether all providers are exhausted.
    pub all_exhausted: bool,
    /// Most recent failover events (newest first), capped at
    /// [`MAX_FAILOVER_HISTORY`].
    pub recent_events: Vec<FailoverEvent>,
    /// The currently selected provider (if any).
    pub selected_provider_id: Option<String>,
}

/// Policy controlling automatic retry/failover behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverPolicy {
    /// Maximum number of providers to try before giving up.
    /// Includes the initial attempt. E.g. `3` means initial + 2 retries.
    pub max_attempts: u8,
    /// Whether to respect privacy/local-only constraints during failover.
    /// When `true`, cloud providers are never used as fallbacks.
    pub respect_privacy: bool,
    /// Minimum cooldown (seconds) after a provider is marked rate-limited
    /// before it can be retried. Overrides header-based reset if shorter.
    pub min_cooldown_secs: u64,
}

impl Default for FailoverPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            respect_privacy: true,
            min_cooldown_secs: 60,
        }
    }
}

/// Outcome of an automatic failover attempt for UI/MCP status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverDecision {
    /// Provider that was originally selected.
    pub original_provider_id: String,
    /// Provider that ultimately handled the request (may be same as original).
    pub final_provider_id: String,
    /// Providers that were tried and failed, in order.
    pub attempts: Vec<FailoverAttempt>,
    /// Whether the request ultimately succeeded after failover.
    pub succeeded: bool,
}

/// A single failed attempt during automatic failover.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverAttempt {
    pub provider_id: String,
    pub reason: FailoverReason,
}

/// Max failover events to keep in the ring buffer.
pub const MAX_FAILOVER_HISTORY: usize = 50;

/// Tracks rate-limit and health state for a single provider.
#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub provider: FreeProvider,
    /// Total requests sent through this provider.
    pub requests_sent: u64,
    /// Remaining requests as reported by `x-ratelimit-remaining-requests`.
    pub remaining_requests: Option<u64>,
    /// Remaining tokens as reported by `x-ratelimit-remaining-tokens`.
    pub remaining_tokens: Option<u64>,
    /// When the rate limit resets (epoch seconds from `x-ratelimit-reset`).
    pub rate_limit_reset: Option<u64>,
    /// Whether the provider is marked as rate-limited (e.g. HTTP 429).
    pub is_rate_limited: bool,
    /// Whether the last health check succeeded.
    pub is_healthy: bool,
    /// Last measured round-trip latency.
    pub latency: Option<Duration>,
    /// When the last health check was performed.
    pub last_health_check: Option<Instant>,
    /// Per-provider circuit breaker (trips after repeated request failures).
    pub circuit_breaker: CircuitBreaker,
}

impl ProviderStatus {
    fn new(provider: FreeProvider) -> Self {
        Self {
            provider,
            requests_sent: 0,
            remaining_requests: None,
            remaining_tokens: None,
            rate_limit_reset: None,
            is_rate_limited: false,
            is_healthy: true,
            latency: None,
            last_health_check: None,
            circuit_breaker: CircuitBreaker::with_defaults(),
        }
    }
}

/// Manages provider selection, health checking, and rate-limit rotation.
#[derive(Debug)]
pub struct ProviderRotator {
    /// Per-provider status keyed by provider id.
    pub providers: HashMap<String, ProviderStatus>,
    /// Provider ids sorted by latency (fastest first). Updated after health checks.
    sorted_ids: Vec<String>,
    /// Recent failover events (newest at front), capped at [`MAX_FAILOVER_HISTORY`].
    failover_history: VecDeque<FailoverEvent>,
}

impl ProviderRotator {
    /// Create a new rotator pre-loaded with the free provider catalogue.
    pub fn new() -> Self {
        let catalogue = free_provider_catalogue();
        let sorted_ids: Vec<String> = catalogue.iter().map(|p| p.id.clone()).collect();
        let providers = catalogue
            .into_iter()
            .map(|p| (p.id.clone(), ProviderStatus::new(p)))
            .collect();
        Self {
            providers,
            sorted_ids,
            failover_history: VecDeque::new(),
        }
    }

    /// Run a health check against all providers in parallel.
    ///
    /// Sends a minimal HEAD request to each provider's base URL, records
    /// latency, and re-sorts providers fastest-first.
    pub async fn health_check_all(&mut self) {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };

        let ids: Vec<String> = self.providers.keys().cloned().collect();
        let mut handles = Vec::new();

        for id in &ids {
            if let Some(status) = self.providers.get(id) {
                let url = status.provider.base_url.clone();
                let client = client.clone();
                let id = id.clone();
                handles.push(tokio::spawn(async move {
                    let start = Instant::now();
                    let result = client.head(&url).send().await;
                    let elapsed = start.elapsed();
                    (id, result.is_ok(), elapsed)
                }));
            }
        }

        for handle in handles {
            if let Ok((id, ok, elapsed)) = handle.await {
                if let Some(status) = self.providers.get_mut(&id) {
                    status.is_healthy = ok;
                    status.latency = Some(elapsed);
                    status.last_health_check = Some(Instant::now());
                }
            }
        }

        self.sort_by_latency();
    }

    /// Parse standard rate-limit headers from a response and update provider state.
    pub fn record_response_headers(
        &mut self,
        provider_id: &str,
        headers: &reqwest::header::HeaderMap,
    ) {
        let Some(status) = self.providers.get_mut(provider_id) else {
            return;
        };

        status.requests_sent += 1;

        if let Some(val) = headers
            .get("x-ratelimit-remaining-requests")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            status.remaining_requests = Some(val);
            if val == 0 {
                status.is_rate_limited = true;
            }
        }

        if let Some(val) = headers
            .get("x-ratelimit-remaining-tokens")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            status.remaining_tokens = Some(val);
        }

        if let Some(val) = headers
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            status.rate_limit_reset = Some(val);
        }
    }

    /// Mark a provider as rate-limited (e.g. after receiving HTTP 429).
    pub fn record_rate_limit(&mut self, provider_id: &str) {
        if let Some(status) = self.providers.get_mut(provider_id) {
            status.is_rate_limited = true;
        }
    }

    /// Return the next healthy, non-rate-limited provider (fastest first).
    ///
    /// Automatically clears expired rate limits before selecting.
    pub fn next_healthy_provider(&mut self) -> Option<&FreeProvider> {
        self.clear_expired_limits();

        for id in &self.sorted_ids {
            if let Some(status) = self.providers.get(id) {
                if !status.is_healthy || status.is_rate_limited {
                    continue;
                }
                return Some(&status.provider);
            }
        }
        None
    }

    /// Returns `true` when every provider is either rate-limited or unhealthy.
    pub fn all_exhausted(&self) -> bool {
        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.providers.values().all(|s| {
            if !s.is_healthy {
                return true;
            }
            if s.is_rate_limited {
                // If the reset time has passed, it's no longer exhausted
                if let Some(reset) = s.rate_limit_reset {
                    return now_epoch < reset;
                }
                return true;
            }
            false
        })
    }

    /// Snapshot current provider health and recent failover events for UI/MCP status.
    pub fn failover_summary(&self) -> FailoverSummary {
        let mut healthy_count = 0usize;
        let mut rate_limited_count = 0usize;
        let mut unhealthy_count = 0usize;

        for status in self.providers.values() {
            if status.is_healthy && !status.is_rate_limited {
                healthy_count += 1;
            }
            if status.is_rate_limited {
                rate_limited_count += 1;
            }
            if !status.is_healthy {
                unhealthy_count += 1;
            }
        }

        let selected_provider_id = self.sorted_ids.iter().find_map(|provider_id| {
            let status = self.providers.get(provider_id)?;
            if status.is_healthy && !status.is_rate_limited {
                Some(provider_id.clone())
            } else {
                None
            }
        });

        FailoverSummary {
            healthy_count,
            rate_limited_count,
            unhealthy_count,
            all_exhausted: self.all_exhausted(),
            recent_events: self.failover_history.iter().cloned().collect(),
            selected_provider_id,
        }
    }

    /// Clear rate-limit flags for providers whose reset time has passed.
    pub fn clear_expired_limits(&mut self) {
        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for status in self.providers.values_mut() {
            if status.is_rate_limited {
                if let Some(reset) = status.rate_limit_reset {
                    if now_epoch >= reset {
                        status.is_rate_limited = false;
                        status.rate_limit_reset = None;
                        status.remaining_requests = None;
                    }
                }
            }
        }
    }

    /// Select the best provider given constraints. On success returns the
    /// provider reference; on failure returns the dominant `FailoverReason`
    /// explaining why no provider qualified.
    ///
    /// This is the constraint-aware replacement for `next_healthy_provider()`.
    /// It evaluates each provider against health, rate-limit state, context-
    /// window fit, token-cap policy, and privacy preference — recording a
    /// `FailoverEvent` for each skipped candidate.
    pub fn select_provider(
        &mut self,
        constraints: &SelectionConstraints,
    ) -> Result<&FreeProvider, FailoverReason> {
        self.clear_expired_limits();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Iterate sorted (fastest-first) candidates.
        // We need to collect ids first since we mutably borrow `self` for
        // recording events.
        let ids: Vec<String> = self.sorted_ids.clone();

        for id in &ids {
            let Some(status) = self.providers.get(id) else {
                continue;
            };

            // --- Health gate ---
            if !status.is_healthy {
                self.record_failover(id.clone(), FailoverReason::Unhealthy, now_ms);
                continue;
            }

            // --- Circuit breaker gate ---
            // Snapshot values we need from the immutable borrow before
            // potentially calling get_mut.
            let breaker_state = status.circuit_breaker.state();
            let is_rate_limited = status.is_rate_limited;

            if breaker_state == CircuitState::Open {
                // Attempt is_allowed() which may transition to HalfOpen.
                let allowed = self
                    .providers
                    .get_mut(id)
                    .map(|s| s.circuit_breaker.is_allowed())
                    .unwrap_or(false);
                if !allowed {
                    self.record_failover(id.clone(), FailoverReason::CircuitBreakerOpen, now_ms);
                    continue;
                }
            }

            // --- Rate-limit gate ---
            if is_rate_limited {
                self.record_failover(id.clone(), FailoverReason::RateLimit, now_ms);
                continue;
            }

            // --- Privacy gate (cloud providers skipped when local_only) ---
            if constraints.local_only {
                // All providers in the free catalogue are cloud-based.
                self.record_failover(id.clone(), FailoverReason::PrivacyConstraint, now_ms);
                continue;
            }

            // --- Token-cap gate (policy budget) ---
            if let (Some(estimated), Some(cap)) =
                (constraints.estimated_tokens, constraints.token_cap)
            {
                if estimated > cap {
                    self.record_failover(
                        id.clone(),
                        FailoverReason::TokenCapExceeded { cap, estimated },
                        now_ms,
                    );
                    continue;
                }
            }

            // --- Context-window gate (provider max context) ---
            if let Some(estimated) = constraints.estimated_tokens {
                // Re-borrow for context window check.
                let provider_max = self
                    .providers
                    .get(id)
                    .map(|s| provider_context_limit(&s.provider))
                    .unwrap_or(8_000);
                if estimated > provider_max {
                    self.record_failover(
                        id.clone(),
                        FailoverReason::ContextOverflow {
                            estimated_tokens: estimated,
                            provider_max,
                        },
                        now_ms,
                    );
                    continue;
                }
            }

            // All gates passed — return this provider.
            return Ok(&self.providers.get(id).expect("id exists").provider);
        }

        // No provider qualified.
        if constraints.local_only {
            Err(FailoverReason::PrivacyConstraint)
        } else {
            Err(FailoverReason::FreeTierExhausted)
        }
    }

    /// Record a failover event into the ring buffer.
    fn record_failover(&mut self, provider_id: String, reason: FailoverReason, timestamp_ms: u64) {
        self.failover_history.push_front(FailoverEvent {
            provider_id,
            reason,
            timestamp_ms,
        });
        if self.failover_history.len() > MAX_FAILOVER_HISTORY {
            self.failover_history.pop_back();
        }
    }

    /// Return an ordered list of provider IDs to try for automatic failover.
    ///
    /// Applies health, rate-limit, privacy, token-cap, and context-window gates.
    /// Returns up to `policy.max_attempts` candidates (fastest-first).
    /// An empty vec means no providers qualify.
    pub fn select_failover_chain(
        &mut self,
        constraints: &SelectionConstraints,
        policy: &FailoverPolicy,
    ) -> Vec<String> {
        self.clear_expired_limits();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let ids: Vec<String> = self.sorted_ids.clone();
        let mut chain = Vec::new();

        for id in &ids {
            if chain.len() >= policy.max_attempts as usize {
                break;
            }

            let Some(status) = self.providers.get(id) else {
                continue;
            };

            // Health gate
            if !status.is_healthy {
                self.record_failover(id.clone(), FailoverReason::Unhealthy, now_ms);
                continue;
            }

            // Circuit breaker gate — check state without mutable borrow first.
            let cb_state = status.circuit_breaker.state();
            let is_rate_limited = status.is_rate_limited;

            if cb_state == CircuitState::Open {
                let allowed = self
                    .providers
                    .get_mut(id)
                    .map(|s| s.circuit_breaker.is_allowed())
                    .unwrap_or(false);
                if !allowed {
                    self.record_failover(id.clone(), FailoverReason::CircuitBreakerOpen, now_ms);
                    continue;
                }
            }

            // Rate-limit gate
            if is_rate_limited {
                self.record_failover(id.clone(), FailoverReason::RateLimit, now_ms);
                continue;
            }

            // Privacy gate — all free catalogue providers are cloud-based
            if constraints.local_only {
                self.record_failover(id.clone(), FailoverReason::PrivacyConstraint, now_ms);
                continue;
            }

            // Token-cap gate
            if let (Some(estimated), Some(cap)) =
                (constraints.estimated_tokens, constraints.token_cap)
            {
                if estimated > cap {
                    self.record_failover(
                        id.clone(),
                        FailoverReason::TokenCapExceeded { cap, estimated },
                        now_ms,
                    );
                    continue;
                }
            }

            // Context-window gate
            if let Some(estimated) = constraints.estimated_tokens {
                let provider_max = self
                    .providers
                    .get(id)
                    .map(|s| provider_context_limit(&s.provider))
                    .unwrap_or(8_000);
                if estimated > provider_max {
                    self.record_failover(
                        id.clone(),
                        FailoverReason::ContextOverflow {
                            estimated_tokens: estimated,
                            provider_max,
                        },
                        now_ms,
                    );
                    continue;
                }
            }

            chain.push(id.clone());
        }

        chain
    }

    /// Mark a provider as rate-limited with a minimum cooldown from policy.
    pub fn record_rate_limit_with_cooldown(&mut self, provider_id: &str, min_cooldown_secs: u64) {
        if let Some(status) = self.providers.get_mut(provider_id) {
            status.is_rate_limited = true;
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            // Only extend the reset time, never shorten it
            let min_reset = now_epoch + min_cooldown_secs;
            match status.rate_limit_reset {
                Some(existing) if existing > min_reset => {}
                _ => status.rate_limit_reset = Some(min_reset),
            }
        }
    }

    /// Record a successful request to a provider's circuit breaker.
    ///
    /// Call this after a provider returns a valid response. Resets the
    /// circuit breaker to CLOSED if it was in HALF_OPEN probe state.
    pub fn record_request_success(&mut self, provider_id: &str) {
        if let Some(status) = self.providers.get_mut(provider_id) {
            status.circuit_breaker.record_success();
        }
    }

    /// Record a failed request to a provider's circuit breaker.
    ///
    /// Call this after a provider returns an error (timeout, 5xx, etc.).
    /// May trip the circuit breaker to OPEN after the failure threshold.
    pub fn record_request_failure(&mut self, provider_id: &str) {
        if let Some(status) = self.providers.get_mut(provider_id) {
            status.circuit_breaker.record_failure();
        }
    }

    /// Get the circuit breaker state for a specific provider.
    pub fn circuit_breaker_state(&self, provider_id: &str) -> Option<CircuitState> {
        self.providers
            .get(provider_id)
            .map(|s| s.circuit_breaker.state())
    }

    /// Sort the internal id list by latency (fastest first).
    /// Providers with no latency data are placed at the end.
    fn sort_by_latency(&mut self) {
        let providers = &self.providers;
        self.sorted_ids.sort_by(|a, b| {
            let la = providers.get(a).and_then(|s| s.latency);
            let lb = providers.get(b).and_then(|s| s.latency);
            match (la, lb) {
                (Some(a), Some(b)) => a.cmp(&b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
    }
}

impl Default for ProviderRotator {
    fn default() -> Self {
        Self::new()
    }
}

/// Approximate max context window for a free provider's default model.
///
/// These are conservative estimates — most free-tier models support at least
/// 8k context, and many support 128k+. We use known minimums to avoid
/// sending a request that will be rejected or silently truncated.
fn provider_context_limit(provider: &FreeProvider) -> u32 {
    match provider.id.as_str() {
        "groq" => 128_000,          // llama-3.3-70b-versatile: 128k
        "cerebras" => 128_000,      // llama-3.3-70b: 128k
        "siliconflow" => 32_000,    // Qwen3-8B: 32k
        "mistral" => 32_000,        // mistral-small-latest: 32k
        "github-models" => 128_000, // gpt-4o: 128k
        "openrouter" => 128_000,    // varies, 128k conservative
        "nvidia-nim" => 128_000,    // nemotron: 128k
        "gemini" => 1_000_000,      // gemini-3-flash: 1M
        "pollinations" => 128_000,  // llama/various: 128k
        _ => 8_000,                 // safe fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    // ── helpers ──────────────────────────────────────────────────────

    fn make_rotator_with(ids: &[&str]) -> ProviderRotator {
        let catalogue = free_provider_catalogue();
        let selected: Vec<FreeProvider> = catalogue
            .into_iter()
            .filter(|p| ids.contains(&p.id.as_str()))
            .collect();
        let sorted_ids: Vec<String> = selected.iter().map(|p| p.id.clone()).collect();
        let providers = selected
            .into_iter()
            .map(|p| (p.id.clone(), ProviderStatus::new(p)))
            .collect();
        ProviderRotator {
            providers,
            sorted_ids,
            failover_history: VecDeque::new(),
        }
    }

    // ── rotation & skipping ─────────────────────────────────────────

    #[test]
    fn next_healthy_provider_returns_first_available() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        let p = rotator.next_healthy_provider().unwrap();
        assert_eq!(p.id, "groq");
    }

    #[test]
    fn skips_rate_limited_provider() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.record_rate_limit("groq");
        let p = rotator.next_healthy_provider().unwrap();
        assert_eq!(p.id, "cerebras");
    }

    #[test]
    fn skips_unhealthy_provider() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.providers.get_mut("groq").unwrap().is_healthy = false;
        let p = rotator.next_healthy_provider().unwrap();
        assert_eq!(p.id, "cerebras");
    }

    #[test]
    fn returns_none_when_all_rate_limited() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.record_rate_limit("groq");
        rotator.record_rate_limit("cerebras");
        assert!(rotator.next_healthy_provider().is_none());
    }

    // ── record_response_headers ─────────────────────────────────────

    #[test]
    fn records_remaining_requests_header() {
        let mut rotator = make_rotator_with(&["groq"]);
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-ratelimit-remaining-requests",
            HeaderValue::from_static("42"),
        );
        rotator.record_response_headers("groq", &headers);

        let s = rotator.providers.get("groq").unwrap();
        assert_eq!(s.remaining_requests, Some(42));
        assert_eq!(s.requests_sent, 1);
        assert!(!s.is_rate_limited);
    }

    #[test]
    fn records_remaining_tokens_header() {
        let mut rotator = make_rotator_with(&["groq"]);
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-ratelimit-remaining-tokens",
            HeaderValue::from_static("5000"),
        );
        rotator.record_response_headers("groq", &headers);

        let s = rotator.providers.get("groq").unwrap();
        assert_eq!(s.remaining_tokens, Some(5000));
    }

    #[test]
    fn records_reset_header() {
        let mut rotator = make_rotator_with(&["groq"]);
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-reset", HeaderValue::from_static("1700000000"));
        rotator.record_response_headers("groq", &headers);

        let s = rotator.providers.get("groq").unwrap();
        assert_eq!(s.rate_limit_reset, Some(1700000000));
    }

    #[test]
    fn auto_rate_limits_when_remaining_zero() {
        let mut rotator = make_rotator_with(&["groq"]);
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-ratelimit-remaining-requests",
            HeaderValue::from_static("0"),
        );
        rotator.record_response_headers("groq", &headers);

        let s = rotator.providers.get("groq").unwrap();
        assert!(s.is_rate_limited);
        assert_eq!(s.remaining_requests, Some(0));
    }

    #[test]
    fn ignores_unknown_provider() {
        let mut rotator = make_rotator_with(&["groq"]);
        let headers = HeaderMap::new();
        // Should not panic
        rotator.record_response_headers("nonexistent", &headers);
    }

    // ── all_exhausted ───────────────────────────────────────────────

    #[test]
    fn all_exhausted_false_when_some_available() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.record_rate_limit("groq");
        assert!(!rotator.all_exhausted());
    }

    #[test]
    fn all_exhausted_true_when_all_rate_limited() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.record_rate_limit("groq");
        rotator.record_rate_limit("cerebras");
        assert!(rotator.all_exhausted());
    }

    #[test]
    fn all_exhausted_true_with_mixed_unhealthy_and_rate_limited() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.providers.get_mut("groq").unwrap().is_healthy = false;
        rotator.record_rate_limit("cerebras");
        assert!(rotator.all_exhausted());
    }

    // ── sorting by latency ──────────────────────────────────────────

    #[test]
    fn sort_by_latency_reorders_providers() {
        let mut rotator = make_rotator_with(&["groq", "cerebras", "mistral"]);

        // Simulate latency results: mistral fastest, then groq, then cerebras
        rotator.providers.get_mut("groq").unwrap().latency = Some(Duration::from_millis(200));
        rotator.providers.get_mut("cerebras").unwrap().latency = Some(Duration::from_millis(500));
        rotator.providers.get_mut("mistral").unwrap().latency = Some(Duration::from_millis(50));

        rotator.sort_by_latency();

        let p = rotator.next_healthy_provider().unwrap();
        assert_eq!(p.id, "mistral", "fastest provider should be first");

        // Verify full order
        assert_eq!(rotator.sorted_ids[0], "mistral");
        assert_eq!(rotator.sorted_ids[1], "groq");
        assert_eq!(rotator.sorted_ids[2], "cerebras");
    }

    #[test]
    fn providers_without_latency_sorted_last() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.providers.get_mut("cerebras").unwrap().latency = Some(Duration::from_millis(100));
        // groq has no latency data

        rotator.sort_by_latency();

        assert_eq!(rotator.sorted_ids[0], "cerebras");
        assert_eq!(rotator.sorted_ids[1], "groq");
    }

    // ── rate limit expiry ───────────────────────────────────────────

    #[test]
    fn rate_limited_provider_available_after_reset() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);

        // Rate-limit groq with a reset time in the past
        rotator.record_rate_limit("groq");
        rotator.providers.get_mut("groq").unwrap().rate_limit_reset = Some(1);

        // groq should be available again because reset epoch (1) < now
        let p = rotator.next_healthy_provider().unwrap();
        assert_eq!(p.id, "groq");
    }

    #[test]
    fn rate_limited_provider_stays_limited_before_reset() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);

        // Rate-limit groq with a reset time far in the future
        rotator.record_rate_limit("groq");
        rotator.providers.get_mut("groq").unwrap().rate_limit_reset = Some(u64::MAX);

        let p = rotator.next_healthy_provider().unwrap();
        assert_eq!(p.id, "cerebras", "groq should still be skipped");
    }

    #[test]
    fn clear_expired_limits_resets_flags() {
        let mut rotator = make_rotator_with(&["groq"]);
        rotator.record_rate_limit("groq");
        rotator.providers.get_mut("groq").unwrap().rate_limit_reset = Some(1);

        rotator.clear_expired_limits();

        let s = rotator.providers.get("groq").unwrap();
        assert!(!s.is_rate_limited);
        assert!(s.rate_limit_reset.is_none());
    }

    #[test]
    fn clear_expired_limits_keeps_future_limits() {
        let mut rotator = make_rotator_with(&["groq"]);
        rotator.record_rate_limit("groq");
        rotator.providers.get_mut("groq").unwrap().rate_limit_reset = Some(u64::MAX);

        rotator.clear_expired_limits();

        let s = rotator.providers.get("groq").unwrap();
        assert!(s.is_rate_limited);
    }

    #[test]
    fn all_exhausted_false_after_reset_expires() {
        let mut rotator = make_rotator_with(&["groq"]);
        rotator.record_rate_limit("groq");
        rotator.providers.get_mut("groq").unwrap().rate_limit_reset = Some(1);

        // Reset is in the past, so provider is available
        assert!(!rotator.all_exhausted());
    }

    // ── constructor ─────────────────────────────────────────────────

    #[test]
    fn new_loads_all_catalogue_providers() {
        let rotator = ProviderRotator::new();
        let catalogue = free_provider_catalogue();
        assert_eq!(rotator.providers.len(), catalogue.len());
    }

    #[test]
    fn default_all_healthy_and_not_rate_limited() {
        let rotator = ProviderRotator::new();
        for status in rotator.providers.values() {
            assert!(status.is_healthy);
            assert!(!status.is_rate_limited);
            assert_eq!(status.requests_sent, 0);
        }
    }

    #[test]
    fn record_rate_limit_increments_nothing_for_unknown() {
        let mut rotator = make_rotator_with(&["groq"]);
        // Should not panic
        rotator.record_rate_limit("nonexistent");
        assert!(!rotator.providers.get("groq").unwrap().is_rate_limited);
    }

    #[test]
    fn multiple_headers_parsed_together() {
        let mut rotator = make_rotator_with(&["groq"]);
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-ratelimit-remaining-requests",
            HeaderValue::from_static("10"),
        );
        headers.insert(
            "x-ratelimit-remaining-tokens",
            HeaderValue::from_static("3000"),
        );
        headers.insert("x-ratelimit-reset", HeaderValue::from_static("1700000000"));
        rotator.record_response_headers("groq", &headers);

        let s = rotator.providers.get("groq").unwrap();
        assert_eq!(s.remaining_requests, Some(10));
        assert_eq!(s.remaining_tokens, Some(3000));
        assert_eq!(s.rate_limit_reset, Some(1700000000));
        assert_eq!(s.requests_sent, 1);
    }

    // ── select_provider (constraint-aware failover) ─────────────────

    #[test]
    fn select_provider_no_constraints_returns_first_healthy() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        let constraints = SelectionConstraints::default();
        let p = rotator.select_provider(&constraints).unwrap();
        assert_eq!(p.id, "groq");
    }

    #[test]
    fn select_provider_skips_rate_limited() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.record_rate_limit("groq");
        let constraints = SelectionConstraints::default();
        let p = rotator.select_provider(&constraints).unwrap();
        assert_eq!(p.id, "cerebras");
    }

    #[test]
    fn select_provider_skips_unhealthy() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.providers.get_mut("groq").unwrap().is_healthy = false;
        let constraints = SelectionConstraints::default();
        let p = rotator.select_provider(&constraints).unwrap();
        assert_eq!(p.id, "cerebras");
    }

    #[test]
    fn select_provider_local_only_rejects_all_cloud() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        let constraints = SelectionConstraints {
            local_only: true,
            ..Default::default()
        };
        let err = rotator.select_provider(&constraints).unwrap_err();
        assert_eq!(err, FailoverReason::PrivacyConstraint);
    }

    #[test]
    fn select_provider_token_cap_filters_providers() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        // Set estimated tokens higher than a cap
        let constraints = SelectionConstraints {
            estimated_tokens: Some(5000),
            token_cap: Some(2000),
            local_only: false,
        };
        let err = rotator.select_provider(&constraints).unwrap_err();
        assert_eq!(
            err,
            FailoverReason::FreeTierExhausted,
            "all providers should be skipped due to token cap"
        );
    }

    #[test]
    fn select_provider_context_overflow_skips_small_providers() {
        let mut rotator = make_rotator_with(&["siliconflow", "gemini"]);
        // siliconflow has 32k context, gemini has 1M
        let constraints = SelectionConstraints {
            estimated_tokens: Some(100_000),
            token_cap: None,
            local_only: false,
        };
        let p = rotator.select_provider(&constraints).unwrap();
        assert_eq!(
            p.id, "gemini",
            "should skip siliconflow (32k) and pick gemini (1M)"
        );
    }

    #[test]
    fn select_provider_records_failover_events() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.record_rate_limit("groq");
        rotator.providers.get_mut("cerebras").unwrap().is_healthy = false;

        let constraints = SelectionConstraints::default();
        let _ = rotator.select_provider(&constraints);

        // Should have recorded at least 2 failover events
        assert!(rotator.failover_history.len() >= 2);
        let events: Vec<_> = rotator.failover_history.iter().collect();
        assert_eq!(events[0].provider_id, "cerebras");
        assert_eq!(events[0].reason, FailoverReason::Unhealthy);
        assert_eq!(events[1].provider_id, "groq");
        assert_eq!(events[1].reason, FailoverReason::RateLimit);
    }

    #[test]
    fn select_provider_all_exhausted_returns_free_tier_exhausted() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.record_rate_limit("groq");
        rotator.record_rate_limit("cerebras");
        let constraints = SelectionConstraints::default();
        let err = rotator.select_provider(&constraints).unwrap_err();
        assert_eq!(err, FailoverReason::FreeTierExhausted);
    }

    #[test]
    fn select_provider_failover_history_capped() {
        let mut rotator = make_rotator_with(&["groq"]);
        rotator.providers.get_mut("groq").unwrap().is_healthy = false;

        // Call select_provider many times to exceed MAX_FAILOVER_HISTORY
        for _ in 0..(MAX_FAILOVER_HISTORY + 20) {
            let _ = rotator.select_provider(&SelectionConstraints::default());
        }

        assert_eq!(
            rotator.failover_history.len(),
            MAX_FAILOVER_HISTORY,
            "ring buffer should cap at MAX_FAILOVER_HISTORY"
        );
    }

    #[test]
    fn failover_summary_reflects_state() {
        let mut rotator = make_rotator_with(&["groq", "cerebras", "mistral"]);
        rotator.record_rate_limit("groq");
        rotator.providers.get_mut("cerebras").unwrap().is_healthy = false;

        let summary = rotator.failover_summary();
        assert_eq!(summary.healthy_count, 1);
        assert_eq!(summary.rate_limited_count, 1);
        assert_eq!(summary.unhealthy_count, 1);
        assert!(!summary.all_exhausted);
        assert_eq!(summary.selected_provider_id, Some("mistral".to_string()));
    }

    // ── select_failover_chain ───────────────────────────────────────

    #[test]
    fn failover_chain_returns_ordered_healthy_providers() {
        let mut rotator = make_rotator_with(&["groq", "cerebras", "mistral"]);
        let policy = FailoverPolicy {
            max_attempts: 3,
            respect_privacy: false,
            min_cooldown_secs: 60,
        };
        let constraints = SelectionConstraints::default();
        let chain = rotator.select_failover_chain(&constraints, &policy);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0], "groq");
        assert_eq!(chain[1], "cerebras");
        assert_eq!(chain[2], "mistral");
    }

    #[test]
    fn failover_chain_skips_rate_limited() {
        let mut rotator = make_rotator_with(&["groq", "cerebras", "mistral"]);
        rotator.record_rate_limit("groq");
        let policy = FailoverPolicy::default();
        let constraints = SelectionConstraints::default();
        let chain = rotator.select_failover_chain(&constraints, &policy);
        assert!(!chain.contains(&"groq".to_string()));
        assert!(chain.contains(&"cerebras".to_string()));
    }

    #[test]
    fn failover_chain_respects_max_attempts() {
        let mut rotator = make_rotator_with(&["groq", "cerebras", "mistral", "gemini"]);
        let policy = FailoverPolicy {
            max_attempts: 2,
            respect_privacy: false,
            min_cooldown_secs: 60,
        };
        let constraints = SelectionConstraints::default();
        let chain = rotator.select_failover_chain(&constraints, &policy);
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn failover_chain_empty_when_all_unhealthy() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        rotator.providers.get_mut("groq").unwrap().is_healthy = false;
        rotator.providers.get_mut("cerebras").unwrap().is_healthy = false;
        let policy = FailoverPolicy::default();
        let constraints = SelectionConstraints::default();
        let chain = rotator.select_failover_chain(&constraints, &policy);
        assert!(chain.is_empty());
    }

    #[test]
    fn failover_chain_local_only_skips_all_cloud() {
        let mut rotator = make_rotator_with(&["groq", "cerebras"]);
        let policy = FailoverPolicy::default();
        let constraints = SelectionConstraints {
            local_only: true,
            ..Default::default()
        };
        let chain = rotator.select_failover_chain(&constraints, &policy);
        assert!(chain.is_empty());
    }

    #[test]
    fn failover_chain_skips_context_overflow() {
        let mut rotator = make_rotator_with(&["siliconflow", "gemini"]);
        let policy = FailoverPolicy {
            max_attempts: 5,
            respect_privacy: false,
            min_cooldown_secs: 60,
        };
        let constraints = SelectionConstraints {
            estimated_tokens: Some(100_000),
            ..Default::default()
        };
        let chain = rotator.select_failover_chain(&constraints, &policy);
        // siliconflow is 32k, gemini is 1M
        assert_eq!(chain, vec!["gemini"]);
    }

    // ── record_rate_limit_with_cooldown ─────────────────────────────

    #[test]
    fn cooldown_sets_minimum_reset_time() {
        let mut rotator = make_rotator_with(&["groq"]);
        rotator.record_rate_limit_with_cooldown("groq", 120);
        let s = rotator.providers.get("groq").unwrap();
        assert!(s.is_rate_limited);
        assert!(s.rate_limit_reset.is_some());
        // Reset should be at least 120s from now
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(s.rate_limit_reset.unwrap() >= now + 119); // allow 1s drift
    }

    #[test]
    fn cooldown_does_not_shorten_existing_reset() {
        let mut rotator = make_rotator_with(&["groq"]);
        // Set a far-future reset time
        rotator.providers.get_mut("groq").unwrap().rate_limit_reset = Some(u64::MAX - 1000);
        rotator.providers.get_mut("groq").unwrap().is_rate_limited = true;
        // Apply a short cooldown — should NOT reduce the reset
        rotator.record_rate_limit_with_cooldown("groq", 60);
        let s = rotator.providers.get("groq").unwrap();
        assert_eq!(s.rate_limit_reset, Some(u64::MAX - 1000));
    }
}
