import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { setActivePinia, createPinia } from 'pinia';
import { ref, shallowRef } from 'vue';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn(),
}));

import SettingsPanel from './SettingsPanel.vue';
import type { BgmPlayerHandle, BgmTrack } from '../composables/useBgmPlayer';

/** Build a minimal BgmPlayerHandle stub that satisfies the component's
 *  prop contract without touching real <audio>. */
function makeBgm(): BgmPlayerHandle {
  const allTracks = ref<BgmTrack[]>([
    { id: 'prelude', name: 'Prelude', src: 'sample-bgm.mp3' },
  ]);
  const customTracks = ref<BgmTrack[]>([]);
  return {
    audioEl: shallowRef<HTMLAudioElement | null>(null),
    currentTrack: shallowRef<BgmTrack | null>(null),
    isPlaying: ref(false),
    volume: ref(0.15),
    allTracks,
    customTracks,
    play: vi.fn(),
    stop: vi.fn(),
    setVolume: vi.fn(),
    addCustomTrack: vi.fn(() => 'custom-1'),
    removeTrack: vi.fn(),
    loadCustomTracks: vi.fn(),
  } as unknown as BgmPlayerHandle;
}

function mountPanel(propsOverrides: Partial<{ isPetMode: boolean; bgmEnabled: boolean; bgmVolume: number; bgmTrackId: string }> = {}) {
  return mount(SettingsPanel, {
    props: {
      isPetMode: false,
      bgm: makeBgm(),
      bgmEnabled: false,
      bgmVolume: 0.15,
      bgmTrackId: 'prelude',
      ...propsOverrides,
    },
    global: {
      stubs: {
        FloatingMenu: { template: '<div class="settings-dropdown" data-testid="settings-panel"><slot /></div>', inheritAttrs: false },
        ThemePicker: true,
        Teleport: { template: '<div><slot /></div>' },
      },
    },
  });
}

describe('SettingsPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders the floating dropdown root', () => {
    const wrapper = mountPanel();
    expect(wrapper.find('[data-testid="settings-panel"]').exists()).toBe(true);
  });

  it('renders the view-mode row, quest portal, and theme picker', () => {
    const wrapper = mountPanel();
    const text = wrapper.text();
    expect(text).toContain('View Mode');
    expect(text).toContain('3D');
    expect(text).toContain('Chat');
    expect(text).toContain('Pet');
    expect(wrapper.find('#corner-cluster-portal').exists()).toBe(true);
  });

  it('emits close when the header × is clicked', async () => {
    const wrapper = mountPanel();
    await wrapper.find('.settings-close-btn').trigger('click');
    expect(wrapper.emitted('close')).toBeTruthy();
  });

  it('emits request-set-display-mode and close when 3D button is clicked', async () => {
    const wrapper = mountPanel({ isPetMode: true });
    const buttons = wrapper.findAll('.settings-mode-btn');
    await buttons[0].trigger('click');
    const events = wrapper.emitted('request-set-display-mode');
    expect(events?.[0]).toEqual(['desktop']);
    expect(wrapper.emitted('close')).toBeTruthy();
  });

  it('emits toggle-system-info and toggle-audio-controls', async () => {
    const wrapper = mountPanel();
    const buttons = wrapper.findAll('.dropdown-btn').filter(b => /System Information|Audio Controls/.test(b.text()));
    expect(buttons.length).toBeGreaterThanOrEqual(2);
    await buttons[0].trigger('click');
    await buttons[1].trigger('click');
    expect(wrapper.emitted('toggle-system-info')).toBeTruthy();
    expect(wrapper.emitted('toggle-audio-controls')).toBeTruthy();
  });

  it('marks the 3D button active when not in pet mode', () => {
    const wrapper = mountPanel({ isPetMode: false });
    const btn = wrapper.findAll('.settings-mode-btn')[0];
    expect(btn.classes()).toContain('active');
  });

  it('renders the character profile editor fields', () => {
    const wrapper = mountPanel();
    const text = wrapper.text();
    expect(text).toContain('Character');
    expect(text).toContain('Persona');
    expect(text).toContain('Voice');
  });

  it('renders the mood grid with multiple chips', () => {
    const wrapper = mountPanel();
    const chips = wrapper.findAll('.mood-chip');
    expect(chips.length).toBeGreaterThan(3);
  });
});
