/**
 * Hermes suggest-hook classifier.
 *
 * Determines whether a user message + response turn qualifies for a
 * dismissable one-line hint suggesting Hermes Desktop for heavyweight
 * workloads: deep research, multi-day workflows, or full-IDE coding.
 *
 * The classification is purely heuristic (regex/keyword based) and runs
 * in the frontend — no LLM call required. It only fires when all three
 * conditions are true:
 *   1. Turn token estimate ≥ threshold (default 4000)
 *   2. User message intent matches a heavyweight pattern
 *   3. settings.hermes_hint_enabled === true
 */

import { estimateTurnTokens } from './token-estimation';

/**
 * Default token threshold for the Hermes hint. Can be overridden by the
 * `TS_HERMES_HINT_TOKENS` environment variable at build time.
 */
export const HERMES_HINT_TOKEN_THRESHOLD: number =
  parseInt(import.meta.env.VITE_HERMES_HINT_TOKENS ?? '', 10) || 4000;

/** The three intent categories that qualify for the Hermes hint. */
export type HermesHintIntent = 'deep_research' | 'long_running_workflow' | 'full_ide_coding';

// ── Pattern sets for each intent ──────────────────────────────────────────────

const DEEP_RESEARCH_PATTERNS: RegExp[] = [
  /\b(deep|extensive|thorough|comprehensive|exhaustive)\s+(research|investigation|analysis|review|study)\b/i,
  /\bresearch\s+(across|over|dozens|hundreds)\b/i,
  /\b(survey|compare|audit)\s+(all|every|multiple|many|dozens)\b/i,
  /\b(literature\s+review|systematic\s+review|meta-?analysis)\b/i,
  /\b(scrape|crawl|index)\s+(the\s+web|multiple\s+sites|all\s+sources)\b/i,
  /\b(find\s+everything|gather\s+all\s+(info|information|data|sources))\b/i,
  /\binvestigat(e|ion)\s+(every|all|each)\b/i,
];

const LONG_RUNNING_WORKFLOW_PATTERNS: RegExp[] = [
  /\b(schedule|cron|overnight|multi[- ]?day|continuous|recurring)\s+(job|task|workflow|run|process)\b/i,
  /\b(keep\s+running|run\s+(overnight|continuously|indefinitely|in\s+the\s+background))\b/i,
  /\b(batch|pipeline|etl|data\s+pipeline)\b.*\b(process|run|execute)\b/i,
  /\b(monitor|watch|poll)\s+(for|until|continuously)\b/i,
  /\b(long[- ]?running|background)\s+(task|job|workflow|process)\b/i,
  /\b(automate|automation)\s+(daily|weekly|hourly|nightly)\b/i,
];

const FULL_IDE_CODING_PATTERNS: RegExp[] = [
  /\b(refactor|rewrite)\s+(the\s+entire|all|every|the\s+whole)\b/i,
  /\b(full|complete)\s+(codebase|project|repo|repository)\b.*\b(refactor|rewrite|migration|update)\b/i,
  /\b(edit|modify|change)\s+(dozens|hundreds|all)\s+(of\s+)?files\b/i,
  /\b(multi[- ]?file|cross[- ]?file|project[- ]?wide)\s+(edit|refactor|change)\b/i,
  /\b(terminal[- ]?first|cli[- ]?first|keyboard[- ]?driven)\s+(coding|editing|session)\b/i,
  /\b(heavy\s+edit[- ]?loop|coding\s+session|code\s+sprint)\b/i,
  /\b(implement|build)\s+(the\s+entire|a\s+full|the\s+whole)\s+(feature|system|module|app)\b/i,
];

/**
 * Classify whether a user message expresses a Hermes-hint-eligible intent.
 * Returns the matched intent or null if none match.
 */
export function classifyHermesIntent(userMessage: string): HermesHintIntent | null {
  if (!userMessage || userMessage.trim().length < 20) return null;

  for (const pattern of DEEP_RESEARCH_PATTERNS) {
    if (pattern.test(userMessage)) return 'deep_research';
  }
  for (const pattern of LONG_RUNNING_WORKFLOW_PATTERNS) {
    if (pattern.test(userMessage)) return 'long_running_workflow';
  }
  for (const pattern of FULL_IDE_CODING_PATTERNS) {
    if (pattern.test(userMessage)) return 'full_ide_coding';
  }

  return null;
}

export interface HermesHintCheck {
  /** Whether the hint should be shown. */
  show: boolean;
  /** The matched intent (null if show is false). */
  intent: HermesHintIntent | null;
  /** Estimated turn token count. */
  turnTokens: number;
}

/**
 * Check whether a completed turn should trigger the Hermes hint.
 *
 * @param userMessage - The user's original message
 * @param assistantResponse - The assistant's full response text
 * @param hermesHintEnabled - Whether the setting is enabled
 * @param hermesAlreadyConfigured - Whether Hermes MCP block is present
 * @param threshold - Token threshold (default: HERMES_HINT_TOKEN_THRESHOLD)
 */
export function shouldShowHermesHint(
  userMessage: string,
  assistantResponse: string,
  hermesHintEnabled: boolean,
  hermesAlreadyConfigured: boolean,
  threshold: number = HERMES_HINT_TOKEN_THRESHOLD,
): HermesHintCheck {
  const turnTokens = estimateTurnTokens(userMessage, assistantResponse);

  // Gate 1: setting must be enabled
  if (!hermesHintEnabled) {
    return { show: false, intent: null, turnTokens };
  }

  // Gate 2: don't show if Hermes is already configured
  if (hermesAlreadyConfigured) {
    return { show: false, intent: null, turnTokens };
  }

  // Gate 3: token threshold
  if (turnTokens < threshold) {
    return { show: false, intent: null, turnTokens };
  }

  // Gate 4: intent classification
  const intent = classifyHermesIntent(userMessage);
  if (!intent) {
    return { show: false, intent: null, turnTokens };
  }

  return { show: true, intent, turnTokens };
}
