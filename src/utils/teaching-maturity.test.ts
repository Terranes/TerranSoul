import { describe, expect, it } from 'vitest';
import {
  averageRating,
  deriveTeachingMaturity,
  formatRelativeTime,
  teachingMaturityColor,
  teachingMaturityLabel,
} from './teaching-maturity';

describe('teaching maturity helpers', () => {
  it('derives shared maturity tiers', () => {
    expect(deriveTeachingMaturity({ usage_count: 0, rating_sum: 0, rating_count: 0 })).toBe('untested');
    expect(deriveTeachingMaturity({ usage_count: 1, rating_sum: 0, rating_count: 0 })).toBe('learning');
    expect(deriveTeachingMaturity({ usage_count: 10, rating_sum: 8, rating_count: 2 })).toBe('proven');
    expect(deriveTeachingMaturity({ usage_count: 50, rating_sum: 6, rating_count: 2 })).toBe('learning');
    expect(deriveTeachingMaturity({ usage_count: 0, rating_sum: 0, rating_count: 0, promoted_at: 1 })).toBe('canon');
  });

  it('treats disabled capability-style records as untested', () => {
    expect(
      deriveTeachingMaturity({
        enabled: false,
        usage_count: 50,
        rating_sum: 50,
        rating_count: 10,
      }),
    ).toBe('untested');
  });

  it('computes rating, labels, colors, and relative time', () => {
    expect(averageRating({ rating_sum: 0, rating_count: 0 })).toBe(0);
    expect(averageRating({ rating_sum: 9, rating_count: 3 })).toBe(3);
    expect(teachingMaturityLabel('canon')).toBe('Canon');
    expect(teachingMaturityColor('proven')).toBe('var(--ts-success)');
    expect(formatRelativeTime(1_000, 62_000)).toBe('1m ago');
    expect(formatRelativeTime(1_000, 1_000)).toBe('0s ago');
  });
});