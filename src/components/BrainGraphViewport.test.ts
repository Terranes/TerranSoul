/**
 * Tests for BrainGraphViewport (chunk 33B.5).
 *
 * We test the pure utility functions used by the component:
 * - classifyMemoryKind (via the shared TS/Rust cognitive-kind mirror)
 * - relTypeColour (deterministic hashing)
 * - truncate
 *
 * These are exercised as standalone pure functions mirroring the component code.
 */
import { describe, it, expect } from 'vitest';
import type { MemoryEntry } from '../types';
import { classifyCognitiveKind } from '../utils/cognitive-kind';

// ── Mirror of component logic ────────────────────────────────────────────────

function classifyMemoryKind(m: MemoryEntry): string {
  return classifyCognitiveKind(m.memory_type, m.tags, m.content);
}

function relTypeColour(rel: string): string {
  const palette = [
    'var(--ts-text-secondary)',
    'var(--ts-warning)',
    'var(--ts-info)',
    'var(--ts-error)',
    'var(--ts-success-dim)',
    'var(--ts-accent)',
    'var(--ts-success)',
    'var(--ts-accent-violet)',
  ];
  let h = 0;
  for (let i = 0; i < rel.length; i++) h = ((h << 5) - h + rel.charCodeAt(i)) | 0;
  return palette[Math.abs(h) % palette.length];
}

function truncate(text: string, max: number): string {
  if (!text) return '';
  return text.length <= max ? text : text.slice(0, max - 1) + '…';
}

// ── Test helpers ─────────────────────────────────────────────────────────────

function makeEntry(
  overrides: Partial<MemoryEntry> & { content: string },
): MemoryEntry {
  return {
    id: 1,
    tags: '',
    importance: 3,
    memory_type: 'fact',
    tier: 'long',
    decay_score: 1.0,
    access_count: 1,
    created_at: Date.now(),
    last_accessed: null,
    token_count: 5,
    session_id: null,
    parent_id: null,
    ...overrides,
  };
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('classifyCognitiveKind', () => {
  it('returns episodic when tags include episodic prefix', () => {
    const m = makeEntry({ content: 'A meeting note', tags: 'episodic:meeting,work' });
    expect(classifyMemoryKind(m)).toBe('episodic');
  });

  it('returns procedural when tags include procedural prefix', () => {
    const m = makeEntry({ content: 'Anything', tags: 'procedural:deploy' });
    expect(classifyMemoryKind(m)).toBe('procedural');
  });

  it('returns judgment when tags include judgment', () => {
    const m = makeEntry({ content: 'Always use TS strict', tags: 'judgment' });
    expect(classifyMemoryKind(m)).toBe('judgment');
  });

  it('returns episodic for summary type', () => {
    const m = makeEntry({ content: 'Session recap', memory_type: 'summary' });
    expect(classifyMemoryKind(m)).toBe('episodic');
  });

  it('returns semantic for preference type', () => {
    const m = makeEntry({ content: 'Dark mode', memory_type: 'preference' });
    expect(classifyMemoryKind(m)).toBe('semantic');
  });

  it('returns procedural for content with how-to verb', () => {
    const m = makeEntry({ content: 'How to deploy the app in production' });
    expect(classifyMemoryKind(m)).toBe('procedural');
  });

  it('returns episodic for content with time anchor', () => {
    const m = makeEntry({ content: 'Yesterday we discussed the API refactor' });
    expect(classifyMemoryKind(m)).toBe('episodic');
  });

  it('defaults to semantic when no signals match', () => {
    const m = makeEntry({ content: 'Rust uses ownership for memory safety' });
    expect(classifyMemoryKind(m)).toBe('semantic');
  });

  it('tag override wins over content heuristic', () => {
    // Content says "yesterday" (episodic) but tag says procedural
    const m = makeEntry({
      content: 'Yesterday I learned how to deploy',
      tags: 'procedural:deploy',
    });
    expect(classifyMemoryKind(m)).toBe('procedural');
  });
});

describe('relTypeColour', () => {
  it('returns a design-token colour for any input', () => {
    expect(relTypeColour('derived_from')).toMatch(/^var\(--ts-[^)]+\)$/);
    expect(relTypeColour('contradicts')).toMatch(/^var\(--ts-[^)]+\)$/);
  });

  it('is deterministic', () => {
    const a = relTypeColour('related_to');
    const b = relTypeColour('related_to');
    expect(a).toBe(b);
  });

  it('maps common relation types across more than one palette colour', () => {
    const colours = new Set([
      relTypeColour('derived_from'),
      relTypeColour('contradicts'),
      relTypeColour('supports'),
      relTypeColour('related_to'),
      relTypeColour('supersedes'),
    ]);
    expect(colours.size).toBeGreaterThan(1);
  });
});

describe('truncate (BrainGraphViewport)', () => {
  it('returns empty string for falsy input', () => {
    expect(truncate('', 10)).toBe('');
  });

  it('returns unchanged when within limit', () => {
    expect(truncate('short', 10)).toBe('short');
  });

  it('truncates with ellipsis', () => {
    expect(truncate('hello world foo', 10)).toBe('hello wor…');
  });
});
