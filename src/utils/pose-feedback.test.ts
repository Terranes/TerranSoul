import { describe, it, expect } from 'vitest';
import { serializePoseContext, buildPoseContextSuffix } from './pose-feedback';

function makeWeights(entries: Record<string, number>): Map<string, number> {
  return new Map(Object.entries(entries));
}

describe('serializePoseContext', () => {
  it('returns empty string when no active poses and no gesture', () => {
    const result = serializePoseContext({
      weights: new Map(),
      lastGestureId: null,
      secondsSinceLastGesture: null,
    });
    expect(result).toBe('');
  });

  it('serializes a single active pose', () => {
    const result = serializePoseContext({
      weights: makeWeights({ confident: 0.7 }),
      lastGestureId: null,
      secondsSinceLastGesture: null,
    });
    expect(result).toBe('Current character pose: confident=0.7.');
  });

  it('serializes multiple poses sorted by weight descending', () => {
    const result = serializePoseContext({
      weights: makeWeights({ attentive: 0.3, confident: 0.6 }),
      lastGestureId: null,
      secondsSinceLastGesture: null,
    });
    expect(result).toContain('confident=0.6');
    // confident should come before attentive (higher weight)
    const confIdx = result.indexOf('confident');
    const attIdx = result.indexOf('attentive');
    expect(confIdx).toBeLessThan(attIdx);
  });

  it('includes last gesture with seconds ago', () => {
    const result = serializePoseContext({
      weights: new Map(),
      lastGestureId: 'nod',
      secondsSinceLastGesture: 3.2,
    });
    expect(result).toBe('Last gesture: nod (3.2s ago).');
  });

  it('includes last gesture without time when secondsSince is null', () => {
    const result = serializePoseContext({
      weights: new Map(),
      lastGestureId: 'wave',
      secondsSinceLastGesture: null,
    });
    expect(result).toBe('Last gesture: wave.');
  });

  it('combines pose and gesture info', () => {
    const result = serializePoseContext({
      weights: makeWeights({ thoughtful: 0.75 }),
      lastGestureId: 'nod',
      secondsSinceLastGesture: 1.5,
    });
    expect(result).toContain('Current character pose: thoughtful=0.75.');
    expect(result).toContain('Last gesture: nod (1.5s ago).');
  });

  it('excludes poses below 0.05 threshold', () => {
    const result = serializePoseContext({
      weights: makeWeights({ confident: 0.8, shy: 0.03 }),
      lastGestureId: null,
      secondsSinceLastGesture: null,
    });
    expect(result).toContain('confident');
    expect(result).not.toContain('shy');
  });

  it('limits output to at most 3 presets', () => {
    const result = serializePoseContext({
      weights: makeWeights({ confident: 0.5, shy: 0.4, excited: 0.3, bored: 0.2 }),
      lastGestureId: null,
      secondsSinceLastGesture: null,
    });
    // Count commas in pose section — max 2 commas = 3 entries
    const poseSection = result.split('Last')[0];
    const commaCount = (poseSection.match(/,/g) || []).length;
    expect(commaCount).toBeLessThanOrEqual(2);
  });

  it('rounds weights to 2 decimal places', () => {
    const result = serializePoseContext({
      weights: makeWeights({ relaxed: 0.666666 }),
      lastGestureId: null,
      secondsSinceLastGesture: null,
    });
    expect(result).toContain('relaxed=0.67');
  });

  it('output is compact (under 200 chars)', () => {
    const result = serializePoseContext({
      weights: makeWeights({ confident: 0.6, attentive: 0.3, playful: 0.1 }),
      lastGestureId: 'nod-slow',
      secondsSinceLastGesture: 12.5,
    });
    expect(result.length).toBeLessThan(200);
  });

  it('handles zero seconds since gesture', () => {
    const result = serializePoseContext({
      weights: new Map(),
      lastGestureId: 'wave',
      secondsSinceLastGesture: 0,
    });
    expect(result).toContain('wave (0s ago)');
  });
});

describe('buildPoseContextSuffix', () => {
  it('returns empty string when no context', () => {
    const result = buildPoseContextSuffix({
      weights: new Map(),
      lastGestureId: null,
      secondsSinceLastGesture: null,
    });
    expect(result).toBe('');
  });

  it('prefixes context with [Character state] when present', () => {
    const result = buildPoseContextSuffix({
      weights: makeWeights({ thoughtful: 0.75 }),
      lastGestureId: null,
      secondsSinceLastGesture: null,
    });
    expect(result).toMatch(/\[Character state\]/);
    expect(result).toContain('thoughtful=0.75');
  });

  it('starts with two newlines to separate from system prompt', () => {
    const result = buildPoseContextSuffix({
      weights: makeWeights({ confident: 0.5 }),
      lastGestureId: null,
      secondsSinceLastGesture: null,
    });
    expect(result.startsWith('\n\n')).toBe(true);
  });
});
