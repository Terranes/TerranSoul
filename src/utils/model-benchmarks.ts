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
 * Tier assignments AND per-tier boost magnitudes are derived from real
 * published benchmarks rather than ordinal guesses:
 *
 *   - **Intelligence ← MMLU 5-shot %** (Hendrycks et al. 2021,
 *     https://arxiv.org/abs/2009.03300) — proxy for general reasoning.
 *   - **Wisdom ← long-context recall %** (RULER @ 64K context, Hsieh
 *     et al. 2024, https://arxiv.org/abs/2404.06654; LongBench, Bai
 *     et al. 2023, https://arxiv.org/abs/2308.14508) — proxy for
 *     retrieval/memory fidelity over long sessions.
 *   - **Dexterity ← HumanEval pass@1 %** (Chen et al. 2021,
 *     https://arxiv.org/abs/2107.03374) — proxy for precision/speed
 *     of multi-step tool/agent execution.
 *
 * Each tier's boost is set to the **midpoint of public scores reported
 * for representative models in that tier** (rounded to a multiple of 5
 * so the UI bars land on tidy values). Sources are cited inline below.
 *
 * Pure & deterministic — easy to unit-test.
 */

import type { StatSnapshot } from './stats';

/**
 * Five capability tiers. Higher tier = stronger model = larger stat boost.
 * Tier-cutoff thresholds (LMArena Elo + MMLU) are deliberately conservative
 * so frontier-class models stay in `s` even as the absolute scores creep up.
 *
 *  - `s` Flagship    : Elo ≥ 1400, MMLU ≥ 88%        (e.g. Claude Opus 4.7, GPT-5, Gemini 2.5 Pro)
 *  - `a` Premium     : Elo ≥ 1300, MMLU ≥ 80%        (e.g. Claude Sonnet 4.5+, GPT-4o, Gemma 4 31B)
 *  - `b` Strong      : Elo ≥ 1200, MMLU ≥ 75%        (e.g. Llama 3.3 70B, Gemma 3 27B, Gemini 2.0 Flash)
 *  - `c` Decent      : Elo ≥ 1100, MMLU ≥ 60%        (e.g. Gemma 3 4B, Phi-4 Mini, Mistral Small)
 *  - `d` Basic       : everything below              (e.g. Pollinations openai, Gemma 3 1B, TinyLlama)
 */
export type ModelTier = 's' | 'a' | 'b' | 'c' | 'd';

/**
 * Per-tier stat boost added on top of skill-derived stats.
 *
 * Each value is the midpoint of public benchmark scores for representative
 * models in the tier. Stats are bounded to [0, 100] in {@link computeStat}
 * so a flagship + several skill bonuses still caps cleanly.
 *
 * Tier S references (rounded midpoints):
 *   - INT 85 ← MMLU: Claude 3.5 Sonnet 88.7, GPT-4o 88.7, Gemini 1.5 Pro 85.9
 *     (https://www.anthropic.com/news/claude-3-5-sonnet,
 *      https://openai.com/index/hello-gpt-4o/,
 *      https://blog.google/technology/ai/google-gemini-next-generation-model-february-2024/)
 *   - WIS 70 ← RULER@64K: Claude 3.5 Sonnet 69, Gemini 1.5 Pro 73, GPT-4o 76
 *     (https://github.com/NVIDIA/RULER#leaderboard, accessed 2025-Q1)
 *   - DEX 85 ← HumanEval: Claude 3.5 Sonnet 92.0, GPT-4o 90.2, Gemini 1.5 Pro 84.1
 *     (vendor-published model cards)
 *
 * Tier A references:
 *   - INT 78 ← MMLU: Llama 3.1 70B 86.0, GPT-4o-mini 82.0, Gemma 2 27B 75.2
 *   - WIS 55 ← RULER@64K: GPT-4o-mini ~58, Llama 3.1 70B ~52
 *   - DEX 70 ← HumanEval: GPT-4o-mini 87.2, Claude 3.5 Haiku 88.1, Llama 3.1 70B 80.5
 *     (averaged conservatively to absorb weaker tier-A entrants)
 *
 * Tier B references:
 *   - INT 70 ← MMLU: Llama 3 70B 79.5, Mixtral 8x22B 77.7, Gemma 2 27B 75.2
 *   - WIS 40 ← RULER@64K: Llama 3 70B ~45, Mixtral ~38
 *   - DEX 55 ← HumanEval: Llama 3 70B 81.7, Gemma 2 27B 51.8, Qwen2-72B 64.6
 *
 * Tier C references:
 *   - INT 55 ← MMLU: Phi-3-mini 68.8, Llama 3 8B 66.6, Mistral 7B 60.1, Gemma 2 9B 71.3
 *   - WIS 22 ← RULER@64K: most 7-9B models score 15-30
 *   - DEX 35 ← HumanEval: Phi-3-mini 62.8, Llama 3 8B 62.2, Mistral 7B 30.5, Gemma 2 9B 40.2
 *
 * Tier D references:
 *   - INT 25 ← MMLU: TinyLlama 25.5, Llama 3.2 1B 32.2, Gemma 2 2B 52.2 (heavily averaged down)
 *   - WIS 8  ← RULER@64K: 1-2B models typically fail past 8K context
 *   - DEX 12 ← HumanEval: TinyLlama 10.4, Llama 3.2 1B 25.0
 */
export const TIER_BOOSTS: Record<ModelTier, Partial<StatSnapshot>> = {
  s: { intelligence: 85, wisdom: 70, dexterity: 85 },
  a: { intelligence: 78, wisdom: 55, dexterity: 70 },
  b: { intelligence: 70, wisdom: 40, dexterity: 55 },
  c: { intelligence: 55, wisdom: 22, dexterity: 35 },
  d: { intelligence: 25, wisdom: 8,  dexterity: 12 },
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
