/* eslint-disable prefer-spread */
import { describe, it, expect, beforeEach } from 'vitest';
import { PoseAnimator, POSE_BONES, type LlmPoseFrame } from './pose-animator';

/**
 * Build a minimal mock VRM whose `humanoid.getNormalizedBoneNode`
 * returns a unique three-like rotation object per bone. The blender
 * mutates `node.rotation.{x,y,z}` directly, so the tests can read those
 * back to verify it produced the right offsets.
 */
function makeMockVrm() {
  const nodes = new Map<string, { rotation: { x: number; y: number; z: number } }>();
  for (const name of POSE_BONES) {
    nodes.set(name, { rotation: { x: 0, y: 0, z: 0 } });
  }
  const expressionSets: Array<[string, number]> = [];
  return {
    nodes,
    expressionSets,
    vrm: {
      humanoid: {
        getNormalizedBoneNode(n: string) {
          return nodes.get(n) ?? null;
        },
      },
      expressionManager: {
        setValue(name: string, value: number) {
          expressionSets.push([name, value]);
        },
      },
    },
  };
}

/** Reset every bone rotation to zero (simulates `CharacterAnimator.flushBones`). */
function resetBones(mock: ReturnType<typeof makeMockVrm>) {
  for (const node of mock.nodes.values()) {
    node.rotation.x = 0;
    node.rotation.y = 0;
    node.rotation.z = 0;
  }
}

/** Run the blender for `seconds` of simulated time at 60fps. */
function tick(animator: PoseAnimator, mock: ReturnType<typeof makeMockVrm>, seconds: number) {
  const frames = Math.max(1, Math.round(seconds * 60));
  const dt = seconds / frames;
  for (let i = 0; i < frames; i++) {
    resetBones(mock);
    animator.apply(mock.vrm as never, dt);
  }
}

describe('PoseAnimator', () => {
  let animator: PoseAnimator;
  let mock: ReturnType<typeof makeMockVrm>;

  beforeEach(() => {
    animator = new PoseAnimator();
    mock = makeMockVrm();
  });

  it('starts with no active pose and zero weight', () => {
    expect(animator.isActive).toBe(false);
    expect(animator.currentWeight).toBe(0);
  });

  it('applies a pose frame to canonical bones after fade-in', () => {
    const frame: LlmPoseFrame = {
      bones: { head: [0.3, 0, 0], spine: [0, 0, 0.1] },
      duration_s: 1.0,
    };
    animator.applyFrame(frame);
    expect(animator.isActive).toBe(true);

    // Run past fade-in (0.3s) plus a hold buffer to settle the spring.
    tick(animator, mock, 0.8);

    const head = mock.nodes.get('head')!;
    const spine = mock.nodes.get('spine')!;
    // After ~0.8s the weight should be near 1 and the bone offsets
    // should approach their targets (allow loose tolerance for spring).
    expect(head.rotation.x).toBeGreaterThan(0.2);
    expect(spine.rotation.z).toBeGreaterThan(0.07);
  });

  it('drops unknown bones', () => {
    const frame: LlmPoseFrame = {
      bones: { head: [0.2, 0, 0], tail: [1, 1, 1] },
    };
    animator.applyFrame(frame);
    tick(animator, mock, 0.6);

    expect(mock.nodes.has('tail')).toBe(false);
    expect(mock.nodes.get('head')!.rotation.x).toBeGreaterThan(0.1);
  });

  it('clamps out-of-range Eulers to ±0.5', () => {
    const frame: LlmPoseFrame = {
      bones: { head: [5.0, -10.0, 0] },
      duration_s: 1.0,
    };
    animator.applyFrame(frame);
    tick(animator, mock, 1.0);

    const head = mock.nodes.get('head')!;
    expect(head.rotation.x).toBeLessThanOrEqual(0.5 + 1e-6);
    expect(head.rotation.y).toBeGreaterThanOrEqual(-0.5 - 1e-6);
  });

  it('replaces non-finite values with zero', () => {
    const frame: LlmPoseFrame = {
      bones: { head: [Infinity, NaN, 0] },
      duration_s: 1.0,
    };
    animator.applyFrame(frame);
    tick(animator, mock, 1.0);

    const head = mock.nodes.get('head')!;
    expect(head.rotation.x).toBeCloseTo(0, 5);
    expect(head.rotation.y).toBeCloseTo(0, 5);
  });

  it('ignores frames with no recognised bones', () => {
    animator.applyFrame({ bones: { tail: [0.1, 0, 0] } });
    expect(animator.isActive).toBe(false);
  });

  it('fades back to idle after the hold duration', () => {
    animator.applyFrame({ bones: { head: [0.3, 0, 0] }, duration_s: 0.2 });
    // Past fade-in (0.3) + hold (0.2) + fade-out (0.5) + slack
    tick(animator, mock, 1.5);

    expect(animator.isActive).toBe(false);
    // Bones damp back toward zero; allow a small residual.
    expect(Math.abs(mock.nodes.get('head')!.rotation.x)).toBeLessThan(0.05);
  });

  it('yields to VRMA playback by fading out', () => {
    animator.applyFrame({ bones: { head: [0.3, 0, 0] }, duration_s: 5.0 });
    tick(animator, mock, 0.4); // settle into hold

    animator.setVrmaPlaying(true);
    tick(animator, mock, 1.0); // past fade-out

    expect(animator.isActive).toBe(false);
    expect(Math.abs(mock.nodes.get('head')!.rotation.x)).toBeLessThan(0.05);
  });

  it('drops new frames while VRMA is playing', () => {
    animator.setVrmaPlaying(true);
    animator.applyFrame({ bones: { head: [0.3, 0, 0] } });
    expect(animator.isActive).toBe(false);
  });

  it('applies expression weights to the VRM', () => {
    animator.applyFrame({
      bones: { head: [0.1, 0, 0] },
      expression: { happy: 0.8 },
      duration_s: 1.0,
    });
    tick(animator, mock, 0.6);
    // The VRM should have received at least one happy=… set call.
    const happy = mock.expressionSets.filter(([k]) => k === 'happy');
    expect(happy.length).toBeGreaterThan(0);
    expect(happy[happy.length - 1][1]).toBeGreaterThan(0);
  });

  it('clamps expression weights to [0, 1]', () => {
    animator.applyFrame({
      bones: { head: [0.1, 0, 0] },
      expression: { happy: 5 },
      duration_s: 1.0,
    });
    tick(animator, mock, 0.5);
    const last = mock.expressionSets.filter(([k]) => k === 'happy').pop();
    expect(last).toBeDefined();
    // weight ≤ 1 (input weight) × current blend weight (≤1) → ≤1.
    expect(last![1]).toBeLessThanOrEqual(1.0 + 1e-6);
  });

  it('reset() fades out an active pose', () => {
    animator.applyFrame({ bones: { head: [0.3, 0, 0] }, duration_s: 5.0 });
    tick(animator, mock, 0.4);
    animator.reset();
    tick(animator, mock, 1.0);
    expect(animator.isActive).toBe(false);
  });

  it('handles a missing VRM gracefully', () => {
    animator.applyFrame({ bones: { head: [0.2, 0, 0] } });
    expect(() => animator.apply(null, 0.016)).not.toThrow();
  });

  it('replacing an active pose damps toward the new target', () => {
    animator.applyFrame({ bones: { head: [0.3, 0, 0] }, duration_s: 5.0 });
    tick(animator, mock, 0.4);
    animator.applyFrame({ bones: { head: [-0.3, 0, 0] }, duration_s: 5.0 });
    tick(animator, mock, 1.0);
    expect(mock.nodes.get('head')!.rotation.x).toBeLessThan(0);
  });
});
