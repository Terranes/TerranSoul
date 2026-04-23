import { describe, it, expect } from 'vitest';
import { classifyCognitiveKind, summariseCognitiveKinds } from './cognitive-kind';

/**
 * Mirror of `src-tauri/src/memory/cognitive_kind.rs::tests`. Each Rust test
 * has a matching TS test here so the two implementations stay in sync.
 *
 * If you change one classifier, change the other — and update both test
 * files. See `docs/brain-advanced-design.md` § 3.5.
 */
describe('classifyCognitiveKind', () => {
  it('explicit episodic tag wins', () => {
    expect(classifyCognitiveKind('fact', 'episodic, work', 'Mars has two moons')).toBe('episodic');
  });

  it('explicit semantic tag wins over episodic content', () => {
    expect(
      classifyCognitiveKind('fact', 'semantic', 'Yesterday I learned Mars has two moons'),
    ).toBe('semantic');
  });

  it('explicit procedural tag wins', () => {
    expect(classifyCognitiveKind('context', 'procedural:release', 'bump tag push')).toBe('procedural');
  });

  it('tag prefix with detail is recognised', () => {
    expect(classifyCognitiveKind('fact', 'episodic:meeting', 'team sync notes')).toBe('episodic');
  });

  it('summary type classifies as episodic by default', () => {
    expect(classifyCognitiveKind('summary', '', 'Discussed the rust refactor.')).toBe('episodic');
  });

  it('preference type classifies as semantic', () => {
    expect(classifyCognitiveKind('preference', '', 'User prefers dark mode')).toBe('semantic');
  });

  it('fact with episodic time anchor is episodic', () => {
    expect(
      classifyCognitiveKind('fact', '', 'Yesterday Alex finished the rust refactor'),
    ).toBe('episodic');
  });

  it('fact with general knowledge is semantic', () => {
    expect(classifyCognitiveKind('fact', '', 'Rust uses ownership for memory safety')).toBe('semantic');
  });

  it('how-to content is procedural', () => {
    expect(classifyCognitiveKind('fact', '', 'How to ship: bump version, tag, push')).toBe('procedural');
  });

  it('numbered list is procedural', () => {
    expect(
      classifyCognitiveKind('context', '', 'Release process: 1. bump version 2. tag commit 3. push tag'),
    ).toBe('procedural');
  });

  it('single numbered item is not procedural', () => {
    expect(classifyCognitiveKind('fact', '', 'Item 1. is not a list')).toBe('semantic');
  });

  it('context with no hints defaults to semantic', () => {
    expect(classifyCognitiveKind('context', '', 'Working on the marketplace feature')).toBe('semantic');
  });

  it('classify is pure — no panics on empty input', () => {
    expect(classifyCognitiveKind('fact', '', '')).toBe('semantic');
  });

  it('unrecognised tags fall through to heuristic', () => {
    expect(classifyCognitiveKind('fact', 'work, important', 'Mars has two moons')).toBe('semantic');
  });

  it('comma-separated tag list works', () => {
    expect(classifyCognitiveKind('fact', 'work,episodic,important', 'x')).toBe('episodic');
  });

  it('case-insensitive tag match', () => {
    expect(classifyCognitiveKind('fact', 'Episodic', 'x')).toBe('episodic');
  });
});

describe('summariseCognitiveKinds', () => {
  it('returns zero counts for empty input', () => {
    expect(summariseCognitiveKinds([])).toEqual({
      episodic: 0, semantic: 0, procedural: 0, total: 0,
    });
  });

  it('counts each kind correctly', () => {
    const memories = [
      { memory_type: 'summary' as const, tags: '', content: 'session recap' },
      { memory_type: 'preference' as const, tags: '', content: 'dark mode' },
      { memory_type: 'fact' as const, tags: '', content: 'How to deploy: 1. build 2. ship' },
      { memory_type: 'fact' as const, tags: '', content: 'Yesterday we shipped the release' },
      { memory_type: 'fact' as const, tags: '', content: 'Mars has two moons' },
    ];
    const result = summariseCognitiveKinds(memories);
    expect(result.episodic).toBe(2); // summary + "Yesterday"
    expect(result.semantic).toBe(2); // preference + Mars
    expect(result.procedural).toBe(1); // numbered "How to deploy"
    expect(result.total).toBe(5);
  });
});
