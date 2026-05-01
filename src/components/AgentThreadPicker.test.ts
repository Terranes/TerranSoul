import { describe, it, expect, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { setActivePinia, createPinia } from 'pinia';
import AgentThreadPicker from './AgentThreadPicker.vue';
import { useAgentRosterStore } from '../stores/agent-roster';
import type { Message } from '../types';

function makeMsg(overrides: Partial<Message>): Message {
  return {
    id: overrides.id ?? `m-${Math.random().toString(36).slice(2)}`,
    role: overrides.role ?? 'assistant',
    content: overrides.content ?? 'hi',
    timestamp: overrides.timestamp ?? Date.now(),
    ...overrides,
  } as Message;
}

describe('AgentThreadPicker (Chunk 23.3)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders nothing when no messages have an agentId', () => {
    const wrapper = mount(AgentThreadPicker, {
      props: {
        messages: [
          makeMsg({ id: '1', role: 'user', content: 'hello' }),
          makeMsg({ id: '2', role: 'assistant', content: 'hi back' }),
        ],
        currentAgent: 'auto',
      },
    });
    expect(wrapper.find('.agent-thread-picker').exists()).toBe(false);
  });

  it('renders an "All" pill plus one pill per unique agentId', () => {
    const wrapper = mount(AgentThreadPicker, {
      props: {
        messages: [
          makeMsg({ id: '1', agentId: 'coder' }),
          makeMsg({ id: '2', agentId: 'coder' }),
          makeMsg({ id: '3', agentId: 'planner' }),
        ],
        currentAgent: 'auto',
      },
    });
    const pills = wrapper.findAll('.agent-thread-pill');
    // "All" + coder + planner = 3
    expect(pills).toHaveLength(3);
    expect(pills[0].text()).toContain('All');
  });

  it('shows the message count badge per agent pill', () => {
    const wrapper = mount(AgentThreadPicker, {
      props: {
        messages: [
          makeMsg({ id: '1', agentId: 'coder' }),
          makeMsg({ id: '2', agentId: 'coder' }),
          makeMsg({ id: '3', agentId: 'coder' }),
        ],
        currentAgent: 'auto',
      },
    });
    const counts = wrapper.findAll('.agent-thread-count');
    expect(counts).toHaveLength(1);
    expect(counts[0].text()).toBe('3');
  });

  it('marks the active pill when currentAgent matches', () => {
    const wrapper = mount(AgentThreadPicker, {
      props: {
        messages: [makeMsg({ id: '1', agentId: 'coder' })],
        currentAgent: 'coder',
      },
    });
    const pills = wrapper.findAll('.agent-thread-pill');
    // First pill is "All", second is coder
    expect(pills[0].classes()).not.toContain('active');
    expect(pills[1].classes()).toContain('active');
    expect(pills[1].attributes('aria-selected')).toBe('true');
  });

  it('marks the "All" pill active when currentAgent is "auto"', () => {
    const wrapper = mount(AgentThreadPicker, {
      props: {
        messages: [makeMsg({ id: '1', agentId: 'coder' })],
        currentAgent: 'auto',
      },
    });
    const pills = wrapper.findAll('.agent-thread-pill');
    expect(pills[0].classes()).toContain('active');
    expect(pills[1].classes()).not.toContain('active');
  });

  it('emits "pick" with the agent ID when an agent pill is clicked', async () => {
    const wrapper = mount(AgentThreadPicker, {
      props: {
        messages: [
          makeMsg({ id: '1', agentId: 'coder' }),
          makeMsg({ id: '2', agentId: 'planner' }),
        ],
        currentAgent: 'auto',
      },
    });
    const pills = wrapper.findAll('.agent-thread-pill');
    // Sorted alphabetically: All, coder, planner
    await pills[1].trigger('click');
    const events = wrapper.emitted('pick');
    expect(events).toBeTruthy();
    expect(events![0]).toEqual(['coder']);
  });

  it('emits "pick" with "auto" when the All pill is clicked', async () => {
    const wrapper = mount(AgentThreadPicker, {
      props: {
        messages: [makeMsg({ id: '1', agentId: 'coder' })],
        currentAgent: 'coder',
      },
    });
    const allPill = wrapper.find('.agent-thread-pill');
    await allPill.trigger('click');
    const events = wrapper.emitted('pick');
    expect(events).toBeTruthy();
    expect(events![0]).toEqual(['auto']);
  });

  it('uses display_name from the agent roster when available', () => {
    const roster = useAgentRosterStore();
    roster.agents = [
      {
        id: 'coder',
        display_name: 'Code Wizard',
        model: 'gemma3:4b',
        provider: 'ollama',
        capabilities: [],
        ram_gb: 4,
      } as never,
    ];
    const wrapper = mount(AgentThreadPicker, {
      props: {
        messages: [makeMsg({ id: '1', agentId: 'coder' })],
        currentAgent: 'auto',
      },
    });
    const pills = wrapper.findAll('.agent-thread-pill');
    expect(pills[1].text()).toContain('Code Wizard');
  });

  it('falls back to agentId when no roster entry exists', () => {
    const wrapper = mount(AgentThreadPicker, {
      props: {
        messages: [makeMsg({ id: '1', agentId: 'unknown-agent' })],
        currentAgent: 'auto',
      },
    });
    const pills = wrapper.findAll('.agent-thread-pill');
    expect(pills[1].text()).toContain('unknown-agent');
  });
});
