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
};

const sampleTtsProvider: VoiceProviderInfo = {
  id: 'web-speech',
  display_name: 'Web Speech (browser, free)',
  description: 'Microsoft Edge neural voices.',
  kind: 'cloud',
  requires_api_key: false,
};

const whisperProvider: VoiceProviderInfo = {
  id: 'whisper-api',
  display_name: 'OpenAI Whisper API',
  description: 'Cloud-based transcription via OpenAI.',
  kind: 'cloud',
  requires_api_key: true,
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
      if (cmd === 'list_asr_providers') return [sampleAsrProvider, whisperProvider];
      if (cmd === 'list_tts_providers') return [sampleTtsProvider];
      if (cmd === 'get_voice_config')
        return { asr_provider: null, tts_provider: null, api_key: null, endpoint_url: null };
      return null;
    });

    const store = useVoiceStore();
    await store.initialise();

    expect(store.asrProviders).toHaveLength(2);
    expect(store.ttsProviders).toHaveLength(1);
    expect(store.isTextOnly).toBe(true);
  });

  it('initialise uses fallback providers when Tauri is unavailable', async () => {
    mockInvoke.mockRejectedValue(new Error('no Tauri'));

    const store = useVoiceStore();
    await store.initialise();

    expect(store.asrProviders.length).toBeGreaterThan(0);
    expect(store.ttsProviders.length).toBeGreaterThan(0);
    expect(store.asrProviders.some((p) => p.id === 'web-speech')).toBe(true);
    expect(store.ttsProviders.some((p) => p.id === 'web-speech')).toBe(true);
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
    await store.setTtsProvider('web-speech');

    expect(store.config.tts_provider).toBe('web-speech');
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
    await store.setEndpointUrl('http://localhost:8000');

    expect(store.config.endpoint_url).toBe('http://localhost:8000');
  });

  it('clearConfig resets to text-only', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    await store.setAsrProvider('web-speech');
    await store.setTtsProvider('web-speech');
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
    store.asrProviders = [sampleAsrProvider, whisperProvider];
    await store.setAsrProvider('whisper-api');

    expect(store.selectedAsrProvider).not.toBeNull();
    expect(store.selectedAsrProvider?.id).toBe('whisper-api');
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

  it('cloud providers can be set with API key', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    store.asrProviders = [sampleAsrProvider, whisperProvider];
    store.ttsProviders = [sampleTtsProvider];

    await store.setAsrProvider('whisper-api');
    await store.setTtsProvider('web-speech');
    await store.setApiKey('sk-test-key');

    expect(store.config.asr_provider).toBe('whisper-api');
    expect(store.config.tts_provider).toBe('web-speech');
    expect(store.config.api_key).toBe('sk-test-key');
    expect(store.selectedAsrProvider?.id).toBe('whisper-api');
    expect(store.selectedTtsProvider?.id).toBe('web-speech');
  });

  // ── autoConfigureVoice Tests ────────────────────────────────────────────

  it('autoConfigureVoice enables Web Speech API for both ASR and TTS', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    expect(store.isTextOnly).toBe(true);

    await store.autoConfigureVoice();

    expect(store.config.asr_provider).toBe('web-speech');
    expect(store.config.tts_provider).toBe('web-speech');
    expect(store.hasVoice).toBe(true);
    expect(store.isTextOnly).toBe(false);
  });

  it('autoConfigureVoice works when Tauri is unavailable', async () => {
    mockInvoke.mockRejectedValue(new Error('no Tauri'));
    const store = useVoiceStore();

    await store.autoConfigureVoice();

    expect(store.config.asr_provider).toBe('web-speech');
    expect(store.config.tts_provider).toBe('web-speech');
    expect(store.hasVoice).toBe(true);
  });

  it('autoConfigureVoice persists to Tauri when available', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();

    await store.autoConfigureVoice();

    expect(mockInvoke).toHaveBeenCalledWith('set_asr_provider', { providerId: 'web-speech' });
    expect(mockInvoke).toHaveBeenCalledWith('set_tts_provider', { providerId: 'web-speech' });
  });
});

// ── IPC Contract Tests ─────────────────────────────────────────────────────

describe('voice store — IPC contract', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('setAsrProvider sends providerId (camelCase)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    await store.setAsrProvider('groq-whisper');
    expect(mockInvoke).toHaveBeenCalledWith('set_asr_provider', { providerId: 'groq-whisper' });
  });

  it('setTtsProvider sends providerId (camelCase)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    await store.setTtsProvider('web-speech');
    expect(mockInvoke).toHaveBeenCalledWith('set_tts_provider', { providerId: 'web-speech' });
  });

  it('setApiKey sends apiKey (camelCase)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    await store.setApiKey('sk-test-key');
    expect(mockInvoke).toHaveBeenCalledWith('set_voice_api_key', { apiKey: 'sk-test-key' });
  });

  it('setEndpointUrl sends endpointUrl (camelCase)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useVoiceStore();
    await store.setEndpointUrl('https://custom.api/v1');
    expect(mockInvoke).toHaveBeenCalledWith('set_voice_endpoint', { endpointUrl: 'https://custom.api/v1' });
  });
});
