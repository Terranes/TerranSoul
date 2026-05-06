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

// ── Audit tab (chunk 33B.4) ──────────────────────────────────────────────
//
// Pure-function tests mirroring `auditCandidates` and `truncate` in
// MemoryView.vue. These verify that selecting a memory for the
// provenance view returns the expected filtered + sorted candidate list.

function auditCandidates(memories: MemoryEntry[], search: string): MemoryEntry[] {
  const q = search.trim().toLowerCase();
  const list =
    q.length === 0
      ? memories
      : memories.filter(
          (m) =>
            m.content.toLowerCase().includes(q) ||
            (m.tags ?? '').toLowerCase().includes(q),
        );
  return [...list].sort((a, b) => b.created_at - a.created_at).slice(0, 200);
}

function truncate(text: string, max: number): string {
  if (!text) return '';
  return text.length <= max ? text : text.slice(0, max - 1) + '…';
}

function makeAuditEntry(
  id: number,
  content: string,
  tags: string,
  createdAt: number,
): MemoryEntry {
  return {
    id,
    content,
    tags,
    memory_type: 'fact',
    tier: 'long',
    importance: 3,
    decay_score: 1.0,
    access_count: 1,
    created_at: createdAt,
    last_accessed: createdAt,
    token_count: 5,
    session_id: null,
    parent_id: null,
  };
}

describe('auditCandidates (chunk 33B.4)', () => {
  const memories: MemoryEntry[] = [
    makeAuditEntry(1, 'User likes coffee in the morning', 'personal:beverage', 1000),
    makeAuditEntry(2, 'Project TerranSoul uses Rust + Vue', 'code:rust,project:ts', 3000),
    makeAuditEntry(3, 'Cook County family law deadline', 'domain:law', 2000),
    makeAuditEntry(4, 'Plain note', '', 4000),
  ];

  it('returns all memories sorted newest-first when search is empty', () => {
    const result = auditCandidates(memories, '');
    expect(result.map((m) => m.id)).toEqual([4, 2, 3, 1]);
  });

  it('filters by content substring (case-insensitive)', () => {
    const result = auditCandidates(memories, 'rust');
    expect(result.map((m) => m.id)).toEqual([2]);
  });

  it('filters by tag substring', () => {
    const result = auditCandidates(memories, 'domain:law');
    expect(result.map((m) => m.id)).toEqual([3]);
  });

  it('returns empty when no match', () => {
    const result = auditCandidates(memories, 'nonexistent-token-xyz');
    expect(result).toHaveLength(0);
  });

  it('caps result list at 200 entries', () => {
    const big: MemoryEntry[] = Array.from({ length: 250 }, (_, i) =>
      makeAuditEntry(i + 1, `entry ${i}`, '', i),
    );
    const result = auditCandidates(big, '');
    expect(result).toHaveLength(200);
  });

  it('does not mutate the input array', () => {
    const before = memories.map((m) => m.id);
    auditCandidates(memories, '');
    expect(memories.map((m) => m.id)).toEqual(before);
  });
});

describe('truncate', () => {
  it('returns empty string for falsy input', () => {
    expect(truncate('', 10)).toBe('');
  });

  it('returns input unchanged when within limit', () => {
    expect(truncate('hi', 10)).toBe('hi');
  });

  it('truncates with ellipsis when over limit', () => {
    expect(truncate('hello world', 6)).toBe('hello…');
  });

  it('handles boundary exactly at max', () => {
    expect(truncate('abcde', 5)).toBe('abcde');
  });
});
