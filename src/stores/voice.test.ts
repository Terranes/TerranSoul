import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useVoiceStore } from './voice';
import type { VoiceProviderInfo } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const sampleAsrProvider: VoiceProviderInfo = {
  id: 'web-speech',
  display_name: 'Web Speech API',
  description: 'Browser-native speech recognition.',
  kind: 'local',
  requires_api_key: false,
  requires_sidecar: false,
};

const sampleTtsProvider: VoiceProviderInfo = {
  id: 'edge-tts',
  display_name: 'Edge TTS (free)',
  description: 'Microsoft Edge neural voices.',
  kind: 'cloud',
  requires_api_key: false,
  requires_sidecar: false,
};

const ollvProvider: VoiceProviderInfo = {
  id: 'open-llm-vtuber',
  display_name: 'Open-LLM-VTuber',
  description: 'Connect to Open-LLM-VTuber server.',
  kind: 'sidecar',
  requires_api_key: false,
  requires_sidecar: true,
};

describe('voice store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('isTextOnly is true by default', () => {
    const store = useVoiceStore();
    expect(store.isTextOnly).toBe(true);
    expect(store.hasVoice).toBe(false);
  });

  it('initialise loads providers and config via Tauri', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'list_asr_providers') return [sampleAsrProvider, ollvProvider];
      if (cmd === 'list_tts_providers') return [sampleTtsProvider, ollvProvider];
      if (cmd === 'get_voice_config')
        return { asr_provider: null, tts_provider: null, api_key: null, endpoint_url: null };
      return null;
    });

    const store = useVoiceStore();
    await store.initialise();

    expect(store.asrProviders).toHaveLength(2);
    expect(store.ttsProviders).toHaveLength(2);
    expect(store.isTextOnly).toBe(true);
  });

  it('initialise uses fallback providers when Tauri is unavailable', async () => {
    mockInvoke.mockRejectedValue(new Error('no Tauri'));

    const store = useVoiceStore();
    await store.initialise();

    expect(store.asrProviders.length).toBeGreaterThan(0);
    expect(store.ttsProviders.length).toBeGreaterThan(0);
    expect(store.asrProviders.some((p) => p.id === 'open-llm-vtuber')).toBe(true);
    expect(store.ttsProviders.some((p) => p.id === 'open-llm-vtuber')).toBe(true);
  });

  it('setAsrProvider updates config', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    await store.setAsrProvider('web-speech');

    expect(store.config.asr_provider).toBe('web-speech');
    expect(store.hasVoice).toBe(true);
    expect(store.isTextOnly).toBe(false);
  });

  it('setTtsProvider updates config', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    await store.setTtsProvider('edge-tts');

    expect(store.config.tts_provider).toBe('edge-tts');
    expect(store.hasVoice).toBe(true);
  });

  it('setApiKey updates config', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    await store.setApiKey('sk-test-key');

    expect(store.config.api_key).toBe('sk-test-key');
  });

  it('setEndpointUrl updates config', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    await store.setEndpointUrl('ws://localhost:12393/client-ws');

    expect(store.config.endpoint_url).toBe('ws://localhost:12393/client-ws');
  });

  it('clearConfig resets to text-only', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    await store.setAsrProvider('web-speech');
    await store.setTtsProvider('edge-tts');
    expect(store.hasVoice).toBe(true);

    await store.clearConfig();
    expect(store.isTextOnly).toBe(true);
    expect(store.config.asr_provider).toBeNull();
    expect(store.config.tts_provider).toBeNull();
    expect(store.config.api_key).toBeNull();
    expect(store.config.endpoint_url).toBeNull();
  });

  it('selectedAsrProvider returns matching provider', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    store.asrProviders = [sampleAsrProvider, ollvProvider];
    await store.setAsrProvider('open-llm-vtuber');

    expect(store.selectedAsrProvider).not.toBeNull();
    expect(store.selectedAsrProvider?.id).toBe('open-llm-vtuber');
  });

  it('selectedTtsProvider returns null when no provider is set', () => {
    const store = useVoiceStore();
    expect(store.selectedTtsProvider).toBeNull();
  });

  it('setAsrProvider works when Tauri is unavailable', async () => {
    mockInvoke.mockRejectedValue(new Error('no Tauri'));
    const store = useVoiceStore();
    await store.setAsrProvider('web-speech');

    expect(store.config.asr_provider).toBe('web-speech');
    expect(store.hasVoice).toBe(true);
  });

  it('Open-LLM-VTuber can be set as both ASR and TTS provider', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    store.asrProviders = [sampleAsrProvider, ollvProvider];
    store.ttsProviders = [sampleTtsProvider, ollvProvider];

    await store.setAsrProvider('open-llm-vtuber');
    await store.setTtsProvider('open-llm-vtuber');
    await store.setEndpointUrl('ws://localhost:12393/client-ws');

    expect(store.config.asr_provider).toBe('open-llm-vtuber');
    expect(store.config.tts_provider).toBe('open-llm-vtuber');
    expect(store.config.endpoint_url).toBe('ws://localhost:12393/client-ws');
    expect(store.selectedAsrProvider?.id).toBe('open-llm-vtuber');
    expect(store.selectedTtsProvider?.id).toBe('open-llm-vtuber');
  });
});
