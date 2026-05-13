/**
 * Integration tests for BrainView.vue — the unified Brain hub.
 *
 * Mocks @tauri-apps/api/core invoke() and lets the real brain & memory Pinia
 * stores wire themselves up so we exercise the full data path.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { setActivePinia, createPinia } from 'pinia';
import BrainView from './BrainView.vue';
import type { MemoryEntry } from '../types';

const mockInvoke = vi.fn();
const mockListen = vi.fn((..._args: unknown[]) => Promise.resolve(() => {}));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

// Cytoscape needs some DOM APIs that JSDOM only partially provides — and it's
// orthogonal to what BrainView is testing — so stub MemoryGraph out. (BrainView
// itself no longer renders MemoryGraph; the stub is kept defensively in case
// any nested component pulls it in.)
vi.mock('../components/MemoryGraph.vue', () => ({
  default: { name: 'MemoryGraph', template: '<div data-testid="memory-graph-stub" />' },
}));

const sampleMemories: MemoryEntry[] = [
  {
    id: 1, content: 'User prefers dark mode', tags: '', memory_type: 'preference',
    tier: 'long', importance: 5, decay_score: 1.0, access_count: 3,
    created_at: Date.now(), last_accessed: Date.now(), token_count: 5,
    session_id: null, parent_id: null, confidence: 1.0,
  },
  {
    id: 2, content: 'Yesterday we shipped the v1.0 release', tags: '', memory_type: 'fact',
    tier: 'working', importance: 3, decay_score: 0.8, access_count: 1,
    created_at: Date.now(), last_accessed: Date.now(), token_count: 7,
    session_id: null, parent_id: null, confidence: 1.0,
  },
  {
    id: 3, content: 'How to deploy: 1. build 2. tag 3. push', tags: '', memory_type: 'fact',
    tier: 'long', importance: 4, decay_score: 0.9, access_count: 5,
    created_at: Date.now(), last_accessed: Date.now(), token_count: 10,
    session_id: null, parent_id: null, confidence: 1.0,
  },
];

function makeInvokeMock(opts: {
  brainMode?: unknown;
  memories?: MemoryEntry[];
  lmStudioRunning?: boolean;
  lmStudioModels?: unknown[];
} = {}) {
  const memories = opts.memories ?? sampleMemories;
  const lmStudioModels = opts.lmStudioModels ?? [];
  return (cmd: string) => {
    switch (cmd) {
      case 'get_brain_mode': return Promise.resolve(opts.brainMode ?? null);
      case 'list_free_providers': return Promise.resolve([
        { id: 'pollinations', display_name: 'Pollinations AI', base_url: 'https://gen.pollinations.ai',
          model: 'llama', rpm_limit: 30, rpd_limit: 0, requires_api_key: true, notes: '' },
      ]);
      case 'get_system_info': return Promise.resolve({
        os_name: 'Linux', arch: 'x86_64', cpu_name: 'Test CPU', cpu_cores: 8,
        total_ram_mb: 16384, ram_tier_label: 'High', gpu_name: '',
      });
      case 'check_ollama_status': return Promise.resolve({ running: false, model_count: 0 });
      case 'get_ollama_models': return Promise.resolve([]);
      case 'check_lm_studio_status': return Promise.resolve({
        running: opts.lmStudioRunning ?? false,
        model_count: lmStudioModels.length,
        loaded_count: lmStudioModels.filter((m: any) => m.loaded_instances?.length).length,
      });
      case 'get_lm_studio_models': return Promise.resolve(lmStudioModels);
      case 'set_brain_mode': return Promise.resolve(undefined);
      case 'get_active_brain': return Promise.resolve(null);
      case 'recommend_brain_models': return Promise.resolve([]);
      case 'get_memories': return Promise.resolve(memories);
      case 'list_memories': return Promise.resolve(memories);
      case 'get_memory_stats': return Promise.resolve({
        total: memories.length,
        short_count: memories.filter((m) => m.tier === 'short').length,
        working_count: memories.filter((m) => m.tier === 'working').length,
        long_count: memories.filter((m) => m.tier === 'long').length,
        total_tokens: memories.reduce((s, m) => s + m.token_count, 0),
        avg_decay: memories.reduce((s, m) => s + m.decay_score, 0) / Math.max(1, memories.length),
      });
      case 'list_memory_edges': return Promise.resolve([]);
      case 'get_edge_stats': return Promise.resolve({
        total_edges: 0, by_rel_type: [], by_source: [], connected_memories: 0,
      });
      case 'get_short_term_memory': return Promise.resolve([]);
      case 'get_app_settings': return Promise.resolve({
        version: 2, selected_model_id: 'shinra', camera_azimuth: 0,
        camera_distance: 2.8, bgm_enabled: false, bgm_volume: 0.15,
        bgm_track_id: 'prelude', bgm_custom_tracks: [], auto_tag: false,
        contextual_retrieval: false, late_chunking: false, lan_enabled: false,
        lan_auth_mode: 'token_required',
      });
      default:
        // Other unrelated commands — return a reasonable default.
        return Promise.resolve(null);
    }
  };
}

beforeEach(() => {
  setActivePinia(createPinia());
  mockInvoke.mockReset();
  mockListen.mockClear();
});

describe('BrainView', () => {
  it('renders the hero, mode switcher, all data cards, and stat sheet', async () => {
    mockInvoke.mockImplementation(makeInvokeMock());
    const w = mount(BrainView);
    await flushPromises();

    expect(w.find('[data-testid="brain-view"]').exists()).toBe(true);
    expect(w.find('[data-testid="brain-avatar"]').exists()).toBe(true);
    expect(w.find('[data-testid="bv-mode-switcher"]').exists()).toBe(true);
    expect(w.find('[data-testid="bv-card-config"]').exists()).toBe(true);
    expect(w.find('[data-testid="bv-card-hardware"]').exists()).toBe(true);
    expect(w.find('[data-testid="bv-card-memory"]').exists()).toBe(true);
    expect(w.find('[data-testid="bv-cognitive-breakdown"]').exists()).toBe(true);
    expect(w.find('[data-testid="bv-rag-capability"]').exists()).toBe(true);
    expect(w.find('[data-testid="bv-lan-share-section"]').exists()).toBe(true);
    expect(w.find('[data-testid="brain-stat-sheet"]').exists()).toBe(true);
    // Memory graph section has been moved to MemoryView; BrainView no longer
    // renders MemoryGraph.
    expect(w.find('[data-testid="memory-graph-stub"]').exists()).toBe(false);
  });

  it('persists the LAN brain sharing opt-in toggle', async () => {
    mockInvoke.mockImplementation(makeInvokeMock());
    const w = mount(BrainView);
    await flushPromises();

    await w.find('[data-testid="bv-lan-enabled-toggle"]').setValue(true);

    expect(mockInvoke).toHaveBeenCalledWith('save_app_settings', {
      settings: expect.objectContaining({ lan_enabled: true }),
    });
  });

  it('shows "No brain configured" when brainMode is null', async () => {
    mockInvoke.mockImplementation(makeInvokeMock({ brainMode: null }));
    const w = mount(BrainView);
    await flushPromises();

    expect(w.text()).toContain('No brain configured');
    // Avatar should be in no-brain state.
    expect(w.find('[data-testid="brain-avatar"]').classes()).toContain('mood-none');
  });

  it('shows "Free cloud" pill when brainMode is free_api', async () => {
    mockInvoke.mockImplementation(makeInvokeMock({
      brainMode: { mode: 'free_api', provider_id: 'pollinations', api_key: null },
    }));
    const w = mount(BrainView);
    await flushPromises();

    expect(w.text()).toContain('Free cloud');
    expect(w.find('[data-testid="brain-avatar"]').classes()).toContain('mood-free');
  });

  it('classifies memories into the four cognitive kinds', async () => {
    mockInvoke.mockImplementation(makeInvokeMock({
      brainMode: { mode: 'free_api', provider_id: 'pollinations', api_key: null },
      memories: [
        ...sampleMemories,
        {
          id: 4, content: 'Prefer minimal patches before broad refactors', tags: 'judgment', memory_type: 'fact',
          tier: 'long', importance: 4, decay_score: 0.95, access_count: 2,
          created_at: Date.now(), last_accessed: Date.now(), token_count: 8,
          session_id: null, parent_id: null, confidence: 1.0,
        },
      ],
    }));
    const w = mount(BrainView);
    await flushPromises();

    // Sample contains 1 preference (semantic), 1 episodic ("Yesterday…"),
    // 1 procedural (numbered "How to deploy"), and 1 judgment tag.
    expect(w.find('[data-testid="bv-cog-episodic"]').text()).toContain('1');
    expect(w.find('[data-testid="bv-cog-semantic"]').text()).toContain('1');
    expect(w.find('[data-testid="bv-cog-procedural"]').text()).toContain('1');
    expect(w.find('[data-testid="bv-cog-judgment"]').text()).toContain('1');
  });

  it('marks vector RAG signal off in cloud modes and on in local mode', async () => {
    // Free cloud → vector OFF, others ON.
    mockInvoke.mockImplementation(makeInvokeMock({
      brainMode: { mode: 'free_api', provider_id: 'pollinations', api_key: null },
    }));
    const wFree = mount(BrainView);
    await flushPromises();
    const ragGrid = wFree.find('[data-testid="bv-rag-capability"]');
    const cells = ragGrid.findAll('.bv-rag-cell');
    const vector = cells.find((c) => c.text().includes('Vector'));
    expect(vector?.classes()).toContain('is-off');
    const keyword = cells.find((c) => c.text().includes('Keyword'));
    expect(keyword?.classes()).toContain('is-on');

    // Local Ollama → all ON.
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockImplementation(makeInvokeMock({
      brainMode: { mode: 'local_ollama', model: 'gemma3:1b' },
    }));
    const wLocal = mount(BrainView);
    await flushPromises();
    const localCells = wLocal.find('[data-testid="bv-rag-capability"]').findAll('.bv-rag-cell');
    const localVector = localCells.find((c) => c.text().includes('Vector'));
    expect(localVector?.classes()).toContain('is-on');
  });

  it('emits navigate("memory") when the memory-card link is clicked', async () => {
    mockInvoke.mockImplementation(makeInvokeMock({
      brainMode: { mode: 'free_api', provider_id: 'pollinations', api_key: null },
    }));
    const w = mount(BrainView);
    await flushPromises();
    const link = w.find('[data-testid="bv-card-memory"] .bv-card-link');
    await link.trigger('click');
    expect(w.emitted('navigate')?.[0]).toEqual(['memory']);
  });

  it('emits navigate("brain-setup") from the hero "Brain setup" button', async () => {
    mockInvoke.mockImplementation(makeInvokeMock());
    const w = mount(BrainView);
    await flushPromises();
    const buttons = w.findAll('button');
    const setupBtn = buttons.find((b) => b.text().includes('Brain setup'));
    await setupBtn?.trigger('click');
    expect(w.emitted('navigate')?.[0]).toEqual(['brain-setup']);
  });

  it('routes LM Studio availability through the Local LLM quick mode', async () => {
    mockInvoke.mockImplementation(makeInvokeMock({
      lmStudioRunning: true,
      lmStudioModels: [
        {
          key: 'text-embedding-nomic',
          display_name: 'Nomic Embed',
          type: 'embedding',
          size_bytes: 100,
          loaded_instances: [],
        },
        {
          key: 'google/gemma-4-e4b',
          display_name: 'Gemma 4 E4B',
          type: 'llm',
          size_bytes: 100,
          loaded_instances: [{ id: 'loaded-1' }],
        },
      ],
    }));
    const w = mount(BrainView);
    await flushPromises();

    const cards = w.findAll('.bv-mode-card');
    expect(cards.find((b) => b.text().includes('Local LM Studio'))).toBeUndefined();
    const local = cards.find((b) => b.text().includes('Local LLM'));
    expect(local).toBeTruthy();
    expect(local!.text()).toContain('LM Studio');
    expect(local!.attributes('disabled')).toBeUndefined();

    await local!.trigger('click');
    await flushPromises();

    expect(mockInvoke).not.toHaveBeenCalledWith('set_brain_mode', expect.anything());
    expect(w.emitted('navigate')?.[0]).toEqual(['marketplace']);
  });

  it('shows LM Studio as visibly selected when it is the active brain mode', async () => {
    mockInvoke.mockImplementation(makeInvokeMock({
      brainMode: {
        mode: 'local_lm_studio',
        model: 'google/gemma-4-e4b',
        base_url: 'http://127.0.0.1:1234',
        api_key: null,
        embedding_model: null,
      },
      lmStudioRunning: true,
      lmStudioModels: [
        {
          key: 'google/gemma-4-e4b',
          display_name: 'Gemma 4 E4B',
          type: 'llm',
          size_bytes: 100,
          loaded_instances: [{ id: 'loaded-1' }],
        },
      ],
    }));
    const w = mount(BrainView);
    await flushPromises();

    const cards = w.findAll('.bv-mode-card');
    expect(cards.find((b) => b.text().includes('Local LM Studio'))).toBeUndefined();
    const local = cards.find((b) => b.text().includes('Local LLM'));

    expect(local).toBeTruthy();
    expect(local!.classes()).toContain('active');
    expect(local!.text()).toContain('LM Studio');
  });
});
