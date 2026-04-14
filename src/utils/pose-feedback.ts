/**
 * Pose feedback serializer for autoregressive LLM context injection.
 *
 * Serializes the character's current pose state into a compact text descriptor
 * that can be prepended to the system prompt. This lets the LLM make coherent
 * animation decisions across conversation turns — avoiding repeated gestures,
 * transitioning poses smoothly, and maintaining narrative consistency.
 *
 * Example output:
 *   "Current character pose: thoughtful=0.75. Last gesture: nod (3.2s ago)."
 *
 * The descriptor is intentionally compact (< 200 chars) to minimize token usage.
 */

export interface PoseContextInput {
  /** Current blend weights from PoseBlender.getCurrentWeights(). */
  weights: Map<string, number>;
  /** Id of the last gesture played, or null. */
  lastGestureId: string | null;
  /** Seconds since the last gesture ended, or null if no gesture has played. */
  secondsSinceLastGesture: number | null;
}

/** Minimum weight threshold to include a preset in the context descriptor. */
const WEIGHT_THRESHOLD = 0.05;
/** Maximum number of presets to mention in the context string. */
const MAX_PRESETS_IN_CONTEXT = 3;

/**
 * Serialize the character's current pose state into a compact string suitable
 * for injection into the LLM system prompt.
 *
 * @param input  Current pose context data.
 * @returns      A short human-readable descriptor, or an empty string if
 *               neither pose nor gesture info is available.
 */
export function serializePoseContext(input: PoseContextInput): string {
  const parts: string[] = [];

  // ── Active pose blend ────────────────────────────────────────────────
  const activeWeights = [...input.weights.entries()]
    .filter(([, w]) => w >= WEIGHT_THRESHOLD)
    .sort((a, b) => b[1] - a[1])                // descending weight
    .slice(0, MAX_PRESETS_IN_CONTEXT)
    .map(([id, w]) => `${id}=${roundWeight(w)}`);

  if (activeWeights.length > 0) {
    parts.push(`Current character pose: ${activeWeights.join(', ')}.`);
  }

  // ── Last gesture ─────────────────────────────────────────────────────
  if (input.lastGestureId !== null) {
    if (input.secondsSinceLastGesture !== null && input.secondsSinceLastGesture >= 0) {
      const ago = Math.round(input.secondsSinceLastGesture * 10) / 10;
      parts.push(`Last gesture: ${input.lastGestureId} (${ago}s ago).`);
    } else {
      parts.push(`Last gesture: ${input.lastGestureId}.`);
    }
  }

  return parts.join(' ');
}

function roundWeight(w: number): number {
  return Math.round(w * 100) / 100;
}

/**
 * Build a full system prompt suffix from pose context.
 * Returns an empty string when there is nothing meaningful to report.
 */
export function buildPoseContextSuffix(input: PoseContextInput): string {
  const ctx = serializePoseContext(input);
  if (!ctx) return '';
  return `\n\n[Character state] ${ctx}`;
}
