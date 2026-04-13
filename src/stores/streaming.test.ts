/**
 * Integration tests for the streaming store.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useStreamingStore } from './streaming';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('streaming store — IPC integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('sendStreaming sends correct IPC command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useStreamingStore();
    const result = await store.sendStreaming('Hello');
    expect(mockInvoke).toHaveBeenCalledWith('send_message_stream', { message: 'Hello' });
    expect(result).toBe(true);
    expect(store.isStreaming).toBe(true); // Still streaming until done chunk
  });

  it('sendStreaming sets error on failure', async () => {
    mockInvoke.mockRejectedValue(new Error('stream error'));
    const store = useStreamingStore();
    const result = await store.sendStreaming('Hello');
    expect(result).toBe(false);
    expect(store.error).toBe('Error: stream error');
    expect(store.isStreaming).toBe(false);
  });

  it('handleChunk accumulates text', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleChunk({ text: 'Hello ', done: false });
    // parseTags trims each chunk, but the store concatenates
    expect(store.streamText).toBe('Hello');

    store.handleChunk({ text: 'World!', done: false });
    expect(store.streamText).toBe('HelloWorld!');
  });

  it('handleChunk sets done flag', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleChunk({ text: 'Hello', done: false });
    expect(store.isStreaming).toBe(true);

    store.handleChunk({ text: '', done: true });
    expect(store.isStreaming).toBe(false);
  });

  it('handleChunk extracts emotion tags', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleChunk({ text: '[happy] Great to see you!', done: false });
    expect(store.currentEmotion).toBe('happy');
    expect(store.streamText).toBe('Great to see you!');
  });

  it('handleChunk extracts motion tags', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleChunk({ text: '[motion:wave] Hello!', done: false });
    expect(store.currentMotion).toBe('wave');
    expect(store.streamText).toBe('Hello!');
  });

  it('handleChunk extracts both emotion and motion', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleChunk({ text: '[happy] [motion:nod] Absolutely!', done: false });
    expect(store.currentEmotion).toBe('happy');
    expect(store.currentMotion).toBe('nod');
    expect(store.streamText).toBe('Absolutely!');
  });

  it('latest emotion wins across chunks', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleChunk({ text: '[happy] Hi!', done: false });
    expect(store.currentEmotion).toBe('happy');

    store.handleChunk({ text: ' [sad] Sorry.', done: false });
    expect(store.currentEmotion).toBe('sad');
  });

  it('reset clears all state', () => {
    const store = useStreamingStore();
    store.isStreaming = true;
    store.streamText = 'some text';
    store.streamRawText = '[happy] some text';
    store.currentEmotion = 'happy';
    store.currentMotion = 'wave';
    store.error = 'some error';

    store.reset();

    expect(store.isStreaming).toBe(false);
    expect(store.streamText).toBe('');
    expect(store.streamRawText).toBe('');
    expect(store.currentEmotion).toBeNull();
    expect(store.currentMotion).toBeNull();
    expect(store.error).toBeNull();
  });

  it('sendStreaming resets previous state', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useStreamingStore();
    store.streamText = 'old text';
    store.currentEmotion = 'sad';
    store.error = 'old error';

    await store.sendStreaming('New message');

    expect(store.streamText).toBe('');
    expect(store.currentEmotion).toBeNull();
    expect(store.error).toBeNull();
  });

  it('accumulates raw text with tags', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleChunk({ text: '[happy] Hello', done: false });
    expect(store.streamRawText).toBe('[happy] Hello');
    expect(store.streamText).toBe('Hello');
  });
});
