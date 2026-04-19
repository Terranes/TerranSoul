/**
 * Integration tests for the character store.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC without the Rust backend.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useCharacterStore } from './character';

// Mock the Tauri invoke API
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('character store — IPC integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('loadVrm success: calls invoke with correct args and sets vrmPath', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    const store = useCharacterStore();
    await store.loadVrm('/models/avatar.vrm');

    expect(mockInvoke).toHaveBeenCalledWith('load_vrm', { path: '/models/avatar.vrm' });
    expect(store.vrmPath).toBe('/models/avatar.vrm');
    expect(store.loadError).toBeUndefined();
  });

  it('loadVrm error: sets vrmPath even if backend invoke fails', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('File not found'));

    const store = useCharacterStore();
    await store.loadVrm('/bad/path.vrm');

    // vrmPath is set immediately so the frontend can attempt Three.js loading
    expect(store.vrmPath).toBe('/bad/path.vrm');
    // Backend error is swallowed (frontend loading is independent)
    expect(store.loadError).toBeUndefined();
  });

  it('loadVrm clears previous error and metadata before loading', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const store = useCharacterStore();
    store.setLoadError('old error');
    store.setMetadata({ title: 'Old', author: 'Old Author', license: 'MIT' });

    await store.loadVrm('/models/new.vrm');

    expect(store.loadError).toBeUndefined();
    expect(store.vrmMetadata).toBeUndefined();
    expect(store.vrmPath).toBe('/models/new.vrm');
  });

  it('resetCharacter clears all state and resets selectedModelId', () => {
    const store = useCharacterStore();
    store.setState('happy');
    store.setMetadata({ title: 'Test', author: 'Author', license: 'MIT' });
    store.setLoadError('error');

    store.resetCharacter();

    expect(store.vrmPath).toBeUndefined();
    expect(store.vrmMetadata).toBeUndefined();
    expect(store.loadError).toBeUndefined();
    expect(store.isLoading).toBe(false);
    expect(store.selectedModelId).toBe('annabelle');
  });

  it('loadVrm sets isLoading and setLoaded clears it', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    const store = useCharacterStore();
    // isLoading starts true (loading screen shown until first model loads)
    expect(store.isLoading).toBe(true);

    await store.loadVrm('/models/avatar.vrm');

    // Still loading (setLoaded must be called by the viewport after Three.js finishes)
    expect(store.isLoading).toBe(true);

    store.setLoaded();
    expect(store.isLoading).toBe(false);
  });

  it('selectModel loads the correct default model path', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    const store = useCharacterStore();
    await store.selectModel('annabelle');

    expect(mockInvoke).toHaveBeenCalledWith('load_vrm', { path: '/models/default/Annabelle the Sorcerer.vrm' });
    expect(store.vrmPath).toBe('/models/default/Annabelle the Sorcerer.vrm');
    expect(store.selectedModelId).toBe('annabelle');
  });

  it('selectModel ignores unknown model ids', async () => {
    const store = useCharacterStore();
    await store.selectModel('nonexistent');

    expect(mockInvoke).not.toHaveBeenCalled();
    expect(store.vrmPath).toBeUndefined();
  });

  it('loadDefaultModel loads annabelle by default', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    const store = useCharacterStore();
    await store.loadDefaultModel();

    expect(mockInvoke).toHaveBeenCalledWith('load_vrm', { path: '/models/default/Annabelle the Sorcerer.vrm' });
    expect(store.vrmPath).toBe('/models/default/Annabelle the Sorcerer.vrm');
    expect(store.selectedModelId).toBe('annabelle');
  });

  it('defaultModels contains the bundled model list', () => {
    const store = useCharacterStore();
    expect(store.defaultModels.length).toBeGreaterThanOrEqual(2);
    expect(store.defaultModels[0].id).toBe('annabelle');
    expect(store.defaultModels[1].id).toBe('m58');
  });

  it('currentGender returns female for annabelle', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useCharacterStore();
    await store.selectModel('annabelle');
    expect(store.currentGender()).toBe('female');
  });

  it('currentGender returns male for m58', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useCharacterStore();
    await store.selectModel('m58');
    expect(store.currentGender()).toBe('male');
  });

  it('currentGender returns female for genshin', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useCharacterStore();
    await store.selectModel('genshin');
    expect(store.currentGender()).toBe('female');
  });

  it('currentGender defaults to female for unknown model', () => {
    const store = useCharacterStore();
    // Force an unknown model id that bypasses selectModel validation
    store.selectedModelId = 'unknown-id';
    expect(store.currentGender()).toBe('female');
  });

  it('selectModel sets Edge TTS voice for female character', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useCharacterStore();
    await store.selectModel('annabelle');
    expect(mockInvoke).toHaveBeenCalledWith('set_tts_voice', { voiceName: 'en-US-AnaNeural' });
    expect(mockInvoke).toHaveBeenCalledWith('set_tts_prosody', { pitch: 50, rate: 15 });
  });

  it('selectModel sets Edge TTS voice for male character', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useCharacterStore();
    await store.selectModel('m58');
    expect(mockInvoke).toHaveBeenCalledWith('set_tts_voice', { voiceName: 'en-US-AndrewNeural' });
    expect(mockInvoke).toHaveBeenCalledWith('set_tts_prosody', { pitch: -10, rate: 0 });
  });

  it('setLoadError stores and clears error message', () => {
    const store = useCharacterStore();
    expect(store.loadError).toBeUndefined();

    store.setLoadError('Failed to load VRM model');
    expect(store.loadError).toBe('Failed to load VRM model');

    store.setLoadError(undefined);
    expect(store.loadError).toBeUndefined();
  });

  it('loadVrm clears loadError before loading', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useCharacterStore();
    store.setLoadError('previous error');

    await store.loadVrm('/models/test.vrm');

    expect(store.loadError).toBeUndefined();
    expect(store.isLoading).toBe(true);
  });

  it('loadError persists across setLoaded calls', () => {
    const store = useCharacterStore();
    store.setLoadError('Failed to load VRM model');
    store.setLoaded();

    // Error should still be visible after loading overlay dismisses
    expect(store.loadError).toBe('Failed to load VRM model');
    expect(store.isLoading).toBe(false);
  });
});
