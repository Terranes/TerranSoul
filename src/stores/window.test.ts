/**
 * Integration tests for the window store.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useWindowStore } from './window';
import type { MonitorInfo } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const sampleMonitors: MonitorInfo[] = [
  { name: 'Primary', x: 0, y: 0, width: 1920, height: 1080, scale_factor: 1.0 },
  { name: 'Secondary', x: 1920, y: 0, width: 2560, height: 1440, scale_factor: 1.25 },
];

describe('window store — IPC integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('loadMode fetches current window mode', async () => {
    mockInvoke.mockResolvedValue('pet');
    const store = useWindowStore();
    const result = await store.loadMode();
    expect(mockInvoke).toHaveBeenCalledWith('get_window_mode');
    expect(result).toBe('pet');
    expect(store.mode).toBe('pet');
  });

  it('loadMode defaults to window on error', async () => {
    mockInvoke.mockRejectedValue(new Error('not available'));
    const store = useWindowStore();
    const result = await store.loadMode();
    expect(result).toBe('window');
    expect(store.error).toBeTruthy();
  });

  it('setMode sends correct IPC command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useWindowStore();
    const success = await store.setMode('pet');
    expect(mockInvoke).toHaveBeenCalledWith('set_window_mode', { mode: 'pet' });
    expect(success).toBe(true);
    expect(store.mode).toBe('pet');
    expect(store.isLoading).toBe(false);
  });

  it('setMode falls back to local flip when Tauri is unavailable', async () => {
    // Browser / e2e path: invoke throws because the Tauri backend is missing.
    // The store should still flip the local mode so the UI is switchable,
    // and record the error so callers can surface it if they want to.
    mockInvoke.mockRejectedValue(new Error('window error'));
    const store = useWindowStore();
    const success = await store.setMode('pet');
    expect(success).toBe(true);
    expect(store.error).toBe('Error: window error');
    expect(store.mode).toBe('pet'); // local flip applied
  });

  it('toggleMode toggles and returns new mode', async () => {
    mockInvoke.mockResolvedValue('pet');
    const store = useWindowStore();
    const result = await store.toggleMode();
    expect(mockInvoke).toHaveBeenCalledWith('toggle_window_mode');
    expect(result).toBe('pet');
    expect(store.mode).toBe('pet');
  });

  it('toggleMode falls back to local flip when Tauri is unavailable', async () => {
    mockInvoke.mockRejectedValue(new Error('toggle error'));
    const store = useWindowStore();
    // First call: window → pet locally.
    const first = await store.toggleMode();
    expect(first).toBe('pet');
    expect(store.mode).toBe('pet');
    // Second call: pet → window locally.
    const second = await store.toggleMode();
    expect(second).toBe('window');
    expect(store.mode).toBe('window');
    expect(store.error).toBeTruthy();
  });

  it('setCursorPassthrough sends ignore flag', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useWindowStore();
    const ok = await store.setCursorPassthrough(true);
    expect(mockInvoke).toHaveBeenCalledWith('set_cursor_passthrough', { ignore: true });
    expect(ok).toBe(true);
  });

  it('setCursorPassthrough sends false', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useWindowStore();
    await store.setCursorPassthrough(false);
    expect(mockInvoke).toHaveBeenCalledWith('set_cursor_passthrough', { ignore: false });
  });

  it('setCursorPassthrough handles failure', async () => {
    mockInvoke.mockRejectedValue(new Error('passthrough error'));
    const store = useWindowStore();
    const ok = await store.setCursorPassthrough(true);
    expect(ok).toBe(false);
    expect(store.error).toBeTruthy();
  });

  it('loadMonitors fetches monitor list', async () => {
    mockInvoke.mockResolvedValue(sampleMonitors);
    const store = useWindowStore();
    const result = await store.loadMonitors();
    expect(mockInvoke).toHaveBeenCalledWith('get_all_monitors');
    expect(result).toEqual(sampleMonitors);
    expect(store.monitors).toEqual(sampleMonitors);
  });

  it('loadMonitors handles failure', async () => {
    mockInvoke.mockRejectedValue(new Error('no monitors'));
    const store = useWindowStore();
    const result = await store.loadMonitors();
    expect(result).toEqual([]);
    expect(store.error).toBeTruthy();
  });

  it('spanAllMonitors sends IPC command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useWindowStore();
    const ok = await store.spanAllMonitors();
    expect(mockInvoke).toHaveBeenCalledWith('set_pet_mode_bounds');
    expect(ok).toBe(true);
  });

  it('spanAllMonitors handles failure', async () => {
    mockInvoke.mockRejectedValue(new Error('span error'));
    const store = useWindowStore();
    const ok = await store.spanAllMonitors();
    expect(ok).toBe(false);
    expect(store.error).toBeTruthy();
  });

  it('clearError resets error', async () => {
    mockInvoke.mockRejectedValue(new Error('test error'));
    const store = useWindowStore();
    await store.loadMode();
    expect(store.error).toBeTruthy();
    store.clearError();
    expect(store.error).toBeNull();
  });

  it('initial state is window mode', () => {
    const store = useWindowStore();
    expect(store.mode).toBe('window');
    expect(store.monitors).toEqual([]);
    expect(store.error).toBeNull();
    expect(store.isLoading).toBe(false);
  });
});
