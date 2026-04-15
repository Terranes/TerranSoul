/**
 * Detects when the current LLM model is struggling with user requests.
 *
 * Tracks response quality signals and determines when to suggest
 * upgrading to a more capable model. Used by the conversation store
 * to trigger the in-chat upgrade dialog.
 */

/** Signals that indicate a response may be low quality. */
const INCAPABLE_PATTERNS: RegExp[] = [
  /i (?:can'?t|cannot|am not able to|don'?t have the ability)/i,
  /(?:beyond|outside|exceeds?) (?:my|the) (?:capabilities?|capacity|scope)/i,
  /i'?m (?:just|only) (?:a (?:language|text|chat)|an ai)/i,
  /(?:sorry|unfortunately),? i (?:can'?t|cannot|don'?t)/i,
  /(?:i'?m )?not (?:sure|certain) (?:how to|about|if i can)/i,
  /(?:please (?:try|ask|use) (?:a )?(?:more|better|different|another))/i,
  /(?:this (?:is|requires|needs) (?:a )?(?:more|complex|advanced))/i,
];

/** Minimum response length (chars) — very short responses may indicate failure. */
const MIN_USEFUL_LENGTH = 30;

/** Maximum ratio of "I can't" signals per recent window to consider struggling. */
const STRUGGLE_THRESHOLD = 2;

/** How many recent responses to consider. */
const WINDOW_SIZE = 5;

export interface CapacitySignal {
  /** Whether this specific response shows incapability signals. */
  isLowQuality: boolean;
  /** Whether the model is consistently struggling (upgrade recommended). */
  shouldSuggestUpgrade: boolean;
  /** Number of recent low-quality responses. */
  recentLowCount: number;
}

/** Tracks recent quality assessments. */
const recentQuality: boolean[] = [];

/**
 * Analyse an assistant response for capacity/quality signals.
 *
 * @param response  The assistant's response text.
 * @param userQuery The user's original query (for context).
 */
export function assessCapacity(response: string, userQuery: string): CapacitySignal {
  const isLowQuality = detectLowQuality(response, userQuery);

  recentQuality.push(isLowQuality);
  if (recentQuality.length > WINDOW_SIZE) {
    recentQuality.shift();
  }

  const recentLowCount = recentQuality.filter(Boolean).length;

  return {
    isLowQuality,
    shouldSuggestUpgrade: recentLowCount >= STRUGGLE_THRESHOLD,
    recentLowCount,
  };
}

/** Reset the quality tracking window (e.g. after an upgrade). */
export function resetCapacityTracking(): void {
  recentQuality.length = 0;
}

function detectLowQuality(response: string, _userQuery: string): boolean {
  // Very short responses often indicate the model punted
  if (response.trim().length < MIN_USEFUL_LENGTH) {
    return true;
  }

  // Check for explicit incapability patterns
  for (const pattern of INCAPABLE_PATTERNS) {
    if (pattern.test(response)) {
      return true;
    }
  }

  return false;
}
