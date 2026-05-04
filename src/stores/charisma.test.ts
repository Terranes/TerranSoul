import { describe, expect, it } from 'vitest';
import {
  avgRating,
  deriveMaturity,
  kindIcon,
  kindLabel,
  maturityColor,
  maturityLabel,
  type CharismaStat,
} from './charisma';

function stat(overrides: Partial<CharismaStat> = {}): CharismaStat {
  return {
    kind: 'expression',
    asset_id: 'lex_a',
    display_name: 'A',
    taught_at: 0,
    usage_count: 0,
    last_used_at: 0,
    rating_sum: 0,
    rating_count: 0,
    promoted_at: null,
    last_promotion_plan_id: null,
    ...overrides,
  };
}

describe('charisma helpers', () => {
  describe('deriveMaturity', () => {
    it('reports untested when no usage', () => {
      expect(deriveMaturity(stat())).toBe('untested');
    });

    it('reports learning after first use', () => {
      expect(deriveMaturity(stat({ usage_count: 3 }))).toBe('learning');
    });

    it('reports proven when usage >= 10 and avg >= 4', () => {
      expect(
        deriveMaturity(stat({ usage_count: 10, rating_sum: 8, rating_count: 2 })),
      ).toBe('proven');
    });

    it('does not promote to proven when rating below 4', () => {
      expect(
        deriveMaturity(stat({ usage_count: 50, rating_sum: 6, rating_count: 2 })),
      ).toBe('learning');
    });

    it('reports canon when promoted_at is set', () => {
      expect(
        deriveMaturity(stat({ usage_count: 0, promoted_at: 12345 })),
      ).toBe('canon');
    });
  });

  describe('avgRating', () => {
    it('returns 0 when no ratings', () => {
      expect(avgRating(stat())).toBe(0);
    });

    it('computes mean correctly', () => {
      expect(avgRating(stat({ rating_sum: 9, rating_count: 3 }))).toBe(3);
    });
  });

  describe('maturityLabel / maturityColor', () => {
    it('labels every tier', () => {
      expect(maturityLabel('untested')).toBe('Untested');
      expect(maturityLabel('learning')).toBe('Learning');
      expect(maturityLabel('proven')).toBe('Proven');
      expect(maturityLabel('canon')).toBe('Canon');
    });

    it('returns a CSS variable for each tier', () => {
      for (const m of ['untested', 'learning', 'proven', 'canon'] as const) {
        expect(maturityColor(m)).toMatch(/var\(--ts-/);
      }
    });
  });

  describe('kindIcon / kindLabel', () => {
    it('returns a non-empty icon and label per kind', () => {
      for (const k of ['trait', 'expression', 'motion'] as const) {
        expect(kindIcon(k).length).toBeGreaterThan(0);
        expect(kindLabel(k).length).toBeGreaterThan(0);
      }
    });
  });
});
