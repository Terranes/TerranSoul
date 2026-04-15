/**
 * Tests for the useIdleManager composable.
 *
 * Uses fake timers so tests run synchronously without real delays.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useIdleManager, IDLE_TIMEOUT_MS, IDLE_GREETINGS } from './useIdleManager';

describe('useIdleManager', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('isIdle is false initially', () => {
    const mgr = useIdleManager({ onSpeak: vi.fn() });
    expect(mgr.isIdle.value).toBe(false);
  });

  it('onSpeak is not called before timeout', () => {
    const onSpeak = vi.fn();
    const mgr = useIdleManager({ onSpeak, timeoutMs: IDLE_TIMEOUT_MS });
    mgr.start();
    vi.advanceTimersByTime(IDLE_TIMEOUT_MS - 1);
    expect(onSpeak).not.toHaveBeenCalled();
  });

  it('onSpeak is called after timeout and isIdle becomes true', () => {
    const onSpeak = vi.fn();
    const mgr = useIdleManager({ onSpeak, timeoutMs: 1000, repeatMs: 5000 });
    mgr.start();
    vi.advanceTimersByTime(1000);
    expect(onSpeak).toHaveBeenCalledTimes(1);
    expect(mgr.isIdle.value).toBe(true);
  });

  it('onSpeak is called with one of the IDLE_GREETINGS', () => {
    const onSpeak = vi.fn();
    const mgr = useIdleManager({ onSpeak, timeoutMs: 1000, repeatMs: 5000 });
    mgr.start();
    vi.advanceTimersByTime(1000);
    const spoken = onSpeak.mock.calls[0][0] as string;
    expect(IDLE_GREETINGS).toContain(spoken);
  });

  it('repeats after repeatMs', () => {
    const onSpeak = vi.fn();
    const mgr = useIdleManager({ onSpeak, timeoutMs: 1000, repeatMs: 2000 });
    mgr.start();
    vi.advanceTimersByTime(1000);  // first fire
    vi.advanceTimersByTime(2000);  // repeat
    expect(onSpeak).toHaveBeenCalledTimes(2);
  });

  it('resetIdle cancels pending timer and restarts the countdown', () => {
    const onSpeak = vi.fn();
    const mgr = useIdleManager({ onSpeak, timeoutMs: 1000, repeatMs: 5000 });
    mgr.start();
    vi.advanceTimersByTime(800);
    mgr.resetIdle();    // reset at 800ms — should not fire at 1000ms
    vi.advanceTimersByTime(300); // only 300ms since reset
    expect(onSpeak).not.toHaveBeenCalled();
    // Fire after full timeout from reset
    vi.advanceTimersByTime(700);
    expect(onSpeak).toHaveBeenCalledTimes(1);
  });

  it('resetIdle sets isIdle to false', () => {
    const onSpeak = vi.fn();
    const mgr = useIdleManager({ onSpeak, timeoutMs: 500, repeatMs: 5000 });
    mgr.start();
    vi.advanceTimersByTime(500);
    expect(mgr.isIdle.value).toBe(true);
    mgr.resetIdle();
    expect(mgr.isIdle.value).toBe(false);
  });

  it('stop prevents onSpeak from firing', () => {
    const onSpeak = vi.fn();
    const mgr = useIdleManager({ onSpeak, timeoutMs: 1000, repeatMs: 5000 });
    mgr.start();
    vi.advanceTimersByTime(500);
    mgr.stop();
    vi.advanceTimersByTime(1000);
    expect(onSpeak).not.toHaveBeenCalled();
  });

  it('suppresses greeting when isBlocked returns true', () => {
    const onSpeak = vi.fn();
    const mgr = useIdleManager({
      onSpeak,
      isBlocked: () => true,
      timeoutMs: 500,
      repeatMs: 5000,
    });
    mgr.start();
    vi.advanceTimersByTime(500);
    // isIdle should be true but onSpeak should not fire
    expect(mgr.isIdle.value).toBe(true);
    expect(onSpeak).not.toHaveBeenCalled();
  });

  it('all IDLE_GREETINGS appear before any repeats (round-robin)', () => {
    const onSpeak = vi.fn();
    const mgr = useIdleManager({ onSpeak, timeoutMs: 100, repeatMs: 100 });
    mgr.start();
    const n = IDLE_GREETINGS.length;
    // Advance through all greetings
    for (let i = 0; i < n; i++) {
      vi.advanceTimersByTime(100);
    }
    const spoken = onSpeak.mock.calls.map((c) => c[0] as string);
    // All n greetings should have appeared
    for (const g of IDLE_GREETINGS) {
      expect(spoken).toContain(g);
    }
  });
});
