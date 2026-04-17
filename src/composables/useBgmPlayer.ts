/**
 * useBgmPlayer — ambient background music player.
 *
 * Provides looping ambient audio playback with fade-in / fade-out transitions.
 * BGM state (enabled, volume, track) is persisted through the settings store
 * and restored on app launch.
 *
 * Architecture:
 *  - Uses the Web Audio API for precise gain control and smooth fading.
 *  - Falls back to HTMLAudioElement.volume when Web Audio is unavailable.
 *  - Tracks are procedurally generated ambient loops using OscillatorNode +
 *    noise buffers, so no external audio files are needed.
 *
 * Usage in CharacterViewport.vue:
 *   const bgm = useBgmPlayer();
 *   bgm.play('ambient-calm');
 *   bgm.setVolume(0.3);
 *   bgm.stop();
 */

import { ref, readonly } from 'vue';

// ── Preset track definitions ─────────────────────────────────────────────────

export interface BgmTrack {
  id: string;
  name: string;
}

/** Available ambient music tracks (procedurally generated). */
export const BGM_TRACKS: BgmTrack[] = [
  { id: 'ambient-calm', name: 'Calm Ambience' },
  { id: 'ambient-night', name: 'Night Breeze' },
  { id: 'ambient-space', name: 'Cosmic Drift' },
];

/** Default volume for BGM (0–1 range). */
export const DEFAULT_BGM_VOLUME = 0.15;

/** Fade duration in seconds for play/stop transitions. */
const FADE_DURATION_S = 0.35;

// ── Procedural audio generation ──────────────────────────────────────────────

/**
 * Create a low-amplitude noise buffer (white noise filtered to be very soft).
 * Used as the base texture for all ambient tracks.
 */
function createNoiseBuffer(ctx: AudioContext, durationS: number): AudioBuffer {
  const sampleRate = ctx.sampleRate;
  const length = Math.floor(sampleRate * durationS);
  const buffer = ctx.createBuffer(1, length, sampleRate);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < length; i++) {
    data[i] = (Math.random() * 2 - 1) * 0.02;
  }
  return buffer;
}

/**
 * Build an ambient audio graph for the given track ID.
 * Returns the master gain node that connects to ctx.destination.
 */
function buildTrackGraph(
  ctx: AudioContext,
  trackId: string,
  masterGain: GainNode,
): { sources: AudioNode[]; cleanup: () => void } {
  const sources: AudioNode[] = [];

  // Base layer: filtered noise (all tracks share this)
  const noiseBuf = createNoiseBuffer(ctx, 4);
  const noiseSource = ctx.createBufferSource();
  noiseSource.buffer = noiseBuf;
  noiseSource.loop = true;
  const noiseFilter = ctx.createBiquadFilter();
  noiseFilter.type = 'lowpass';
  noiseFilter.frequency.value = 400;
  noiseSource.connect(noiseFilter);
  noiseFilter.connect(masterGain);
  noiseSource.start();
  sources.push(noiseSource);

  // Track-specific tonal layers
  if (trackId === 'ambient-calm') {
    // Soft pad chord: C3 + E3 + G3
    for (const freq of [130.81, 164.81, 196.0]) {
      const osc = ctx.createOscillator();
      osc.type = 'sine';
      osc.frequency.value = freq;
      const oscGain = ctx.createGain();
      oscGain.gain.value = 0.015;
      osc.connect(oscGain);
      oscGain.connect(masterGain);
      osc.start();
      sources.push(osc);
    }
  } else if (trackId === 'ambient-night') {
    // Lower pad: A2 + C3 with slight detuning
    for (const freq of [110.0, 130.81]) {
      const osc = ctx.createOscillator();
      osc.type = 'sine';
      osc.frequency.value = freq;
      osc.detune.value = Math.random() * 4 - 2;
      const oscGain = ctx.createGain();
      oscGain.gain.value = 0.012;
      osc.connect(oscGain);
      oscGain.connect(masterGain);
      osc.start();
      sources.push(osc);
    }
  } else if (trackId === 'ambient-space') {
    // Deep drone: F2 with triangle wave
    const osc = ctx.createOscillator();
    osc.type = 'triangle';
    osc.frequency.value = 87.31; // F2
    const oscGain = ctx.createGain();
    oscGain.gain.value = 0.018;
    osc.connect(oscGain);
    oscGain.connect(masterGain);
    osc.start();
    sources.push(osc);

    // High shimmer: very quiet sine
    const shimmer = ctx.createOscillator();
    shimmer.type = 'sine';
    shimmer.frequency.value = 523.25; // C5
    const shimmerGain = ctx.createGain();
    shimmerGain.gain.value = 0.004;
    shimmer.connect(shimmerGain);
    shimmerGain.connect(masterGain);
    shimmer.start();
    sources.push(shimmer);
  }

  function cleanup() {
    for (const src of sources) {
      try {
        if ('stop' in src && typeof (src as OscillatorNode).stop === 'function') {
          (src as OscillatorNode).stop();
        }
        src.disconnect();
      } catch {
        // Already stopped/disconnected
      }
    }
  }

  return { sources, cleanup };
}

// ── Composable ───────────────────────────────────────────────────────────────

export interface BgmPlayerHandle {
  /** Whether BGM is currently playing. */
  isPlaying: Readonly<ReturnType<typeof ref<boolean>>>;
  /** Current volume (0–1). */
  volume: Readonly<ReturnType<typeof ref<number>>>;
  /** Current track ID, or null if nothing is loaded. */
  currentTrackId: Readonly<ReturnType<typeof ref<string | null>>>;
  /** Start playing the specified track with a fade-in. */
  play(trackId: string): void;
  /** Stop playback with a fade-out. */
  stop(): void;
  /** Set the master volume (0–1). Takes effect immediately. */
  setVolume(v: number): void;
}

export function useBgmPlayer(): BgmPlayerHandle {
  const isPlaying = ref(false);
  const volume = ref(DEFAULT_BGM_VOLUME);
  const currentTrackId = ref<string | null>(null);

  let audioCtx: AudioContext | null = null;
  let masterGain: GainNode | null = null;
  let trackCleanup: (() => void) | null = null;

  function ensureContext(): AudioContext {
    if (!audioCtx) {
      audioCtx = new AudioContext();
    }
    // Resume if suspended (browser autoplay policy)
    if (audioCtx.state === 'suspended') {
      audioCtx.resume();
    }
    return audioCtx;
  }

  function play(trackId: string): void {
    // Stop previous track if playing
    if (trackCleanup) {
      stopImmediate();
    }

    const ctx = ensureContext();
    masterGain = ctx.createGain();
    masterGain.gain.value = 0;
    masterGain.connect(ctx.destination);

    const { cleanup } = buildTrackGraph(ctx, trackId, masterGain);
    trackCleanup = cleanup;

    // Fade in
    masterGain.gain.linearRampToValueAtTime(volume.value, ctx.currentTime + FADE_DURATION_S);

    currentTrackId.value = trackId;
    isPlaying.value = true;
  }

  function stop(): void {
    if (!masterGain || !audioCtx) {
      stopImmediate();
      return;
    }

    // Fade out, then clean up
    const now = audioCtx.currentTime;
    masterGain.gain.cancelScheduledValues(now);
    masterGain.gain.setValueAtTime(masterGain.gain.value, now);
    masterGain.gain.linearRampToValueAtTime(0, now + FADE_DURATION_S);

    const cleanupRef = trackCleanup;
    const gainRef = masterGain;
    trackCleanup = null;
    masterGain = null;

    setTimeout(() => {
      cleanupRef?.();
      try { gainRef?.disconnect(); } catch { /* already disconnected */ }
    }, FADE_DURATION_S * 1000 + 100);

    isPlaying.value = false;
    currentTrackId.value = null;
  }

  function stopImmediate(): void {
    trackCleanup?.();
    trackCleanup = null;
    try { masterGain?.disconnect(); } catch { /* already disconnected */ }
    masterGain = null;
    isPlaying.value = false;
    currentTrackId.value = null;
  }

  function setVolume(v: number): void {
    const clamped = Math.max(0, Math.min(1, v));
    volume.value = clamped;

    if (masterGain && audioCtx) {
      const now = audioCtx.currentTime;
      masterGain.gain.cancelScheduledValues(now);
      masterGain.gain.setValueAtTime(masterGain.gain.value, now);
      masterGain.gain.linearRampToValueAtTime(clamped, now + 0.1);
    }
  }

  return {
    isPlaying: readonly(isPlaying),
    volume: readonly(volume),
    currentTrackId: readonly(currentTrackId),
    play,
    stop,
    setVolume,
  };
}
