import { describe, expect, it, vi, beforeEach } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const { mockInvoke } = vi.hoisted(() => ({ mockInvoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }));

import {
  avgCapabilityRating,
  deriveCapabilityMaturity,
  type TeachableCapability,
  useTeachableCapabilitiesStore,
} from './teachable-capabilities';

function capability(overrides: Partial<TeachableCapability> = {}): TeachableCapability {
  return {
    id: 'wake_word',
    category: 'voice',
    display_name: 'Wake-word activation',
    summary: 'Listen for a configurable hotword.',
    source_ref: 'external_research:voice/wake-word',
    enabled: false,
    config: { phrase: 'hey terra' },
    config_schema: { phrase: { type: 'string', label: 'Wake phrase' } },
    target_files: ['src/stores/voice.ts'],
    usage_count: 0,
    last_used_at: 0,
    rating_sum: 0,
    rating_count: 0,
    promoted_at: null,
    last_promotion_plan_id: null,
    ...overrides,
  };
}

describe('teachable capabilities store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('derives maturity with the same thresholds as the backend', () => {
    expect(deriveCapabilityMaturity(capability())).toBe('untested');
    expect(deriveCapabilityMaturity(capability({ enabled: true, usage_count: 1 }))).toBe('learning');
    expect(deriveCapabilityMaturity(capability({ enabled: false, usage_count: 50, rating_sum: 50, rating_count: 10 }))).toBe('untested');
    expect(deriveCapabilityMaturity(capability({ enabled: true, usage_count: 10, rating_sum: 8, rating_count: 2 }))).toBe('proven');
    expect(deriveCapabilityMaturity(capability({ promoted_at: 123 }))).toBe('canon');
  });

  it('computes average rating safely', () => {
    expect(avgCapabilityRating(capability())).toBe(0);
    expect(avgCapabilityRating(capability({ rating_sum: 9, rating_count: 3 }))).toBe(3);
  });

  it('loads capabilities and groups them by category', async () => {
    mockInvoke.mockResolvedValue({
      capabilities: [
        capability(),
        capability({ id: 'vision_input', category: 'vision', display_name: 'Webcam vision input' }),
      ],
      summary: { total: 2, enabled: 0, untested: 2, learning: 0, proven: 0, canon: 0 },
    });

    const store = useTeachableCapabilitiesStore();
    await store.load();

    expect(mockInvoke).toHaveBeenCalledWith('teachable_capabilities_list');
    expect(store.byCategory.voice).toHaveLength(1);
    expect(store.byCategory.vision).toHaveLength(1);
    expect(store.summary.total).toBe(2);
  });

  it('updates local state after enabling and rating', async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'teachable_capabilities_set_enabled') {
        return Promise.resolve(capability({ enabled: true }));
      }
      if (command === 'teachable_capabilities_set_rating') {
        return Promise.resolve(capability({ enabled: true, usage_count: 10, rating_sum: 5, rating_count: 1 }));
      }
      return Promise.resolve(null);
    });

    const store = useTeachableCapabilitiesStore();
    await store.setEnabled('wake_word', true);
    expect(store.enabledCapabilities).toHaveLength(1);

    await store.setRating('wake_word', 5);
    expect(store.summary.proven).toBe(1);
  });

  it('stores the latest promotion plan id', async () => {
    mockInvoke.mockResolvedValue({
      plan_id: 'plan-1',
      capability: capability({ promoted_at: 456, last_promotion_plan_id: 'plan-1' }),
    });

    const store = useTeachableCapabilitiesStore();
    const response = await store.promote('wake_word');

    expect(mockInvoke).toHaveBeenCalledWith('teachable_capabilities_promote', { args: { id: 'wake_word' } });
    expect(response?.plan_id).toBe('plan-1');
    expect(store.lastPromotionPlanId).toBe('plan-1');
    expect(store.summary.canon).toBe(1);
  });
});