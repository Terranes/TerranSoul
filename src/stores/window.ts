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
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
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
      return newMode;
    } catch (err) {
      error.value = String(err);
      return mode.value;
    } finally {
      isLoading.value = false;
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

  return {
    mode,
    monitors,
    error,
    isLoading,
    loadMode,
    setMode,
    toggleMode,
    setCursorPassthrough,
    loadMonitors,
    spanAllMonitors,
    clearError,
  };
});
