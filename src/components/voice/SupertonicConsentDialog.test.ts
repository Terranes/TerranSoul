import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { mount } from '@vue/test-utils';
import { useVoiceStore } from '../../stores/voice';
import { useSupertonicConsent } from '../../composables/useSupertonicConsent';
import SupertonicConsentDialog from './SupertonicConsentDialog.vue';
import type { VoiceProviderInfo } from '../../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const mockUnlisten = vi.fn();
const mockListen = vi.fn(async (..._args: unknown[]) => mockUnlisten);
vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

const installedSupertonic: VoiceProviderInfo = {
  id: 'supertonic',
  display_name: 'Supertonic (on-device, neural)',
  description: 'On-device neural TTS.',
  kind: 'local',
  requires_api_key: false,
  installed: true,
  requires_install: true,
};

const uninstalledSupertonic: VoiceProviderInfo = {
  ...installedSupertonic,
  installed: false,
};

const webSpeech: VoiceProviderInfo = {
  id: 'web-speech',
  display_name: 'Web Speech (browser, free)',
  description: 'Browser-native.',
  kind: 'local',
  requires_api_key: false,
  installed: true,
  requires_install: false,
};

describe('SupertonicConsentDialog', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders OpenRAIL-M restrictions and ~268 MB disclosure in consent stage', () => {
    const wrapper = mount(SupertonicConsentDialog, {
      props: { visible: true, stage: 'consent' },
      attachTo: document.body,
    });
    const html = document.body.innerHTML;
    expect(html).toContain('OpenRAIL-M');
    expect(html).toContain('268');
    // Headline restrictions surface in the body.
    expect(html.toLowerCase()).toContain('discrimination');
    expect(html.toLowerCase()).toContain('surveillance');
    wrapper.unmount();
  });

  it('exposes upstream model card + licensing-audit links', () => {
    const wrapper = mount(SupertonicConsentDialog, {
      props: { visible: true, stage: 'consent' },
      attachTo: document.body,
    });
    const modelCard = document.querySelector<HTMLAnchorElement>(
      '[data-testid="sc-model-card-link"]',
    );
    const license = document.querySelector<HTMLAnchorElement>(
      '[data-testid="sc-licensing-link"]',
    );
    expect(modelCard?.href).toContain('huggingface.co/Supertone');
    expect(license?.href).toContain('licensing-audit');
    wrapper.unmount();
  });

  it('emits accept when Accept is clicked', async () => {
    const wrapper = mount(SupertonicConsentDialog, {
      props: { visible: true, stage: 'consent' },
      attachTo: document.body,
    });
    const accept = document.querySelector<HTMLButtonElement>('[data-testid="sc-accept"]');
    accept?.click();
    expect(wrapper.emitted('accept')).toBeTruthy();
    wrapper.unmount();
  });

  it('emits cancel when Cancel is clicked (no accept)', async () => {
    const wrapper = mount(SupertonicConsentDialog, {
      props: { visible: true, stage: 'consent' },
      attachTo: document.body,
    });
    const cancel = document.querySelector<HTMLButtonElement>('[data-testid="sc-cancel"]');
    cancel?.click();
    expect(wrapper.emitted('cancel')).toBeTruthy();
    expect(wrapper.emitted('accept')).toBeFalsy();
    wrapper.unmount();
  });

  it('maps download progress payload to percent + file index label', () => {
    const wrapper = mount(SupertonicConsentDialog, {
      props: {
        visible: true,
        stage: 'downloading',
        progress: {
          current_file: 'duration_predictor.onnx',
          current_bytes: 0,
          current_total: 0,
          overall_bytes: 134_000_000,
          overall_total: 268_000_000,
          file_index: 1,
          file_count: 4,
        },
      },
      attachTo: document.body,
    });
    const bar = document.querySelector<HTMLElement>('[data-testid="sc-progress-bar"]');
    expect(bar?.getAttribute('aria-valuenow')).toBe('50');
    const label = document.querySelector('[data-testid="sc-progress-label"]')?.textContent ?? '';
    expect(label).toContain('50%');
    expect(label).toContain('file 2 of 4');
    expect(label).toContain('duration_predictor.onnx');
    wrapper.unmount();
  });

  it('shows offline-specific hint for network errors', () => {
    const wrapper = mount(SupertonicConsentDialog, {
      props: {
        visible: true,
        stage: 'error',
        errorMessage: 'network error fetching duration_predictor.onnx: offline',
      },
      attachTo: document.body,
    });
    const html = document.body.innerHTML.toLowerCase();
    expect(html).toContain('internet connection');
    wrapper.unmount();
  });

  it('shows integrity-specific hint for SHA mismatch errors', () => {
    const wrapper = mount(SupertonicConsentDialog, {
      props: {
        visible: true,
        stage: 'error',
        errorMessage: 'size mismatch for vocoder.onnx',
      },
      attachTo: document.body,
    });
    const html = document.body.innerHTML.toLowerCase();
    expect(html).toContain('integrity');
    wrapper.unmount();
  });
});

describe('useSupertonicConsent — state machine', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockListen.mockClear();
    mockUnlisten.mockClear();
  });

  it('openIfNeeded returns false and stays hidden for non-Supertonic providers', () => {
    const voice = useVoiceStore();
    voice.ttsProviders = [webSpeech];
    const consent = useSupertonicConsent(voice);
    expect(consent.openIfNeeded('web-speech')).toBe(false);
    expect(consent.consentVisible.value).toBe(false);
  });

  it('openIfNeeded returns false when Supertonic is already installed', () => {
    const voice = useVoiceStore();
    voice.ttsProviders = [installedSupertonic];
    const consent = useSupertonicConsent(voice);
    expect(consent.openIfNeeded('supertonic')).toBe(false);
    expect(consent.consentVisible.value).toBe(false);
  });

  it('openIfNeeded opens dialog at consent stage for uninstalled Supertonic', () => {
    const voice = useVoiceStore();
    voice.ttsProviders = [uninstalledSupertonic];
    const consent = useSupertonicConsent(voice);
    expect(consent.openIfNeeded('supertonic')).toBe(true);
    expect(consent.consentVisible.value).toBe(true);
    expect(consent.consentStage.value).toBe('consent');
  });

  it('onAccept transitions through downloading → done on success and auto-promotes', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'supertonic_download_model') return Promise.resolve();
      if (cmd === 'list_tts_providers') return Promise.resolve([installedSupertonic]);
      if (cmd === 'set_tts_provider') return Promise.resolve();
      return Promise.resolve();
    });
    const voice = useVoiceStore();
    voice.ttsProviders = [uninstalledSupertonic];
    voice.config.tts_provider = 'web-speech';
    const consent = useSupertonicConsent(voice);
    consent.openIfNeeded('supertonic');

    await consent.onAccept();

    expect(consent.consentStage.value).toBe('done');
    expect(voice.config.tts_provider).toBe('supertonic');
    expect(voice.supertonicPromotion).not.toBeNull();
    expect(voice.supertonicPromotion?.previousProvider).toBe('web-speech');
  });

  it('onAccept transitions to error stage when download rejects', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'supertonic_download_model') {
        return Promise.reject(new Error('size mismatch for duration_predictor.onnx'));
      }
      return Promise.resolve();
    });
    const voice = useVoiceStore();
    voice.ttsProviders = [uninstalledSupertonic];
    const consent = useSupertonicConsent(voice);
    consent.openIfNeeded('supertonic');

    await consent.onAccept();

    expect(consent.consentStage.value).toBe('error');
    expect(voice.supertonicError).toContain('size mismatch');
  });

  it('onCancel closes the dialog without invoking download', () => {
    mockInvoke.mockResolvedValue(undefined);
    const voice = useVoiceStore();
    voice.ttsProviders = [uninstalledSupertonic];
    const consent = useSupertonicConsent(voice);
    consent.openIfNeeded('supertonic');
    consent.onCancel();
    expect(consent.consentVisible.value).toBe(false);
    expect(mockInvoke).not.toHaveBeenCalledWith('supertonic_download_model');
  });
});

describe('voice store — Supertonic auto-promotion', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('promotes when current is null and Supertonic is installed', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const voice = useVoiceStore();
    voice.ttsProviders = [installedSupertonic];
    voice.config.tts_provider = null;

    const promoted = await voice.autoPromoteSupertonicIfReady();

    expect(promoted).toBe(true);
    expect(voice.config.tts_provider).toBe('supertonic');
    expect(voice.supertonicPromotion?.previousProvider).toBeNull();
  });

  it('promotes when current is web-speech and Supertonic is installed', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const voice = useVoiceStore();
    voice.ttsProviders = [installedSupertonic];
    voice.config.tts_provider = 'web-speech';

    const promoted = await voice.autoPromoteSupertonicIfReady();

    expect(promoted).toBe(true);
    expect(voice.config.tts_provider).toBe('supertonic');
    expect(voice.supertonicPromotion?.previousProvider).toBe('web-speech');
  });

  it('does NOT promote when user explicitly picked openai-tts', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const voice = useVoiceStore();
    voice.ttsProviders = [installedSupertonic];
    voice.config.tts_provider = 'openai-tts';

    const promoted = await voice.autoPromoteSupertonicIfReady();

    expect(promoted).toBe(false);
    expect(voice.config.tts_provider).toBe('openai-tts');
    expect(voice.supertonicPromotion).toBeNull();
  });

  it('does NOT promote when Supertonic is not installed', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const voice = useVoiceStore();
    voice.ttsProviders = [uninstalledSupertonic];
    voice.config.tts_provider = null;

    const promoted = await voice.autoPromoteSupertonicIfReady();

    expect(promoted).toBe(false);
    expect(voice.config.tts_provider).toBeNull();
    expect(voice.supertonicPromotion).toBeNull();
  });

  it('revertSupertonicPromotion restores previous provider', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const voice = useVoiceStore();
    voice.ttsProviders = [installedSupertonic];
    voice.config.tts_provider = 'web-speech';
    await voice.autoPromoteSupertonicIfReady();
    expect(voice.config.tts_provider).toBe('supertonic');

    await voice.revertSupertonicPromotion();

    expect(voice.config.tts_provider).toBe('web-speech');
    expect(voice.supertonicPromotion).toBeNull();
  });

  it('revertSupertonicPromotion is a no-op when no promotion is recorded', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const voice = useVoiceStore();
    voice.config.tts_provider = 'openai-tts';

    await voice.revertSupertonicPromotion();

    expect(voice.config.tts_provider).toBe('openai-tts');
  });
});
