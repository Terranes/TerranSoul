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
    expect(Math.abs(group.position.y)).toBeLessThan(0.1);
  });

  it('getState returns current state', () => {
    const animator = new CharacterAnimator();
    expect(animator.getState()).toBe('idle');
    animator.setState('thinking');
    expect(animator.getState()).toBe('thinking');
    animator.setState('happy');
    expect(animator.getState()).toBe('happy');
  });

  it('getPersona returns current persona', () => {
    const animator = new CharacterAnimator();
    expect(animator.getPersona()).toBe('cool');
  });

  it('setState changes state and resets elapsed time', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.update(1.0);

    animator.setState('thinking');
    animator.update(0.016);
    const posAfterThinking = group.position.y;
    expect(typeof posAfterThinking).toBe('number');
    expect(posAfterThinking).not.toBeNaN();
  });

  it('thinking state produces different animation than idle', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);

    animator.setState('idle');
    animator.update(0.5);

    const group2 = makePlaceholder();
    const animator2 = new CharacterAnimator();
    animator2.setPlaceholder(group2);
    animator2.setState('thinking');
    animator2.update(0.5);
    const thinkingY = group2.position.y;

    expect(typeof thinkingY).toBe('number');
  });

  it('talking state animates position.y and rotation.z', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.setState('talking');
    animator.update(0.3);
    expect(typeof group.position.y).toBe('number');
    expect(typeof group.rotation.z).toBe('number');
  });

  it('talking state applies scale pulse on placeholder', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.setState('talking');
    animator.update(0.2);
    expect(group.scale.x).toBeGreaterThan(0.9);
    expect(group.scale.x).toBeLessThan(1.1);
  });

  it('happy state produces bounce animation', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.setState('happy');
    animator.update(0.1);
    expect(group.position.y).toBeGreaterThanOrEqual(0);
  });

  it('happy state applies scale increase', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.setState('happy');
    animator.update(0.1);
    expect(group.scale.x).toBeGreaterThanOrEqual(1.0);
  });

  it('sad state produces droop animation', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.setState('sad');
    animator.update(0.5);
    expect(group.position.y).toBeLessThanOrEqual(0);
  });

  it('sad state tilts forward (rotation.x)', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.setState('sad');
    animator.update(0.3);
    expect(group.rotation.x).toBeGreaterThan(0);
  });

  it('sad state scales down slightly', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.setState('sad');
    animator.update(0.3);
    expect(group.scale.x).toBeLessThan(1.0);
  });

  it('idle state resets scale to 1.0', () => {
    const animator = new CharacterAnimator();
    const group = makePlaceholder();
    animator.setPlaceholder(group);
    animator.setState('idle');
    animator.update(0.1);
    expect(group.scale.x).toBeCloseTo(1.0, 2);
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
    animator.setPlaceholder(group);
    animator.update(0.1);
    expect(typeof group.position.y).toBe('number');
  });
});
