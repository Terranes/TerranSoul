/**
 * Three-Stream Synchronization Tests
 *
 * Validates the contract between TerranSoul's three concurrent streams:
 *   1. Text Stream   — `llm-chunk` events → streaming store → UI text
 *   2. Animation Stream — `llm-animation` events → streaming store → avatar state
 *   3. Voice Stream  — `feedChunk()` → TTS sentence detection → audio → lip sync
 *
 * These tests run WITHOUT a running LLM. They simulate the exact event
 * sequence that Rust emits during a streamed response and verify that
 * the three streams remain synchronised at every stage of the lifecycle:
 *   thinking → streaming/talking → TTS speaking → final emotion → idle
 *
 * See also: rules/three-stream-testing.md for the testing methodology.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useStreamingStore } from '../stores/streaming';
import { useTtsPlayback } from './useTtsPlayback';

// ── Mock Tauri IPC ────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Mock HTMLAudioElement ─────────────────────────────────────────────────────

let audioEndDelay = 0;

class MockAudio {
  src = '';
  muted = false;
  duration = 1.5;
  onended: (() => void) | null = null;
  onerror: (() => void) | null = null;

  play(): Promise<void> {
    if (audioEndDelay > 0) {
      setTimeout(() => this.onended?.(), audioEndDelay);
    } else {
      Promise.resolve().then(() => this.onended?.());
    }
    return Promise.resolve();
  }

  pause() {}
}

vi.stubGlobal('Audio', MockAudio);
vi.stubGlobal('Blob', class MockBlob {
  constructor(public parts: BlobPart[], public options?: BlobPropertyBag) {}
});
vi.stubGlobal('URL', {
  createObjectURL: vi.fn(() => 'blob:mock'),
  revokeObjectURL: vi.fn(),
});

class MockSpeechSynthesisUtterance {
  text: string;
  lang = '';
  pitch = 1.0;
  rate = 1.0;
  voice: SpeechSynthesisVoice | null = null;
  volume = 1;
  onend: (() => void) | null = null;
  onerror: (() => void) | null = null;
  constructor(text: string) { this.text = text; }
}

vi.stubGlobal('SpeechSynthesisUtterance', MockSpeechSynthesisUtterance);
vi.stubGlobal('speechSynthesis', {
  speak: vi.fn((u: MockSpeechSynthesisUtterance) => {
    Promise.resolve().then(() => u.onend?.());
  }),
  getVoices: vi.fn((): SpeechSynthesisVoice[] => []),
  cancel: vi.fn(),
  pause: vi.fn(),
  resume: vi.fn(),
});

// ── Helpers ───────────────────────────────────────────────────────────────────

function stubWavBytes(): number[] {
  const totalDataBytes = 100;
  const buf = new ArrayBuffer(44 + totalDataBytes);
  const view = new DataView(buf);
  view.setUint8(0, 0x52); view.setUint8(1, 0x49); view.setUint8(2, 0x46); view.setUint8(3, 0x46);
  view.setUint32(4, 36 + totalDataBytes, true);
  view.setUint8(8, 0x57); view.setUint8(9, 0x41); view.setUint8(10, 0x56); view.setUint8(11, 0x45);
  view.setUint8(12, 0x66); view.setUint8(13, 0x6d); view.setUint8(14, 0x74); view.setUint8(15, 0x20);
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true); view.setUint16(22, 1, true);
  view.setUint32(24, 16000, true); view.setUint32(28, 32000, true);
  view.setUint16(32, 2, true); view.setUint16(34, 16, true);
  view.setUint8(36, 0x64); view.setUint8(37, 0x61); view.setUint8(38, 0x74); view.setUint8(39, 0x61);
  view.setUint32(40, totalDataBytes, true);
  return Array.from(new Uint8Array(buf));
}

/**
 * Simulate a typical LLM streaming session with interleaved text and
 * animation events. This mirrors what Rust's StreamTagParser emits.
 */
function simulateStreamSession(
  streaming: ReturnType<typeof useStreamingStore>,
  tts: ReturnType<typeof useTtsPlayback>,
  opts?: { feedTts?: boolean },
) {
  const feedTts = opts?.feedTts ?? true;

  // Chunk 1: emotion arrives before text
  streaming.handleAnimation({ emotion: 'happy', intensity: 0.8 });

  // Chunk 2-5: text tokens arrive
  const tokens = ['Hello! ', 'I am ', 'doing ', 'great. '];
  for (const token of tokens) {
    streaming.handleChunk({ text: token, done: false });
    if (feedTts) tts.feedChunk(token);
  }

  // Chunk 6: second emotion mid-stream
  streaming.handleAnimation({ emotion: 'relaxed' });

  // Chunk 7-8: more text
  const moreTokens = ['How are ', 'you?'];
  for (const token of moreTokens) {
    streaming.handleChunk({ text: token, done: false });
    if (feedTts) tts.feedChunk(token);
  }

  // Final: stream done
  streaming.handleChunk({ text: '', done: true });
  if (feedTts) tts.flush();
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('three-stream sync — text + animation coordination', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    audioEndDelay = 0;
  });

  it('text accumulates correctly across all chunks', () => {
    const streaming = useStreamingStore();
    const tts = useTtsPlayback();
    simulateStreamSession(streaming, tts, { feedTts: false });

    expect(streaming.streamText).toBe('Hello! I am doing great. How are you?');
    expect(streaming.isStreaming).toBe(false);
  });

  it('animation events update currentEmotion in real-time', () => {
    const streaming = useStreamingStore();

    streaming.handleAnimation({ emotion: 'happy', intensity: 0.8 });
    expect(streaming.currentEmotion).toBe('happy');
    expect(streaming.currentEmotionIntensity).toBe(0.8);

    streaming.handleAnimation({ emotion: 'relaxed' });
    expect(streaming.currentEmotion).toBe('relaxed');
    expect(streaming.currentEmotionIntensity).toBe(1);
  });

  it('emotion changes do NOT reset accumulated text', () => {
    const streaming = useStreamingStore();

    streaming.handleChunk({ text: 'Hello ', done: false });
    streaming.handleAnimation({ emotion: 'happy' });
    streaming.handleChunk({ text: 'world!', done: false });

    expect(streaming.streamText).toBe('Hello world!');
    expect(streaming.currentEmotion).toBe('happy');
  });

  it('done:true stops streaming even when emotion is set', () => {
    const streaming = useStreamingStore();

    streaming.handleChunk({ text: 'Hi!', done: false });
    streaming.handleAnimation({ emotion: 'happy' });
    streaming.handleChunk({ text: '', done: true });

    expect(streaming.isStreaming).toBe(false);
    // Emotion persists after done for the isSpeaking watcher to use
    expect(streaming.currentEmotion).toBe('happy');
  });

  it('motion events are recorded alongside emotions', () => {
    const streaming = useStreamingStore();

    streaming.handleAnimation({ emotion: 'happy', motion: 'wave' });
    expect(streaming.currentEmotion).toBe('happy');
    expect(streaming.currentMotion).toBe('wave');
  });

  it('isStreaming becomes true on first text chunk only', () => {
    const streaming = useStreamingStore();

    // Empty text doesn't start streaming
    streaming.handleChunk({ text: '', done: false });
    expect(streaming.isStreaming).toBe(false);

    // Animation doesn't start streaming
    streaming.handleAnimation({ emotion: 'happy' });
    expect(streaming.isStreaming).toBe(false);

    // First text starts streaming
    streaming.handleChunk({ text: 'Hi', done: false });
    expect(streaming.isStreaming).toBe(true);
  });
});

describe('three-stream sync — TTS sentence pipeline', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(stubWavBytes());
    audioEndDelay = 0;
  });

  it('TTS synthesizes sentences as they stream', () => {
    const tts = useTtsPlayback();

    // Stream tokens that form a complete sentence
    tts.feedChunk('Hello! ');
    expect(mockInvoke).toHaveBeenCalledWith('synthesize_tts', { text: 'Hello!' });
  });

  it('TTS queues multiple sentences in order', () => {
    const tts = useTtsPlayback();

    tts.feedChunk('Hello! ');
    tts.feedChunk('How are you? ');

    expect(mockInvoke).toHaveBeenCalledTimes(2);
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'synthesize_tts', { text: 'Hello!' });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'synthesize_tts', { text: 'How are you?' });
  });

  it('TTS flush synthesizes remaining text after stream ends', () => {
    const tts = useTtsPlayback();

    tts.feedChunk('Hello! ');
    tts.feedChunk('Final words');
    expect(mockInvoke).toHaveBeenCalledTimes(1); // Only "Hello!"

    tts.flush();
    expect(mockInvoke).toHaveBeenCalledTimes(2);
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'synthesize_tts', { text: 'Final words' });
  });

  it('full session produces correct TTS calls', () => {
    const streaming = useStreamingStore();
    const tts = useTtsPlayback();
    simulateStreamSession(streaming, tts);

    // Expected sentences: "Hello!" + "I am doing great." + "How are you?"
    const ttsCalls = mockInvoke.mock.calls
      .filter((c: unknown[]) => c[0] === 'synthesize_tts')
      .map((c: unknown[]) => (c[1] as { text: string }).text);

    expect(ttsCalls).toContain('Hello!');
    expect(ttsCalls).toContain('I am doing great.');
    // "How are you?" comes via flush (no trailing space in last token)
    expect(ttsCalls).toContain('How are you?');
  });

  it('stop() cancels pending TTS and allows fresh start', () => {
    const tts = useTtsPlayback();

    tts.feedChunk('Hello world');
    tts.stop();
    expect(tts.isSpeaking.value).toBe(false);

    // New session
    mockInvoke.mockClear();
    tts.feedChunk('New session. ');
    expect(mockInvoke).toHaveBeenCalledWith('synthesize_tts', { text: 'New session.' });
  });
});

describe('three-stream sync — lifecycle state machine', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(stubWavBytes());
    audioEndDelay = 0;
  });

  it('streaming state ends before TTS finishes (correct overlap)', async () => {
    audioEndDelay = 40; // TTS takes 40ms per sentence
    const streaming = useStreamingStore();
    const tts = useTtsPlayback();

    // Stream a sentence
    streaming.handleChunk({ text: 'Hello world. ', done: false });
    tts.feedChunk('Hello world. ');
    expect(streaming.isStreaming).toBe(true);

    // Wait for TTS to start
    await new Promise((r) => setTimeout(r, 10));
    expect(tts.isSpeaking.value).toBe(true);

    // End the stream — TTS should still be speaking
    streaming.handleChunk({ text: '', done: true });
    expect(streaming.isStreaming).toBe(false);
    expect(tts.isSpeaking.value).toBe(true);

    // Wait for TTS to finish
    await new Promise((r) => setTimeout(r, 50));
    expect(tts.isSpeaking.value).toBe(false);
  });

  it('emotion persists after streaming ends for TTS watcher', () => {
    const streaming = useStreamingStore();

    streaming.handleAnimation({ emotion: 'happy', intensity: 0.7 });
    streaming.handleChunk({ text: 'Yay!', done: false });
    streaming.handleChunk({ text: '', done: true });

    // Stream is done, but emotion is still available for the isSpeaking watcher
    expect(streaming.isStreaming).toBe(false);
    expect(streaming.currentEmotion).toBe('happy');
    expect(streaming.currentEmotionIntensity).toBe(0.7);
  });

  it('reset() clears everything for a clean next turn', () => {
    const streaming = useStreamingStore();

    streaming.handleChunk({ text: 'Hello', done: false });
    streaming.handleAnimation({ emotion: 'happy', motion: 'wave' });
    streaming.handleChunk({ text: '', done: true });
    streaming.reset();

    expect(streaming.isStreaming).toBe(false);
    expect(streaming.streamText).toBe('');
    expect(streaming.currentEmotion).toBeNull();
    expect(streaming.currentMotion).toBeNull();
  });

  it('multiple turns do not leak state', async () => {
    const streaming = useStreamingStore();
    const tts = useTtsPlayback();

    // Turn 1
    streaming.handleChunk({ text: 'Hello! ', done: false });
    tts.feedChunk('Hello! ');
    streaming.handleAnimation({ emotion: 'happy' });
    streaming.handleChunk({ text: '', done: true });
    tts.flush();
    await new Promise((r) => setTimeout(r, 20));

    expect(streaming.currentEmotion).toBe('happy');

    // Reset between turns (as conversation store does)
    streaming.reset();
    tts.stop();

    expect(streaming.currentEmotion).toBeNull();
    expect(streaming.streamText).toBe('');
    expect(tts.isSpeaking.value).toBe(false);

    // Turn 2 — fresh state
    streaming.handleChunk({ text: 'Goodbye! ', done: false });
    tts.feedChunk('Goodbye! ');
    streaming.handleAnimation({ emotion: 'sad' });
    streaming.handleChunk({ text: '', done: true });
    tts.flush();

    expect(streaming.streamText).toBe('Goodbye! ');
    expect(streaming.currentEmotion).toBe('sad');
  });
});

describe('three-stream sync — animation during TTS overlap', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(stubWavBytes());
    audioEndDelay = 30;
  });

  afterEach(() => {
    audioEndDelay = 0;
  });

  it('emotion is available while TTS is still speaking after stream ends', async () => {
    const streaming = useStreamingStore();
    const tts = useTtsPlayback();

    // Stream and feed TTS
    streaming.handleChunk({ text: 'I feel great! ', done: false });
    tts.feedChunk('I feel great! ');
    streaming.handleAnimation({ emotion: 'happy', intensity: 0.9 });
    streaming.handleChunk({ text: '', done: true });
    tts.flush();

    // Wait for TTS to start but not finish
    await new Promise((r) => setTimeout(r, 10));

    // Stream is done but TTS is still playing
    expect(streaming.isStreaming).toBe(false);
    expect(tts.isSpeaking.value).toBe(true);
    // Emotion is preserved for the watcher to use when TTS finishes
    expect(streaming.currentEmotion).toBe('happy');
    expect(streaming.currentEmotionIntensity).toBe(0.9);

    // Wait for TTS to finish
    await new Promise((r) => setTimeout(r, 40));
    expect(tts.isSpeaking.value).toBe(false);
    // Emotion still available
    expect(streaming.currentEmotion).toBe('happy');
  });

  it('multiple animation events during TTS keep latest emotion', async () => {
    const streaming = useStreamingStore();
    const tts = useTtsPlayback();

    tts.feedChunk('First sentence. ');
    streaming.handleChunk({ text: 'First sentence. ', done: false });
    streaming.handleAnimation({ emotion: 'happy' });

    tts.feedChunk('Second sentence. ');
    streaming.handleChunk({ text: 'Second sentence. ', done: false });
    streaming.handleAnimation({ emotion: 'surprised' });

    streaming.handleChunk({ text: '', done: true });
    tts.flush();

    await new Promise((r) => setTimeout(r, 10));

    // Latest emotion wins
    expect(streaming.currentEmotion).toBe('surprised');
  });

  it('audio lifecycle callbacks fire in correct order', async () => {
    audioEndDelay = 0; // immediate for simplicity
    const tts = useTtsPlayback();
    const events: string[] = [];

    tts.onAudioStart(() => events.push('start'));
    tts.onAudioEnd(() => events.push('end'));

    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 20));

    expect(events).toEqual(['start', 'end']);
  });

  it('onPlaybackStop fires on user-initiated stop', () => {
    const tts = useTtsPlayback();
    let stopped = false;
    tts.onPlaybackStop(() => { stopped = true; });

    tts.feedChunk('Hello ');
    tts.stop();

    expect(stopped).toBe(true);
    expect(tts.isSpeaking.value).toBe(false);
  });
});

describe('three-stream sync — edge cases', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(stubWavBytes());
    audioEndDelay = 0;
  });

  it('empty stream (no text) does not crash TTS', () => {
    const streaming = useStreamingStore();
    const tts = useTtsPlayback();

    streaming.handleChunk({ text: '', done: true });
    tts.flush();

    expect(streaming.isStreaming).toBe(false);
    expect(tts.isSpeaking.value).toBe(false);
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('animation-only stream (no text) is valid', () => {
    const streaming = useStreamingStore();

    streaming.handleAnimation({ emotion: 'happy', motion: 'wave' });
    streaming.handleChunk({ text: '', done: true });

    expect(streaming.isStreaming).toBe(false);
    expect(streaming.currentEmotion).toBe('happy');
    expect(streaming.currentMotion).toBe('wave');
  });

  it('rapid stop/restart does not corrupt state', async () => {
    const tts = useTtsPlayback();

    tts.feedChunk('Hello world. ');
    tts.stop();
    tts.feedChunk('New message. ');

    await new Promise((r) => setTimeout(r, 20));

    // Old synthesis was cancelled; new one should have fired
    const ttsCalls = mockInvoke.mock.calls
      .filter((c: unknown[]) => c[0] === 'synthesize_tts');
    expect(ttsCalls.length).toBe(2);
  });

  it('very long text gets chunked into sentences correctly', () => {
    const tts = useTtsPlayback();

    const longText = 'This is sentence one. This is sentence two. This is sentence three. ';
    tts.feedChunk(longText);

    const ttsCalls = mockInvoke.mock.calls
      .filter((c: unknown[]) => c[0] === 'synthesize_tts')
      .map((c: unknown[]) => (c[1] as { text: string }).text);

    expect(ttsCalls).toEqual([
      'This is sentence one.',
      'This is sentence two.',
      'This is sentence three.',
    ]);
  });

  it('concurrent animation and text in same event cycle', () => {
    const streaming = useStreamingStore();

    // Simulate what happens when Rust emits both events in the same tick
    streaming.handleChunk({ text: 'Hi! ', done: false });
    streaming.handleAnimation({ emotion: 'happy' });

    expect(streaming.streamText).toBe('Hi! ');
    expect(streaming.isStreaming).toBe(true);
    expect(streaming.currentEmotion).toBe('happy');
  });

  it('TTS synthesis failure does not block streaming', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS backend down'));
    const streaming = useStreamingStore();
    const tts = useTtsPlayback();

    streaming.handleChunk({ text: 'Hello! ', done: false });
    tts.feedChunk('Hello! ');

    // Wait for synthesis failure to be handled
    await new Promise((r) => setTimeout(r, 30));

    // Streaming state is independent of TTS success
    expect(streaming.isStreaming).toBe(true);
    expect(streaming.streamText).toBe('Hello! ');

    streaming.handleChunk({ text: '', done: true });
    expect(streaming.isStreaming).toBe(false);
  });
});

describe('three-stream sync — state machine invariants', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(stubWavBytes());
    audioEndDelay = 0;
  });

  it('INVARIANT: currentEmotion persists from handleAnimation until reset()', () => {
    const streaming = useStreamingStore();

    // Phase 1: animation arrives
    streaming.handleAnimation({ emotion: 'happy' });
    expect(streaming.currentEmotion).toBe('happy');

    // Phase 2: text streams
    streaming.handleChunk({ text: 'Hello', done: false });
    expect(streaming.currentEmotion).toBe('happy');

    // Phase 3: stream ends
    streaming.handleChunk({ text: '', done: true });
    expect(streaming.currentEmotion).toBe('happy');

    // Phase 4: only reset() clears it
    streaming.reset();
    expect(streaming.currentEmotion).toBeNull();
  });

  it('INVARIANT: isStreaming transitions false→true only on text, true→false only on done', () => {
    const streaming = useStreamingStore();

    // Initial state
    expect(streaming.isStreaming).toBe(false);

    // Animation alone doesn't start streaming
    streaming.handleAnimation({ emotion: 'happy' });
    expect(streaming.isStreaming).toBe(false);

    // First text starts streaming
    streaming.handleChunk({ text: 'Hi', done: false });
    expect(streaming.isStreaming).toBe(true);

    // More text keeps streaming
    streaming.handleChunk({ text: ' there', done: false });
    expect(streaming.isStreaming).toBe(true);

    // done:true ends streaming
    streaming.handleChunk({ text: '', done: true });
    expect(streaming.isStreaming).toBe(false);
  });

  it('INVARIANT: TTS isSpeaking becomes true only after synthesis, false only after all audio completes', async () => {
    audioEndDelay = 20;
    const tts = useTtsPlayback();

    // Before feeding — not speaking
    expect(tts.isSpeaking.value).toBe(false);

    // Feed a sentence — synthesis starts
    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 5));
    expect(tts.isSpeaking.value).toBe(true);

    // Wait for audio to finish
    await new Promise((r) => setTimeout(r, 30));
    expect(tts.isSpeaking.value).toBe(false);
  });

  it('INVARIANT: spokenText only grows during a session (no regression)', async () => {
    const tts = useTtsPlayback();
    const spokenSnapshots: string[] = [];

    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 10));
    spokenSnapshots.push(tts.spokenText.value ?? '');

    tts.feedChunk('Goodbye world. ');
    await new Promise((r) => setTimeout(r, 10));
    spokenSnapshots.push(tts.spokenText.value ?? '');

    // Each snapshot should be >= the previous
    for (let i = 1; i < spokenSnapshots.length; i++) {
      expect(spokenSnapshots[i].length).toBeGreaterThanOrEqual(spokenSnapshots[i - 1].length);
    }
  });
});
