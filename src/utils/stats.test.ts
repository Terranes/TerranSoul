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
    // Baseline 1 across the board — fresh adventurer starts at level 1.
    expect(base.intelligence).toBe(1);
    expect(base.wisdom).toBe(1);
    expect(base.charisma).toBe(1);
    expect(base.perception).toBe(1);
    expect(base.dexterity).toBe(1);
    expect(base.endurance).toBe(1);
  });

  it('clamps to [0, 100]', () => {
    // Many heavy contributors → must cap at 100.
    // Brain skills no longer have flat weights here — supply a max-tier brain
    // boost instead so the cap behaviour is still exercised.
    const heavy = ['agents', 'memory', 'vision'];
    const flagshipBoost = { intelligence: 70, wisdom: 25, dexterity: 15 };
    expect(computeStat('intelligence', heavy, flagshipBoost)).toBeLessThanOrEqual(100);
    expect(computeStat('intelligence', heavy, flagshipBoost)).toBeGreaterThan(50);
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

  it('applies the brainBoost on top of the skill-derived sum', () => {
    const baseInt = computeStat('intelligence', []);
    const boosted = computeStat('intelligence', [], { intelligence: 30 });
    expect(boosted - baseInt).toBe(30);
  });

  it('a flagship-tier brain boosts INT/WIS/DEX much more than basic', () => {
    const basic = computeStats([], { intelligence: 5,  wisdom: 2,  dexterity: 2  });
    const flag  = computeStats([], { intelligence: 70, wisdom: 25, dexterity: 15 });
    expect(flag.intelligence - basic.intelligence).toBeGreaterThanOrEqual(60);
    expect(flag.wisdom       - basic.wisdom).toBeGreaterThanOrEqual(20);
    expect(flag.dexterity    - basic.dexterity).toBeGreaterThanOrEqual(10);
  });

  it('does not bleed brainBoost into unrelated stats', () => {
    const before = computeStats([]);
    const after  = computeStats([], { intelligence: 70 });
    expect(after.intelligence).toBeGreaterThan(before.intelligence);
    expect(after.endurance).toBe(before.endurance);
    expect(after.charisma).toBe(before.charisma);
  });
});
