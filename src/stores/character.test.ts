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
});
