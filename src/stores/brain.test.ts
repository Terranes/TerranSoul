import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useBrainStore } from './brain';
import type { ModelRecommendation, OllamaStatus, SystemInfo } from '../types';

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

describe('brain store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('hasBrain is false when activeBrain is null', () => {
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
});
