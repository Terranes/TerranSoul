import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.vue';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

async function mountBrowserApp() {
  const wrapper = mount(App, {
    global: {
      stubs: {
        BrowserLandingView: {
          name: 'BrowserLandingView',
          emits: ['open-app-window'],
          template: '<button class="mock-open" @click="$emit(\'open-app-window\')">Open</button>',
        },
        ChatView: {
          name: 'ChatView',
          emits: ['navigate'],
          props: ['chatboxMode'],
          template: '<button class="mock-chat" @click="$emit(\'navigate\', \'pet-mode\')">Chat</button>',
        },
        BackgroundScene: true,
        BrainView: true,
        ComboToast: true,
        FirstLaunchWizard: true,
        FloatingBadge: true,
        MarketplaceView: true,
        MemoryView: true,
        MobilePairingView: true,
        PetOverlayView: true,
        QuestBubble: true,
        QuestRewardCeremony: true,
        SkillTreeView: true,
        SplashScreen: true,
        VoiceSetupView: true,
      },
    },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  await wrapper.vm.$nextTick();
  return wrapper;
}

describe('App browser app window', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockRejectedValue(new Error('Tauri unavailable'));
  });

  it('opens a dialog-like in-page app window from browser landing', async () => {
    const wrapper = await mountBrowserApp();

    await wrapper.get('.mock-open').trigger('click');
    await wrapper.vm.$nextTick();

    const window = wrapper.get('.browser-app-window');
    expect(window.attributes('role')).toBe('dialog');
    expect(window.attributes('tabindex')).toBe('-1');
    expect(wrapper.get('button[aria-label="Switch browser app window to 3D view"]').attributes('aria-pressed')).toBe('true');
  });

  it('switches chat mode and closes back to pet preview', async () => {
    const wrapper = await mountBrowserApp();
    await wrapper.get('.mock-open').trigger('click');
    await wrapper.vm.$nextTick();

    await wrapper.get('button[aria-label="Switch browser app window to chat view"]').trigger('click');
    expect(wrapper.get('button[aria-label="Switch browser app window to chat view"]').attributes('aria-pressed')).toBe('true');

    await wrapper.get('button[aria-label="Close app window and return to pet preview"]').trigger('click');
    expect(wrapper.find('.browser-app-window').exists()).toBe(false);
  });

  it('returns to browser pet preview when a quest navigates to pet mode', async () => {
    const wrapper = await mountBrowserApp();
    await wrapper.get('.mock-open').trigger('click');
    await wrapper.vm.$nextTick();

    await wrapper.get('.mock-chat').trigger('click');

    expect(wrapper.find('.browser-app-window').exists()).toBe(false);
  });
});
