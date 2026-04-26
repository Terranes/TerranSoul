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

  it('shows the choose step when visible', () => {
    const wrapper = mount(FirstLaunchWizard, {
      props: { visible: true },
      global: { stubs: { Teleport: true } },
    });
    expect(wrapper.text()).toContain('Welcome to TerranSoul');
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
    // Second button = "Set Up From Scratch"
    await buttons[1].trigger('click');
    expect(wrapper.emitted('done')).toBeTruthy();
  });

  it('advances to quest-mode step on Recommended click', async () => {
    const wrapper = mount(FirstLaunchWizard, {
      props: { visible: true },
      global: { stubs: { Teleport: true } },
    });
    const buttons = wrapper.findAll('.flw-option');
    // First button = Recommended
    await buttons[0].trigger('click');
    expect(wrapper.text()).toContain('Quest Activation');
    expect(wrapper.text()).toContain('Auto-Accept All');
    expect(wrapper.text()).toContain('Accept One by One');
  });

  it('shows back button on quest-mode step', async () => {
    const wrapper = mount(FirstLaunchWizard, {
      props: { visible: true },
      global: { stubs: { Teleport: true } },
    });
    // Click Recommended to advance to quest-mode step
    await wrapper.findAll('.flw-option')[0].trigger('click');
    const backBtn = wrapper.find('.flw-back');
    expect(backBtn.exists()).toBe(true);
    await backBtn.trigger('click');
    expect(wrapper.text()).toContain('Welcome to TerranSoul');
  });
});
