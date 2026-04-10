import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import TypingIndicator from './TypingIndicator.vue';

describe('TypingIndicator', () => {
  it('renders the typing indicator container', () => {
    const wrapper = mount(TypingIndicator);
    expect(wrapper.find('.typing-indicator').exists()).toBe(true);
  });

  it('renders exactly three dots', () => {
    const wrapper = mount(TypingIndicator);
    const dots = wrapper.findAll('.dot');
    expect(dots).toHaveLength(3);
  });

  it('each dot is a span element', () => {
    const wrapper = mount(TypingIndicator);
    const dots = wrapper.findAll('.dot');
    dots.forEach((dot) => {
      expect(dot.element.tagName).toBe('SPAN');
    });
  });
});
