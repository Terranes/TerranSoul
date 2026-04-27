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

  // ── Markdown rendering tests ──

  it('strips bold markers from text', () => {
    const messages = [makeMessage({ content: 'This is **bold** text' })];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    const bubble = wrapper.find('.bubble');
    expect(bubble.text()).toContain('This is bold text');
    expect(bubble.html()).not.toContain('**');
  });

  it('strips italic markers from text', () => {
    const messages = [makeMessage({ content: 'This is *italic* text' })];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    const bubble = wrapper.find('.bubble');
    expect(bubble.text()).toContain('This is italic text');
    expect(bubble.html()).not.toContain('<em>');
  });

  it('renders inline code with <code> tags', () => {
    const messages = [makeMessage({ content: 'Run `npm test` here' })];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    const bubble = wrapper.find('.bubble');
    expect(bubble.html()).toContain('<code class="md-inline-code">npm test</code>');
  });

  it('renders code blocks with <pre><code> tags', () => {
    const messages = [makeMessage({ content: '```js\nconst x = 1;\n```' })];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    const bubble = wrapper.find('.bubble');
    expect(bubble.html()).toContain('<pre class="md-code-block">');
    expect(bubble.html()).toContain('const x = 1;');
  });

  it('escapes HTML in message content', () => {
    const messages = [makeMessage({ content: '<script>alert("xss")</script>' })];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    const bubble = wrapper.find('.bubble');
    expect(bubble.html()).not.toContain('<script>');
    expect(bubble.html()).toContain('&lt;script&gt;');
  });

  // ── Welcome screen & suggestion tests ──

  it('shows welcome state with quest buttons when no messages', () => {
    const wrapper = mount(ChatMessageList, {
      props: { messages: [], isThinking: false },
    });
    expect(wrapper.find('.welcome-state').exists()).toBe(true);
    expect(wrapper.find('.welcome-title').text()).toBe('Welcome to TerranSoul');
    expect(wrapper.findAll('.welcome-quest-btn').length).toBe(3);
  });

  it('emits startQuest when Start First Quest clicked', async () => {
    const wrapper = mount(ChatMessageList, {
      props: { messages: [], isThinking: false },
    });
    const btn = wrapper.find('.welcome-quest-btn.primary');
    await btn.trigger('click');
    expect(wrapper.emitted('startQuest')).toBeTruthy();
  });

  it('hides welcome state when messages exist', () => {
    const messages = [makeMessage()];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    expect(wrapper.find('.welcome-state').exists()).toBe(false);
  });

  // ── Quest choice rendering moved to QuestChoiceOverlay ──

  it('does NOT render inline quest-choice buttons (moved to hot-seat overlay)', () => {
    const messages = [
      makeMessage({
        role: 'assistant',
        content: 'A quest!',
        questId: 'tts',
        questChoices: [
          { label: 'Accept', value: 'accept', icon: '⚔️' },
          { label: 'Dismiss', value: 'dismiss', icon: '💤' },
        ],
      }),
    ];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    expect(wrapper.find('.quest-choice-btn').exists()).toBe(false);
    expect(wrapper.find('.quest-choices').exists()).toBe(false);
  });

  it('does not render quest choices when message has no questChoices', () => {
    const messages = [makeMessage({ role: 'assistant', content: 'Hello' })];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    expect(wrapper.find('.quest-choices').exists()).toBe(false);
  });

  it('shows quick replies when assistant ends with a yes/no question (default policy)', async () => {
    const { setActivePinia, createPinia } = await import('pinia');
    setActivePinia(createPinia());
    const messages = [
      makeMessage({
        role: 'assistant',
        content: 'Looks like you have everything you need. Shall we?',
      }),
    ];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    expect(wrapper.find('.quick-replies').exists()).toBe(true);
  });

  it('hides quick replies when quickRepliesEnabled is false', async () => {
    const { setActivePinia, createPinia } = await import('pinia');
    setActivePinia(createPinia());
    const { useAiDecisionPolicyStore } = await import('../stores/ai-decision-policy');
    useAiDecisionPolicyStore().policy.quickRepliesEnabled = false;
    const messages = [
      makeMessage({
        role: 'assistant',
        content: 'Looks like you have everything you need. Shall we?',
      }),
    ];
    const wrapper = mount(ChatMessageList, {
      props: { messages, isThinking: false },
    });
    expect(wrapper.find('.quick-replies').exists()).toBe(false);
  });
});
