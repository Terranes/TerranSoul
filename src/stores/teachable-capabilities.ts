import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  averageRating,
  deriveTeachingMaturity,
  teachingMaturityColor,
  teachingMaturityLabel,
  type TeachingMaturity,
} from '../utils/teaching-maturity';

export type CapabilityCategory =
  | 'voice'
  | 'vision'
  | 'persona'
  | 'phone'
  | 'files'
  | 'game'
  | 'visuals'
  | 'hardware'
  | 'integrations';

export type CapabilityMaturity = TeachingMaturity;

export type ConfigValue = string | number | boolean | string[] | Record<string, unknown> | Array<Record<string, unknown>> | null;
export type ConfigObject = Record<string, ConfigValue>;

export interface ConfigFieldSchema {
  type?: string;
  label?: string;
  hint?: string;
  min?: number;
  max?: number;
  step?: number;
  options?: string[];
  shape?: Record<string, string>;
}

export type ConfigSchema = Record<string, ConfigFieldSchema | string | undefined>;

export interface TeachableCapability {
  id: string;
  category: CapabilityCategory;
  display_name: string;
  summary: string;
  source_ref: string;
  enabled: boolean;
  config: ConfigObject;
  config_schema: ConfigSchema;
  target_files: string[];
  usage_count: number;
  last_used_at: number;
  rating_sum: number;
  rating_count: number;
  promoted_at: number | null;
  last_promotion_plan_id: string | null;
}

export interface CapabilitySummary {
  total: number;
  enabled: number;
  untested: number;
  learning: number;
  proven: number;
  canon: number;
}

export interface CapabilityListResponse {
  capabilities: TeachableCapability[];
  summary: CapabilitySummary;
}

export interface CapabilityPromoteResponse {
  plan_id: string;
  capability: TeachableCapability;
}

export const CAPABILITY_CATEGORY_ORDER: CapabilityCategory[] = [
  'voice',
  'vision',
  'persona',
  'phone',
  'files',
  'game',
  'visuals',
  'hardware',
  'integrations',
];

export function categoryLabel(category: CapabilityCategory): string {
  switch (category) {
    case 'voice':
      return 'Voice';
    case 'vision':
      return 'Vision';
    case 'persona':
      return 'Persona';
    case 'phone':
      return 'Phone Control';
    case 'files':
      return 'File Assistant';
    case 'game':
      return 'Game Companion';
    case 'visuals':
      return 'Visual Generation';
    case 'hardware':
      return 'Hardware';
    case 'integrations':
      return 'Integrations';
  }
}

export function avgCapabilityRating(capability: TeachableCapability): number {
  return averageRating(capability);
}

export function deriveCapabilityMaturity(capability: TeachableCapability): CapabilityMaturity {
  return deriveTeachingMaturity(capability);
}

export function capabilityMaturityLabel(maturity: CapabilityMaturity): string {
  return teachingMaturityLabel(maturity);
}

export function capabilityMaturityColor(maturity: CapabilityMaturity): string {
  return teachingMaturityColor(maturity);
}

function emptySummary(): CapabilitySummary {
  return {
    total: 0,
    enabled: 0,
    untested: 0,
    learning: 0,
    proven: 0,
    canon: 0,
  };
}

function sortCapabilities(capabilities: TeachableCapability[]): TeachableCapability[] {
  return [...capabilities].sort((left, right) => {
    const categoryDiff = CAPABILITY_CATEGORY_ORDER.indexOf(left.category) - CAPABILITY_CATEGORY_ORDER.indexOf(right.category);
    if (categoryDiff !== 0) return categoryDiff;
    return left.display_name.localeCompare(right.display_name);
  });
}

export const useTeachableCapabilitiesStore = defineStore('teachable-capabilities', () => {
  const capabilities = ref<TeachableCapability[]>([]);
  const summary = ref<CapabilitySummary>(emptySummary());
  const loading = ref(false);
  const error = ref<string | null>(null);
  const lastPromotionPlanId = ref<string | null>(null);

  const byCategory = computed<Record<CapabilityCategory, TeachableCapability[]>>(() => {
    const grouped = Object.fromEntries(
      CAPABILITY_CATEGORY_ORDER.map((category) => [category, [] as TeachableCapability[]]),
    ) as Record<CapabilityCategory, TeachableCapability[]>;
    for (const capability of capabilities.value) {
      grouped[capability.category].push(capability);
    }
    return grouped;
  });

  const categoriesWithCounts = computed(() =>
    CAPABILITY_CATEGORY_ORDER.map((category) => ({
      category,
      label: categoryLabel(category),
      count: byCategory.value[category].length,
    })).filter((entry) => entry.count > 0),
  );

  const enabledCapabilities = computed(() => capabilities.value.filter((capability) => capability.enabled));
  const provenCapabilities = computed(() => capabilities.value.filter((capability) => deriveCapabilityMaturity(capability) === 'proven'));

  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const response = await invoke<CapabilityListResponse>('teachable_capabilities_list');
      capabilities.value = sortCapabilities(response.capabilities);
      summary.value = response.summary;
    } catch (err) {
      error.value = String(err);
    } finally {
      loading.value = false;
    }
  }

  async function setEnabled(id: string, enabled: boolean): Promise<TeachableCapability | null> {
    error.value = null;
    try {
      const capability = await invoke<TeachableCapability>('teachable_capabilities_set_enabled', {
        args: { id, enabled },
      });
      upsertLocal(capability);
      return capability;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function setConfig(id: string, config: ConfigObject): Promise<TeachableCapability | null> {
    error.value = null;
    try {
      const capability = await invoke<TeachableCapability>('teachable_capabilities_set_config', {
        args: { id, config },
      });
      upsertLocal(capability);
      return capability;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function recordUsage(id: string): Promise<TeachableCapability | null> {
    error.value = null;
    try {
      const capability = await invoke<TeachableCapability>('teachable_capabilities_record_usage', {
        args: { id },
      });
      upsertLocal(capability);
      return capability;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function setRating(id: string, rating: number): Promise<TeachableCapability | null> {
    error.value = null;
    try {
      const capability = await invoke<TeachableCapability>('teachable_capabilities_set_rating', {
        args: { id, rating },
      });
      upsertLocal(capability);
      return capability;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function reset(id: string): Promise<TeachableCapability | null> {
    error.value = null;
    try {
      const capability = await invoke<TeachableCapability>('teachable_capabilities_reset', {
        args: { id },
      });
      upsertLocal(capability);
      return capability;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function promote(id: string): Promise<CapabilityPromoteResponse | null> {
    error.value = null;
    lastPromotionPlanId.value = null;
    try {
      const response = await invoke<CapabilityPromoteResponse>('teachable_capabilities_promote', {
        args: { id },
      });
      lastPromotionPlanId.value = response.plan_id;
      upsertLocal(response.capability);
      return response;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function refreshSummary(): Promise<void> {
    try {
      summary.value = await invoke<CapabilitySummary>('teachable_capabilities_summary');
    } catch (err) {
      error.value = String(err);
    }
  }

  function upsertLocal(capability: TeachableCapability): void {
    const index = capabilities.value.findIndex((item) => item.id === capability.id);
    if (index >= 0) {
      capabilities.value[index] = capability;
    } else {
      capabilities.value.push(capability);
    }
    capabilities.value = sortCapabilities(capabilities.value);
    recomputeSummary();
  }

  function recomputeSummary(): void {
    const next = emptySummary();
    for (const capability of capabilities.value) {
      next.total += 1;
      if (capability.enabled) next.enabled += 1;
      const maturity = deriveCapabilityMaturity(capability);
      next[maturity] += 1;
    }
    summary.value = next;
  }

  return {
    capabilities,
    summary,
    loading,
    error,
    lastPromotionPlanId,
    byCategory,
    categoriesWithCounts,
    enabledCapabilities,
    provenCapabilities,
    load,
    setEnabled,
    setConfig,
    recordUsage,
    setRating,
    reset,
    promote,
    refreshSummary,
  };
});