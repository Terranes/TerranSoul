import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import BrainStatSheet from './BrainStatSheet.vue';
import { useSkillTreeStore } from '../stores/skill-tree';

vi.mock('../stores/skill-tree');

interface FakeNode { id: string; }

const NODES: FakeNode[] = [
  { id: 'free-brain' }, { id: 'paid-brain' }, { id: 'local-brain' },
  { id: 'tts' }, { id: 'asr' }, { id: 'memory' }, { id: 'hotwords' },
  { id: 'bgm' }, { id: 'pet-mode' }, { id: 'agents' },
];

const activeIds = new Set<string>();

const mockStore = {
  nodes: NODES,
  getSkillStatus: vi.fn((id: string) => activeIds.has(id) ? 'active' : 'available'),
};

beforeEach(() => {
  activeIds.clear();
  vi.mocked(useSkillTreeStore).mockReturnValue(mockStore as never);
});

describe('BrainStatSheet', () => {
  it('renders all six stats', () => {
    const wrapper = mount(BrainStatSheet);
    expect(wrapper.find('[data-testid="brain-stat-sheet"]').exists()).toBe(true);
    for (const id of ['intelligence', 'wisdom', 'charisma', 'perception', 'dexterity', 'endurance']) {
      expect(wrapper.find(`[data-testid="stat-${id}"]`).exists()).toBe(true);
    }
  });

  it('shows baseline values when no skills are active', () => {
    const wrapper = mount(BrainStatSheet);
    const intValue = wrapper.find('[data-testid="stat-value-intelligence"]').text();
    expect(parseInt(intValue, 10)).toBeGreaterThanOrEqual(1);
    expect(parseInt(intValue, 10)).toBeLessThan(20);
  });

  it('reflects activated skills in the stat values', async () => {
    activeIds.add('memory');
    activeIds.add('tts');
    const wrapper = mount(BrainStatSheet);
    await nextTick();
    const wisValue = parseInt(wrapper.find('[data-testid="stat-value-wisdom"]').text(), 10);
    const chaValue = parseInt(wrapper.find('[data-testid="stat-value-charisma"]').text(), 10);
    expect(wisValue).toBeGreaterThan(40);
    expect(chaValue).toBeGreaterThan(40);
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
