import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import QuestRewardPanel from './QuestRewardPanel.vue';
import type { SkillNode } from '../stores/skill-tree';
import type { RewardChoice } from './QuestRewardPanel.vue';

const mockQuest: SkillNode = {
  id: 'test-quest',
  name: 'Test Quest',
  tagline: 'A quest for testing',
  description: 'This is a test quest description',
  icon: '⚔️',
  category: 'brain',
  questSteps: [
    { label: 'Step 1: Learn basics', action: 'info', target: undefined },
    { label: 'Step 2: Practice', action: 'info', target: undefined },
    { label: 'Step 3: Master it', action: 'navigate', target: 'settings' },
  ],
  rewards: ['XP', 'Knowledge', 'Skills'],
  rewardIcons: ['🎯', '📚', '⚡'],
  requires: [],
  tier: 'foundation',
  combos: [],
};

const mockChoices: RewardChoice[] = [
  { label: 'Yes', value: 'accept', primary: true },
  { label: 'No', value: 'decline', primary: false },
];

describe('QuestRewardPanel', () => {
  it('should render when visible with quest data', () => {
    const wrapper = mount(QuestRewardPanel, {
      props: {
        visible: true,
        quest: mockQuest,
        showChoices: false,
      }
    });

    expect(wrapper.find('.reward-panel').exists()).toBe(true);
    expect(wrapper.text()).toContain('Test Quest');
    expect(wrapper.text()).toContain('A quest for testing');
    expect(wrapper.find('.rp-icon').text()).toBe('⚔️');
  });

  it('should not render when not visible', () => {
    const wrapper = mount(QuestRewardPanel, {
      props: {
        visible: false,
        quest: mockQuest,
        showChoices: false,
      }
    });

    expect(wrapper.find('.reward-panel').exists()).toBe(false);
  });

  it('should display reward items correctly', () => {
    const wrapper = mount(QuestRewardPanel, {
      props: {
        visible: true,
        quest: mockQuest,
        showChoices: false,
      }
    });

    const rewardItems = wrapper.findAll('.rp-reward-item');
    expect(rewardItems.length).toBe(3);
    
    expect(rewardItems[0].text()).toContain('XP');
    expect(rewardItems[1].text()).toContain('Knowledge'); 
    expect(rewardItems[2].text()).toContain('Skills');
    
    // Check icons
    expect(rewardItems[0].find('.rp-reward-icon').text()).toBe('🎯');
    expect(rewardItems[1].find('.rp-reward-icon').text()).toBe('📚');
    expect(rewardItems[2].find('.rp-reward-icon').text()).toBe('⚡');
  });

  it('should display quest steps preview', () => {
    const wrapper = mount(QuestRewardPanel, {
      props: {
        visible: true,
        quest: mockQuest,
        showChoices: false,
      }
    });

    const stepItems = wrapper.findAll('.rp-step-item');
    expect(stepItems.length).toBe(3);
    
    expect(stepItems[0].text()).toContain('1');
    expect(stepItems[0].text()).toContain('Learn basics');
    expect(stepItems[1].text()).toContain('2');
    expect(stepItems[1].text()).toContain('Practice');
    expect(stepItems[2].text()).toContain('3');
    expect(stepItems[2].text()).toContain('Master it');
  });

  it('should show more steps indicator when quest has many steps', () => {
    const questWithManySteps: SkillNode = {
      ...mockQuest,
      questSteps: [
        { label: 'Step 1', action: 'info' },
        { label: 'Step 2', action: 'info' },
        { label: 'Step 3', action: 'info' },
        { label: 'Step 4', action: 'info' },
        { label: 'Step 5', action: 'info' },
      ]
    };

    const wrapper = mount(QuestRewardPanel, {
      props: {
        visible: true,
        quest: questWithManySteps,
        showChoices: false,
      }
    });

    expect(wrapper.text()).toContain('+2 more steps');
    expect(wrapper.findAll('.rp-step-item').length).toBe(3); // Only shows first 3
  });

  it('should show choices when showChoices is true', async () => {
    const wrapper = mount(QuestRewardPanel, {
      props: {
        visible: true,
        quest: mockQuest,
        showChoices: true,
        choiceQuestion: 'Ready to start?',
        choices: mockChoices,
      }
    });

    expect(wrapper.find('.rp-choices').exists()).toBe(true);
    expect(wrapper.text()).toContain('Ready to start?');
    
    const choiceButtons = wrapper.findAll('.rp-choice-btn');
    expect(choiceButtons.length).toBe(2);
    expect(choiceButtons[0].text()).toBe('Yes');
    expect(choiceButtons[1].text()).toBe('No');
  });

  it('should emit close event when close button is clicked', async () => {
    const wrapper = mount(QuestRewardPanel, {
      props: {
        visible: true,
        quest: mockQuest,
        showChoices: false,
      }
    });

    await wrapper.find('.rp-close').trigger('click');
    expect(wrapper.emitted('close')).toHaveLength(1);
  });

  it('should emit choice event when choice button is clicked', async () => {
    const wrapper = mount(QuestRewardPanel, {
      props: {
        visible: true,
        quest: mockQuest,
        showChoices: true,
        choiceQuestion: 'Ready to start?',
        choices: mockChoices,
      }
    });

    const yesButton = wrapper.find('.rp-choice-btn.primary');
    await yesButton.trigger('click');
    
    expect(wrapper.emitted('choice')).toHaveLength(1);
    expect(wrapper.emitted('choice')![0]).toEqual(['accept']);
  });

  it('should handle quest without rewards gracefully', () => {
    const questNoRewards: SkillNode = {
      ...mockQuest,
      rewards: [],
      rewardIcons: [],
    };

    const wrapper = mount(QuestRewardPanel, {
      props: {
        visible: true,
        quest: questNoRewards,
        showChoices: false,
      }
    });

    expect(wrapper.find('.rp-rewards').exists()).toBe(false);
  });

  it('should handle quest without steps gracefully', () => {
    const questNoSteps: SkillNode = {
      ...mockQuest,
      questSteps: [],
    };

    const wrapper = mount(QuestRewardPanel, {
      props: {
        visible: true,
        quest: questNoSteps,
        showChoices: false,
      }
    });

    expect(wrapper.find('.rp-steps').exists()).toBe(false);
  });

  it('should apply primary/secondary button classes correctly', () => {
    const wrapper = mount(QuestRewardPanel, {
      props: {
        visible: true,
        quest: mockQuest,
        showChoices: true,
        choiceQuestion: 'Ready?',
        choices: mockChoices,
      }
    });

    const buttons = wrapper.findAll('.rp-choice-btn');
    expect(buttons[0].classes()).toContain('primary');
    expect(buttons[1].classes()).toContain('secondary');
  });
});