import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { BgmTrack } from '../composables/useBgmPlayer';

// ── Types ─────────────────────────────────────────────────────────────────────

/** A user-defined folder scanned for knowledge ingestion. */
export interface ContextFolder {
  path: string;
  label: string;
  enabled: boolean;
  last_synced_at: number;
  last_file_count: number;
}

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
  /** When true, MCP/gRPC brain servers may bind to LAN interfaces for local sharing. */
  lan_enabled?: boolean;
  /** LAN auth mode: token-required or public read-only. */
  lan_auth_mode?: 'token_required' | 'public_read_only';
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
  /** Hard cap on long-term memory entries before capacity-based eviction kicks in. Default 1,000,000. */
  max_long_term_entries?: number;
  /** Minimum hybrid-search score (0.0–1.0) for a memory to be injected into the RAG context. Default 0.30. */
  relevance_threshold?: number;
  /** Whether background maintenance jobs (decay, GC, promotion) are enabled. */
  background_maintenance_enabled?: boolean;
  /** Minimum hours between maintenance job runs. 1–168. Default 24. */
  maintenance_interval_hours?: number;
  /** Skip maintenance if user was active within this many minutes. Default 5. */
  maintenance_idle_minimum_minutes?: number;
  /** Whether to enable web search fallback (CRAG / DuckDuckGo). */
  web_search_enabled?: boolean;
  /** Per-workspace data root override — when set, runtime data (DB, HNSW, etc.) is stored here instead of the default platform path. */
  data_root?: string;
  /** Optional Hive relay URL (gRPC endpoint). When set, the app participates in hive federation. */
  hive_url?: string;
  /** Obsidian export folder layout: 'flat' (default) or 'para' (PARA method subfolders). */
  obsidian_layout?: 'flat' | 'para';
  /** SQLite page-cache for memory.db (MiB). Default 16. Takes effect on restart. */
  sqlite_cache_mb?: number;
  /** SQLite mmap window for memory.db (MiB). Default 64. Takes effect on restart. */
  sqlite_mmap_mb?: number;
  /** SQLite page-cache for code_index.sqlite (MiB). Default 8. Takes effect on restart. */
  code_index_cache_mb?: number;
  /** SQLite mmap window for code_index.sqlite (MiB). Default 32. Takes effect on restart. */
  code_index_mmap_mb?: number;
  /** User-defined context folders for knowledge ingestion (brute-force scan, not recommended for large trees). */
  context_folders?: ContextFolder[];
}

const MIN_MAX_MEMORY_GB = 1;
const MAX_MAX_MEMORY_GB = 100;
const DEFAULT_MAX_MEMORY_GB = 10;

const MIN_MAX_MEMORY_MB = 1;
const MAX_MAX_MEMORY_MB = 1024;
const DEFAULT_MAX_MEMORY_MB = 256;

export const DEFAULT_SQLITE_CACHE_MB = 16;
export const MIN_SQLITE_CACHE_MB = 2;
export const MAX_SQLITE_CACHE_MB = 512;

export const DEFAULT_SQLITE_MMAP_MB = 64;
export const MIN_SQLITE_MMAP_MB = 0;
export const MAX_SQLITE_MMAP_MB = 2048;

export const DEFAULT_CODE_INDEX_CACHE_MB = 8;
export const MIN_CODE_INDEX_CACHE_MB = 2;
export const MAX_CODE_INDEX_CACHE_MB = 256;

export const DEFAULT_CODE_INDEX_MMAP_MB = 32;
export const MIN_CODE_INDEX_MMAP_MB = 0;
export const MAX_CODE_INDEX_MMAP_MB = 1024;

export const DEFAULT_MAX_LONG_TERM_ENTRIES = 1_000_000;
export const MIN_MAX_LONG_TERM_ENTRIES = 1_000;
export const MAX_MAX_LONG_TERM_ENTRIES = 10_000_000;

export const DEFAULT_RELEVANCE_THRESHOLD = 0.30;
export const MIN_RELEVANCE_THRESHOLD = 0.0;
export const MAX_RELEVANCE_THRESHOLD = 1.0;

export const DEFAULT_MAINTENANCE_INTERVAL_HOURS = 24;
export const MIN_MAINTENANCE_INTERVAL_HOURS = 1;
export const MAX_MAINTENANCE_INTERVAL_HOURS = 168;

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
  lan_enabled: false,
  lan_auth_mode: 'token_required',
  mobile_notifications_enabled: true,
  mobile_notification_threshold_ms: 30_000,
  mobile_notification_poll_ms: 10_000,
  first_launch_complete: false,
  chatbox_mode: false,
  auto_configured: [],
  prefer_local_brain: true,
  dismissed_model_updates: [],
  last_update_check_date: '',
  max_memory_gb: DEFAULT_MAX_MEMORY_GB,
  max_memory_mb: DEFAULT_MAX_MEMORY_MB,
  max_long_term_entries: DEFAULT_MAX_LONG_TERM_ENTRIES,
  relevance_threshold: DEFAULT_RELEVANCE_THRESHOLD,
  background_maintenance_enabled: true,
  maintenance_interval_hours: DEFAULT_MAINTENANCE_INTERVAL_HOURS,
  maintenance_idle_minimum_minutes: 5,
  web_search_enabled: false,
  sqlite_cache_mb: DEFAULT_SQLITE_CACHE_MB,
  sqlite_mmap_mb: DEFAULT_SQLITE_MMAP_MB,
  code_index_cache_mb: DEFAULT_CODE_INDEX_CACHE_MB,
  code_index_mmap_mb: DEFAULT_CODE_INDEX_MMAP_MB,
  context_folders: [],
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
    const clamped = Math.min(
      MAX_MAX_MEMORY_GB,
      Math.max(MIN_MAX_MEMORY_GB, Number.isFinite(gb) ? gb : DEFAULT_MAX_MEMORY_GB),
    );
    await saveSettings({ max_memory_gb: clamped });
  }

  async function saveMaxMemoryMb(mb: number): Promise<void> {
    const clamped = Math.min(
      MAX_MAX_MEMORY_MB,
      Math.max(MIN_MAX_MEMORY_MB, Number.isFinite(mb) ? mb : DEFAULT_MAX_MEMORY_MB),
    );
    await saveSettings({ max_memory_mb: clamped });
  }

  async function saveMaxLongTermEntries(entries: number): Promise<void> {
    const clamped = Math.min(
      MAX_MAX_LONG_TERM_ENTRIES,
      Math.max(MIN_MAX_LONG_TERM_ENTRIES, Number.isFinite(entries) ? Math.round(entries) : DEFAULT_MAX_LONG_TERM_ENTRIES),
    );
    await saveSettings({ max_long_term_entries: clamped });
  }

  async function saveRelevanceThreshold(threshold: number): Promise<void> {
    const clamped = Math.min(
      MAX_RELEVANCE_THRESHOLD,
      Math.max(MIN_RELEVANCE_THRESHOLD, Number.isFinite(threshold) ? threshold : DEFAULT_RELEVANCE_THRESHOLD),
    );
    await saveSettings({ relevance_threshold: Math.round(clamped * 100) / 100 });
  }

  async function saveMaintenanceInterval(hours: number): Promise<void> {
    const clamped = Math.min(
      MAX_MAINTENANCE_INTERVAL_HOURS,
      Math.max(MIN_MAINTENANCE_INTERVAL_HOURS, Number.isFinite(hours) ? Math.round(hours) : DEFAULT_MAINTENANCE_INTERVAL_HOURS),
    );
    await saveSettings({ maintenance_interval_hours: clamped });
  }

  async function saveSqliteCacheMb(mb: number): Promise<void> {
    const clamped = Math.min(
      MAX_SQLITE_CACHE_MB,
      Math.max(MIN_SQLITE_CACHE_MB, Number.isFinite(mb) ? Math.round(mb) : DEFAULT_SQLITE_CACHE_MB),
    );
    await saveSettings({ sqlite_cache_mb: clamped });
  }

  async function saveSqliteMmapMb(mb: number): Promise<void> {
    const clamped = Math.min(
      MAX_SQLITE_MMAP_MB,
      Math.max(MIN_SQLITE_MMAP_MB, Number.isFinite(mb) ? Math.round(mb) : DEFAULT_SQLITE_MMAP_MB),
    );
    await saveSettings({ sqlite_mmap_mb: clamped });
  }

  async function saveCodeIndexCacheMb(mb: number): Promise<void> {
    const clamped = Math.min(
      MAX_CODE_INDEX_CACHE_MB,
      Math.max(MIN_CODE_INDEX_CACHE_MB, Number.isFinite(mb) ? Math.round(mb) : DEFAULT_CODE_INDEX_CACHE_MB),
    );
    await saveSettings({ code_index_cache_mb: clamped });
  }

  async function saveCodeIndexMmapMb(mb: number): Promise<void> {
    const clamped = Math.min(
      MAX_CODE_INDEX_MMAP_MB,
      Math.max(MIN_CODE_INDEX_MMAP_MB, Number.isFinite(mb) ? Math.round(mb) : DEFAULT_CODE_INDEX_MMAP_MB),
    );
    await saveSettings({ code_index_mmap_mb: clamped });
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
    saveMaxLongTermEntries,
    saveRelevanceThreshold,
    saveMaintenanceInterval,
    saveSqliteCacheMb,
    saveSqliteMmapMb,
    saveCodeIndexCacheMb,
    saveCodeIndexMmapMb,
  };
});
