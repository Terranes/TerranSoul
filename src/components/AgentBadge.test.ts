import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import AgentBadge from './AgentBadge.vue';

describe('AgentBadge', () => {
  it('renders the agent name', () => {
    const wrapper = mount(AgentBadge, { props: { name: 'TerranSoul' } });
    expect(wrapper.text()).toBe('TerranSoul');
  });

  it('applies the agent-badge class', () => {
    const wrapper = mount(AgentBadge, { props: { name: 'Stub' } });
    expect(wrapper.find('.agent-badge').exists()).toBe(true);
  });

  it('renders different agent names', () => {
    const wrapper = mount(AgentBadge, { props: { name: 'OpenClaw' } });
    expect(wrapper.text()).toBe('OpenClaw');
  });
});
