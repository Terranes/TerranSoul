import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { ref, nextTick, type Ref } from 'vue';
import QuestRewardCeremony from './QuestRewardCeremony.vue';
import { useSkillTreeStore } from '../stores/skill-tree';

vi.mock('../stores/skill-tree');

interface FakeNode {
  id: string;
  name: string;
  tagline: string;
  icon: string;
  rewards: string[];
  rewardIcons: string[];
}

const NODES: FakeNode[] = [
  { id: 'memory', name: 'Long-Term Memory', tagline: 'Remember', icon: '📚', rewards: ['Recall'], rewardIcons: ['🧠'] },
  { id: 'tts',    name: 'Gift of Speech',    tagline: 'Speak',     icon: '🗣️', rewards: ['Voice'],   rewardIcons: ['🔊'] },
];

const activeIds = new Set<string>(['memory']); // memory already active
const activations: Ref<Record<string, number>> = ref({ memory: 1000 });
const lastSeen = ref<number>(0);

const mockStore = {
  nodes: NODES,
  activeCombos: [],
  get tracker() {
    return {
      activationTimestamps: activations.value,
      lastSeenActivationTimestamp: lastSeen.value,
      seenComboKeys: [],
    };
  },
  getSkillStatus: vi.fn((id: string) => activeIds.has(id) ? 'active' : 'available'),
  setLastSeenActivationTimestamp: vi.fn((ts: number) => { if (ts > lastSeen.value) lastSeen.value = ts; }),
};

beforeEach(() => {
  activeIds.clear();
  activeIds.add('memory');
  activations.value = { memory: 1000 };
  lastSeen.value = 0;
  mockStore.setLastSeenActivationTimestamp.mockClear();
  vi.mocked(useSkillTreeStore).mockReturnValue(mockStore as never);
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  document.body.innerHTML = '';
});

describe('QuestRewardCeremony', () => {
  it('does NOT show on first launch — establishes the high-water mark instead', async () => {
    mount(QuestRewardCeremony, { attachTo: document.body });
    await nextTick();

    expect(document.querySelector('[data-testid="quest-reward-ceremony"]')).toBeNull();
    expect(mockStore.setLastSeenActivationTimestamp).toHaveBeenCalledWith(1000);
  });

  it('shows a ceremony for a newly-activated skill above the high-water mark', async () => {
    lastSeen.value = 1000;
    mount(QuestRewardCeremony, { attachTo: document.body });
    await nextTick();

    // No new activations yet → no ceremony
    expect(document.querySelector('[data-testid="quest-reward-ceremony"]')).toBeNull();

    // Now unlock 'tts' with a fresher timestamp
    activeIds.add('tts');
    activations.value = { ...activations.value, tts: 2000 };
    await nextTick();
    await nextTick();

    expect(document.querySelector('[data-testid="quest-reward-ceremony"]')).toBeTruthy();
    expect(document.body.textContent).toContain('Gift of Speech');
    expect(document.body.textContent).toContain('Quest Complete');
  });

  it('Close button dismisses the toast and updates the high-water mark', async () => {
    lastSeen.value = 1000;
    mount(QuestRewardCeremony, { attachTo: document.body });
    await nextTick();

    activeIds.add('tts');
    activations.value = { ...activations.value, tts: 2000 };
    await nextTick();
    await nextTick();

    const dismiss = document.querySelector<HTMLButtonElement>('[data-testid="qrc-dismiss"]')!;
    expect(dismiss).toBeTruthy();
    dismiss.click();
    await nextTick();

    expect(mockStore.setLastSeenActivationTimestamp).toHaveBeenCalledWith(2000);
  });

  it('renders stat-delta chips for the activated skill', async () => {
    lastSeen.value = 1000;
    mount(QuestRewardCeremony, { attachTo: document.body });
    await nextTick();

    activeIds.add('tts');
    activations.value = { ...activations.value, tts: 2000 };
    await nextTick();
    await nextTick();

    expect(document.body.textContent).toMatch(/CHA.*\+/);
  });
});
