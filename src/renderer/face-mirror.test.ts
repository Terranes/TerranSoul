import { describe, it, expect } from 'vitest';
import {
  mapBlendshapesToVRM,
  dampWeight,
  smoothWeights,
  zeroWeights,
  type VrmExpressionWeights,
} from './face-mirror';

describe('face-mirror pure mapper', () => {
  // Helper: create a scores map from a partial object
  function scores(obj: Record<string, number>): ReadonlyMap<string, number> {
    return new Map(Object.entries(obj));
  }

  it('returns all-zero for empty blendshapes', () => {
    const w = mapBlendshapesToVRM(scores({}));
    // Neutral should be 1 (1 - sum of emotions = 1 - 0 = 1)
    expect(w.neutral).toBe(1);
    // Relaxed = 1 - max(angry, sad, surprised) = 1
    expect(w.relaxed).toBe(1);
    // All others zero
    expect(w.happy).toBe(0);
    expect(w.sad).toBe(0);
    expect(w.angry).toBe(0);
    expect(w.surprised).toBe(0);
    expect(w.aa).toBe(0);
    expect(w.blink).toBe(0);
  });

  it('maps smiling to happy', () => {
    const w = mapBlendshapesToVRM(scores({
      mouthSmileLeft: 0.9,
      mouthSmileRight: 0.9,
      cheekSquintLeft: 0.5,
      cheekSquintRight: 0.5,
    }));
    // happy = smile * 0.7 + squint * 0.3 = 0.9*0.7 + 0.5*0.3 = 0.63 + 0.15 = 0.78
    expect(w.happy).toBeCloseTo(0.78, 2);
  });

  it('maps frown + browInnerUp to sad', () => {
    const w = mapBlendshapesToVRM(scores({
      mouthFrownLeft: 0.8,
      mouthFrownRight: 0.8,
      browInnerUp: 0.6,
    }));
    // sad = frown * 0.6 + browInnerUp * 0.4 = 0.8*0.6 + 0.6*0.4 = 0.48 + 0.24 = 0.72
    expect(w.sad).toBeCloseTo(0.72, 2);
  });

  it('maps browDown + noseSneer to angry', () => {
    const w = mapBlendshapesToVRM(scores({
      browDownLeft: 0.7,
      browDownRight: 0.7,
      noseSneerLeft: 0.6,
      noseSneerRight: 0.6,
      mouthPressLeft: 0.5,
      mouthPressRight: 0.5,
    }));
    // angry = browDown*0.5 + sneer*0.3 + press*0.2 = 0.35+0.18+0.10 = 0.63
    expect(w.angry).toBeCloseTo(0.63, 2);
  });

  it('maps eyeWide + jawOpen to surprised', () => {
    const w = mapBlendshapesToVRM(scores({
      eyeWideLeft: 0.8,
      eyeWideRight: 0.8,
      jawOpen: 0.6,
      browOuterUpLeft: 0.5,
      browOuterUpRight: 0.5,
    }));
    // surprised = eyeWide*0.5 + jawOpen*0.3 + browOuterUp*0.2 = 0.40+0.18+0.10 = 0.68
    expect(w.surprised).toBeCloseTo(0.68, 2);
  });

  it('blink is max of left/right', () => {
    const w = mapBlendshapesToVRM(scores({
      eyeBlinkLeft: 0.3,
      eyeBlinkRight: 0.9,
    }));
    expect(w.blink).toBeCloseTo(0.9, 2);
  });

  it('viseme aa combines jawOpen + mouthFunnel', () => {
    const w = mapBlendshapesToVRM(scores({
      jawOpen: 0.8,
      mouthFunnel: 0.4,
    }));
    // aa = jawOpen*0.7 + mouthFunnel*0.3 = 0.56 + 0.12 = 0.68
    expect(w.aa).toBeCloseTo(0.68, 2);
  });

  it('viseme ou combines pucker + funnel', () => {
    const w = mapBlendshapesToVRM(scores({
      mouthPucker: 0.7,
      mouthFunnel: 0.5,
    }));
    // ou = mean([0.7, 0.5]) = 0.6
    expect(w.ou).toBeCloseTo(0.6, 2);
  });

  it('lookAt X: right gaze is positive', () => {
    const w = mapBlendshapesToVRM(scores({
      eyeLookOutRight: 0.8,
      eyeLookOutLeft: 0.2,
    }));
    // lookAtX = 0.8 - 0.2 = 0.6
    expect(w.lookAtX).toBeCloseTo(0.6, 2);
  });

  it('lookAt Y: upward gaze is positive', () => {
    const w = mapBlendshapesToVRM(scores({
      eyeLookUpLeft: 0.6,
      eyeLookUpRight: 0.6,
      eyeLookDownLeft: 0.1,
      eyeLookDownRight: 0.1,
    }));
    // lookAtY = 0.6 - 0.1 = 0.5
    expect(w.lookAtY).toBeCloseTo(0.5, 2);
  });

  it('all weights are clamped to [0, 1]', () => {
    // Feed extreme values that could overflow
    const w = mapBlendshapesToVRM(scores({
      mouthSmileLeft: 1, mouthSmileRight: 1,
      cheekSquintLeft: 1, cheekSquintRight: 1,
      eyeWideLeft: 1, eyeWideRight: 1,
      jawOpen: 1, browOuterUpLeft: 1, browOuterUpRight: 1,
      mouthFrownLeft: 1, mouthFrownRight: 1, browInnerUp: 1,
    }));
    for (const [key, val] of Object.entries(w)) {
      expect(val, `${key} should be <= 1`).toBeLessThanOrEqual(1);
      expect(val, `${key} should be >= -1`).toBeGreaterThanOrEqual(-1);
    }
  });
});

describe('dampWeight', () => {
  it('smooths toward target', () => {
    const v = dampWeight(0, 1, 12, 0.016);
    expect(v).toBeGreaterThan(0);
    expect(v).toBeLessThan(1);
  });

  it('reaches target after many frames', () => {
    let v = 0;
    for (let i = 0; i < 300; i++) v = dampWeight(v, 1, 12, 0.016);
    expect(v).toBeCloseTo(1, 3);
  });

  it('stays at target when already there', () => {
    expect(dampWeight(0.5, 0.5, 12, 0.016)).toBe(0.5);
  });
});

describe('smoothWeights', () => {
  it('converges smoothed weights to raw', () => {
    const smoothed = zeroWeights();
    const raw: VrmExpressionWeights = {
      happy: 0.8, sad: 0, angry: 0, relaxed: 0.2, surprised: 0, neutral: 0,
      aa: 0, ih: 0, ou: 0, ee: 0, oh: 0, blink: 0, lookAtX: 0, lookAtY: 0,
    };
    for (let i = 0; i < 300; i++) smoothWeights(smoothed, raw, 12, 0.016);
    expect(smoothed.happy).toBeCloseTo(0.8, 2);
    expect(smoothed.relaxed).toBeCloseTo(0.2, 2);
  });
});

describe('zeroWeights', () => {
  it('returns all zeros', () => {
    const w = zeroWeights();
    for (const val of Object.values(w)) {
      expect(val).toBe(0);
    }
  });
});
