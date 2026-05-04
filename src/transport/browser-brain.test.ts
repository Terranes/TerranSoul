import { describe, expect, it } from 'vitest';
import type { FreeProvider } from '../types';
import { browserDirectFallbackProviders, resolveBrowserBrainTransport } from './browser-brain';

const providers: FreeProvider[] = [
  {
    id: 'pollinations',
    display_name: 'Pollinations',
    base_url: 'https://gen.pollinations.ai',
    model: 'llama',
    rpm_limit: 30,
    rpd_limit: 0,
    requires_api_key: true,
    notes: 'Requires token',
  },
  {
    id: 'groq',
    display_name: 'Groq',
    base_url: 'https://api.groq.com/openai',
    model: 'llama',
    rpm_limit: 30,
    rpd_limit: 1000,
    requires_api_key: true,
    notes: 'Requires key',
  },
];

describe('browser brain transport', () => {
  it('resolves keyed free providers as direct browser transports once configured', () => {
    const result = resolveBrowserBrainTransport(
      { mode: 'free_api', provider_id: 'pollinations', api_key: 'pl-test', model: 'openai-fast' },
      providers,
    );

    expect(result.kind).toBe('direct');
    if (result.kind === 'direct') {
      expect(result.provider.providerId).toBe('pollinations');
      expect(result.provider.apiKey).toBe('pl-test');
      expect(result.provider.model).toBe('openai-fast');
    }
  });

  it('rejects keyed free providers until an API key is configured', () => {
    const result = resolveBrowserBrainTransport(
      { mode: 'free_api', provider_id: 'groq', api_key: null },
      providers,
    );

    expect(result.kind).toBe('unconfigured');
  });

  it('resolves paid API as direct browser transport', () => {
    const result = resolveBrowserBrainTransport(
      {
        mode: 'paid_api',
        provider: 'https://api.openai.com',
        api_key: 'sk-test',
        model: 'gpt-4o',
        base_url: 'https://api.openai.com',
      },
      providers,
    );

    expect(result.kind).toBe('direct');
    if (result.kind === 'direct') {
      expect(result.provider.apiKey).toBe('sk-test');
    }
  });

  it('requires a remote host for local modes in browser mode', () => {
    const result = resolveBrowserBrainTransport(
      { mode: 'local_ollama', model: 'gemma3:4b' },
      providers,
    );

    expect(result.kind).toBe('remote-required');
  });

  it('omits fallback providers that require missing API keys', () => {
    const primary = {
      baseUrl: providers[0].base_url,
      model: providers[0].model,
      apiKey: null,
      providerId: providers[0].id,
    };

    expect(browserDirectFallbackProviders(primary, { mode: 'free_api', provider_id: 'pollinations', api_key: 'pl-test' }, providers))
      .toHaveLength(1);
  });
});
