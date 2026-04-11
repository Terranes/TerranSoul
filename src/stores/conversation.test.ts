/**
 * Integration tests for the conversation store.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC without the Rust backend.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useConversationStore } from './conversation';
import type { Message } from '../types';

// Mock the Tauri invoke API
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('conversation store — IPC integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('sendMessage round-trip: user message added locally, invoke called, response pushed', async () => {
    const serverResponse: Message = {
      id: 'resp-1',
      role: 'assistant',
      content: 'Hello! I am TerranSoul.',
      agentName: 'TerranSoul',
      sentiment: 'happy',
      timestamp: Date.now(),
    };
    mockInvoke.mockResolvedValueOnce(serverResponse);

    const store = useConversationStore();
    await store.sendMessage('hello');

    // invoke was called with the correct command and arguments
    expect(mockInvoke).toHaveBeenCalledWith('send_message', {
      message: 'hello',
      agentId: null,
    });

    // Both user and assistant messages are in the store
    expect(store.messages).toHaveLength(2);
    expect(store.messages[0].role).toBe('user');
    expect(store.messages[0].content).toBe('hello');
    expect(store.messages[1]).toEqual(serverResponse);
  });

  it('sendMessage with custom agent routes agentId in invoke', async () => {
    const serverResponse: Message = {
      id: 'resp-2',
      role: 'assistant',
      content: 'Response from custom agent',
      agentName: 'custom',
      timestamp: Date.now(),
    };
    mockInvoke.mockResolvedValueOnce(serverResponse);

    const store = useConversationStore();
    store.currentAgent = 'my-agent';
    await store.sendMessage('test');

    expect(mockInvoke).toHaveBeenCalledWith('send_message', {
      message: 'test',
      agentId: 'my-agent',
    });
    expect(store.messages[1].agentName).toBe('custom');
  });

  it('sendMessage error: uses persona fallback and clears isThinking', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('Backend unavailable'));

    const store = useConversationStore();
    await store.sendMessage('hello');

    // isThinking should be cleared
    expect(store.isThinking).toBe(false);

    // Two messages: user + persona fallback
    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].agentName).toBe('TerranSoul');
    expect(store.messages[1].sentiment).toBe('happy'); // "hello" triggers happy
    expect(store.messages[1].content).not.toContain('Error:');
  });

  it('isThinking toggles during sendMessage lifecycle', async () => {
    let resolveInvoke: (value: Message) => void;
    const pendingInvoke = new Promise<Message>((resolve) => {
      resolveInvoke = resolve;
    });
    mockInvoke.mockReturnValueOnce(pendingInvoke);

    const store = useConversationStore();
    const sendPromise = store.sendMessage('hi');

    // After calling sendMessage but before invoke resolves, isThinking should be true
    expect(store.isThinking).toBe(true);

    // Resolve the invoke
    resolveInvoke!({
      id: 'resp-3',
      role: 'assistant',
      content: 'Hi there!',
      agentName: 'TerranSoul',
      timestamp: Date.now(),
    });
    await sendPromise;

    // After resolving, isThinking should be false
    expect(store.isThinking).toBe(false);
  });

  it('getConversation populates store from backend history', async () => {
    const history: Message[] = [
      { id: 'h1', role: 'user', content: 'first', timestamp: 1000 },
      { id: 'h2', role: 'assistant', content: 'response 1', agentName: 'TerranSoul', timestamp: 1001 },
      { id: 'h3', role: 'user', content: 'second', timestamp: 2000 },
      { id: 'h4', role: 'assistant', content: 'response 2', agentName: 'TerranSoul', timestamp: 2001 },
    ];
    mockInvoke.mockResolvedValueOnce(history);

    const store = useConversationStore();
    await store.getConversation();

    expect(mockInvoke).toHaveBeenCalledWith('get_conversation');
    expect(store.messages).toEqual(history);
    expect(store.messages).toHaveLength(4);
  });

  it('getConversation silently ignores errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('connection lost'));

    const store = useConversationStore();
    // Should not throw
    await store.getConversation();

    expect(store.messages).toHaveLength(0);
  });

  it('sendMessage sentiment is preserved from backend response', async () => {
    const serverResponse: Message = {
      id: 'resp-s',
      role: 'assistant',
      content: 'That makes me sad to hear.',
      agentName: 'TerranSoul',
      sentiment: 'sad',
      timestamp: Date.now(),
    };
    mockInvoke.mockResolvedValueOnce(serverResponse);

    const store = useConversationStore();
    await store.sendMessage('I am sad');

    expect(store.messages[1].sentiment).toBe('sad');
  });

  it('multiple messages accumulate in correct order', async () => {
    const resp1: Message = {
      id: 'r1',
      role: 'assistant',
      content: 'Hello!',
      agentName: 'TerranSoul',
      sentiment: 'happy',
      timestamp: 1000,
    };
    const resp2: Message = {
      id: 'r2',
      role: 'assistant',
      content: 'I can help with that.',
      agentName: 'TerranSoul',
      sentiment: 'neutral',
      timestamp: 2000,
    };
    mockInvoke.mockResolvedValueOnce(resp1).mockResolvedValueOnce(resp2);

    const store = useConversationStore();
    await store.sendMessage('hello');
    await store.sendMessage('help me');

    expect(store.messages).toHaveLength(4);
    expect(store.messages[0].content).toBe('hello');
    expect(store.messages[0].role).toBe('user');
    expect(store.messages[1].content).toBe('Hello!');
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[2].content).toBe('help me');
    expect(store.messages[2].role).toBe('user');
    expect(store.messages[3].content).toBe('I can help with that.');
    expect(store.messages[3].role).toBe('assistant');
  });
});
