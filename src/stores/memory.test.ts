import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useMemoryStore } from './memory';
import type { MemoryEntry } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const entry = (id: number, content: string): MemoryEntry => ({
  id,
  content,
  tags: 'test',
  importance: 3,
  memory_type: 'fact',
  created_at: Date.now(),
  last_accessed: null,
  access_count: 0,
  tier: 'long',
  decay_score: 1.0,
  session_id: null,
  parent_id: null,
  token_count: 0,
});

describe('memory store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    localStorage.removeItem('ts.browser.rag.memories.v1');
  });

  it('starts with empty memories', () => {
    const store = useMemoryStore();
    expect(store.memories).toHaveLength(0);
  });

  it('fetchAll populates memories', async () => {
    mockInvoke.mockResolvedValue([entry(1, 'User likes Python'), entry(2, 'User works on ML')]);
    const store = useMemoryStore();
    await store.fetchAll();
    expect(store.memories).toHaveLength(2);
    expect(store.memories[0].content).toBe('User likes Python');
  });

  it('addMemory prepends entry to list', async () => {
    const newEntry = entry(3, 'New fact');
    mockInvoke.mockResolvedValue(newEntry);
    const store = useMemoryStore();
    store.memories = [entry(1, 'Existing')];
    const result = await store.addMemory({
      content: 'New fact',
      tags: 'test',
      importance: 3,
      memory_type: 'fact',
    });
    expect(result).toEqual(newEntry);
    expect(store.memories[0].id).toBe(3);
    expect(store.memories).toHaveLength(2);
  });

  it('updateMemory replaces entry in list', async () => {
    const updated = { ...entry(1, 'Updated content'), importance: 5 };
    mockInvoke.mockResolvedValue(updated);
    const store = useMemoryStore();
    store.memories = [entry(1, 'Original')];
    await store.updateMemory(1, { content: 'Updated content', importance: 5 });
    expect(store.memories[0].content).toBe('Updated content');
    expect(store.memories[0].importance).toBe(5);
  });

  it('deleteMemory removes entry from list', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useMemoryStore();
    store.memories = [entry(1, 'To delete'), entry(2, 'Keep')];
    await store.deleteMemory(1);
    expect(store.memories).toHaveLength(1);
    expect(store.memories[0].id).toBe(2);
  });

  it('search returns invoke result', async () => {
    mockInvoke.mockResolvedValue([entry(1, 'Python result')]);
    const store = useMemoryStore();
    const results = await store.search('Python');
    expect(results).toHaveLength(1);
  });

  it('semanticSearch returns invoke result', async () => {
    mockInvoke.mockResolvedValue([entry(2, 'ML project')]);
    const store = useMemoryStore();
    const results = await store.semanticSearch('machine learning', 5);
    expect(results).toHaveLength(1);
    expect(mockInvoke).toHaveBeenCalledWith('semantic_search_memories', {
      query: 'machine learning',
      limit: 5,
    });
  });

  it('extractFromSession calls backend and refreshes list', async () => {
    mockInvoke
      .mockResolvedValueOnce(3) // extract_memories_from_session
      .mockResolvedValueOnce([entry(1, 'New fact')]); // get_memories
    const store = useMemoryStore();
    const count = await store.extractFromSession();
    expect(count).toBe(3);
    expect(store.memories).toHaveLength(1);
  });

  it('summarizeSession returns summary string and refreshes', async () => {
    mockInvoke
      .mockResolvedValueOnce('Session was about Python and ML.') // summarize_session
      .mockResolvedValueOnce([]); // get_memories
    const store = useMemoryStore();
    const summary = await store.summarizeSession();
    expect(summary).toContain('Python');
  });

  it('search returns empty array on error', async () => {
    mockInvoke.mockRejectedValue(new Error('fail'));
    const store = useMemoryStore();
    const results = await store.search('anything');
    expect(results).toEqual([]);
  });

  it('falls back to browser memory storage when Tauri addMemory fails', async () => {
    mockInvoke.mockRejectedValue(new Error('no tauri'));
    const store = useMemoryStore();
    const result = await store.addMemory({
      content: 'User likes browser-native RAG with vectors',
      tags: 'browser-rag',
      importance: 4,
      memory_type: 'fact',
    });

    expect(result?.content).toContain('browser-native RAG');
    expect(store.memories).toHaveLength(1);
    expect(JSON.parse(localStorage.getItem('ts.browser.rag.memories.v1') ?? '[]')).toHaveLength(1);
  });

  it('browser hybrid search retrieves persisted memory by keyword and vector signals', async () => {
    mockInvoke.mockRejectedValue(new Error('no tauri'));
    const store = useMemoryStore();
    await store.addMemory({
      content: 'The user wants Google Drive sync for browser memories',
      tags: 'browser-rag,drive',
      importance: 5,
      memory_type: 'fact',
    });

    const results = await store.hybridSearch('browser memory drive sync', 3);
    expect(results[0]?.content).toContain('Google Drive sync');
  });

  it('exports and imports browser sync payloads for drive-style backup', async () => {
    mockInvoke.mockRejectedValue(new Error('no tauri'));
    const store = useMemoryStore();
    await store.addMemory({
      content: 'Export this browser memory',
      tags: 'browser-rag',
      importance: 3,
      memory_type: 'summary',
    });

    const payload = await store.exportBrowserSyncPayload();
    localStorage.removeItem('ts.browser.rag.memories.v1');
    const count = await store.importBrowserSyncPayload(payload);

    expect(count).toBe(1);
    expect(store.memories[0].content).toContain('Export this browser memory');
  });

  // ── Entity-Relationship Graph (V5) ─────────────────────────────────────────

  it('fetchEdges populates edges array', async () => {
    const edge = {
      id: 1,
      src_id: 1,
      dst_id: 2,
      rel_type: 'cites',
      confidence: 0.9,
      source: 'user' as const,
      created_at: Date.now(),
    };
    mockInvoke.mockResolvedValue([edge]);
    const store = useMemoryStore();
    const result = await store.fetchEdges();
    expect(result).toHaveLength(1);
    expect(store.edges[0].rel_type).toBe('cites');
    expect(mockInvoke).toHaveBeenCalledWith('list_memory_edges');
  });

  it('addEdge appends new edge and replaces existing on duplicate', async () => {
    const edge = {
      id: 7,
      src_id: 1,
      dst_id: 2,
      rel_type: 'related_to',
      confidence: 1.0,
      source: 'user' as const,
      created_at: Date.now(),
    };
    mockInvoke.mockResolvedValue(edge);
    const store = useMemoryStore();
    const created = await store.addEdge(1, 2, 'related_to');
    expect(created?.id).toBe(7);
    expect(store.edges).toHaveLength(1);
    // Re-adding the same (src, dst, rel_type) triggers in-place replace.
    mockInvoke.mockResolvedValue({ ...edge, confidence: 0.5 });
    await store.addEdge(1, 2, 'related_to');
    expect(store.edges).toHaveLength(1);
    expect(store.edges[0].confidence).toBe(0.5);
  });

  it('deleteEdge removes edge from cache', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useMemoryStore();
    store.edges = [
      { id: 1, src_id: 1, dst_id: 2, rel_type: 'r', confidence: 1, source: 'user', created_at: 0 },
      { id: 2, src_id: 2, dst_id: 3, rel_type: 'r', confidence: 1, source: 'user', created_at: 0 },
    ];
    await store.deleteEdge(1);
    expect(store.edges).toHaveLength(1);
    expect(store.edges[0].id).toBe(2);
  });

  it('extractEdgesViaBrain calls backend and refreshes when count > 0', async () => {
    mockInvoke
      .mockResolvedValueOnce(4) // extract_edges_via_brain
      .mockResolvedValueOnce([]); // list_memory_edges (refresh)
    const store = useMemoryStore();
    const count = await store.extractEdgesViaBrain();
    expect(count).toBe(4);
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'extract_edges_via_brain', { chunkSize: 25 });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'list_memory_edges');
  });

  it('multiHopSearch forwards hops parameter', async () => {
    mockInvoke.mockResolvedValue([entry(1, 'graph hit')]);
    const store = useMemoryStore();
    const results = await store.multiHopSearch('query', 5, 2);
    expect(results).toHaveLength(1);
    expect(mockInvoke).toHaveBeenCalledWith('multi_hop_search_memories', {
      query: 'query',
      limit: 5,
      hops: 2,
    });
  });

  it('getEdgeStats caches stats', async () => {
    const stats = { total_edges: 3, by_rel_type: [['cites', 2]], by_source: [['user', 3]], connected_memories: 4 };
    mockInvoke.mockResolvedValue(stats);
    const store = useMemoryStore();
    const result = await store.getEdgeStats();
    expect(result?.total_edges).toBe(3);
    expect(store.edgeStats?.connected_memories).toBe(4);
  });
});
