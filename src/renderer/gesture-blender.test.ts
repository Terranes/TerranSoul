import { describe, it, expect } from 'vitest';
import { GestureBlender, STATE_BLEND_CONFIGS } from './gesture-blender';

describe('GestureBlender', () => {
  it('computeOffset returns a 3-element array', () => {
    const blender = new GestureBlender();
    const offset = blender.computeOffset('head', 'idle', 1.0);
    expect(offset).toHaveLength(3);
    expect(typeof offset[0]).toBe('number');
    expect(typeof offset[1]).toBe('number');
    expect(typeof offset[2]).toBe('number');
  });

  it('computeOffset values change over time', () => {
    const blender = new GestureBlender();
    const a = blender.computeOffset('head', 'idle', 0);
    const b = blender.computeOffset('head', 'idle', 1.5);
    const changed = a[0] !== b[0] || a[1] !== b[1] || a[2] !== b[2];
    expect(changed).toBe(true);
  });

  it('different bones produce different offsets (phase independence)', () => {
    const blender = new GestureBlender();
    const head = blender.computeOffset('head', 'idle', 1.0);
    const spine = blender.computeOffset('spine', 'idle', 1.0);
    const same = head[0] === spine[0] && head[1] === spine[1] && head[2] === spine[2];
    expect(same).toBe(false);
  });

  it('happy state produces larger offsets than sad state on average', () => {
    const blender = new GestureBlender();
    let happyMag = 0;
    let sadMag = 0;
    const samples = 50;
    for (let i = 0; i < samples; i++) {
      const t = i * 0.1;
      const h = blender.computeOffset('head', 'happy', t);
      const s = blender.computeOffset('head', 'sad', t);
      happyMag += Math.abs(h[0]) + Math.abs(h[1]) + Math.abs(h[2]);
      sadMag += Math.abs(s[0]) + Math.abs(s[1]) + Math.abs(s[2]);
    }
    expect(happyMag).toBeGreaterThan(sadMag);
  });

  it('transitionTo starts cross-fade and alpha goes 0→1', () => {
    const blender = new GestureBlender();
    blender.transitionTo('happy', 0);
    const alphaStart = blender.getTransitionAlpha(0);
    expect(alphaStart).toBeCloseTo(0, 1);
    const alphaMid = blender.getTransitionAlpha(0.2);
    expect(alphaMid).toBeGreaterThan(0);
    expect(alphaMid).toBeLessThan(1);
  });

  it('after cross-fade duration alpha is 1.0', () => {
    const blender = new GestureBlender();
    blender.transitionTo('happy', 0);
    const config = STATE_BLEND_CONFIGS['happy'];
    const alpha = blender.getTransitionAlpha(config.crossFadeDuration + 0.01);
    expect(alpha).toBe(1);
  });

  it('getPreviousState returns null initially, then previous state after transition', () => {
    const blender = new GestureBlender();
    expect(blender.getPreviousState()).toBeNull();
    blender.transitionTo('happy', 0);
    expect(blender.getPreviousState()).toBe('idle');
    blender.transitionTo('sad', 1);
    expect(blender.getPreviousState()).toBe('happy');
  });

  it('noise function produces bounded output', () => {
    const blender = new GestureBlender();
    for (let t = 0; t < 100; t += 0.1) {
      const offset = blender.computeOffset('head', 'happy', t);
      for (const v of offset) {
        expect(Math.abs(v)).toBeLessThan(0.1);
      }
    }
  });

  it('custom config overrides work', () => {
    const custom = new GestureBlender({ idle: { amplitude: 0.5 } });
    const standard = new GestureBlender();
    const c = custom.computeOffset('head', 'idle', 1.0);
    const s = standard.computeOffset('head', 'idle', 1.0);
    const cMag = Math.abs(c[0]) + Math.abs(c[1]) + Math.abs(c[2]);
    const sMag = Math.abs(s[0]) + Math.abs(s[1]) + Math.abs(s[2]);
    expect(cMag).toBeGreaterThan(sMag);
  });

  it('transitionTo with same state is a no-op', () => {
    const blender = new GestureBlender();
    blender.transitionTo('idle', 0);
    expect(blender.getPreviousState()).toBeNull();
    expect(blender.getTransitionAlpha(0)).toBe(1);
  });

  it('bone weight overrides scale offsets', () => {
    const half = new GestureBlender({ idle: { boneWeights: { head: 0.5 } } });
    const full = new GestureBlender();
    const h = half.computeOffset('head', 'idle', 2.0);
    const f = full.computeOffset('head', 'idle', 2.0);
    expect(Math.abs(h[0])).toBeCloseTo(Math.abs(f[0]) * 0.5, 6);
    expect(Math.abs(h[1])).toBeCloseTo(Math.abs(f[1]) * 0.5, 6);
    expect(Math.abs(h[2])).toBeCloseTo(Math.abs(f[2]) * 0.5, 6);
  });

  it('all character states have blend configs', () => {
    const states = ['idle', 'thinking', 'talking', 'happy', 'sad', 'angry', 'relaxed', 'surprised'] as const;
    for (const state of states) {
      const config = STATE_BLEND_CONFIGS[state];
      expect(config).toBeDefined();
      expect(config.amplitude).toBeGreaterThan(0);
      expect(config.frequency).toBeGreaterThan(0);
      expect(config.crossFadeDuration).toBeGreaterThan(0);
    }
  });

  it('multiple sequential transitions maintain valid alpha', () => {
    const blender = new GestureBlender();
    blender.transitionTo('happy', 0);
    blender.transitionTo('angry', 0.1);
    blender.transitionTo('sad', 0.15);
    const alpha = blender.getTransitionAlpha(0.15);
    expect(alpha).toBeGreaterThanOrEqual(0);
    expect(alpha).toBeLessThanOrEqual(1);
    expect(blender.getPreviousState()).toBe('angry');
  });
});
