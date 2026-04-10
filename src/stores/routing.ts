import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { PendingCommand, CommandResultResponse } from '../types';

export const useRoutingStore = defineStore('routing', () => {
  const pendingCommands = ref<PendingCommand[]>([]);
  const lastResult = ref<CommandResultResponse | null>(null);
  const error = ref<string | null>(null);
  const isLoading = ref(false);

  const hasPending = computed(() => pendingCommands.value.length > 0);

  async function fetchPendingCommands() {
    try {
      pendingCommands.value = await invoke<PendingCommand[]>('list_pending_commands');
      error.value = null;
    } catch (err) {
      error.value = String(err);
    }
  }

  async function approveCommand(commandId: string, remember = false) {
    isLoading.value = true;
    error.value = null;
    try {
      const result = await invoke<CommandResultResponse>('approve_remote_command', {
        commandId,
        remember,
      });
      lastResult.value = result;
      await fetchPendingCommands();
    } catch (err) {
      error.value = String(err);
    } finally {
      isLoading.value = false;
    }
  }

  async function denyCommand(commandId: string, block = false) {
    isLoading.value = true;
    error.value = null;
    try {
      const result = await invoke<CommandResultResponse>('deny_remote_command', {
        commandId,
        block,
      });
      lastResult.value = result;
      await fetchPendingCommands();
    } catch (err) {
      error.value = String(err);
    } finally {
      isLoading.value = false;
    }
  }

  async function setDevicePermission(deviceId: string, policy: 'allow' | 'deny' | 'ask') {
    try {
      await invoke('set_device_permission', { deviceId, policy });
      error.value = null;
    } catch (err) {
      error.value = String(err);
    }
  }

  async function getDevicePermissions(): Promise<Array<[string, string]>> {
    try {
      const result = await invoke<Array<[string, string]>>('get_device_permissions');
      error.value = null;
      return result;
    } catch (err) {
      error.value = String(err);
      return [];
    }
  }

  function clearError() {
    error.value = null;
  }

  return {
    pendingCommands,
    lastResult,
    error,
    isLoading,
    hasPending,
    fetchPendingCommands,
    approveCommand,
    denyCommand,
    setDevicePermission,
    getDevicePermissions,
    clearError,
  };
});
