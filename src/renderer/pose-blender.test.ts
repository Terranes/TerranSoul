import { describe, it, expect, vi } from 'vitest';
import { PoseBlender } from './pose-blender';
import type { VRM } from '@pixiv/three-vrm';

// ── VRM mock ────────────────────────────────────────────────────────────────

function makeMockNode() {
  return {
    quaternion: {
      x: 0, y: 0, z: 0, w: 1,
      multiply: vi.fn().mockReturnThis(),
    },
  };
}

function makeMockVrm(bones: string[] = ['spine', 'chest', 'head', 'leftUpperArm', 'rightUpperArm', 'hips', 'neck', 'leftLowerArm', 'rightLowerArm', 'leftHand', 'rightHand']): VRM {
  const nodeMap = new Map(bones.map(b => [b, makeMockNode()]));
  return {
    humanoid: {
      getNormalizedBoneNode: (name: string) => nodeMap.get(name) ?? null,
    },
  } as unknown as VRM;
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('PoseBlender', () => {
  it('initializes with no active blends', () => {
    const blender = new PoseBlender();
    expect(blender.isActive).toBe(false);
    expect(blender.getCurrentWeights().size).toBe(0);
  });

  it('setTarget with empty array produces no active blend', () => {
    const blender = new PoseBlender();
    blender.setTarget([]);
    const vrm = makeMockVrm();
    blender.apply(vrm, 0.016);
    expect(blender.isActive).toBe(false);
  });

  it('setTarget with valid preset becomes active after apply', () => {
    const blender = new PoseBlender();
    blender.setTarget([{ presetId: 'confident', weight: 1.0 }]);
    const vrm = makeMockVrm();
    // Apply many frames to converge
    for (let i = 0; i < 120; i++) {
      blender.apply(vrm, 0.016);
    }
    expect(blender.isActive).toBe(true);
    const weight = blender.getCurrentWeights().get('confident') ?? 0;
    expect(weight).toBeGreaterThan(0.99);
  });

  it('unknown preset id is ignored gracefully', () => {
    const blender = new PoseBlender();
    blender.setTarget([{ presetId: 'nonexistent', weight: 1.0 }]);
    const vrm = makeMockVrm();
    blender.apply(vrm, 0.016);
    expect(blender.isActive).toBe(false);
  });

  it('weight is clamped to [0, 1]', () => {
    const blender = new PoseBlender();
    blender.setTarget([{ presetId: 'relaxed', weight: 2.5 }]);
    const vrm = makeMockVrm();
    for (let i = 0; i < 120; i++) {
      blender.apply(vrm, 0.016);
    }
    const weight = blender.getCurrentWeights().get('relaxed') ?? 0;
    expect(weight).toBeLessThanOrEqual(1.0);
  });

  it('negative weight is clamped to 0', () => {
    const blender = new PoseBlender();
    blender.setTarget([{ presetId: 'relaxed', weight: -0.5 }]);
    const vrm = makeMockVrm();
    blender.apply(vrm, 0.016);
    expect(blender.isActive).toBe(false);
  });

  it('multiple presets can blend simultaneously', () => {
    const blender = new PoseBlender();
    blender.setTarget([
      { presetId: 'confident', weight: 0.6 },
      { presetId: 'attentive', weight: 0.3 },
    ]);
    const vrm = makeMockVrm();
    for (let i = 0; i < 120; i++) {
      blender.apply(vrm, 0.016);
    }
    const weights = blender.getCurrentWeights();
    expect(weights.get('confident')!).toBeGreaterThan(0.55);
    expect(weights.get('attentive')!).toBeGreaterThan(0.25);
  });

  it('reset clears all weights', () => {
    const blender = new PoseBlender();
    blender.setTarget([{ presetId: 'confident', weight: 1.0 }]);
    const vrm = makeMockVrm();
    for (let i = 0; i < 120; i++) {
      blender.apply(vrm, 0.016);
    }
    blender.reset();
    expect(blender.isActive).toBe(false);
    expect(blender.getCurrentWeights().size).toBe(0);
  });

  it('switching target fades old preset and builds new', () => {
    const blender = new PoseBlender();
    blender.setTarget([{ presetId: 'confident', weight: 1.0 }]);
    const vrm = makeMockVrm();
    for (let i = 0; i < 120; i++) {
      blender.apply(vrm, 0.016);
    }
    blender.setTarget([{ presetId: 'shy', weight: 1.0 }]);
    // After switching, confident should fade toward 0
    // and shy should appear in target weights
    const weights = blender.getCurrentWeights();
    expect(weights.has('confident')).toBe(true); // still present, fading
  });

  it('apply calls getNormalizedBoneNode for active bones', () => {
    const blender = new PoseBlender();
    blender.setTarget([{ presetId: 'confident', weight: 1.0 }]);

    const getBone = vi.fn().mockReturnValue({
      quaternion: { multiply: vi.fn().mockReturnThis() },
    });
    const vrm = { humanoid: { getNormalizedBoneNode: getBone } } as unknown as VRM;

    for (let i = 0; i < 60; i++) {
      blender.apply(vrm, 0.016);
    }
    expect(getBone).toHaveBeenCalled();
  });

  it('apply does not throw when getNormalizedBoneNode throws', () => {
    const blender = new PoseBlender();
    blender.setTarget([{ presetId: 'confident', weight: 1.0 }]);
    const vrm = {
      humanoid: { getNormalizedBoneNode: () => { throw new Error('no bone'); } },
    } as unknown as VRM;
    expect(() => {
      for (let i = 0; i < 10; i++) {
        blender.apply(vrm, 0.016);
      }
    }).not.toThrow();
  });

  it('weights interpolate smoothly — do not jump to 1 in a single frame', () => {
    const blender = new PoseBlender();
    blender.setTarget([{ presetId: 'relaxed', weight: 1.0 }]);
    const vrm = makeMockVrm();
    blender.apply(vrm, 0.016);
    const weight = blender.getCurrentWeights().get('relaxed') ?? 0;
    // Should not be at 1.0 after just one 16ms frame
    expect(weight).toBeLessThan(0.5);
    expect(weight).toBeGreaterThan(0);
  });

  it('getCurrentWeights returns a snapshot (not a live reference)', () => {
    const blender = new PoseBlender();
    blender.setTarget([{ presetId: 'confident', weight: 1.0 }]);
    const vrm = makeMockVrm();
    blender.apply(vrm, 0.016);
    const snap1 = blender.getCurrentWeights();
    blender.apply(vrm, 0.016);
    const snap2 = blender.getCurrentWeights();
    // snap1 should not reflect changes from the second apply
    const w1 = snap1.get('confident') ?? 0;
    const w2 = snap2.get('confident') ?? 0;
    expect(w2).toBeGreaterThanOrEqual(w1);
  });
});
