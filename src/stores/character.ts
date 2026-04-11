import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { CharacterState, VrmMetadata } from '../types';
import { DEFAULT_MODELS, DEFAULT_MODEL_ID, type DefaultModel } from '../config/default-models';

export const useCharacterStore = defineStore('character', () => {
  const state = ref<CharacterState>('idle');
  const vrmPath = ref<string | undefined>(undefined);
  const vrmMetadata = ref<VrmMetadata | undefined>(undefined);
  const loadError = ref<string | undefined>(undefined);
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
    try {
      await invoke('load_vrm', { path });
      vrmPath.value = path;
    } catch (err) {
      const message = String(err);
      loadError.value = message;
      console.error('Failed to load VRM:', message);
    }
  }

  async function selectModel(modelId: string) {
    const model = DEFAULT_MODELS.find(m => m.id === modelId);
    if (!model) return;
    selectedModelId.value = modelId;
    await loadVrm(model.path);
  }

  async function loadDefaultModel() {
    await selectModel(DEFAULT_MODEL_ID);
  }

  function resetCharacter() {
    vrmPath.value = undefined;
    vrmMetadata.value = undefined;
    loadError.value = undefined;
    selectedModelId.value = DEFAULT_MODEL_ID;
  }

  return { state, vrmPath, vrmMetadata, loadError, selectedModelId, defaultModels, setState, setMetadata, setLoadError, loadVrm, selectModel, loadDefaultModel, resetCharacter };
});
