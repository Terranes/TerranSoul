import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type {
  EdgeDirection,
  EdgeStats,
  MemoryEdge,
  MemoryEntry,
  MemoryStats,
  MemoryTier,
  Message,
  NewMemory,
} from '../types';

export const useMemoryStore = defineStore('memory', () => {
  const memories = ref<MemoryEntry[]>([]);
  const stats = ref<MemoryStats | null>(null);
  const edges = ref<MemoryEdge[]>([]);
  const edgeStats = ref<EdgeStats | null>(null);
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

  // ── Entity-Relationship Graph (V5 schema) ─────────────────────────────────

  /** Load all typed edges in the knowledge graph. */
  async function fetchEdges(): Promise<MemoryEdge[]> {
    try {
      const list = await invoke<MemoryEdge[]>('list_memory_edges');
      edges.value = list;
      return list;
    } catch (e) {
      error.value = String(e);
      return [];
    }
  }

  /** Insert (or fetch existing) typed edge. */
  async function addEdge(
    srcId: number,
    dstId: number,
    relType: string,
    confidence = 1.0,
    source: 'user' | 'llm' | 'auto' = 'user',
  ): Promise<MemoryEdge | null> {
    try {
      const edge = await invoke<MemoryEdge>('add_memory_edge', {
        srcId,
        dstId,
        relType,
        confidence,
        source,
      });
      // Replace if this (src,dst,rel_type) already in cache, else append.
      const i = edges.value.findIndex(
        (e) => e.src_id === edge.src_id && e.dst_id === edge.dst_id && e.rel_type === edge.rel_type,
      );
      if (i === -1) edges.value.push(edge);
      else edges.value[i] = edge;
      return edge;
    } catch (e) {
      error.value = String(e);
      return null;
    }
  }

  async function deleteEdge(edgeId: number): Promise<boolean> {
    try {
      await invoke('delete_memory_edge', { edgeId });
      edges.value = edges.value.filter((e) => e.id !== edgeId);
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    }
  }

  async function getEdgesForMemory(
    memoryId: number,
    direction: EdgeDirection = 'both',
  ): Promise<MemoryEdge[]> {
    try {
      return await invoke<MemoryEdge[]>('get_edges_for_memory', { memoryId, direction });
    } catch {
      return [];
    }
  }

  async function getEdgeStats(): Promise<EdgeStats | null> {
    try {
      const s = await invoke<EdgeStats>('get_edge_stats');
      edgeStats.value = s;
      return s;
    } catch {
      return null;
    }
  }

  async function listRelationTypes(): Promise<string[]> {
    try {
      return await invoke<string[]>('list_relation_types');
    } catch {
      return [];
    }
  }

  /** Ask the brain to scan all memories and propose typed edges. */
  async function extractEdgesViaBrain(chunkSize = 25): Promise<number> {
    try {
      const count = await invoke<number>('extract_edges_via_brain', { chunkSize });
      if (count > 0) await fetchEdges();
      return count;
    } catch (e) {
      error.value = String(e);
      return 0;
    }
  }

  /** Multi-hop hybrid search (vector + keyword + graph traversal). */
  async function multiHopSearch(query: string, limit = 10, hops = 1): Promise<MemoryEntry[]> {
    try {
      return await invoke<MemoryEntry[]>('multi_hop_search_memories', { query, limit, hops });
    } catch {
      return [];
    }
  }

  /** Export long-tier memories to an Obsidian vault directory. */
  async function exportToObsidian(vaultDir: string): Promise<{ written: number; skipped: number; total: number; output_dir: string }> {
    return await invoke('export_to_obsidian', { vaultDir });
  }

  /** Get the version history of a memory (chunk 16.12). */
  async function getMemoryHistory(memoryId: number): Promise<Array<{
    id: number; memory_id: number; version_num: number;
    content: string; tags: string; importance: number;
    memory_type: string; created_at: number;
  }>> {
    try {
      return await invoke('get_memory_history', { memoryId });
    } catch {
      return [];
    }
  }

  /** Auto-adjust importance based on access patterns (chunk 17.4). */
  async function adjustImportance(
    hotThreshold?: number,
    coldDays?: number,
  ): Promise<{ boosted: number; demoted: number }> {
    return await invoke('adjust_memory_importance', { hotThreshold, coldDays });
  }

  return {
    memories,
    stats,
    edges,
    edgeStats,
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
    fetchEdges,
    addEdge,
    deleteEdge,
    getEdgesForMemory,
    getEdgeStats,
    listRelationTypes,
    extractEdgesViaBrain,
    multiHopSearch,
    exportToObsidian,
    getMemoryHistory,
    adjustImportance,
  };
});
