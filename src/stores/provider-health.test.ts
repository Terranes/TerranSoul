/**
 * Integration tests for the provider health store.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useProviderHealthStore } from './provider-health';
import type { ProviderHealthInfo } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('provider health store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('fetchProviderHealth populates providers', async () => {
    const data: ProviderHealthInfo[] = [
      {
        id: 'groq',
        display_name: 'Groq',
        is_healthy: true,
        is_rate_limited: false,
        requests_sent: 5,
        remaining_requests: 25,
        remaining_tokens: null,
        latency_ms: 120,
      },
    ];
    mockInvoke.mockResolvedValueOnce(data);

    const store = useProviderHealthStore();
    await store.fetchProviderHealth();

    expect(mockInvoke).toHaveBeenCalledWith('health_check_providers');
    expect(store.providers).toEqual(data);
  });

  it('fetchProviderHealth sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('no backend'));

    const store = useProviderHealthStore();
    await store.fetchProviderHealth();

    expect(store.error).toBe('Error: no backend');
  });

  it('getNextProvider returns provider id', async () => {
    mockInvoke.mockResolvedValueOnce('cerebras');

    const store = useProviderHealthStore();
    const result = await store.getNextProvider();

    expect(mockInvoke).toHaveBeenCalledWith('get_next_provider');
    expect(result).toBe('cerebras');
  });

  it('getNextProvider returns null on error', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('fail'));

    const store = useProviderHealthStore();
    const result = await store.getNextProvider();

    expect(result).toBeNull();
  });

  it('allExhausted is false when some providers are healthy', () => {
    const store = useProviderHealthStore();
    store.providers = [
      { id: 'a', display_name: 'A', is_healthy: true, is_rate_limited: false, requests_sent: 0, remaining_requests: null, remaining_tokens: null, latency_ms: null },
      { id: 'b', display_name: 'B', is_healthy: true, is_rate_limited: true, requests_sent: 0, remaining_requests: null, remaining_tokens: null, latency_ms: null },
    ];
    expect(store.allExhausted).toBe(false);
  });

  it('allExhausted is true when all providers are rate-limited or unhealthy', () => {
    const store = useProviderHealthStore();
    store.providers = [
      { id: 'a', display_name: 'A', is_healthy: true, is_rate_limited: true, requests_sent: 0, remaining_requests: null, remaining_tokens: null, latency_ms: null },
      { id: 'b', display_name: 'B', is_healthy: false, is_rate_limited: false, requests_sent: 0, remaining_requests: null, remaining_tokens: null, latency_ms: null },
    ];
    expect(store.allExhausted).toBe(true);
  });

  it('allExhausted is false when no providers exist', () => {
    const store = useProviderHealthStore();
    expect(store.allExhausted).toBe(false);
  });

  it('markRateLimited updates existing provider', () => {
    const store = useProviderHealthStore();
    store.providers = [
      { id: 'groq', display_name: 'Groq', is_healthy: true, is_rate_limited: false, requests_sent: 3, remaining_requests: null, remaining_tokens: null, latency_ms: null },
    ];
    store.markRateLimited('groq');
    expect(store.providers[0].is_rate_limited).toBe(true);
  });

  it('markRateLimited adds new entry for unknown provider', () => {
    const store = useProviderHealthStore();
    store.markRateLimited('new-provider');
    expect(store.providers).toHaveLength(1);
    expect(store.providers[0].id).toBe('new-provider');
    expect(store.providers[0].is_rate_limited).toBe(true);
  });

  it('nextHealthyBrowserProvider returns first non-limited provider', () => {
    const store = useProviderHealthStore();
    store.providers = [
      { id: 'groq', display_name: 'Groq', is_healthy: true, is_rate_limited: true, requests_sent: 0, remaining_requests: null, remaining_tokens: null, latency_ms: null },
    ];

    const allProviders = [{ id: 'groq' }, { id: 'cerebras' }];
    const result = store.nextHealthyBrowserProvider(allProviders);
    expect(result).toBe('cerebras'); // groq is limited, cerebras has no status = available
  });

  it('nextHealthyBrowserProvider returns null when all exhausted', () => {
    const store = useProviderHealthStore();
    store.providers = [
      { id: 'groq', display_name: 'Groq', is_healthy: true, is_rate_limited: true, requests_sent: 0, remaining_requests: null, remaining_tokens: null, latency_ms: null },
      { id: 'cerebras', display_name: 'Cerebras', is_healthy: false, is_rate_limited: false, requests_sent: 0, remaining_requests: null, remaining_tokens: null, latency_ms: null },
    ];

    const allProviders = [{ id: 'groq' }, { id: 'cerebras' }];
    const result = store.nextHealthyBrowserProvider(allProviders);
    expect(result).toBeNull();
  });

  it('reset clears all state', () => {
    const store = useProviderHealthStore();
    store.providers = [
      { id: 'x', display_name: 'X', is_healthy: true, is_rate_limited: true, requests_sent: 0, remaining_requests: null, remaining_tokens: null, latency_ms: null },
    ];
    store.error = 'some error';

    store.reset();

    expect(store.providers).toEqual([]);
    expect(store.error).toBeNull();
    expect(store.isChecking).toBe(false);
  });
});
