import { describe, it, expect, vi } from 'vitest';
import { GesturePlayer } from './gesture-player';
import { getAllGestures } from './gestures';
import type { VRM } from '@pixiv/three-vrm';

// ── Mock VRM ────────────────────────────────────────────────────────────────

function makeMockNode() {
  return {
    quaternion: {
      x: 0, y: 0, z: 0, w: 1,
      multiply: vi.fn().mockReturnThis(),
    },
  };
}

function makeMockVrm(bones: string[] = [
  'head', 'neck', 'spine', 'chest', 'hips',
  'leftShoulder', 'rightShoulder',
  'rightUpperArm', 'rightLowerArm',
  'leftUpperArm', 'leftLowerArm',
]): VRM {
  const nodeMap = new Map(bones.map(b => [b, makeMockNode()]));
  return {
    humanoid: {
      getNormalizedBoneNode: (name: string) => nodeMap.get(name) ?? null,
    },
  } as unknown as VRM;
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('GesturePlayer', () => {
  it('initializes as not playing', () => {
    const player = new GesturePlayer();
    expect(player.isPlaying).toBe(false);
    expect(player.currentId).toBeNull();
    expect(player.queueLength).toBe(0);
  });

  it('play() with known gesture starts it', () => {
    const player = new GesturePlayer();
    const result = player.play('nod');
    expect(result).toBe(true);
    expect(player.isPlaying).toBe(true);
    expect(player.currentId).toBe('nod');
  });

  it('play() with unknown gesture returns false', () => {
    const player = new GesturePlayer();
    const result = player.play('nonexistent-gesture');
    expect(result).toBe(false);
    expect(player.isPlaying).toBe(false);
  });

  it('apply() does not throw on null VRM humanoid', () => {
    const player = new GesturePlayer();
    player.play('nod');
    const vrm = { humanoid: null } as unknown as VRM;
    expect(() => player.apply(vrm, 0.016)).not.toThrow();
  });

  it('apply() advances elapsed and gesture completes', () => {
    const player = new GesturePlayer();
    player.play('nod'); // duration 0.6s
    const vrm = makeMockVrm();
    // Advance past duration
    for (let i = 0; i < 50; i++) {
      player.apply(vrm, 0.02);
    }
    // 50 * 0.02 = 1.0s > 0.6s — should be done
    expect(player.isPlaying).toBe(false);
  });

  it('queues a second gesture after the first', () => {
    const player = new GesturePlayer();
    player.play('nod');
    player.play('wave');
    expect(player.currentId).toBe('nod');
    expect(player.queueLength).toBe(1);
  });

  it('queued gesture starts after first completes', () => {
    const player = new GesturePlayer();
    player.play('nod'); // 0.6s
    player.play('shrug');
    const vrm = makeMockVrm();
    // Advance past nod duration
    for (let i = 0; i < 40; i++) {
      player.apply(vrm, 0.02);
    }
    // 40 * 0.02 = 0.8s > 0.6s — nod done, shrug should start
    expect(player.currentId).toBe('shrug');
  });

  it('stop() clears active gesture and queue', () => {
    const player = new GesturePlayer();
    player.play('nod');
    player.play('wave');
    player.stop();
    expect(player.isPlaying).toBe(false);
    expect(player.currentId).toBeNull();
    expect(player.queueLength).toBe(0);
  });

  it('same gesture while playing does not re-queue', () => {
    const player = new GesturePlayer();
    player.play('nod');
    player.play('nod'); // same gesture — should be ignored
    expect(player.queueLength).toBe(0);
  });

  it('respects queue max depth (4)', () => {
    const player = new GesturePlayer();
    player.play('nod');
    player.play('wave');
    player.play('shrug');
    player.play('bow');
    player.play('lean-in');
    player.play('head-tilt'); // 6th — should be dropped
    expect(player.queueLength).toBeLessThanOrEqual(4);
  });

  it('apply() calls getNormalizedBoneNode for active bones', () => {
    const player = new GesturePlayer();
    player.play('nod');
    const getBone = vi.fn().mockReturnValue({
      quaternion: { multiply: vi.fn().mockReturnThis() },
    });
    const vrm = { humanoid: { getNormalizedBoneNode: getBone } } as unknown as VRM;
    player.apply(vrm, 0.016);
    expect(getBone).toHaveBeenCalled();
  });
});

describe('GesturePlayer — built-in gestures', () => {
  it('all 10 built-in gestures can be played', () => {
    const ids = ['nod', 'wave', 'shrug', 'lean-in', 'head-tilt', 'reach-out', 'bow', 'nod-slow', 'shake-head', 'idle-fidget'];
    for (const id of ids) {
      const player = new GesturePlayer();
      expect(player.play(id)).toBe(true);
    }
  });

  it('all built-in gestures have positive duration', () => {
    for (const g of getAllGestures()) {
      expect(g.duration).toBeGreaterThan(0);
    }
  });

  it('all built-in gestures have at least 2 keyframes', () => {
    for (const g of getAllGestures()) {
      expect(g.keyframes.length).toBeGreaterThanOrEqual(2);
    }
  });

  it('all keyframe times are within [0, duration]', () => {
    for (const g of getAllGestures()) {
      for (const kf of g.keyframes) {
        expect(kf.time).toBeGreaterThanOrEqual(0);
        expect(kf.time).toBeLessThanOrEqual(g.duration + 0.001);
      }
    }
  });
});
