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
});

describe('memory store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
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
});
