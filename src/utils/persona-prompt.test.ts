import { describe, it, expect } from 'vitest';
import { buildPersonaBlock, type LearnedMotionRef } from './persona-prompt';
import { defaultPersona, type PersonaTraits } from '../stores/persona-types';

const baseTraits = (overrides: Partial<PersonaTraits> = {}): PersonaTraits => ({
  ...defaultPersona(),
  active: true,
  updatedAt: 1,
  ...overrides,
});

describe('buildPersonaBlock', () => {
  it('returns empty string when traits are null/undefined', () => {
    expect(buildPersonaBlock(null)).toBe('');
    expect(buildPersonaBlock(undefined)).toBe('');
  });

  it('returns empty string when persona is inactive', () => {
    expect(buildPersonaBlock(baseTraits({ active: false }))).toBe('');
  });

  it('returns empty string when all renderable fields are blank', () => {
    const empty = baseTraits({
      name: '', role: '', bio: '', tone: [], quirks: [], avoid: [],
    });
    expect(buildPersonaBlock(empty)).toBe('');
  });

  it('renders the default persona with identity + tone + avoid lines', () => {
    const block = buildPersonaBlock(baseTraits());
    expect(block).toContain('[PERSONA]');
    expect(block).toContain('[/PERSONA]');
    expect(block).toContain('You are Soul, TerranSoul companion.');
    expect(block).toContain('Tone: warm, concise.');
    expect(block).toContain('Never:');
  });

  it('uses "You are X." when only name is set', () => {
    const block = buildPersonaBlock(baseTraits({
      name: 'Lia', role: '', bio: '', tone: [], quirks: [], avoid: [],
    }));
    expect(block).toContain('You are Lia.');
    expect(block).not.toContain(',');
  });

  it('uses "You are a Y." when only role is set', () => {
    const block = buildPersonaBlock(baseTraits({
      name: '', role: 'rogue scholar', bio: '', tone: [], quirks: [], avoid: [],
    }));
    expect(block).toContain('You are a rogue scholar.');
  });

  it('renders quirks in quoted form', () => {
    const block = buildPersonaBlock(baseTraits({
      quirks: ['ends sentences with indeed', 'hums occasionally'],
    }));
    expect(block).toContain('Quirks: "ends sentences with indeed"; "hums occasionally".');
  });

  it('caps the bio at 500 characters', () => {
    const longBio = 'x'.repeat(800);
    const block = buildPersonaBlock(baseTraits({ bio: longBio }));
    const bioLine = block.split('\n').find(l => l.startsWith('Background: '));
    expect(bioLine).toBeDefined();
    // "Background: " (12) + 500 chars
    expect(bioLine!.length).toBe(12 + 500);
  });

  it('caps each list field at 8 items', () => {
    const tone = Array.from({ length: 20 }, (_, i) => `t${i}`);
    const block = buildPersonaBlock(baseTraits({ tone }));
    const toneLine = block.split('\n').find(l => l.startsWith('Tone: '));
    expect(toneLine).toBeDefined();
    // 8 items joined by ", " and ending in "."
    expect(toneLine).toBe('Tone: t0, t1, t2, t3, t4, t5, t6, t7.');
  });

  it('deduplicates list items case-insensitively, keeping first occurrence', () => {
    const block = buildPersonaBlock(baseTraits({
      tone: ['Warm', 'warm', 'Concise', 'CONCISE', 'witty'],
    }));
    const toneLine = block.split('\n').find(l => l.startsWith('Tone: '));
    expect(toneLine).toBe('Tone: Warm, Concise, witty.');
  });

  it('strips control characters and collapses whitespace from string fields', () => {
    const block = buildPersonaBlock(baseTraits({
      name: 'Lia\u0007\u0000  the\tcurious',
      tone: ['warm\u0001'],
    }));
    expect(block).toContain('You are Lia the curious');
    expect(block).toContain('Tone: warm.');
    // No raw control chars made it through
    expect(/[\u0000-\u0008]/.test(block)).toBe(false);
  });

  it('ignores non-string entries in list fields', () => {
    const block = buildPersonaBlock(baseTraits({
      tone: ['warm', 42 as unknown as string, null as unknown as string, 'concise'],
    }));
    const toneLine = block.split('\n').find(l => l.startsWith('Tone: '));
    expect(toneLine).toBe('Tone: warm, concise.');
  });

  it('appends learned motion triggers when provided', () => {
    const motions: LearnedMotionRef[] = [
      { name: 'Master shrug', trigger: 'shrug' },
      { name: 'Headtilt', trigger: 'headtilt' },
    ];
    const block = buildPersonaBlock(baseTraits(), motions);
    expect(block).toContain('Personal motions');
    expect(block).toContain('shrug');
    expect(block).toContain('headtilt');
  });

  it('skips the motions line when no learned motions exist', () => {
    const block = buildPersonaBlock(baseTraits(), []);
    expect(block).not.toContain('Personal motions');
  });

  it('deduplicates motion triggers across expressions and motions', () => {
    const motions: LearnedMotionRef[] = [
      { name: 'a', trigger: 'shrug' },
      { name: 'b', trigger: 'Shrug' },
      { name: 'c', trigger: 'wave' },
    ];
    const block = buildPersonaBlock(baseTraits(), motions);
    const motionLine = block.split('\n').find(l => l.startsWith('Personal motions'));
    expect(motionLine).toContain('shrug');
    expect(motionLine).toContain('wave');
    // "Shrug" should be dedup'd against "shrug"
    expect((motionLine!.match(/[Ss]hrug/g) ?? []).length).toBe(1);
  });

  it('block leads with double-newline so it composes safely with existing prompt', () => {
    const block = buildPersonaBlock(baseTraits());
    expect(block.startsWith('\n\n[PERSONA]')).toBe(true);
    expect(block.endsWith('[/PERSONA]')).toBe(true);
  });
});
