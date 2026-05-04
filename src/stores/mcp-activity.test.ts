import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useMcpActivityStore, type McpActivitySnapshot } from './mcp-activity';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  unlisten: vi.fn(),
  eventHandler: undefined as ((event: { payload: McpActivitySnapshot }) => void) | undefined,
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

describe('mcp activity store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mocks.invoke.mockReset();
    mocks.listen.mockReset();
    mocks.unlisten.mockReset();
    mocks.eventHandler = undefined;
  });

  it('loads the latest backend snapshot and exposes concise labels', async () => {
    mocks.invoke.mockResolvedValueOnce(snapshot());

    const store = useMcpActivityStore();
    await store.loadSnapshot();

    expect(mocks.invoke).toHaveBeenCalledWith('get_mcp_activity');
    expect(store.statusLabel).toBe('Working');
    expect(store.modelLabel).toBe('Free Api - pollinations-openai');
    expect(store.workLabel).toBe('Search memory');
    expect(store.speechText).toBe('Using Pollinations, I am searching memory.');
  });

  it('listens for mcp-activity events and disposes the listener', async () => {
    mocks.invoke.mockResolvedValueOnce(snapshot({ status: 'idle', speak: false }));

    const store = useMcpActivityStore();
    await store.initialise();

    expect(mocks.listen).toHaveBeenCalledWith('mcp-activity', expect.any(Function));
    expect(store.isListening).toBe(true);

    mocks.eventHandler?.({
      payload: snapshot({
        status: 'success',
        phase: 'ready',
        message: 'MCP brain is ready on port 7423 using Pollinations.',
        toolName: null,
        toolTitle: null,
        updatedAtMs: 456,
      }),
    });

    expect(store.statusLabel).toBe('Ready');
    expect(store.snapshot.phase).toBe('ready');
    expect(store.speechText).toBe('MCP brain is ready on port 7423 using Pollinations.');

    store.dispose();
    expect(mocks.unlisten).toHaveBeenCalledOnce();
    expect(store.isListening).toBe(false);
  });
});