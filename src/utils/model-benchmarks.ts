/**
 * Model benchmark → stat boost mapping.
 *
 * The Brain Stat Sheet stops applying flat per-skill bonuses for the three
 * brain-tier skills (`free-brain`, `paid-brain`, `local-brain`). Instead, the
 * boost a brain contributes scales with the actual capability of the model
 * the user has selected, so a fresh install on Pollinations starts the
 * adventurer at level 1 and high-end frontier models like Claude Opus 4.7
 * provide a *much* larger increase than entry-level local models like
 * Gemma 3 1B.
 *
 * Tier assignments are derived from public benchmarks (LMArena Elo, MMLU,
 * GPQA, HumanEval) — see the JSDoc on each tier for the rough boundaries.
 * Numbers are intentionally ordinal rather than precise, so ranking is what
 * matters; a flagship will always boost more than a mid-tier model.
 *
 * Pure & deterministic — easy to unit-test.
 */

import type { StatSnapshot } from './stats';

/**
 * Five capability tiers. Higher tier = stronger model = larger stat boost.
 *
 *  - `s` Flagship    : Elo ≥ 1400, MMLU ≥ 88%        (e.g. Claude Opus 4.7, GPT-5, Gemini 2.5 Pro)
 *  - `a` Premium     : Elo ≥ 1300, MMLU ≥ 85%        (e.g. Claude Sonnet 4.5+, GPT-4o, Gemma 4 31B)
 *  - `b` Strong      : Elo ≥ 1200, MMLU ≥ 75%        (e.g. Llama 3.3 70B, Gemma 3 27B, Gemini 2.0 Flash)
 *  - `c` Decent      : Elo ≥ 1100, MMLU ≥ 60%        (e.g. Gemma 3 4B, Phi-4 Mini, Mistral Small)
 *  - `d` Basic       : everything below              (e.g. Pollinations openai, Gemma 3 1B, TinyLlama)
 */
export type ModelTier = 's' | 'a' | 'b' | 'c' | 'd';

/** Per-tier stat boost added on top of skill-derived stats. */
export const TIER_BOOSTS: Record<ModelTier, Partial<StatSnapshot>> = {
  s: { intelligence: 70, wisdom: 25, dexterity: 15 },
  a: { intelligence: 50, wisdom: 20, dexterity: 12 },
  b: { intelligence: 30, wisdom: 15, dexterity: 10 },
  c: { intelligence: 15, wisdom: 8,  dexterity: 5  },
  d: { intelligence: 5,  wisdom: 2,  dexterity: 2  },
};

/**
 * Substring → tier mapping. Matched against the lowercased model identifier
 * with the first match winning, so list more specific patterns first.
 *
 * Sources (April 2026):
 *   LMArena (lmsys/Chatbot Arena leaderboard)   https://lmarena.ai
 *   Artificial Analysis quality index           https://artificialanalysis.ai
 *   MMLU / GPQA / HumanEval published values
 */
const PATTERN_TIERS: ReadonlyArray<readonly [pattern: string, tier: ModelTier]> = [
  // ── Tier S — frontier flagships ────────────────────────────────────────
  ['claude-opus-4.7',       's'],
  ['claude-opus-4-7',       's'],
  ['claude-opus',           's'],
  ['gpt-5',                 's'],
  ['gemini-2.5-pro',        's'],
  ['o4-mini',               's'],
  ['o3',                    's'],
  ['glm-5',                 's'],

  // ── Tier A — premium ───────────────────────────────────────────────────
  ['claude-sonnet-4.6',     'a'],
  ['claude-sonnet-4.5',     'a'],
  ['claude-sonnet',         'a'],
  ['claude-3-5-sonnet',     'a'],
  ['claude-haiku-4',        'a'],
  ['gpt-4o',                'a'],
  ['gpt-4.1',               'a'],
  ['gemma4:31b',            'a'],
  ['gemma-4-31b',           'a'],
  ['gemini-1.5-pro',        'a'],
  ['mistral-large',         'a'],
  ['llama-3.3-405b',        'a'],

  // ── Tier B — strong ────────────────────────────────────────────────────
  ['llama-3.3-70b',         'b'],
  ['llama-3-70b',           'b'],
  ['meta/llama-3.3-70b',    'b'],
  ['meta-llama/llama-3.3',  'b'],
  ['gemma4:26b',            'b'],
  ['gemma4:e4b',            'b'],
  ['gemma-4-e4b',           'b'],
  ['gemma3:27b',            'b'],
  ['gemma-3-27b',           'b'],
  ['gemini-2.0-flash',      'b'],
  ['gpt-4.1-mini',          'b'],
  ['gpt-4o-mini',           'b'],
  ['mistral-medium',        'b'],
  ['qwen3-72b',             'b'],
  ['qwen-3-72b',            'b'],
  ['deepseek-v3',           'b'],
  ['deepseek-r1',           'b'],

  // ── Tier C — decent ────────────────────────────────────────────────────
  ['gemma3:4b',             'c'],
  ['gemma-3-4b',            'c'],
  ['gemma4:e2b',            'c'],
  ['gemma-4-e2b',           'c'],
  ['phi4-mini',             'c'],
  ['phi-4-mini',            'c'],
  ['phi-4',                 'c'],
  ['mistral-small',         'c'],
  ['qwen3-8b',              'c'],
  ['qwen-3-8b',             'c'],
  ['llama-3.2-3b',          'c'],
  ['llama-3-8b',            'c'],

  // ── Tier D — basic / fallback ──────────────────────────────────────────
  ['gemma3:1b',             'd'],
  ['gemma-3-1b',            'd'],
  ['gemma2:2b',             'd'],
  ['gemma-2-2b',            'd'],
  ['tinyllama',             'd'],
  ['llama-3.2-1b',          'd'],
  ['openai',                'd'],   // Pollinations' opaque "openai" alias
];

/** Default tier when the model identifier is not in the table. */
export const DEFAULT_MODEL_TIER: ModelTier = 'd';

/**
 * Pick the tier for the given model identifier.
 *
 * The match is case-insensitive substring on the lowercased model id, with
 * the first matching pattern winning. Returns {@link DEFAULT_MODEL_TIER} if
 * no pattern matches.
 */
export function getModelTier(modelId: string | null | undefined): ModelTier {
  if (!modelId) return DEFAULT_MODEL_TIER;
  const id = modelId.toLowerCase();
  for (const [pattern, tier] of PATTERN_TIERS) {
    if (id.includes(pattern)) return tier;
  }
  return DEFAULT_MODEL_TIER;
}

/** Look up the boost vector for a given tier. */
export function getTierBoost(tier: ModelTier): Partial<StatSnapshot> {
  return TIER_BOOSTS[tier];
}

/**
 * Convenience: compute the brain stat boost for an arbitrary model identifier.
 * Returns an empty object when no model is supplied (so callers can spread it
 * unconditionally).
 */
export function getModelBoost(modelId: string | null | undefined): Partial<StatSnapshot> {
  if (!modelId) return {};
  return TIER_BOOSTS[getModelTier(modelId)];
}
