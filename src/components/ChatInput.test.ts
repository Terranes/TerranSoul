import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import ChatInput from './ChatInput.vue';

describe('ChatInput', () => {
  it('renders the input field and send button', () => {
    const wrapper = mount(ChatInput, { props: { disabled: false } });
    expect(wrapper.find('.chat-input').exists()).toBe(true);
    expect(wrapper.find('.send-btn').exists()).toBe(true);
  });

  it('disables input and button when disabled prop is true', () => {
    const wrapper = mount(ChatInput, { props: { disabled: true } });
    const input = wrapper.find('.chat-input').element as HTMLInputElement;
    expect(input.disabled).toBe(true);
    const button = wrapper.find('.send-btn').element as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  it('send button is disabled when input is empty', () => {
    const wrapper = mount(ChatInput, { props: { disabled: false } });
    const button = wrapper.find('.send-btn').element as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  it('send button is enabled when input has text', async () => {
    const wrapper = mount(ChatInput, { props: { disabled: false } });
    await wrapper.find('.chat-input').setValue('Hello');
    const button = wrapper.find('.send-btn').element as HTMLButtonElement;
    expect(button.disabled).toBe(false);
  });

  it('emits submit event with trimmed message on form submit', async () => {
    const wrapper = mount(ChatInput, { props: { disabled: false } });
    await wrapper.find('.chat-input').setValue('  Hello TerranSoul  ');
    await wrapper.find('form').trigger('submit');
    expect(wrapper.emitted('submit')).toBeTruthy();
    expect(wrapper.emitted('submit')![0]).toEqual(['Hello TerranSoul']);
  });

  it('clears input after successful submit', async () => {
    const wrapper = mount(ChatInput, { props: { disabled: false } });
    await wrapper.find('.chat-input').setValue('Test message');
    await wrapper.find('form').trigger('submit');
    const input = wrapper.find('.chat-input').element as HTMLInputElement;
    expect(input.value).toBe('');
  });

  it('does not emit submit when disabled', async () => {
    const wrapper = mount(ChatInput, { props: { disabled: true } });
    await wrapper.find('.chat-input').setValue('Hello');
    await wrapper.find('form').trigger('submit');
    expect(wrapper.emitted('submit')).toBeFalsy();
  });

  it('does not emit submit for whitespace-only input', async () => {
    const wrapper = mount(ChatInput, { props: { disabled: false } });
    await wrapper.find('.chat-input').setValue('   ');
    await wrapper.find('form').trigger('submit');
    expect(wrapper.emitted('submit')).toBeFalsy();
  });

  it('has correct placeholder text', () => {
    const wrapper = mount(ChatInput, { props: { disabled: false } });
    const input = wrapper.find('.chat-input').element as HTMLInputElement;
    expect(input.placeholder).toBe('Type a message…');
  });
});
