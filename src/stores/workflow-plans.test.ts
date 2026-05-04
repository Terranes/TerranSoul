import { describe, expect, it } from 'vitest';
import {
  agentRoleIcon,
  agentRoleLabel,
  formatRecurrence,
  isoDayKey,
  startOfWeek,
  weekdayShort,
} from './workflow-plans';

describe('workflow-plans helpers', () => {
  describe('startOfWeek', () => {
    it('returns Sunday 00:00 for any timestamp in the same week', () => {
      // 2026-05-04 is a Monday. startOfWeek should return Sunday 2026-05-03 00:00 local.
      const monday = new Date(2026, 4, 4, 14, 30).getTime();
      const start = startOfWeek(monday);
      const d = new Date(start);
      expect(d.getDay()).toBe(0); // Sunday
      expect(d.getHours()).toBe(0);
      expect(d.getMinutes()).toBe(0);
    });
  });

  describe('isoDayKey', () => {
    it('formats local date as YYYY-MM-DD', () => {
      const ts = new Date(2026, 4, 4, 12).getTime();
      expect(isoDayKey(ts)).toBe('2026-05-04');
    });

    it('zero-pads single-digit months and days', () => {
      const ts = new Date(2026, 0, 5, 12).getTime();
      expect(isoDayKey(ts)).toBe('2026-01-05');
    });
  });

  describe('formatRecurrence', () => {
    it('formats once', () => {
      expect(formatRecurrence({ kind: 'once' })).toBe('One time');
    });

    it('formats daily with interval 1', () => {
      expect(formatRecurrence({ kind: 'daily', interval: 1 })).toBe('Every day');
    });

    it('formats daily with interval > 1', () => {
      expect(formatRecurrence({ kind: 'daily', interval: 3 })).toBe('Every 3 days');
    });

    it('formats weekly with weekdays', () => {
      const result = formatRecurrence({
        kind: 'weekly',
        interval: 1,
        weekdays: ['monday', 'wednesday', 'friday'],
      });
      expect(result).toBe('Weekly on Mon, Wed, Fri');
    });

    it('formats every-other-week', () => {
      const result = formatRecurrence({
        kind: 'weekly',
        interval: 2,
        weekdays: ['tuesday'],
      });
      expect(result).toBe('Every 2 weeks on Tue');
    });

    it('formats monthly', () => {
      expect(formatRecurrence({ kind: 'monthly', interval: 1, day_of_month: 15 })).toBe(
        'Monthly on day 15',
      );
    });

    it('formats every-N-months', () => {
      expect(formatRecurrence({ kind: 'monthly', interval: 3, day_of_month: 1 })).toBe(
        'Every 3 months on day 1',
      );
    });
  });

  describe('weekdayShort', () => {
    it('returns 3-letter abbreviations', () => {
      expect(weekdayShort('monday')).toBe('Mon');
      expect(weekdayShort('sunday')).toBe('Sun');
      expect(weekdayShort('saturday')).toBe('Sat');
    });
  });

  describe('agentRoleLabel', () => {
    it('returns capitalised role names', () => {
      expect(agentRoleLabel('planner')).toBe('Planner');
      expect(agentRoleLabel('orchestrator')).toBe('Orchestrator');
    });
  });

  describe('agentRoleIcon', () => {
    it('returns an emoji for each role', () => {
      const roles = ['planner', 'coder', 'reviewer', 'tester', 'researcher', 'orchestrator'] as const;
      for (const role of roles) {
        expect(agentRoleIcon(role)).toBeTruthy();
        expect(agentRoleIcon(role).length).toBeGreaterThan(0);
      }
    });
  });
});
