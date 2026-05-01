import { describe, it, expect, vi } from 'vitest';
import {
  applyExpandedBlendshapes,
  clearExpandedBlendshapes,
  ARKIT_BLENDSHAPE_NAMES,
} from './expanded-blendshapes';

interface MockExpressionManager {
  setValue: ReturnType<typeof vi.fn>;
  getExpression: ReturnType<typeof vi.fn>;
}

function makeVrm(shippedNames: string[]): { expressionManager: MockExpressionManager } {
  const set = new Set(shippedNames);
  return {
    expressionManager: {
      setValue: vi.fn(),
      getExpression: vi.fn((name: string) => (set.has(name) ? { name } : null)),
    },
  };
}

describe('applyExpandedBlendshapes', () => {
  it('writes only channels the rig ships', () => {
    const vrm = makeVrm(['mouthSmileLeft', 'browInnerUp']);
    const scores = new Map<string, number>([
      ['mouthSmileLeft', 0.8],
      ['browInnerUp', 0.5],
      ['cheekPuff', 0.9], // not on rig — must be skipped
    ]);
    applyExpandedBlendshapes(vrm as never, scores);
    expect(vrm.expressionManager.setValue).toHaveBeenCalledWith('mouthSmileLeft', 0.8);
    expect(vrm.expressionManager.setValue).toHaveBeenCalledWith('browInnerUp', 0.5);
    const calls = vrm.expressionManager.setValue.mock.calls.map((c) => c[0]);
    expect(calls).not.toContain('cheekPuff');
  });

  it('skips eyeBlink channels (handled by 6-preset baseline)', () => {
    const vrm = makeVrm(['eyeBlinkLeft', 'eyeBlinkRight', 'mouthSmileLeft']);
    const scores = new Map<string, number>([
      ['eyeBlinkLeft', 0.7],
      ['eyeBlinkRight', 0.7],
      ['mouthSmileLeft', 0.4],
    ]);
    applyExpandedBlendshapes(vrm as never, scores);
    const calls = vrm.expressionManager.setValue.mock.calls.map((c) => c[0]);
    expect(calls).not.toContain('eyeBlinkLeft');
    expect(calls).not.toContain('eyeBlinkRight');
    expect(calls).toContain('mouthSmileLeft');
  });

  it('clamps scores to [0, 1]', () => {
    const vrm = makeVrm(['cheekPuff']);
    const scores = new Map<string, number>([['cheekPuff', 1.6]]);
    applyExpandedBlendshapes(vrm as never, scores);
    expect(vrm.expressionManager.setValue).toHaveBeenCalledWith('cheekPuff', 1);
  });

  it('clamps negative scores to 0', () => {
    const vrm = makeVrm(['cheekPuff']);
    const scores = new Map<string, number>([['cheekPuff', -0.3]]);
    applyExpandedBlendshapes(vrm as never, scores);
    expect(vrm.expressionManager.setValue).toHaveBeenCalledWith('cheekPuff', 0);
  });

  it('is a no-op on rigs with no expressionManager', () => {
    const vrm = { expressionManager: null };
    expect(() => applyExpandedBlendshapes(vrm as never, new Map([['cheekPuff', 0.5]]))).not.toThrow();
  });

  it('is a no-op on stock 6-preset rigs (no ARKit channels)', () => {
    const vrm = makeVrm([]); // rig ships nothing matching ARKit names
    const scores = new Map<string, number>([
      ['mouthSmileLeft', 0.8],
      ['cheekPuff', 0.5],
    ]);
    applyExpandedBlendshapes(vrm as never, scores);
    expect(vrm.expressionManager.setValue).not.toHaveBeenCalled();
  });

  it('skips entries missing from the scores map', () => {
    const vrm = makeVrm(['cheekPuff', 'mouthSmileLeft']);
    const scores = new Map<string, number>([['cheekPuff', 0.5]]);
    applyExpandedBlendshapes(vrm as never, scores);
    expect(vrm.expressionManager.setValue).toHaveBeenCalledTimes(1);
    expect(vrm.expressionManager.setValue).toHaveBeenCalledWith('cheekPuff', 0.5);
  });
});

describe('clearExpandedBlendshapes', () => {
  it('zeroes every shipped ARKit channel', () => {
    const vrm = makeVrm(['cheekPuff', 'mouthSmileLeft']);
    clearExpandedBlendshapes(vrm as never);
    const calls = vrm.expressionManager.setValue.mock.calls;
    // Two shipped channels → two zero writes
    expect(calls.length).toBe(2);
    for (const [, weight] of calls) expect(weight).toBe(0);
  });

  it('skips eyeBlink channels (owned by baseline)', () => {
    const vrm = makeVrm(['eyeBlinkLeft', 'eyeBlinkRight']);
    clearExpandedBlendshapes(vrm as never);
    expect(vrm.expressionManager.setValue).not.toHaveBeenCalled();
  });

  it('is a no-op without expressionManager', () => {
    const vrm = { expressionManager: null };
    expect(() => clearExpandedBlendshapes(vrm as never)).not.toThrow();
  });
});

describe('ARKIT_BLENDSHAPE_NAMES catalogue', () => {
  it('lists exactly 52 ARKit shapes', () => {
    expect(ARKIT_BLENDSHAPE_NAMES.length).toBe(52);
  });

  it('contains the canonical baseline-overlap blink shapes', () => {
    expect(ARKIT_BLENDSHAPE_NAMES).toContain('eyeBlinkLeft');
    expect(ARKIT_BLENDSHAPE_NAMES).toContain('eyeBlinkRight');
  });

  it('has no duplicates', () => {
    const set = new Set<string>(ARKIT_BLENDSHAPE_NAMES);
    expect(set.size).toBe(ARKIT_BLENDSHAPE_NAMES.length);
  });
});
