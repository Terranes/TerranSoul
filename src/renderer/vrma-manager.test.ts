import { describe, expect, it } from 'vitest';
import { getIdleAnimationForGender } from './vrma-manager';

describe('vrma-manager idle selector', () => {
  it('returns ladylike for female when random is below threshold', () => {
    const entry = getIdleAnimationForGender('female', () => 0.1);
    expect(entry?.motionKey).toBe('ladylike');
    expect(entry?.loop).toBe(true);
  });

  it('returns standard idle for female when random is above threshold', () => {
    const entry = getIdleAnimationForGender('female', () => 0.95);
    expect(entry?.motionKey).toBe('idle');
    expect(entry?.loop).toBe(true);
  });

  it('always returns standard idle for male', () => {
    const low = getIdleAnimationForGender('male', () => 0.01);
    const high = getIdleAnimationForGender('male', () => 0.99);
    expect(low?.motionKey).toBe('idle');
    expect(high?.motionKey).toBe('idle');
  });
});
