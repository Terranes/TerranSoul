import { describe, expect, it } from 'vitest';
import { collectCharismaTurnAssets } from './charisma-turn-assets';
import type { PersonaTraits } from '../stores/persona-types';

const traits: PersonaTraits = {
  version: 1,
  name: 'Soul',
  role: 'TerranSoul companion',
  bio: '',
  tone: ['warm'],
  quirks: ['spark check'],
  avoid: [],
  exampleDialogue: [],
  active: true,
  updatedAt: 0,
};

describe('collectCharismaTurnAssets', () => {
  it('collects active tone and quirk matches from assistant text', () => {
    const assets = collectCharismaTurnAssets({
      text: 'A warm spark check before we continue.',
      traits,
    });

    expect(assets).toEqual([
      { kind: 'trait', assetId: 'tone_warm', displayName: 'Tone: warm' },
      { kind: 'trait', assetId: 'quirk_spark_check', displayName: 'Quirk: spark check' },
    ]);
  });

  it('collects learned expression and motion assets by trigger', () => {
    const assets = collectCharismaTurnAssets({
      text: 'I will bow and give a bright grin.',
      motion: 'bow',
      learnedExpressions: [
        {
          id: 'lex_grin',
          kind: 'expression',
          name: 'Bright Grin',
          trigger: 'bright grin',
          weights: {},
          learnedAt: 1,
        },
      ],
      learnedMotions: [
        {
          id: 'lmo_bow',
          kind: 'motion',
          name: 'Polite Bow',
          trigger: 'bow',
          fps: 30,
          duration_s: 1,
          frames: [],
          learnedAt: 1,
        },
      ],
    });

    expect(assets).toEqual([
      { kind: 'expression', assetId: 'lex_grin', displayName: 'Bright Grin' },
      { kind: 'motion', assetId: 'lmo_bow', displayName: 'Polite Bow' },
    ]);
  });

  it('deduplicates assets that match both motion and text', () => {
    const assets = collectCharismaTurnAssets({
      text: 'Here is a bow.',
      motion: 'bow',
      learnedMotions: [
        {
          id: 'lmo_bow',
          kind: 'motion',
          name: 'Bow',
          trigger: 'bow',
          fps: 30,
          duration_s: 1,
          frames: [],
          learnedAt: 1,
        },
      ],
    });

    expect(assets).toEqual([{ kind: 'motion', assetId: 'lmo_bow', displayName: 'Bow' }]);
  });
});