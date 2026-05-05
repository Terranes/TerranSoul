import { ref, computed } from 'vue';
import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';

export interface DiscoveredBrain {
  brain_name: string;
  host: string;
  port: number;
  provider: string;
  memory_count: number;
  read_only: boolean;
  hostname: string;
}

export interface RemoteBrainConnection {
  id: string;
  host: string;
  port: number;
  token: string;
  brain_name: string;
  connected: boolean;
}

export interface RemoteSearchResult {
  id: number;
  content: string;
  tags: string | null;
  importance: number;
  score: number;
  source_url: string | null;
  tier: string;
}

export interface TaggedRemoteResult {
  connection_id: string;
  brain_name: string;
  result: RemoteSearchResult;
}

export interface LanShareStatus {
  hosting: boolean;
  brain_name: string | null;
  port: number | null;
  token: string | null;
  connected_brains: number;
  connections: RemoteBrainConnection[];
}

export interface RemoteBrainHealth {
  version: string;
  brain_provider: string | null;
  brain_model: string | null;
  rag_quality_pct: number | null;
  memory_total: number | null;
}

export type LanShareMode = 'idle' | 'hosting' | 'discovering' | 'connected';

export const useLanShareStore = defineStore('lan-share', () => {
  // ─── State ─────────────────────────────────────────────────────────
  const hosting = ref(false);
  const hostBrainName = ref('');
  const hostPort = ref<number | null>(null);
  const hostToken = ref<string | null>(null);
  const discovering = ref(false);
  const discovered = ref<DiscoveredBrain[]>([]);
  const connections = ref<RemoteBrainConnection[]>([]);
  const error = ref<string | null>(null);
  const loading = ref(false);

  // ─── Computed ──────────────────────────────────────────────────────
  const mode = computed<LanShareMode>(() => {
    if (hosting.value) return 'hosting';
    if (connections.value.length > 0) return 'connected';
    if (discovering.value) return 'discovering';
    return 'idle';
  });

  const connectedCount = computed(() => connections.value.length);

  // ─── Actions ───────────────────────────────────────────────────────

  /** Start sharing this brain on the LAN. */
  async function startHosting(brainName: string): Promise<void> {
    error.value = null;
    loading.value = true;
    try {
      const status = await invoke<LanShareStatus>('lan_share_start', {
        brainName,
      });
      hosting.value = true;
      hostBrainName.value = brainName;
      hostPort.value = status.port ?? null;
      hostToken.value = status.token ?? null;
    } catch (e) {
      error.value = String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  /** Stop sharing on the LAN. */
  async function stopHosting(): Promise<void> {
    error.value = null;
    try {
      await invoke('lan_share_stop');
      hosting.value = false;
      hostBrainName.value = '';
      hostPort.value = null;
      hostToken.value = null;
    } catch (e) {
      error.value = String(e);
    }
  }

  /** Discover brains on the LAN via UDP broadcast. */
  async function startDiscovery(): Promise<DiscoveredBrain[]> {
    error.value = null;
    discovering.value = true;
    loading.value = true;
    try {
      const brains = await invoke<DiscoveredBrain[]>('lan_share_discover');
      discovered.value = brains;
      return brains;
    } catch (e) {
      error.value = String(e);
      return [];
    } finally {
      loading.value = false;
    }
  }

  /** Stop UDP broadcast discovery. */
  async function stopDiscovery(): Promise<void> {
    try {
      await invoke('lan_share_stop_discovery');
      discovering.value = false;
      discovered.value = [];
    } catch (e) {
      error.value = String(e);
    }
  }

  /** Connect to a remote brain. */
  async function connect(
    host: string,
    port: number,
    token: string,
    brainName?: string,
  ): Promise<RemoteBrainConnection | null> {
    error.value = null;
    loading.value = true;
    try {
      const conn = await invoke<RemoteBrainConnection>('lan_share_connect', {
        host,
        port,
        token,
        brainName: brainName ?? null,
      });
      connections.value.push(conn);
      return conn;
    } catch (e) {
      error.value = String(e);
      return null;
    } finally {
      loading.value = false;
    }
  }

  /** Disconnect from a remote brain. */
  async function disconnect(connectionId: string): Promise<void> {
    error.value = null;
    try {
      await invoke('lan_share_disconnect', { id: connectionId });
      connections.value = connections.value.filter((c) => c.id !== connectionId);
    } catch (e) {
      error.value = String(e);
    }
  }

  /** Search a specific connected remote brain. */
  async function searchRemote(
    connectionId: string,
    query: string,
    limit?: number,
  ): Promise<RemoteSearchResult[]> {
    error.value = null;
    try {
      return await invoke<RemoteSearchResult[]>('lan_share_search', {
        connectionId,
        query,
        limit: limit ?? null,
      });
    } catch (e) {
      error.value = String(e);
      return [];
    }
  }

  /** Search ALL connected remote brains simultaneously. */
  async function searchAll(
    query: string,
    limit?: number,
  ): Promise<TaggedRemoteResult[]> {
    error.value = null;
    try {
      return await invoke<TaggedRemoteResult[]>('lan_share_search_all', {
        query,
        limit: limit ?? null,
      });
    } catch (e) {
      error.value = String(e);
      return [];
    }
  }

  /** Get health info for a connected remote brain. */
  async function getRemoteHealth(
    connectionId: string,
  ): Promise<RemoteBrainHealth | null> {
    try {
      return await invoke<RemoteBrainHealth>('lan_share_remote_health', {
        connectionId,
      });
    } catch (e) {
      error.value = String(e);
      return null;
    }
  }

  /** Refresh status from the backend. */
  async function refreshStatus(): Promise<void> {
    try {
      const status = await invoke<LanShareStatus>('lan_share_status');
      hosting.value = status.hosting;
      hostPort.value = status.port ?? null;
      hostToken.value = status.token ?? null;
      connections.value = status.connections;
    } catch (e) {
      error.value = String(e);
    }
  }

  return {
    // State
    hosting,
    hostBrainName,
    hostPort,
    hostToken,
    discovering,
    discovered,
    connections,
    error,
    loading,
    // Computed
    mode,
    connectedCount,
    // Actions
    startHosting,
    stopHosting,
    startDiscovery,
    stopDiscovery,
    connect,
    disconnect,
    searchRemote,
    searchAll,
    getRemoteHealth,
    refreshStatus,
  };
});
