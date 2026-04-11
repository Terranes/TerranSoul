import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { LinkStatusValue, LinkPeer, LinkStatusResponse } from '../types';

export const useLinkStore = defineStore('link', () => {
  const status = ref<LinkStatusValue>('disconnected');
  const transport = ref<string>('Quic');
  const peer = ref<LinkPeer | null>(null);
  const serverPort = ref<number | null>(null);
  const error = ref<string | null>(null);
  const isLoading = ref(false);

  const isConnected = computed(() => status.value === 'connected');

  async function fetchStatus() {
    try {
      const res = await invoke<LinkStatusResponse>('get_link_status');
      status.value = res.status;
      transport.value = res.transport;
      peer.value = res.peer;
      serverPort.value = res.server_port;
      error.value = null;
    } catch (err) {
      error.value = String(err);
    }
  }

  async function startServer(port?: number) {
    isLoading.value = true;
    error.value = null;
    try {
      const boundPort = await invoke<number>('start_link_server', { port: port ?? null });
      serverPort.value = boundPort;
      status.value = 'connecting';
    } catch (err) {
      error.value = String(err);
    } finally {
      isLoading.value = false;
    }
  }

  async function connectToPeer(host: string, port: number, deviceId: string, name: string) {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('connect_to_peer', { host, port, deviceId, name });
      status.value = 'connected';
      peer.value = { device_id: deviceId, name, addr: `${host}:${port}` };
    } catch (err) {
      error.value = String(err);
    } finally {
      isLoading.value = false;
    }
  }

  async function disconnect() {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('disconnect_link');
      status.value = 'disconnected';
      peer.value = null;
      serverPort.value = null;
    } catch (err) {
      error.value = String(err);
    } finally {
      isLoading.value = false;
    }
  }

  function clearError() {
    error.value = null;
  }

  return {
    status,
    transport,
    peer,
    serverPort,
    error,
    isLoading,
    isConnected,
    fetchStatus,
    startServer,
    connectToPeer,
    disconnect,
    clearError,
  };
});
