import { describe, it, expect } from 'vitest';
import { getModelTier, getModelBoost, getTierBoost, TIER_BOOSTS, DEFAULT_MODEL_TIER } from './model-benchmarks';

describe('model-benchmarks', () => {
  it('returns the default tier (basic) for unknown models', () => {
    expect(getModelTier('something-totally-made-up')).toBe(DEFAULT_MODEL_TIER);
    expect(getModelTier(null)).toBe(DEFAULT_MODEL_TIER);
    expect(getModelTier(undefined)).toBe(DEFAULT_MODEL_TIER);
    expect(getModelTier('')).toBe(DEFAULT_MODEL_TIER);
  });

  it('matches case-insensitively', () => {
    expect(getModelTier('Claude-Opus-4.7')).toBe('s');
    expect(getModelTier('CLAUDE-OPUS-4.7')).toBe('s');
  });

  it('classifies frontier flagships as tier S', () => {
    expect(getModelTier('claude-opus-4.7')).toBe('s');
    expect(getModelTier('gpt-5')).toBe('s');
    expect(getModelTier('gemini-2.5-pro')).toBe('s');
  });

  it('classifies premium models as tier A', () => {
    expect(getModelTier('claude-sonnet-4.5')).toBe('a');
    expect(getModelTier('gpt-4o')).toBe('a');
    expect(getModelTier('gemma4:31b')).toBe('a');
  });

  it('classifies strong open models as tier B', () => {
    expect(getModelTier('llama-3.3-70b-versatile')).toBe('b');
    expect(getModelTier('meta-llama/llama-3.3-70b-instruct:free')).toBe('b');
    expect(getModelTier('gemma3:27b')).toBe('b');
    expect(getModelTier('gemini-2.0-flash')).toBe('b');
  });

  it('classifies decent mid-size models as tier C', () => {
    expect(getModelTier('gemma3:4b')).toBe('c');
    expect(getModelTier('phi4-mini')).toBe('c');
    expect(getModelTier('mistral-small-latest')).toBe('c');
  });

  it('classifies basic / tiny models as tier D', () => {
    expect(getModelTier('gemma3:1b')).toBe('d');
    expect(getModelTier('tinyllama')).toBe('d');
    expect(getModelTier('openai')).toBe('d'); // Pollinations' opaque alias
  });

  it('tier boosts strictly increase from D to S for INT', () => {
    expect(TIER_BOOSTS.s.intelligence!).toBeGreaterThan(TIER_BOOSTS.a.intelligence!);
    expect(TIER_BOOSTS.a.intelligence!).toBeGreaterThan(TIER_BOOSTS.b.intelligence!);
    expect(TIER_BOOSTS.b.intelligence!).toBeGreaterThan(TIER_BOOSTS.c.intelligence!);
    expect(TIER_BOOSTS.c.intelligence!).toBeGreaterThan(TIER_BOOSTS.d.intelligence!);
  });

  it('Claude Opus 4.7 boosts INT much more than Gemma 4 e2b (per the spec)', () => {
    const opus = getModelBoost('claude-opus-4.7');
    const gemma = getModelBoost('gemma4:e2b');
    expect((opus.intelligence ?? 0) - (gemma.intelligence ?? 0)).toBeGreaterThanOrEqual(40);
  });

  it('getModelBoost returns an empty object when no model is given', () => {
    expect(getModelBoost(null)).toEqual({});
    expect(getModelBoost(undefined)).toEqual({});
    expect(getModelBoost('')).toEqual({});
  });

  it('getTierBoost returns the same object as the table', () => {
    expect(getTierBoost('s')).toBe(TIER_BOOSTS.s);
    expect(getTierBoost('d')).toBe(TIER_BOOSTS.d);
  });
});
