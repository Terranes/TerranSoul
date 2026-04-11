import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface SystemInfo {
  cpu_name: string;
  total_ram_mb: number;
}

export interface ModelRecommendation {
  model_tag: string;
  display_name: string;
  description: string;
  required_ram_mb: number;
  is_top_pick: boolean;
}

export interface OllamaStatus {
  running: boolean;
}

export interface OllamaModel {
  name: string;
}

export const useBrainStore = defineStore('brain', () => {
  const systemInfo = ref<SystemInfo | null>(null);
  const recommendations = ref<ModelRecommendation[]>([]);
  const ollamaStatus = ref<OllamaStatus>({ running: false });
  const installedModels = ref<OllamaModel[]>([]);
  const activeBrain = ref<string | null>(null);
  const isPulling = ref(false);
  const pullError = ref<string | null>(null);

  const hasBrain = computed(() => activeBrain.value !== null);

  const topRecommendation = computed(() =>
    recommendations.value.find((m) => m.is_top_pick) ?? null,
  );

  async function initialise() {
    try {
      systemInfo.value = await invoke<SystemInfo>('get_system_info');
    } catch {
      // Backend not available
    }
    try {
      const ram = systemInfo.value?.total_ram_mb ?? 0;
      recommendations.value = await invoke<ModelRecommendation[]>('recommend_models', {
        totalRamMb: ram,
      });
    } catch {
      // Backend not available
    }
    await checkOllamaStatus();
    await loadInstalledModels();
  }

  async function checkOllamaStatus() {
    try {
      ollamaStatus.value = await invoke<OllamaStatus>('get_ollama_status');
    } catch {
      ollamaStatus.value = { running: false };
    }
  }

  async function loadInstalledModels() {
    try {
      installedModels.value = await invoke<OllamaModel[]>('list_ollama_models');
    } catch {
      installedModels.value = [];
    }
  }

  async function pullModel(modelTag: string): Promise<boolean> {
    isPulling.value = true;
    pullError.value = null;
    try {
      await invoke('pull_model', { modelTag });
      await loadInstalledModels();
      isPulling.value = false;
      return true;
    } catch (e) {
      pullError.value = String(e);
      isPulling.value = false;
      return false;
    }
  }

  async function setActiveBrain(modelTag: string) {
    try {
      await invoke('set_active_model', { modelTag });
    } catch {
      // Backend not available — set locally
    }
    activeBrain.value = modelTag;
  }

  async function loadActiveBrain() {
    try {
      const model = await invoke<string | null>('get_active_model');
      if (model) activeBrain.value = model;
    } catch {
      // Backend not available
    }
  }

  return {
    systemInfo,
    recommendations,
    ollamaStatus,
    installedModels,
    activeBrain,
    isPulling,
    pullError,
    hasBrain,
    topRecommendation,
    initialise,
    checkOllamaStatus,
    loadInstalledModels,
    pullModel,
    setActiveBrain,
    loadActiveBrain,
  };
});
