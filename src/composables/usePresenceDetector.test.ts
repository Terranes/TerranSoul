/**
 * Tests for the usePresenceDetector composable.
 *
 * Uses fake timers so tests run synchronously without real delays.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  classifyAwayDuration,
  getGreetingForDuration,
  usePresenceDetector,
  SHORT_GREETINGS,
  MEDIUM_GREETINGS,
  LONG_GREETINGS,
  EXTENDED_GREETINGS,
  SHORT_AWAY_THRESHOLD,
  MEDIUM_AWAY_THRESHOLD,
  LONG_AWAY_THRESHOLD,
  AWAY_TIMEOUT,
} from './usePresenceDetector';
import type { AwayDuration, PresenceEvent } from './usePresenceDetector';

// Stub Vue lifecycle hooks so the composable can be used outside a component.
vi.mock('vue', async () => {
  const actual = await vi.importActual<typeof import('vue')>('vue');
  return { ...actual, onMounted: vi.fn((cb: () => void) => cb()), onUnmounted: vi.fn() };
});

describe('usePresenceDetector', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // ── Pure function tests ───────────────────────────────────────────────

  describe('classifyAwayDuration', () => {
    it('returns "short" for durations below SHORT_AWAY_THRESHOLD', () => {
      expect(classifyAwayDuration(0)).toBe('short');
      expect(classifyAwayDuration(SHORT_AWAY_THRESHOLD - 1)).toBe('short');
    });

    it('returns "medium" for durations at SHORT and below MEDIUM threshold', () => {
      expect(classifyAwayDuration(SHORT_AWAY_THRESHOLD)).toBe('medium');
      expect(classifyAwayDuration(MEDIUM_AWAY_THRESHOLD - 1)).toBe('medium');
    });

    it('returns "long" for durations at MEDIUM and below LONG threshold', () => {
      expect(classifyAwayDuration(MEDIUM_AWAY_THRESHOLD)).toBe('long');
      expect(classifyAwayDuration(LONG_AWAY_THRESHOLD - 1)).toBe('long');
    });

    it('returns "extended" for durations at or above LONG threshold', () => {
      expect(classifyAwayDuration(LONG_AWAY_THRESHOLD)).toBe('extended');
      expect(classifyAwayDuration(LONG_AWAY_THRESHOLD + 1_000_000)).toBe('extended');
    });
  });

  describe('getGreetingForDuration', () => {
    const cases: AwayDuration[] = ['short', 'medium', 'long', 'extended'];

    it.each(cases)('returns a string for duration "%s"', (dur) => {
      const greeting = getGreetingForDuration(dur);
      expect(typeof greeting).toBe('string');
      expect(greeting.length).toBeGreaterThan(0);
    });

    it('short greetings differ from long greetings', () => {
      expect(SHORT_GREETINGS).not.toEqual(LONG_GREETINGS);
    });
  });

  describe('greeting pools are non-empty', () => {
    it('all pools contain at least one greeting', () => {
      expect(SHORT_GREETINGS.length).toBeGreaterThan(0);
      expect(MEDIUM_GREETINGS.length).toBeGreaterThan(0);
      expect(LONG_GREETINGS.length).toBeGreaterThan(0);
      expect(EXTENDED_GREETINGS.length).toBeGreaterThan(0);
    });
  });

  // ── Composable reactive tests ─────────────────────────────────────────

  it('initial state is "present"', () => {
    const { presenceState, stop } = usePresenceDetector({ awayTimeout: AWAY_TIMEOUT });
    expect(presenceState.value).toBe('present');
    stop();
  });

  it('activity events reset lastActivityAt', () => {
    const { lastActivityAt, stop } = usePresenceDetector({ awayTimeout: 5000 });
    const initial = lastActivityAt.value;

    // Advance time so the throttle window passes, then fire an event.
    vi.advanceTimersByTime(2000);
    document.dispatchEvent(new Event('mousemove'));

    expect(lastActivityAt.value).toBeGreaterThan(initial);
    stop();
  });

  it('state becomes "away" after awayTimeout with no activity', () => {
    const { presenceState, stop } = usePresenceDetector({ awayTimeout: 5000 });
    expect(presenceState.value).toBe('present');

    // Advance past the timeout + one check interval tick.
    vi.advanceTimersByTime(6000);
    expect(presenceState.value).toBe('away');
    stop();
  });

  it('state becomes "away" on visibility hidden', () => {
    const { presenceState, stop } = usePresenceDetector({ awayTimeout: 60000 });

    Object.defineProperty(document, 'hidden', { value: true, configurable: true });
    document.dispatchEvent(new Event('visibilitychange'));

    expect(presenceState.value).toBe('away');

    // Restore
    Object.defineProperty(document, 'hidden', { value: false, configurable: true });
    stop();
  });

  it('state becomes "present" and awayDurationMs is set on return', () => {
    const { presenceState, awayDurationMs, stop } = usePresenceDetector({ awayTimeout: 5000 });

    // Go away
    vi.advanceTimersByTime(6000);
    expect(presenceState.value).toBe('away');

    // Come back — advance a bit then fire activity
    vi.advanceTimersByTime(3000);
    document.dispatchEvent(new Event('keydown'));

    expect(presenceState.value).toBe('present');
    expect(awayDurationMs.value).toBeGreaterThan(0);
    stop();
  });

  it('onPresenceChange callback fires with correct PresenceEvent on away', () => {
    const cb = vi.fn();
    const { onPresenceChange, stop } = usePresenceDetector({ awayTimeout: 3000 });
    onPresenceChange(cb);

    vi.advanceTimersByTime(4000);
    expect(cb).toHaveBeenCalledWith(
      expect.objectContaining({ state: 'away', awayMs: 0 }),
    );
    stop();
  });

  it('onPresenceChange callback fires with "returned" state when user comes back', () => {
    const cb = vi.fn();
    const { onPresenceChange, stop } = usePresenceDetector({ awayTimeout: 3000 });
    onPresenceChange(cb);

    // Go away
    vi.advanceTimersByTime(4000);
    // Come back
    vi.advanceTimersByTime(2000);
    document.dispatchEvent(new Event('click'));

    const returnedCall = cb.mock.calls.find(
      (c: PresenceEvent[]) => c[0].state === 'returned',
    );
    expect(returnedCall).toBeDefined();
    expect((returnedCall as PresenceEvent[])[0].awayDuration).not.toBeNull();
    expect((returnedCall as PresenceEvent[])[0].awayMs).toBeGreaterThan(0);
    stop();
  });

  it('cleanup removes event listeners on stop', () => {
    const removeSpy = vi.spyOn(document, 'removeEventListener');
    const { stop } = usePresenceDetector({ awayTimeout: 5000 });

    stop();

    // Should have removed at least the 5 activity events + visibilitychange
    expect(removeSpy).toHaveBeenCalledWith('visibilitychange', expect.any(Function));
    expect(removeSpy).toHaveBeenCalledWith('mousemove', expect.any(Function));
    removeSpy.mockRestore();
  });
});
