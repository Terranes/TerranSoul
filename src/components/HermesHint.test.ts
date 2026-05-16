import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import HermesHint from './HermesHint.vue';

describe('HermesHint.vue', () => {
  it('renders when visible with deep_research intent', () => {
    const wrapper = mount(HermesHint, {
      props: { intent: 'deep_research', visible: true },
    });
    expect(wrapper.find('[data-testid="hermes-hint-banner"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('deep research');
    expect(wrapper.text()).toContain('Hermes Desktop');
  });

  it('renders correct text for long_running_workflow', () => {
    const wrapper = mount(HermesHint, {
      props: { intent: 'long_running_workflow', visible: true },
    });
    expect(wrapper.text()).toContain('long-running workflows');
  });

  it('renders correct text for full_ide_coding', () => {
    const wrapper = mount(HermesHint, {
      props: { intent: 'full_ide_coding', visible: true },
    });
    expect(wrapper.text()).toContain('full-IDE coding sessions');
  });

  it('does not render when visible is false', () => {
    const wrapper = mount(HermesHint, {
      props: { intent: 'deep_research', visible: false },
    });
    expect(wrapper.find('[data-testid="hermes-hint-banner"]').exists()).toBe(false);
  });

  it('emits dismiss when close button clicked', async () => {
    const wrapper = mount(HermesHint, {
      props: { intent: 'deep_research', visible: true },
    });
    await wrapper.find('[data-testid="hermes-hint-dismiss"]').trigger('click');
    expect(wrapper.emitted('dismiss')).toHaveLength(1);
  });

  it('emits navigate when Learn more clicked', async () => {
    const wrapper = mount(HermesHint, {
      props: { intent: 'deep_research', visible: true },
    });
    await wrapper.find('[data-testid="hermes-hint-learn"]').trigger('click');
    expect(wrapper.emitted('navigate')).toHaveLength(1);
  });

  it('hides after dismiss is clicked (internal state)', async () => {
    const wrapper = mount(HermesHint, {
      props: { intent: 'deep_research', visible: true },
    });
    expect(wrapper.find('[data-testid="hermes-hint-banner"]').exists()).toBe(true);
    await wrapper.find('[data-testid="hermes-hint-dismiss"]').trigger('click');
    expect(wrapper.find('[data-testid="hermes-hint-banner"]').exists()).toBe(false);
  });
});
