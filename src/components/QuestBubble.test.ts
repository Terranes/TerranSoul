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
  handleQuestChoice: vi.fn().mockResolvedValue(undefined),
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
    mockSkillTreeStore.handleQuestChoice.mockClear();
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

  it('routes confirmation-dialog Accept directly to handleQuestChoice (no double prompt)', async () => {
    const wrapper = mount(QuestBubble);
    await wrapper.find('.ff-orb').trigger('click');
    await nextTick();

    await wrapper.find('.mock-begin').trigger('click');
    await nextTick();

    await wrapper.find('.qcd-btn-primary').trigger('click');
    await nextTick();

    expect(wrapper.emitted('trigger')).toBeTruthy();
    // The user already confirmed in the dialog — don't push another
    // "A New Quest Appears!" prompt asking them to accept again.
    expect(mockSkillTreeStore.triggerQuestEvent).not.toHaveBeenCalled();
    expect(mockSkillTreeStore.handleQuestChoice).toHaveBeenCalledWith('quest-1', 'accept');
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

  describe('positioning (corner-cluster portal)', () => {
    // The orb no longer has hand-tuned `top: 44px` / `right: 16px` magic
    // numbers — it teleports into `CharacterViewport`'s `.corner-cluster`
    // flex column via Vue's `<Teleport>` API.  See `rules/coding-standards.md`
    // § "UI Framework — No CSS Hacking".

    it('renders the orb without any inline `top` / `right` / `position` styles', () => {
      const wrapper = mount(QuestBubble);
      const orb = wrapper.find('.ff-orb').element as HTMLElement;
      // The flex parent (.corner-cluster) owns the layout — the orb itself
      // must NOT carry hand-tuned positioning.
      expect(orb.style.top).toBe('');
      expect(orb.style.right).toBe('');
      expect(orb.style.position).toBe('');
    });

    it('falls back to in-place rendering when the corner-cluster portal is missing', () => {
      // In tests there is no `#corner-cluster-portal` target, so `<Teleport
      // :disabled>` keeps the orb in the component's DOM where test
      // selectors can still reach it.
      const wrapper = mount(QuestBubble);
      expect(wrapper.find('.ff-orb').exists()).toBe(true);
    });
  });
});
