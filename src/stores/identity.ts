import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { DeviceInfo, TrustedDevice } from '../types';

export const useIdentityStore = defineStore('identity', () => {
  const deviceInfo = ref<DeviceInfo | null>(null);
  const qrSvg = ref<string | null>(null);
  const trustedDevices = ref<TrustedDevice[]>([]);
  const error = ref<string | null>(null);
  const isLoading = ref(false);

  async function loadIdentity() {
    isLoading.value = true;
    error.value = null;
    try {
      deviceInfo.value = await invoke<DeviceInfo>('get_device_identity');
    } catch (err) {
      error.value = String(err);
    } finally {
      isLoading.value = false;
    }
  }

  async function loadPairingQr() {
    isLoading.value = true;
    error.value = null;
    try {
      qrSvg.value = await invoke<string>('get_pairing_qr');
    } catch (err) {
      error.value = String(err);
    } finally {
      isLoading.value = false;
    }
  }

  async function loadTrustedDevices() {
    try {
      trustedDevices.value = await invoke<TrustedDevice[]>('list_trusted_devices');
    } catch (err) {
      error.value = String(err);
    }
  }

  async function addTrustedDevice(device: TrustedDevice) {
    try {
      await invoke('add_trusted_device_cmd', { device });
      await loadTrustedDevices();
    } catch (err) {
      error.value = String(err);
      throw err;
    }
  }

  async function removeTrustedDevice(deviceId: string) {
    try {
      await invoke('remove_trusted_device_cmd', { deviceId });
      await loadTrustedDevices();
    } catch (err) {
      error.value = String(err);
      throw err;
    }
  }

  function clearError() {
    error.value = null;
  }

  return {
    deviceInfo,
    qrSvg,
    trustedDevices,
    error,
    isLoading,
    loadIdentity,
    loadPairingQr,
    loadTrustedDevices,
    addTrustedDevice,
    removeTrustedDevice,
    clearError,
  };
});
