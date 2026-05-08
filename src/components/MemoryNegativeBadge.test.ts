import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import MemoryNegativeBadge from './MemoryNegativeBadge.vue';

describe('MemoryNegativeBadge', () => {
  it('renders badge when cognitiveKind is negative', () => {
    const wrapper = mount(MemoryNegativeBadge, {
      props: { cognitiveKind: 'negative' },
    });
    expect(wrapper.find('.negative-badge').exists()).toBe(true);
    expect(wrapper.text()).toContain('Negative');
  });

  it('hides badge for non-negative kinds', () => {
    const wrapper = mount(MemoryNegativeBadge, {
      props: { cognitiveKind: 'episodic' },
    });
    expect(wrapper.find('.negative-badge').exists()).toBe(false);
  });

  it('shows trigger count when triggers provided', () => {
    const wrapper = mount(MemoryNegativeBadge, {
      props: {
        cognitiveKind: 'negative',
        triggers: ['pattern1', 'pattern2'],
      },
    });
    expect(wrapper.text()).toContain('2 triggers');
  });

  it('shows singular trigger label for one trigger', () => {
    const wrapper = mount(MemoryNegativeBadge, {
      props: {
        cognitiveKind: 'negative',
        triggers: ['only-one'],
      },
    });
    expect(wrapper.text()).toContain('1 trigger');
    expect(wrapper.text()).not.toContain('triggers');
  });

  it('hides trigger count when no triggers', () => {
    const wrapper = mount(MemoryNegativeBadge, {
      props: { cognitiveKind: 'negative' },
    });
    expect(wrapper.find('.trigger-count').exists()).toBe(false);
  });

  it('sets tooltip with trigger list', () => {
    const wrapper = mount(MemoryNegativeBadge, {
      props: {
        cognitiveKind: 'negative',
        triggers: ['unwrap()', 'todo!()'],
      },
    });
    expect(wrapper.attributes('title')).toContain('unwrap()');
    expect(wrapper.attributes('title')).toContain('todo!()');
  });
});
