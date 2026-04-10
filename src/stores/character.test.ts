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

  it('loadVrm error: sets loadError and does not change vrmPath', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('File not found'));

    const store = useCharacterStore();
    await store.loadVrm('/bad/path.vrm');

    expect(store.loadError).toContain('File not found');
    expect(store.vrmPath).toBeUndefined();
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

  it('resetCharacter clears all state', () => {
    const store = useCharacterStore();
    store.setState('happy');
    store.setMetadata({ title: 'Test', author: 'Author', license: 'MIT' });
    store.setLoadError('error');

    store.resetCharacter();

    expect(store.vrmPath).toBeUndefined();
    expect(store.vrmMetadata).toBeUndefined();
    expect(store.loadError).toBeUndefined();
  });
});
