import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { CharacterState, VrmMetadata } from '../types';
import { DEFAULT_MODELS, DEFAULT_MODEL_ID, GENDER_VOICES, type DefaultModel, type ModelGender } from '../config/default-models';
import { useSettingsStore } from './settings';

export const useCharacterStore = defineStore('character', () => {
  const state = ref<CharacterState>('idle');
  const vrmPath = ref<string | undefined>(undefined);
  const vrmMetadata = ref<VrmMetadata | undefined>(undefined);
  const loadError = ref<string | undefined>(undefined);
  const isLoading = ref(true);
  const selectedModelId = ref<string>(DEFAULT_MODEL_ID);
  const defaultModels = ref<DefaultModel[]>(DEFAULT_MODELS);
  /** When true, the character-animator pins the idle pose rotation to the
   *  seated pose and the sofa + teacup props are visible. Set via the Mood
   *  submenu or via the animator when it rotates into the seated idle. */
  const sittingPinned = ref(false);

  function setState(newState: CharacterState) {
    state.value = newState;
  }

  function setSittingPinned(pinned: boolean) {
    sittingPinned.value = pinned;
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

  /** Get the gender of the currently selected model. */
  function currentGender(): ModelGender {
    const model = DEFAULT_MODELS.find(m => m.id === selectedModelId.value);
    return model?.gender ?? 'female';
  }

  async function selectModel(modelId: string) {
    const model = DEFAULT_MODELS.find(m => m.id === modelId);
    if (!model) return;
    selectedModelId.value = modelId;
    await loadVrm(model.path);
    // Set TTS voice and prosody to match the character's gender
    const voiceInfo = GENDER_VOICES[model.gender ?? 'female'];
    try {
      await invoke('set_tts_voice', { voiceName: voiceInfo.edgeVoice });
      await invoke('set_tts_prosody', { pitch: voiceInfo.edgePitch, rate: voiceInfo.edgeRate });
    } catch {
      // Tauri unavailable — voice will use browser fallback pitch instead
    }
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

  return { state, vrmPath, vrmMetadata, loadError, isLoading, selectedModelId, defaultModels, sittingPinned, setState, setSittingPinned, setMetadata, setLoadError, setLoaded, loadVrm, selectModel, loadDefaultModel, resetCharacter, currentGender };
});
