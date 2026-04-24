import { describe, it, expect } from 'vitest';
import { bakeMotionToClip, bakeAllMotions } from './vrma-baker';
import type { LearnedMotion } from '../stores/persona-types';

// ── Helpers ─────────────────────────────────────────────────────────────────

function makeMotion(overrides?: Partial<LearnedMotion>): LearnedMotion {
  return {
    id: 'test-motion-1',
    kind: 'motion',
    name: 'Test Wave',
    trigger: 'wave',
    fps: 30,
    duration_s: 1.0,
    frames: [
      {
        t: 0,
        bones: {
          head: [0, 0, 0],
          leftUpperArm: [0, 0, 1.0],
          rightUpperArm: [0, 0, -1.0],
        },
      },
      {
        t: 0.5,
        bones: {
          head: [0.1, 0, 0],
          leftUpperArm: [0, 0, 0.5],
          rightUpperArm: [0, 0, -0.5],
        },
      },
      {
        t: 1.0,
        bones: {
          head: [0, 0, 0],
          leftUpperArm: [0, 0, 1.0],
          rightUpperArm: [0, 0, -1.0],
        },
      },
    ],
    learnedAt: Date.now(),
    ...overrides,
  };
}

// ── Tests ───────────────────────────────────────────────────────────────────

describe('bakeMotionToClip', () => {
  it('returns null for empty frames', () => {
    const motion = makeMotion({ frames: [] });
    expect(bakeMotionToClip(motion)).toBeNull();
  });

  it('creates an AnimationClip from a valid motion', () => {
    const motion = makeMotion();
    const clip = bakeMotionToClip(motion);
    expect(clip).not.toBeNull();
    expect(clip!.name).toBe('wave');
    expect(clip!.duration).toBe(1.0);
  });

  it('creates quaternion tracks for each bone with data', () => {
    const motion = makeMotion();
    const clip = bakeMotionToClip(motion)!;
    // 3 bones have data: head, leftUpperArm, rightUpperArm
    expect(clip.tracks.length).toBe(3);
    const trackNames = clip.tracks.map(t => t.name).sort();
    expect(trackNames).toEqual([
      'head.quaternion',
      'leftUpperArm.quaternion',
      'rightUpperArm.quaternion',
    ]);
  });

  it('each track has correct number of keyframes', () => {
    const motion = makeMotion();
    const clip = bakeMotionToClip(motion)!;
    for (const track of clip.tracks) {
      // 3 frames → 3 timestamps
      expect(track.times.length).toBe(3);
      // quaternion = 4 values per keyframe → 12 total
      expect(track.values.length).toBe(12);
    }
  });

  it('uses trigger as clip name by default', () => {
    const motion = makeMotion({ trigger: 'shrug' });
    const clip = bakeMotionToClip(motion)!;
    expect(clip.name).toBe('shrug');
  });

  it('allows overriding clip name', () => {
    const motion = makeMotion();
    const clip = bakeMotionToClip(motion, 'custom-name')!;
    expect(clip.name).toBe('custom-name');
  });

  it('handles single-frame motion', () => {
    const motion = makeMotion({
      frames: [{ t: 0, bones: { head: [0.1, 0.2, 0.3] } }],
      duration_s: 0,
    });
    const clip = bakeMotionToClip(motion);
    expect(clip).not.toBeNull();
    expect(clip!.tracks.length).toBe(1);
    expect(clip!.tracks[0].times.length).toBe(1);
  });

  it('skips bones with no keyframes', () => {
    const motion = makeMotion({
      frames: [
        { t: 0, bones: { head: [0, 0, 0] } },
        { t: 0.5, bones: { head: [0.1, 0, 0] } },
      ],
    });
    const clip = bakeMotionToClip(motion)!;
    // Only head has data
    expect(clip.tracks.length).toBe(1);
    expect(clip.tracks[0].name).toBe('head.quaternion');
  });

  it('produces valid quaternion values (unit quaternions)', () => {
    const motion = makeMotion();
    const clip = bakeMotionToClip(motion)!;
    for (const track of clip.tracks) {
      for (let i = 0; i < track.values.length; i += 4) {
        const x = track.values[i];
        const y = track.values[i + 1];
        const z = track.values[i + 2];
        const w = track.values[i + 3];
        const len = Math.sqrt(x * x + y * y + z * z + w * w);
        expect(len).toBeCloseTo(1.0, 4);
      }
    }
  });
});

describe('bakeAllMotions', () => {
  it('returns empty map for empty array', () => {
    const result = bakeAllMotions([]);
    expect(result.size).toBe(0);
  });

  it('bakes multiple motions keyed by trigger', () => {
    const motions = [
      makeMotion({ trigger: 'wave' }),
      makeMotion({ trigger: 'shrug', name: 'Shrug' }),
    ];
    const result = bakeAllMotions(motions);
    expect(result.size).toBe(2);
    expect(result.has('wave')).toBe(true);
    expect(result.has('shrug')).toBe(true);
  });

  it('skips motions with empty frames', () => {
    const motions = [
      makeMotion({ trigger: 'wave' }),
      makeMotion({ trigger: 'empty', frames: [] }),
    ];
    const result = bakeAllMotions(motions);
    expect(result.size).toBe(1);
    expect(result.has('wave')).toBe(true);
  });
});
