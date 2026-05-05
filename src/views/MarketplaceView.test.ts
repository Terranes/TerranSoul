/**
 * Integration tests for MarketplaceView.vue.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { setActivePinia, createPinia } from 'pinia';
import MarketplaceView from './MarketplaceView.vue';
import type { AgentSearchResult, InstalledAgentInfo, ConsentInfo } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const sampleAgents: AgentSearchResult[] = [
  {
    name: 'stub-agent',
    version: '1.0.0',
    description: 'Built-in stub agent for testing',
    capabilities: ['chat'],
    homepage: 'https://terranes.dev',
  },
  {
    name: 'file-fetch-agent',
    version: '1.0.0',
    description: 'Capability-gated file and network bridge',
    capabilities: ['chat', 'filesystem', 'network'],
    homepage: null,
  },
];

const sampleInstalled: InstalledAgentInfo[] = [
  {
    name: 'stub-agent',
    version: '1.0.0',
    description: 'Built-in stub agent',
    install_path: '/data/agents/stub-agent',
  },
];

const sampleConsents: ConsentInfo[] = [
  { agent_name: 'stub-agent', capability: 'file_read', granted: true },
];

function setupMocks(opts: {
  agents?: AgentSearchResult[];
  installed?: InstalledAgentInfo[];
  consents?: ConsentInfo[];
} = {}) {
  const agents = opts.agents ?? sampleAgents;
  const installed = opts.installed ?? [];
  const consents = opts.consents ?? [];

  mockInvoke.mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case 'search_agents':
        return agents;
      case 'list_installed_agents':
        return installed;
      case 'list_agent_capabilities':
        return consents;
      case 'install_agent':
        return { name: 'stub-agent', version: '1.0.0', description: 'stub', install_path: '/a' };
      case 'remove_agent':
        return undefined;
      case 'update_agent':
        return { name: 'stub-agent', version: '1.1.0', description: 'stub updated', install_path: '/a' };
      case 'grant_agent_capability':
        return undefined;
      case 'clear_agent_capabilities':
        return undefined;
      case 'check_lm_studio_status':
        return { running: true, model_count: 1, loaded_count: 0 };
      case 'get_lm_studio_models':
        return [{
          key: 'qwen/qwen3-4b',
          display_name: 'Qwen 3 4B',
          publisher: 'qwen',
          type: 'llm',
          size_bytes: 4_000_000_000,
          loaded_instances: [],
        }];
      default:
        throw new Error(`Unmocked command: ${cmd}`);
    }
  });
}

describe('MarketplaceView', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    // Simulate Tauri desktop environment for marketplace tests
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = { invoke: mockInvoke };
  });

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it('renders header and tabs', async () => {
    setupMocks();
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    expect(wrapper.text()).toContain('Agent Marketplace');
    expect(wrapper.text()).toContain('Browse');
    expect(wrapper.text()).toContain('Installed');
  });

  it('loads and displays registry agents on mount', async () => {
    setupMocks();
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    expect(wrapper.text()).toContain('stub-agent');
    expect(wrapper.text()).toContain('file-fetch-agent');
    expect(wrapper.text()).toContain('Built-in stub agent for testing');
  });

  it('shows Install button for non-installed agents', async () => {
    setupMocks();
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    const installButtons = wrapper.findAll('button').filter((b) => b.text().includes('Install'));
    expect(installButtons.length).toBeGreaterThan(0);
  });

  it('shows Installed badge for installed agents', async () => {
    setupMocks({ installed: sampleInstalled });
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    expect(wrapper.text()).toContain('✅ Installed');
  });

  it('shows capability badges on agent cards', async () => {
    setupMocks();
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    expect(wrapper.text()).toContain('chat');
    expect(wrapper.text()).toContain('filesystem');
    expect(wrapper.text()).toContain('network');
  });

  it('shows consent dialog on install click', async () => {
    setupMocks();
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    // Click install on the capability-gated sample agent.
    const cards = wrapper.findAll('.mp-card');
    const sensitiveCard = cards.find((c) => c.text().includes('file-fetch-agent'));
    expect(sensitiveCard).toBeTruthy();
    const installBtn = sensitiveCard!.find('button');
    await installBtn.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('Capability Consent');
    expect(wrapper.text()).toContain('file-fetch-agent');
  });

  it('switches to installed tab', async () => {
    setupMocks({ installed: sampleInstalled, consents: sampleConsents });
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    const installedTab = wrapper.findAll('.mp-tab').find((t) => t.text().includes('Installed'));
    expect(installedTab).toBeTruthy();
    await installedTab!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('stub-agent');
  });

  it('performs search on enter', async () => {
    setupMocks();
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    const input = wrapper.find('.mp-search');
    await input.setValue('stub');
    await input.trigger('keyup.enter');
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith('search_agents', { query: 'stub' });
  });

  it('shows sandbox status badges on installed agents', async () => {
    setupMocks({ installed: sampleInstalled, consents: sampleConsents });
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    const installedTab = wrapper.findAll('.mp-tab').find((t) => t.text().includes('Installed'));
    await installedTab!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toMatch(/Sandboxed|Unrestricted|Unknown/);
  });

  it('shows error message on store error', async () => {
    mockInvoke.mockRejectedValue(new Error('Network error'));
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    expect(wrapper.text()).toContain('Network error');
  });

  it('shows version on agent cards', async () => {
    setupMocks();
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    expect(wrapper.text()).toContain('v1.0.0');
  });

  it('shows homepage link when available', async () => {
    setupMocks();
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    expect(wrapper.text()).toContain('terranes.dev');
  });

  it('shows Tauri unavailable banner when Tauri is not present', async () => {
    // Remove Tauri to simulate browser/UAT/Vercel environment
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    const wrapper = mount(MarketplaceView);
    await flushPromises();
    expect(wrapper.text()).toContain('Tauri Desktop Backend Unavailable');
    expect(wrapper.text()).toContain('desktop app');
    expect(wrapper.text()).toContain('Configure LLM');
    expect(wrapper.text()).not.toContain('stub-agent');
  });

  it('shows LLM config UI with free providers and chat hint', async () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    const wrapper = mount(MarketplaceView);
    await flushPromises();

    // Click "Configure LLM" to expand
    const configHeader = wrapper.find('.llm-config-header');
    expect(configHeader.exists()).toBe(true);
    await configHeader.trigger('click');
    await flushPromises();

    // Should show provider options and chat hint
    expect(wrapper.text()).toContain('Free Cloud');
    expect(wrapper.text()).toContain('Paid API');
    expect(wrapper.text()).toContain('Pollinations AI');
    expect(wrapper.text()).toContain('OpenRouter');
    expect(wrapper.text()).toContain('NVIDIA NIM');

    const paidTab = wrapper.findAll('.llm-tier-tab').find((button) => button.text().includes('Paid API'));
    expect(paidTab).toBeTruthy();
    await paidTab!.trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('Authorize with ChatGPT');
    expect(wrapper.text()).toContain('ask TerranSoul in chat');
  });

  it('shows LM Studio via Local LLM tab with provider pill switcher', async () => {
    setupMocks();
    const wrapper = mount(MarketplaceView);
    await flushPromises();

    const configHeader = wrapper.find('.llm-config-header');
    await configHeader.trigger('click');
    await flushPromises();

    // Click the unified "Local LLM" tab
    const localTab = wrapper.findAll('.llm-tier-tab').find((t) => t.text().includes('Local LLM'));
    expect(localTab).toBeTruthy();
    await localTab!.trigger('click');
    await flushPromises();

    // Click the "LM Studio" provider pill
    const lmPill = wrapper.findAll('.llm-provider-pill').find((t) => t.text().includes('LM Studio'));
    expect(lmPill).toBeTruthy();
    await lmPill!.trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('Refresh LM Studio');
    expect(wrapper.text()).toContain('Qwen 3 4B');
    const modelCard = wrapper.findAll('.llm-provider-card').find((c) => c.text().includes('Qwen 3 4B'));
    expect(modelCard).toBeTruthy();
    await modelCard!.trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('Activate qwen/qwen3-4b');
  });
});
