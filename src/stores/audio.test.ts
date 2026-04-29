import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useAudioStore } from './audio';

const STORAGE_KEY = 'terransoul.audio.muted';

describe('useAudioStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    globalThis.localStorage?.removeItem(STORAGE_KEY);
  });

  afterEach(() => {
    globalThis.localStorage?.removeItem(STORAGE_KEY);
  });

  it('starts unmuted by default', () => {
    const store = useAudioStore();
    expect(store.muted).toBe(false);
  });

  it('toggleMuted flips the muted flag', () => {
    const store = useAudioStore();
    store.toggleMuted();
    expect(store.muted).toBe(true);
    store.toggleMuted();
    expect(store.muted).toBe(false);
  });

  it('setMuted is idempotent', () => {
    const store = useAudioStore();
    store.setMuted(true);
    store.setMuted(true);
    expect(store.muted).toBe(true);
  });

  it('persists muted state to localStorage', () => {
    const store = useAudioStore();
    store.setMuted(true);
    expect(globalThis.localStorage?.getItem(STORAGE_KEY)).toBe('true');
    store.setMuted(false);
    expect(globalThis.localStorage?.getItem(STORAGE_KEY)).toBe('false');
  });

  it('restores muted state on a fresh pinia from localStorage', () => {
    globalThis.localStorage?.setItem(STORAGE_KEY, 'true');
    setActivePinia(createPinia());
    const store = useAudioStore();
    expect(store.muted).toBe(true);
  });
});
