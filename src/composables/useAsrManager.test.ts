/**
 * Tests for useAsrManager composable.
 *
 * Tests cover:
 * - Provider routing (web-speech vs Tauri IPC)
 * - No provider guard
 * - Transcript callback invocation
 * - Error handling (transcription failure, SpeechRecognition unavailable)
 * - isListening state transitions
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useAsrManager } from './useAsrManager';

// ── Tauri IPC mock ────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Voice store mock ─────────────────────────────────────────────────────────

vi.mock('../stores/voice', () => ({
  useVoiceStore: () => mockVoiceStore,
}));

const mockVoiceStore = {
  config: { asr_provider: null as string | null },
};

// ── VAD mock ─────────────────────────────────────────────────────────────────

const capturedVadCallbacks: { onSpeechEnd?: (audio: Float32Array) => void } = {};
const mockStartMic = vi.fn().mockResolvedValue(undefined);
const mockStopMic = vi.fn();
const mockVadError = { value: null as string | null };

vi.mock('../utils/vad', () => ({
  useVad: (callbacks: { onSpeechEnd?: (audio: Float32Array) => void }) => {
    capturedVadCallbacks.onSpeechEnd = callbacks.onSpeechEnd;
    return {
      startMic: mockStartMic,
      stopMic: mockStopMic,
      error: mockVadError,
      micOn: { value: false },
      isSpeaking: { value: false },
      lastProbability: { value: 0 },
      canStart: { value: true },
    };
  },
}));

// ── SpeechRecognition mock ────────────────────────────────────────────────────

class MockSpeechRecognition {
  continuous = false;
  interimResults = false;
  lang = '';
  onresult: ((event: unknown) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;
  onend: (() => void) | null = null;

  start = vi.fn(() => {
    // Simulate immediate result for testing
  });
  stop = vi.fn();
  abort = vi.fn(() => {
    this.onend?.();
  });
}

let mockRecognitionInstance: MockSpeechRecognition;

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('useAsrManager — provider routing', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStartMic.mockReset().mockResolvedValue(undefined);
    mockStopMic.mockReset();
    mockVadError.value = null;
    mockVoiceStore.config.asr_provider = null;

    // Install SpeechRecognition mock (must be regular function, not arrow, to be constructable)
    mockRecognitionInstance = new MockSpeechRecognition();
    vi.stubGlobal('SpeechRecognition', vi.fn(function () { return mockRecognitionInstance; }));
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns isListening=false initially', () => {
    const asr = useAsrManager();
    expect(asr.isListening.value).toBe(false);
  });

  it('returns error when no provider is configured', async () => {
    mockVoiceStore.config.asr_provider = null;
    const errors: string[] = [];
    const asr = useAsrManager({ onError: (e) => errors.push(e) });

    await asr.startListening();

    expect(asr.error.value).toBe('No ASR provider configured');
    expect(errors).toHaveLength(1);
    expect(asr.isListening.value).toBe(false);
  });

  it('uses SpeechRecognition for web-speech provider', async () => {
    mockVoiceStore.config.asr_provider = 'web-speech';
    const asr = useAsrManager();

    await asr.startListening();

    expect(mockRecognitionInstance.start).toHaveBeenCalledOnce();
    expect(asr.isListening.value).toBe(true);
  });

  it('fires onTranscript when SpeechRecognition returns a result', async () => {
    mockVoiceStore.config.asr_provider = 'web-speech';
    const transcripts: string[] = [];
    const asr = useAsrManager({ onTranscript: (t) => transcripts.push(t) });

    await asr.startListening();

    // Simulate a recognition result
    mockRecognitionInstance.onresult?.({
      results: { 0: { 0: { transcript: 'hello world' } } },
    });

    expect(transcripts).toEqual(['hello world']);
  });

  it('does not fire onTranscript for empty web-speech result', async () => {
    mockVoiceStore.config.asr_provider = 'web-speech';
    const transcripts: string[] = [];
    const asr = useAsrManager({ onTranscript: (t) => transcripts.push(t) });

    await asr.startListening();

    mockRecognitionInstance.onresult?.({
      results: { 0: { 0: { transcript: '   ' } } },
    });

    expect(transcripts).toHaveLength(0);
  });

  it('uses VAD + Tauri IPC for stub provider', async () => {
    mockVoiceStore.config.asr_provider = 'stub';
    const asr = useAsrManager();

    await asr.startListening();

    expect(mockStartMic).toHaveBeenCalledOnce();
    expect(asr.isListening.value).toBe(true);
  });

  it('uses VAD + Tauri IPC for whisper-api provider', async () => {
    mockVoiceStore.config.asr_provider = 'whisper-api';
    const asr = useAsrManager();

    await asr.startListening();

    expect(mockStartMic).toHaveBeenCalledOnce();
    expect(asr.isListening.value).toBe(true);
  });

  it('uses VAD + Tauri IPC for groq-whisper provider', async () => {
    mockVoiceStore.config.asr_provider = 'groq-whisper';
    const asr = useAsrManager();

    await asr.startListening();

    expect(mockStartMic).toHaveBeenCalledOnce();
    expect(asr.isListening.value).toBe(true);
  });

  it('calls invoke transcribe_audio when VAD speech ends', async () => {
    mockVoiceStore.config.asr_provider = 'stub';
    mockInvoke.mockResolvedValue({ text: 'test transcript', language: 'en', confidence: 0.95 });

    const transcripts: string[] = [];
    const asr = useAsrManager({ onTranscript: (t) => transcripts.push(t) });
    await asr.startListening();

    const audio = new Float32Array([0.1, 0.2, 0.3]);
    await capturedVadCallbacks.onSpeechEnd?.(audio);

    expect(mockInvoke).toHaveBeenCalledWith('transcribe_audio', {
      samples: Array.from(audio), // Float32Array precision-aware comparison
    });
    expect(transcripts).toEqual(['test transcript']);
  });

  it('handles transcribe_audio IPC error gracefully', async () => {
    mockVoiceStore.config.asr_provider = 'stub';
    mockInvoke.mockRejectedValue(new Error('network error'));

    const errors: string[] = [];
    const asr = useAsrManager({ onError: (e) => errors.push(e) });
    await asr.startListening();

    const audio = new Float32Array([0.1]);
    await capturedVadCallbacks.onSpeechEnd?.(audio);

    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain('Transcription failed');
  });

  it('stopListening stops VAD mic for non-web-speech provider', async () => {
    mockVoiceStore.config.asr_provider = 'stub';
    const asr = useAsrManager();
    await asr.startListening();

    asr.stopListening();

    expect(mockStopMic).toHaveBeenCalledOnce();
    expect(asr.isListening.value).toBe(false);
  });

  it('startListening is a no-op if already listening', async () => {
    mockVoiceStore.config.asr_provider = 'stub';
    const asr = useAsrManager();
    await asr.startListening();
    await asr.startListening();

    expect(mockStartMic).toHaveBeenCalledOnce();
  });

  it('error from VAD propagates to onError', async () => {
    mockVoiceStore.config.asr_provider = 'stub';
    mockVadError.value = 'Microphone permission denied';

    const errors: string[] = [];
    const asr = useAsrManager({ onError: (e) => errors.push(e) });
    await asr.startListening();

    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain('Microphone permission denied');
    expect(asr.isListening.value).toBe(false);
  });
});
