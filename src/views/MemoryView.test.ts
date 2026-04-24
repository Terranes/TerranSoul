/**
 * Tests for the tag-prefix filter logic used in MemoryView (Chunk 18.3).
 *
 * These are pure-function tests exercising the same filtering logic
 * that MemoryView.vue uses in its `displayedMemories` computed.
 */
import { describe, it, expect } from 'vitest';
import type { MemoryEntry } from '../types';

const TAG_PREFIXES = ['personal', 'domain', 'project', 'tool', 'code', 'external', 'session', 'quest'] as const;

/** Extract tag-prefix counts from memories (mirrors MemoryView computed). */
function tagPrefixCounts(memories: MemoryEntry[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const m of memories) {
    if (!m.tags) continue;
    const seen = new Set<string>();
    for (const tag of m.tags.split(',')) {
      const trimmed = tag.trim();
      const colonIdx = trimmed.indexOf(':');
      if (colonIdx <= 0) continue;
      const prefix = trimmed.slice(0, colonIdx).toLowerCase();
      if (!seen.has(prefix) && (TAG_PREFIXES as readonly string[]).includes(prefix)) {
        seen.add(prefix);
        counts.set(prefix, (counts.get(prefix) ?? 0) + 1);
      }
    }
  }
  return counts;
}

/** Filter memories by tag prefix (mirrors MemoryView computed). */
function filterByTagPrefix(memories: MemoryEntry[], prefix: string | null): MemoryEntry[] {
  if (!prefix) return memories;
  return memories.filter((m) => {
    if (!m.tags) return false;
    return m.tags.split(',').some((t) => {
      const trimmed = t.trim().toLowerCase();
      return trimmed.startsWith(prefix + ':');
    });
  });
}

function makeEntry(id: number, tags: string): MemoryEntry {
  return {
    id,
    content: `Memory ${id}`,
    tags,
    memory_type: 'fact',
    tier: 'long',
    importance: 3,
    decay_score: 1.0,
    access_count: 1,
    created_at: Date.now(),
    last_accessed: Date.now(),
    token_count: 5,
    session_id: null,
    parent_id: null,
  };
}

describe('tagPrefixCounts', () => {
  it('counts each curated prefix once per memory', () => {
    const memories = [
      makeEntry(1, 'personal:name,personal:goal,domain:law'),
      makeEntry(2, 'domain:law,code:rust'),
      makeEntry(3, 'tool:cli'),
    ];
    const counts = tagPrefixCounts(memories);
    expect(counts.get('personal')).toBe(1);
    expect(counts.get('domain')).toBe(2);
    expect(counts.get('code')).toBe(1);
    expect(counts.get('tool')).toBe(1);
  });

  it('ignores non-curated prefixes', () => {
    const memories = [makeEntry(1, 'custom:foo,personal:bar')];
    const counts = tagPrefixCounts(memories);
    expect(counts.has('custom')).toBe(false);
    expect(counts.get('personal')).toBe(1);
  });

  it('ignores legacy flat tags', () => {
    const memories = [makeEntry(1, 'fact,preference,user')];
    const counts = tagPrefixCounts(memories);
    expect(counts.size).toBe(0);
  });

  it('handles empty tags', () => {
    const memories = [makeEntry(1, '')];
    const counts = tagPrefixCounts(memories);
    expect(counts.size).toBe(0);
  });

  it('is case-insensitive on prefix', () => {
    const memories = [makeEntry(1, 'Personal:Name,DOMAIN:law')];
    const counts = tagPrefixCounts(memories);
    expect(counts.get('personal')).toBe(1);
    expect(counts.get('domain')).toBe(1);
  });
});

describe('filterByTagPrefix', () => {
  const memories = [
    makeEntry(1, 'personal:name,domain:law'),
    makeEntry(2, 'code:rust,tool:cli'),
    makeEntry(3, 'personal:goal'),
    makeEntry(4, 'fact'),
    makeEntry(5, ''),
  ];

  it('returns all when prefix is null', () => {
    expect(filterByTagPrefix(memories, null)).toHaveLength(5);
  });

  it('filters to personal prefix', () => {
    const result = filterByTagPrefix(memories, 'personal');
    expect(result.map((m) => m.id)).toEqual([1, 3]);
  });

  it('filters to code prefix', () => {
    const result = filterByTagPrefix(memories, 'code');
    expect(result.map((m) => m.id)).toEqual([2]);
  });

  it('excludes memories with no tags', () => {
    const result = filterByTagPrefix(memories, 'personal');
    expect(result.every((m) => m.tags.length > 0)).toBe(true);
  });

  it('excludes memories with only flat tags', () => {
    const result = filterByTagPrefix(memories, 'personal');
    expect(result.map((m) => m.id)).not.toContain(4);
  });
});
