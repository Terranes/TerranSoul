/**
 * Tests for the BRAIN-REPO-RAG-1c-b-ii-b frontend cross-source chat wiring:
 *
 *   1. `parseSourceMentions` — `@source-id` extraction from chat input.
 *   2. `groupHitsBySource` — preserve first-appearance order per source.
 *   3. `formatCrossSourceContextPack` — single- vs multi-source rendering.
 *   4. Citation footer grouping (mirrors the Vue helper logic).
 *
 * The conversation store's full sendMessage flow is covered separately in
 * `conversation.test.ts`; here we exercise the pure helpers so failures
 * surface with crisp messages.
 */
import { describe, it, expect } from 'vitest';
import {
  parseSourceMentions,
  groupHitsBySource,
  formatCrossSourceContextPack,
} from './conversation';
import type { MultiSourceHit } from '../types';

function hit(partial: Partial<MultiSourceHit>): MultiSourceHit {
  return {
    source_id: 'self',
    source_label: 'TerranSoul',
    local_id: 1,
    content: 'sample',
    score: 0.5,
    file_path: null,
    parent_symbol: null,
    tier: 'long',
    tags: '',
    ...partial,
  };
}

describe('parseSourceMentions', () => {
  it('returns empty mentions when none are present', () => {
    const r = parseSourceMentions('How are you today?');
    expect(r.mentioned).toEqual([]);
    expect(r.cleaned).toBe('How are you today?');
  });

  it('extracts a single `@source-id` mention and strips it from the query', () => {
    const r = parseSourceMentions('@owner/repo what does main do?');
    expect(r.mentioned).toEqual(['owner/repo']);
    expect(r.cleaned).toBe('what does main do?');
  });

  it('extracts multiple distinct mentions in order, dedup keeps first occurrence', () => {
    const r = parseSourceMentions('Compare @owner/a with @owner/b and @owner/a again.');
    expect(r.mentioned).toEqual(['owner/a', 'owner/b']);
    expect(r.cleaned).toBe('Compare with and again.');
  });

  it('strips trailing punctuation from the captured id', () => {
    // The trailing `,` after `repo` is NOT part of the captured id
    // (regex char-class excludes commas), so the comma survives the
    // strip — that's the documented behaviour, and it gives the LLM a
    // grammatical sentence to work from.
    const r = parseSourceMentions('Tell me @owner/repo, especially the loop.');
    expect(r.mentioned).toEqual(['owner/repo']);
    expect(r.cleaned).toContain('especially the loop.');
    expect(r.cleaned).not.toContain('@owner/repo');
  });

  it('ignores `@` inside an email address (no word boundary)', () => {
    const r = parseSourceMentions('Ping me at user@example.com about it');
    expect(r.mentioned).toEqual([]);
    expect(r.cleaned).toBe('Ping me at user@example.com about it');
  });

  it('treats start-of-string as a valid mention boundary', () => {
    const r = parseSourceMentions('@self what facts do you have?');
    expect(r.mentioned).toEqual(['self']);
  });

  it('preserves the original message when stripping would leave only whitespace', () => {
    const r = parseSourceMentions('@owner/repo');
    expect(r.mentioned).toEqual(['owner/repo']);
    expect(r.cleaned).toBe('@owner/repo');
  });

  it('handles empty / non-string input defensively', () => {
    expect(parseSourceMentions('').mentioned).toEqual([]);
    // @ts-expect-error — null is not a string but the helper must not throw.
    expect(parseSourceMentions(null).mentioned).toEqual([]);
  });
});

describe('groupHitsBySource', () => {
  it('returns an empty array for empty input', () => {
    expect(groupHitsBySource([])).toEqual([]);
  });

  it('groups by source_id preserving first-appearance order', () => {
    const hits: MultiSourceHit[] = [
      hit({ source_id: 'self', source_label: 'TerranSoul', local_id: 1, content: 'a' }),
      hit({ source_id: 'owner/repo', source_label: 'owner/repo', local_id: 10, content: 'b' }),
      hit({ source_id: 'self', source_label: 'TerranSoul', local_id: 2, content: 'c' }),
      hit({ source_id: 'owner/repo', source_label: 'owner/repo', local_id: 11, content: 'd' }),
    ];
    const groups = groupHitsBySource(hits);
    expect(groups.map((g) => g.source_id)).toEqual(['self', 'owner/repo']);
    expect(groups[0].hits.map((h) => h.content)).toEqual(['a', 'c']);
    expect(groups[1].hits.map((h) => h.content)).toEqual(['b', 'd']);
  });
});

describe('formatCrossSourceContextPack', () => {
  it('returns empty string when there are no hits', () => {
    expect(formatCrossSourceContextPack([])).toBe('');
  });

  it('renders a single-source result with the legacy bullet format', () => {
    const hits = [
      hit({ content: 'user prefers dark mode' }),
      hit({ local_id: 2, content: 'user is on Windows' }),
    ];
    const out = formatCrossSourceContextPack(hits);
    expect(out).toContain('[LONG-TERM MEMORY]');
    expect(out).toContain('- user prefers dark mode');
    expect(out).toContain('- user is on Windows');
    // Single source ⇒ no per-source headers.
    expect(out).not.toContain('── 🧠');
    expect(out).not.toContain('── 📦');
  });

  it('renders grouped multi-source headers with appropriate badges', () => {
    const hits: MultiSourceHit[] = [
      hit({ source_id: 'self', source_label: 'TerranSoul', content: 'brain fact A' }),
      hit({
        source_id: 'owner/repo',
        source_label: 'owner/repo',
        local_id: 7,
        content: 'starts the runtime',
        file_path: 'src/lib.rs',
        parent_symbol: 'main',
        tier: null,
      }),
    ];
    const out = formatCrossSourceContextPack(hits);
    expect(out).toContain('── 🧠 TerranSoul ──');
    expect(out).toContain('── 📦 owner/repo ──');
    expect(out).toContain('- brain fact A');
    // Repo hit must render [path::symbol] preface.
    expect(out).toContain('- [src/lib.rs::main] starts the runtime');
    // Cross-source contract differs from the single-source pack.
    expect(out).toContain('cross-source memory/RAG store');
    expect(out).toContain('Cite the source when you use a record');
  });

  it('omits the ::symbol preface when parent_symbol is missing', () => {
    const hits = [
      hit({
        source_id: 'self',
        source_label: 'TerranSoul',
      }),
      hit({
        source_id: 'owner/repo',
        source_label: 'owner/repo',
        local_id: 9,
        content: 'README intro',
        file_path: 'README.md',
        parent_symbol: null,
        tier: null,
      }),
    ];
    const out = formatCrossSourceContextPack(hits);
    expect(out).toContain('- [README.md] README intro');
    expect(out).not.toContain('::null');
  });
});
