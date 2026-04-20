import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import QuestBubble from './QuestBubble.vue';
import { useSkillTreeStore } from '../stores/skill-tree';
import { useBrainStore } from '../stores/brain';

// Create mock objects first
const mockConversationStore = {
  sendMessage: vi.fn(),
  addMessage: vi.fn(),
  messages: [] as unknown[],
  isThinking: false,
};

const mockChatExpansion = {
  isChatExpanded: { value: false },
};

// Mock the stores and composables - must be at top level
vi.mock('../stores/skill-tree');
vi.mock('../stores/brain');
vi.mock('../stores/conversation', () => ({
  useConversationStore: () => mockConversationStore,
}));
vi.mock('../composables/useChatExpansion', () => ({
  useChatExpansion: () => mockChatExpansion,
}));

// Mock heavy child components — we only verify QuestBubble's wiring here.
vi.mock('./QuestRewardPanel.vue', () => ({
  default: { name: 'QuestRewardPanel', template: '<div class="mock-reward-panel"></div>' },
}));
vi.mock('./QuestConfirmationDialog.vue', () => ({
  default: {
    name: 'QuestConfirmationDialog',
    props: ['visible', 'quest'],
    emits: ['accept', 'cancel'],
    template:
      '<div v-if="visible" class="mock-confirm-dialog quest-confirm-dialog">' +
      '  <button class="qcd-btn-primary" @click="$emit(\'accept\')">Accept</button>' +
      '  <button class="qcd-btn-secondary" @click="$emit(\'cancel\')">Cancel</button>' +
      '</div>',
  },
}));
vi.mock('./SkillConstellation.vue', () => ({
  default: {
    name: 'SkillConstellation',
    props: ['visible'],
    emits: ['close', 'navigate', 'begin'],
    template:
      '<div v-if="visible" class="mock-skill-constellation skill-constellation">' +
      '  <button class="mock-begin" @click="$emit(\'begin\', \'quest-1\')">Begin</button>' +
      '  <button class="mock-navigate" @click="$emit(\'navigate\', \'voice\')">Navigate</button>' +
      '  <button class="mock-close" @click="$emit(\'close\')">Close</button>' +
      '</div>',
  },
}));

const mockSkillTreeStore = {
  progressPercent: 42,
  activeCount: 8,
  totalNodes: 15,
  pinnedQuests: [] as unknown[],
  nodes: [
    {
      id: 'quest-1',
      name: 'Available Quest 1',
      tagline: 'First available quest',
      description: 'Description for quest 1',
      icon: '⚔️',
      tier: 'foundation',
      category: 'brain',
      questSteps: [],
      rewards: ['XP'],
      rewardIcons: ['🎯'],
      requires: [],
      combos: [],
    },
  ],
  tracker: { pinnedQuestIds: [] as string[], manuallyCompletedIds: [] as string[] },
  getSkillStatus: vi.fn((id: string) => (id === 'quest-1' ? 'available' : 'locked')),
  pinQuest: vi.fn(),
  unpinQuest: vi.fn(),
  triggerQuestEvent: vi.fn(),
  markComplete: vi.fn(),
  unmarkComplete: vi.fn(),
};

const mockBrainStore = {
  isReady: true,
  hasBrain: true,
  generateResponse: vi.fn().mockResolvedValue(''),
  processPromptSilently: vi.fn().mockResolvedValue(
    '{"priority_order":["quest-1"],"top_recommendation":"quest-1"}',
  ),
};

describe('QuestBubble', () => {
  beforeEach(() => {
    vi.mocked(useSkillTreeStore).mockReturnValue(mockSkillTreeStore as never);
    vi.mocked(useBrainStore).mockReturnValue(mockBrainStore as never);
    mockConversationStore.messages = [];
    mockConversationStore.isThinking = false;
    mockChatExpansion.isChatExpanded.value = false;
    mockSkillTreeStore.triggerQuestEvent.mockClear();
    mockSkillTreeStore.pinQuest.mockClear();
    mockSkillTreeStore.unpinQuest.mockClear();
  });

  it('renders progress orb with percentage', () => {
    const wrapper = mount(QuestBubble);
    expect(wrapper.find('.ff-orb-pct').text()).toBe('42%');
    expect(wrapper.find('.ff-orb').exists()).toBe(true);
  });

  it('opens the constellation when the orb is clicked', async () => {
    const wrapper = mount(QuestBubble);
    expect(wrapper.find('.skill-constellation').exists()).toBe(false);

    await wrapper.find('.ff-orb').trigger('click');
    await nextTick();

    expect(wrapper.find('.skill-constellation').exists()).toBe(true);
  });

  it('toggles the constellation closed when the orb is clicked again', async () => {
    const wrapper = mount(QuestBubble);

    await wrapper.find('.ff-orb').trigger('click');
    await nextTick();
    expect(wrapper.find('.skill-constellation').exists()).toBe(true);

    await wrapper.find('.ff-orb').trigger('click');
    await nextTick();
    expect(wrapper.find('.skill-constellation').exists()).toBe(false);
  });

  it('closes the constellation when its close event fires', async () => {
    const wrapper = mount(QuestBubble);
    await wrapper.find('.ff-orb').trigger('click');
    await nextTick();

    await wrapper.find('.mock-close').trigger('click');
    await nextTick();

    expect(wrapper.find('.skill-constellation').exists()).toBe(false);
  });

  it('shows confirmation dialog when constellation emits begin', async () => {
    const wrapper = mount(QuestBubble);
    await wrapper.find('.ff-orb').trigger('click');
    await nextTick();

    await wrapper.find('.mock-begin').trigger('click');
    await nextTick();

    expect(wrapper.find('.quest-confirm-dialog').exists()).toBe(true);
  });

  it('emits trigger when quest is accepted in the confirmation dialog', async () => {
    const wrapper = mount(QuestBubble);
    await wrapper.find('.ff-orb').trigger('click');
    await nextTick();

    await wrapper.find('.mock-begin').trigger('click');
    await nextTick();

    await wrapper.find('.qcd-btn-primary').trigger('click');
    await nextTick();

    expect(wrapper.emitted('trigger')).toBeTruthy();
    expect(mockSkillTreeStore.triggerQuestEvent).toHaveBeenCalledWith('quest-1');
  });

  it('emits navigate when constellation emits navigate', async () => {
    const wrapper = mount(QuestBubble);
    await wrapper.find('.ff-orb').trigger('click');
    await nextTick();

    await wrapper.find('.mock-navigate').trigger('click');
    await nextTick();

    expect(wrapper.emitted('navigate')).toBeTruthy();
    expect(wrapper.emitted('navigate')![0]).toEqual(['voice']);
  });

  it('shows progress ring with the correct stroke offset', () => {
    const wrapper = mount(QuestBubble);
    const ring = wrapper.find('.ff-orb-ring-fill');
    const circumference = 2 * Math.PI * 22;
    const expectedOffset = circumference - (circumference * 42) / 100;
    expect(ring.attributes('stroke-dashoffset')).toBe(expectedOffset.toString());
  });

  it('calls AI sorting on mount when a brain is available', async () => {
    mockBrainStore.processPromptSilently.mockClear();
    mount(QuestBubble);
    await nextTick();
    expect(mockBrainStore.processPromptSilently).toHaveBeenCalledWith(
      expect.stringContaining('Analyze these available quests'),
    );
  });

  it('handles AI sorting failure gracefully', async () => {
    mockBrainStore.processPromptSilently.mockRejectedValueOnce(new Error('AI unavailable'));
    const wrapper = mount(QuestBubble);
    await nextTick();
    // Component should still render without throwing.
    expect(wrapper.find('.ff-orb').exists()).toBe(true);
  });

  describe('dynamic positioning', () => {
    beforeEach(() => {
      mockChatExpansion.isChatExpanded = { value: false };
      Object.defineProperty(window, 'innerWidth', {
        writable: true,
        configurable: true,
        value: 1024,
      });
    });

    it('uses default desktop positioning', () => {
      const wrapper = mount(QuestBubble);
      const hub = wrapper.find('.quest-hub').element as HTMLElement;
      expect(hub.style.top).toBe('44px');
      expect(hub.style.right).toBe('16px');
      expect(hub.style.position).toBe('fixed');
    });

    it('uses mobile positioning on small screens', async () => {
      Object.defineProperty(window, 'innerWidth', {
        writable: true,
        configurable: true,
        value: 600,
      });
      const wrapper = mount(QuestBubble);
      (wrapper.vm as unknown as { screenWidth: number }).screenWidth = 600;
      await nextTick();
      const hub = wrapper.find('.quest-hub').element as HTMLElement;
      expect(hub.style.top).toBe('6px');
      expect(hub.style.right).toBe('52px');
    });

    it('uses fixed positioning with z-index 19', () => {
      const wrapper = mount(QuestBubble);
      const hub = wrapper.find('.quest-hub').element as HTMLElement;
      expect(hub.style.position).toBe('fixed');
      expect(hub.style.zIndex).toBe('19');
    });
  });
});
