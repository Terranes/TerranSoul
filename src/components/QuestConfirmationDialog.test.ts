import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import QuestConfirmationDialog from './QuestConfirmationDialog.vue';
import type { SkillNode } from '../stores/skill-tree';

const mockQuest: SkillNode = {
  id: 'test-quest',
  name: 'Test Quest',
  tagline: 'A test quest for validation',
  description: 'This is a comprehensive test quest to validate the dialog functionality.',
  icon: '⚔️',
  category: 'brain',
  tier: 'advanced',
  requires: [],
  questSteps: [
    { label: 'Step 1: Initialize', action: 'info', target: undefined },
    { label: 'Step 2: Configure', action: 'navigate', target: 'settings' },
    { label: 'Step 3: Test', action: 'configure', target: undefined },
    { label: 'Step 4: Validate', action: 'info', target: undefined },
  ],
  rewards: ['XP Boost', 'Achievement Badge'],
  rewardIcons: ['🎯', '🏆'],
  combos: [],
};

describe('QuestConfirmationDialog', () => {
  it('renders when visible with quest data', () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: mockQuest,
      },
    });

    expect(wrapper.find('.quest-confirm-dialog').exists()).toBe(true);
    expect(wrapper.text()).toContain('Test Quest');
    expect(wrapper.text()).toContain('A test quest for validation');
    expect(wrapper.text()).toContain('This is a comprehensive test quest');
  });

  it('does not render when not visible', () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: false,
        quest: mockQuest,
      },
    });

    expect(wrapper.find('.quest-confirm-dialog').exists()).toBe(false);
  });

  it('renders quest steps when provided', () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: mockQuest,
      },
    });

    expect(wrapper.text()).toContain('What you\'ll do:');
    expect(wrapper.text()).toContain('Step 1: Initialize');
    expect(wrapper.text()).toContain('Step 2: Configure');
    expect(wrapper.text()).toContain('Step 3: Test');
    // Should show "1 more steps" since we limit to 3 steps
    expect(wrapper.text()).toContain('1 more steps');
  });

  it('renders rewards when provided', () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: mockQuest,
      },
    });

    expect(wrapper.text()).toContain('Rewards:');
    expect(wrapper.text()).toContain('🎯 XP Boost');
    expect(wrapper.text()).toContain('🏆 Achievement Badge');
  });

  it('shows AI guidance notice', () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: mockQuest,
      },
    });

    expect(wrapper.text()).toContain('AI-Guided Quest');
    expect(wrapper.text()).toContain('step-by-step guidance');
    expect(wrapper.text()).toContain('multiple-choice questions');
    expect(wrapper.text()).toContain('No manual configuration needed');
  });

  it('emits accept when Accept Quest button is clicked', async () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: mockQuest,
      },
    });

    const acceptBtn = wrapper.find('.qcd-btn-primary');
    expect(acceptBtn.text()).toContain('Accept Quest');
    
    await acceptBtn.trigger('click');
    expect(wrapper.emitted('accept')).toBeTruthy();
  });

  it('emits cancel when Not Now button is clicked', async () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: mockQuest,
      },
    });

    const cancelBtn = wrapper.find('.qcd-btn-secondary');
    expect(cancelBtn.text()).toContain('Not Now');
    
    await cancelBtn.trigger('click');
    expect(wrapper.emitted('cancel')).toBeTruthy();
  });

  it('emits cancel when close button is clicked', async () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: mockQuest,
      },
    });

    const closeBtn = wrapper.find('.qcd-close');
    await closeBtn.trigger('click');
    expect(wrapper.emitted('cancel')).toBeTruthy();
  });

  it('emits cancel when backdrop is clicked', async () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: mockQuest,
      },
    });

    const backdrop = wrapper.find('.quest-confirm-backdrop');
    await backdrop.trigger('click');
    expect(wrapper.emitted('cancel')).toBeTruthy();
  });

  it('handles quest with no steps or rewards gracefully', () => {
    const simpleQuest: SkillNode = {
      id: 'simple-quest',
      name: 'Simple Quest',
      tagline: 'A minimal quest',
      description: 'A quest with minimal data.',
      icon: '🎯',
      category: 'utility',
      tier: 'foundation',
      requires: [],
      questSteps: [],
      rewards: [],
      rewardIcons: [],
      combos: [],
    };

    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: simpleQuest,
      },
    });

    expect(wrapper.find('.quest-confirm-dialog').exists()).toBe(true);
    expect(wrapper.text()).toContain('Simple Quest');
    // Should not show steps or rewards sections
    expect(wrapper.text()).not.toContain('What you\'ll do:');
    expect(wrapper.text()).not.toContain('Rewards:');
  });

  it('handles null quest gracefully', () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: null,
      },
    });

    expect(wrapper.find('.quest-confirm-dialog').exists()).toBe(true);
    expect(wrapper.text()).toContain('Quest Available');
    expect(wrapper.text()).toContain('Ready to embark on this journey?');
  });

  it('displays correct quest icon from props', () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: mockQuest,
      },
    });

    const icon = wrapper.find('.qcd-icon');
    expect(icon.text()).toBe('⚔️');
  });

  it('shows truncated steps list when more than 3 steps', () => {
    const questWithManySteps: SkillNode = {
      ...mockQuest,
      questSteps: [
        { label: 'Step 1', action: 'info', target: undefined },
        { label: 'Step 2', action: 'info', target: undefined },
        { label: 'Step 3', action: 'info', target: undefined },
        { label: 'Step 4', action: 'info', target: undefined },
        { label: 'Step 5', action: 'info', target: undefined },
      ],
    };

    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: questWithManySteps,
      },
    });

    expect(wrapper.text()).toContain('Step 1');
    expect(wrapper.text()).toContain('Step 2');
    expect(wrapper.text()).toContain('Step 3');
    expect(wrapper.text()).toContain('2 more steps'); // 5 total - 3 shown = 2 more
  });

  it('applies proper CSS classes for styling', () => {
    const wrapper = mount(QuestConfirmationDialog, {
      props: {
        visible: true,
        quest: mockQuest,
      },
    });

    expect(wrapper.find('.quest-confirm-backdrop').exists()).toBe(true);
    expect(wrapper.find('.quest-confirm-dialog').exists()).toBe(true);
    expect(wrapper.find('.qcd-header').exists()).toBe(true);
    expect(wrapper.find('.qcd-content').exists()).toBe(true);
    expect(wrapper.find('.qcd-actions').exists()).toBe(true);
    expect(wrapper.find('.qcd-ai-notice').exists()).toBe(true);
  });
});