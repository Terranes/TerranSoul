/**
 * Pinia store for window mode management (window vs pet mode).
 */
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { WindowMode, MonitorInfo } from '../types';

export const useWindowStore = defineStore('window', () => {
  const mode = ref<WindowMode>('window');
  const monitors = ref<MonitorInfo[]>([]);
  const error = ref<string | null>(null);
  const isLoading = ref(false);
  /** True when running a dev/debug build (separate MCP port, DEV badge). */
  const isDevBuild = ref(false);

  /** Query the backend for the build profile (dev vs release). */
  async function loadDevBuildFlag(): Promise<boolean> {
    try {
      const dev = await invoke<boolean>('is_dev_build');
      isDevBuild.value = dev;
      return dev;
    } catch {
      // Tauri unavailable (browser/test) — assume dev in that case
      isDevBuild.value = true;
      return true;
    }
  }

  async function loadMode(): Promise<WindowMode> {
    error.value = null;
    try {
      const m = await invoke<WindowMode>('get_window_mode');
      mode.value = m;
      return m;
    } catch (err) {
      error.value = String(err);
      return mode.value;
    }
  }

  async function setMode(newMode: WindowMode): Promise<boolean> {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('set_window_mode', { mode: newMode });
      mode.value = newMode;
      // Safety net: every mode change forces cursor passthrough OFF so the
      // window can never get stuck in a click-through state.  Pet mode in
      // this app captures clicks via the transparent overlay instead of OS
      // click-through, and desktop mode must obviously be interactive.
      await ensurePassthroughOff();
      return true;
    } catch (err) {
      // Tauri unavailable (browser / e2e) — fall back to a local mode flip
      // so the UI is still switchable for testing and pure-web builds.
      error.value = String(err);
      mode.value = newMode;
      return true;
    } finally {
      isLoading.value = false;
    }
  }

  async function toggleMode(): Promise<WindowMode> {
    isLoading.value = true;
    error.value = null;
    try {
      const newMode = await invoke<WindowMode>('toggle_window_mode');
      mode.value = newMode;
      await ensurePassthroughOff();
      return newMode;
    } catch (err) {
      // Tauri unavailable — perform a local flip so the UI remains usable
      // in the browser/e2e context where the backend command is missing.
      error.value = String(err);
      const next: WindowMode = mode.value === 'pet' ? 'window' : 'pet';
      mode.value = next;
      return next;
    } finally {
      isLoading.value = false;
    }
  }

  /** Fire-and-forget passthrough reset. */
  async function ensurePassthroughOff() {
    try {
      await invoke('set_cursor_passthrough', { ignore: false });
      await invoke('stop_pet_cursor_poll');
    } catch {
      // Tauri unavailable or command missing — no-op
    }
  }

  async function setCursorPassthrough(ignore: boolean): Promise<boolean> {
    error.value = null;
    try {
      await invoke('set_cursor_passthrough', { ignore });
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  async function loadMonitors(): Promise<MonitorInfo[]> {
    error.value = null;
    try {
      const m = await invoke<MonitorInfo[]>('get_all_monitors');
      monitors.value = m;
      return m;
    } catch (err) {
      error.value = String(err);
      return [];
    }
  }

  async function spanAllMonitors(): Promise<boolean> {
    error.value = null;
    try {
      await invoke('set_pet_mode_bounds');
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  function clearError() {
    error.value = null;
  }

  async function startWindowDrag(): Promise<boolean> {
    try {
      await invoke('start_window_drag');
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  async function setPetWindowSize(width: number, height: number): Promise<boolean> {
    try {
      await invoke('set_pet_window_size', { width, height });
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  async function startPetCursorPoll(): Promise<boolean> {
    try {
      await invoke('start_pet_cursor_poll');
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  async function stopPetCursorPoll(): Promise<boolean> {
    try {
      await invoke('stop_pet_cursor_poll');
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  return {
    mode,
    monitors,
    error,
    isLoading,
    isDevBuild,
    loadMode,
    loadDevBuildFlag,
    setMode,
    toggleMode,
    setCursorPassthrough,
    loadMonitors,
    spanAllMonitors,
    startWindowDrag,
    setPetWindowSize,
    startPetCursorPoll,
    stopPetCursorPoll,
    clearError,
  };
});
