/**
 * Charisma teaching store (Chunk 30.4).
 *
 * Mirrors `src-tauri/src/persona/charisma.rs` + `commands/charisma.rs`.
 * Tracks usage, ratings, and maturity for user-taught persona traits,
 * facial expressions, and motion clips, and exposes a one-click promote
 * path that materialises a multi-agent workflow plan editing source-code
 * defaults.
 */

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { CharismaTurnAsset } from '../types';
import {
  averageRating,
  deriveTeachingMaturity,
  teachingMaturityColor,
  teachingMaturityLabel,
  type TeachingMaturity,
} from '../utils/teaching-maturity';

// ---------------------------------------------------------------------------
// Types (mirrors of Rust)
// ---------------------------------------------------------------------------

export type CharismaAssetKind = 'trait' | 'expression' | 'motion';

export type Maturity = TeachingMaturity;

export interface CharismaStat {
  kind: CharismaAssetKind;
  asset_id: string;
  display_name: string;
  taught_at: number;
  usage_count: number;
  last_used_at: number;
  rating_sum: number;
  rating_count: number;
  promoted_at: number | null;
  last_promotion_plan_id: string | null;
}

export interface CharismaSummary {
  total: number;
  untested: number;
  learning: number;
  proven: number;
  canon: number;
}

export interface CharismaListResponse {
  stats: CharismaStat[];
  summary: CharismaSummary;
}

export interface CharismaPromoteResponse {
  plan_id: string;
  stat: CharismaStat;
}

interface BackendTurnAsset {
  kind: CharismaAssetKind;
  asset_id: string;
  display_name: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Match the Rust `CharismaStat::maturity()` derivation. */
export function deriveMaturity(stat: CharismaStat): Maturity {
  return deriveTeachingMaturity(stat);
}

export function avgRating(stat: CharismaStat): number {
  return averageRating(stat);
}

export function maturityLabel(m: Maturity): string {
  return teachingMaturityLabel(m);
}

export function maturityColor(m: Maturity): string {
  return teachingMaturityColor(m);
}

export function kindIcon(kind: CharismaAssetKind): string {
  switch (kind) {
    case 'trait':
      return '📝';
    case 'expression':
      return '😊';
    case 'motion':
      return '💃';
  }
}

export function kindLabel(kind: CharismaAssetKind): string {
  switch (kind) {
    case 'trait':
      return 'Trait';
    case 'expression':
      return 'Expression';
    case 'motion':
      return 'Motion';
  }
}

function uniqueTurnAssets(assets: CharismaTurnAsset[]): CharismaTurnAsset[] {
  const seen = new Set<string>();
  const unique: CharismaTurnAsset[] = [];
  for (const asset of assets) {
    const key = `${asset.kind}:${asset.assetId}`;
    if (seen.has(key)) continue;
    seen.add(key);
    unique.push(asset);
  }
  return unique;
}

function toBackendTurnAsset(asset: CharismaTurnAsset): BackendTurnAsset {
  return {
    kind: asset.kind,
    asset_id: asset.assetId,
    display_name: asset.displayName,
  };
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useCharismaStore = defineStore('charisma', () => {
  const stats = ref<CharismaStat[]>([]);
  const summary = ref<CharismaSummary>({
    total: 0,
    untested: 0,
    learning: 0,
    proven: 0,
    canon: 0,
  });
  const loading = ref(false);
  const error = ref<string | null>(null);

  const byKind = computed(() => {
    const out: Record<CharismaAssetKind, CharismaStat[]> = {
      trait: [],
      expression: [],
      motion: [],
    };
    for (const s of stats.value) {
      out[s.kind].push(s);
    }
    return out;
  });

  const provenCount = computed(() => summary.value.proven);
  const totalTaught = computed(() => summary.value.total);

  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const resp = await invoke<CharismaListResponse>('charisma_list');
      stats.value = resp.stats;
      summary.value = resp.summary;
    } catch (err) {
      error.value = String(err);
    } finally {
      loading.value = false;
    }
  }

  async function recordUsage(
    kind: CharismaAssetKind,
    assetId: string,
    displayName: string,
  ): Promise<CharismaStat | null> {
    error.value = null;
    try {
      const stat = await invoke<CharismaStat>('charisma_record_usage', {
        args: { kind, asset_id: assetId, display_name: displayName },
      });
      upsertLocal(stat);
      return stat;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function setRating(
    kind: CharismaAssetKind,
    assetId: string,
    displayName: string,
    rating: number,
  ): Promise<CharismaStat | null> {
    error.value = null;
    try {
      const stat = await invoke<CharismaStat>('charisma_set_rating', {
        args: { kind, asset_id: assetId, display_name: displayName, rating },
      });
      upsertLocal(stat);
      return stat;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function recordTurnUsage(assets: CharismaTurnAsset[]): Promise<CharismaStat[]> {
    const unique = uniqueTurnAssets(assets);
    const recorded: CharismaStat[] = [];
    for (const asset of unique) {
      const stat = await recordUsage(asset.kind, asset.assetId, asset.displayName);
      if (stat) recorded.push(stat);
    }
    return recorded;
  }

  async function rateTurnAssets(
    assets: CharismaTurnAsset[],
    rating: number,
  ): Promise<CharismaStat[]> {
    error.value = null;
    try {
      const rated = await invoke<CharismaStat[]>('charisma_rate_turn', {
        args: {
          assets: uniqueTurnAssets(assets).map(toBackendTurnAsset),
          rating,
        },
      });
      for (const stat of rated) {
        upsertLocal(stat);
      }
      return rated;
    } catch (err) {
      error.value = String(err);
      return [];
    }
  }

  async function remove(kind: CharismaAssetKind, assetId: string): Promise<boolean> {
    error.value = null;
    try {
      await invoke('charisma_delete', { args: { kind, asset_id: assetId } });
      stats.value = stats.value.filter((s) => !(s.kind === kind && s.asset_id === assetId));
      recomputeSummary();
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  async function promote(
    kind: CharismaAssetKind,
    assetId: string,
  ): Promise<CharismaPromoteResponse | null> {
    error.value = null;
    try {
      const resp = await invoke<CharismaPromoteResponse>('charisma_promote', {
        args: { kind, asset_id: assetId },
      });
      upsertLocal(resp.stat);
      return resp;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function refreshSummary(): Promise<void> {
    try {
      summary.value = await invoke<CharismaSummary>('charisma_summary');
    } catch (err) {
      error.value = String(err);
    }
  }

  function upsertLocal(stat: CharismaStat): void {
    const idx = stats.value.findIndex(
      (s) => s.kind === stat.kind && s.asset_id === stat.asset_id,
    );
    if (idx >= 0) {
      stats.value[idx] = stat;
    } else {
      stats.value.unshift(stat);
    }
    recomputeSummary();
  }

  function recomputeSummary(): void {
    const next: CharismaSummary = {
      total: 0,
      untested: 0,
      learning: 0,
      proven: 0,
      canon: 0,
    };
    for (const s of stats.value) {
      next.total++;
      const m = deriveMaturity(s);
      switch (m) {
        case 'untested':
          next.untested++;
          break;
        case 'learning':
          next.learning++;
          break;
        case 'proven':
          next.proven++;
          break;
        case 'canon':
          next.canon++;
          break;
      }
    }
    summary.value = next;
  }

  return {
    // state
    stats,
    summary,
    loading,
    error,
    // computed
    byKind,
    provenCount,
    totalTaught,
    // actions
    load,
    recordUsage,
    recordTurnUsage,
    setRating,
    rateTurnAssets,
    remove,
    promote,
    refreshSummary,
  };
});
