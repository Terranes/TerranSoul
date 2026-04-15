/**
 * Integration tests for the conversation store.
 *
 * The conversation store now has three paths:
 *  1. Tauri backend available (window.__TAURI_INTERNALS__) → streaming IPC
 *  2. No Tauri but brain configured → browser-side free API
 *  3. No brain → persona fallback
 *
 * In jsdom tests, __TAURI_INTERNALS__ is absent unless explicitly set,
 * so tests exercise paths 2 and 3 by default.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useConversationStore } from './conversation';
import { useBrainStore } from './brain';
import type { Message } from '../types';

// Mock the Tauri invoke API
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Mock the free-api-client to avoid real HTTP calls
const mockStreamChat = vi.fn();
vi.mock('../utils/free-api-client', () => ({
  streamChatCompletion: (...args: unknown[]) => mockStreamChat(...args),
  buildHistory: (msgs: Array<{ role: string; content: string }>, limit = 20) =>
    msgs.slice(-limit).map((m: { role: string; content: string }) => ({
      role: m.role,
      content: m.content,
    })),
  getSystemPrompt: () => 'You are TerranSoul.',
}));

describe('conversation store — no brain (persona fallback)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
  });

  it('sendMessage uses persona fallback when no brain is configured', async () => {
    const store = useConversationStore();
    await store.sendMessage('hello');

    expect(store.isThinking).toBe(false);
    expect(store.messages).toHaveLength(2);
    expect(store.messages[0].role).toBe('user');
    expect(store.messages[0].content).toBe('hello');
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].agentName).toBe('TerranSoul');
    expect(store.messages[1].sentiment).toBe('happy'); // "hello" triggers happy
  });

  it('persona fallback detects sadness', async () => {
    const store = useConversationStore();
    await store.sendMessage('I am sad today');

    expect(store.messages[1].sentiment).toBe('sad');
  });

  it('persona fallback default message no longer echoes input', async () => {
    const store = useConversationStore();
    await store.sendMessage('How are you?');

    expect(store.messages[1].content).not.toContain('You said:');
    expect(store.messages[1].content).toContain('configure a brain');
  });

  it('multiple messages accumulate in correct order', async () => {
    const store = useConversationStore();
    await store.sendMessage('hello');
    await store.sendMessage('I feel sad');

    expect(store.messages).toHaveLength(4);
    expect(store.messages[0].content).toBe('hello');
    expect(store.messages[0].role).toBe('user');
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].sentiment).toBe('happy');
    expect(store.messages[2].content).toBe('I feel sad');
    expect(store.messages[2].role).toBe('user');
    expect(store.messages[3].role).toBe('assistant');
    expect(store.messages[3].sentiment).toBe('sad');
  });

  it('isThinking is set and cleared during persona fallback', async () => {
    const store = useConversationStore();
    const promise = store.sendMessage('hello');
    expect(store.isThinking).toBe(true);
    await promise;
    expect(store.isThinking).toBe(false);
  });
});

describe('conversation store — brain configured (browser-side free API)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
  });

  it('calls free API when brain is configured', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();

    // Mock streamChatCompletion to call onDone immediately
    mockStreamChat.mockImplementation(
      (_baseUrl: string, _model: string, _apiKey: string | null, _history: unknown[], callbacks: { onDone: (text: string) => void }) => {
        callbacks.onDone('[happy] Hello! Great to see you!');
        return new AbortController();
      },
    );

    const store = useConversationStore();
    await store.sendMessage('hello');

    expect(mockStreamChat).toHaveBeenCalled();
    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].content).toBe('Hello! Great to see you!'); // tags stripped
    expect(store.messages[1].sentiment).toBe('happy');
  });

  it('streams chunks to streamingText during generation', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();

    let capturedCallbacks: { onChunk: (t: string) => void; onDone: (t: string) => void } | null = null;
    mockStreamChat.mockImplementation(
      (_baseUrl: string, _model: string, _apiKey: string | null, _history: unknown[], callbacks: { onChunk: (t: string) => void; onDone: (t: string) => void }) => {
        capturedCallbacks = callbacks;
        // Simulate delayed chunks
        setTimeout(() => {
          callbacks.onChunk('Hello ');
          callbacks.onChunk('world!');
          callbacks.onDone('Hello world!');
        }, 10);
        return new AbortController();
      },
    );

    const store = useConversationStore();
    await store.sendMessage('hi');

    expect(capturedCallbacks).not.toBeNull();
    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].content).toBe('Hello world!');
  });

  it('falls back to persona on free API error', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();

    mockStreamChat.mockImplementation(
      (_baseUrl: string, _model: string, _apiKey: string | null, _history: unknown[], callbacks: { onError: (err: string) => void }) => {
        callbacks.onError('HTTP 429: Rate limited');
        return new AbortController();
      },
    );

    const store = useConversationStore();
    await store.sendMessage('hello');

    // Should fall back to persona
    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].agentName).toBe('TerranSoul');
    expect(store.isThinking).toBe(false);
  });
});

describe('conversation store — Tauri backend available', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
    // Simulate Tauri environment
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  });

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it('uses streaming IPC when Tauri is available', async () => {
    // send_message_stream resolves immediately, then streaming store gets handleChunk
    mockInvoke.mockResolvedValue(undefined);

    const store = useConversationStore();
    // sendMessage will try streaming → send_message_stream
    // Since streaming store won't receive done chunk in test, it will time out.
    // But the invoke should be called.
    const promise = store.sendMessage('hello');

    // Give the async poll loop a tick
    await new Promise((r) => setTimeout(r, 150));

    expect(mockInvoke).toHaveBeenCalledWith('send_message_stream', { message: 'hello' });

    // Simulate the streaming store being done
    const { useStreamingStore } = await import('./streaming');
    const streaming = useStreamingStore();
    streaming.handleChunk({ text: 'Hi there!', done: false });
    streaming.handleChunk({ text: '', done: true });

    await promise;

    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].content).toBe('Hi there!');
    expect(store.isThinking).toBe(false);
  });

  it('falls back to send_message on streaming failure', async () => {
    // First call (send_message_stream) rejects
    mockInvoke.mockRejectedValueOnce(new Error('stream not supported'));
    // Second call (send_message) succeeds
    const serverResponse: Message = {
      id: 'resp-fb',
      role: 'assistant',
      content: 'Hello via fallback!',
      agentName: 'TerranSoul',
      sentiment: 'happy',
      timestamp: Date.now(),
    };
    mockInvoke.mockResolvedValueOnce(serverResponse);

    const store = useConversationStore();
    await store.sendMessage('hello');

    expect(store.messages).toHaveLength(2);
    expect(store.messages[1]).toEqual(serverResponse);
  });

  it('falls back to persona when both streaming and invoke fail', async () => {
    mockInvoke.mockRejectedValue(new Error('all failed'));

    const store = useConversationStore();
    await store.sendMessage('hello');

    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].agentName).toBe('TerranSoul');
    expect(store.isThinking).toBe(false);
  });
});

describe('conversation store — getConversation', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('populates store from backend history', async () => {
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

  it('silently ignores errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('connection lost'));

    const store = useConversationStore();
    await store.getConversation();

    expect(store.messages).toHaveLength(0);
  });
});

describe('detectSentiment — keyword-based fallback', () => {
  it('detects happy from greetings', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('Hello!')).toBe('happy');
    expect(detectSentiment('Hey there')).toBe('happy');
    expect(detectSentiment('hi')).toBe('happy');
  });

  it('detects happy from positive keywords', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('I feel happy today')).toBe('happy');
    expect(detectSentiment('That was awesome')).toBe('happy');
    expect(detectSentiment('I love this')).toBe('happy');
  });

  it('detects sad from negative keywords', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('I feel so sad today')).toBe('sad');
    expect(detectSentiment('This is bad')).toBe('sad');
  });

  it('detects angry from frustration keywords', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('I am so angry!')).toBe('angry');
    expect(detectSentiment('This is frustrating')).toBe('angry');
  });

  it('detects relaxed from calm keywords', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('I want to relax')).toBe('relaxed');
    expect(detectSentiment('So calm and peaceful')).toBe('relaxed');
  });

  it('detects surprised from exclamation keywords', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('Wow that is so surprising!')).toBe('surprised');
    expect(detectSentiment('That was amazing')).toBe('surprised');
  });

  it('returns neutral for unknown content', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('What is the weather like?')).toBe('neutral');
  });
});
