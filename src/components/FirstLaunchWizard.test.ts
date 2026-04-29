import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia, defineStore } from 'pinia';
import FirstLaunchWizard from './FirstLaunchWizard.vue';

// ── Mock Tauri invoke ─────────────────────────────────────────────────────────

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

// ── Minimal store mocks ───────────────────────────────────────────────────────

vi.mock('../stores/brain', () => ({
  useBrainStore: defineStore('brain', () => {
    const hasBrain = { value: false };
    return {
      hasBrain,
      autoConfigureFreeApi: vi.fn(),
      autoConfigureForDesktop: vi.fn().mockResolvedValue(undefined),
    };
  }),
}));

vi.mock('../stores/voice', () => ({
  useVoiceStore: defineStore('voice', () => ({
    hasVoice: { value: false },
    autoConfigureVoice: vi.fn().mockResolvedValue(undefined),
  })),
}));

vi.mock('../stores/skill-tree', () => ({
  useSkillTreeStore: defineStore('skillTree', () => ({
    initialise: vi.fn().mockResolvedValue(undefined),
    markComplete: vi.fn(),
  })),
}));

vi.mock('../stores/settings', () => ({
  useSettingsStore: defineStore('settings', () => ({
    settings: { first_launch_complete: false },
    saveSettings: vi.fn().mockResolvedValue(undefined),
  })),
}));

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('FirstLaunchWizard', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    // Clean up any teleported content from previous tests.
    document.body.querySelectorAll('.flw-backdrop').forEach(el => el.remove());
  });

  it('shows the quest-mode step when visible', () => {
    const wrapper = mount(FirstLaunchWizard, {
      props: { visible: true },
      global: { stubs: { Teleport: true } },
    });
    expect(wrapper.text()).toContain('Welcome to TerranSoul');
    expect(wrapper.text()).toContain('Auto-Accept All');
    expect(wrapper.text()).toContain('Accept One by One');
  });

  it('is hidden when visible=false', () => {
    mount(FirstLaunchWizard, {
      props: { visible: false },
      global: { stubs: { Teleport: true } },
    });
    expect(document.body.querySelector('.flw-backdrop')).toBeNull();
  });

  it('emits done when "Set Up From Scratch" is clicked', async () => {
    const wrapper = mount(FirstLaunchWizard, {
      props: { visible: true },
      global: { stubs: { Teleport: true } },
    });
    const buttons = wrapper.findAll('.flw-option');
    // Third button = "Set Up From Scratch"
    await buttons[2].trigger('click');
    expect(wrapper.emitted('done')).toBeTruthy();
  });

  it('starts directly on quest-mode without a choose step', () => {
    const wrapper = mount(FirstLaunchWizard, {
      props: { visible: true },
      global: { stubs: { Teleport: true } },
    });
    // No separate "Recommended Setup" button — all options are on the first screen
    expect(wrapper.text()).toContain('Auto-Accept All');
    expect(wrapper.text()).not.toContain('Recommended Setup');
  });

  it('does not show a back button on the initial step', () => {
    const wrapper = mount(FirstLaunchWizard, {
      props: { visible: true },
      global: { stubs: { Teleport: true } },
    });
    expect(wrapper.find('.flw-back').exists()).toBe(false);
  });
});
