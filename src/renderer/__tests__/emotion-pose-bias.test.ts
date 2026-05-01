/**
 * Tests for emotion-pose-bias.ts (Chunk 14.16d).
 */
import { describe, it, expect } from 'vitest';
import {
  emotionTargetBias,
  EmotionPoseBias,
  EMOTION_BIAS_TABLE,
  BIAS_BONES,
  MAX_BIAS_RAD,
  type BiasEmotion,
} from '../emotion-pose-bias';

describe('emotion-pose-bias', () => {
  it('every table entry stays within MAX_BIAS_RAD', () => {
    for (const emotion of Object.keys(EMOTION_BIAS_TABLE) as BiasEmotion[]) {
      const table = EMOTION_BIAS_TABLE[emotion];
      for (const [bone, vals] of Object.entries(table)) {
        for (const v of vals!) {
          expect(Math.abs(v)).toBeLessThanOrEqual(MAX_BIAS_RAD);
        }
        // ensure bone is in the canonical list
        expect(BIAS_BONES).toContain(bone as never);
      }
    }
  });

  it('neutral emotion produces all-zero offsets', () => {
    const m = emotionTargetBias('neutral', 1);
    for (const bone of BIAS_BONES) {
      const v = m.get(bone)!;
      expect(v).toEqual([0, 0, 0]);
    }
  });

  it('happy lifts chest and head (negative X)', () => {
    const m = emotionTargetBias('happy', 1);
    expect(m.get('chest')![0]).toBeLessThan(0);
    expect(m.get('head')![0]).toBeLessThan(0);
  });

  it('sad drops head forward (positive X)', () => {
    const m = emotionTargetBias('sad', 1);
    expect(m.get('head')![0]).toBeGreaterThan(0);
    expect(m.get('chest')![0]).toBeGreaterThan(0);
  });

  it('shoulder bias is symmetric around left/right for each emotion', () => {
    const emotions: BiasEmotion[] = ['happy', 'sad', 'angry', 'relaxed', 'surprised'];
    for (const e of emotions) {
      const m = emotionTargetBias(e, 1);
      const left = m.get('leftShoulder')!;
      const right = m.get('rightShoulder')!;
      // Z-axis should be opposite-signed (mirrored body-side roll).
      expect(left[2] + right[2]).toBeCloseTo(0, 3);
    }
  });

  it('intensity scales the bias linearly', () => {
    const full = emotionTargetBias('happy', 1);
    const half = emotionTargetBias('happy', 0.5);
    for (const bone of BIAS_BONES) {
      const f = full.get(bone)!;
      const h = half.get(bone)!;
      for (let i = 0; i < 3; i++) {
        expect(h[i]).toBeCloseTo(f[i] * 0.5, 5);
      }
    }
  });

  it('intensity is clamped to [0, 1]', () => {
    const huge = emotionTargetBias('happy', 99);
    const negative = emotionTargetBias('happy', -2);
    for (const bone of BIAS_BONES) {
      const v = huge.get(bone)!;
      for (const x of v) expect(Math.abs(x)).toBeLessThanOrEqual(MAX_BIAS_RAD);
      const z = negative.get(bone)!;
      for (const x of z) expect(x).toBeCloseTo(0, 9);
    }
  });

  it('non-finite input never escapes the clamp', () => {
    const m = emotionTargetBias('happy', NaN);
    for (const bone of BIAS_BONES) {
      const v = m.get(bone)!;
      for (const x of v) expect(Number.isFinite(x)).toBe(true);
    }
  });

  it('EmotionPoseBias damps the weight toward zero on yield()', () => {
    const bias = new EmotionPoseBias();
    bias.setEmotion('happy', 1);
    // Step a few frames so the weight ramps toward 1.
    for (let i = 0; i < 10; i++) bias.apply(null, 0.1);
    expect(bias.currentWeight).toBeGreaterThan(0.5);
    bias.yield();
    for (let i = 0; i < 30; i++) bias.apply(null, 0.1);
    expect(bias.currentWeight).toBeLessThan(0.05);
  });

  it('EmotionPoseBias suppress=true still damps to zero even with active emotion', () => {
    const bias = new EmotionPoseBias();
    bias.setEmotion('angry', 1);
    for (let i = 0; i < 10; i++) bias.apply(null, 0.1, false);
    expect(bias.currentWeight).toBeGreaterThan(0.5);
    for (let i = 0; i < 30; i++) bias.apply(null, 0.1, true);
    expect(bias.currentWeight).toBeLessThan(0.05);
  });

  it('apply() with null vrm is a no-op (does not throw)', () => {
    const bias = new EmotionPoseBias();
    bias.setEmotion('happy', 1);
    expect(() => bias.apply(null, 0.016)).not.toThrow();
  });

  it('weight ramps toward 1 over ~0.5s with active non-neutral emotion', () => {
    const bias = new EmotionPoseBias();
    bias.setEmotion('happy', 1);
    let lastWeight = 0;
    for (let i = 0; i < 30; i++) {
      bias.apply(null, 0.05);
      // Weight is monotonically non-decreasing while target is 1.
      expect(bias.currentWeight).toBeGreaterThanOrEqual(lastWeight - 1e-9);
      lastWeight = bias.currentWeight;
    }
    expect(lastWeight).toBeGreaterThan(0.85);
  });

  it('switching to neutral fades the weight to zero', () => {
    const bias = new EmotionPoseBias();
    bias.setEmotion('happy', 1);
    for (let i = 0; i < 20; i++) bias.apply(null, 0.05);
    bias.setEmotion('neutral', 1);
    for (let i = 0; i < 40; i++) bias.apply(null, 0.05);
    expect(bias.currentWeight).toBeLessThan(0.05);
  });

  it('intensity = 0 fades the weight to zero even on a non-neutral emotion', () => {
    const bias = new EmotionPoseBias();
    bias.setEmotion('happy', 1);
    for (let i = 0; i < 20; i++) bias.apply(null, 0.05);
    bias.setEmotion('happy', 0);
    for (let i = 0; i < 40; i++) bias.apply(null, 0.05);
    expect(bias.currentWeight).toBeLessThan(0.05);
  });
});
