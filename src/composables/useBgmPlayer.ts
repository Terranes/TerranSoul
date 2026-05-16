/**
 * useBgmPlayer — ambient background music player.
 *
 * Provides looping audio playback with fade-in / fade-out transitions.
 * BGM state (enabled, volume, track) is persisted through the settings store
 * and restored on app launch.
 *
 * Architecture:
 *  - All tracks (built-in and user-added) are played via HTMLAudioElement.
 *  - Built-in tracks are WAV files shipped in public/audio/.
 *  - Users can add custom tracks from local files or URLs.
 *
 * Usage in CharacterViewport.vue:
 *   const bgm = useBgmPlayer();
 *   bgm.play('prelude');
 *   bgm.setVolume(0.3);
 *   bgm.stop();
 */

import { ref, readonly, computed, type DeepReadonly, type Ref, type ComputedRef } from 'vue';

// ── Preset track definitions ─────────────────────────────────────────────────

export interface BgmTrack {
  id: string;
  name: string;
  /** Audio source URL (public asset path, object-URL, or remote URL). */
  src?: string;
  /** Whether this track can be removed by the user. */
  removable?: boolean;
}

/** Available ambient music tracks (shipped as WAV files in public/audio/). */
export const BGM_TRACKS: BgmTrack[] = [
  { id: 'prelude',   name: 'Crystal Prelude',       src: '/audio/prelude.wav' },
  { id: 'moonflow',  name: 'Dream of Zanarkand',    src: '/audio/moonflow.wav' },
  { id: 'sanctuary', name: 'Promised Land',          src: '/audio/sanctuary.wav' },
];

/** Default volume for BGM (0–1 range). */
export const DEFAULT_BGM_VOLUME = 0.15;

/** Fade duration in milliseconds for play/stop transitions. */
const FADE_DURATION_MS = 350;
const FADE_STEPS = 20;

// ── Composable ───────────────────────────────────────────────────────────────

export interface BgmPlayerHandle {
  /** Whether BGM is currently playing. */
  isPlaying: DeepReadonly<Ref<boolean>>;
  /** Current volume (0–1). */
  volume: DeepReadonly<Ref<number>>;
  /** Current track ID, or null if nothing is loaded. */
  currentTrackId: DeepReadonly<Ref<string | null>>;
  /** All available tracks (presets + user-added). */
  allTracks: ComputedRef<BgmTrack[]>;
  /** Start playing the specified track with a fade-in. */
  play(trackId: string): void;
  /** Stop playback with a fade-out. */
  stop(): void;
  /** Set the master volume (0–1). Takes effect immediately. */
  setVolume(v: number): void;
  /** Add a custom track from a file, video, or URL. Returns the new track ID. */
  addCustomTrack(name: string, src: string): string;
  /** Remove a custom track by ID. Returns true if removed. */
  removeTrack(trackId: string): boolean;
  /** Load persisted custom tracks (call once on startup). */
  loadCustomTracks(tracks: BgmTrack[]): void;
  /** User-added custom tracks (for persistence). */
  customTracks: DeepReadonly<Ref<BgmTrack[]>>;
}

export function useBgmPlayer(): BgmPlayerHandle {
  const isPlaying = ref(false);
  const volume = ref(DEFAULT_BGM_VOLUME);
  const currentTrackId = ref<string | null>(null);
  const customTracks = ref<BgmTrack[]>([]);

  /** All tracks: builtins + custom */
  const allTracks = computed<BgmTrack[]>(() => [...BGM_TRACKS, ...customTracks.value]);

  /** Currently playing HTMLAudioElement. */
  let audioEl: HTMLAudioElement | null = null;
  /** Active fade interval ID. */
  let fadeInterval: ReturnType<typeof setInterval> | null = null;

  /** Cancel any running fade animation. */
  function cancelFade(): void {
    if (fadeInterval !== null) {
      clearInterval(fadeInterval);
      fadeInterval = null;
    }
  }

  /** Stop and release the current audio element. */
  function stopAudio(): void {
    cancelFade();
    if (audioEl) {
      audioEl.pause();
      audioEl.removeAttribute('src');
      audioEl.load();
      audioEl = null;
    }
  }

  /** Fade audio element volume from `from` to `to` over FADE_DURATION_MS. */
  function fade(el: HTMLAudioElement, from: number, to: number, onDone?: () => void): void {
    cancelFade();
    let step = 0;
    const stepMs = FADE_DURATION_MS / FADE_STEPS;
    el.volume = Math.max(0, Math.min(1, from));
    fadeInterval = setInterval(() => {
      step++;
      const progress = step / FADE_STEPS;
      el.volume = Math.max(0, Math.min(1, from + (to - from) * progress));
      if (step >= FADE_STEPS) {
        cancelFade();
        onDone?.();
      }
    }, stepMs);
  }

  function play(trackId: string): void {
    // Stop any previous playback
    stopAudio();

    const track = allTracks.value.find(t => t.id === trackId);
    if (!track?.src) return;

    const audio = new Audio(track.src);
    audio.loop = true;
    audio.volume = 0;
    audioEl = audio;

    audio.play().then(() => {
      fade(audio, 0, volume.value);
    }).catch(() => {
      // Autoplay blocked — will become audible after next user gesture
      audio.volume = volume.value;
    });

    currentTrackId.value = track.id;
    isPlaying.value = true;
  }

  function stop(): void {
    if (!audioEl) {
      isPlaying.value = false;
      currentTrackId.value = null;
      return;
    }

    const el = audioEl;
    fade(el, el.volume, 0, () => {
      el.pause();
      if (audioEl === el) {
        audioEl = null;
      }
    });

    isPlaying.value = false;
    currentTrackId.value = null;
  }

  function setVolume(v: number): void {
    const clamped = Math.max(0, Math.min(1, v));
    volume.value = clamped;
    if (audioEl) {
      audioEl.volume = clamped;
    }
  }

  function addCustomTrack(name: string, src: string): string {
    const id = `custom-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    customTracks.value = [...customTracks.value, { id, name, src, removable: true }];
    return id;
  }

  function removeTrack(trackId: string): boolean {
    const before = customTracks.value.length;
    customTracks.value = customTracks.value.filter(t => t.id !== trackId);
    if (customTracks.value.length < before) {
      if (currentTrackId.value === trackId) {
        stop();
      }
      return true;
    }
    return false;
  }

  function loadCustomTracks(tracks: BgmTrack[]): void {
    customTracks.value = tracks.map(t => ({ ...t, removable: true }));
  }

  return {
    isPlaying: readonly(isPlaying),
    volume: readonly(volume),
    currentTrackId: readonly(currentTrackId),
    allTracks,
    customTracks: readonly(customTracks),
    play,
    stop,
    setVolume,
    addCustomTrack,
    removeTrack,
    loadCustomTracks,
  };
}

// ── App-wide singleton accessor ──────────────────────────────────────────────
// `useBgmPlayer()` is a factory (returns a fresh handle each call) so unit
// tests can isolate state. Production code should share ONE audio element
// across the whole app — otherwise multiple BGM surfaces (e.g. the inline
// SettingsPanel inside CharacterViewport + the global SettingsModal) would
// own independent audio elements and could play overlapping tracks.
// `getSharedBgmPlayer()` lazily instantiates and caches one handle for the
// lifetime of the module (i.e. the app session).

let _sharedBgmPlayer: BgmPlayerHandle | null = null;

/** Get (and lazily create) the app-wide shared BGM player handle. All
 *  production callers should use this instead of `useBgmPlayer()` so the
 *  same audio element drives every BGM control surface. */
export function getSharedBgmPlayer(): BgmPlayerHandle {
  if (!_sharedBgmPlayer) {
    _sharedBgmPlayer = useBgmPlayer();
  }
  return _sharedBgmPlayer;
}

/** Test-only helper: drop the cached singleton so the next
 *  `getSharedBgmPlayer()` call returns a fresh instance. */
export function _resetSharedBgmPlayerForTests(): void {
  _sharedBgmPlayer = null;
}

