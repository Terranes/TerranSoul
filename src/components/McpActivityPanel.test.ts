import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import McpActivityPanel from './McpActivityPanel.vue';
import type { McpActivitySnapshot } from '../stores/mcp-activity';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  unlisten: vi.fn(),
  eventHandler: undefined as ((event: { payload: McpActivitySnapshot }) => void) | undefined,
  feedChunk: vi.fn(),
  flush: vi.fn(),
  stop: vi.fn(),
  isSpeaking: { value: false },
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mocks.invoke(...args),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: async (eventName: string, handler: (event: { payload: McpActivitySnapshot }) => void) => {
    mocks.listen(eventName, handler);
    mocks.eventHandler = handler;
    return mocks.unlisten;
  },
}));

vi.mock('../composables/useTtsPlayback', () => ({
  useTtsPlayback: () => ({
    feedChunk: mocks.feedChunk,
    flush: mocks.flush,
    stop: mocks.stop,
    isSpeaking: mocks.isSpeaking,
  }),
}));

function snapshot(overrides: Partial<McpActivitySnapshot> = {}): McpActivitySnapshot {
  return {
    status: 'working',
    phase: 'tool_start',
    message: 'Using Pollinations, I am searching memory.',
    toolName: 'brain_search',
    toolTitle: 'Search memory',
    brainProvider: 'free_api',
    brainModel: 'pollinations-openai',
    updatedAtMs: 123,
    speak: true,
    ...overrides,
  };
}

describe('McpActivityPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mocks.invoke.mockReset();
    mocks.listen.mockReset();
    mocks.unlisten.mockReset();
    mocks.feedChunk.mockReset();
    mocks.flush.mockReset();
    mocks.stop.mockReset();
    mocks.isSpeaking.value = false;
    mocks.eventHandler = undefined;
  });

  it('renders model activity and speaks new spoken snapshots', async () => {
    mocks.invoke.mockResolvedValueOnce(snapshot({ status: 'idle', speak: false }));

    const wrapper = mount(McpActivityPanel);
    await flushPromises();

    mocks.eventHandler?.({
      payload: snapshot({
        message: 'Using Pollinations, I am searching memory for workspace context',
        updatedAtMs: 777,
      }),
    });
    await flushPromises();

    expect(wrapper.text()).toContain('Working');
    expect(wrapper.text()).toContain('Free Api - pollinations-openai');
    expect(wrapper.text()).toContain('Using Pollinations, I am searching memory for workspace context');
    expect(mocks.feedChunk).toHaveBeenCalledWith(
      'Using Pollinations, I am searching memory for workspace context.',
    );
    expect(mocks.flush).toHaveBeenCalledOnce();

    wrapper.unmount();
    expect(mocks.unlisten).toHaveBeenCalledOnce();
  });

  it('collapse toggle hides message and model, expand shows them', async () => {
    mocks.invoke.mockResolvedValueOnce(snapshot({ status: 'idle', speak: false }));

    const wrapper = mount(McpActivityPanel);
    await flushPromises();

    mocks.eventHandler?.({ payload: snapshot({ updatedAtMs: 1, speak: false }) });
    await flushPromises();

    // Initially expanded — message visible
    expect(wrapper.find('.mcp-activity__message').exists()).toBe(true);
    expect(wrapper.find('.mcp-activity__model').exists()).toBe(true);

    // Click toggle to collapse
    const toggle = wrapper.find('[data-testid="mcp-activity-toggle"]');
    expect(toggle.exists()).toBe(true);
    await toggle.trigger('click');

    // Message and model should be hidden
    expect(wrapper.find('.mcp-activity__message').exists()).toBe(false);
    expect(wrapper.find('.mcp-activity__model').exists()).toBe(false);

    // Panel itself still rendered
    expect(wrapper.find('[data-testid="mcp-activity-panel"]').exists()).toBe(true);

    // Click again to expand
    await toggle.trigger('click');
    expect(wrapper.find('.mcp-activity__message').exists()).toBe(true);
    expect(wrapper.find('.mcp-activity__model').exists()).toBe(true);

    wrapper.unmount();
  });
});