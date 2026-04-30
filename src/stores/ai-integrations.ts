/**
 * AI Coding Integrations control-plane store (Chunk 15.4).
 *
 * Wraps the existing MCP / auto-setup / vscode-workspace Tauri
 * commands behind a single Pinia store so the
 * `AICodingIntegrationsView` view can stay thin.
 *
 * No persistence here — server status is read live from the backend
 * each time the user opens the panel; client-config status is read
 * from `list_mcp_clients`.
 */

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface McpServerStatus {
  running: boolean;
  port: number | null;
  token: string | null;
  is_dev: boolean;
}

export interface ClientStatus {
  client: string;
  configured: boolean;
  config_path: string | null;
}

export interface SetupResult {
  /** Backend `auto_setup::SetupResult` shape; we don't enumerate
   * variants because the UI only needs the human-readable message. */
  message: string;
  config_path?: string;
}

export type Transport = 'http' | 'stdio';

export interface VsCodeWindow {
  pid: number;
  root: string;
  launched_at_ms: number;
  launched_by: string;
}

export const useAiIntegrationsStore = defineStore('ai-integrations', () => {
  const serverStatus = ref<McpServerStatus | null>(null);
  const clientStatuses = ref<ClientStatus[]>([]);
  const vscodeWindows = ref<VsCodeWindow[]>([]);
  const loading = ref(false);
  /** Generic error from the most recent action; cleared on next successful op. */
  const error = ref<string | null>(null);
  /** Transport preference for auto-setup buttons. Defaults to stdio
   * (canonical since 15.9). */
  const preferredTransport = ref<Transport>('stdio');

  const isRunning = computed(() => serverStatus.value?.running === true);
  const port = computed(() => serverStatus.value?.port ?? null);
  /** Truncated token preview for display (first 8 chars + "…"). */
  const tokenPreview = computed(() => {
    const t = serverStatus.value?.token;
    if (!t) return null;
    return t.length > 12 ? `${t.slice(0, 8)}…` : t;
  });

  async function refreshStatus(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      serverStatus.value = await invoke<McpServerStatus>('mcp_server_status');
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function refreshClients(workspaceRoot: string): Promise<void> {
    error.value = null;
    try {
      clientStatuses.value = await invoke<ClientStatus[]>('list_mcp_clients', {
        workspaceRoot,
      });
    } catch (e) {
      error.value = String(e);
    }
  }

  async function refreshVscodeWindows(): Promise<void> {
    error.value = null;
    try {
      vscodeWindows.value = await invoke<VsCodeWindow[]>(
        'vscode_list_known_windows',
      );
    } catch (e) {
      error.value = String(e);
    }
  }

  async function startServer(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      serverStatus.value = await invoke<McpServerStatus>('mcp_server_start');
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function stopServer(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      await invoke<void>('mcp_server_stop');
      // Refresh to get the not-running snapshot.
      serverStatus.value = await invoke<McpServerStatus>('mcp_server_status');
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function regenerateToken(): Promise<void> {
    error.value = null;
    try {
      const newToken = await invoke<string>('mcp_regenerate_token');
      // Token is only displayed after restart; surface it now too.
      if (serverStatus.value) {
        serverStatus.value = { ...serverStatus.value, token: newToken };
      }
    } catch (e) {
      error.value = String(e);
    }
  }

  /** Run auto-setup for a given client. Selects HTTP vs stdio command. */
  async function setupClient(
    client: 'vscode' | 'claude' | 'codex',
    workspaceRoot: string,
    transport: Transport = preferredTransport.value,
  ): Promise<SetupResult | null> {
    error.value = null;
    const suffix = transport === 'stdio' ? '_stdio' : '';
    const cmd = `setup_${client}_mcp${suffix}`;
    try {
      const args: Record<string, unknown> = {};
      if (client === 'vscode') {
        args.workspaceRoot = workspaceRoot;
      }
      const result = await invoke<SetupResult>(cmd, args);
      // Refresh the client list so the UI reflects the new state.
      await refreshClients(workspaceRoot);
      return result;
    } catch (e) {
      error.value = String(e);
      return null;
    }
  }

  async function removeClient(
    client: 'vscode' | 'claude' | 'codex',
    workspaceRoot: string,
  ): Promise<SetupResult | null> {
    error.value = null;
    const cmd = `remove_${client}_mcp`;
    try {
      const args: Record<string, unknown> = {};
      if (client === 'vscode') {
        args.workspaceRoot = workspaceRoot;
      }
      const result = await invoke<SetupResult>(cmd, args);
      await refreshClients(workspaceRoot);
      return result;
    } catch (e) {
      error.value = String(e);
      return null;
    }
  }

  async function forgetWindow(pid: number): Promise<void> {
    error.value = null;
    try {
      await invoke<void>('vscode_forget_window', { pid });
      await refreshVscodeWindows();
    } catch (e) {
      error.value = String(e);
    }
  }

  function setTransport(t: Transport): void {
    preferredTransport.value = t;
  }

  return {
    // State
    serverStatus,
    clientStatuses,
    vscodeWindows,
    loading,
    error,
    preferredTransport,
    // Getters
    isRunning,
    port,
    tokenPreview,
    // Actions
    refreshStatus,
    refreshClients,
    refreshVscodeWindows,
    startServer,
    stopServer,
    regenerateToken,
    setupClient,
    removeClient,
    forgetWindow,
    setTransport,
  };
});
