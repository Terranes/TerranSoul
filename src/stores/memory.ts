import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

/** A detected contradiction between two memories (Chunk 17.2). */
export interface MemoryConflict {
  id: number;
  entry_a_id: number;
  entry_b_id: number;
  status: 'open' | 'resolved' | 'dismissed';
  winner_id: number | null;
  created_at: number;
  resolved_at: number | null;
  reason: string;
}

export interface MemoryCleanupReport {
  before_bytes: number;
  after_bytes: number;
  max_bytes: number;
  deleted: number;
}
import type {
  EdgeDirection,
  EdgeStats,
  MemoryEdge,
  MemoryProvenance,
  MemoryEntry,
  MemoryStats,
  MemoryTier,
  Message,
  NewMemory,
} from '../types';
import {
  addBrowserMemory,
  browserHybridSearch,
  browserKeywordSearch,
  browserMemoryStats,
  clearBrowserMemories,
  exportBrowserRagSyncPayload,
  importBrowserRagSyncPayload,
  listBrowserMemories,
  type BrowserRagSyncPayload,
} from '../transport/browser-rag';

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
      const result = await invoke<MemoryEntry[]>('get_memories');
      memories.value = Array.isArray(result) ? result : await listBrowserMemories();
    } catch (e) {
      error.value = String(e);
      memories.value = await listBrowserMemories();
    } finally {
      isLoading.value = false;
    }
  }

  async function search(query: string): Promise<MemoryEntry[]> {
    try {
      const result = await invoke<MemoryEntry[]>('search_memories', { query });
      return Array.isArray(result) ? result : browserKeywordSearch(query);
    } catch {
      return browserKeywordSearch(query);
    }
  }

  /** Brain-powered semantic search — uses the active Ollama model. */
  async function semanticSearch(query: string, limit = 10): Promise<MemoryEntry[]> {
    try {
      const result = await invoke<MemoryEntry[]>('semantic_search_memories', { query, limit });
      return Array.isArray(result) ? result : browserHybridSearch(query, limit);
    } catch {
      return browserHybridSearch(query, limit);
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
      if (!entry || typeof entry.id !== 'number') throw new Error('Invalid memory entry');
      memories.value.unshift(entry);
      return entry;
    } catch (e) {
      error.value = String(e);
      const entry = await addBrowserMemory(m);
      memories.value.unshift(entry);
      return entry;
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

  /** Delete **all** persisted data: memories, brain, voice, persona, quests, settings. */
  async function clearAllData(): Promise<void> {
    try {
      await invoke('clear_all_data');
      memories.value = [];
      stats.value = null;
      edges.value = [];
      edgeStats.value = null;
    } catch (e) {
      error.value = String(e);
      await clearBrowserMemories();
      memories.value = [];
      stats.value = null;
      edges.value = [];
      edgeStats.value = null;
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
      const result = await invoke<MemoryEntry[]>('hybrid_search_memories', { query, limit });
      return Array.isArray(result) ? result : browserHybridSearch(query, limit);
    } catch {
      return browserHybridSearch(query, limit);
    }
  }

  /** Get memory statistics per tier. */
  async function getStats(): Promise<MemoryStats | null> {
    try {
      const s = await invoke<MemoryStats>('get_memory_stats');
      stats.value = s;
      return s;
    } catch {
      const s = await browserMemoryStats();
      stats.value = s;
      return s;
    }
  }

  async function exportBrowserSyncPayload(): Promise<BrowserRagSyncPayload> {
    return exportBrowserRagSyncPayload();
  }

  async function importBrowserSyncPayload(payload: BrowserRagSyncPayload): Promise<number> {
    const count = await importBrowserRagSyncPayload(payload);
    memories.value = await listBrowserMemories();
    return count;
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

  async function enforceStorageLimit(): Promise<MemoryCleanupReport | null> {
    try {
      const report = await invoke<MemoryCleanupReport>('enforce_memory_storage_limit');
      if (report.deleted > 0) await fetchAll();
      await getStats();
      return report;
    } catch (e) {
      error.value = String(e);
      return null;
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

  /** Get the joined provenance tree for a memory (chunk 33B.4). */
  async function getMemoryProvenance(memoryId: number): Promise<MemoryProvenance | null> {
    try {
      return await invoke<MemoryProvenance>('get_memory_provenance', { memoryId });
    } catch (e) {
      error.value = String(e);
      return null;
    }
  }

  /** Auto-adjust importance based on access patterns (chunk 17.4). */
  async function adjustImportance(
    hotThreshold?: number,
    coldDays?: number,
  ): Promise<{ boosted: number; demoted: number }> {
    return await invoke('adjust_memory_importance', { hotThreshold, coldDays });
  }

  // ── Contradiction resolution (Chunk 17.2) ──────────────────────────────

  /** List memory conflicts, optionally filtered by status. */
  async function listConflicts(
    status?: 'open' | 'resolved' | 'dismissed',
  ): Promise<MemoryConflict[]> {
    return await invoke('list_memory_conflicts', { status });
  }

  /** Resolve a conflict by picking a winner. Loser is soft-closed. */
  async function resolveConflict(
    conflictId: number,
    winnerId: number,
  ): Promise<MemoryConflict> {
    return await invoke('resolve_memory_conflict', { conflictId, winnerId });
  }

  /** Dismiss a conflict (user says "not a real conflict"). */
  async function dismissConflict(conflictId: number): Promise<void> {
    return await invoke('dismiss_memory_conflict', { conflictId });
  }

  /** Count open (unresolved) conflicts. */
  async function countConflicts(): Promise<number> {
    return await invoke('count_memory_conflicts');
  }

  // ─── Judgment Rules (Chunk 33B.1) ─────────────────────────────────────

  /** Add a new judgment rule. */
  async function addJudgment(content: string, tags: string, importance: number): Promise<MemoryEntry> {
    return await invoke('judgment_add', { content, tags, importance });
  }

  /** List all persisted judgment rules. */
  async function listJudgments(): Promise<MemoryEntry[]> {
    return await invoke('judgment_list');
  }

  /** Search judgment rules relevant to a query. */
  async function applyJudgments(query: string, limit?: number): Promise<MemoryEntry[]> {
    return await invoke('judgment_apply', { query, limit });
  }

  /** Backfill embeddings for memories that don't have them yet. */
  async function backfillEmbeddings(): Promise<number> {
    return await invoke<number>('backfill_embeddings');
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
    clearAllData,
    extractFromSession,
    summarizeSession,
    getShortTermMemory,
    getStats,
    applyDecay,
    gcMemories,
    enforceStorageLimit,
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
    getMemoryProvenance,
    adjustImportance,
    exportBrowserSyncPayload,
    importBrowserSyncPayload,
    // Contradiction resolution (Chunk 17.2)
    listConflicts,
    resolveConflict,
    dismissConflict,
    countConflicts,
    // Judgment rules (Chunk 33B.1)
    addJudgment,
    listJudgments,
    applyJudgments,
    // Embedding management (Chunk 44.2)
    backfillEmbeddings,
  };
});
