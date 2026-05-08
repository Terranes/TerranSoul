import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn(),
}));

import MemoryGapsPanel from './MemoryGapsPanel.vue';
import { invoke } from '@tauri-apps/api/core';

describe('MemoryGapsPanel', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it('shows empty state when no gaps', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const wrapper = mount(MemoryGapsPanel);
    await flushPromises();
    expect(wrapper.find('.empty-state').exists()).toBe(true);
  });

  it('renders gap items when gaps exist', async () => {
    vi.mocked(invoke).mockResolvedValue([
      { id: 1, context_snippet: 'quantum entanglement', session_id: null, ts: Date.now() },
      { id: 2, context_snippet: 'dark matter theory', session_id: 's-1', ts: Date.now() },
    ]);
    const wrapper = mount(MemoryGapsPanel);
    await flushPromises();
    const items = wrapper.findAll('.gap-item');
    expect(items.length).toBe(2);
    expect(items[0].text()).toContain('quantum entanglement');
    expect(items[1].text()).toContain('dark matter theory');
  });

  it('shows gap count in title', async () => {
    vi.mocked(invoke).mockResolvedValue([
      { id: 1, context_snippet: 'test', session_id: null, ts: Date.now() },
    ]);
    const wrapper = mount(MemoryGapsPanel);
    await flushPromises();
    expect(wrapper.find('.gap-count').text()).toBe('(1)');
  });

  it('dismiss removes gap from list', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce([
        { id: 1, context_snippet: 'test gap', session_id: null, ts: Date.now() },
      ])
      .mockResolvedValueOnce({ dismissed: true });
    const wrapper = mount(MemoryGapsPanel);
    await flushPromises();
    expect(wrapper.findAll('.gap-item').length).toBe(1);

    await wrapper.find('.dismiss-btn').trigger('click');
    await flushPromises();
    expect(wrapper.findAll('.gap-item').length).toBe(0);
  });

  it('has refresh button', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const wrapper = mount(MemoryGapsPanel);
    await flushPromises();
    expect(wrapper.find('.refresh-btn').exists()).toBe(true);
  });
});
