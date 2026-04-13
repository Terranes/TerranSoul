import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type {
  ModelRecommendation,
  OllamaModelEntry,
  OllamaStatus,
  SystemInfo,
} from '../types';

export const useBrainStore = defineStore('brain', () => {
  const activeBrain = ref<string | null>(null);
  const systemInfo = ref<SystemInfo | null>(null);
  const recommendations = ref<ModelRecommendation[]>([]);
  const ollamaStatus = ref<OllamaStatus>({ running: false, model_count: 0 });
  const installedModels = ref<OllamaModelEntry[]>([]);
  const isPulling = ref(false);
  const pullError = ref<string | null>(null);
  const isLoading = ref(false);

  const hasBrain = computed(() => activeBrain.value !== null);
  const topRecommendation = computed(() =>
    recommendations.value.find((m) => m.is_top_pick) ?? recommendations.value[0] ?? null,
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

  /** Full initialisation for the brain setup wizard. */
  async function initialise(): Promise<void> {
    isLoading.value = true;
    try {
      await Promise.all([
        loadActiveBrain(),
        fetchSystemInfo(),
        fetchRecommendations(),
        checkOllamaStatus(),
        fetchInstalledModels(),
      ]);
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
    hasBrain,
    topRecommendation,
    loadActiveBrain,
    fetchSystemInfo,
    fetchRecommendations,
    checkOllamaStatus,
    fetchInstalledModels,
    pullModel,
    setActiveBrain,
    clearActiveBrain,
    initialise,
  };
});
