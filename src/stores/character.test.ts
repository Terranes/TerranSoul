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
    expect(store.selectedModelId).toBe('shinra');
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
    await store.selectModel('shinra');

    expect(mockInvoke).toHaveBeenCalledWith('load_vrm', { path: '/models/default/Shinra.vrm' });
    expect(store.vrmPath).toBe('/models/default/Shinra.vrm');
    expect(store.selectedModelId).toBe('shinra');
  });

  it('selectModel ignores unknown model ids', async () => {
    const store = useCharacterStore();
    await store.selectModel('nonexistent');

    expect(mockInvoke).not.toHaveBeenCalled();
    expect(store.vrmPath).toBeUndefined();
  });

  it('loadDefaultModel loads ao by default', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    const store = useCharacterStore();
    await store.loadDefaultModel();

    expect(mockInvoke).toHaveBeenCalledWith('load_vrm', { path: '/models/default/Shinra.vrm' });
    expect(store.vrmPath).toBe('/models/default/Shinra.vrm');
    expect(store.selectedModelId).toBe('shinra');
  });

  it('defaultModels contains the bundled model list', () => {
    const store = useCharacterStore();
    expect(store.defaultModels.length).toBeGreaterThanOrEqual(2);
    expect(store.defaultModels[0].id).toBe('shinra');
    expect(store.defaultModels[1].id).toBe('komori');
  });

  it('currentGender returns female for ao', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useCharacterStore();
    await store.selectModel('shinra');
    expect(store.currentGender()).toBe('female');
  });

  it('currentGender returns female for komori', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useCharacterStore();
    await store.selectModel('komori');
    expect(store.currentGender()).toBe('female');
  });

  it('currentGender returns female for unknown / removed model', () => {
    const store = useCharacterStore();
    // Force an unknown model id (e.g. one that was removed from defaults)
    store.selectedModelId = 'genshin';
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
    await store.selectModel('shinra');
    expect(mockInvoke).toHaveBeenCalledWith('set_tts_voice', { voiceName: 'en-US-AnaNeural' });
    expect(mockInvoke).toHaveBeenCalledWith('set_tts_prosody', { pitch: 50, rate: 15 });
  });

  it('selectModel sets Edge TTS voice for komori character', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useCharacterStore();
    await store.selectModel('komori');
    expect(mockInvoke).toHaveBeenCalledWith('set_tts_voice', { voiceName: 'en-US-AnaNeural' });
    expect(mockInvoke).toHaveBeenCalledWith('set_tts_prosody', { pitch: 50, rate: 15 });
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

describe('character store — user-imported models', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    // Provide minimal Blob URL stubs for jsdom — vitest's environment
    // exposes URL.createObjectURL but blob:url -> data fetching isn't needed
    // here since we only assert the path is set to a blob: URL.
    if (!('createObjectURL' in URL)) {
      // @ts-expect-error — jsdom polyfill
      URL.createObjectURL = () => 'blob:mock';
    }
    if (!('revokeObjectURL' in URL)) {
      // @ts-expect-error — jsdom polyfill
      URL.revokeObjectURL = () => {};
    }
  });

  it('loadUserModels populates userModels from backend', async () => {
    const fixtures = [
      { id: 'u-1', name: 'Aria', original_filename: 'aria.vrm', gender: 'female', imported_at: 1 },
      { id: 'u-2', name: 'Kai',  original_filename: 'kai.vrm',  gender: 'male',   imported_at: 2 },
    ];
    mockInvoke.mockResolvedValueOnce(fixtures);
    const store = useCharacterStore();
    await store.loadUserModels();
    expect(mockInvoke).toHaveBeenCalledWith('list_user_models');
    expect(store.userModels).toHaveLength(2);
    expect(store.userModels[0].name).toBe('Aria');
  });

  it('loadUserModels swallows backend error and keeps empty list', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('no Tauri'));
    const store = useCharacterStore();
    await store.loadUserModels();
    expect(store.userModels).toEqual([]);
  });

  it('importUserModel calls backend and appends to userModels', async () => {
    const entry = { id: 'u-9', name: 'New', original_filename: 'new.vrm', gender: 'female', imported_at: 99 };
    mockInvoke.mockResolvedValueOnce(entry);
    const store = useCharacterStore();
    const result = await store.importUserModel('/abs/path/new.vrm');
    expect(mockInvoke).toHaveBeenCalledWith('import_user_model', { sourcePath: '/abs/path/new.vrm' });
    expect(result).toEqual(entry);
    expect(store.userModels).toHaveLength(1);
    expect(store.userModels[0].id).toBe('u-9');
  });

  it('selectModel loads a user model via blob URL', async () => {
    const entry = { id: 'u-3', name: 'Imp', original_filename: 'imp.vrm', gender: 'female', imported_at: 1 };
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'list_user_models') return Promise.resolve([entry]);
      if (cmd === 'read_user_model_bytes') return Promise.resolve(new Uint8Array([1, 2, 3]));
      return Promise.resolve(undefined);
    });
    const store = useCharacterStore();
    await store.loadUserModels();
    await store.selectModel('u-3');
    expect(mockInvoke).toHaveBeenCalledWith('read_user_model_bytes', { id: 'u-3' });
    expect(store.selectedModelId).toBe('u-3');
    expect(store.vrmPath).toMatch(/^blob:/);
  });

  it('deleteUserModel removes entry and falls back to default if active', async () => {
    const entry = { id: 'u-4', name: 'Gone', original_filename: 'gone.vrm', gender: 'female', imported_at: 1 };
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'list_user_models') return Promise.resolve([entry]);
      if (cmd === 'read_user_model_bytes') return Promise.resolve(new Uint8Array([0]));
      // delete_user_model + load_vrm + tts setters + save_app_settings
      return Promise.resolve(undefined);
    });
    const store = useCharacterStore();
    await store.loadUserModels();
    await store.selectModel('u-4');
    expect(store.selectedModelId).toBe('u-4');
    await store.deleteUserModel('u-4');
    expect(mockInvoke).toHaveBeenCalledWith('delete_user_model', { id: 'u-4' });
    expect(store.userModels).toEqual([]);
    expect(store.selectedModelId).toBe('shinra');
  });

  it('allModels concatenates defaults and user models', async () => {
    const entry = { id: 'u-5', name: 'Extra', original_filename: 'extra.vrm', gender: 'female', imported_at: 1 };
    mockInvoke.mockResolvedValueOnce([entry]);
    const store = useCharacterStore();
    await store.loadUserModels();
    expect(store.allModels.length).toBe(store.defaultModels.length + 1);
    expect(store.allModels[store.allModels.length - 1].id).toBe('u-5');
  });

  it('currentGender works for user models', async () => {
    const entry = { id: 'u-6', name: 'Bo', original_filename: 'bo.vrm', gender: 'male', imported_at: 1 };
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'list_user_models') return Promise.resolve([entry]);
      if (cmd === 'read_user_model_bytes') return Promise.resolve(new Uint8Array([0]));
      return Promise.resolve(undefined);
    });
    const store = useCharacterStore();
    await store.loadUserModels();
    await store.selectModel('u-6');
    expect(store.currentGender()).toBe('male');
  });
});
