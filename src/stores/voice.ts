import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { VoiceProviderInfo, VoiceConfig } from '../types';

/**
 * Payload emitted by the `supertonic-download-progress` Tauri event. Mirrors
 * the Rust `voice::supertonic_download::DownloadProgress` struct one-for-one.
 */
export interface SupertonicDownloadProgress {
  current_file: string;
  current_bytes: number;
  current_total: number;
  overall_bytes: number;
  overall_total: number;
  file_index: number;
  file_count: number;
}

/**
 * Auto-promotion record so the user can revert to whatever provider was
 * active before TerranSoul auto-switched them to on-device Supertonic.
 */
export interface SupertonicPromotionRecord {
  previousProvider: string | null;
  promotedAt: number;
}

// ── Fallback catalogue for when Tauri backend is unavailable ──────────────────

const FALLBACK_ASR_PROVIDERS: VoiceProviderInfo[] = [
  {
    id: 'web-speech',
    display_name: 'Web Speech API',
    description: 'Browser-native speech recognition. Zero setup.',
    kind: 'local',
    requires_api_key: false,
  },
];

const FALLBACK_TTS_PROVIDERS: VoiceProviderInfo[] = [
  {
    id: 'web-speech',
    display_name: 'Web Speech (browser, free)',
    description: 'Browser-native SpeechSynthesis. Free, offline-capable, no telemetry.',
    kind: 'local',
    requires_api_key: false,
  },
];

export const useVoiceStore = defineStore('voice', () => {
  const asrProviders = ref<VoiceProviderInfo[]>([]);
  const ttsProviders = ref<VoiceProviderInfo[]>([]);
  const config = ref<VoiceConfig>({
    asr_provider: null,
    tts_provider: null,
    tts_voice: null,
    tts_pitch: 0,
    tts_rate: 0,
    api_key: null,
    endpoint_url: null,
    hotwords: [],
  });
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  // ── Supertonic download / promotion state ──────────────────────────────────

  /** Set while a Supertonic model download is in flight. */
  const supertonicDownloading = ref(false);
  /** Latest progress event from the backend, or `null` when idle. */
  const supertonicProgress = ref<SupertonicDownloadProgress | null>(null);
  /** Last download error message, or `null` on success/idle. */
  const supertonicError = ref<string | null>(null);
  /** Whether the model files are present on disk. Refreshed by reloadTtsProviders. */
  const supertonicInstalled = computed(
    () => ttsProviders.value.find((p) => p.id === 'supertonic')?.installed ?? false,
  );
  /**
   * Record of an auto-promotion to Supertonic so the user can revert. Persists
   * inside the Pinia store for the session; the underlying `tts_provider`
   * change is already persisted via `set_tts_provider`.
   */
  const supertonicPromotion = ref<SupertonicPromotionRecord | null>(null);

  /** Unsubscribe handle for the Tauri progress listener. */
  let progressUnlisten: (() => void) | null = null;
  let completeUnlisten: (() => void) | null = null;

  const hasVoice = computed(
    () => config.value.asr_provider !== null || config.value.tts_provider !== null,
  );
  const isTextOnly = computed(
    () => config.value.asr_provider === null && config.value.tts_provider === null,
  );
  const selectedAsrProvider = computed(() =>
    asrProviders.value.find((p) => p.id === config.value.asr_provider) ?? null,
  );
  const selectedTtsProvider = computed(() =>
    ttsProviders.value.find((p) => p.id === config.value.tts_provider) ?? null,
  );

  // ── Load data ───────────────────────────────────────────────────────────────

  async function loadAsrProviders(): Promise<void> {
    try {
      asrProviders.value = await invoke<VoiceProviderInfo[]>('list_asr_providers');
    } catch {
      asrProviders.value = FALLBACK_ASR_PROVIDERS;
    }
  }

  async function loadTtsProviders(): Promise<void> {
    try {
      ttsProviders.value = await invoke<VoiceProviderInfo[]>('list_tts_providers');
    } catch {
      ttsProviders.value = FALLBACK_TTS_PROVIDERS;
    }
  }

  async function loadConfig(): Promise<void> {
    try {
      config.value = await invoke<VoiceConfig>('get_voice_config');
    } catch {
      // Tauri unavailable — keep default text-only config
    }
  }

  async function initialise(): Promise<void> {
    isLoading.value = true;
    error.value = null;
    try {
      await Promise.all([loadAsrProviders(), loadTtsProviders(), loadConfig()]);
    } catch {
      // Use fallbacks
      asrProviders.value = FALLBACK_ASR_PROVIDERS;
      ttsProviders.value = FALLBACK_TTS_PROVIDERS;
    } finally {
      isLoading.value = false;
    }
  }

  // ── Set providers ───────────────────────────────────────────────────────────

  async function setAsrProvider(providerId: string | null): Promise<void> {
    error.value = null;
    try {
      await invoke('set_asr_provider', { providerId });
    } catch {
      // Tauri unavailable — set locally
    }
    config.value = { ...config.value, asr_provider: providerId };
  }

  async function setTtsProvider(providerId: string | null): Promise<void> {
    error.value = null;
    try {
      await invoke('set_tts_provider', { providerId });
    } catch {
      // Tauri unavailable — set locally
    }
    config.value = { ...config.value, tts_provider: providerId };
  }

  async function setApiKey(apiKey: string | null): Promise<void> {
    try {
      await invoke('set_voice_api_key', { apiKey });
    } catch {
      // Tauri unavailable
    }
    config.value = { ...config.value, api_key: apiKey };
  }

  async function setEndpointUrl(endpointUrl: string | null): Promise<void> {
    try {
      await invoke('set_voice_endpoint', { endpointUrl });
    } catch {
      // Tauri unavailable
    }
    config.value = { ...config.value, endpoint_url: endpointUrl };
  }

  async function clearConfig(): Promise<void> {
    try {
      await invoke('clear_voice_config');
    } catch {
      // Tauri unavailable
    }
    config.value = {
      asr_provider: null,
      tts_provider: null,
      tts_voice: null,
      tts_pitch: 0,
      tts_rate: 0,
      api_key: null,
      endpoint_url: null,
      hotwords: [],
    };
  }

  /**
   * Auto-configure voice with free defaults (Web Speech API for both
   * ASR and TTS — browser-native, no third-party endpoints, commercial-OK).
   * Called when the user hasn't explicitly configured voice yet so that
   * voice is enabled out of the box.
   */
  async function autoConfigureVoice(): Promise<void> {
    await setAsrProvider('web-speech');
    await setTtsProvider('web-speech');
  }

  /**
   * Synthesize a sample sentence with a specific TTS provider WITHOUT
   * changing the persisted config. Returns the WAV bytes (or an empty
   * `Uint8Array` for browser-side providers like `web-speech` — the caller
   * should fall back to `window.speechSynthesis.speak()` in that case).
   */
  async function testTtsProvider(
    providerId: string,
    sampleText?: string,
  ): Promise<Uint8Array> {
    const result = await invoke<number[] | Uint8Array>('test_tts_provider', {
      providerId,
      sampleText: sampleText ?? null,
    });
    return result instanceof Uint8Array ? result : new Uint8Array(result);
  }

  // ── Supertonic download / promotion ────────────────────────────────────────

  /**
   * Start the Supertonic model download. Attaches listeners for the
   * `supertonic-download-progress` + `supertonic-download-complete` events,
   * invokes `supertonic_download_model`, and refreshes the provider catalogue
   * on success so {@link supertonicInstalled} flips to `true`.
   *
   * If the command rejects, the rejection message is mirrored into
   * {@link supertonicError} and re-thrown so the dialog can move to its
   * error stage.
   */
  async function downloadSupertonic(): Promise<void> {
    supertonicDownloading.value = true;
    supertonicError.value = null;
    supertonicProgress.value = null;
    try {
      const { listen } = await import('@tauri-apps/api/event');
      progressUnlisten = await listen<SupertonicDownloadProgress>(
        'supertonic-download-progress',
        (e) => {
          supertonicProgress.value = e.payload;
        },
      );
      completeUnlisten = await listen('supertonic-download-complete', () => {
        // The `invoke` resolution path is authoritative; this listener exists
        // for callers that want a fire-and-forget completion signal.
      });
    } catch {
      // Tauri unavailable — proceed; the invoke below will reject cleanly.
    }
    try {
      await invoke('supertonic_download_model');
      await loadTtsProviders();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      supertonicError.value = message;
      throw err;
    } finally {
      supertonicDownloading.value = false;
      try { progressUnlisten?.(); } catch { /* ignore */ }
      try { completeUnlisten?.(); } catch { /* ignore */ }
      progressUnlisten = null;
      completeUnlisten = null;
    }
  }

  /**
   * Promote Supertonic to the active TTS provider when:
   *   1. The user has no provider yet (`tts_provider === null`), OR
   *      they are on the placeholder `web-speech` browser default, AND
   *   2. The Supertonic model files are present on disk.
   *
   * Records the previous provider in {@link supertonicPromotion} so the user
   * can revert via {@link revertSupertonicPromotion}. No-ops when conditions
   * are not met. Returns `true` when a promotion happened.
   */
  async function autoPromoteSupertonicIfReady(): Promise<boolean> {
    const current = config.value.tts_provider;
    const eligible = current === null || current === 'web-speech';
    if (!eligible) return false;
    if (!supertonicInstalled.value) return false;
    const previous = current;
    await setTtsProvider('supertonic');
    supertonicPromotion.value = {
      previousProvider: previous,
      promotedAt: Date.now(),
    };
    return true;
  }

  /**
   * Revert an auto-promotion to Supertonic, restoring the provider that was
   * active beforehand. No-op when no promotion record exists.
   */
  async function revertSupertonicPromotion(): Promise<void> {
    const record = supertonicPromotion.value;
    if (!record) return;
    await setTtsProvider(record.previousProvider);
    supertonicPromotion.value = null;
  }

  return {
    // state
    asrProviders,
    ttsProviders,
    config,
    isLoading,
    error,
    supertonicDownloading,
    supertonicProgress,
    supertonicError,
    supertonicPromotion,
    // computed
    hasVoice,
    isTextOnly,
    selectedAsrProvider,
    selectedTtsProvider,
    supertonicInstalled,
    // actions
    loadAsrProviders,
    loadTtsProviders,
    loadConfig,
    initialise,
    setAsrProvider,
    setTtsProvider,
    setApiKey,
    setEndpointUrl,
    clearConfig,
    autoConfigureVoice,
    testTtsProvider,
    downloadSupertonic,
    autoPromoteSupertonicIfReady,
    revertSupertonicPromotion,
  };
});
