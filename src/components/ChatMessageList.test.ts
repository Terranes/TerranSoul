import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import ChatMessageList from './ChatMessageList.vue';
import type { Message } from '../types';

const makeMessage = (overrides: Partial<Message> = {}): Message => ({
  id: crypto.randomUUID(),
  role: 'user',
  content: 'Hello',
  timestamp: Date.now(),
  ...overrides,
});

describe('ChatMessageList', () => {
  it('renders an empty list when no messages', () => {
    const wrapper = mount(ChatMessageList, {
      props: { messages: [], isThinking: false },
    });
    expect(wrapper.findAll('.message-row')).toHaveLength(0);
  });

  it('renders user messages with user class', () => {
    const messages = [makeMessage({ role: 'user', content: 'Hi!' })];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    const rows = wrapper.findAll('.message-row');
    expect(rows).toHaveLength(1);
    expect(rows[0].classes()).toContain('user');
  });

  it('renders assistant messages with assistant class', () => {
    const messages = [
      makeMessage({ role: 'assistant', content: 'Hello!', agentName: 'Stub' }),
    ];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    const rows = wrapper.findAll('.message-row');
    expect(rows).toHaveLength(1);
    expect(rows[0].classes()).toContain('assistant');
  });

  it('renders message content in bubbles', () => {
    const messages = [makeMessage({ content: 'Test message content' })];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    expect(wrapper.find('.bubble').text()).toBe('Test message content');
  });

  it('renders multiple messages in order', () => {
    const messages = [
      makeMessage({ role: 'user', content: 'First' }),
      makeMessage({ role: 'assistant', content: 'Second', agentName: 'Stub' }),
      makeMessage({ role: 'user', content: 'Third' }),
    ];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    const bubbles = wrapper.findAll('.bubble');
    expect(bubbles).toHaveLength(3);
    expect(bubbles[0].text()).toBe('First');
    expect(bubbles[1].text()).toBe('Second');
    expect(bubbles[2].text()).toBe('Third');
  });

  it('shows TypingIndicator when isThinking is true', () => {
    const wrapper = mount(ChatMessageList, {
      props: { messages: [], isThinking: true },
    });
    expect(wrapper.find('.typing-indicator').exists()).toBe(true);
  });

  it('hides TypingIndicator when isThinking is false', () => {
    const wrapper = mount(ChatMessageList, {
      props: { messages: [], isThinking: false },
    });
    expect(wrapper.find('.typing-indicator').exists()).toBe(false);
  });

  it('shows AgentBadge for assistant messages', () => {
    const messages = [
      makeMessage({ role: 'assistant', content: 'Hi', agentName: 'OpenClaw' }),
    ];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    expect(wrapper.find('.agent-badge').exists()).toBe(true);
    expect(wrapper.find('.agent-badge').text()).toBe('OpenClaw');
  });

  it('does not show AgentBadge for user messages', () => {
    const messages = [makeMessage({ role: 'user', content: 'Hi' })];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    expect(wrapper.find('.agent-badge').exists()).toBe(false);
  });

  it('shows default agent name when agentName is not set', () => {
    const messages = [
      makeMessage({ role: 'assistant', content: 'Hi' }),
    ];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    expect(wrapper.find('.agent-badge').text()).toBe('TerranSoul');
  });

  it('formats timestamps', () => {
    const messages = [makeMessage({ timestamp: new Date('2026-04-10T12:30:00').getTime() })];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    const timestamp = wrapper.find('.timestamp');
    expect(timestamp.exists()).toBe(true);
    expect(timestamp.text()).toBeTruthy();
  });
});
