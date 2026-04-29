import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { BgmTrack } from '../composables/useBgmPlayer';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface AppSettings {
  /** Schema version — used for migration/corruption detection. */
  version: number;
  /** ID of the selected character model (maps to DEFAULT_MODELS). */
  selected_model_id: string;
  /** Camera horizontal orbit angle (radians). */
  camera_azimuth: number;
  /** Camera distance from the orbit target (zoom level). */
  camera_distance: number;
  /** Whether background music is enabled. */
  bgm_enabled: boolean;
  /** Background music volume (0–1). */
  bgm_volume: number;
  /** ID of the selected ambient track. */
  bgm_track_id: string;
  /** User-added custom BGM tracks (file/URL). */
  bgm_custom_tracks: BgmTrack[];
  /** When true, LLM auto-tags new memories with curated-prefix tags. */
  auto_tag?: boolean;
  /** When true, ingest prepends document-level context to each chunk before embedding (Anthropic 2024). */
  contextual_retrieval?: boolean;
  /** Set to true after the first-launch wizard completes (recommended or manual). */
  first_launch_complete?: boolean;
  /** When true, hide the 3D character and show a clean chatbox-only layout. */
  chatbox_mode?: boolean;
  /** Components auto-configured by the first-launch wizard (e.g. "brain", "voice", "quests"). */
  auto_configured?: string[];
  /**
   * When true (default), first-launch wizard tries local Ollama before
   * falling back to a free cloud provider. See rules/local-first-brain.md.
   */
  prefer_local_brain?: boolean;
  /** Model tags the user dismissed when offered an upgrade (never re-shown). */
  dismissed_model_updates?: string[];
  /** ISO date (`YYYY-MM-DD`) of the last auto-update check. */
  last_update_check_date?: string;
}

const DEFAULT_SETTINGS: AppSettings = {
  version: 2,
  selected_model_id: 'shinra',
  camera_azimuth: 0,
  camera_distance: 2.8,
  bgm_enabled: false,
  bgm_volume: 0.15,
  bgm_track_id: 'prelude',
  bgm_custom_tracks: [],
  auto_tag: false,
  contextual_retrieval: false,
  first_launch_complete: false,
  chatbox_mode: false,
  auto_configured: [],
  prefer_local_brain: true,
  dismissed_model_updates: [],
  last_update_check_date: '',
};

// ── Store ─────────────────────────────────────────────────────────────────────

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({ ...DEFAULT_SETTINGS });
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  // ── Load / Save ─────────────────────────────────────────────────────────────

  async function loadSettings(): Promise<void> {
    isLoading.value = true;
    error.value = null;
    try {
      settings.value = await invoke<AppSettings>('get_app_settings');
    } catch {
      // Tauri unavailable — keep defaults
    } finally {
      isLoading.value = false;
    }
  }

  async function saveSettings(patch: Partial<AppSettings>): Promise<void> {
    const updated: AppSettings = { ...settings.value, ...patch };
    settings.value = updated;
    try {
      await invoke('save_app_settings', { settings: updated });
    } catch {
      // Tauri unavailable — settings live in memory only
    }
  }

  // ── Convenience methods ─────────────────────────────────────────────────────

  /** Persist the selected character model ID. */
  async function saveModelId(modelId: string): Promise<void> {
    await saveSettings({ selected_model_id: modelId });
  }

  /** Persist the camera state after user interaction. */
  async function saveCameraState(azimuth: number, distance: number): Promise<void> {
    await saveSettings({ camera_azimuth: azimuth, camera_distance: distance });
  }

  /** Persist BGM state (enabled, volume, track). */
  async function saveBgmState(enabled: boolean, bgmVolume: number, trackId: string): Promise<void> {
    await saveSettings({ bgm_enabled: enabled, bgm_volume: bgmVolume, bgm_track_id: trackId });
  }

  /** Toggle chatbox-only mode (hides the 3D character). */
  async function setChatboxMode(enabled: boolean): Promise<void> {
    await saveSettings({ chatbox_mode: enabled });
  }

  return {
    // state
    settings,
    isLoading,
    error,
    // actions
    loadSettings,
    saveSettings,
    saveModelId,
    saveCameraState,
    saveBgmState,
    setChatboxMode,
  };
});
