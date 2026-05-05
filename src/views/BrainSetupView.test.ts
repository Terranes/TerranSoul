import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import BrainSetupView from './BrainSetupView.vue';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const freeProviders = [
  {
    id: 'openrouter',
    display_name: 'OpenRouter',
    base_url: 'https://openrouter.ai/api',
    model: 'openrouter/owl-alpha',
    rpm_limit: 20,
    rpd_limit: 50,
    requires_api_key: true,
    notes: 'Multi-model gateway with selectable free models',
  },
  {
    id: 'pollinations',
    display_name: 'Pollinations AI',
    base_url: 'https://gen.pollinations.ai',
    model: 'llama',
    rpm_limit: 30,
    rpd_limit: 0,
    requires_api_key: true,
    notes: 'Register at enter.pollinations.ai for a token',
  },
];

function mockBrainSetupInvoke(command: string) {
  switch (command) {
    case 'get_active_brain': return Promise.resolve(null);
    case 'get_brain_mode': return Promise.resolve(null);
    case 'list_free_providers': return Promise.resolve(freeProviders);
    case 'get_system_info': return Promise.resolve({
      os_name: 'Windows',
      arch: 'x86_64',
      cpu_name: 'Test CPU',
      cpu_cores: 8,
      total_ram_mb: 32768,
      ram_tier_label: 'High',
      gpu_name: 'Test GPU',
    });
    case 'recommend_brain_models': return Promise.resolve([]);
    case 'check_ollama_status': return Promise.resolve({ running: false, model_count: 0 });
    case 'get_ollama_models': return Promise.resolve([]);
    case 'check_lm_studio_status': return Promise.resolve({ running: false, model_count: 0, loaded_count: 0 });
    case 'get_lm_studio_models': return Promise.resolve([]);
    case 'set_brain_mode': return Promise.resolve(undefined);
    default: return Promise.resolve(null);
  }
}

describe('BrainSetupView', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockImplementation(mockBrainSetupInvoke);
  });

  it('persists the selected free-provider model from setup', async () => {
    const wrapper = mount(BrainSetupView);
    await flushPromises();

    await wrapper.get('button.btn-primary').trigger('click');
    await flushPromises();

    await wrapper.get('#free-model-select').setValue('poolside/laguna-xs.2:free');
    await wrapper.get('.bs-manual-toggle').trigger('click');
    await wrapper.get('input[type="password"]').setValue('or-test-key');

    const connectButton = wrapper
      .findAll('button.btn-primary')
      .find((button) => button.text().includes('Connect provider'));
    expect(connectButton).toBeTruthy();
    await connectButton!.trigger('click');

    expect(mockInvoke).toHaveBeenCalledWith('set_brain_mode', {
      mode: {
        mode: 'free_api',
        provider_id: 'openrouter',
        api_key: 'or-test-key',
        model: 'poolside/laguna-xs.2:free',
      },
    });
  });
});
