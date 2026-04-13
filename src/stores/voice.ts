import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { VoiceProviderInfo, VoiceConfig } from '../types';

// ── Fallback catalogue for when Tauri backend is unavailable ──────────────────

const FALLBACK_ASR_PROVIDERS: VoiceProviderInfo[] = [
  {
    id: 'web-speech',
    display_name: 'Web Speech API',
    description: 'Browser-native speech recognition. Zero setup.',
    kind: 'local',
    requires_api_key: false,
  },
  {
    id: 'open-llm-vtuber',
    display_name: 'Open-LLM-VTuber',
    description: 'Connect to a running Open-LLM-VTuber server. Supports 7+ ASR engines via WebSocket.',
    kind: 'external',
    requires_api_key: false,
  },
];

const FALLBACK_TTS_PROVIDERS: VoiceProviderInfo[] = [
  {
    id: 'open-llm-vtuber',
    display_name: 'Open-LLM-VTuber',
    description: 'Connect to a running Open-LLM-VTuber server. Supports 18+ TTS engines via WebSocket.',
    kind: 'external',
    requires_api_key: false,
  },
];

export const useVoiceStore = defineStore('voice', () => {
  const asrProviders = ref<VoiceProviderInfo[]>([]);
  const ttsProviders = ref<VoiceProviderInfo[]>([]);
  const config = ref<VoiceConfig>({
    asr_provider: null,
    tts_provider: null,
    api_key: null,
    endpoint_url: null,
  });
  const isLoading = ref(false);
  const error = ref<string | null>(null);

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
      api_key: null,
      endpoint_url: null,
    };
  }

  return {
    // state
    asrProviders,
    ttsProviders,
    config,
    isLoading,
    error,
    // computed
    hasVoice,
    isTextOnly,
    selectedAsrProvider,
    selectedTtsProvider,
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
  };
});
