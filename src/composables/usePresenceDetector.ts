/**
 * usePresenceDetector — browser-activity-based presence tracking.
 *
 * Monitors mouse, keyboard, touch, scroll, and page-visibility events to
 * determine whether the user is present, away, or just returned.  When the
 * user returns after being away the composable classifies the absence length
 * and picks a random greeting from a duration-appropriate pool.
 *
 * This is complementary to useIdleManager (which drives the *character's*
 * idle greetings after silence).  The presence detector reacts to the *user*
 * leaving and coming back.
 *
 * Design notes:
 *  - Uses `setInterval` (1 s tick) to compare `lastActivityAt` against
 *    the away-timeout; avoids a long single `setTimeout` that can drift
 *    when the tab is throttled.
 *  - `visibilitychange` triggers immediate away/returned transitions so
 *    switching tabs feels responsive.
 *  - Activity listeners are throttled via a timestamp check (1 s) to avoid
 *    flooding updates on every mousemove pixel.
 */

import { ref, onMounted, onUnmounted } from 'vue';

// ── Types ─────────────────────────────────────────────────────────────────────

export type PresenceState = 'present' | 'away' | 'returned';

export type AwayDuration = 'short' | 'medium' | 'long' | 'extended';

export interface PresenceEvent {
  state: PresenceState;
  awayDuration: AwayDuration | null;
  awayMs: number;
  timestamp: number;
}

// ── Thresholds ────────────────────────────────────────────────────────────────

/** Duration thresholds for different greeting responses (ms). */
export const SHORT_AWAY_THRESHOLD = 30_000;   // 30 s — quick away
export const MEDIUM_AWAY_THRESHOLD = 300_000;  // 5 min
export const LONG_AWAY_THRESHOLD = 1_800_000; // 30 min

/** How long with no activity before marking as away (ms). */
export const AWAY_TIMEOUT = 60_000; // 1 minute

/** Interval between activity-check ticks (ms). */
const CHECK_INTERVAL = 1_000;

/** Minimum gap between activity timestamp updates (ms) — throttle. */
const ACTIVITY_THROTTLE = 1_000;

// ── Greeting pools ────────────────────────────────────────────────────────────

export const SHORT_GREETINGS = [
  'Welcome back! 👋',
  "Hey, you're back!",
  'Oh hi again! ✨',
];

export const MEDIUM_GREETINGS = [
  'Hey! I was starting to miss you! 💙',
  'Welcome back! How was your break?',
  "You're back! I kept things running for you. ✨",
];

export const LONG_GREETINGS = [
  "Hey, it's been a while! How are you? 🌟",
  'Welcome back! I missed our chats! 💫',
  "Oh wow, you're back! I've been waiting for you! 😊",
];

export const EXTENDED_GREETINGS = [
  "It's so good to see you again! It's been quite a while! 🎉",
  'Welcome back after such a long time! I have so much to catch up on! 💖',
  "You're finally back! I was getting worried! 🌈",
];

// ── Pure helpers ──────────────────────────────────────────────────────────────

/** Classify an absence length into a named bucket. */
export function classifyAwayDuration(ms: number): AwayDuration {
  if (ms < SHORT_AWAY_THRESHOLD) return 'short';
  if (ms < MEDIUM_AWAY_THRESHOLD) return 'medium';
  if (ms < LONG_AWAY_THRESHOLD) return 'long';
  return 'extended';
}

/** Pick a random greeting from the pool matching the given duration. */
export function getGreetingForDuration(duration: AwayDuration): string {
  const pools: Record<AwayDuration, string[]> = {
    short: SHORT_GREETINGS,
    medium: MEDIUM_GREETINGS,
    long: LONG_GREETINGS,
    extended: EXTENDED_GREETINGS,
  };
  const pool = pools[duration];
  return pool[Math.floor(Math.random() * pool.length)];
}

// ── Composable ────────────────────────────────────────────────────────────────

export interface PresenceDetectorOptions {
  /** Override for the away-timeout (ms). Defaults to AWAY_TIMEOUT. */
  awayTimeout?: number;
}

export function usePresenceDetector(options?: PresenceDetectorOptions) {
  const awayTimeout = options?.awayTimeout ?? AWAY_TIMEOUT;

  const presenceState = ref<PresenceState>('present');
  const lastActivityAt = ref(Date.now());
  const awayDurationMs = ref(0);

  let wentAwayAt: number | null = null;
  let checkTimer: ReturnType<typeof setInterval> | null = null;
  let presenceCallback: ((event: PresenceEvent) => void) | null = null;

  // ── Internal helpers ────────────────────────────────────────────────────

  function emitEvent(state: PresenceState, awayMs: number) {
    presenceState.value = state;
    if (presenceCallback) {
      presenceCallback({
        state,
        awayDuration: awayMs > 0 ? classifyAwayDuration(awayMs) : null,
        awayMs,
        timestamp: Date.now(),
      });
    }
  }

  function markAway() {
    if (presenceState.value === 'away') return;
    wentAwayAt = Date.now();
    awayDurationMs.value = 0;
    emitEvent('away', 0);
  }

  function markReturned() {
    if (presenceState.value !== 'away') return;
    const now = Date.now();
    const elapsed = wentAwayAt !== null ? now - wentAwayAt : 0;
    awayDurationMs.value = elapsed;
    wentAwayAt = null;

    // Briefly transition through 'returned', then settle to 'present'.
    emitEvent('returned', elapsed);
    presenceState.value = 'present';
  }

  // ── Activity tracking (throttled) ──────────────────────────────────────

  function onActivity() {
    const now = Date.now();
    if (now - lastActivityAt.value < ACTIVITY_THROTTLE) return;
    lastActivityAt.value = now;

    if (presenceState.value === 'away') {
      markReturned();
    }
  }

  // ── Visibility change ──────────────────────────────────────────────────

  function onVisibilityChange() {
    if (document.hidden) {
      markAway();
    } else {
      lastActivityAt.value = Date.now();
      if (presenceState.value === 'away') {
        markReturned();
      }
    }
  }

  // ── Periodic check ─────────────────────────────────────────────────────

  function tick() {
    if (presenceState.value === 'away') return;
    if (Date.now() - lastActivityAt.value >= awayTimeout) {
      markAway();
    }
  }

  // ── Event list ─────────────────────────────────────────────────────────

  const ACTIVITY_EVENTS: (keyof DocumentEventMap)[] = [
    'mousemove',
    'keydown',
    'touchstart',
    'scroll',
    'click',
  ];

  // ── Lifecycle ──────────────────────────────────────────────────────────

  function start() {
    for (const evt of ACTIVITY_EVENTS) {
      document.addEventListener(evt, onActivity, { passive: true });
    }
    document.addEventListener('visibilitychange', onVisibilityChange);
    checkTimer = setInterval(tick, CHECK_INTERVAL);
  }

  function stop() {
    for (const evt of ACTIVITY_EVENTS) {
      document.removeEventListener(evt, onActivity);
    }
    document.removeEventListener('visibilitychange', onVisibilityChange);
    if (checkTimer !== null) {
      clearInterval(checkTimer);
      checkTimer = null;
    }
  }

  onMounted(start);
  onUnmounted(stop);

  // ── Public API ─────────────────────────────────────────────────────────

  function onPresenceChange(cb: (event: PresenceEvent) => void) {
    presenceCallback = cb;
  }

  return {
    presenceState,
    lastActivityAt,
    awayDurationMs,
    onPresenceChange,
    getGreetingForDuration,
    classifyAwayDuration,
    /** Exposed for testing — manually start listeners outside Vue lifecycle. */
    start,
    /** Exposed for testing — manually stop listeners outside Vue lifecycle. */
    stop,
  };
}
