import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { ManifestInfo } from '../types';

export const usePackageStore = defineStore('package', () => {
  const currentManifest = ref<ManifestInfo | null>(null);
  const error = ref<string | null>(null);
  const isLoading = ref(false);

  async function parseManifest(json: string): Promise<ManifestInfo | null> {
    isLoading.value = true;
    error.value = null;
    try {
      const info = await invoke<ManifestInfo>('parse_agent_manifest', { json });
      currentManifest.value = info;
      return info;
    } catch (err) {
      error.value = String(err);
      currentManifest.value = null;
      return null;
    } finally {
      isLoading.value = false;
    }
  }

  async function validateManifest(json: string): Promise<boolean> {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('validate_agent_manifest', { json });
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    } finally {
      isLoading.value = false;
    }
  }

  async function getIpcProtocolRange(): Promise<[number, number] | null> {
    try {
      const range = await invoke<[number, number]>('get_ipc_protocol_range');
      error.value = null;
      return range;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  function clearManifest() {
    currentManifest.value = null;
    error.value = null;
  }

  function clearError() {
    error.value = null;
  }

  return {
    currentManifest,
    error,
    isLoading,
    parseManifest,
    validateManifest,
    getIpcProtocolRange,
    clearManifest,
    clearError,
  };
});
