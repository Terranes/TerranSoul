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
 * The values aren't tightly tuned: they only need to be ordinal so the bars
 * move sensibly as the user picks up skills.
 */
const STAT_WEIGHTS: Record<StatId, Record<string, number>> = {
  intelligence: {
    'free-brain':       30,
    'paid-brain':       45,
    'local-brain':      40,
    'agents':           20,
    'memory':           10,
    'vision':           15,
  },
  wisdom: {
    'memory':           50,
    'free-brain':       15,
    'paid-brain':       25,
    'local-brain':      25,
    'presence':         10,
  },
  charisma: {
    'tts':              45,
    'asr':              15,
    'voice-cloning':    35,
    'presence':         15,
    'agents':           10,
  },
  perception: {
    'asr':              35,
    'whisper-asr':      25,
    'hotwords':         30,
    'diarization':      15,
    'vision':           20,
  },
  dexterity: {
    'asr':              10,
    'translation':      30,
    'whisper-asr':      15,
    'windows-shortcuts':25,
    'agents':           15,
    'paid-brain':       10,
  },
  endurance: {
    'bgm':              30,
    'bgm-custom':       15,
    'presence':         25,
    'pet-mode':         30,
    'device-link':      20,
    'windows-startup':  15,
  },
};

/** Baseline so even a fresh install reads a non-zero value. */
const STAT_BASE = 5;

/**
 * Compute a single stat from a list of active skill IDs.
 * Pure & deterministic — easy to unit-test.
 */
export function computeStat(statId: StatId, activeSkillIds: ReadonlyArray<string>): number {
  const weights = STAT_WEIGHTS[statId];
  let sum = STAT_BASE;
  for (const skillId of activeSkillIds) {
    sum += weights[skillId] ?? 0;
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
export function computeStats(activeSkillIds: ReadonlyArray<string>): StatSnapshot {
  return {
    intelligence: computeStat('intelligence', activeSkillIds),
    wisdom:       computeStat('wisdom',       activeSkillIds),
    charisma:     computeStat('charisma',     activeSkillIds),
    perception:   computeStat('perception',   activeSkillIds),
    dexterity:    computeStat('dexterity',    activeSkillIds),
    endurance:    computeStat('endurance',    activeSkillIds),
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
