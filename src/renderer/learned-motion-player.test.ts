import { describe, it, expect, vi } from 'vitest';
import { LearnedMotionPlayer, applyLearnedExpression, clearExpressionPreview } from './learned-motion-player';
import type { LearnedExpression, LearnedMotion } from '../stores/persona-types';

// ── Helpers ─────────────────────────────────────────────────────────────────

function makeMotion(overrides?: Partial<LearnedMotion>): LearnedMotion {
  return {
    id: 'lmo-test-1',
    kind: 'motion',
    name: 'Test Wave',
    trigger: 'wave',
    fps: 30,
    duration_s: 1.0,
    frames: [
      { t: 0, bones: { head: [0, 0, 0], leftUpperArm: [0, 0, 1.0] } },
      { t: 0.5, bones: { head: [0.1, 0, 0], leftUpperArm: [0, 0, 0.5] } },
      { t: 1.0, bones: { head: [0, 0, 0], leftUpperArm: [0, 0, 1.0] } },
    ],
    learnedAt: Date.now(),
    ...overrides,
  };
}

function makeExpression(overrides?: Partial<LearnedExpression>): LearnedExpression {
  return {
    id: 'lex-test-1',
    kind: 'expression',
    name: 'Test Smile',
    trigger: 'smile',
    weights: { happy: 0.8, sad: 0, angry: 0, relaxed: 0.2, surprised: 0, neutral: 0, aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 },
    blink: 0.1,
    learnedAt: Date.now(),
    ...overrides,
  };
}

function makeMockVrmaManager(overrides?: Record<string, unknown>) {
  return {
    playClip: vi.fn().mockReturnValue(true),
    stop: vi.fn(),
    isPlaying: false,
    ...overrides,
  };
}

function makeMockVrm() {
  const values: Record<string, number> = {};
  return {
    expressionManager: {
      setValue: vi.fn((name: string, weight: number) => {
        values[name] = weight;
      }),
      _values: values,
    },
  };
}

// ── LearnedMotionPlayer ─────────────────────────────────────────────────────

describe('LearnedMotionPlayer', () => {
  it('plays a valid motion', () => {
    const mgr = makeMockVrmaManager();
    const player = new LearnedMotionPlayer(mgr as never);
    const result = player.play(makeMotion());
    expect(result).toBe(true);
    expect(mgr.playClip).toHaveBeenCalledOnce();
    // Clip arg is a THREE.AnimationClip
    const clip = mgr.playClip.mock.calls[0][0];
    expect(clip.name).toBe('wave');
  });

  it('returns false for empty frames', () => {
    const mgr = makeMockVrmaManager();
    const player = new LearnedMotionPlayer(mgr as never);
    const result = player.play(makeMotion({ frames: [] }));
    expect(result).toBe(false);
    expect(mgr.playClip).not.toHaveBeenCalled();
  });

  it('passes loop and fadeIn to playClip', () => {
    const mgr = makeMockVrmaManager();
    const player = new LearnedMotionPlayer(mgr as never);
    player.play(makeMotion(), true, 0.5);
    expect(mgr.playClip).toHaveBeenCalledWith(expect.anything(), true, 0.5);
  });

  it('delegates stop to VrmaManager', () => {
    const mgr = makeMockVrmaManager();
    const player = new LearnedMotionPlayer(mgr as never);
    player.stop(0.2);
    expect(mgr.stop).toHaveBeenCalledWith(0.2);
  });

  it('exposes isPlaying from VrmaManager', () => {
    const mgr = makeMockVrmaManager({ isPlaying: true });
    const player = new LearnedMotionPlayer(mgr as never);
    expect(player.isPlaying).toBe(true);
  });
});

// ── Expression preview ──────────────────────────────────────────────────────

describe('applyLearnedExpression', () => {
  it('sets all weight values on the VRM expression manager', () => {
    const vrm = makeMockVrm();
    const expr = makeExpression();
    applyLearnedExpression(vrm as never, expr);
    expect(vrm.expressionManager.setValue).toHaveBeenCalledWith('happy', 0.8);
    expect(vrm.expressionManager.setValue).toHaveBeenCalledWith('relaxed', 0.2);
    expect(vrm.expressionManager.setValue).toHaveBeenCalledWith('blink', 0.1);
  });

  it('handles missing expressionManager gracefully', () => {
    const vrm = { expressionManager: null };
    expect(() => applyLearnedExpression(vrm as never, makeExpression())).not.toThrow();
  });

  it('skips blink when undefined', () => {
    const vrm = makeMockVrm();
    const expr = makeExpression({ blink: undefined });
    applyLearnedExpression(vrm as never, expr);
    const blinkCalls = vrm.expressionManager.setValue.mock.calls.filter(
      (c: [string, number]) => c[0] === 'blink',
    );
    expect(blinkCalls.length).toBe(0);
  });
});

describe('clearExpressionPreview', () => {
  it('resets all expression channels to zero', () => {
    const vrm = makeMockVrm();
    clearExpressionPreview(vrm as never);
    const calls = vrm.expressionManager.setValue.mock.calls;
    expect(calls.length).toBe(12); // 11 emotions/visemes + blink
    for (const [, weight] of calls) {
      expect(weight).toBe(0);
    }
  });

  it('handles missing expressionManager gracefully', () => {
    const vrm = { expressionManager: null };
    expect(() => clearExpressionPreview(vrm as never)).not.toThrow();
  });
});
