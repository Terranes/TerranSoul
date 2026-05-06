import { describe, expect, it, beforeEach, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

import WikiPanel from './WikiPanel.vue';
import type { MemoryEntry, MemoryEdge } from '../types';

function memory(id: number, content: string): MemoryEntry {
  return {
    id,
    content,
    tags: 'test',
    importance: 3,
    memory_type: 'fact',
    created_at: 1,
    last_accessed: null,
    access_count: 0,
    tier: 'long',
    decay_score: 1,
    session_id: null,
    parent_id: null,
    token_count: 4,
  };
}

function edge(id: number): MemoryEdge {
  return {
    id,
    src_id: 1,
    dst_id: 2,
    rel_type: 'related_to',
    confidence: 0.92,
    source: 'llm',
    created_at: 1,
    valid_from: null,
    valid_to: null,
    edge_source: null,
  };
}

describe('WikiPanel', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockInvoke.mockImplementation((command: string) => {
      switch (command) {
        case 'brain_wiki_audit':
          return Promise.resolve({
            open_conflicts: [{}],
            orphan_ids: [3, 4],
            stale_ids: [5],
            pending_embeddings: 6,
            total_memories: 42,
            total_edges: 17,
            generated_at: 1,
          });
        case 'brain_wiki_spotlight':
          return Promise.resolve([{ entry: memory(1, 'Central memory'), degree: 9 }]);
        case 'brain_wiki_serendipity':
          return Promise.resolve([{ edge: edge(7), src: memory(1, 'First cluster'), dst: memory(2, 'Second cluster'), label: 'inferred_strong' }]);
        case 'brain_wiki_revisit':
          return Promise.resolve([{ entry: memory(8, 'Old review candidate'), gravity: 0.42 }]);
        default:
          return Promise.reject(new Error(`unexpected command ${command}`));
      }
    });
  });

  it('loads audit metrics on mount', async () => {
    const wrapper = mount(WikiPanel);
    await flushPromises();

    expect(wrapper.find('[data-testid="wiki-panel"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('42');
    expect(wrapper.text()).toContain('17');
    expect(wrapper.text()).toContain('Embedding queue');
    expect(mockInvoke).toHaveBeenCalledWith('brain_wiki_audit', { limit: 50 });
  });

  it('switches to spotlight results', async () => {
    const wrapper = mount(WikiPanel);
    await flushPromises();

    await wrapper.findAll('.wiki-panel__tab')[1].trigger('click');
    expect(wrapper.find('[data-testid="wiki-panel-spotlight"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('Central memory');
    expect(wrapper.text()).toContain('9 edges');
  });

  it('renders serendipity and revisit tabs', async () => {
    const wrapper = mount(WikiPanel);
    await flushPromises();

    await wrapper.findAll('.wiki-panel__tab')[2].trigger('click');
    expect(wrapper.text()).toContain('First cluster');
    expect(wrapper.text()).toContain('inferred_strong');

    await wrapper.findAll('.wiki-panel__tab')[3].trigger('click');
    expect(wrapper.text()).toContain('Old review candidate');
    expect(wrapper.text()).toContain('gravity 0.42');
  });

  it('surfaces refresh errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('boom'));
    const wrapper = mount(WikiPanel);
    await flushPromises();

    expect(wrapper.find('[data-testid="wiki-panel-error"]').text()).toContain('boom');
  });
});
