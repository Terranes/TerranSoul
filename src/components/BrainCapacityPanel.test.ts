import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { setActivePinia, createPinia } from 'pinia';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn(),
}));

import BrainCapacityPanel from './BrainCapacityPanel.vue';
import { useSettingsStore } from '../stores/settings';
import { useMemoryStore } from '../stores/memory';

describe('BrainCapacityPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders without errors', () => {
    const wrapper = mount(BrainCapacityPanel);
    expect(wrapper.find('[data-testid="brain-capacity-panel"]').exists()).toBe(true);
  });

  it('displays memory tier counts from store', async () => {
    const memory = useMemoryStore();
    memory.stats = { total: 1500, short_count: 100, working_count: 400, long_count: 1000, total_tokens: 50000, avg_decay: 0.85, storage_bytes: 5242880 };

    const wrapper = mount(BrainCapacityPanel);
    expect(wrapper.text()).toContain('100');
    expect(wrapper.text()).toContain('400');
    expect(wrapper.text()).toContain('1K'); // 1000 long-term formatted
    expect(wrapper.text()).toContain('2K'); // 1500 total formatted as 2K (rounded)
  });

  it('shows capacity bar with correct percentage', async () => {
    const memory = useMemoryStore();
    const settings = useSettingsStore();
    memory.stats = { total: 500000, short_count: 0, working_count: 0, long_count: 500000, total_tokens: 0, avg_decay: 0.9, storage_bytes: 0 };
    settings.settings.max_long_term_entries = 1_000_000;

    const wrapper = mount(BrainCapacityPanel);
    expect(wrapper.text()).toContain('50%');
  });

  it('shows storage usage', async () => {
    const memory = useMemoryStore();
    // 10 MB in bytes
    memory.stats = { total: 100, short_count: 0, working_count: 0, long_count: 100, total_tokens: 0, avg_decay: 1, storage_bytes: 10485760 };

    const wrapper = mount(BrainCapacityPanel);
    expect(wrapper.text()).toContain('10 MB used');
  });

  it('renders RAG toggles', () => {
    const wrapper = mount(BrainCapacityPanel);
    expect(wrapper.find('[data-testid="bcp-contextual-retrieval"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="bcp-late-chunking"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="bcp-web-search"]').exists()).toBe(true);
  });

  it('renders maintenance controls when enabled', () => {
    const settings = useSettingsStore();
    settings.settings.background_maintenance_enabled = true;

    const wrapper = mount(BrainCapacityPanel);
    expect(wrapper.find('[data-testid="bcp-maintenance-toggle"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="bcp-maintenance-interval"]').exists()).toBe(true);
  });

  it('hides maintenance details when disabled', async () => {
    const settings = useSettingsStore();
    settings.settings.background_maintenance_enabled = false;

    const wrapper = mount(BrainCapacityPanel);
    expect(wrapper.find('[data-testid="bcp-maintenance-interval"]').exists()).toBe(false);
  });

  it('saves max long-term entries on change', async () => {
    const settings = useSettingsStore();
    const spy = vi.spyOn(settings, 'saveMaxLongTermEntries');
    const wrapper = mount(BrainCapacityPanel);
    const range = wrapper.find('[data-testid="bcp-max-entries-range"]');
    await range.setValue(500000);
    await range.trigger('change');
    expect(spy).toHaveBeenCalledWith(500000);
  });

  it('saves relevance threshold on change', async () => {
    const settings = useSettingsStore();
    const spy = vi.spyOn(settings, 'saveRelevanceThreshold');
    const wrapper = mount(BrainCapacityPanel);
    const range = wrapper.find('[data-testid="bcp-relevance-range"]');
    await range.setValue(0.5);
    await range.trigger('change');
    expect(spy).toHaveBeenCalledWith(0.5);
  });
});
