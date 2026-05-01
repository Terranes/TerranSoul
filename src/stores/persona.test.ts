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

  it('round-trips exampleDialogue through save and load', async () => {
    const store = usePersonaStore();
    await store.saveTraits({
      exampleDialogue: ['User: Hi / Assistant: Hello there!'],
    });

    setActivePinia(createPinia());
    const fresh = usePersonaStore();
    await fresh.load();
    expect(fresh.traits.exampleDialogue).toEqual(['User: Hi / Assistant: Hello there!']);
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

  // ── Master-Echo brain-extraction loop (Chunk 14.2) ────────────────────

  it('suggestPersonaFromBrain returns null when Tauri is unavailable', async () => {
    const store = usePersonaStore();
    // Default mock throws — same as a non-Tauri / no-brain context.
    const suggestion = await store.suggestPersonaFromBrain();
    expect(suggestion).toBeNull();
    // Failed extraction must NOT stamp lastBrainExtractedAt.
    expect(store.lastBrainExtractedAt).toBeNull();
  });

  it('suggestPersonaFromBrain returns null and skips stamp when reply is empty', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as ReturnType<typeof vi.fn>).mockImplementationOnce(async () => '');
    const store = usePersonaStore();
    const suggestion = await store.suggestPersonaFromBrain();
    expect(suggestion).toBeNull();
    expect(store.lastBrainExtractedAt).toBeNull();
  });

  it('suggestPersonaFromBrain returns null and skips stamp when JSON is malformed', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as ReturnType<typeof vi.fn>).mockImplementationOnce(async () => 'not json');
    const store = usePersonaStore();
    expect(await store.suggestPersonaFromBrain()).toBeNull();
    expect(store.lastBrainExtractedAt).toBeNull();
  });

  it('suggestPersonaFromBrain returns null when required fields are missing', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as ReturnType<typeof vi.fn>).mockImplementationOnce(async () =>
      JSON.stringify({ name: 'Lia', role: '   ' }),
    );
    const store = usePersonaStore();
    expect(await store.suggestPersonaFromBrain()).toBeNull();
    expect(store.lastBrainExtractedAt).toBeNull();
  });

  it('suggestPersonaFromBrain returns parsed candidate and stamps timestamp on success', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as ReturnType<typeof vi.fn>).mockImplementationOnce(async () =>
      JSON.stringify({
        name: 'Lia',
        role: 'librarian',
        bio: 'Quiet bookworm.',
        tone: ['warm', 'concise'],
        quirks: ['hums'],
        avoid: ['medical advice'],
      }),
    );
    const store = usePersonaStore();
    const before = Date.now();
    const suggestion = await store.suggestPersonaFromBrain();
    expect(suggestion).not.toBeNull();
    expect(suggestion!.name).toBe('Lia');
    expect(suggestion!.tone).toEqual(['warm', 'concise']);
    expect(store.lastBrainExtractedAt).not.toBeNull();
    expect(store.lastBrainExtractedAt!).toBeGreaterThanOrEqual(before);
  });

  it('suggestPersonaFromBrain drops non-string list entries from the candidate', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as ReturnType<typeof vi.fn>).mockImplementationOnce(async () =>
      JSON.stringify({
        name: 'Lia',
        role: 'librarian',
        bio: 'Quiet bookworm.',
        tone: ['warm', 7, null, 'concise'],
        quirks: 'not-an-array',
      }),
    );
    const store = usePersonaStore();
    const suggestion = await store.suggestPersonaFromBrain();
    expect(suggestion).not.toBeNull();
    expect(suggestion!.tone).toEqual(['warm', 'concise']);
    // `quirks` was the wrong shape → safely defaulted to [].
    expect(suggestion!.quirks).toEqual([]);
  });

  // ── Persona pack export / import (Chunk 14.7) ────────────────────────

  it('exportPack returns null when Tauri is unavailable', async () => {
    const store = usePersonaStore();
    const out = await store.exportPack('a note');
    expect(out).toBeNull();
  });

  it('exportPack returns the JSON string returned by the backend', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as ReturnType<typeof vi.fn>).mockImplementationOnce(
      async () => '{"packVersion":1,"exportedAt":0,"traits":{}}',
    );
    const store = usePersonaStore();
    const out = await store.exportPack('hello');
    expect(out).toContain('"packVersion"');
  });

  it('previewImportPack propagates backend parse errors as thrown', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as ReturnType<typeof vi.fn>).mockImplementationOnce(async () => {
      throw new Error('Pack is not valid JSON: expected value at line 1');
    });
    const store = usePersonaStore();
    await expect(store.previewImportPack('garbage')).rejects.toThrow(/not valid JSON/);
  });

  it('previewImportPack returns the report on success', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as ReturnType<typeof vi.fn>).mockImplementationOnce(async () => ({
      traits_applied: true,
      expressions_accepted: 2,
      motions_accepted: 1,
      skipped: ['expression #2: missing id'],
    }));
    const store = usePersonaStore();
    const r = await store.previewImportPack('{"packVersion":1,"traits":{}}');
    expect(r.traits_applied).toBe(true);
    expect(r.expressions_accepted).toBe(2);
    expect(r.motions_accepted).toBe(1);
    expect(r.skipped).toHaveLength(1);
  });

  it('importPack reloads store state after a successful apply', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const mock = invoke as ReturnType<typeof vi.fn>;
    // 1: import_persona_pack returns the report.
    mock.mockImplementationOnce(async () => ({
      traits_applied: true,
      expressions_accepted: 0,
      motions_accepted: 0,
      skipped: [],
    }));
    // 2: load() → get_persona returns updated traits.
    mock.mockImplementationOnce(async () =>
      JSON.stringify({
        version: 1,
        name: 'Imported',
        role: 'r',
        bio: 'b',
        tone: [],
        quirks: [],
        avoid: [],
        active: true,
        updatedAt: 999,
      }),
    );
    // 3 + 4: list_learned_expressions + list_learned_motions return [].
    mock.mockImplementationOnce(async () => []);
    mock.mockImplementationOnce(async () => []);
    // 5: set_persona_block (syncBlockToBackend) — accept anything.
    mock.mockImplementationOnce(async () => undefined);

    const store = usePersonaStore();
    const r = await store.importPack('{"packVersion":1,"traits":{}}');
    expect(r.traits_applied).toBe(true);
    expect(store.traits.name).toBe('Imported');
  });

  it('importPack surfaces backend errors to the caller', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as ReturnType<typeof vi.fn>).mockImplementationOnce(async () => {
      throw new Error('Pack format version 99 is newer than this build supports');
    });
    const store = usePersonaStore();
    await expect(store.importPack('{}')).rejects.toThrow(/newer than this build/);
  });
});
