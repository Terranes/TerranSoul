import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';

const { mockInvoke } = vi.hoisted(() => ({ mockInvoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }));
// Stub the events plugin so `listen` resolves to a no-op unsubscriber
// during tests instead of trying to talk to the Tauri runtime.
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

import { useSelfImproveStore } from './self-improve';

describe('self-improve store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('starts disabled with no config and zero progress beyond static foundation', () => {
    const store = useSelfImproveStore();
    expect(store.isEnabled).toBe(false);
    expect(store.isConfigured).toBe(false);
    expect(store.canEnable).toBe(false);
    // The static `foundation` phase is marked completed in the roadmap.
    expect(store.completedCount).toBeGreaterThanOrEqual(1);
    expect(store.progressPercent).toBeGreaterThanOrEqual(0);
    expect(store.progressPercent).toBeLessThanOrEqual(100);
  });

  it('exposes a phase roadmap that includes the major milestones', () => {
    const store = useSelfImproveStore();
    const ids = store.phases.map((p) => p.id);
    expect(ids).toEqual(
      expect.arrayContaining([
        'foundation',
        'coding-llm',
        'progress-ui',
        'github-bind',
        'autonomy-loop',
        'service-tray',
        'resilience',
        'failure-triage-radar',
        'online-tool-model-radar',
        'redis-code-memory',
      ]),
    );
  });

  it('queues research-backed improvement chunks by default', () => {
    const store = useSelfImproveStore();
    const ids = store.improvementChunks.map((chunk) => chunk.id);
    expect(ids).toEqual(expect.arrayContaining([
      'research-better-approach',
      'redis-vector-memory-scout',
      'model-tool-news-radar',
    ]));
    expect(store.improvementChunks.some((chunk) => /redis/i.test(chunk.title))).toBe(true);
    expect(store.improvementChunks.some((chunk) => /model|api/i.test(chunk.title))).toBe(true);
  });

  it('promotes a high-priority bug triage chunk when the last run failed', () => {
    const store = useSelfImproveStore();
    store.metrics.last_error = 'tests failed';
    store.metrics.last_error_chunk = '28.9';
    const triage = store.improvementChunks.find((chunk) => chunk.id === 'bug-triage-from-run');
    expect(triage).toBeDefined();
    expect(triage!.priority).toBe('high');
    expect(triage!.status).toBe('ready');
    expect(triage!.description).toContain('28.9');
  });

  it('flips coding-llm phase to completed once a config is loaded', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_self_improve_settings') {
        return Promise.resolve({
          enabled: false,
          updated_at: 0,
          last_acknowledged_at: 0,
          last_provider: '',
        });
      }
      if (cmd === 'get_coding_llm_config') {
        return Promise.resolve({
          provider: 'anthropic',
          model: 'claude-sonnet-4-5',
          base_url: 'https://api.anthropic.com/v1',
          api_key: 'sk-test',
        });
      }
      if (cmd === 'list_coding_llm_recommendations') return Promise.resolve([]);
      return Promise.resolve(null);
    });

    const store = useSelfImproveStore();
    await store.initialise();
    expect(store.isConfigured).toBe(true);
    expect(store.canEnable).toBe(true);
    const codingLlmPhase = store.phases.find((p) => p.id === 'coding-llm');
    expect(codingLlmPhase?.status).toBe('completed');
  });

  it('throws when enabling without a coding LLM and surfaces lastError', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'set_self_improve_enabled') {
        return Promise.reject('Configure a Coding LLM before enabling self-improve.');
      }
      return Promise.resolve(null);
    });

    const store = useSelfImproveStore();
    await expect(store.enable()).rejects.toBeTruthy();
    expect(store.lastError).toMatch(/Coding LLM/);
  });

  it('disable always succeeds and pushes an info activity entry', async () => {
    mockInvoke.mockImplementation((cmd: string, args: unknown) => {
      if (cmd === 'set_self_improve_enabled') {
        const a = args as { enabled: boolean };
        return Promise.resolve({
          enabled: a.enabled,
          updated_at: 1,
          last_acknowledged_at: 0,
          last_provider: '',
        });
      }
      return Promise.resolve(null);
    });

    const store = useSelfImproveStore();
    const result = await store.disable();
    expect(result.enabled).toBe(false);
    expect(store.activity[0].level).toBe('info');
    expect(store.activity[0].message.toLowerCase()).toContain('disabled');
  });

  it('logActivity caps the in-memory feed at 100 entries', () => {
    const store = useSelfImproveStore();
    for (let i = 0; i < 120; i++) store.logActivity('info', `entry ${i}`);
    expect(store.activity.length).toBe(100);
    // newest first
    expect(store.activity[0].message).toBe('entry 119');
  });

  it('testCodingLlmConnection populates reachability on success', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'test_coding_llm_connection') {
        return Promise.resolve({ ok: true, summary: '✓ Reachable', detail: 'ok' });
      }
      return Promise.resolve(null);
    });
    const store = useSelfImproveStore();
    await store.testCodingLlmConnection();
    expect(store.reachability?.ok).toBe(true);
    expect(store.activity[0].level).toBe('success');
  });

  it('testCodingLlmConnection records error result when probe fails', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'test_coding_llm_connection') {
        return Promise.reject('connection refused');
      }
      return Promise.resolve(null);
    });
    const store = useSelfImproveStore();
    await store.testCodingLlmConnection();
    expect(store.reachability?.ok).toBe(false);
    expect(store.reachability?.detail).toContain('connection refused');
  });

  it('startEngine flips running flag and stopEngine clears it', async () => {
    mockInvoke.mockImplementation(() => Promise.resolve(null));
    const store = useSelfImproveStore();
    await store.startEngine();
    expect(store.running).toBe(true);
    await store.stopEngine();
    expect(store.running).toBe(false);
  });

  it('setAutostart records the new state', async () => {
    mockInvoke.mockImplementation((cmd: string, args: unknown) => {
      if (cmd === 'set_self_improve_autostart') {
        const a = args as { enabled: boolean };
        return Promise.resolve(a.enabled);
      }
      return Promise.resolve(null);
    });
    const store = useSelfImproveStore();
    const result = await store.setAutostart(true);
    expect(result).toBe(true);
    expect(store.autostartEnabled).toBe(true);
  });

  it('loadMetrics populates the metrics summary', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_self_improve_metrics') {
        return Promise.resolve({
          total_runs: 10,
          successes: 8,
          failures: 2,
          success_rate: 0.8,
          failure_rate: 0.2,
          avg_duration_ms: 1234,
          last_error: 'rate limit',
          last_error_chunk: '25.4',
          last_error_at_ms: 100,
        });
      }
      return Promise.resolve(null);
    });
    const store = useSelfImproveStore();
    await store.loadMetrics();
    expect(store.metrics.total_runs).toBe(10);
    expect(store.metrics.success_rate).toBe(0.8);
    expect(store.metrics.last_error).toBe('rate limit');
  });

  it('loadRuns hydrates the runs list', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_self_improve_runs') {
        return Promise.resolve([
          {
            started_at_ms: 1000,
            finished_at_ms: 1500,
            chunk_id: '25.4',
            chunk_title: 'Engine MVP',
            outcome: 'success',
            duration_ms: 500,
            provider: 'ollama',
            model: 'gemma3:4b',
            plan_chars: 200,
            error: null,
          },
        ]);
      }
      return Promise.resolve(null);
    });
    const store = useSelfImproveStore();
    await store.loadRuns();
    expect(store.runs.length).toBe(1);
    expect(store.runs[0].chunk_id).toBe('25.4');
  });

  it('clearRunLog wipes runs and refreshes metrics', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'clear_self_improve_log') {
        return Promise.resolve({
          total_runs: 0,
          successes: 0,
          failures: 0,
          success_rate: 0,
          failure_rate: 0,
          avg_duration_ms: 0,
          last_error: null,
          last_error_chunk: null,
          last_error_at_ms: 0,
        });
      }
      return Promise.resolve(null);
    });
    const store = useSelfImproveStore();
    // Pre-populate runs.
    store.runs.push({
      started_at_ms: 1, finished_at_ms: 1, chunk_id: 'x', chunk_title: 't',
      outcome: 'success', duration_ms: 1, provider: 'p', model: 'm',
      plan_chars: 10, error: null,
    });
    await store.clearRunLog();
    expect(store.runs.length).toBe(0);
    expect(store.metrics.total_runs).toBe(0);
  });
});
