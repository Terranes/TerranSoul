import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { MemoryEntry, MemoryStats, MemoryTier, NewMemory, Message } from '../types';

export const useMemoryStore = defineStore('memory', () => {
  const memories = ref<MemoryEntry[]>([]);
  const stats = ref<MemoryStats | null>(null);
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  async function fetchAll(): Promise<void> {
    isLoading.value = true;
    error.value = null;
    try {
      memories.value = await invoke<MemoryEntry[]>('get_memories');
    } catch (e) {
      error.value = String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function search(query: string): Promise<MemoryEntry[]> {
    try {
      return await invoke<MemoryEntry[]>('search_memories', { query });
    } catch {
      return [];
    }
  }

  /** Brain-powered semantic search — uses the active Ollama model. */
  async function semanticSearch(query: string, limit = 10): Promise<MemoryEntry[]> {
    try {
      return await invoke<MemoryEntry[]>('semantic_search_memories', { query, limit });
    } catch {
      return [];
    }
  }

  async function addMemory(m: NewMemory): Promise<MemoryEntry | null> {
    try {
      const entry = await invoke<MemoryEntry>('add_memory', {
        content: m.content,
        tags: m.tags,
        importance: m.importance,
        memoryType: m.memory_type,
      });
      memories.value.unshift(entry);
      return entry;
    } catch (e) {
      error.value = String(e);
      return null;
    }
  }

  async function updateMemory(
    id: number,
    patch: Partial<Pick<MemoryEntry, 'content' | 'tags' | 'importance' | 'memory_type'>>,
  ): Promise<MemoryEntry | null> {
    try {
      const updated = await invoke<MemoryEntry>('update_memory', {
        id,
        content: patch.content ?? null,
        tags: patch.tags ?? null,
        importance: patch.importance ?? null,
        memoryType: patch.memory_type ?? null,
      });
      const idx = memories.value.findIndex((m) => m.id === id);
      if (idx !== -1) memories.value[idx] = updated;
      return updated;
    } catch (e) {
      error.value = String(e);
      return null;
    }
  }

  async function deleteMemory(id: number): Promise<boolean> {
    try {
      await invoke('delete_memory', { id });
      memories.value = memories.value.filter((m) => m.id !== id);
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    }
  }

  /** Ask the brain to extract memories from the current session. */
  async function extractFromSession(): Promise<number> {
    try {
      const count = await invoke<number>('extract_memories_from_session');
      if (count > 0) await fetchAll();
      return count;
    } catch (e) {
      error.value = String(e);
      return 0;
    }
  }

  /** Ask the brain to summarize the current session into one memory entry. */
  async function summarizeSession(): Promise<string | null> {
    try {
      const summary = await invoke<string>('summarize_session');
      await fetchAll();
      return summary;
    } catch (e) {
      error.value = String(e);
      return null;
    }
  }

  async function getShortTermMemory(limit = 20): Promise<Message[]> {
    try {
      return await invoke<Message[]>('get_short_term_memory', { limit });
    } catch {
      return [];
    }
  }

  /** Hybrid search: vector + keyword + recency + importance + decay scoring. */
  async function hybridSearch(query: string, limit = 5): Promise<MemoryEntry[]> {
    try {
      return await invoke<MemoryEntry[]>('hybrid_search_memories', { query, limit });
    } catch {
      return [];
    }
  }

  /** Get memory statistics per tier. */
  async function getStats(): Promise<MemoryStats | null> {
    try {
      const s = await invoke<MemoryStats>('get_memory_stats');
      stats.value = s;
      return s;
    } catch {
      return null;
    }
  }

  /** Apply time-based decay to all memory scores. */
  async function applyDecay(): Promise<number> {
    try {
      return await invoke<number>('apply_memory_decay');
    } catch {
      return 0;
    }
  }

  /** Garbage-collect fully-decayed memories (decay_score ≤ threshold). */
  async function gcMemories(threshold = 0.01): Promise<number> {
    try {
      const count = await invoke<number>('gc_memories', { threshold });
      if (count > 0) await fetchAll();
      return count;
    } catch {
      return 0;
    }
  }

  /** Promote a memory to a higher tier. */
  async function promoteMemory(id: number, tier: MemoryTier): Promise<boolean> {
    try {
      await invoke('promote_memory', { id, tier });
      const idx = memories.value.findIndex((m) => m.id === id);
      if (idx !== -1) memories.value[idx].tier = tier;
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    }
  }

  /** Get memories filtered by tier. */
  async function getByTier(tier: MemoryTier): Promise<MemoryEntry[]> {
    try {
      return await invoke<MemoryEntry[]>('get_memories_by_tier', { tier });
    } catch {
      return [];
    }
  }

  return {
    memories,
    stats,
    isLoading,
    error,
    fetchAll,
    search,
    semanticSearch,
    hybridSearch,
    addMemory,
    updateMemory,
    deleteMemory,
    extractFromSession,
    summarizeSession,
    getShortTermMemory,
    getStats,
    applyDecay,
    gcMemories,
    promoteMemory,
    getByTier,
  };
});
