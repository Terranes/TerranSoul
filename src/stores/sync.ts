import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { SyncState } from '../types';

/**
 * Sync store — local representation of CRDT sync state.
 *
 * In production the Rust backend owns the CRDT data structures and this
 * store mirrors their summary via periodic IPC polling.  For now the store
 * provides the reactive shape that the UI can bind to.
 */
export const useSyncStore = defineStore('sync', () => {
  const conversationCount = ref(0);
  const characterSelection = ref<string | null>(null);
  const agentCount = ref(0);
  const lastSyncedAt = ref<number | null>(null);
  const error = ref<string | null>(null);

  const isSynced = computed(() => lastSyncedAt.value !== null);

  function applySyncState(state: SyncState) {
    conversationCount.value = state.conversation_count;
    characterSelection.value = state.character_selection;
    agentCount.value = state.agent_count;
    lastSyncedAt.value = state.last_synced_at;
    error.value = null;
  }

  function setError(msg: string) {
    error.value = msg;
  }

  function clearError() {
    error.value = null;
  }

  function reset() {
    conversationCount.value = 0;
    characterSelection.value = null;
    agentCount.value = 0;
    lastSyncedAt.value = null;
    error.value = null;
  }

  return {
    conversationCount,
    characterSelection,
    agentCount,
    lastSyncedAt,
    error,
    isSynced,
    applySyncState,
    setError,
    clearError,
    reset,
  };
});
