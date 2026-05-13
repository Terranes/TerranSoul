import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import BrainStatSheet from './BrainStatSheet.vue';
import { useSkillTreeStore } from '../stores/skill-tree';
import { useBrainStore } from '../stores/brain';

vi.mock('../stores/skill-tree');
vi.mock('../stores/brain');

interface FakeNode { id: string; }

const NODES: FakeNode[] = [
  { id: 'free-brain' }, { id: 'paid-brain' }, { id: 'local-brain' },
  { id: 'tts' }, { id: 'asr' }, { id: 'memory' }, { id: 'hotwords' },
  { id: 'bgm' }, { id: 'pet-mode' }, { id: 'agents' },
];

const activeIds = new Set<string>();

const mockSkillTreeStore = {
  nodes: NODES,
  getSkillStatus: vi.fn((id: string) => activeIds.has(id) ? 'active' : 'available'),
};

// Default brain store mock — no brain configured. Individual tests override
// `mockBrainStore.brainMode` / `freeProviders` as needed.
const mockBrainStore: {
  brainMode: { mode: string; provider_id?: string; model?: string } | null;
  freeProviders: { id: string; model: string }[];
} = {
  brainMode: null,
  freeProviders: [],
};

beforeEach(() => {
  activeIds.clear();
  mockBrainStore.brainMode = null;
  mockBrainStore.freeProviders = [];
  vi.mocked(useSkillTreeStore).mockReturnValue(mockSkillTreeStore as never);
  vi.mocked(useBrainStore).mockReturnValue(mockBrainStore as never);
});

describe('BrainStatSheet', () => {
  it('renders all six stats', () => {
    const wrapper = mount(BrainStatSheet);
    expect(wrapper.find('[data-testid="brain-stat-sheet"]').exists()).toBe(true);
    for (const id of ['intelligence', 'wisdom', 'charisma', 'perception', 'dexterity', 'endurance']) {
      expect(wrapper.find(`[data-testid="stat-${id}"]`).exists()).toBe(true);
    }
  });

  it('starts at level 1 with all stats at 1 when no brain or skills are active', () => {
    const wrapper = mount(BrainStatSheet);
    for (const id of ['intelligence', 'wisdom', 'charisma', 'perception', 'dexterity', 'endurance']) {
      const value = parseInt(wrapper.find(`[data-testid="stat-value-${id}"]`).text(), 10);
      expect(value).toBe(1);
    }
    expect(wrapper.text()).toMatch(/Lv\.\s*1/);
  });

  it('reflects activated skills in the stat values', async () => {
    activeIds.add('memory');
    activeIds.add('tts');
    const wrapper = mount(BrainStatSheet);
    await nextTick();
    const wisValue = parseInt(wrapper.find('[data-testid="stat-value-wisdom"]').text(), 10);
    const chaValue = parseInt(wrapper.find('[data-testid="stat-value-charisma"]').text(), 10);
    expect(wisValue).toBeGreaterThan(5);
    expect(chaValue).toBeGreaterThan(5);
  });

  it('boosts intelligence when a flagship paid model is selected', async () => {
    mockBrainStore.brainMode = { mode: 'paid_api', model: 'claude-opus-4.7' };
    const wrapper = mount(BrainStatSheet);
    await nextTick();
    const intValue = parseInt(wrapper.find('[data-testid="stat-value-intelligence"]').text(), 10);
    // Tier S boost is now calibrated lower so recommended setups stay near Lv.20.
    // weights are tuned later.
    expect(intValue).toBeGreaterThanOrEqual(25);
  });

  it('a flagship boost is much higher than a basic free brain', async () => {
    mockBrainStore.brainMode = { mode: 'free_api', provider_id: 'pollinations' };
    mockBrainStore.freeProviders = [{ id: 'pollinations', model: 'openai' }];
    const free = mount(BrainStatSheet);
    await nextTick();
    const freeInt = parseInt(free.find('[data-testid="stat-value-intelligence"]').text(), 10);

    mockBrainStore.brainMode = { mode: 'paid_api', model: 'claude-opus-4.7' };
    mockBrainStore.freeProviders = [];
    const opus = mount(BrainStatSheet);
    await nextTick();
    const opusInt = parseInt(opus.find('[data-testid="stat-value-intelligence"]').text(), 10);

    expect(opusInt - freeInt).toBeGreaterThanOrEqual(15);
  });

  it('renders the Active Modifiers panel (Chunk 134)', () => {
    const wrapper = mount(BrainStatSheet);
    const mods = wrapper.find('[data-testid="bss-modifiers"]');
    expect(mods.exists()).toBe(true);
    expect(mods.text()).toMatch(/Memory recall depth/);
    expect(mods.text()).toMatch(/Chat history kept/);
    expect(mods.text()).toMatch(/Hotword sensitivity/);
    expect(mods.text()).toMatch(/TTS expressiveness/);
  });

  it('shows a level badge', () => {
    const wrapper = mount(BrainStatSheet);
    expect(wrapper.text()).toMatch(/Lv\.\s*\d+/);
  });
});
