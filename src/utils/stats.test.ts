import { describe, it, expect } from 'vitest';
import { computeStat, computeStats, diffStats, STAT_DESCRIPTORS } from './stats';

describe('stats utility', () => {
  it('exposes all six stat descriptors with unique ids', () => {
    expect(STAT_DESCRIPTORS).toHaveLength(6);
    const ids = STAT_DESCRIPTORS.map(s => s.id);
    expect(new Set(ids).size).toBe(6);
    expect(ids).toContain('intelligence');
    expect(ids).toContain('endurance');
  });

  it('returns the baseline for an empty active list', () => {
    const base = computeStats([]);
    // Baseline 5 across the board
    expect(base.intelligence).toBe(5);
    expect(base.wisdom).toBe(5);
    expect(base.charisma).toBe(5);
    expect(base.perception).toBe(5);
    expect(base.dexterity).toBe(5);
    expect(base.endurance).toBe(5);
  });

  it('clamps to [0, 100]', () => {
    // Many heavy contributors → must cap at 100
    const heavy = ['free-brain', 'paid-brain', 'local-brain', 'agents', 'memory', 'vision'];
    expect(computeStat('intelligence', heavy)).toBeLessThanOrEqual(100);
    expect(computeStat('intelligence', heavy)).toBeGreaterThan(50);
  });

  it('weights TTS heavily for charisma', () => {
    const before = computeStat('charisma', []);
    const after = computeStat('charisma', ['tts']);
    expect(after - before).toBeGreaterThanOrEqual(40); // TTS weight 45
  });

  it('weights memory heavily for wisdom', () => {
    const before = computeStat('wisdom', []);
    const after = computeStat('wisdom', ['memory']);
    expect(after - before).toBeGreaterThanOrEqual(45); // memory weight 50
  });

  it('weights hotwords for perception', () => {
    const before = computeStat('perception', []);
    const after = computeStat('perception', ['hotwords']);
    expect(after - before).toBeGreaterThanOrEqual(20);
  });

  it('weights bgm/pet-mode for endurance', () => {
    const before = computeStat('endurance', []);
    const afterBgm = computeStat('endurance', ['bgm']);
    const afterBoth = computeStat('endurance', ['bgm', 'pet-mode']);
    expect(afterBgm).toBeGreaterThan(before);
    expect(afterBoth).toBeGreaterThan(afterBgm);
  });

  it('returns zero contribution for unknown skill ids', () => {
    const before = computeStat('intelligence', []);
    const after = computeStat('intelligence', ['totally-fake-skill']);
    expect(after).toBe(before);
  });

  it('diffStats returns per-stat deltas', () => {
    const before = computeStats([]);
    const after = computeStats(['memory']);
    const delta = diffStats(before, after);
    expect(delta.wisdom).toBeGreaterThan(0);
    expect(delta.intelligence).toBeGreaterThanOrEqual(0);
    expect(delta.endurance).toBe(0);
  });
});
