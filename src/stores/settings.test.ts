/**
 * Tests for useSettingsStore.
 *
 * Covers loading defaults, Tauri IPC integration, partial saves, convenience
 * helpers (saveModelId, saveCameraState), and error resilience.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useSettingsStore, type AppSettings } from './settings';

// ── Tauri IPC mock ────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const SCHEMA_V1_SETTINGS: AppSettings = {
  version: 2,
  selected_model_id: 'komori',
  camera_azimuth: 0.5,
  camera_distance: 3.2,
  bgm_enabled: false,
  bgm_volume: 0.15,
  bgm_track_id: 'prelude',
  bgm_custom_tracks: [],
  max_memory_gb: 10,
};

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('useSettingsStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('starts with default settings', () => {
    const store = useSettingsStore();
    expect(store.settings.selected_model_id).toBe('shinra');
    expect(store.settings.camera_azimuth).toBe(0);
    expect(store.settings.camera_distance).toBeCloseTo(2.8);
    expect(store.settings.version).toBe(2);
    expect(store.settings.bgm_enabled).toBe(false);
    expect(store.settings.bgm_volume).toBeCloseTo(0.15);
    expect(store.settings.bgm_track_id).toBe('prelude');
    expect(store.settings.max_memory_gb).toBe(10);
  });

  it('loadSettings populates settings from Tauri IPC', async () => {
    mockInvoke.mockResolvedValue(SCHEMA_V1_SETTINGS);
    const store = useSettingsStore();
    await store.loadSettings();
    expect(store.settings.selected_model_id).toBe('komori');
    expect(store.settings.camera_azimuth).toBeCloseTo(0.5);
    expect(store.settings.camera_distance).toBeCloseTo(3.2);
  });

  it('loadSettings falls back to defaults when Tauri unavailable', async () => {
    mockInvoke.mockRejectedValue(new Error('Tauri not available'));
    const store = useSettingsStore();
    await store.loadSettings();
    // Should still have defaults
    expect(store.settings.selected_model_id).toBe('shinra');
    expect(store.error).toBeNull();
  });

  it('loadSettings clears isLoading on completion', async () => {
    mockInvoke.mockResolvedValue(SCHEMA_V1_SETTINGS);
    const store = useSettingsStore();
    const p = store.loadSettings();
    expect(store.isLoading).toBe(true);
    await p;
    expect(store.isLoading).toBe(false);
  });

  it('saveSettings patches and calls invoke', async () => {
    mockInvoke.mockResolvedValue({});
    const store = useSettingsStore();
    await store.saveSettings({ selected_model_id: 'komori' });
    expect(store.settings.selected_model_id).toBe('komori');
    expect(mockInvoke).toHaveBeenCalledWith('save_app_settings', {
      settings: expect.objectContaining({ selected_model_id: 'komori' }),
    });
  });

  it('saveSettings updates in-memory even when Tauri unavailable', async () => {
    mockInvoke.mockRejectedValue(new Error('no Tauri'));
    const store = useSettingsStore();
    await store.saveSettings({ selected_model_id: 'komori' });
    expect(store.settings.selected_model_id).toBe('komori');
  });

  it('saveModelId persists model selection', async () => {
    mockInvoke.mockResolvedValue({});
    const store = useSettingsStore();
    await store.saveModelId('komori');
    expect(store.settings.selected_model_id).toBe('komori');
    expect(mockInvoke).toHaveBeenCalledWith('save_app_settings', {
      settings: expect.objectContaining({ selected_model_id: 'komori' }),
    });
  });

  it('saveCameraState persists azimuth and distance', async () => {
    mockInvoke.mockResolvedValue({});
    const store = useSettingsStore();
    await store.saveCameraState(1.57, 4.0);
    expect(store.settings.camera_azimuth).toBeCloseTo(1.57);
    expect(store.settings.camera_distance).toBeCloseTo(4.0);
    expect(mockInvoke).toHaveBeenCalledWith('save_app_settings', {
      settings: expect.objectContaining({ camera_azimuth: 1.57, camera_distance: 4.0 }),
    });
  });

  it('saveSettings preserves fields not in the patch', async () => {
    mockInvoke.mockResolvedValue(SCHEMA_V1_SETTINGS);
    const store = useSettingsStore();
    await store.loadSettings();
    // Patch only azimuth
    await store.saveSettings({ camera_azimuth: 2.0 });
    expect(store.settings.selected_model_id).toBe('komori'); // preserved
    expect(store.settings.camera_azimuth).toBeCloseTo(2.0); // patched
  });

  it('saveBgmState persists BGM settings', async () => {
    mockInvoke.mockResolvedValue({});
    const store = useSettingsStore();
    await store.saveBgmState(true, 0.4, 'ambient-night');
    expect(store.settings.bgm_enabled).toBe(true);
    expect(store.settings.bgm_volume).toBeCloseTo(0.4);
    expect(store.settings.bgm_track_id).toBe('ambient-night');
    expect(mockInvoke).toHaveBeenCalledWith('save_app_settings', {
      settings: expect.objectContaining({
        bgm_enabled: true,
        bgm_volume: 0.4,
        bgm_track_id: 'ambient-night',
      }),
    });
  });

  it('default chatbox_mode is false', () => {
    const store = useSettingsStore();
    expect(store.settings.chatbox_mode).toBe(false);
  });

  it('setChatboxMode enables chatbox-only mode', async () => {
    mockInvoke.mockResolvedValue({});
    const store = useSettingsStore();
    await store.setChatboxMode(true);
    expect(store.settings.chatbox_mode).toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith('save_app_settings', {
      settings: expect.objectContaining({ chatbox_mode: true }),
    });
  });

  it('setChatboxMode can disable chatbox mode', async () => {
    mockInvoke.mockResolvedValue({});
    const store = useSettingsStore();
    await store.setChatboxMode(true);
    await store.setChatboxMode(false);
    expect(store.settings.chatbox_mode).toBe(false);
  });

  it('saveMaxMemoryGb persists a clamped memory cap', async () => {
    mockInvoke.mockResolvedValue({});
    const store = useSettingsStore();
    await store.saveMaxMemoryGb(12.5);
    expect(store.settings.max_memory_gb).toBe(12.5);
    expect(mockInvoke).toHaveBeenCalledWith('save_app_settings', {
      settings: expect.objectContaining({ max_memory_gb: 12.5 }),
    });
  });
});
