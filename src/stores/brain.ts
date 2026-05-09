import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { tryContainerFallback } from './brain-container-fallback';
import type {
  AgentRole,
  AgentRouteConfig,
  BrainMode,
  DiskInfo,
  FreeProvider,
  LmStudioDownloadStatus,
  LmStudioLoadResult,
  LmStudioModelEntry,
  LmStudioStatus,
  LmStudioUnloadResult,
  ModelRecommendation,
  OllamaInstallStatus,
  OllamaModelEntry,
  OllamaStatus,
  ProviderPolicy,
  ResolvedAgentProvider,
  ResolvedProvider,
  SystemInfo,
  TaskKind,
  TaskOverride,
} from '../types';

export type BrowserAuthProviderId = 'chatgpt' | 'gemini' | 'openrouter' | 'nvidia-nim' | 'pollinations' | 'google';

export interface BrowserAuthModelOption {
  label: string;
  model: string;
}

type BrowserAuthProviderMode =
  | { kind: 'free'; providerId: string }
  | { kind: 'paid'; provider: string; baseUrl: string; defaultModel: string };

export interface BrowserAuthProvider {
  id: BrowserAuthProviderId;
  label: string;
  brainLabel: string;
  privacyLabel: string;
  authorizationUrl: string;
  authorizationLabel: string;
  recommendation?: string;
  mode: BrowserAuthProviderMode;
  requiresApiKey: boolean;
  apiKeyPlaceholder?: string;
  modelOptions?: BrowserAuthModelOption[];
}

export interface BrowserAuthSession {
  providerId: BrowserAuthProviderId;
  label: string;
  model?: string;
  connectedAt: number;
}

export const OPENROUTER_FREE_MODELS: BrowserAuthModelOption[] = [
  { label: 'OpenRouter Owl Alpha (free, long context)', model: 'openrouter/owl-alpha' },
  { label: 'NVIDIA Nemotron 3 Nano Omni (free)', model: 'nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free' },
  { label: 'Poolside Laguna XS.2 (free coding)', model: 'poolside/laguna-xs.2:free' },
  { label: 'Llama 3.3 70B Instruct (free)', model: 'meta-llama/llama-3.3-70b-instruct:free' },
];

export const NVIDIA_FREE_MODELS: BrowserAuthModelOption[] = [
  { label: 'Nemotron 3 Super 120B', model: 'nvidia/nemotron-3-super-120b-a12b' },
  { label: 'Gemma 4 31B IT', model: 'google/gemma-4-31b-it' },
  { label: 'Llama 3.3 70B Instruct', model: 'meta/llama-3.3-70b-instruct' },
];

export const POLLINATIONS_MODELS: BrowserAuthModelOption[] = [
  { label: 'Llama 3.3 70B', model: 'llama' },
  { label: 'OpenAI-compatible default', model: 'openai' },
  { label: 'OpenAI fast', model: 'openai-fast' },
];

/** Built-in free provider catalogue for use when Tauri backend is unavailable. */
const FALLBACK_FREE_PROVIDERS: FreeProvider[] = [
  {
    id: 'pollinations',
    display_name: 'Pollinations AI',
    base_url: 'https://gen.pollinations.ai',
    model: POLLINATIONS_MODELS[0].model,
    rpm_limit: 30,
    rpd_limit: 0,
    requires_api_key: true,
    notes: 'Register at enter.pollinations.ai for a token and higher limits',
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
  {
    id: 'openrouter',
    display_name: 'OpenRouter',
    base_url: 'https://openrouter.ai/api',
    model: OPENROUTER_FREE_MODELS[0].model,
    rpm_limit: 20,
    rpd_limit: 50,
    requires_api_key: true,
    notes: 'Multi-model gateway with selectable free models',
  },
  {
    id: 'nvidia-nim',
    display_name: 'NVIDIA NIM',
    base_url: 'https://integrate.api.nvidia.com',
    model: NVIDIA_FREE_MODELS[0].model,
    rpm_limit: 40,
    rpd_limit: 0,
    requires_api_key: true,
    notes: 'NVIDIA hosted free tier with an API key',
  },
  {
    id: 'gemini',
    display_name: 'Google Gemini',
    base_url: 'https://generativelanguage.googleapis.com/v1beta/openai',
    model: 'gemini-3-flash-preview',
    rpm_limit: 15,
    rpd_limit: 1000,
    requires_api_key: true,
    notes: 'Google AI Studio free-tier key, OpenAI-compatible endpoint',
  },
];

const BROWSER_AUTH_STORAGE_KEY = 'ts.browser.auth.session';
const BROWSER_BRAIN_MODE_STORAGE_KEY = 'ts.browser.brain.mode';

const BROWSER_AUTH_PROVIDERS: BrowserAuthProvider[] = [
  {
    id: 'chatgpt',
    label: 'Authorize with ChatGPT',
    brainLabel: 'ChatGPT / OpenAI browser session',
    privacyLabel: 'Open OpenAI, create a restricted key, then connect it only in this browser.',
    authorizationUrl: 'https://platform.openai.com/api-keys',
    authorizationLabel: 'Open OpenAI API keys',
    mode: { kind: 'paid', provider: 'openai', baseUrl: 'https://api.openai.com', defaultModel: 'gpt-4o-mini' },
    requiresApiKey: true,
    apiKeyPlaceholder: 'OpenAI API key',
    modelOptions: [
      { label: 'GPT-4o mini', model: 'gpt-4o-mini' },
      { label: 'GPT-4o', model: 'gpt-4o' },
    ],
  },
  {
    id: 'gemini',
    label: 'Authorize with Gemini',
    brainLabel: 'Google Gemini browser session',
    privacyLabel: 'Good free-tier direct provider via Google AI Studio.',
    authorizationUrl: 'https://aistudio.google.com/app/apikey',
    authorizationLabel: 'Open Google AI Studio',
    mode: { kind: 'free', providerId: 'gemini' },
    requiresApiKey: true,
    apiKeyPlaceholder: 'Google AI Studio API key',
    modelOptions: [
      { label: 'Gemini 3 Flash Preview', model: 'gemini-3-flash-preview' },
      { label: 'Gemini 2.5 Flash', model: 'gemini-2.5-flash' },
      { label: 'Gemini 2.0 Flash', model: 'gemini-2.0-flash' },
    ],
  },
  {
    id: 'openrouter',
    label: 'Authorize with OpenRouter',
    brainLabel: 'OpenRouter free-model browser session',
    privacyLabel: 'Recommended: one key, many current free models, optional app credit limits.',
    authorizationUrl: 'https://openrouter.ai/keys',
    authorizationLabel: 'Open OpenRouter keys',
    recommendation: 'Best free default for the backendless web app',
    mode: { kind: 'free', providerId: 'openrouter' },
    requiresApiKey: true,
    apiKeyPlaceholder: 'OpenRouter API key',
    modelOptions: OPENROUTER_FREE_MODELS,
  },
  {
    id: 'nvidia-nim',
    label: 'Authorize with NVIDIA',
    brainLabel: 'NVIDIA NIM browser session',
    privacyLabel: 'Free serverless development APIs for NVIDIA-hosted models.',
    authorizationUrl: 'https://build.nvidia.com/explore/discover',
    authorizationLabel: 'Open NVIDIA Build',
    mode: { kind: 'free', providerId: 'nvidia-nim' },
    requiresApiKey: true,
    apiKeyPlaceholder: 'NVIDIA API key',
    modelOptions: NVIDIA_FREE_MODELS,
  },
  {
    id: 'pollinations',
    label: 'Authorize with Pollinations',
    brainLabel: 'Pollinations browser session',
    privacyLabel: 'Use a Pollinations token from enter.pollinations.ai for higher limits.',
    authorizationUrl: 'https://enter.pollinations.ai/',
    authorizationLabel: 'Open Pollinations',
    mode: { kind: 'free', providerId: 'pollinations' },
    requiresApiKey: true,
    apiKeyPlaceholder: 'Pollinations token',
    modelOptions: POLLINATIONS_MODELS,
  },
];

function readBrowserAuthSession(): BrowserAuthSession | null {
  if (typeof localStorage === 'undefined') return null;
  try {
    const raw = localStorage.getItem(BROWSER_AUTH_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<BrowserAuthSession>;
    const parsedProviderId = parsed.providerId === 'google' ? 'gemini' : parsed.providerId;
    const provider = BROWSER_AUTH_PROVIDERS.find((item) => item.id === parsedProviderId);
    if (!provider || typeof parsed.connectedAt !== 'number') return null;
    const session: BrowserAuthSession = {
      providerId: provider.id,
      label: provider.brainLabel,
      connectedAt: parsed.connectedAt,
    };
    if (typeof parsed.model === 'string') session.model = parsed.model;
    return session;
  } catch {
    return null;
  }
}

function readBrowserBrainMode(): BrainMode | null {
  if (typeof localStorage === 'undefined') return null;
  try {
    const raw = localStorage.getItem(BROWSER_BRAIN_MODE_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as BrainMode;
    if (parsed?.mode === 'free_api' || parsed?.mode === 'paid_api') return parsed;
  } catch {
    // Ignore stale or malformed browser-only config.
  }
  return null;
}

function writeBrowserBrainMode(mode: BrainMode | null): void {
  if (typeof localStorage === 'undefined') return;
  try {
    if (mode) {
      localStorage.setItem(BROWSER_BRAIN_MODE_STORAGE_KEY, JSON.stringify(mode));
    } else {
      localStorage.removeItem(BROWSER_BRAIN_MODE_STORAGE_KEY);
    }
  } catch {
    // Browser storage can be disabled; keep Pinia state in-memory.
  }
}

function writeBrowserAuthSession(session: BrowserAuthSession | null): void {
  if (typeof localStorage === 'undefined') return;
  try {
    if (session) {
      localStorage.setItem(BROWSER_AUTH_STORAGE_KEY, JSON.stringify(session));
    } else {
      localStorage.removeItem(BROWSER_AUTH_STORAGE_KEY);
    }
  } catch {
    // Browser storage can be disabled; keep Pinia state in-memory.
  }
}

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
  const browserAuthSession = ref<BrowserAuthSession | null>(readBrowserAuthSession());

  const hasBrain = computed(() => activeBrain.value !== null || brainMode.value !== null);
  const topRecommendation = computed(() =>
    recommendations.value.find((m) => m.is_top_pick) ?? recommendations.value[0] ?? null,
  );

  /** Whether the system is using a free cloud API (no local setup needed). */
  const isFreeApiMode = computed(() =>
    brainMode.value !== null && brainMode.value.mode === 'free_api',
  );
  const browserAuthProviders = computed(() => BROWSER_AUTH_PROVIDERS);
  const browserAuthProvider = computed(() =>
    browserAuthSession.value
      ? BROWSER_AUTH_PROVIDERS.find((provider) => provider.id === browserAuthSession.value?.providerId) ?? null
      : null,
  );

  function ensureFallbackFreeProviders(): void {
    if (freeProviders.value.length === 0) {
      freeProviders.value = FALLBACK_FREE_PROVIDERS.map((provider) => ({ ...provider }));
    }
  }

  function setFallbackProviderModel(providerId: string, model?: string): void {
    if (!model) return;
    ensureFallbackFreeProviders();
    const provider = freeProviders.value.find((item) => item.id === providerId);
    if (provider) provider.model = model;
  }

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

  /** Detect Ollama installation status (binary on disk + service responding). */
  async function detectOllamaInstall(): Promise<OllamaInstallStatus> {
    return invoke<OllamaInstallStatus>('detect_ollama_install');
  }

  /** Try to start the Ollama service. Returns true if running by end of timeout. */
  async function startOllamaService(timeoutSecs = 15): Promise<boolean> {
    return invoke<boolean>('start_ollama_service', { timeoutSecs });
  }

  /** Download + install Ollama. Emits 'ollama-install-progress' events. */
  async function installOllama(): Promise<string> {
    return invoke<string>('install_ollama');
  }

  async function fetchInstalledModels(): Promise<void> {
    installedModels.value = await invoke<OllamaModelEntry[]>('get_ollama_models');
  }

  /** Get the path where Ollama stores downloaded models. */
  async function getOllamaModelsDir(): Promise<string> {
    return invoke<string>('get_ollama_models_dir');
  }

  /** Get disk space info for the drive containing the given path. */
  async function getDiskSpace(path: string): Promise<DiskInfo> {
    return invoke<DiskInfo>('get_disk_space', { path });
  }

  /** List all mounted drives with their available and total space. */
  async function listDrives(): Promise<DiskInfo[]> {
    return invoke<DiskInfo[]>('list_drives');
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

  /**
   * Pre-load the active local Ollama chat model into VRAM with a long
   * `keep_alive` so the next user reply lands in milliseconds instead of
   * paying a 10–20s cold-load on consumer GPUs.
   *
   * Fire-and-forget: returns immediately. Errors are swallowed (the chat
   * path will still work, just with a one-time cold-start). Call after
   * installing a recommended model, on app start when LocalOllama is
   * active, and on brain-mode change to LocalOllama.
   */
  async function warmupLocalOllama(model?: string): Promise<number | null> {
    try {
      const ms = await invoke<number>('warmup_local_ollama', {
        model: model ?? null,
      });
      return ms;
    } catch {
      return null;
    }
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
      // Pre-warm chat model into VRAM in the background so the first
      // user reply is fast. Fire-and-forget — no await.
      void warmupLocalOllama(mode.model);
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
    ensureFallbackFreeProviders();
    brainMode.value = {
      mode: 'free_api',
      provider_id: 'pollinations',
      api_key: null,
      model: POLLINATIONS_MODELS[0].model,
    };
  }

  function prepareBrowserProviderChoices(): void {
    ensureFallbackFreeProviders();
    if (!browserAuthSession.value) {
      if (import.meta.env.VITE_E2E) {
        autoConfigureFreeApi();
        return;
      }
      brainMode.value = null;
      writeBrowserBrainMode(null);
      return;
    }
    if (!brainMode.value && browserAuthSession.value) {
      const stored = readBrowserBrainMode();
      if (stored) brainMode.value = stored;
    }
  }

  function authoriseBrowserProvider(
    providerId: BrowserAuthProviderId,
    options: { apiKey?: string; model?: string } = {},
  ): BrowserAuthSession {
    const normalizedProviderId = providerId === 'google' ? 'gemini' : providerId;
    const provider = BROWSER_AUTH_PROVIDERS.find((item) => item.id === normalizedProviderId) ??
      BROWSER_AUTH_PROVIDERS[BROWSER_AUTH_PROVIDERS.length - 1];
    ensureFallbackFreeProviders();

    const selectedModel = options.model || provider.modelOptions?.[0]?.model;
    let mode: BrainMode;
    if (provider.mode.kind === 'free') {
      setFallbackProviderModel(provider.mode.providerId, selectedModel);
      mode = {
        mode: 'free_api',
        provider_id: provider.mode.providerId,
        api_key: options.apiKey?.trim() || null,
        model: selectedModel || null,
      };
    } else {
      mode = {
        mode: 'paid_api',
        provider: provider.mode.provider,
        api_key: options.apiKey?.trim() || '',
        model: selectedModel || provider.mode.defaultModel,
        base_url: provider.mode.baseUrl,
      };
    }
    brainMode.value = mode;
    activeBrain.value = null;
    const session: BrowserAuthSession = {
      providerId: provider.id,
      label: provider.brainLabel,
      model: mode.mode === 'free_api'
        ? freeProviders.value.find((item) => item.id === mode.provider_id)?.model
        : mode.model,
      connectedAt: Date.now(),
    };
    browserAuthSession.value = session;
    writeBrowserAuthSession(session);
    writeBrowserBrainMode(mode);
    return session;
  }

  function clearBrowserAuthorisation(): void {
    browserAuthSession.value = null;
    writeBrowserAuthSession(null);
    writeBrowserBrainMode(null);
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
      model: POLLINATIONS_MODELS[0].model,
    };
    try {
      await setBrainMode(mode);
    } catch {
      // setBrainMode invoke failed — set locally as fallback
      brainMode.value = mode;
    }
    if (freeProviders.value.length === 0) {
      ensureFallbackFreeProviders();
    }
  }

  /** RAM-aware fallback when the catalogue/recommendations are unavailable. */
  function ramAwareFallback(totalRamMb?: number): string {
    const ram = totalRamMb ?? 0;
    // Optimised for sub-1 s first-token latency on consumer GPUs.
    if (ram >= 32_768) return 'gemma4:e4b';
    if (ram >= 16_384) return 'gemma3:4b';
    if (ram >= 8_192) return 'gemma3:1b';
    if (ram >= 4_096) return 'gemma3:1b';
    return 'tinyllama';
  }

  /**
   * Remove weaker auto-configured Ollama models now that a better one is active.
   * Only removes models that appear in the recommendation catalogue (never
   * user-installed custom models). Best-effort — errors are silently ignored.
   */
  async function removeWeakerAutoModels(
    activeModel: string,
    installed: { name: string }[],
    report?: (msg: string) => void,
  ): Promise<void> {
    const recTags = new Set(
      recommendations.value.filter(r => !r.is_cloud).map(r => r.model_tag),
    );
    for (const m of installed) {
      if (m.name === activeModel) continue; // keep the active model
      if (!recTags.has(m.name)) continue; // not in catalogue → user model, skip
      try {
        report?.(`Removing superseded model: ${m.name}...`);
        await invoke('delete_ollama_model', { modelName: m.name });
      } catch {
        // Best-effort — Ollama may already have removed it.
      }
    }
    // Refresh the installed list after cleanup.
    await fetchInstalledModels();
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
  }): Promise<{ mode: 'local' | 'cloud'; model: string; pulled: boolean; pullFailed?: string; ollamaInstalled?: boolean; ollamaStarted?: boolean }> {
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

    let ollamaInstalled = false;
    let ollamaStarted = false;

    if (!ollamaStatus.value.running) {
      // Ollama not responding — check if it's installed but stopped, or missing entirely.
      report('Ollama not responding — checking install status...');
      let installStatus: OllamaInstallStatus;
      try {
        installStatus = await detectOllamaInstall();
      } catch {
        installStatus = { installed: false, running: false, binary_path: null };
      }
      ollamaInstalled = installStatus.installed;

      if (installStatus.installed) {
        // Try to start it.
        report('Ollama is installed — starting service...');
        try {
          const started = await startOllamaService(20);
          if (started) {
            ollamaStarted = true;
            report('Ollama service started successfully');
            await checkOllamaStatus();
            await fetchInstalledModels();
          } else {
            report('Ollama did not start within 20 seconds');
          }
        } catch (e) {
          report(`Failed to start Ollama: ${e}`);
        }
      } else {
        // Try to install it.
        report('Ollama not installed — downloading installer...');
        try {
          await installOllama();
          ollamaInstalled = true;
          report('Ollama installed — starting service...');
          const started = await startOllamaService(30);
          if (started) {
            ollamaStarted = true;
            report('Ollama service started successfully');
            await checkOllamaStatus();
            await fetchInstalledModels();
          }
        } catch (e) {
          report(`Ollama auto-install failed: ${e}`);
        }
      }

      // If still not running after native install/start attempts,
      // try Docker/Podman-based Ollama before falling back to cloud.
      // Priority: Docker Desktop > Podman. Auto-install if neither present.
      if (!ollamaStatus.value.running) {
        let dockerFallbackOk = false;
        try {
          const modelTag = topRecommendation.value?.model_tag
            || ramAwareFallback(systemInfo.value?.total_ram_mb);
          dockerFallbackOk = await tryContainerFallback({
            report,
            modelTag,
            checkOllamaStatus,
            fetchInstalledModels,
            isOllamaRunning: () => ollamaStatus.value.running,
          });
        } catch (e) {
          report(`Container-based Ollama setup failed: ${e}`);
        }

        if (!dockerFallbackOk) {
          report('Could not get Ollama running — using free cloud AI...');
          await autoConfigureForDesktop();
          return {
            mode: 'cloud',
            model: 'Pollinations AI',
            pulled: false,
            ollamaInstalled,
            ollamaStarted,
          };
        }
      }
    }

    // Step 2: Ollama is running — pick the best model
    const top = topRecommendation.value;
    const installed = installedModels.value;
    const topTag = top?.model_tag;

    // If the top-pick for this hardware is already installed, activate it directly.
    if (topTag && installed.some(m => m.name === topTag)) {
      report(`Activating local model: ${topTag}...`);
      try {
        await setBrainMode({ mode: 'local_ollama', model: topTag });

        // Auto-remove weaker auto-configured models (same as Step 3).
        await removeWeakerAutoModels(topTag, installed, report);

        return { mode: 'local', model: topTag, pulled: false };
      } catch {
        report('Failed to activate local model — using free cloud AI...');
        await autoConfigureForDesktop();
        return { mode: 'cloud', model: 'Pollinations AI', pulled: false };
      }
    }

    // Step 3: Top-pick is NOT installed — pull it (even if weaker models exist).
    // A weaker installed model is only used as fallback if the pull fails.
    const modelToPull = topTag || ramAwareFallback(systemInfo.value?.total_ram_mb);
    report(`Downloading recommended model: ${modelToPull}... (this may take a few minutes)`);
    const pullOk = await pullModel(modelToPull);

    if (pullOk) {
      report(`Activating local model: ${modelToPull}...`);
      try {
        await setBrainMode({ mode: 'local_ollama', model: modelToPull });

        // Auto-remove weaker auto-configured models now that a better one is active.
        await removeWeakerAutoModels(modelToPull, installed, report);

        return { mode: 'local', model: modelToPull, pulled: true };
      } catch {
        // Activation failed — fall through to installed fallback
      }
    }

    // Step 4: Pull failed or activation failed — fall back to any installed model.
    const allRecTags = recommendations.value
      .filter(r => !r.is_cloud)
      .map(r => r.model_tag);
    let fallbackModel: string | null = null;
    for (const tag of allRecTags) {
      if (installed.some(m => m.name === tag)) {
        fallbackModel = tag;
        break;
      }
    }
    if (!fallbackModel && installed.length > 0) {
      fallbackModel = installed[0].name;
    }

    if (fallbackModel) {
      report(`Using installed model: ${fallbackModel}...`);
      try {
        await setBrainMode({ mode: 'local_ollama', model: fallbackModel });
        return { mode: 'local', model: fallbackModel, pulled: false };
      } catch {
        // Last resort — cloud
      }
    }

    // Nothing worked — cloud fallback
    report('Model download failed — using free cloud AI...');
    await autoConfigureForDesktop();
    return {
      mode: 'cloud',
      model: 'Pollinations AI',
      pulled: false,
      pullFailed: pullError.value || `Failed to download ${modelToPull}`,
    };
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
        loadProviderPolicy(),
      ]);
    } catch {
      // Tauri backend unavailable in the browser — let the user choose a provider.
      if (typeof window !== 'undefined' && !('__TAURI_INTERNALS__' in window)) {
        prepareBrowserProviderChoices();
      } else {
        autoConfigureFreeApi();
      }
    } finally {
      isLoading.value = false;
    }
  }

  /**
   * Background model-update check — runs once per calendar day.
   * If a better model is available (and not dismissed), pushes an upgrade
   * quest message into the conversation store.
   */
  async function checkForModelUpdates(): Promise<void> {
    try {
      // 1. Read persisted check state (date + dismissed list).
      const [lastDate, dismissed] = await invoke<[string, string[]]>('get_update_check_state');
      const today = new Date().toISOString().slice(0, 10); // YYYY-MM-DD
      if (lastDate === today) return; // Already checked today.

      // 2. Mark today's check done first (prevents re-runs on error).
      await invoke('mark_update_check_done', { date: today });

      // 3. Ask the backend for the update comparison.
      const info = await invoke<{
        has_update: boolean;
        current_model: string;
        recommended_model: string;
        recommended_display: string;
      }>('check_model_updates');

      if (!info.has_update) return;
      if (dismissed.includes(info.recommended_model)) return;

      // 4. Push an upgrade quest message into the chat.
      const { useConversationStore } = await import('./conversation');
      const conversation = useConversationStore();
      conversation.addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content:
          `A better local model is available for your hardware!\n\n` +
          `**Current:** ${info.current_model}\n` +
          `**Recommended:** ${info.recommended_display} (\`${info.recommended_model}\`)\n\n` +
          `Would you like to upgrade?`,
        agentName: 'System',
        sentiment: 'neutral',
        timestamp: Date.now(),
        questId: 'model-update',
        questChoices: [
          { label: 'Upgrade now', value: `model-update:upgrade:${info.recommended_model}`, icon: '🚀' },
          { label: 'Ignore this update', value: `model-update:dismiss:${info.recommended_model}`, icon: '💤' },
        ],
      });
    } catch {
      // Silent failure — this is a background convenience check.
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
        model = mode.model ?? provider.model;
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

  // -------------------------------------------------------------------------
  // Provider policy registry (Chunk 35.1)
  // -------------------------------------------------------------------------

  const providerPolicy = ref<ProviderPolicy>({ version: 1, overrides: {} });

  async function loadProviderPolicy(): Promise<void> {
    try {
      const policy = await invoke<ProviderPolicy>('get_provider_policy');
      providerPolicy.value = policy;
    } catch (e) {
      console.warn('[brain] provider policy load failed:', e);
    }
  }

  async function setProviderPolicy(policy: ProviderPolicy): Promise<void> {
    await invoke<void>('set_provider_policy', { policy });
    providerPolicy.value = policy;
  }

  async function setTaskOverride(taskOverride: TaskOverride): Promise<TaskOverride> {
    const result = await invoke<TaskOverride>('set_provider_task_override', { taskOverride });
    await loadProviderPolicy();
    return result;
  }

  async function removeTaskOverride(kind: TaskKind): Promise<void> {
    await invoke<TaskOverride | null>('remove_provider_task_override', { kind });
    await loadProviderPolicy();
  }

  async function resolveForTask(kind: TaskKind): Promise<ResolvedProvider> {
    return invoke<ResolvedProvider>('resolve_provider_for_task', { kind });
  }

  // ── Agent-Role Routing (Chunk 35.3) ────────────────────────────────────────

  async function getAgentRouting(): Promise<AgentRouteConfig[]> {
    return invoke<AgentRouteConfig[]>('get_agent_routing');
  }

  async function setAgentRoute(config: AgentRouteConfig): Promise<AgentRouteConfig> {
    const result = await invoke<AgentRouteConfig>('set_agent_route', { config });
    await loadProviderPolicy();
    return result;
  }

  async function removeAgentRoute(role: AgentRole): Promise<void> {
    await invoke<AgentRouteConfig | null>('remove_agent_route', { role });
    await loadProviderPolicy();
  }

  async function resolveForRole(role: AgentRole): Promise<ResolvedAgentProvider> {
    return invoke<ResolvedAgentProvider>('resolve_provider_for_role', { role });
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
    browserAuthSession,
    hasBrain,
    topRecommendation,
    isFreeApiMode,
    browserAuthProviders,
    browserAuthProvider,
    prepareBrowserProviderChoices,
    setFallbackProviderModel,
    loadActiveBrain,
    fetchSystemInfo,
    fetchRecommendations,
    refreshModelCatalogue,
    checkOllamaStatus,
    detectOllamaInstall,
    startOllamaService,
    installOllama,
    fetchInstalledModels,
    getOllamaModelsDir,
    getDiskSpace,
    listDrives,
    checkLmStudioStatus,
    fetchLmStudioModels,
    pullModel,
    setActiveBrain,
    warmupLocalOllama,
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
    authoriseBrowserProvider,
    clearBrowserAuthorisation,
    autoConfigureForDesktop,
    autoConfigureLocalFirst,
    initialise,
    checkForModelUpdates,
    processPromptSilently,
    // Provider policy (Chunk 35.1)
    providerPolicy,
    loadProviderPolicy,
    setProviderPolicy,
    setTaskOverride,
    removeTaskOverride,
    resolveForTask,
    // Agent routing (Chunk 35.3)
    getAgentRouting,
    setAgentRoute,
    removeAgentRoute,
    resolveForRole,
  };
});
