/**
 * Pinia store for agent sandbox capability management.
 */
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { CapabilityName, ConsentInfo } from '../types';

export const useSandboxStore = defineStore('sandbox', () => {
  const consents = ref<ConsentInfo[]>([]);
  const error = ref<string | null>(null);
  const isLoading = ref(false);

  async function grantCapability(agentName: string, capability: CapabilityName): Promise<boolean> {
    error.value = null;
    try {
      await invoke('grant_agent_capability', { agentName, capability });
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  async function revokeCapability(agentName: string, capability: CapabilityName): Promise<boolean> {
    error.value = null;
    try {
      await invoke('revoke_agent_capability', { agentName, capability });
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  async function listCapabilities(agentName: string): Promise<ConsentInfo[]> {
    isLoading.value = true;
    error.value = null;
    try {
      const records = await invoke<ConsentInfo[]>('list_agent_capabilities', { agentName });
      consents.value = records;
      return records;
    } catch (err) {
      error.value = String(err);
      return [];
    } finally {
      isLoading.value = false;
    }
  }

  async function clearCapabilities(agentName: string): Promise<boolean> {
    error.value = null;
    try {
      await invoke('clear_agent_capabilities', { agentName });
      consents.value = consents.value.filter((c) => c.agent_name !== agentName);
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  async function runInSandbox(agentName: string, wasmBytes: number[]): Promise<number | null> {
    isLoading.value = true;
    error.value = null;
    try {
      const result = await invoke<number>('run_agent_in_sandbox', { agentName, wasmBytes });
      return result;
    } catch (err) {
      error.value = String(err);
      return null;
    } finally {
      isLoading.value = false;
    }
  }

  function clearError() {
    error.value = null;
  }

  return {
    consents,
    error,
    isLoading,
    grantCapability,
    revokeCapability,
    listCapabilities,
    clearCapabilities,
    runInSandbox,
    clearError,
  };
});
