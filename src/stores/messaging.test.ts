/**
 * Integration tests for the messaging store.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useMessagingStore } from './messaging';
import type { AgentMessageInfo } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const sampleMessage: AgentMessageInfo = {
  id: 'msg-001',
  sender: 'agent-a',
  topic: 'events',
  payload: { action: 'hello' },
  timestamp: Date.now(),
};

describe('messaging store — IPC integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('publish sends correct IPC command', async () => {
    mockInvoke.mockResolvedValue(sampleMessage);
    const store = useMessagingStore();
    const result = await store.publish('agent-a', 'events', { action: 'hello' });
    expect(mockInvoke).toHaveBeenCalledWith('publish_agent_message', {
      sender: 'agent-a',
      topic: 'events',
      payload: { action: 'hello' },
    });
    expect(result).toEqual(sampleMessage);
    expect(store.error).toBeNull();
  });

  it('publish sets error on failure', async () => {
    mockInvoke.mockRejectedValue(new Error('bus error'));
    const store = useMessagingStore();
    const result = await store.publish('agent-a', 'events', {});
    expect(result).toBeNull();
    expect(store.error).toBe('Error: bus error');
  });

  it('subscribe sends correct IPC command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useMessagingStore();
    const result = await store.subscribe('agent-b', 'events');
    expect(mockInvoke).toHaveBeenCalledWith('subscribe_agent_topic', {
      agentName: 'agent-b',
      topic: 'events',
    });
    expect(result).toBe(true);
    expect(store.error).toBeNull();
  });

  it('subscribe sets error on failure', async () => {
    mockInvoke.mockRejectedValue(new Error('sub error'));
    const store = useMessagingStore();
    const result = await store.subscribe('agent-b', 'events');
    expect(result).toBe(false);
    expect(store.error).toBe('Error: sub error');
  });

  it('unsubscribe sends correct IPC command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useMessagingStore();
    const result = await store.unsubscribe('agent-b', 'events');
    expect(mockInvoke).toHaveBeenCalledWith('unsubscribe_agent_topic', {
      agentName: 'agent-b',
      topic: 'events',
    });
    expect(result).toBe(true);
  });

  it('unsubscribe sets error on failure', async () => {
    mockInvoke.mockRejectedValue(new Error('unsub error'));
    const store = useMessagingStore();
    const result = await store.unsubscribe('agent-b', 'events');
    expect(result).toBe(false);
    expect(store.error).toBe('Error: unsub error');
  });

  it('getMessages fetches and stores messages', async () => {
    mockInvoke.mockResolvedValue([sampleMessage]);
    const store = useMessagingStore();
    const msgs = await store.getMessages('agent-b');
    expect(mockInvoke).toHaveBeenCalledWith('get_agent_messages', { agentName: 'agent-b' });
    expect(msgs).toEqual([sampleMessage]);
    expect(store.messages).toEqual([sampleMessage]);
    expect(store.isLoading).toBe(false);
  });

  it('getMessages sets error on failure', async () => {
    mockInvoke.mockRejectedValue(new Error('fetch error'));
    const store = useMessagingStore();
    const msgs = await store.getMessages('agent-b');
    expect(msgs).toEqual([]);
    expect(store.error).toBe('Error: fetch error');
  });

  it('listSubscriptions fetches subscriptions', async () => {
    mockInvoke.mockResolvedValue(['topic1', 'topic2']);
    const store = useMessagingStore();
    const subs = await store.listSubscriptions('agent-a');
    expect(mockInvoke).toHaveBeenCalledWith('list_agent_subscriptions', { agentName: 'agent-a' });
    expect(subs).toEqual(['topic1', 'topic2']);
    expect(store.subscriptions).toEqual(['topic1', 'topic2']);
  });

  it('listSubscriptions sets error on failure', async () => {
    mockInvoke.mockRejectedValue(new Error('list error'));
    const store = useMessagingStore();
    const subs = await store.listSubscriptions('agent-a');
    expect(subs).toEqual([]);
    expect(store.error).toBe('Error: list error');
  });

  it('clearError resets error', async () => {
    mockInvoke.mockRejectedValue(new Error('test error'));
    const store = useMessagingStore();
    await store.publish('a', 'b', {});
    expect(store.error).toBeTruthy();
    store.clearError();
    expect(store.error).toBeNull();
  });
});

// ── IPC Contract Tests ─────────────────────────────────────────────────────

describe('messaging store — IPC contract', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('subscribe sends agentName (camelCase)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useMessagingStore();
    await store.subscribe('my-agent', 'events');
    expect(mockInvoke).toHaveBeenCalledWith('subscribe_agent_topic', { agentName: 'my-agent', topic: 'events' });
  });

  it('unsubscribe sends agentName (camelCase)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useMessagingStore();
    await store.unsubscribe('my-agent', 'events');
    expect(mockInvoke).toHaveBeenCalledWith('unsubscribe_agent_topic', { agentName: 'my-agent', topic: 'events' });
  });

  it('getMessages sends agentName (camelCase)', async () => {
    mockInvoke.mockResolvedValue([]);
    const store = useMessagingStore();
    await store.getMessages('my-agent');
    expect(mockInvoke).toHaveBeenCalledWith('get_agent_messages', { agentName: 'my-agent' });
  });

  it('listSubscriptions sends agentName (camelCase)', async () => {
    mockInvoke.mockResolvedValue([]);
    const store = useMessagingStore();
    await store.listSubscriptions('my-agent');
    expect(mockInvoke).toHaveBeenCalledWith('list_agent_subscriptions', { agentName: 'my-agent' });
  });
});
