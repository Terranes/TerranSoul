import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { CharacterState } from '../types';

export const useCharacterStore = defineStore('character', () => {
  const state = ref<CharacterState>('idle');
  const vrmPath = ref<string | undefined>(undefined);

  function setState(newState: CharacterState) {
    state.value = newState;
  }

  async function loadVrm(path: string) {
    try {
      await invoke('load_vrm', { path });
      vrmPath.value = path;
    } catch (err) {
      console.error('Failed to load VRM:', err);
    }
  }

  return { state, vrmPath, setState, loadVrm };
});
