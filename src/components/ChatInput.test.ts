import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia } from 'pinia';
import ChatInput from './ChatInput.vue';

function mountChatInput(props = { disabled: false }) {
  return mount(ChatInput, {
    props,
    global: { plugins: [createPinia()] },
  });
}

describe('ChatInput', () => {
  it('renders the input field and send button', () => {
    const wrapper = mountChatInput();
    expect(wrapper.find('.chat-input').exists()).toBe(true);
    expect(wrapper.find('.send-btn').exists()).toBe(true);
  });

  it('disables send button but keeps input enabled when disabled prop is true', () => {
    const wrapper = mountChatInput({ disabled: true });
    const input = wrapper.find('.chat-input').element as HTMLInputElement;
    expect(input.disabled).toBe(false);
    const button = wrapper.find('.send-btn').element as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  it('send button is disabled when input is empty', () => {
    const wrapper = mountChatInput();
    const button = wrapper.find('.send-btn').element as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  it('send button is enabled when input has text', async () => {
    const wrapper = mountChatInput();
    await wrapper.find('.chat-input').setValue('Hello');
    const button = wrapper.find('.send-btn').element as HTMLButtonElement;
    expect(button.disabled).toBe(false);
  });

  it('emits submit event with trimmed message on form submit', async () => {
    const wrapper = mountChatInput();
    await wrapper.find('.chat-input').setValue('  Hello TerranSoul  ');
    await wrapper.find('form').trigger('submit');
    expect(wrapper.emitted('submit')).toBeTruthy();
    expect(wrapper.emitted('submit')![0]).toEqual(['Hello TerranSoul']);
  });

  it('clears input after successful submit', async () => {
    const wrapper = mountChatInput();
    await wrapper.find('.chat-input').setValue('Test message');
    await wrapper.find('form').trigger('submit');
    const input = wrapper.find('.chat-input').element as HTMLInputElement;
    expect(input.value).toBe('');
  });

  it('does not emit submit when disabled', async () => {
    const wrapper = mountChatInput({ disabled: true });
    await wrapper.find('.chat-input').setValue('Hello');
    await wrapper.find('form').trigger('submit');
    expect(wrapper.emitted('submit')).toBeFalsy();
  });

  it('does not emit submit for whitespace-only input', async () => {
    const wrapper = mountChatInput();
    await wrapper.find('.chat-input').setValue('   ');
    await wrapper.find('form').trigger('submit');
    expect(wrapper.emitted('submit')).toBeFalsy();
  });

  it('has correct placeholder text', () => {
    const wrapper = mountChatInput();
    const input = wrapper.find('.chat-input').element as HTMLTextAreaElement;
    // Placeholder includes a Shift+Enter hint since the input is now a
    // multi-line auto-grow textarea (see ChatInput.vue).
    expect(input.placeholder).toContain('Type a message');
    expect(input.placeholder).toContain('Shift+Enter');
  });
});
