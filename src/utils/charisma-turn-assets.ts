import type { CharismaTurnAsset } from '../types';
import type { LearnedExpression, LearnedMotion, PersonaTraits } from '../stores/persona-types';

export interface CharismaTurnAssetInput {
  text: string;
  motion?: string;
  traits?: PersonaTraits;
  learnedExpressions?: LearnedExpression[];
  learnedMotions?: LearnedMotion[];
}

type LearnedCharismaAsset = LearnedExpression | LearnedMotion;

const NON_WORD_BOUNDARY = /[^a-z0-9]+/g;

export function collectCharismaTurnAssets(input: CharismaTurnAssetInput): CharismaTurnAsset[] {
  const out: CharismaTurnAsset[] = [];
  const seen = new Set<string>();
  const text = normalize(input.text);
  const motion = normalize(input.motion ?? '');

  for (const asset of traitAssets(input.traits)) {
    if (phraseMatches(text, asset.phrase)) {
      pushUnique(out, seen, {
        kind: 'trait',
        assetId: asset.assetId,
        displayName: asset.displayName,
      });
    }
  }

  for (const asset of input.learnedExpressions ?? []) {
    if (learnedAssetFired(asset, text, motion)) {
      pushUnique(out, seen, {
        kind: 'expression',
        assetId: asset.id,
        displayName: asset.name,
      });
    }
  }

  for (const asset of input.learnedMotions ?? []) {
    if (learnedAssetFired(asset, text, motion)) {
      pushUnique(out, seen, {
        kind: 'motion',
        assetId: asset.id,
        displayName: asset.name,
      });
    }
  }

  return out;
}

function traitAssets(traits?: PersonaTraits): Array<{ phrase: string; assetId: string; displayName: string }> {
  if (!traits?.active) return [];
  const assets: Array<{ phrase: string; assetId: string; displayName: string }> = [];
  for (const tone of traits.tone) {
    const phrase = tone.trim();
    if (!phrase) continue;
    assets.push({
      phrase,
      assetId: `tone_${slugify(phrase)}`,
      displayName: `Tone: ${phrase}`,
    });
  }
  for (const quirk of traits.quirks) {
    const phrase = quirk.trim();
    if (!phrase) continue;
    assets.push({
      phrase,
      assetId: `quirk_${slugify(phrase)}`,
      displayName: `Quirk: ${phrase}`,
    });
  }
  return assets;
}

function learnedAssetFired(asset: LearnedCharismaAsset, text: string, motion: string): boolean {
  const trigger = normalize(asset.trigger);
  const name = normalize(asset.name);
  return Boolean(
    (trigger && (trigger === motion || phraseMatches(text, trigger))) ||
    (name && name === motion),
  );
}

function pushUnique(out: CharismaTurnAsset[], seen: Set<string>, asset: CharismaTurnAsset): void {
  const key = `${asset.kind}:${asset.assetId}`;
  if (seen.has(key)) return;
  seen.add(key);
  out.push(asset);
}

function normalize(value: string): string {
  return value.toLowerCase().trim().replace(NON_WORD_BOUNDARY, ' ').replace(/\s+/g, ' ');
}

function phraseMatches(text: string, phrase: string): boolean {
  const normalizedPhrase = normalize(phrase);
  if (normalizedPhrase.length < 3) return false;
  const words = normalizedPhrase.split(' ');
  if (words.length === 1) {
    return new RegExp(`(?:^| )${escapeRegExp(normalizedPhrase)}(?: |$)`).test(text);
  }
  return text.includes(normalizedPhrase);
}

function slugify(value: string): string {
  const slug = normalize(value).replace(/ /g, '_').slice(0, 64);
  return slug || 'trait';
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}