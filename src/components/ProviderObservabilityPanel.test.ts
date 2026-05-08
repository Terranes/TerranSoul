import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import ProviderObservabilityPanel from './ProviderObservabilityPanel.vue';

// Mock @tauri-apps/api/core
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const defaultSummary = {
  healthy_count: 5,
  rate_limited_count: 1,
  unhealthy_count: 0,
  all_exhausted: false,
  recent_events: [],
  selected_provider_id: 'groq',
};

const defaultPolicy = {
  max_attempts: 3,
  respect_privacy: true,
  min_cooldown_secs: 60,
};

const defaultResolved = {
  source: 'brain_mode:local_ollama',
  provider_id: 'ollama',
  model: 'gemma3:4b',
  base_url: 'http://localhost:11434',
  api_key: '',
  max_tokens: null,
};

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === 'get_failover_summary') return Promise.resolve(defaultSummary);
    if (cmd === 'get_failover_policy') return Promise.resolve(defaultPolicy);
    if (cmd === 'resolve_provider_for_task') return Promise.resolve(defaultResolved);
    if (cmd === 'get_agent_routing') return Promise.resolve([]);
    if (cmd === 'health_check_providers') return Promise.resolve(null);
    if (cmd === 'set_failover_policy') return Promise.resolve(defaultPolicy);
    return Promise.resolve(null);
  });
});

describe('ProviderObservabilityPanel', () => {
  it('renders panel with title', async () => {
    const wrapper = mount(ProviderObservabilityPanel);
    await flushPromises();
    expect(wrapper.find('[data-testid="provider-observability-panel"]').exists()).toBe(true);
    expect(wrapper.find('.po-title').text()).toBe('Provider Status');
  });

  it('displays summary stats from backend', async () => {
    const wrapper = mount(ProviderObservabilityPanel);
    await flushPromises();
    const stats = wrapper.findAll('.po-stat-num');
    expect(stats[0].text()).toBe('5'); // healthy
    expect(stats[1].text()).toBe('1'); // rate-limited
    expect(stats[2].text()).toBe('0'); // unhealthy
    expect(stats[3].text()).toBe('groq'); // active
  });

  it('displays per-task model resolutions', async () => {
    const wrapper = mount(ProviderObservabilityPanel);
    await flushPromises();
    const rows = wrapper.findAll('.po-task-row');
    // Should have 6 task kinds
    expect(rows.length).toBeGreaterThanOrEqual(6);
    expect(rows[0].find('.po-task-kind').text()).toBe('Chat');
    expect(rows[0].find('.po-task-provider').text()).toBe('ollama');
    expect(rows[0].find('.po-task-model').text()).toBe('gemma3:4b');
  });

  it('shows recent failover events when present', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_failover_summary') {
        return Promise.resolve({
          ...defaultSummary,
          recent_events: [
            { provider_id: 'groq', reason: 'rate_limit', timestamp_ms: Date.now() },
            { provider_id: 'cerebras', reason: 'unhealthy', timestamp_ms: Date.now() - 5000 },
          ],
        });
      }
      if (cmd === 'get_failover_policy') return Promise.resolve(defaultPolicy);
      if (cmd === 'resolve_provider_for_task') return Promise.resolve(defaultResolved);
      if (cmd === 'get_agent_routing') return Promise.resolve([]);
      return Promise.resolve(null);
    });
    const wrapper = mount(ProviderObservabilityPanel);
    await flushPromises();
    const events = wrapper.findAll('.po-event');
    expect(events.length).toBe(2);
    expect(events[0].find('.po-event-provider').text()).toBe('groq');
  });

  it('shows all-exhausted alert', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_failover_summary') {
        return Promise.resolve({ ...defaultSummary, all_exhausted: true });
      }
      if (cmd === 'get_failover_policy') return Promise.resolve(defaultPolicy);
      if (cmd === 'resolve_provider_for_task') return Promise.resolve(defaultResolved);
      if (cmd === 'get_agent_routing') return Promise.resolve([]);
      return Promise.resolve(null);
    });
    const wrapper = mount(ProviderObservabilityPanel);
    await flushPromises();
    expect(wrapper.find('.po-alert').exists()).toBe(true);
  });

  it('calls set_failover_policy on field change', async () => {
    const wrapper = mount(ProviderObservabilityPanel);
    await flushPromises();
    const input = wrapper.find('input[type="number"]');
    await input.setValue(5);
    await input.trigger('change');
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith('set_failover_policy', { max_attempts: 5 });
  });

  it('calls health_check_providers on refresh', async () => {
    const wrapper = mount(ProviderObservabilityPanel);
    await flushPromises();
    await wrapper.find('.po-refresh-btn').trigger('click');
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith('health_check_providers');
  });
});
