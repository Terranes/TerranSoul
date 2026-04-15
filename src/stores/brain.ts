import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type {
  BrainMode,
  FreeProvider,
  ModelRecommendation,
  OllamaModelEntry,
  OllamaStatus,
  SystemInfo,
} from '../types';

/** Built-in free provider catalogue for use when Tauri backend is unavailable. */
const FALLBACK_FREE_PROVIDERS: FreeProvider[] = [
  {
    id: 'pollinations',
    display_name: 'Pollinations AI',
    base_url: 'https://text.pollinations.ai/openai',
    model: 'openai',
    rpm_limit: 30,
    rpd_limit: 0,
    requires_api_key: false,
    notes: 'Free, no API key needed — works instantly',
  },
  {
    id: 'groq',
    display_name: 'Groq',
    base_url: 'https://api.groq.com/openai',
    model: 'llama-3.3-70b-versatile',
    rpm_limit: 30,
    rpd_limit: 1000,
    requires_api_key: true,
    notes: 'Fast inference, free tier with API key',
  },
  {
    id: 'cerebras',
    display_name: 'Cerebras',
    base_url: 'https://api.cerebras.ai',
    model: 'llama-3.3-70b',
    rpm_limit: 30,
    rpd_limit: 14400,
    requires_api_key: true,
    notes: 'Generous free limits, fast inference',
  },
];

export const useBrainStore = defineStore('brain', () => {
  const activeBrain = ref<string | null>(null);
  const systemInfo = ref<SystemInfo | null>(null);
  const recommendations = ref<ModelRecommendation[]>([]);
  const ollamaStatus = ref<OllamaStatus>({ running: false, model_count: 0 });
  const installedModels = ref<OllamaModelEntry[]>([]);
  const isPulling = ref(false);
  const pullError = ref<string | null>(null);
  const isLoading = ref(false);

  // Three-tier brain state
  const brainMode = ref<BrainMode | null>(null);
  const freeProviders = ref<FreeProvider[]>([]);

  const hasBrain = computed(() => activeBrain.value !== null || brainMode.value !== null);
  const topRecommendation = computed(() =>
    recommendations.value.find((m) => m.is_top_pick) ?? recommendations.value[0] ?? null,
  );

  /** Whether the system is using a free cloud API (no local setup needed). */
  const isFreeApiMode = computed(() =>
    brainMode.value !== null && brainMode.value.mode === 'free_api',
  );

  async function loadActiveBrain(): Promise<void> {
    activeBrain.value = await invoke<string | null>('get_active_brain');
  }

  async function fetchSystemInfo(): Promise<void> {
    systemInfo.value = await invoke<SystemInfo>('get_system_info');
  }

  async function fetchRecommendations(): Promise<void> {
    recommendations.value = await invoke<ModelRecommendation[]>('recommend_brain_models');
  }

  async function checkOllamaStatus(): Promise<void> {
    ollamaStatus.value = await invoke<OllamaStatus>('check_ollama_status');
  }

  async function fetchInstalledModels(): Promise<void> {
    installedModels.value = await invoke<OllamaModelEntry[]>('get_ollama_models');
  }

  async function pullModel(modelTag: string): Promise<boolean> {
    isPulling.value = true;
    pullError.value = null;
    try {
      await invoke('pull_ollama_model', { modelName: modelTag });
      await fetchInstalledModels();
      return true;
    } catch (e) {
      pullError.value = String(e);
      return false;
    } finally {
      isPulling.value = false;
    }
  }

  async function setActiveBrain(modelName: string): Promise<void> {
    await invoke('set_active_brain', { modelName });
    activeBrain.value = modelName;
  }

  async function clearActiveBrain(): Promise<void> {
    await invoke('clear_active_brain');
    activeBrain.value = null;
  }

  // ── Three-Tier Brain Methods ─────────────────────────────────────────────

  async function fetchFreeProviders(): Promise<void> {
    freeProviders.value = await invoke<FreeProvider[]>('list_free_providers');
  }

  async function loadBrainMode(): Promise<void> {
    brainMode.value = await invoke<BrainMode | null>('get_brain_mode');
  }

  async function setBrainMode(mode: BrainMode): Promise<void> {
    await invoke('set_brain_mode', { mode });
    brainMode.value = mode;
    // Update legacy activeBrain for backwards compatibility
    if (mode.mode === 'local_ollama') {
      activeBrain.value = mode.model;
    } else {
      activeBrain.value = null;
    }
  }

  /**
   * Auto-configure free API as the default brain mode (browser-side only).
   * Sets state in the Pinia store but does NOT persist to the Tauri backend.
   * Use {@link autoConfigureForDesktop} when Tauri is available.
   */
  function autoConfigureFreeApi(): void {
    freeProviders.value = FALLBACK_FREE_PROVIDERS;
    brainMode.value = {
      mode: 'free_api',
      provider_id: 'pollinations',
      api_key: null,
    };
  }

  /**
   * Auto-configure free API on desktop: persists to the Tauri backend
   * so that the Rust `send_message_stream` command knows the brain mode.
   * Without this, the backend's AppState keeps `brain_mode = None` and
   * returns a stub response instead of calling the real LLM API.
   */
  async function autoConfigureForDesktop(): Promise<void> {
    const mode: BrainMode = {
      mode: 'free_api',
      provider_id: 'pollinations',
      api_key: null,
    };
    try {
      await setBrainMode(mode);
    } catch {
      // setBrainMode invoke failed — set locally as fallback
      brainMode.value = mode;
    }
    if (freeProviders.value.length === 0) {
      freeProviders.value = FALLBACK_FREE_PROVIDERS;
    }
  }

  /** Full initialisation for the brain setup wizard. */
  async function initialise(): Promise<void> {
    isLoading.value = true;
    try {
      // Core commands that must succeed for the brain to be usable:
      //   - loadActiveBrain: legacy brain state
      //   - loadBrainMode: three-tier brain config (free_api, paid_api, local_ollama)
      //   - fetchFreeProviders: catalogue of free providers for the config UI
      await Promise.all([
        loadActiveBrain(),
        loadBrainMode(),
        fetchFreeProviders(),
      ]);
      // Non-critical: load hardware info and Ollama status for the setup wizard.
      // These may fail if Ollama isn't installed — that's fine, we still function.
      await Promise.allSettled([
        fetchSystemInfo(),
        fetchRecommendations(),
        checkOllamaStatus(),
        fetchInstalledModels(),
      ]);
    } catch {
      // Tauri backend unavailable — auto-default to free API
      autoConfigureFreeApi();
    } finally {
      isLoading.value = false;
    }
  }

  return {
    activeBrain,
    systemInfo,
    recommendations,
    ollamaStatus,
    installedModels,
    isPulling,
    pullError,
    isLoading,
    brainMode,
    freeProviders,
    hasBrain,
    topRecommendation,
    isFreeApiMode,
    loadActiveBrain,
    fetchSystemInfo,
    fetchRecommendations,
    checkOllamaStatus,
    fetchInstalledModels,
    pullModel,
    setActiveBrain,
    clearActiveBrain,
    fetchFreeProviders,
    loadBrainMode,
    setBrainMode,
    autoConfigureFreeApi,
    autoConfigureForDesktop,
    initialise,
  };
});
