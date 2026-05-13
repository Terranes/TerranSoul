import { describe, it, expect, vi } from 'vitest';
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

  it('starts as a one-row auto-grow textarea', () => {
    const wrapper = mountChatInput();
    const input = wrapper.find('.chat-input').element as HTMLTextAreaElement;
    expect(input.rows).toBe(1);
  });

  it('swaps to the animated grey "Thinking…" hint when thinking prop is true', async () => {
    vi.useFakeTimers();
    try {
      const wrapper = mount(ChatInput, {
        props: { disabled: false, thinking: true },
        global: { plugins: [createPinia()] },
      });
      const input = wrapper.find('.chat-input').element as HTMLTextAreaElement;
      // Initial render: zero dots.
      expect(input.placeholder).toMatch(
        /^Thinking - Expand chat history for details$/,
      );
      // The textarea must keep the dimmed-italic style cue.
      expect(wrapper.find('.chat-input').classes()).toContain('is-thinking-hint');
      // Tick the dot animation forward — the placeholder text must change
      // without disrupting input or focus.
      vi.advanceTimersByTime(400);
      await wrapper.vm.$nextTick();
      expect(input.placeholder).toContain('Thinking.');
      vi.advanceTimersByTime(400);
      await wrapper.vm.$nextTick();
      expect(input.placeholder).toContain('Thinking..');
      vi.advanceTimersByTime(400);
      await wrapper.vm.$nextTick();
      expect(input.placeholder).toContain('Thinking...');
      // Clears back to the idle prompt when the assistant stops thinking.
      await wrapper.setProps({ disabled: false, thinking: false });
      expect(input.placeholder).toBe('Type a message…  (Shift+Enter for newline)');
      expect(wrapper.find('.chat-input').classes()).not.toContain('is-thinking-hint');
    } finally {
      vi.useRealTimers();
    }
  });

  it('still allows the user to type while the thinking hint is showing', async () => {
    const wrapper = mount(ChatInput, {
      props: { disabled: false, thinking: true },
      global: { plugins: [createPinia()] },
    });
    await wrapper.find('.chat-input').setValue('queued message');
    // Once the user types, the hint class drops away (real text wins).
    expect(wrapper.find('.chat-input').classes()).not.toContain('is-thinking-hint');
    const button = wrapper.find('.send-btn').element as HTMLButtonElement;
    expect(button.disabled).toBe(false);
  });

  it('shows history nav buttons after sending a message', async () => {
    const wrapper = mountChatInput();
    expect(wrapper.find('.history-nav').exists()).toBe(false);
    await wrapper.find('.chat-input').setValue('First message');
    await wrapper.find('form').trigger('submit');
    expect(wrapper.find('.history-nav').exists()).toBe(true);
  });

  it('recalls previous message on up arrow and restores draft on down arrow', async () => {
    const wrapper = mountChatInput();
    const textarea = wrapper.find('.chat-input');

    // Send two messages
    await textarea.setValue('Hello');
    await wrapper.find('form').trigger('submit');
    await textarea.setValue('World');
    await wrapper.find('form').trigger('submit');

    // Type a draft
    await textarea.setValue('draft');

    // Press up arrow — should recall "World"
    await textarea.trigger('keydown', { key: 'ArrowUp' });
    expect((textarea.element as HTMLTextAreaElement).value).toBe('World');

    // Press up arrow again — should recall "Hello"
    await textarea.trigger('keydown', { key: 'ArrowUp' });
    expect((textarea.element as HTMLTextAreaElement).value).toBe('Hello');

    // Press down arrow — should go back to "World"
    await textarea.trigger('keydown', { key: 'ArrowDown' });
    expect((textarea.element as HTMLTextAreaElement).value).toBe('World');

    // Press down arrow again — should restore draft
    await textarea.trigger('keydown', { key: 'ArrowDown' });
    expect((textarea.element as HTMLTextAreaElement).value).toBe('draft');
  });

  it('does not duplicate consecutive identical messages in history', async () => {
    const wrapper = mountChatInput();
    const textarea = wrapper.find('.chat-input');

    await textarea.setValue('same');
    await wrapper.find('form').trigger('submit');
    await textarea.setValue('same');
    await wrapper.find('form').trigger('submit');

    // Press up — should get "same"
    await textarea.trigger('keydown', { key: 'ArrowUp' });
    expect((textarea.element as HTMLTextAreaElement).value).toBe('same');

    // Press up again — should still be "same" (only one entry)
    await textarea.trigger('keydown', { key: 'ArrowUp' });
    expect((textarea.element as HTMLTextAreaElement).value).toBe('same');
  });
});
