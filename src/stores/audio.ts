/**
 * Audio store — global mute toggle that silences both background music
 * and TTS voice playback simultaneously.
 *
 * Mute state persists across reloads via `localStorage` so users do not
 * have to re-mute every session.
 *
 * The store is intentionally tiny: BGM playback is muted by routing
 * `bgm.setVolume(0)` from `CharacterViewport.vue` watchers, while TTS
 * playback respects the same flag through the `mutedRef` option of
 * `useTtsPlayback`.
 */
import { defineStore } from 'pinia';
import { ref } from 'vue';

const STORAGE_KEY = 'terransoul.audio.muted';

function readPersisted(): boolean {
  try {
    return globalThis.localStorage?.getItem(STORAGE_KEY) === 'true';
  } catch {
    return false;
  }
}

function writePersisted(value: boolean): void {
  try {
    globalThis.localStorage?.setItem(STORAGE_KEY, String(value));
  } catch {
    // localStorage may be unavailable (private mode, SSR) — ignore.
  }
}

export const useAudioStore = defineStore('audio', () => {
  const muted = ref<boolean>(readPersisted());

  function setMuted(value: boolean): void {
    if (muted.value === value) return;
    muted.value = value;
    writePersisted(value);
  }

  function toggleMuted(): void {
    setMuted(!muted.value);
  }

  return {
    muted,
    setMuted,
    toggleMuted,
  };
});
