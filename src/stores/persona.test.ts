/**
 * Unit tests for the persona Pinia store.
 *
 * Focus: persistence round-trip with the localStorage fallback,
 * `hasCustomPersona` activation logic for the `my-persona` quest gate,
 * and the per-session camera-consent contract from
 * `docs/persona-design.md` § 5.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';

// `invoke` is statically imported by the persona store; mock it before
// the store module is evaluated.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => {
    // Default mock: pretend Tauri is unavailable so the store falls back
    // to localStorage. Individual tests can override this with mockImpl.
    throw new Error(`mock: tauri unavailable (${cmd})`);
  }),
}));

import { usePersonaStore } from './persona';
import { defaultPersona } from './persona-types';

describe('usePersonaStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it('starts with the default persona and an empty learned library', () => {
    const store = usePersonaStore();
    expect(store.traits.name).toBe(defaultPersona().name);
    expect(store.learnedExpressions).toEqual([]);
    expect(store.learnedMotions).toEqual([]);
    expect(store.lastBrainExtractedAt).toBeNull();
  });

  it('reports hasCustomPersona = false for the unedited default', () => {
    const store = usePersonaStore();
    expect(store.hasCustomPersona).toBe(false);
  });

  it('flips hasCustomPersona = true once the user changes the name', async () => {
    const store = usePersonaStore();
    await store.saveTraits({ name: 'Lia' });
    expect(store.hasCustomPersona).toBe(true);
  });

  it('persists changes to localStorage and reloads them on a fresh store', async () => {
    const store = usePersonaStore();
    await store.saveTraits({ name: 'Lia', tone: ['playful'] });

    // New pinia → new store instance, same localStorage.
    setActivePinia(createPinia());
    const fresh = usePersonaStore();
    await fresh.load();
    expect(fresh.traits.name).toBe('Lia');
    expect(fresh.traits.tone).toEqual(['playful']);
    expect(fresh.traitsLoaded).toBe(true);
  });

  it('stamps version + updatedAt on every save', async () => {
    const store = usePersonaStore();
    const before = Date.now();
    await store.saveTraits({ bio: 'A new soul.' });
    expect(store.traits.version).toBeGreaterThanOrEqual(1);
    expect(store.traits.updatedAt).toBeGreaterThanOrEqual(before);
    expect(store.traits.bio).toBe('A new soul.');
  });

  it('resetToDefault restores the cold-start defaults', async () => {
    const store = usePersonaStore();
    await store.saveTraits({ name: 'Lia', quirks: ['hums'] });
    expect(store.hasCustomPersona).toBe(true);
    await store.resetToDefault();
    expect(store.traits.name).toBe(defaultPersona().name);
    expect(store.traits.quirks).toEqual([]);
    expect(store.hasCustomPersona).toBe(false);
  });

  it('renders an empty personaBlock when persona is inactive', async () => {
    const store = usePersonaStore();
    await store.saveTraits({ active: false });
    expect(store.personaBlock).toBe('');
  });

  it('renders a non-empty [PERSONA] block when persona is active', () => {
    const store = usePersonaStore();
    expect(store.personaBlock).toContain('[PERSONA]');
    expect(store.personaBlock).toContain(defaultPersona().name);
  });

  it('recordBrainExtraction stamps lastBrainExtractedAt for the master-echo gate', () => {
    const store = usePersonaStore();
    expect(store.lastBrainExtractedAt).toBeNull();
    store.recordBrainExtraction();
    expect(store.lastBrainExtractedAt).not.toBeNull();
    expect(store.lastBrainExtractedAt!).toBeGreaterThan(0);
  });

  // ── Per-session camera consent contract (persona-design.md § 5) ──────

  it('camera session is inactive by default', () => {
    const store = usePersonaStore();
    expect(store.cameraSession.active).toBe(false);
    expect(store.cameraSession.chatId).toBeNull();
    expect(store.cameraSession.startedAt).toBeNull();
  });

  it('startCameraSession records consent for exactly one chat', () => {
    const store = usePersonaStore();
    store.startCameraSession('chat-A');
    expect(store.cameraSession.active).toBe(true);
    expect(store.cameraSession.chatId).toBe('chat-A');
    expect(store.cameraSession.startedAt).toBeGreaterThan(0);
  });

  it('switching chat tears down any prior camera session (hard § 5 guarantee)', () => {
    const store = usePersonaStore();
    store.startCameraSession('chat-A');
    store.startCameraSession('chat-B');
    expect(store.cameraSession.active).toBe(true);
    expect(store.cameraSession.chatId).toBe('chat-B');
  });

  it('stopCameraSession is idempotent and clears all session state', () => {
    const store = usePersonaStore();
    store.startCameraSession('chat-A');
    store.stopCameraSession();
    store.stopCameraSession(); // second call must not throw or revive state
    expect(store.cameraSession.active).toBe(false);
    expect(store.cameraSession.chatId).toBeNull();
    expect(store.cameraSession.startedAt).toBeNull();
  });

  it('camera session state is NEVER written to localStorage', () => {
    const store = usePersonaStore();
    store.startCameraSession('chat-A');
    // Inspect every persisted key the store could touch.
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i)!;
      const val = localStorage.getItem(key) ?? '';
      expect(val.toLowerCase()).not.toContain('camerasession');
      expect(val.toLowerCase()).not.toContain('chatid');
    }
  });
});
