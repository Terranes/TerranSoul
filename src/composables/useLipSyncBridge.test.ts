/**
 * Tests for the useLipSyncBridge composable.
 *
 * The bridge wires TTS audio → LipSync → AvatarStateMachine.viseme.
 * We mock browser audio APIs and verify the wiring logic.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ref } from 'vue';
import { useLipSyncBridge } from './useLipSyncBridge';
import type { TtsPlaybackHandle } from './useTtsPlayback';
import type { AvatarStateMachine } from '../renderer/avatar-state';

// ── Mock AudioContext + related APIs ──────────────────────────────────────────

class MockAnalyserNode {
  fftSize = 256;
  smoothingTimeConstant = 0.5;
  connect() {}
  disconnect() {}
  frequencyBinCount = 128;
  getByteFrequencyData(arr: Uint8Array) { arr.fill(0); }
}

class MockMediaElementSource {
  connect() {}
  disconnect() {}
}

class MockAudioContext {
  state = 'running' as AudioContextState;
  createAnalyser() { return new MockAnalyserNode() as unknown as AnalyserNode; }
  createMediaElementSource() { return new MockMediaElementSource() as unknown as MediaElementAudioSourceNode; }
  resume() { return Promise.resolve(); }
  close() { this.state = 'closed'; return Promise.resolve(); }
  get destination() { return {} as AudioDestinationNode; }
}

vi.stubGlobal('AudioContext', MockAudioContext);

// Mock requestAnimationFrame / cancelAnimationFrame
let rafCallbacks: Array<FrameRequestCallback> = [];
let rafIdCounter = 1;
vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
  const id = rafIdCounter++;
  rafCallbacks.push(cb);
  return id;
});
vi.stubGlobal('cancelAnimationFrame', () => {});

// ── Helpers ───────────────────────────────────────────────────────────────────

function createMockTts(): TtsPlaybackHandle & {
  _audioStartCb: ((audio: HTMLAudioElement) => void) | null;
  _audioEndCb: (() => void) | null;
  _playbackStopCb: (() => void) | null;
} {
  const handle = {
    isSpeaking: ref(false),
    feedChunk: vi.fn(),
    flush: vi.fn(),
    stop: vi.fn(),
    currentSentence: ref(''),
    spokenText: ref(''),
    _audioStartCb: null as ((audio: HTMLAudioElement) => void) | null,
    _audioEndCb: null as (() => void) | null,
    _playbackStopCb: null as (() => void) | null,
    onAudioStart(cb: (audio: HTMLAudioElement) => void) { handle._audioStartCb = cb; },
    onAudioEnd(cb: () => void) { handle._audioEndCb = cb; },
    onPlaybackStop(cb: () => void) { handle._playbackStopCb = cb; },
  };
  return handle;
}

function createMockAsm(): AvatarStateMachine {
  return {
    state: {
      body: 'idle' as const,
      emotion: 'neutral' as const,
      viseme: { aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 },
      blink: 0,
      lookAt: { yaw: 0, pitch: 0 },
      needsRender: false,
    },
    forceBody: vi.fn(),
    setEmotion: vi.fn(),
    setViseme: vi.fn(),
    zeroVisemes: vi.fn(),
    update: vi.fn(),
    isSettled: vi.fn(() => true),
  } as unknown as AvatarStateMachine;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('useLipSyncBridge', () => {
  beforeEach(() => {
    rafCallbacks = [];
    rafIdCounter = 1;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('registers TTS callbacks on creation', () => {
    const tts = createMockTts();
    const asm = createMockAsm();
    useLipSyncBridge(tts, () => asm);

    expect(tts._audioStartCb).toBeTypeOf('function');
    expect(tts._audioEndCb).toBeTypeOf('function');
    expect(tts._playbackStopCb).toBeTypeOf('function');
  });

  it('start() begins the rAF loop', () => {
    const tts = createMockTts();
    const asm = createMockAsm();
    const bridge = useLipSyncBridge(tts, () => asm);

    bridge.start();
    // rAF should have been called
    expect(rafCallbacks.length).toBeGreaterThanOrEqual(1);
  });

  it('start() is idempotent (calling twice does not double-loop)', () => {
    const tts = createMockTts();
    const bridge = useLipSyncBridge(tts, () => null);

    bridge.start();
    const countAfterFirst = rafCallbacks.length;
    bridge.start();
    expect(rafCallbacks.length).toBe(countAfterFirst);
  });

  it('dispose() stops the loop and cleans up', () => {
    const tts = createMockTts();
    const bridge = useLipSyncBridge(tts, () => null);

    bridge.start();
    bridge.dispose();
    // After dispose, tick should not schedule more frames
    // (running flag is false so tick returns immediately)
    const cbCount = rafCallbacks.length;
    // Simulate one more rAF tick
    if (rafCallbacks.length > 0) {
      rafCallbacks[rafCallbacks.length - 1](performance.now());
    }
    // Should NOT have added another callback (running=false)
    expect(rafCallbacks.length).toBe(cbCount);
  });

  it('onAudioEnd zeroes visemes on the ASM', () => {
    const tts = createMockTts();
    const asm = createMockAsm();
    useLipSyncBridge(tts, () => asm);

    // Simulate audio end
    tts._audioEndCb!();
    expect(asm.zeroVisemes).toHaveBeenCalled();
  });

  it('onPlaybackStop zeroes visemes on the ASM', () => {
    const tts = createMockTts();
    const asm = createMockAsm();
    useLipSyncBridge(tts, () => asm);

    // Simulate playback stop
    tts._playbackStopCb!();
    expect(asm.zeroVisemes).toHaveBeenCalled();
  });

  it('onAudioEnd with null ASM does not throw', () => {
    const tts = createMockTts();
    useLipSyncBridge(tts, () => null);

    expect(() => tts._audioEndCb!()).not.toThrow();
  });

  it('onAudioStart creates audio graph (no throw)', () => {
    const tts = createMockTts();
    const asm = createMockAsm();
    useLipSyncBridge(tts, () => asm);

    const fakeAudio = {} as HTMLAudioElement;
    expect(() => tts._audioStartCb!(fakeAudio)).not.toThrow();
  });
});
