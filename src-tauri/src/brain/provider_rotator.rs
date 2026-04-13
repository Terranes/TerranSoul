use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::brain::free_api::{free_provider_catalogue, FreeProvider};

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
        headers.insert("x-ratelimit-remaining-requests", HeaderValue::from_static("42"));
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
        headers.insert("x-ratelimit-remaining-tokens", HeaderValue::from_static("5000"));
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
        headers.insert("x-ratelimit-remaining-requests", HeaderValue::from_static("0"));
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
        rotator.providers.get_mut("groq").unwrap().rate_limit_reset =
            Some(u64::MAX);

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
        headers.insert("x-ratelimit-remaining-requests", HeaderValue::from_static("10"));
        headers.insert("x-ratelimit-remaining-tokens", HeaderValue::from_static("3000"));
        headers.insert("x-ratelimit-reset", HeaderValue::from_static("1700000000"));
        rotator.record_response_headers("groq", &headers);

        let s = rotator.providers.get("groq").unwrap();
        assert_eq!(s.remaining_requests, Some(10));
        assert_eq!(s.remaining_tokens, Some(3000));
        assert_eq!(s.rate_limit_reset, Some(1700000000));
        assert_eq!(s.requests_sent, 1);
    }
}
