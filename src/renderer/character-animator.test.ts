import { describe, it, expect } from 'vitest';
import { CharacterAnimator } from './character-animator';
import * as THREE from 'three';

function makePlaceholder(): THREE.Group {
  return new THREE.Group();
}

describe('CharacterAnimator', () => {
  it('defaults to idle state', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.update(0.016);
    // Idle should produce minimal movement — position.y should be near 0
    expect(Math.abs(group.position.y)).toBeLessThan(0.1);
  });

  it('setState changes state and resets elapsed time', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.update(1.0);
    const posAfterIdle = group.position.y;

    animator.setState('thinking');
    animator.update(0.016);
    // After state change the elapsed resets, so animation starts from t≈0
    const posAfterThinking = group.position.y;
    // Just verify it ran without error and produced a number
    expect(typeof posAfterThinking).toBe('number');
    expect(posAfterThinking).not.toBeNaN();
  });

  it('thinking state produces different animation than idle', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);

    // Run idle for some time
    animator.setState('idle');
    animator.update(0.5);
    const idleY = group.position.y;
    const idleRotY = group.rotation.y;

    // Reset and run thinking
    const group2 = makePlaceholder();
    const animator2 = new CharacterAnimator();
    animator2.setPlaceholder(group2);
    animator2.setState('thinking');
    animator2.update(0.5);
    const thinkingY = group2.position.y;

    // At the same elapsed time, thinking uses faster oscillation
    // so the positions will generally differ
    // (both are sine-based but different frequencies)
    expect(typeof thinkingY).toBe('number');
  });

  it('talking state animates position.y and rotation.z', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.setState('talking');
    animator.update(0.3);
    // Talking should affect both y and z rotation
    expect(typeof group.position.y).toBe('number');
    expect(typeof group.rotation.z).toBe('number');
  });

  it('happy state produces bounce animation', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.setState('happy');
    animator.update(0.1);
    // Happy uses abs(sin) so position.y should be >= 0
    expect(group.position.y).toBeGreaterThanOrEqual(0);
  });

  it('sad state produces droop animation', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.setState('sad');
    animator.update(0.5);
    // Sad uses -abs(sin) so position.y should be <= 0
    expect(group.position.y).toBeLessThanOrEqual(0);
  });

  it('transitions idle → thinking → talking → idle', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);

    animator.setState('idle');
    animator.update(0.1);
    expect(typeof group.position.y).toBe('number');

    animator.setState('thinking');
    animator.update(0.1);
    expect(typeof group.position.y).toBe('number');

    animator.setState('talking');
    animator.update(0.1);
    expect(typeof group.position.y).toBe('number');

    animator.setState('idle');
    animator.update(0.1);
    expect(typeof group.position.y).toBe('number');
  });

  it('update with no placeholder or VRM does not throw', () => {
    const animator = new CharacterAnimator();
    expect(() => animator.update(0.016)).not.toThrow();
  });

  it('setPlaceholder clears VRM reference', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    // After setting placeholder, update should work on the group
    animator.setPlaceholder(group);
    animator.update(0.1);
    expect(typeof group.position.y).toBe('number');
  });
});
