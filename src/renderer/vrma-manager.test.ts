import { describe, expect, it } from 'vitest';
import { getIdleAnimationForGender, getStandingAnimationForMood, SITTING_ANIMATION_PATHS } from './vrma-manager';

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

  it('still returns ladylike for female when excludeSitting is true (ladylike is a standing animation)', () => {
    // ladylike.vrma is NOT a sitting animation — excludeSitting must not filter it out
    const entry = getIdleAnimationForGender('female', () => 0.01, true);
    expect(entry?.motionKey).toBe('ladylike');
    expect(entry && SITTING_ANIMATION_PATHS.has(entry.path)).toBe(false);
  });
});

describe('vrma-manager getStandingAnimationForMood', () => {
  it('returns a non-sitting animation for moods that have one', () => {
    const angry = getStandingAnimationForMood('angry');
    expect(angry).toBeDefined();
    expect(angry && SITTING_ANIMATION_PATHS.has(angry.path)).toBe(false);
  });

  it('skips sitting variants like relax for the relaxed mood', () => {
    const relaxed = getStandingAnimationForMood('relaxed');
    if (relaxed) {
      expect(SITTING_ANIMATION_PATHS.has(relaxed.path)).toBe(false);
    }
  });
});
