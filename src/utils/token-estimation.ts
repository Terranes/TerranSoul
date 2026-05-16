/**
 * Lightweight token estimation for frontend hint gating.
 *
 * Uses the commonly-accepted English-language heuristic of ~4 characters
 * per token. This is sufficient for the Hermes suggest-hook threshold
 * (≥4000 tokens → ≥16000 chars) where precision isn't critical.
 */

/** Average characters per token for English/code text. */
const CHARS_PER_TOKEN = 4;

/** Estimate the token count of a text string. */
export function estimateTokens(text: string): number {
  if (!text) return 0;
  return Math.ceil(text.length / CHARS_PER_TOKEN);
}

/**
 * Estimate the total token usage of a turn (user message + assistant response).
 */
export function estimateTurnTokens(userMessage: string, assistantResponse: string): number {
  return estimateTokens(userMessage) + estimateTokens(assistantResponse);
}
