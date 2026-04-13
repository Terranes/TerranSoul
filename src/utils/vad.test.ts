import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Mock @ricky0123/vad-web ───────────────────────────────────────────────────

const mockStart = vi.fn().mockResolvedValue(undefined);
const mockPause = vi.fn().mockResolvedValue(undefined);
const mockDestroy = vi.fn().mockResolvedValue(undefined);

let capturedCallbacks: Record<string, (...args: any[]) => void> = {};

const MockMicVAD = {
  new: vi.fn().mockImplementation(async (opts: Record<string, unknown>) => {
    capturedCallbacks = {
      onSpeechStart: opts.onSpeechStart as () => void,
      onSpeechEnd: opts.onSpeechEnd as (audio: Float32Array) => void,
      onVADMisfire: opts.onVADMisfire as () => void,
      onFrameProcessed: opts.onFrameProcessed as (probs: { isSpeech: boolean }) => void,
    };
    return {
      start: mockStart,
      pause: mockPause,
      destroy: mockDestroy,
    };
  }),
};

vi.mock('@ricky0123/vad-web', () => ({
  MicVAD: MockMicVAD,
}));

// ── Mock onUnmounted to prevent Vue runtime errors in test ────────────────────
vi.mock('vue', async () => {
  const actual = await vi.importActual('vue');
  return {
    ...actual,
    onUnmounted: vi.fn(),
  };
});

import { useVad } from './vad';

describe('useVad', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedCallbacks = {};
  });

  it('initial state is not recording', () => {
    const { micOn, isSpeaking, lastProbability, error } = useVad();
    expect(micOn.value).toBe(false);
    expect(isSpeaking.value).toBe(false);
    expect(lastProbability.value).toBe(0);
    expect(error.value).toBeNull();
  });

  it('canStart is true when mic is off', () => {
    const { canStart } = useVad();
    expect(canStart.value).toBe(true);
  });

  it('startMic creates VAD and starts recording', async () => {
    const { micOn, startMic } = useVad();
    await startMic();

    expect(MockMicVAD.new).toHaveBeenCalledOnce();
    expect(mockStart).toHaveBeenCalledOnce();
    expect(micOn.value).toBe(true);
  });

  it('startMic does nothing when already recording', async () => {
    const { startMic } = useVad();
    await startMic();
    await startMic(); // Second call should be no-op

    expect(MockMicVAD.new).toHaveBeenCalledOnce();
  });

  it('stopMic pauses and destroys VAD', async () => {
    const { micOn, startMic, stopMic } = useVad();
    await startMic();
    stopMic();

    expect(mockPause).toHaveBeenCalledOnce();
    expect(mockDestroy).toHaveBeenCalledOnce();
    expect(micOn.value).toBe(false);
  });

  it('stopMic is safe when not started', () => {
    const { stopMic, micOn } = useVad();
    stopMic(); // Should not throw
    expect(micOn.value).toBe(false);
  });

  it('calls onSpeechStart callback', async () => {
    const onSpeechStart = vi.fn();
    const { isSpeaking, startMic } = useVad({ onSpeechStart });
    await startMic();

    capturedCallbacks.onSpeechStart();
    expect(onSpeechStart).toHaveBeenCalledOnce();
    expect(isSpeaking.value).toBe(true);
  });

  it('calls onSpeechEnd callback with audio data', async () => {
    const onSpeechEnd = vi.fn();
    const { isSpeaking, startMic } = useVad({ onSpeechEnd });
    await startMic();

    const audio = new Float32Array([0.1, 0.2, 0.3]);
    capturedCallbacks.onSpeechEnd(audio);
    expect(onSpeechEnd).toHaveBeenCalledWith(audio);
    expect(isSpeaking.value).toBe(false);
  });

  it('calls onMisfire callback', async () => {
    const onMisfire = vi.fn();
    const { isSpeaking, startMic } = useVad({ onMisfire });
    await startMic();

    capturedCallbacks.onSpeechStart(); // Start speaking
    capturedCallbacks.onVADMisfire(); // False positive
    expect(onMisfire).toHaveBeenCalledOnce();
    expect(isSpeaking.value).toBe(false);
  });

  it('calls onFrameProcessed with probability', async () => {
    const onFrameProcessed = vi.fn();
    const { lastProbability, startMic } = useVad({ onFrameProcessed });
    await startMic();

    capturedCallbacks.onFrameProcessed({ isSpeech: 0.85 });
    expect(onFrameProcessed).toHaveBeenCalledWith(0.85);
    expect(lastProbability.value).toBe(0.85);
  });

  it('updateSettings changes VAD config', () => {
    const { settings, updateSettings } = useVad();
    expect(settings.value.positiveSpeechThreshold).toBe(0.5);

    updateSettings({ positiveSpeechThreshold: 0.7 });
    expect(settings.value.positiveSpeechThreshold).toBe(0.7);
    // Other settings remain unchanged
    expect(settings.value.negativeSpeechThreshold).toBe(0.35);
  });

  it('passes settings to MicVAD constructor', async () => {
    const { startMic, updateSettings } = useVad();
    updateSettings({
      positiveSpeechThreshold: 0.8,
      negativeSpeechThreshold: 0.4,
      redemptionMs: 500,
    });
    await startMic();

    expect(MockMicVAD.new).toHaveBeenCalledWith(
      expect.objectContaining({
        positiveSpeechThreshold: 0.8,
        negativeSpeechThreshold: 0.4,
        redemptionMs: 500,
      }),
    );
  });

  it('handles MicVAD creation failure', async () => {
    MockMicVAD.new.mockRejectedValueOnce(new Error('No microphone'));
    const { micOn, error, startMic } = useVad();
    await startMic();

    expect(micOn.value).toBe(false);
    expect(error.value).toContain('No microphone');
  });

  it('default settings match expected values', () => {
    const { settings } = useVad();
    expect(settings.value).toEqual({
      positiveSpeechThreshold: 0.5,
      negativeSpeechThreshold: 0.35,
      redemptionMs: 300,
    });
  });
});
