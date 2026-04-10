import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { CharacterState, VrmMetadata } from '../types';

export const useCharacterStore = defineStore('character', () => {
  const state = ref<CharacterState>('idle');
  const vrmPath = ref<string | undefined>(undefined);
  const vrmMetadata = ref<VrmMetadata | undefined>(undefined);
  const loadError = ref<string | undefined>(undefined);

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

  function resetCharacter() {
    vrmPath.value = undefined;
    vrmMetadata.value = undefined;
    loadError.value = undefined;
  }

  return { state, vrmPath, vrmMetadata, loadError, setState, setMetadata, setLoadError, loadVrm, resetCharacter };
});
