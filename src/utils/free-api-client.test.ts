/**
 * Unit tests for the browser-side free API client.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { buildHistory, resolveChatEndpoint } from './free-api-client';

// We test buildHistory (pure function) and streamChatCompletion's structure.
// Full SSE streaming tests are not practical without a mock HTTP server,
// but the logic is exercised via the conversation store integration tests.

describe('free-api-client — buildHistory', () => {
  it('returns empty array for empty input', () => {
    expect(buildHistory([])).toEqual([]);
  });

  it('maps messages to ChatMessage format', () => {
    const msgs = [
      { role: 'user', content: 'Hello' },
      { role: 'assistant', content: 'Hi!' },
    ];
    const result = buildHistory(msgs);
    expect(result).toEqual([
      { role: 'user', content: 'Hello' },
      { role: 'assistant', content: 'Hi!' },
    ]);
  });

  it('limits to last N messages', () => {
    const msgs = Array.from({ length: 30 }, (_, i) => ({
      role: i % 2 === 0 ? 'user' : 'assistant',
      content: `msg-${i}`,
    }));

    const result = buildHistory(msgs, 5);
    expect(result).toHaveLength(5);
    expect(result[0].content).toBe('msg-25');
    expect(result[4].content).toBe('msg-29');
  });

  it('defaults to 20 message limit', () => {
    const msgs = Array.from({ length: 25 }, (_, i) => ({
      role: 'user',
      content: `msg-${i}`,
    }));

    const result = buildHistory(msgs);
    expect(result).toHaveLength(20);
    expect(result[0].content).toBe('msg-5');
  });

  it('returns all messages when under limit', () => {
    const msgs = [
      { role: 'user', content: 'a' },
      { role: 'assistant', content: 'b' },
    ];
    const result = buildHistory(msgs, 20);
    expect(result).toHaveLength(2);
  });
});

describe('free-api-client — streamChatCompletion', () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  it('handles HTTP error responses', async () => {
    // Mock fetch to return a non-ok response
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 429,
      text: () => Promise.resolve('Rate limited'),
    }) as unknown as typeof fetch;

    const { streamChatCompletion } = await import('./free-api-client');

    const onError = vi.fn();
    streamChatCompletion(
      'https://api.test.com',
      'test-model',
      null,
      [{ role: 'user', content: 'hi' }],
      { onChunk: vi.fn(), onDone: vi.fn(), onError },
    );

    // Wait for the async fetch to complete
    await new Promise((r) => setTimeout(r, 50));

    expect(onError).toHaveBeenCalledWith('HTTP 429: Rate limited');

    globalThis.fetch = originalFetch;
  });

  it('returns an AbortController', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      text: () => Promise.resolve('error'),
    }) as unknown as typeof fetch;

    const { streamChatCompletion } = await import('./free-api-client');

    const controller = streamChatCompletion(
      'https://api.test.com',
      'model',
      'key-123',
      [],
      { onChunk: vi.fn(), onDone: vi.fn(), onError: vi.fn() },
    );

    expect(controller).toBeInstanceOf(AbortController);

    globalThis.fetch = originalFetch;
  });

  it('streams Ollama newline-delimited chat responses from /api/chat', async () => {
    const encoder = new TextEncoder();
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(encoder.encode('{"message":{"content":"Hel"},"done":false}\n'));
        controller.enqueue(encoder.encode('{"message":{"content":"lo"},"done":false}\n'));
        controller.enqueue(encoder.encode('{"done":true}\n'));
        controller.close();
      },
    });
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      body: stream,
    }) as unknown as typeof fetch;

    const { streamChatCompletion } = await import('./free-api-client');
    const onChunk = vi.fn();
    const onDone = vi.fn();
    streamChatCompletion(
      'http://localhost:11434',
      'gemma3:1b',
      null,
      [{ role: 'user', content: 'hi' }],
      { onChunk, onDone, onError: vi.fn() },
    );

    await new Promise((r) => setTimeout(r, 50));

    expect(globalThis.fetch).toHaveBeenCalledWith(
      'http://localhost:11434/api/chat',
      expect.objectContaining({
        method: 'POST',
        body: expect.stringContaining('"model":"gemma3:1b"'),
      }),
    );
    expect(onChunk).toHaveBeenNthCalledWith(1, 'Hel');
    expect(onChunk).toHaveBeenNthCalledWith(2, 'lo');
    expect(onDone).toHaveBeenCalledWith('Hello');

    globalThis.fetch = originalFetch;
  });
});

describe('free-api-client — resolveChatEndpoint', () => {
  it('uses Ollama chat endpoint for local Ollama', () => {
    expect(resolveChatEndpoint('http://localhost:11434')).toEqual({
      url: 'http://localhost:11434/api/chat',
      protocol: 'ollama',
    });
  });

  it('uses the Vite proxy for default LM Studio during local development', () => {
    expect(
      resolveChatEndpoint(
        'http://127.0.0.1:1234',
        new URL('http://localhost:1420'),
      ),
    ).toEqual({
      url: '/__lmstudio/v1/chat/completions',
      protocol: 'openai',
    });
  });
});
