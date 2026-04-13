import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useBrainStore } from './brain';
import type { BrainMode, FreeProvider, ModelRecommendation, OllamaStatus, SystemInfo } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const sampleInfo: SystemInfo = {
  total_ram_mb: 16384,
  ram_tier_label: '16–32 GB',
  cpu_cores: 8,
  cpu_name: 'Intel Core i7',
  os_name: 'Ubuntu 24.04',
  arch: 'x86_64',
};

const sampleRec: ModelRecommendation = {
  model_tag: 'gemma3:4b',
  display_name: 'Gemma 3 4B',
  description: 'Fast and capable.',
  required_ram_mb: 8192,
  is_top_pick: true,
};

const offlineStatus: OllamaStatus = { running: false, model_count: 0 };

const sampleFreeProvider: FreeProvider = {
  id: 'groq',
  display_name: 'Groq',
  base_url: 'https://api.groq.com/openai',
  model: 'llama-3.3-70b-versatile',
  rpm_limit: 30,
  rpd_limit: 1000,
  requires_api_key: true,
  notes: 'Fast inference',
};

describe('brain store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('hasBrain is false when activeBrain is null and brainMode is null', () => {
    const store = useBrainStore();
    expect(store.hasBrain).toBe(false);
  });

  it('hasBrain is true after setActiveBrain', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useBrainStore();
    await store.setActiveBrain('gemma3:4b');
    expect(store.hasBrain).toBe(true);
    expect(store.activeBrain).toBe('gemma3:4b');
  });

  it('clearActiveBrain sets activeBrain to null', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useBrainStore();
    store.activeBrain = 'gemma3:4b';
    await store.clearActiveBrain();
    expect(store.activeBrain).toBeNull();
  });

  it('fetchSystemInfo stores system info', async () => {
    mockInvoke.mockResolvedValue(sampleInfo);
    const store = useBrainStore();
    await store.fetchSystemInfo();
    expect(store.systemInfo).toEqual(sampleInfo);
  });

  it('fetchRecommendations stores recommendations', async () => {
    mockInvoke.mockResolvedValue([sampleRec]);
    const store = useBrainStore();
    await store.fetchRecommendations();
    expect(store.recommendations).toHaveLength(1);
    expect(store.topRecommendation?.model_tag).toBe('gemma3:4b');
  });

  it('topRecommendation is null when no recommendations', () => {
    const store = useBrainStore();
    expect(store.topRecommendation).toBeNull();
  });

  it('checkOllamaStatus stores status', async () => {
    mockInvoke.mockResolvedValue(offlineStatus);
    const store = useBrainStore();
    await store.checkOllamaStatus();
    expect(store.ollamaStatus.running).toBe(false);
  });

  it('pullModel sets isPulling during pull', async () => {
    let resolve!: () => void;
    mockInvoke.mockImplementation(
      (cmd: string) =>
        cmd === 'pull_ollama_model'
          ? new Promise<void>((r) => { resolve = r; })
          : Promise.resolve([]),
    );
    const store = useBrainStore();
    const p = store.pullModel('gemma3:4b');
    expect(store.isPulling).toBe(true);
    resolve();
    await p;
    expect(store.isPulling).toBe(false);
  });

  it('pullModel sets pullError on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('Ollama not found'));
    const store = useBrainStore();
    const ok = await store.pullModel('gemma3:4b');
    expect(ok).toBe(false);
    expect(store.pullError).toContain('Ollama not found');
  });

  // ── Three-Tier Brain Tests ───────────────────────────────────────────────

  it('fetchFreeProviders stores provider list', async () => {
    mockInvoke.mockResolvedValue([sampleFreeProvider]);
    const store = useBrainStore();
    await store.fetchFreeProviders();
    expect(store.freeProviders).toHaveLength(1);
    expect(store.freeProviders[0].id).toBe('groq');
  });

  it('loadBrainMode stores brain mode', async () => {
    const mode: BrainMode = { mode: 'free_api', provider_id: 'groq', api_key: null };
    mockInvoke.mockResolvedValue(mode);
    const store = useBrainStore();
    await store.loadBrainMode();
    expect(store.brainMode).toEqual(mode);
  });

  it('loadBrainMode stores null when no mode configured', async () => {
    mockInvoke.mockResolvedValue(null);
    const store = useBrainStore();
    await store.loadBrainMode();
    expect(store.brainMode).toBeNull();
  });

  it('setBrainMode calls invoke and updates state for free_api', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useBrainStore();
    const mode: BrainMode = { mode: 'free_api', provider_id: 'cerebras', api_key: null };
    await store.setBrainMode(mode);
    expect(mockInvoke).toHaveBeenCalledWith('set_brain_mode', { mode });
    expect(store.brainMode).toEqual(mode);
    expect(store.activeBrain).toBeNull();
  });

  it('setBrainMode updates activeBrain for local_ollama', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useBrainStore();
    const mode: BrainMode = { mode: 'local_ollama', model: 'phi-4:latest' };
    await store.setBrainMode(mode);
    expect(store.brainMode).toEqual(mode);
    expect(store.activeBrain).toBe('phi-4:latest');
  });

  it('setBrainMode supports paid_api mode', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useBrainStore();
    const mode: BrainMode = {
      mode: 'paid_api',
      provider: 'openai',
      api_key: 'sk-test',
      model: 'gpt-4o',
      base_url: 'https://api.openai.com',
    };
    await store.setBrainMode(mode);
    expect(store.brainMode).toEqual(mode);
    expect(store.activeBrain).toBeNull();
  });

  it('hasBrain is true when brainMode is set even if activeBrain is null', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useBrainStore();
    const mode: BrainMode = { mode: 'free_api', provider_id: 'groq', api_key: null };
    await store.setBrainMode(mode);
    expect(store.activeBrain).toBeNull();
    expect(store.hasBrain).toBe(true);
  });

  it('freeProviders starts as empty array', () => {
    const store = useBrainStore();
    expect(store.freeProviders).toEqual([]);
  });

  it('brainMode starts as null', () => {
    const store = useBrainStore();
    expect(store.brainMode).toBeNull();
  });

  // ── Auto-Configure Free API Tests ──────────────────────────────────────

  it('autoConfigureFreeApi sets brainMode to free_api with groq', () => {
    const store = useBrainStore();
    expect(store.hasBrain).toBe(false);
    store.autoConfigureFreeApi();
    expect(store.hasBrain).toBe(true);
    expect(store.brainMode).toEqual({
      mode: 'free_api',
      provider_id: 'groq',
      api_key: null,
    });
  });

  it('autoConfigureFreeApi populates fallback free providers', () => {
    const store = useBrainStore();
    expect(store.freeProviders).toEqual([]);
    store.autoConfigureFreeApi();
    expect(store.freeProviders.length).toBeGreaterThanOrEqual(2);
    expect(store.freeProviders[0].id).toBe('groq');
  });

  it('isFreeApiMode is true after autoConfigureFreeApi', () => {
    const store = useBrainStore();
    expect(store.isFreeApiMode).toBe(false);
    store.autoConfigureFreeApi();
    expect(store.isFreeApiMode).toBe(true);
  });

  it('isFreeApiMode is false for local_ollama', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useBrainStore();
    await store.setBrainMode({ mode: 'local_ollama', model: 'gemma3:4b' });
    expect(store.isFreeApiMode).toBe(false);
  });

  it('initialise auto-defaults to free API when Tauri unavailable', async () => {
    mockInvoke.mockRejectedValue(new Error('window.__TAURI_INTERNALS__ not found'));
    const store = useBrainStore();
    await store.initialise();
    expect(store.hasBrain).toBe(true);
    expect(store.brainMode?.mode).toBe('free_api');
    expect(store.freeProviders.length).toBeGreaterThan(0);
    expect(store.isLoading).toBe(false);
  });
});
