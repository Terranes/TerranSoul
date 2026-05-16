import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import VoiceSetupView from './VoiceSetupView.vue';

const invokeMock = vi.fn(async (cmd: string, _args?: Record<string, unknown>) => {
  switch (cmd) {
    case 'list_tts_providers':
      return [
        {
          id: 'web-speech',
          display_name: 'Web Speech (browser, free)',
          description: 'Browser-native synthesis.',
          kind: 'local',
          requires_api_key: false,
          installed: true,
          requires_install: false,
        },
        {
          id: 'openai-tts',
          display_name: 'OpenAI TTS',
          description: 'Cloud synthesis.',
          kind: 'cloud',
          requires_api_key: true,
          installed: true,
          requires_install: false,
        },
        {
          id: 'supertonic',
          display_name: 'Supertonic',
          description: 'On-device neural.',
          kind: 'local',
          requires_api_key: false,
          installed: false,
          requires_install: true,
        },
      ];
    case 'list_asr_providers':
      return [
        {
          id: 'web-speech',
          display_name: 'Web Speech API',
          description: 'Browser-native recognition.',
          kind: 'local',
          requires_api_key: false,
          installed: true,
          requires_install: false,
        },
      ];
    case 'get_voice_config':
      return {
        asr_provider: null,
        tts_provider: null,
        tts_voice: null,
        tts_pitch: 0,
        tts_rate: 0,
        api_key: null,
        endpoint_url: null,
        hotwords: [],
      };
    default:
      return undefined;
  }
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(args[0] as string, args[1] as Record<string, unknown> | undefined),
}));

describe('VoiceSetupView', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockClear();
  });

  it('renders the voice-setup root with the expected test id', async () => {
    const wrapper = mount(VoiceSetupView);
    await flushPromises();
    expect(wrapper.find('[data-testid="voice-setup-view"]').exists()).toBe(true);
    expect(wrapper.classes()).toContain('voice-setup');
  });

  it('lists TTS and ASR providers from the backend', async () => {
    const wrapper = mount(VoiceSetupView);
    await flushPromises();
    expect(wrapper.find('[data-testid="tts-provider-web-speech"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="tts-provider-openai-tts"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="tts-provider-supertonic"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="asr-provider-web-speech"]').exists()).toBe(true);
  });

  it('separates ready-to-use providers from setup-required providers', async () => {
    const wrapper = mount(VoiceSetupView);
    await flushPromises();
    // Supertonic is `requires_install && !installed` → setup group.
    const setupGroup = wrapper.find('[data-testid="tts-setup-group"]');
    expect(setupGroup.exists()).toBe(true);
    expect(setupGroup.find('[data-testid="tts-provider-supertonic"]').exists()).toBe(true);
    // Web Speech is local + no install → ready group.
    const readyGroup = wrapper.find('[data-testid="tts-ready-group"]');
    expect(readyGroup.exists()).toBe(true);
    expect(readyGroup.find('[data-testid="tts-provider-web-speech"]').exists()).toBe(true);
    // Setup card carries the dimmed-style modifier; its set-as-default button
    // is the consent entry point and must remain enabled so users can start setup.
    const supertonicCard = wrapper.find('[data-testid="tts-provider-supertonic"]');
    expect(supertonicCard.classes()).toContain('vs-provider-card--coming-soon');
    const setBtn = wrapper.find('[data-testid="set-default-tts-supertonic"]');
    expect((setBtn.element as HTMLButtonElement).disabled).toBe(false);
  });

  it('shows the text-only pill when no providers are configured', async () => {
    const wrapper = mount(VoiceSetupView);
    await flushPromises();
    expect(wrapper.find('[data-testid="voice-text-only-pill"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="voice-active-pill"]').exists()).toBe(false);
  });

  it('sets a TTS provider as default and surfaces the active pill', async () => {
    const wrapper = mount(VoiceSetupView);
    await flushPromises();
    await wrapper.find('[data-testid="set-default-tts-web-speech"]').trigger('click');
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith('set_tts_provider', { providerId: 'web-speech' });
    expect(wrapper.find('[data-testid="voice-active-pill"]').exists()).toBe(true);
  });

  it('reveals the API key section when a cloud provider is active', async () => {
    const wrapper = mount(VoiceSetupView);
    await flushPromises();
    expect(wrapper.find('[data-testid="voice-api-key-section"]').exists()).toBe(false);
    await wrapper.find('[data-testid="set-default-tts-openai-tts"]').trigger('click');
    await flushPromises();
    expect(wrapper.find('[data-testid="voice-api-key-section"]').exists()).toBe(true);
  });

  it('disables voice entirely via the footer toggle', async () => {
    const wrapper = mount(VoiceSetupView);
    await flushPromises();
    // First activate a provider so the toggle starts in the "voice-on" state.
    await wrapper.find('[data-testid="set-default-tts-web-speech"]').trigger('click');
    await flushPromises();
    await wrapper.find('[data-testid="voice-text-only-toggle"]').trigger('click');
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith('clear_voice_config', undefined);
    expect(wrapper.find('[data-testid="voice-text-only-pill"]').exists()).toBe(true);
  });
});
