/**
 * Pinia store for provider health checks and rate-limit rotation.
 *
 * Wraps the Tauri `health_check_providers` and `get_next_provider` commands.
 * Also tracks provider health on the frontend for browser-only mode.
 */
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { ProviderHealthInfo } from '../types';

export const useProviderHealthStore = defineStore('providerHealth', () => {
  /** Per-provider health/rate-limit status. */
  const providers = ref<ProviderHealthInfo[]>([]);
  /** Whether all free providers are exhausted. */
  const allExhausted = computed(() =>
    providers.value.length > 0 &&
    providers.value.every((p) => p.is_rate_limited || !p.is_healthy),
  );
  /** Whether a health check is currently running. */
  const isChecking = ref(false);
  /** Error from the last operation, if any. */
  const error = ref<string | null>(null);

  /** Fetch current provider health status from the backend. */
  async function fetchProviderHealth(): Promise<void> {
    error.value = null;
    try {
      providers.value = await invoke<ProviderHealthInfo[]>('health_check_providers');
    } catch (e) {
      error.value = String(e);
    }
  }

  /** Get the next healthy provider ID from the rotator. */
  async function getNextProvider(): Promise<string | null> {
    try {
      return await invoke<string | null>('get_next_provider');
    } catch {
      return null;
    }
  }

  /**
   * Browser-side rate-limit tracking for when Tauri is unavailable.
   * Records that a provider returned HTTP 429 or similar.
   */
  function markRateLimited(providerId: string): void {
    const existing = providers.value.find((p) => p.id === providerId);
    if (existing) {
      existing.is_rate_limited = true;
    } else {
      providers.value.push({
        id: providerId,
        display_name: providerId,
        is_healthy: true,
        is_rate_limited: true,
        requests_sent: 0,
        remaining_requests: null,
        remaining_tokens: null,
        latency_ms: null,
      });
    }
  }

  /**
   * Browser-side: find the next healthy, non-rate-limited provider
   * from the FALLBACK_FREE_PROVIDERS list.
   */
  function nextHealthyBrowserProvider(
    allProviders: Array<{ id: string }>,
  ): string | null {
    for (const p of allProviders) {
      const status = providers.value.find((s) => s.id === p.id);
      if (!status || (!status.is_rate_limited && status.is_healthy)) {
        return p.id;
      }
    }
    return null;
  }

  /** Reset all tracking state. */
  function reset(): void {
    providers.value = [];
    error.value = null;
    isChecking.value = false;
  }

  return {
    providers,
    allExhausted,
    isChecking,
    error,
    fetchProviderHealth,
    getNextProvider,
    markRateLimited,
    nextHealthyBrowserProvider,
    reset,
  };
});
