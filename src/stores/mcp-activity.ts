import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { UnlistenFn } from '@tauri-apps/api/event';

export type McpActivityStatus = 'idle' | 'working' | 'success' | 'error';

export interface McpActivitySnapshot {
  status: McpActivityStatus;
  phase: string;
  message: string;
  toolName: string | null;
  toolTitle: string | null;
  brainProvider: string;
  brainModel: string | null;
  updatedAtMs: number;
  speak: boolean;
}

const DEFAULT_SNAPSHOT: McpActivitySnapshot = {
  status: 'idle',
  phase: 'idle',
  message: 'MCP brain is idle.',
  toolName: null,
  toolTitle: null,
  brainProvider: 'none',
  brainModel: null,
  updatedAtMs: 0,
  speak: false,
};

function titleCaseProvider(provider: string): string {
  if (!provider || provider === 'none') return 'No brain';
  return provider
    .split(/[\s/_-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export const useMcpActivityStore = defineStore('mcpActivity', () => {
  const snapshot = ref<McpActivitySnapshot>({ ...DEFAULT_SNAPSHOT });
  const isListening = ref(false);
  const error = ref<string | null>(null);
  let unlisten: UnlistenFn | null = null;

  const isWorking = computed(() => snapshot.value.status === 'working');
  const statusLabel = computed(() => {
    switch (snapshot.value.status) {
      case 'working': return 'Working';
      case 'success': return 'Ready';
      case 'error': return 'Needs attention';
      default: return 'Idle';
    }
  });
  const modelLabel = computed(() => {
    const provider = titleCaseProvider(snapshot.value.brainProvider);
    return snapshot.value.brainModel
      ? `${provider} - ${snapshot.value.brainModel}`
      : provider;
  });
  const workLabel = computed(() => snapshot.value.toolTitle || snapshot.value.phase || 'MCP');
  const speechText = computed(() => snapshot.value.message.trim());

  function applySnapshot(next: McpActivitySnapshot): void {
    snapshot.value = {
      ...DEFAULT_SNAPSHOT,
      ...next,
      toolName: next.toolName ?? null,
      toolTitle: next.toolTitle ?? null,
      brainModel: next.brainModel ?? null,
    };
  }

  async function loadSnapshot(): Promise<void> {
    try {
      applySnapshot(await invoke<McpActivitySnapshot>('get_mcp_activity'));
      error.value = null;
    } catch (err) {
      error.value = String(err);
    }
  }

  async function initialise(): Promise<void> {
    if (isListening.value) return;
    await loadSnapshot();
    try {
      const { listen } = await import('@tauri-apps/api/event');
      unlisten = await listen<McpActivitySnapshot>('mcp-activity', (event) => {
        applySnapshot(event.payload);
      });
      isListening.value = true;
    } catch (err) {
      error.value = String(err);
    }
  }

  function dispose(): void {
    unlisten?.();
    unlisten = null;
    isListening.value = false;
  }

  return {
    snapshot,
    isListening,
    error,
    isWorking,
    statusLabel,
    modelLabel,
    workLabel,
    speechText,
    loadSnapshot,
    initialise,
    dispose,
  };
});