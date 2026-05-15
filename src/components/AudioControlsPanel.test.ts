import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import AudioControlsPanel from './AudioControlsPanel.vue';

// jsdom doesn't ship a real AudioContext / mediaDevices implementation, so we
// stub the bits the panel touches at module load. Tests focus on UI surface +
// emit contracts, not actual audio plumbing.
beforeEach(() => {
  vi.stubGlobal(
    'AudioContext',
    vi.fn().mockImplementation(() => ({
      createOscillator: () => ({
        connect: vi.fn(),
        frequency: { value: 0 },
        start: vi.fn(),
        stop: vi.fn(),
      }),
      createGain: () => ({ connect: vi.fn(), gain: { value: 0 } }),
      createAnalyser: () => ({
        fftSize: 0,
        frequencyBinCount: 1024,
        getByteFrequencyData: vi.fn(),
      }),
      createMediaStreamSource: () => ({ connect: vi.fn() }),
      destination: {},
      close: vi.fn().mockResolvedValue(undefined),
    })),
  );

  Object.defineProperty(globalThis.navigator, 'mediaDevices', {
    configurable: true,
    value: {
      enumerateDevices: vi.fn().mockResolvedValue([
        { kind: 'audioinput', deviceId: 'mic-1', label: 'Built-in Mic' } as MediaDeviceInfo,
        { kind: 'audiooutput', deviceId: 'spk-1', label: 'Built-in Speakers' } as MediaDeviceInfo,
      ]),
      getUserMedia: vi.fn().mockRejectedValue(new Error('denied')),
    },
  });
});

describe('AudioControlsPanel', () => {
  it('renders the panel-shell with the audio test id', async () => {
    const wrapper = mount(AudioControlsPanel);
    await flushPromises();
    expect(wrapper.find('[data-testid="audio-controls-panel"]').exists()).toBe(true);
  });

  it('renders the five core sections', async () => {
    const wrapper = mount(AudioControlsPanel);
    await flushPromises();
    expect(wrapper.find('[data-testid="audio-system-section"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="audio-bgm-section"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="audio-mic-section"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="audio-speaker-section"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="audio-voice-link-section"]').exists()).toBe(true);
  });

  it('emits update:bgmVolume when the BGM slider changes', async () => {
    const wrapper = mount(AudioControlsPanel);
    await flushPromises();
    const slider = wrapper
      .find('[data-testid="audio-bgm-section"]')
      .find('input[type="range"]');
    await slider.setValue('42');
    const events = wrapper.emitted('update:bgmVolume');
    expect(events).toBeTruthy();
    expect(events?.[events.length - 1]?.[0]).toBeCloseTo(0.42);
  });

  it('emits update:bgmEnabled=false and update:bgmVolume=0 when BGM is muted', async () => {
    const wrapper = mount(AudioControlsPanel);
    await flushPromises();
    await wrapper.find('[data-testid="audio-bgm-mute"]').trigger('click');
    expect(wrapper.emitted('update:bgmEnabled')?.[0]?.[0]).toBe(false);
    expect(wrapper.emitted('update:bgmVolume')?.some((args) => args[0] === 0)).toBe(true);
  });

  it('emits navigate=voice + close when the voice setup link is clicked', async () => {
    const wrapper = mount(AudioControlsPanel);
    await flushPromises();
    await wrapper.find('[data-testid="audio-open-voice-setup"]').trigger('click');
    expect(wrapper.emitted('navigate')?.[0]?.[0]).toBe('voice');
    expect(wrapper.emitted('close')).toBeTruthy();
  });

  it('lists the enumerated audio devices in the dropdowns', async () => {
    const wrapper = mount(AudioControlsPanel);
    await flushPromises();
    const micOptions = wrapper.find('[data-testid="audio-mic-device"]').findAll('option');
    expect(micOptions.length).toBeGreaterThanOrEqual(2); // default + Built-in Mic
    expect(micOptions.some((o) => o.text().includes('Built-in Mic'))).toBe(true);
    const spkOptions = wrapper.find('[data-testid="audio-speaker-device"]').findAll('option');
    expect(spkOptions.some((o) => o.text().includes('Built-in Speakers'))).toBe(true);
  });

  it('emits close when the PanelShell × button is clicked', async () => {
    const wrapper = mount(AudioControlsPanel);
    await flushPromises();
    await wrapper.find('[data-testid="panel-shell-close"]').trigger('click');
    expect(wrapper.emitted('close')).toBeTruthy();
  });
});
