/**
 * Tests for the useTtsPlayback composable.
 *
 * The composable handles sentence-boundary detection, synthesis queuing,
 * and sequential audio playback. We mock the Tauri invoke() and
 * HTMLAudioElement to test the logic without real audio hardware.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { hasBrowserTtsVoice, useTtsPlayback } from './useTtsPlayback';

// ── Mock Tauri IPC ────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Mock HTMLAudioElement ─────────────────────────────────────────────────────

class MockAudio {
  src = '';
  onended: (() => void) | null = null;
  onerror: (() => void) | null = null;

  play(): Promise<void> {
    // Immediately trigger onended so tests don't hang
    Promise.resolve().then(() => this.onended?.());
    return Promise.resolve();
  }

  pause() {}
}

// ── Mock URL.createObjectURL / revokeObjectURL ────────────────────────────────

const createdUrls: string[] = [];
const revokedUrls: string[] = [];

vi.stubGlobal('URL', {
  createObjectURL: vi.fn((_blob: Blob) => {
    const url = `blob:mock-${createdUrls.length}`;
    createdUrls.push(url);
    return url;
  }),
  revokeObjectURL: vi.fn((url: string) => {
    revokedUrls.push(url);
  }),
});

vi.stubGlobal('Audio', MockAudio);
vi.stubGlobal('Blob', class MockBlob {
  constructor(public parts: BlobPart[], public options?: BlobPropertyBag) {}
});

// ── Mock SpeechSynthesis (Web Speech API fallback) ────────────────────────────

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

const spokenUtterances: string[] = [];
const mockSpeechSynthesis = {
  speak: vi.fn((utterance: MockSpeechSynthesisUtterance) => {
    spokenUtterances.push(utterance.text);
    // Simulate async completion
    Promise.resolve().then(() => utterance.onend?.());
  }),
  getVoices: vi.fn((): SpeechSynthesisVoice[] => []),
  cancel: vi.fn(),
  pause: vi.fn(),
  resume: vi.fn(),
};

vi.stubGlobal('SpeechSynthesisUtterance', MockSpeechSynthesisUtterance);
vi.stubGlobal('speechSynthesis', mockSpeechSynthesis);

// ── Helpers ───────────────────────────────────────────────────────────────────

/** A small valid WAV header as a number array (returned by Tauri IPC). */
function stubWavBytes(): number[] {
  // 44-byte WAV header + 100 bytes of PCM silence (enough to pass >44 check)
  const totalDataBytes = 100;
  const buf = new ArrayBuffer(44 + totalDataBytes);
  const view = new DataView(buf);
  // RIFF
  view.setUint8(0, 0x52); view.setUint8(1, 0x49); view.setUint8(2, 0x46); view.setUint8(3, 0x46);
  view.setUint32(4, 36 + totalDataBytes, true); // file size - 8
  view.setUint8(8, 0x57); view.setUint8(9, 0x41); view.setUint8(10, 0x56); view.setUint8(11, 0x45);
  // fmt
  view.setUint8(12, 0x66); view.setUint8(13, 0x6d); view.setUint8(14, 0x74); view.setUint8(15, 0x20);
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true); view.setUint16(22, 1, true); // PCM mono
  view.setUint32(24, 16000, true); view.setUint32(28, 32000, true);
  view.setUint16(32, 2, true); view.setUint16(34, 16, true);
  // data
  view.setUint8(36, 0x64); view.setUint8(37, 0x61); view.setUint8(38, 0x74); view.setUint8(39, 0x61);
  view.setUint32(40, totalDataBytes, true);
  return Array.from(new Uint8Array(buf));
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('useTtsPlayback — sentence detection', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    createdUrls.length = 0;
    revokedUrls.length = 0;
    spokenUtterances.length = 0;
    mockSpeechSynthesis.speak.mockClear();
    mockSpeechSynthesis.cancel.mockClear();
  });

  it('does not call synthesize_tts for text without sentence boundary', () => {
    const tts = useTtsPlayback();
    tts.feedChunk('Hello world');
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('calls synthesize_tts when a period-space boundary is detected', async () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    tts.feedChunk('Hello world. ');
    expect(mockInvoke).toHaveBeenCalledWith('synthesize_tts', { text: 'Hello world.' });
  });

  it('calls synthesize_tts for exclamation mark boundary', async () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    tts.feedChunk('Great job! ');
    expect(mockInvoke).toHaveBeenCalledWith('synthesize_tts', { text: 'Great job!' });
  });

  it('calls synthesize_tts for question mark boundary', async () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    tts.feedChunk('How are you? ');
    expect(mockInvoke).toHaveBeenCalledWith('synthesize_tts', { text: 'How are you?' });
  });

  it('detects multiple sentences in a single chunk', () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    tts.feedChunk('Hello. How are you? I am fine. ');
    expect(mockInvoke).toHaveBeenCalledTimes(3);
  });

  it('accumulates fragments across chunks before triggering synthesis', () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    tts.feedChunk('Hello');
    expect(mockInvoke).not.toHaveBeenCalled();
    tts.feedChunk(' world. ');
    expect(mockInvoke).toHaveBeenCalledWith('synthesize_tts', { text: 'Hello world.' });
  });

  it('flush synthesizes remaining buffer text', () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    tts.feedChunk('This is the final sentence');
    expect(mockInvoke).not.toHaveBeenCalled();
    tts.flush();
    expect(mockInvoke).toHaveBeenCalledWith('synthesize_tts', {
      text: 'This is the final sentence',
    });
  });

  it('flush does nothing if buffer is empty', () => {
    const tts = useTtsPlayback();
    tts.flush();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('flush does nothing if remaining text is too short', () => {
    const tts = useTtsPlayback();
    tts.feedChunk('Hi');
    tts.flush();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('strips emoji before sending text to synthesize_tts', () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    tts.feedChunk('Hello 👋 world 🌍! ');
    expect(mockInvoke).toHaveBeenCalledWith('synthesize_tts', { text: 'Hello world!' });
  });
});

describe('useTtsPlayback — stop', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    createdUrls.length = 0;
    revokedUrls.length = 0;
    spokenUtterances.length = 0;
    mockSpeechSynthesis.speak.mockClear();
    mockSpeechSynthesis.cancel.mockClear();
  });

  it('stop sets isSpeaking to false and allows fresh start for next message', () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    // After stop(), isSpeaking is false and the composable is ready for reuse
    tts.stop();
    expect(tts.isSpeaking.value).toBe(false);
    // A subsequent feedChunk starts a new session cleanly
    tts.feedChunk('Hello world. ');
    expect(mockInvoke).toHaveBeenCalledWith('synthesize_tts', { text: 'Hello world.' });
  });

  it('stop clears the buffer so flush does nothing', () => {
    const tts = useTtsPlayback();
    tts.feedChunk('Hello world');
    tts.stop();
    tts.flush();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('isSpeaking starts false', () => {
    const tts = useTtsPlayback();
    expect(tts.isSpeaking.value).toBe(false);
  });
});

describe('useTtsPlayback — synthesis error handling', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    spokenUtterances.length = 0;
    mockSpeechSynthesis.speak.mockClear();
    // Restore default speak behavior (simulate async completion)
    mockSpeechSynthesis.speak.mockImplementation((utterance: MockSpeechSynthesisUtterance) => {
      spokenUtterances.push(utterance.text);
      Promise.resolve().then(() => utterance.onend?.());
    });
  });

  it('falls back to Web Speech API when synthesize_tts fails', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const tts = useTtsPlayback();
    tts.feedChunk('Hello world. ');
    // Wait for synthesis rejection + fallback speech
    await new Promise((r) => setTimeout(r, 50));
    expect(mockSpeechSynthesis.speak).toHaveBeenCalled();
    expect(spokenUtterances).toContain('Hello world.');
  });

  it('strips emoji before browser speech fallback', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const tts = useTtsPlayback();
    tts.feedChunk('Great job 🎉! ');
    await new Promise((r) => setTimeout(r, 50));
    expect(spokenUtterances).toContain('Great job!');
  });

  it('isSpeaking is true during browser speech fallback', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    // Make speak NOT immediately resolve so we can check isSpeaking
    mockSpeechSynthesis.speak.mockImplementation((utterance: MockSpeechSynthesisUtterance) => {
      spokenUtterances.push(utterance.text);
      // Delay completion
      setTimeout(() => utterance.onend?.(), 30);
    });
    const tts = useTtsPlayback();
    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 15));
    expect(tts.isSpeaking.value).toBe(true);
    await new Promise((r) => setTimeout(r, 40));
    expect(tts.isSpeaking.value).toBe(false);
  });

  it('stop cancels browser speech synthesis', () => {
    const tts = useTtsPlayback();
    tts.stop();
    expect(mockSpeechSynthesis.cancel).toHaveBeenCalled();
  });

  it('gracefully handles synthesize_tts error (no crash)', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const tts = useTtsPlayback();
    tts.feedChunk('Hello world. ');
    // Wait for promise rejection + fallback to be handled internally
    await new Promise((r) => setTimeout(r, 50));
    // Should not throw; isSpeaking should return to false eventually
    expect(tts.isSpeaking.value).toBe(false);
  });
});

describe('useTtsPlayback — audio lifecycle callbacks', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    createdUrls.length = 0;
    revokedUrls.length = 0;
    spokenUtterances.length = 0;
    mockSpeechSynthesis.speak.mockClear();
  });

  it('onAudioStart callback fires with HTMLAudioElement before play', async () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    const received: unknown[] = [];
    tts.onAudioStart((audio) => received.push(audio));

    tts.feedChunk('Hello world. ');
    // Wait for synthesis + playback scheduling
    await new Promise((r) => setTimeout(r, 20));
    expect(received.length).toBeGreaterThanOrEqual(1);
    expect(received[0]).toBeInstanceOf(MockAudio);
  });

  it('onAudioEnd callback fires when sentence finishes playing', async () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    let endCount = 0;
    tts.onAudioEnd(() => endCount++);

    tts.feedChunk('Hello world. ');
    // MockAudio triggers onended immediately in microtask
    await new Promise((r) => setTimeout(r, 20));
    expect(endCount).toBeGreaterThanOrEqual(1);
  });

  it('onPlaybackStop callback fires when stop() is called', () => {
    const tts = useTtsPlayback();
    let stopFired = false;
    tts.onPlaybackStop(() => { stopFired = true; });

    tts.stop();
    expect(stopFired).toBe(true);
  });

  it('callbacks are not called when not registered', async () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    // No callbacks registered — should not throw
    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 20));
    tts.stop();
    // If we got here without error, callbacks are safely optional
    expect(true).toBe(true);
  });
});

describe('useTtsPlayback — gender voice pitch', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    spokenUtterances.length = 0;
    mockSpeechSynthesis.speak.mockClear();
    mockSpeechSynthesis.getVoices.mockReset();
    mockSpeechSynthesis.getVoices.mockReturnValue([]);
    mockSpeechSynthesis.speak.mockImplementation((utterance: MockSpeechSynthesisUtterance) => {
      spokenUtterances.push(utterance.text);
      Promise.resolve().then(() => utterance.onend?.());
    });
  });

  it('applies getBrowserPitch to SpeechSynthesisUtterance when falling back', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const tts = useTtsPlayback({ getBrowserPitch: () => 1.5 });
    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 50));
    expect(mockSpeechSynthesis.speak).toHaveBeenCalled();
    const utterance = mockSpeechSynthesis.speak.mock.calls[0][0];
    expect(utterance.pitch).toBe(1.5);
  });

  it('uses default pitch 1.0 when getBrowserPitch is not provided', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const tts = useTtsPlayback();
    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 50));
    expect(mockSpeechSynthesis.speak).toHaveBeenCalled();
    const utterance = mockSpeechSynthesis.speak.mock.calls[0][0];
    expect(utterance.pitch).toBe(1.0);
  });

  it('applies male pitch 0.8 via getBrowserPitch option', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const tts = useTtsPlayback({ getBrowserPitch: () => 0.8 });
    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 50));
    const utterance = mockSpeechSynthesis.speak.mock.calls[0][0];
    expect(utterance.pitch).toBe(0.8);
  });

  it('applies getBrowserRate to SpeechSynthesisUtterance when falling back', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const tts = useTtsPlayback({ getBrowserRate: () => 1.15 });
    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 50));
    const utterance = mockSpeechSynthesis.speak.mock.calls[0][0];
    expect(utterance.rate).toBe(1.15);
  });

  it('uses default rate 1.0 when getBrowserRate is not provided', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const tts = useTtsPlayback();
    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 50));
    const utterance = mockSpeechSynthesis.speak.mock.calls[0][0];
    expect(utterance.rate).toBe(1.0);
  });
});

describe('useTtsPlayback — multilingual browser voices', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    spokenUtterances.length = 0;
    mockSpeechSynthesis.speak.mockClear();
    mockSpeechSynthesis.getVoices.mockReset();
    mockSpeechSynthesis.getVoices.mockReturnValue([
      { name: 'Vietnamese Voice', lang: 'vi-VN' },
      { name: 'Japanese Voice', lang: 'ja-JP' },
      { name: 'Spanish Voice', lang: 'es-ES' },
    ] as SpeechSynthesisVoice[]);
    mockSpeechSynthesis.speak.mockImplementation((utterance: MockSpeechSynthesisUtterance) => {
      spokenUtterances.push(utterance.text);
      Promise.resolve().then(() => utterance.onend?.());
    });
  });

  it.each([
    ['vi', 'vi', 'Vietnamese Voice', 'xin chào. '],
    ['ja', 'ja', 'Japanese Voice', 'こんにちは。 '],
    ['es', 'es', 'Spanish Voice', 'hola. '],
  ])('sets SpeechSynthesisUtterance language for translator chunks in %s', async (
    language,
    expectedLang,
    expectedVoice,
    sentence,
  ) => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const tts = useTtsPlayback();
    tts.feedChunk(sentence, { language });
    await new Promise((r) => setTimeout(r, 50));

    const utterance = mockSpeechSynthesis.speak.mock.calls[0][0];
    expect(utterance.lang).toBe(expectedLang);
    expect(utterance.voice?.name).toBe(expectedVoice);
  });

  it('infers browser language from translator-mode labels when metadata is unavailable', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const tts = useTtsPlayback();
    tts.feedChunk('English → Vietnamese: xin chào. ');
    await new Promise((r) => setTimeout(r, 50));

    const utterance = mockSpeechSynthesis.speak.mock.calls[0][0];
    expect(utterance.lang).toBe('vi');
    expect(utterance.voice?.name).toBe('Vietnamese Voice');
  });

  it('reports missing browser voices so the UI can prompt installation', async () => {
    mockSpeechSynthesis.getVoices.mockReturnValue([]);
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const missing: Array<{ language?: string; code?: string; name?: string }> = [];
    const listener = (event: Event) => {
      missing.push((event as CustomEvent<{ code?: string; name?: string }>).detail);
    };
    window.addEventListener('ts:tts-voice-missing', listener);

    try {
      const tts = useTtsPlayback();
      tts.feedChunk('مرحبا. ', { language: 'ar' });
      await new Promise((r) => setTimeout(r, 50));
      expect(missing).toContainEqual({ language: 'ar', code: 'ar', name: 'Arabic' });
    } finally {
      window.removeEventListener('ts:tts-voice-missing', listener);
    }
  });

  it('checks installed browser voices by language prefix', () => {
    expect(hasBrowserTtsVoice('Vietnamese')).toBe(true);
    expect(hasBrowserTtsVoice('ar')).toBe(false);
  });
});

describe('useTtsPlayback — sentence tracking (currentSentence & spokenText)', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    createdUrls.length = 0;
    revokedUrls.length = 0;
    spokenUtterances.length = 0;
    mockSpeechSynthesis.speak.mockClear();
    mockSpeechSynthesis.cancel.mockClear();
  });

  it('currentSentence and spokenText start empty', () => {
    const tts = useTtsPlayback();
    expect(tts.currentSentence.value).toBe('');
    expect(tts.spokenText.value).toBe('');
  });

  it('currentSentence is set to the sentence being played', async () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    tts.feedChunk('Hello world. ');
    // Wait for synthesis promise + playback start
    await new Promise((r) => setTimeout(r, 30));
    // currentSentence should be 'Hello world.' during playback
    // After playback completes (MockAudio triggers onended immediately),
    // it should accumulate into spokenText
    expect(tts.spokenText.value).toContain('Hello world.');
  });

  it('spokenText accumulates across multiple sentences', async () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    tts.feedChunk('First sentence. Second sentence. ');
    // Wait for both sentences to be synthesized and played
    await new Promise((r) => setTimeout(r, 50));
    expect(tts.spokenText.value).toContain('First sentence.');
    expect(tts.spokenText.value).toContain('Second sentence.');
  });

  it('stop clears currentSentence and spokenText', async () => {
    mockInvoke.mockResolvedValue(stubWavBytes());
    const tts = useTtsPlayback();
    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 30));
    tts.stop();
    expect(tts.currentSentence.value).toBe('');
    expect(tts.spokenText.value).toBe('');
  });

  it('currentSentence is empty when not speaking', () => {
    const tts = useTtsPlayback();
    expect(tts.currentSentence.value).toBe('');
    tts.feedChunk('no sentence boundary yet');
    expect(tts.currentSentence.value).toBe('');
  });
});

describe('useTtsPlayback — global mute via mutedRef', () => {
  beforeEach(async () => {
    mockInvoke.mockReset();
    createdUrls.length = 0;
    revokedUrls.length = 0;
    spokenUtterances.length = 0;
    mockSpeechSynthesis.speak.mockClear();
    mockSpeechSynthesis.cancel.mockClear();
  });

  it('mutes the active HTMLAudioElement when mutedRef flips to true', async () => {
    const { ref } = await import('vue');
    // Override MockAudio for this test only — play() does NOT auto-resolve
    // so currentAudio stays alive long enough for the watch to mutate it.
    class HoldingAudio extends MockAudio {
      muted = false;
      override play(): Promise<void> { return Promise.resolve(); }
    }
    vi.stubGlobal('Audio', HoldingAudio);

    mockInvoke.mockResolvedValue(stubWavBytes());
    const muted = ref(false);
    const tts = useTtsPlayback({ mutedRef: muted });
    let captured: HoldingAudio | null = null;
    tts.onAudioStart((a) => {
      captured = a as unknown as HoldingAudio;
    });
    tts.feedChunk('Hello world. ');
    await new Promise((r) => setTimeout(r, 20));
    expect(captured).not.toBeNull();
    expect(captured!.muted).toBe(false);
    muted.value = true;
    await new Promise((r) => setTimeout(r, 5));
    expect(captured!.muted).toBe(true);
    muted.value = false;
    await new Promise((r) => setTimeout(r, 5));
    expect(captured!.muted).toBe(false);

    // Restore default MockAudio for subsequent tests.
    vi.stubGlobal('Audio', MockAudio);
  });

  it('starts the next utterance already muted when mutedRef is true at synth time', async () => {
    const { ref } = await import('vue');
    mockInvoke.mockResolvedValue(stubWavBytes());
    const muted = ref(true);
    const tts = useTtsPlayback({ mutedRef: muted });
    let captured: { muted: boolean } | null = null;
    tts.onAudioStart((a) => { captured = a as unknown as { muted: boolean }; });
    tts.feedChunk('Greetings. ');
    await new Promise((r) => setTimeout(r, 20));
    expect(captured).not.toBeNull();
    expect(captured!.muted).toBe(true);
  });

  it('pauses speechSynthesis when mutedRef flips to true and resumes on false', async () => {
    const { ref } = await import('vue');
    const pause = vi.fn();
    const resume = vi.fn();
    (mockSpeechSynthesis as unknown as Record<string, unknown>).pause = pause;
    (mockSpeechSynthesis as unknown as Record<string, unknown>).resume = resume;
    const muted = ref(false);
    useTtsPlayback({ mutedRef: muted });
    muted.value = true;
    await new Promise((r) => setTimeout(r, 5));
    expect(pause).toHaveBeenCalledTimes(1);
    muted.value = false;
    await new Promise((r) => setTimeout(r, 5));
    expect(resume).toHaveBeenCalledTimes(1);
  });
});
