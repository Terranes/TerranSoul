import { describe, it, expect, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import PromptCommandsPanel from './PromptCommandsPanel.vue';
import { usePromptCommandsStore } from '../stores/prompt-commands';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

function mountPanel(preload?: Array<{ name: string; description: string; content: string; source: string; mode: 'all' | 'coding' | 'companion' }>) {
  const pinia = createPinia();
  setActivePinia(pinia);
  if (preload) {
    const store = usePromptCommandsStore();
    store.commands = preload;
    store.loaded = true;
  }
  return mount(PromptCommandsPanel, {
    global: { plugins: [pinia] },
  });
}

describe('PromptCommandsPanel', () => {
  it('renders the panel with title and add button', () => {
    const wrapper = mountPanel();
    expect(wrapper.find('[data-testid="prompt-commands-panel"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="pcp-add-btn"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('Prompt Commands');
  });

  it('shows empty state when no commands loaded', () => {
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain('No prompt commands loaded');
  });

  it('displays loaded commands', () => {
    const wrapper = mountPanel([
      { name: 'hello', description: 'Hello World', content: '# Hello\nWorld', source: '/test/hello.md', mode: 'all' },
      { name: 'review', description: 'Code Review', content: '# Review\nCheck it', source: '/test/review.md', mode: 'all' },
    ]);

    expect(wrapper.find('[data-testid="pcp-item-hello"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="pcp-item-review"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('/hello');
    expect(wrapper.text()).toContain('/review');
  });

  it('opens create form when + New is clicked', async () => {
    const wrapper = mountPanel();
    await wrapper.find('[data-testid="pcp-add-btn"]').trigger('click');
    expect(wrapper.find('[data-testid="pcp-create-form"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="pcp-new-name"]').exists()).toBe(true);
  });

  it('opens editor when edit button is clicked', async () => {
    const wrapper = mountPanel([
      { name: 'test-cmd', description: 'Test', content: '# Test\nContent', source: '/test.md', mode: 'all' },
    ]);

    await wrapper.find('[data-testid="pcp-edit-test-cmd"]').trigger('click');
    expect(wrapper.find('[data-testid="pcp-textarea-test-cmd"]').exists()).toBe(true);
  });

  it('shows delete confirmation when delete is clicked', async () => {
    const wrapper = mountPanel([
      { name: 'doomed', description: 'To be deleted', content: 'bye', source: '/doomed.md', mode: 'all' },
    ]);

    await wrapper.find('[data-testid="pcp-delete-doomed"]').trigger('click');
    expect(wrapper.find('[data-testid="pcp-delete-confirm"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('Delete /doomed');
  });
});
