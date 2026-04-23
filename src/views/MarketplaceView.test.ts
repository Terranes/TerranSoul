import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { setActivePinia, createPinia } from 'pinia';
import MarketplaceView from './MarketplaceView.vue';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

function setupDesktopMocks() {
  mockInvoke.mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case 'list_free_providers':
        return [
          {
            id: 'pollinations',
            display_name: 'Pollinations AI',
            base_url: 'https://text.pollinations.ai/openai',
            model: 'openai',
            rpm_limit: 30,
            rpd_limit: 0,
            requires_api_key: false,
            notes: 'Free, no API key needed — works instantly',
          },
        ];
      case 'get_brain_mode':
        return { mode: 'free_api', provider_id: 'pollinations', api_key: null };
      case 'check_ollama_status':
        return { running: false, model_count: 0 };
      case 'recommend_brain_models':
        return [];
      case 'get_ollama_models':
        return [];
      default:
        return null;
    }
  });
}

describe('MarketplaceView', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it('renders brain configuration header', async () => {
    setupDesktopMocks();
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = { invoke: mockInvoke };

    const wrapper = mount(MarketplaceView);
    await flushPromises();

    expect(wrapper.text()).toContain('Brain Configuration');
    expect(wrapper.text()).toContain('Configure LLM');
  });

  it('shows browser fallback banner when Tauri is unavailable', async () => {
    const wrapper = mount(MarketplaceView);
    await flushPromises();

    expect(wrapper.text()).toContain('Tauri Desktop Backend Unavailable');
    expect(wrapper.text()).toContain('Configure LLM');
    expect(wrapper.text()).toContain('Pollinations AI');
  });

  it('shows free and paid config tiers with chat hint', async () => {
    const wrapper = mount(MarketplaceView);
    await flushPromises();

    expect(wrapper.text()).toContain('Free Cloud');
    expect(wrapper.text()).toContain('Paid API');
    expect(wrapper.text()).toContain('ask TerranSoul in chat');
  });
});
