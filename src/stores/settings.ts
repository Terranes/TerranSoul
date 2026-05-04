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
  /** When true, local-Ollama ingest uses whole-document token vectors for late chunking when available. */
  late_chunking?: boolean;
  /** When true, paired mobile shells can show local notifications for long-running desktop work. */
  mobile_notifications_enabled?: boolean;
  /** Minimum observed duration before mobile local notifications fire. */
  mobile_notification_threshold_ms?: number;
  /** Poll interval for the paired-mobile notification watcher. */
  mobile_notification_poll_ms?: number;
  /**
   * Opt-in per-ARKit-blendshape passthrough for advanced VRM rigs
   * (Chunk 27.3). When true, the camera mirror also writes the raw 52
   * ARKit shapes by name; rigs without those channels are silent
   * no-ops via `applyExpandedBlendshapes`. See
   * `docs/persona-design.md` § 6.3.
   */
  expanded_blendshapes?: boolean;
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
  /** Maximum persistent memory/RAG storage in GB before automatic cleanup. */
  max_memory_gb?: number;
  /** Maximum brain memory/RAG list cache in MB; storage still keeps the full corpus. */
  max_memory_mb?: number;
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
  late_chunking: false,
  mobile_notifications_enabled: true,
  mobile_notification_threshold_ms: 30_000,
  mobile_notification_poll_ms: 10_000,
  first_launch_complete: false,
  chatbox_mode: false,
  auto_configured: [],
  prefer_local_brain: true,
  dismissed_model_updates: [],
  last_update_check_date: '',
  max_memory_gb: 10,
  max_memory_mb: 10,
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

  async function saveMaxMemoryGb(gb: number): Promise<void> {
    const clamped = Math.min(100, Math.max(1, Number.isFinite(gb) ? gb : 10));
    await saveSettings({ max_memory_gb: clamped });
  }

  async function saveMaxMemoryMb(mb: number): Promise<void> {
    const clamped = Math.min(1024, Math.max(1, Number.isFinite(mb) ? mb : 10));
    await saveSettings({ max_memory_mb: clamped });
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
    saveMaxMemoryGb,
    saveMaxMemoryMb,
  };
});
