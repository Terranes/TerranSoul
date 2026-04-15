/**
 * Tests for useBgmPlayer composable.
 *
 * Web Audio API is not available in jsdom, so we test the reactive state
 * transitions and API contract with a lightweight AudioContext mock.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useBgmPlayer, BGM_TRACKS, DEFAULT_BGM_VOLUME } from './useBgmPlayer';

// ── Web Audio API mock ────────────────────────────────────────────────────────

function createMockGainNode(): GainNode {
  return {
    gain: {
      value: 0,
      cancelScheduledValues: vi.fn().mockReturnThis(),
      setValueAtTime: vi.fn().mockReturnThis(),
      linearRampToValueAtTime: vi.fn().mockReturnThis(),
    },
    connect: vi.fn(),
    disconnect: vi.fn(),
  } as unknown as GainNode;
}

function createMockOscillator(): OscillatorNode {
  return {
    type: 'sine',
    frequency: { value: 0, setValueAtTime: vi.fn() },
    detune: { value: 0 },
    connect: vi.fn(),
    disconnect: vi.fn(),
    start: vi.fn(),
    stop: vi.fn(),
  } as unknown as OscillatorNode;
}

function createMockBufferSource(): AudioBufferSourceNode {
  return {
    buffer: null,
    loop: false,
    connect: vi.fn(),
    disconnect: vi.fn(),
    start: vi.fn(),
    stop: vi.fn(),
  } as unknown as AudioBufferSourceNode;
}

const mockAudioContext = {
  currentTime: 0,
  sampleRate: 44100,
  destination: {},
  createGain: vi.fn(() => createMockGainNode()),
  createOscillator: vi.fn(() => createMockOscillator()),
  createBufferSource: vi.fn(() => createMockBufferSource()),
  createBiquadFilter: vi.fn(() => ({
    type: 'lowpass',
    frequency: { value: 0 },
    connect: vi.fn(),
    disconnect: vi.fn(),
  })),
  createBuffer: vi.fn((_channels: number, length: number, _sampleRate: number) => ({
    getChannelData: vi.fn(() => new Float32Array(length)),
    length,
  })),
} as unknown as AudioContext;

// Override global AudioContext
const originalAudioContext = globalThis.AudioContext;
beforeEach(() => {
  // vi.fn() produces a function, but `new AudioContext()` needs a constructor.
  // Wrapping in a class ensures `new` works correctly.
  (globalThis as Record<string, unknown>).AudioContext = class {
    currentTime = mockAudioContext.currentTime;
    sampleRate = mockAudioContext.sampleRate;
    destination = mockAudioContext.destination;
    createGain = mockAudioContext.createGain;
    createOscillator = mockAudioContext.createOscillator;
    createBufferSource = mockAudioContext.createBufferSource;
    createBiquadFilter = mockAudioContext.createBiquadFilter;
    createBuffer = mockAudioContext.createBuffer;
  };
});
afterEach(() => {
  (globalThis as Record<string, unknown>).AudioContext = originalAudioContext;
});

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('useBgmPlayer', () => {
  it('exports preset tracks', () => {
    expect(BGM_TRACKS.length).toBeGreaterThanOrEqual(3);
    expect(BGM_TRACKS[0]).toHaveProperty('id');
    expect(BGM_TRACKS[0]).toHaveProperty('name');
  });

  it('has sensible default volume', () => {
    expect(DEFAULT_BGM_VOLUME).toBeGreaterThan(0);
    expect(DEFAULT_BGM_VOLUME).toBeLessThanOrEqual(0.5);
  });

  it('starts in stopped state', () => {
    const bgm = useBgmPlayer();
    expect(bgm.isPlaying.value).toBe(false);
    expect(bgm.currentTrackId.value).toBeNull();
    expect(bgm.volume.value).toBeCloseTo(DEFAULT_BGM_VOLUME);
  });

  it('play() sets isPlaying and currentTrackId', () => {
    const bgm = useBgmPlayer();
    bgm.play('ambient-calm');
    expect(bgm.isPlaying.value).toBe(true);
    expect(bgm.currentTrackId.value).toBe('ambient-calm');
  });

  it('stop() clears playing state', () => {
    const bgm = useBgmPlayer();
    bgm.play('ambient-calm');
    bgm.stop();
    expect(bgm.isPlaying.value).toBe(false);
    expect(bgm.currentTrackId.value).toBeNull();
  });

  it('play() switches tracks when already playing', () => {
    const bgm = useBgmPlayer();
    bgm.play('ambient-calm');
    expect(bgm.currentTrackId.value).toBe('ambient-calm');
    bgm.play('ambient-night');
    expect(bgm.currentTrackId.value).toBe('ambient-night');
    expect(bgm.isPlaying.value).toBe(true);
  });

  it('setVolume() clamps to [0, 1]', () => {
    const bgm = useBgmPlayer();
    bgm.setVolume(0.5);
    expect(bgm.volume.value).toBeCloseTo(0.5);
    bgm.setVolume(-0.1);
    expect(bgm.volume.value).toBe(0);
    bgm.setVolume(1.5);
    expect(bgm.volume.value).toBe(1);
  });

  it('setVolume() updates while playing', () => {
    const bgm = useBgmPlayer();
    bgm.play('ambient-calm');
    bgm.setVolume(0.7);
    expect(bgm.volume.value).toBeCloseTo(0.7);
    expect(bgm.isPlaying.value).toBe(true);
  });

  it('stop() is safe to call when already stopped', () => {
    const bgm = useBgmPlayer();
    expect(() => bgm.stop()).not.toThrow();
    expect(bgm.isPlaying.value).toBe(false);
  });

  it('all preset track IDs are unique', () => {
    const ids = BGM_TRACKS.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
