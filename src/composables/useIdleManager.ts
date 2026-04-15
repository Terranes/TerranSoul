/**
 * useIdleManager — time-based idle detection for the character.
 *
 * When the user hasn't interacted for IDLE_TIMEOUT_MS, the character picks a
 * random greeting from the IDLE_GREETINGS list and fires it through the
 * `onSpeak` callback (which the caller wires to `handleSend`). Greetings are
 * not repeated until all variants have been used (round-robin shuffle).
 *
 * The manager resets automatically when:
 *   - `resetIdle()` is called (user typed or sent a message)
 *   - `stop()` is called (component unmounted)
 *
 * Design notes:
 *  - Uses `setTimeout` (not `setInterval`) so each firing schedules the next;
 *    this avoids accumulated drift and allows easy stop/reset.
 *  - Greetings are shuffled before each round so consecutive sessions see
 *    different first greetings.
 *  - No greeting fires while the character is already thinking or talking
 *    (checked via `isBlocked` callback supplied by the caller).
 */

import { ref } from 'vue';

// ── Constants ─────────────────────────────────────────────────────────────────

/** Milliseconds of silence before the first idle greeting fires. */
export const IDLE_TIMEOUT_MS = 45_000;

/** Minimum gap between successive idle greetings (ms). */
export const IDLE_REPEAT_MS = 90_000;

/** Pool of idle greetings. Cycled in shuffled order before repeating. */
export const IDLE_GREETINGS = [
  'Are you still there?',
  "I'm here whenever you'd like to chat.",
  'Anything on your mind?',
  "It's been quiet. How are you doing?",
  'Feel free to ask me anything!',
];

// ── Options ───────────────────────────────────────────────────────────────────

export interface IdleManagerOptions {
  /** Called when idle timer fires — use to send a greeting message. */
  onSpeak: (text: string) => void;
  /** Return true when a greeting should be suppressed (e.g. already thinking/talking). */
  isBlocked?: () => boolean;
  /** Idle timeout override (ms). Defaults to IDLE_TIMEOUT_MS. */
  timeoutMs?: number;
  /** Repeat gap override (ms). Defaults to IDLE_REPEAT_MS. */
  repeatMs?: number;
}

// ── Composable ────────────────────────────────────────────────────────────────

export function useIdleManager(options: IdleManagerOptions) {
  const { onSpeak, isBlocked, timeoutMs = IDLE_TIMEOUT_MS, repeatMs = IDLE_REPEAT_MS } = options;

  const isIdle = ref(false);
  let timer: ReturnType<typeof setTimeout> | null = null;

  // Shuffled greeting queue — refilled when empty.
  let greetingQueue: string[] = [];

  function nextGreeting(): string {
    if (greetingQueue.length === 0) {
      greetingQueue = [...IDLE_GREETINGS].sort(() => Math.random() - 0.5);
    }
    return greetingQueue.shift()!;
  }

  function scheduleNext(delayMs: number) {
    timer = setTimeout(() => {
      isIdle.value = true;
      if (!isBlocked?.()) {
        onSpeak(nextGreeting());
      }
      // Schedule the repeat after the repeat gap.
      scheduleNext(repeatMs);
    }, delayMs);
  }

  /** Start the idle timer. Call this on component mount. */
  function start() {
    stop();
    scheduleNext(timeoutMs);
  }

  /** Reset the idle timer (user activity detected). */
  function resetIdle() {
    isIdle.value = false;
    stop();
    scheduleNext(timeoutMs);
  }

  /** Cancel all timers. Call on component unmount. */
  function stop() {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  }

  return { isIdle, start, resetIdle, stop };
}
