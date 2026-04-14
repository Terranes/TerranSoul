/**
 * Tests for PetOverlayView — desktop pet overlay with floating chat.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { setActivePinia, createPinia } from 'pinia';
import PetOverlayView from './PetOverlayView.vue';

// Mock Tauri APIs
const mockInvoke = vi.fn().mockResolvedValue(undefined);
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Suppress Three.js / WebGL warnings in test environment
vi.mock('../renderer/scene', () => ({
  initScene: vi.fn().mockReturnValue({
    scene: {},
    camera: {},
    renderer: { domElement: document.createElement('canvas'), render: vi.fn(), setSize: vi.fn(), dispose: vi.fn() },
    clock: { getDelta: () => 0.016 },
  }),
}));

vi.mock('../renderer/vrm-loader', () => ({
  loadVRMSafe: vi.fn().mockResolvedValue(null),
}));

vi.mock('../renderer/character-animator', () => ({
  CharacterAnimator: vi.fn().mockImplementation(() => ({
    setVRM: vi.fn(),
    setState: vi.fn(),
    update: vi.fn(),
  })),
}));

describe('PetOverlayView', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset().mockResolvedValue(undefined);
  });

  it('renders the pet overlay container', () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true } },
    });
    expect(wrapper.find('.pet-overlay').exists()).toBe(true);
  });

  it('renders the character area', () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true } },
    });
    expect(wrapper.find('.pet-character').exists()).toBe(true);
  });

  it('renders pet controls', () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true } },
    });
    const controls = wrapper.findAll('.pet-ctrl-btn');
    expect(controls).toHaveLength(2); // chat toggle + exit
  });

  it('toggles chat on button click', async () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true } },
    });
    // Chat auto-expands on mount
    expect(wrapper.find('.pet-chat').exists()).toBe(true);

    // Click the chat toggle button to collapse
    const chatBtn = wrapper.findAll('.pet-ctrl-btn').find((b) => b.text() === '💬');
    expect(chatBtn).toBeDefined();
    await chatBtn!.trigger('click');

    expect(wrapper.find('.pet-chat').exists()).toBe(false);

    // Click again to expand
    await chatBtn!.trigger('click');
    expect(wrapper.find('.pet-chat').exists()).toBe(true);
  });

  it('chat input is present when expanded', async () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true } },
    });
    // Chat auto-expands on mount
    const input = wrapper.find('.pet-chat-input input');
    expect(input.exists()).toBe(true);
  });

  it('exit button calls setMode window', async () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true } },
    });
    const exitBtn = wrapper.findAll('.pet-ctrl-btn').find((b) => b.text() === '✕');
    await exitBtn!.trigger('click');

    expect(mockInvoke).toHaveBeenCalledWith('set_window_mode', { mode: 'window' });
  });

  it('shows controls on mouse enter', async () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true } },
    });
    await wrapper.find('.pet-overlay').trigger('mouseenter');

    expect(wrapper.find('.pet-controls.visible').exists()).toBe(true);
  });

  it('hides controls on mouse leave when chat is collapsed and hint has dismissed', async () => {
    vi.useFakeTimers();
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true } },
    });

    // Flush the async onMounted (loadActiveBrain, listen, etc.)
    await vi.runAllTimersAsync();
    await wrapper.vm.$nextTick();

    // Chat auto-expands; collapse it first
    const chatBtn = wrapper.findAll('.pet-ctrl-btn').find((b) => b.text() === '💬');
    await chatBtn!.trigger('click');

    // Fast-forward past the initial hint timeout (5 seconds)
    await vi.advanceTimersByTimeAsync(6000);
    await wrapper.vm.$nextTick();

    // Now hover and unhover
    await wrapper.find('.pet-overlay').trigger('mouseenter');
    expect(wrapper.find('.pet-controls.visible').exists()).toBe(true);

    await wrapper.find('.pet-overlay').trigger('mouseleave');
    expect(wrapper.find('.pet-controls.visible').exists()).toBe(false);

    vi.useRealTimers();
  });

  it('does not show bubble when no messages', () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true } },
    });
    expect(wrapper.find('.pet-bubble').exists()).toBe(false);
  });
});
