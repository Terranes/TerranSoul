/**
 * Tests for the useTtsPlayback composable.
 *
 * The composable handles sentence-boundary detection, synthesis queuing,
 * and sequential audio playback. We mock the Tauri invoke() and
 * HTMLAudioElement to test the logic without real audio hardware.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useTtsPlayback } from './useTtsPlayback';

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

// ── Helpers ───────────────────────────────────────────────────────────────────

/** A small valid WAV header as a number array (returned by Tauri IPC). */
function stubWavBytes(): number[] {
  // 44-byte WAV header with zero data (silence)
  const buf = new ArrayBuffer(44);
  const view = new DataView(buf);
  // RIFF
  view.setUint8(0, 0x52); view.setUint8(1, 0x49); view.setUint8(2, 0x46); view.setUint8(3, 0x46);
  view.setUint32(4, 36, true); // file size - 8
  view.setUint8(8, 0x57); view.setUint8(9, 0x41); view.setUint8(10, 0x56); view.setUint8(11, 0x45);
  // fmt
  view.setUint8(12, 0x66); view.setUint8(13, 0x6d); view.setUint8(14, 0x74); view.setUint8(15, 0x20);
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true); view.setUint16(22, 1, true); // PCM mono
  view.setUint32(24, 16000, true); view.setUint32(28, 32000, true);
  view.setUint16(32, 2, true); view.setUint16(34, 16, true);
  // data
  view.setUint8(36, 0x64); view.setUint8(37, 0x61); view.setUint8(38, 0x74); view.setUint8(39, 0x61);
  view.setUint32(40, 0, true);
  return Array.from(new Uint8Array(buf));
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('useTtsPlayback — sentence detection', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    createdUrls.length = 0;
    revokedUrls.length = 0;
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
});

describe('useTtsPlayback — stop', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    createdUrls.length = 0;
    revokedUrls.length = 0;
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
  });

  it('gracefully handles synthesize_tts error (no crash)', async () => {
    mockInvoke.mockRejectedValue(new Error('TTS error'));
    const tts = useTtsPlayback();
    tts.feedChunk('Hello world. ');
    // Wait for promise rejection to be handled internally
    await new Promise((r) => setTimeout(r, 10));
    // Should not throw; isSpeaking should return to false eventually
    expect(tts.isSpeaking.value).toBe(false);
  });
});
