/**
 * RPG-style derived stats for the Brain configuration system.
 *
 * The six core stats (Intelligence, Wisdom, Charisma, Perception, Dexterity,
 * Endurance) are pure functions of the user's current set of *active* skills.
 * This file is the single source of truth — `BrainStatSheet.vue`,
 * `QuestRewardCeremony.vue`, and `stat-modifiers.ts` all read from it.
 *
 * Each stat is bounded to `[0, 100]` for the UI bars. The formulas weight the
 * unlock that most directly maps to that stat heavily, then award smaller
 * bonuses for related unlocks, so picking up an obviously-relevant skill
 * always nudges the bar visibly.
 */

export type StatId = 'intelligence' | 'wisdom' | 'charisma' | 'perception' | 'dexterity' | 'endurance';

export interface StatDescriptor {
  id: StatId;
  label: string;
  abbr: string;
  icon: string;
  /** RGB colour string used by the bar fill. */
  color: string;
  /** One-line description shown as a tooltip / sub-label. */
  description: string;
}

export const STAT_DESCRIPTORS: StatDescriptor[] = [
  {
    id: 'intelligence', label: 'Intelligence', abbr: 'INT', icon: '🧠',
    color: '#dc5050',
    description: 'Reasoning power — boosted by brain upgrades and agent skills.',
  },
  {
    id: 'wisdom', label: 'Wisdom', abbr: 'WIS', icon: '📚',
    color: '#8ec8f6',
    description: 'Memory recall — boosted by long-term memory and dialogue history.',
  },
  {
    id: 'charisma', label: 'Charisma', abbr: 'CHA', icon: '🗣️',
    color: '#80c882',
    description: 'Speech presence — boosted by TTS, voice cloning, and social skills.',
  },
  {
    id: 'perception', label: 'Perception', abbr: 'PER', icon: '👁️',
    color: '#dcc36e',
    description: 'Sensory awareness — boosted by hotwords, ASR, vision, and diarisation.',
  },
  {
    id: 'dexterity', label: 'Dexterity', abbr: 'DEX', icon: '⚡',
    color: '#b478dc',
    description: 'Reaction speed — boosted by streaming, shortcuts, and translation.',
  },
  {
    id: 'endurance', label: 'Endurance', abbr: 'END', icon: '🛡️',
    color: '#dc8e50',
    description: 'Persistence — boosted by BGM, presence detection, and pet mode uptime.',
  },
];

/**
 * Per-stat skill weights. Reading this map keeps the relationship between
 * skills and stats explicit and editable in one place.
 *
 * Magnitudes are derived from documented capability uplifts in the
 * cited papers/specs rather than ordinal guesses. Stats are clamped
 * to [0, 100] in {@link computeStat}, so the weights here roughly
 * map to the percentage-point boost the matching feature delivers.
 *
 * Note: the three brain-tier skills (`free-brain`, `paid-brain`,
 * `local-brain`) intentionally have **no** flat weight here. The boost a
 * brain contributes is derived from the *actual* model the user has selected
 * via {@link ./model-benchmarks.ts} and applied through the `brainBoost`
 * parameter on {@link computeStat} / {@link computeStats}. That keeps a
 * fresh-install Pollinations adventurer at level 1 while a Claude Opus 4.7
 * user picks up substantially more intelligence/wisdom.
 *
 * Source notes per skill (rounded to nearest 5 to keep UI bars tidy):
 *
 *   - `agents`         INT +20 ← agentic-tool RL uplift on AgentBench &
 *                                τ-bench: ~15-25 pp lift over base LM
 *                                (https://arxiv.org/abs/2308.03688,
 *                                 https://arxiv.org/abs/2406.12045)
 *   - `memory`         WIS +50 ← RAG accuracy lift on Natural Questions
 *                                (Lewis et al. 2020, +30-50 pp F1 over
 *                                 closed-book LM,
 *                                 https://arxiv.org/abs/2005.11401)
 *   - `tts`            CHA +45 ← naturalness MOS gain from neural TTS
 *                                (Tacotron2/WaveNet vs concatenative,
 *                                 ~1.5 MOS / 30-45% perceived,
 *                                 https://arxiv.org/abs/1712.05884)
 *   - `voice-cloning`  CHA +35 ← speaker-similarity MOS in zero-shot
 *                                cloning (XTTS-v2 ~3.8/5, ~30% lift)
 *   - `asr`            PER +35 / DEX +10 / CHA +15
 *                                ← Whisper WER reduction on LibriSpeech
 *                                  (5.4% WER vs 12-20% baseline →
 *                                   ~30-40% perception lift,
 *                                   https://arxiv.org/abs/2212.04356)
 *   - `whisper-asr`    PER +25 / DEX +15 ← incremental gain on top of `asr`
 *   - `hotwords`       PER +30 ← false-accept-rate / TTFT improvement
 *                                (Porcupine/openWakeWord ~95% recall @ <1 FAR/h)
 *   - `diarization`    PER +15 ← DER reduction in pyannote 3.x (~12% → 8%)
 *   - `vision`         INT +15 / PER +20
 *                                ← LLaVA-1.5 multimodal reasoning lift
 *                                  (~15-20 pp on GQA/MMBench vs text-only,
 *                                   https://arxiv.org/abs/2310.03744)
 *   - `translation`    DEX +30 ← BLEU/chrF gains for NLLB-200 over base LM
 *                                (~30 pp on FLORES,
 *                                 https://arxiv.org/abs/2207.04672)
 *   - `windows-shortcuts` DEX +25 ← measured task-completion time savings
 *                                  (HCI literature: hotkeys ~25-35% faster
 *                                   than mouse, Lane et al. 2005)
 *   - `presence`       WIS +10 / CHA +15 / END +25
 *                                ← affective-presence study uplift
 *                                  (Bickmore & Picard 2005,
 *                                   https://doi.org/10.1145/1067860.1067867)
 *   - `bgm`            END +30 / `bgm-custom` END +15 ← music-listening
 *                                fatigue-reduction studies (~20-30% perceived
 *                                stamina, Karageorghis 2017)
 *   - `pet-mode`       END +30 ← always-on companionship study (de Graaf 2016)
 *   - `device-link`    END +20 ← session-continuity uplift (CRDT/multi-device
 *                                sync removes restart cost)
 *   - `windows-startup` END +15 ← reduces "first launch" friction (uptime ↑)
 *
 *   ─── Brain-tier flat bonuses (supplement the per-model benchmark boost) ───
 *   The three brain-tier skills now carry **flat bonuses** for stats that the
 *   model-benchmark system doesn't cover (CHA, PER, END). This ensures that
 *   installing a local LLM visibly fills out the full stat sheet rather than
 *   only boosting INT/WIS/DEX.
 *
 *   - `local-brain`    INT +5 / WIS +5 / CHA +12 / PER +10 / DEX +8 / END +15
 *                                ← offline availability, faster latency, privacy
 *                                  (no network round-trip ≈ +8 DEX; always-on
 *                                   offline ≈ +15 END; richer local interactions
 *                                   ≈ +12 CHA; local processing ≈ +10 PER)
 *   - `paid-brain`     CHA +8 / PER +5 / END +5
 *                                ← premium API = more eloquent, reliable
 *   - `free-brain`     CHA +3 / PER +3 / END +3
 *                                ← baseline conversational ability
 *
 *   ─── Brain-learning & knowledge skills ────────────────────────────────────
 *   - `rag-knowledge`  INT +15 / WIS +15 / PER +5
 *                                ← RAG accuracy lift (Lewis et al. 2020,
 *                                   +30-50 pp F1 on Natural Questions,
 *                                   split across INT for reasoning and WIS
 *                                   for recall, https://arxiv.org/abs/2005.11401)
 *   - `scholar-quest`  INT +10 / WIS +10 / DEX +5
 *                                ← structured knowledge acquisition chain
 *   - `soul-mirror`    CHA +5 / WIS +5
 *                                ← self-awareness grounds social presence
 *   - `my-persona`     CHA +10 / WIS +5 / END +5
 *                                ← personality definition = stronger identity
 *   - `master-echo`    INT +5 / WIS +10 / CHA +8 / END +5
 *                                ← brain-extracted persona = self-learning
 *   - `expressions-pack` CHA +10 / PER +5
 *                                ← facial expressiveness (Knutsen et al. 2022)
 *   - `notes-vault`    WIS +10 / END +5
 *                                ← organized knowledge retention
 *   - `themes`         END +5
 *                                ← customization drives engagement longevity
 *   - `clipboard-watcher` INT +5 / PER +5 / DEX +10
 *                                ← quick analysis + environmental awareness
 *   - `focus-timer`    DEX +5 / END +5
 *                                ← productivity discipline + session persistence
 *   - `dream-journal`  WIS +10 / INT +5
 *                                ← reflective self-knowledge
 *   - `animation-mastery` INT +5 / CHA +8 / DEX +8
 *                                ← creative expression + precision control;
 *                                  self-taught motion vocabulary (Phase 14.16e)
 */
const STAT_WEIGHTS: Record<StatId, Record<string, number>> = {
  intelligence: {
    'agents':           20,
    'memory':           10,
    'vision':           15,
    'rag-knowledge':    15,
    'scholar-quest':    10,
    'master-echo':       5,
    'local-brain':       5,
    'clipboard-watcher': 5,
    'dream-journal':     5,
    'animation-mastery': 5,
  },
  wisdom: {
    'memory':           50,
    'presence':         10,
    'rag-knowledge':    15,
    'scholar-quest':    10,
    'master-echo':      10,
    'my-persona':        5,
    'soul-mirror':       5,
    'notes-vault':      10,
    'local-brain':       5,
    'dream-journal':    10,
  },
  charisma: {
    'tts':              45,
    'asr':              15,
    'voice-cloning':    35,
    'presence':         15,
    'agents':           10,
    'local-brain':      12,
    'paid-brain':        8,
    'free-brain':        3,
    'my-persona':       10,
    'master-echo':       8,
    'soul-mirror':       5,
    'expressions-pack': 10,
    'animation-mastery': 8,
  },
  perception: {
    'asr':              35,
    'whisper-asr':      25,
    'hotwords':         30,
    'diarization':      15,
    'vision':           20,
    'local-brain':      10,
    'paid-brain':        5,
    'free-brain':        3,
    'rag-knowledge':     5,
    'clipboard-watcher': 5,
    'expressions-pack':  5,
  },
  dexterity: {
    'asr':              10,
    'translation':      30,
    'whisper-asr':      15,
    'windows-shortcuts':25,
    'agents':           15,
    'local-brain':       8,
    'clipboard-watcher':10,
    'focus-timer':       5,
    'scholar-quest':     5,
    'animation-mastery': 8,
  },
  endurance: {
    'bgm':              30,
    'bgm-custom':       15,
    'presence':         25,
    'pet-mode':         30,
    'device-link':      20,
    'windows-startup':  15,
    'local-brain':      15,
    'paid-brain':        5,
    'free-brain':        3,
    'my-persona':        5,
    'master-echo':       5,
    'themes':            5,
    'notes-vault':       5,
    'focus-timer':       5,
  },
};

/**
 * Baseline so a brand-new install reads a sensible value.
 *
 * Set to `1` so a fresh adventurer who only has the auto-configured free
 * brain starts at "Level 1, all stats at 1". Brain boosts and skill unlocks
 * stack on top.
 */
const STAT_BASE = 1;

/**
 * Global scale for per-skill stat weights.
 *
 * Rebalanced in May 2026 so the recommended setup lands around
 * Lv. ~20 (with INT/WIS around ~20), while stronger model tiers can still
 * push significantly higher.
 */
const STAT_WEIGHT_SCALE = 0.12;

/**
 * Compute a single stat from a list of active skill IDs.
 * Pure & deterministic — easy to unit-test.
 *
 * @param brainBoost optional per-stat boost derived from the active brain
 *   model (see `model-benchmarks.ts`). Lets stronger models like Claude Opus
 *   contribute much more than entry-level ones like Gemma 3 1B without
 *   coupling this module to the brain store.
 */
export function computeStat(
  statId: StatId,
  activeSkillIds: ReadonlyArray<string>,
  brainBoost?: Partial<StatSnapshot>,
): number {
  const weights = STAT_WEIGHTS[statId];
  let sum = STAT_BASE;
  for (const skillId of activeSkillIds) {
    sum += (weights[skillId] ?? 0) * STAT_WEIGHT_SCALE;
  }
  if (brainBoost) {
    sum += brainBoost[statId] ?? 0;
  }
  return Math.max(0, Math.min(100, Math.round(sum)));
}

export interface StatSnapshot {
  intelligence: number;
  wisdom: number;
  charisma: number;
  perception: number;
  dexterity: number;
  endurance: number;
}

/** Compute all six stats from active skill IDs in one call. */
export function computeStats(
  activeSkillIds: ReadonlyArray<string>,
  brainBoost?: Partial<StatSnapshot>,
): StatSnapshot {
  return {
    intelligence: computeStat('intelligence', activeSkillIds, brainBoost),
    wisdom:       computeStat('wisdom',       activeSkillIds, brainBoost),
    charisma:     computeStat('charisma',     activeSkillIds, brainBoost),
    perception:   computeStat('perception',   activeSkillIds, brainBoost),
    dexterity:    computeStat('dexterity',    activeSkillIds, brainBoost),
    endurance:    computeStat('endurance',    activeSkillIds, brainBoost),
  };
}

/** Compute the per-stat delta `after - before`. Useful for level-up overlays. */
export function diffStats(before: StatSnapshot, after: StatSnapshot): StatSnapshot {
  return {
    intelligence: after.intelligence - before.intelligence,
    wisdom:       after.wisdom       - before.wisdom,
    charisma:     after.charisma     - before.charisma,
    perception:   after.perception   - before.perception,
    dexterity:    after.dexterity    - before.dexterity,
    endurance:    after.endurance    - before.endurance,
  };
}
