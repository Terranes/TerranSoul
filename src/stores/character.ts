import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { CharacterState, VrmMetadata } from '../types';
import { DEFAULT_MODELS, DEFAULT_MODEL_ID, GENDER_VOICES, type DefaultModel, type ModelGender } from '../config/default-models';
import { useSettingsStore } from './settings';

/** A VRM model the user imported. Stored under `<app_data_dir>/user_models/`
 *  by the Rust backend so it survives a fresh build / app upgrade. */
export interface UserModel {
  id: string;
  name: string;
  original_filename: string;
  gender: ModelGender;
  persona: string;
  imported_at: number;
}

export const useCharacterStore = defineStore('character', () => {
  const state = ref<CharacterState>('idle');
  /** Intensity of the current emotion in [0, 1]. Defaults to 1. */
  const emotionIntensity = ref(1);
  const vrmPath = ref<string | undefined>(undefined);
  const vrmMetadata = ref<VrmMetadata | undefined>(undefined);
  const loadError = ref<string | undefined>(undefined);
  const isLoading = ref(true);
  const selectedModelId = ref<string>(DEFAULT_MODEL_ID);
  const defaultModels = ref<DefaultModel[]>(DEFAULT_MODELS);
  const userModels = ref<UserModel[]>([]);

  /** Currently active blob URL — revoked before a new one is created
   *  to avoid leaking object URLs across model switches. */
  let activeBlobUrl: string | undefined;

  /** All models the picker should show: bundled defaults followed by
   *  user-imported VRMs (newest first). */
  const allModels = computed<Array<DefaultModel | UserModel>>(() => [
    ...defaultModels.value,
    ...userModels.value,
  ]);

  function isUserModel(model: DefaultModel | UserModel): model is UserModel {
    return (model as UserModel).original_filename !== undefined;
  }

  function findModel(id: string): DefaultModel | UserModel | undefined {
    return defaultModels.value.find(m => m.id === id)
      ?? userModels.value.find(m => m.id === id);
  }

  function setState(newState: CharacterState, intensity: number = 1) {
    state.value = newState;
    emotionIntensity.value = Math.max(0, Math.min(1, intensity));
  }

  function setMetadata(metadata: VrmMetadata) {
    vrmMetadata.value = metadata;
  }

  function setLoadError(error: string | undefined) {
    loadError.value = error;
  }

  function revokeActiveBlobUrl() {
    if (activeBlobUrl) {
      try { URL.revokeObjectURL(activeBlobUrl); } catch { /* noop */ }
      activeBlobUrl = undefined;
    }
  }

  async function loadVrm(path: string) {
    loadError.value = undefined;
    vrmMetadata.value = undefined;
    isLoading.value = true;
    revokeActiveBlobUrl();
    // Set the path immediately so the viewport watcher can start loading via Three.js
    vrmPath.value = path;
    // Notify the backend (fire-and-forget; the real 3D load happens in the viewport)
    try {
      await invoke('load_vrm', { path });
    } catch {
      // Backend not available (e.g. pure browser dev) — frontend loading still works
    }
  }

  /** Load a user-imported VRM. Pulls bytes from the Rust backend and wraps
   *  them in a Blob URL so `GLTFLoader` can fetch them like any other URL. */
  async function loadUserVrm(model: UserModel) {
    loadError.value = undefined;
    vrmMetadata.value = undefined;
    isLoading.value = true;
    try {
      const bytes = await invoke<number[] | Uint8Array>('read_user_model_bytes', { id: model.id });
      const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
      const blob = new Blob([u8 as BlobPart], { type: 'model/gltf-binary' });
      revokeActiveBlobUrl();
      const url = URL.createObjectURL(blob);
      activeBlobUrl = url;
      vrmPath.value = url;
    } catch (err) {
      loadError.value = `Failed to load imported model: ${err}`;
      isLoading.value = false;
      throw err;
    }
  }

  /** Get the gender of the currently selected model (default or user). */
  function currentGender(): ModelGender {
    const model = findModel(selectedModelId.value);
    return model?.gender ?? 'female';
  }

  async function selectModel(modelId: string) {
    const model = findModel(modelId);
    if (!model) return;
    selectedModelId.value = modelId;
    if (isUserModel(model)) {
      await loadUserVrm(model);
    } else {
      await loadVrm(model.path);
    }
    // Set TTS voice and prosody to match the character's gender
    const gender: ModelGender = model.gender ?? 'female';
    const voiceInfo = GENDER_VOICES[gender];
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

  /** Refresh the user-model list from the Rust backend (called on startup). */
  async function loadUserModels(): Promise<void> {
    try {
      userModels.value = await invoke<UserModel[]>('list_user_models');
    } catch {
      // Tauri unavailable (e.g. browser dev) — keep empty list
    }
  }

  /** Import a VRM from an absolute filesystem path. The backend copies the
   *  bytes into `<app_data_dir>/user_models/` so the model persists across
   *  fresh builds. Returns the new entry. */
  async function importUserModel(
    sourcePath: string,
    opts?: { name?: string; gender?: ModelGender; persona?: string },
  ): Promise<UserModel> {
    const entry = await invoke<UserModel>('import_user_model', {
      sourcePath,
      name: opts?.name,
      gender: opts?.gender,
      persona: opts?.persona,
    });
    userModels.value = [...userModels.value, entry];
    return entry;
  }

  /** Update a user model's name, gender, and/or persona. */
  async function updateUserModel(
    id: string,
    opts: { name?: string; gender?: ModelGender; persona?: string },
  ): Promise<UserModel> {
    const updated = await invoke<UserModel>('update_user_model', {
      id,
      name: opts.name,
      gender: opts.gender,
      persona: opts.persona,
    });
    userModels.value = userModels.value.map((m) => (m.id === id ? updated : m));
    return updated;
  }

  /** Delete a user-imported model (file + metadata). If it was the active
   *  model, fall back to the bundled default. */
  async function deleteUserModel(id: string): Promise<void> {
    await invoke('delete_user_model', { id });
    userModels.value = userModels.value.filter(m => m.id !== id);
    if (selectedModelId.value === id) {
      await selectModel(DEFAULT_MODEL_ID);
    }
  }

  function setLoaded() {
    isLoading.value = false;
  }

  function resetCharacter() {
    revokeActiveBlobUrl();
    vrmPath.value = undefined;
    vrmMetadata.value = undefined;
    loadError.value = undefined;
    isLoading.value = false;
    selectedModelId.value = DEFAULT_MODEL_ID;
  }

  return {
    state, emotionIntensity, vrmPath, vrmMetadata, loadError, isLoading, selectedModelId,
    defaultModels, userModels, allModels,
    setState, setMetadata, setLoadError, setLoaded,
    loadVrm, selectModel, loadDefaultModel, resetCharacter, currentGender,
    loadUserModels, importUserModel, updateUserModel, deleteUserModel,
  };
});
