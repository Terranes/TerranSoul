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
    // isStreaming starts false — only handleChunk sets it true on first text
    expect(store.isStreaming).toBe(false);
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
    expect(store.streamText).toBe('Hello ');

    store.handleChunk({ text: 'World!', done: false });
    expect(store.streamText).toBe('Hello World!');
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

    // Text from Rust is already clean — emotion comes via handleAnimation
    store.handleChunk({ text: 'Great to see you!', done: false });
    expect(store.streamText).toBe('Great to see you!');
    expect(store.currentEmotion).toBeNull();
  });

  it('handleAnimation sets emotion', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleAnimation({ emotion: 'happy' });
    expect(store.currentEmotion).toBe('happy');
  });

  it('handleAnimation sets motion', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleAnimation({ motion: 'wave' });
    expect(store.currentMotion).toBe('wave');
  });

  it('handleAnimation sets both emotion and motion', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleAnimation({ emotion: 'surprised', motion: 'nod' });
    expect(store.currentEmotion).toBe('surprised');
    expect(store.currentMotion).toBe('nod');
  });

  it('handleAnimation ignores invalid emotions', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleAnimation({ emotion: 'ecstatic' });
    expect(store.currentEmotion).toBeNull();
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
    store.currentMotion = 'wave';
    store.error = 'old error';

    await store.sendStreaming('New message');

    expect(store.streamText).toBe('');
    expect(store.currentEmotion).toBeNull();
    expect(store.currentMotion).toBeNull();
    expect(store.error).toBeNull();
  });

  it('accumulates raw text with tags', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    // Text from Rust is already clean — streamRawText mirrors streamText
    store.handleChunk({ text: 'Hello', done: false });
    expect(store.streamRawText).toBe('Hello');
    expect(store.streamText).toBe('Hello');
  });

  it('handleAnimation updates emotion across multiple calls', () => {
    const store = useStreamingStore();
    store.isStreaming = true;

    store.handleAnimation({ emotion: 'happy' });
    expect(store.currentEmotion).toBe('happy');

    store.handleAnimation({ emotion: 'sad' });
    expect(store.currentEmotion).toBe('sad');
  });

  it('handleAnimation sets intensity when provided', () => {
    const store = useStreamingStore();
    store.handleAnimation({ emotion: 'happy', intensity: 0.6 });
    expect(store.currentEmotion).toBe('happy');
    expect(store.currentEmotionIntensity).toBe(0.6);
  });

  it('handleAnimation defaults intensity to 1 when absent', () => {
    const store = useStreamingStore();
    store.handleAnimation({ emotion: 'angry' });
    expect(store.currentEmotionIntensity).toBe(1);
  });

  it('handleAnimation clamps intensity to [0, 1]', () => {
    const store = useStreamingStore();
    store.handleAnimation({ emotion: 'sad', intensity: 2.5 });
    expect(store.currentEmotionIntensity).toBe(1);
    store.handleAnimation({ emotion: 'sad', intensity: -0.3 });
    expect(store.currentEmotionIntensity).toBe(0);
  });

  // ── Extended thinking / reasoning effort tests ──

  it('handleChunk accumulates thinking text separately', () => {
    const store = useStreamingStore();
    store.handleChunk({ text: 'Let me think...', done: false, thinking: true });
    expect(store.thinkingText).toBe('Let me think...');
    expect(store.streamText).toBe('');
    expect(store.isThinkingPhase).toBe(true);
    expect(store.isStreaming).toBe(true);
  });

  it('handleChunk transitions from thinking to answer phase', () => {
    const store = useStreamingStore();
    store.handleChunk({ text: 'reasoning...', done: false, thinking: true });
    expect(store.isThinkingPhase).toBe(true);

    store.handleChunk({ text: 'The answer is 42.', done: false, thinking: false });
    expect(store.isThinkingPhase).toBe(false);
    expect(store.streamText).toBe('The answer is 42.');
    expect(store.thinkingText).toBe('reasoning...');
  });

  it('handleChunk done clears thinking phase', () => {
    const store = useStreamingStore();
    store.handleChunk({ text: 'thinking...', done: false, thinking: true });
    expect(store.isThinkingPhase).toBe(true);

    store.handleChunk({ text: '', done: true });
    expect(store.isThinkingPhase).toBe(false);
    expect(store.isStreaming).toBe(false);
  });

  it('reset clears thinking state', () => {
    const store = useStreamingStore();
    store.handleChunk({ text: 'deep thoughts', done: false, thinking: true });
    store.handleChunk({ text: 'answer', done: false });

    store.reset();

    expect(store.thinkingText).toBe('');
    expect(store.isThinkingPhase).toBe(false);
    expect(store.streamText).toBe('');
  });

  it('sendStreaming resets thinking state', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useStreamingStore();
    store.thinkingText = 'old thinking';
    store.isThinkingPhase = true;

    await store.sendStreaming('New message');

    expect(store.thinkingText).toBe('');
    expect(store.isThinkingPhase).toBe(false);
  });

  it('handleChunk without thinking field defaults to false', () => {
    const store = useStreamingStore();
    store.handleChunk({ text: 'normal text', done: false });
    expect(store.streamText).toBe('normal text');
    expect(store.thinkingText).toBe('');
    expect(store.isThinkingPhase).toBe(false);
  });
});
