import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { CharacterState, VrmMetadata } from '../types';
import { DEFAULT_MODELS, DEFAULT_MODEL_ID, type DefaultModel } from '../config/default-models';
import { useSettingsStore } from './settings';

export const useCharacterStore = defineStore('character', () => {
  const state = ref<CharacterState>('idle');
  const vrmPath = ref<string | undefined>(undefined);
  const vrmMetadata = ref<VrmMetadata | undefined>(undefined);
  const loadError = ref<string | undefined>(undefined);
  const isLoading = ref(true);
  const selectedModelId = ref<string>(DEFAULT_MODEL_ID);
  const defaultModels = ref<DefaultModel[]>(DEFAULT_MODELS);

  function setState(newState: CharacterState) {
    state.value = newState;
  }

  function setMetadata(metadata: VrmMetadata) {
    vrmMetadata.value = metadata;
  }

  function setLoadError(error: string | undefined) {
    loadError.value = error;
  }

  async function loadVrm(path: string) {
    loadError.value = undefined;
    vrmMetadata.value = undefined;
    isLoading.value = true;
    // Set the path immediately so the viewport watcher can start loading via Three.js
    vrmPath.value = path;
    // Notify the backend (fire-and-forget; the real 3D load happens in the viewport)
    try {
      await invoke('load_vrm', { path });
    } catch {
      // Backend not available (e.g. pure browser dev) — frontend loading still works
    }
  }

  async function selectModel(modelId: string) {
    const model = DEFAULT_MODELS.find(m => m.id === modelId);
    if (!model) return;
    selectedModelId.value = modelId;
    await loadVrm(model.path);
    // Persist the model selection across sessions
    try {
      const settingsStore = useSettingsStore();
      await settingsStore.saveModelId(modelId);
    } catch {
      // Settings store unavailable — not critical
    }
  }

  async function loadDefaultModel() {
    await selectModel(DEFAULT_MODEL_ID);
  }

  function setLoaded() {
    isLoading.value = false;
  }

  function resetCharacter() {
    vrmPath.value = undefined;
    vrmMetadata.value = undefined;
    loadError.value = undefined;
    isLoading.value = false;
    selectedModelId.value = DEFAULT_MODEL_ID;
  }

  return { state, vrmPath, vrmMetadata, loadError, isLoading, selectedModelId, defaultModels, setState, setMetadata, setLoadError, setLoaded, loadVrm, selectModel, loadDefaultModel, resetCharacter };
});
