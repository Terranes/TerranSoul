import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type {
  BrainMode,
  FreeProvider,
  LmStudioDownloadStatus,
  LmStudioLoadResult,
  LmStudioModelEntry,
  LmStudioStatus,
  LmStudioUnloadResult,
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
  const lmStudioStatus = ref<LmStudioStatus>({ running: false, model_count: 0, loaded_count: 0 });
  const lmStudioModels = ref<LmStudioModelEntry[]>([]);
  const lmStudioDownload = ref<LmStudioDownloadStatus | null>(null);
  const lmStudioError = ref<string | null>(null);
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

  /** Fetch the latest model catalogue from the upstream repo, then refresh recommendations. */
  async function refreshModelCatalogue(): Promise<number> {
    const count = await invoke<number>('refresh_model_catalogue');
    await fetchRecommendations();
    return count;
  }

  async function checkOllamaStatus(): Promise<void> {
    ollamaStatus.value = await invoke<OllamaStatus>('check_ollama_status');
  }

  async function fetchInstalledModels(): Promise<void> {
    installedModels.value = await invoke<OllamaModelEntry[]>('get_ollama_models');
  }

  async function checkLmStudioStatus(baseUrl?: string, apiKey?: string | null): Promise<void> {
    lmStudioStatus.value = await invoke<LmStudioStatus>('check_lm_studio_status', {
      baseUrl: baseUrl || null,
      apiKey: apiKey || null,
    });
  }

  async function fetchLmStudioModels(baseUrl?: string, apiKey?: string | null): Promise<void> {
    lmStudioModels.value = await invoke<LmStudioModelEntry[]>('get_lm_studio_models', {
      baseUrl: baseUrl || null,
      apiKey: apiKey || null,
    });
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

  /** Factory-reset: clear persisted brain config, caches, and revert to unconfigured state. */
  async function factoryReset(): Promise<void> {
    await invoke('factory_reset_brain');
    activeBrain.value = null;
    brainMode.value = null;
  }

  async function downloadLmStudioModel(args: {
    model: string;
    baseUrl?: string;
    apiKey?: string | null;
    quantization?: string | null;
  }): Promise<LmStudioDownloadStatus | null> {
    lmStudioError.value = null;
    try {
      const status = await invoke<LmStudioDownloadStatus>('download_lm_studio_model', {
        model: args.model,
        baseUrl: args.baseUrl || null,
        apiKey: args.apiKey || null,
        quantization: args.quantization || null,
      });
      lmStudioDownload.value = status;
      return status;
    } catch (e) {
      lmStudioError.value = String(e);
      return null;
    }
  }

  async function getLmStudioDownloadStatus(
    jobId: string,
    baseUrl?: string,
    apiKey?: string | null,
  ): Promise<LmStudioDownloadStatus | null> {
    lmStudioError.value = null;
    try {
      const status = await invoke<LmStudioDownloadStatus>('get_lm_studio_download_status', {
        jobId,
        baseUrl: baseUrl || null,
        apiKey: apiKey || null,
      });
      lmStudioDownload.value = status;
      return status;
    } catch (e) {
      lmStudioError.value = String(e);
      return null;
    }
  }

  async function loadLmStudioModel(args: {
    model: string;
    baseUrl?: string;
    apiKey?: string | null;
    contextLength?: number | null;
  }): Promise<LmStudioLoadResult | null> {
    lmStudioError.value = null;
    try {
      const result = await invoke<LmStudioLoadResult>('load_lm_studio_model', {
        model: args.model,
        baseUrl: args.baseUrl || null,
        apiKey: args.apiKey || null,
        contextLength: args.contextLength || null,
      });
      await fetchLmStudioModels(args.baseUrl, args.apiKey);
      return result;
    } catch (e) {
      lmStudioError.value = String(e);
      return null;
    }
  }

  async function unloadLmStudioModel(
    instanceId: string,
    baseUrl?: string,
    apiKey?: string | null,
  ): Promise<LmStudioUnloadResult | null> {
    lmStudioError.value = null;
    try {
      const result = await invoke<LmStudioUnloadResult>('unload_lm_studio_model', {
        instanceId,
        baseUrl: baseUrl || null,
        apiKey: apiKey || null,
      });
      await fetchLmStudioModels(baseUrl, apiKey);
      return result;
    } catch (e) {
      lmStudioError.value = String(e);
      return null;
    }
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

  /**
   * Local-first brain auto-configuration (rules/local-first-brain.md).
   *
   * Decision cascade:
   * 1. If Ollama is running and has models → pick best installed model
   * 2. If Ollama is running but no models → pull §26 top-pick, then activate
   * 3. If Ollama is unreachable → fall back to Pollinations free cloud API
   *
   * Returns a summary object describing what was configured.
   */
  async function autoConfigureLocalFirst(callbacks?: {
    onProgress?: (message: string) => void;
  }): Promise<{ mode: 'local' | 'cloud'; model: string; pulled: boolean }> {
    const report = (msg: string) => callbacks?.onProgress?.(msg);

    // Step 0: Refresh model catalogue from online (best-effort)
    report('Checking for latest model recommendations...');
    try { await invoke('refresh_model_catalogue'); } catch { /* offline — use cached/bundled */ }

    // Step 1: Detect Ollama + system info + fresh recommendations
    report('Detecting local AI runtime (Ollama)...');
    await Promise.allSettled([
      checkOllamaStatus(),
      fetchSystemInfo(),
      fetchRecommendations(),
      fetchInstalledModels(),
    ]);

    if (!ollamaStatus.value.running) {
      // Ollama not available — cloud fallback
      report('Ollama not detected — using free cloud AI...');
      await autoConfigureForDesktop();
      return { mode: 'cloud', model: 'Pollinations AI', pulled: false };
    }

    // Step 2: Ollama is running — pick the best model
    const top = topRecommendation.value;
    const installed = installedModels.value;

    // Check if the §26 top-pick (or any recommended model) is already installed
    const topTag = top?.model_tag;
    const allRecTags = recommendations.value
      .filter(r => !r.is_cloud)
      .map(r => r.model_tag);

    // Find best installed model: prefer the top-pick, then any recommended, then largest installed
    let bestInstalled: string | null = null;
    if (topTag && installed.some(m => m.name === topTag)) {
      bestInstalled = topTag;
    } else {
      // Try any recommended model that's installed (in tier order)
      for (const tag of allRecTags) {
        if (installed.some(m => m.name === tag)) {
          bestInstalled = tag;
          break;
        }
      }
      // Last resort: use any installed model
      if (!bestInstalled && installed.length > 0) {
        bestInstalled = installed[0].name;
      }
    }

    if (bestInstalled) {
      report(`Activating local model: ${bestInstalled}...`);
      try {
        await setBrainMode({ mode: 'local_ollama', model: bestInstalled });
        return { mode: 'local', model: bestInstalled, pulled: false };
      } catch {
        // setBrainMode failed — cloud fallback
        report('Failed to activate local model — using free cloud AI...');
        await autoConfigureForDesktop();
        return { mode: 'cloud', model: 'Pollinations AI', pulled: false };
      }
    }

    // Step 3: Ollama running but no models installed → pull the §26 top-pick
    const modelToPull = topTag || 'gemma3:4b';
    report(`Downloading local model: ${modelToPull}... (this may take a few minutes)`);
    const pullOk = await pullModel(modelToPull);

    if (pullOk) {
      report(`Activating local model: ${modelToPull}...`);
      try {
        await setBrainMode({ mode: 'local_ollama', model: modelToPull });
        return { mode: 'local', model: modelToPull, pulled: true };
      } catch {
        // Activation failed after pull — cloud fallback
        report('Failed to activate local model — using free cloud AI...');
        await autoConfigureForDesktop();
        return { mode: 'cloud', model: 'Pollinations AI', pulled: false };
      }
    }

    // Pull failed — cloud fallback
    report('Model download failed — using free cloud AI...');
    await autoConfigureForDesktop();
    return { mode: 'cloud', model: 'Pollinations AI', pulled: false };
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
        refreshModelCatalogue().catch(() => fetchRecommendations()),
        checkOllamaStatus(),
        fetchInstalledModels(),
        checkLmStudioStatus(),
        fetchLmStudioModels(),
      ]);
    } catch {
      // Tauri backend unavailable — auto-default to free API
      autoConfigureFreeApi();
    } finally {
      isLoading.value = false;
    }
  }

  /** Process a prompt silently (for quest analysis) without adding to conversation history. */
  async function processPromptSilently(prompt: string): Promise<string> {
    try {
      if (!hasBrain.value) return '';

      const mode = brainMode.value;
      if (!mode) return '';

      // Resolve the API endpoint and model for the current brain mode.
      let baseUrl: string;
      let model: string;
      let apiKey: string | null = null;

      if (mode.mode === 'free_api') {
        if (freeProviders.value.length === 0) return '';
        const provider = freeProviders.value.find(
          p => p.id === mode.provider_id,
        ) ?? freeProviders.value[0];
        baseUrl = provider.base_url;
        model = provider.model;
        apiKey = provider.requires_api_key ? (mode.api_key ?? null) : null;
      } else if (mode.mode === 'paid_api') {
        baseUrl = mode.base_url;
        model = mode.model;
        apiKey = mode.api_key;
      } else if (mode.mode === 'local_ollama') {
        baseUrl = 'http://localhost:11434';
        model = mode.model;
      } else if (mode.mode === 'local_lm_studio') {
        baseUrl = mode.base_url;
        model = mode.model;
        apiKey = mode.api_key;
      } else {
        return '';
      }

      const { streamChatCompletion } = await import('../utils/free-api-client');
      return new Promise<string>((resolve) => {
        let text = '';
        streamChatCompletion(
          baseUrl,
          model,
          apiKey,
          [{ role: 'user', content: prompt }],
          {
            onChunk(chunk: string) { text += chunk; },
            onDone() { resolve(text); },
            onError() { resolve(''); },
          },
          'You are a helpful assistant. Respond with only the requested JSON format.',
        );
      });
    } catch (error) {
      console.warn('Silent prompt processing failed:', error);
      return '';
    }
  }

  return {
    activeBrain,
    systemInfo,
    recommendations,
    ollamaStatus,
    installedModels,
    lmStudioStatus,
    lmStudioModels,
    lmStudioDownload,
    lmStudioError,
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
    refreshModelCatalogue,
    checkOllamaStatus,
    fetchInstalledModels,
    checkLmStudioStatus,
    fetchLmStudioModels,
    pullModel,
    setActiveBrain,
    clearActiveBrain,
    factoryReset,
    downloadLmStudioModel,
    getLmStudioDownloadStatus,
    loadLmStudioModel,
    unloadLmStudioModel,
    fetchFreeProviders,
    loadBrainMode,
    setBrainMode,
    autoConfigureFreeApi,
    autoConfigureForDesktop,
    autoConfigureLocalFirst,
    initialise,
    processPromptSilently,
  };
});
