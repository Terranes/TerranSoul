/**
 * Tests for the useModelCameraStore composable.
 *
 * Covers per-model save/load, fallback to null, overwriting, and Tauri IPC
 * integration (mocked).
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useModelCameraStore } from './useModelCameraStore';

// ── Tauri IPC mock ────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('useModelCameraStore', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    // Reset shared state between tests
    const store = useModelCameraStore();
    store.positions.value = new Map();
  });

  it('getCameraForModel returns null when no position saved', () => {
    const store = useModelCameraStore();
    expect(store.getCameraForModel('ao')).toBeNull();
  });

  it('saveCameraForModel stores position in memory', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useModelCameraStore();
    await store.saveCameraForModel('ao', 1.5, 3.0);
    const pos = store.getCameraForModel('ao');
    expect(pos).not.toBeNull();
    expect(pos!.azimuth).toBeCloseTo(1.5);
    expect(pos!.distance).toBeCloseTo(3.0);
  });

  it('saveCameraForModel calls invoke with correct arguments', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useModelCameraStore();
    await store.saveCameraForModel('karina', 0.8, 2.5);
    expect(mockInvoke).toHaveBeenCalledWith('save_model_camera_position', {
      modelId: 'karina',
      azimuth: 0.8,
      distance: 2.5,
    });
  });

  it('different models have independent positions', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useModelCameraStore();
    await store.saveCameraForModel('ao', 1.0, 3.0);
    await store.saveCameraForModel('karina', 2.0, 4.0);

    const ao = store.getCameraForModel('ao');
    const karina = store.getCameraForModel('karina');
    expect(ao!.azimuth).toBeCloseTo(1.0);
    expect(karina!.azimuth).toBeCloseTo(2.0);
    expect(ao!.distance).toBeCloseTo(3.0);
    expect(karina!.distance).toBeCloseTo(4.0);
  });

  it('overwriting a position replaces previous values', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useModelCameraStore();
    await store.saveCameraForModel('ao', 1.0, 3.0);
    await store.saveCameraForModel('ao', 2.5, 4.5);

    const pos = store.getCameraForModel('ao');
    expect(pos!.azimuth).toBeCloseTo(2.5);
    expect(pos!.distance).toBeCloseTo(4.5);
  });

  it('loadAll populates positions from Tauri IPC', async () => {
    mockInvoke.mockResolvedValue({
      ao: { azimuth: 0.5, distance: 3.2 },
      karina: { azimuth: 1.2, distance: 2.0 },
    });
    const store = useModelCameraStore();
    await store.loadAll();

    expect(store.getCameraForModel('ao')).toEqual({
      azimuth: 0.5,
      distance: 3.2,
    });
    expect(store.getCameraForModel('karina')).toEqual({
      azimuth: 1.2,
      distance: 2.0,
    });
  });

  it('loadAll calls get_model_camera_positions command', async () => {
    mockInvoke.mockResolvedValue({});
    const store = useModelCameraStore();
    await store.loadAll();
    expect(mockInvoke).toHaveBeenCalledWith('get_model_camera_positions');
  });

  it('loadAll keeps current state when Tauri is unavailable', async () => {
    mockInvoke.mockRejectedValue(new Error('Tauri not available'));
    const store = useModelCameraStore();
    // Pre-populate a position
    store.positions.value = new Map([
      ['ao', { azimuth: 0.5, distance: 3.0 }],
    ]);
    await store.loadAll();
    // Should still have the pre-existing position
    expect(store.getCameraForModel('ao')).toEqual({
      azimuth: 0.5,
      distance: 3.0,
    });
  });

  it('saveCameraForModel updates memory even when Tauri is unavailable', async () => {
    mockInvoke.mockRejectedValue(new Error('no Tauri'));
    const store = useModelCameraStore();
    await store.saveCameraForModel('ao', 1.0, 3.0);
    const pos = store.getCameraForModel('ao');
    expect(pos).not.toBeNull();
    expect(pos!.azimuth).toBeCloseTo(1.0);
  });

  it('getCameraForModel returns null for unknown model after loadAll', async () => {
    mockInvoke.mockResolvedValue({
      ao: { azimuth: 0.5, distance: 3.0 },
    });
    const store = useModelCameraStore();
    await store.loadAll();
    expect(store.getCameraForModel('nonexistent')).toBeNull();
  });

  it('loadAll replaces existing positions with backend data', async () => {
    const store = useModelCameraStore();
    // Pre-populate with local-only data
    store.positions.value = new Map([
      ['old-model', { azimuth: 9.9, distance: 9.9 }],
    ]);

    mockInvoke.mockResolvedValue({
      ao: { azimuth: 0.5, distance: 3.0 },
    });
    await store.loadAll();

    // Old data replaced
    expect(store.getCameraForModel('old-model')).toBeNull();
    expect(store.getCameraForModel('ao')).not.toBeNull();
  });
});

// ── IPC Contract Tests ─────────────────────────────────────────────────────

describe('useModelCameraStore — IPC contract', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('save sends modelId (camelCase)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useModelCameraStore();
    await store.saveCameraForModel('ao', 0.5, 3.0);
    expect(mockInvoke).toHaveBeenCalledWith('save_model_camera_position', {
      modelId: 'ao',
      azimuth: 0.5,
      distance: 3.0,
    });
  });
});
