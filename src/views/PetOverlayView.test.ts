/**
 * Tests for PetOverlayView — desktop pet overlay with floating chat.
 *
 * Interaction model (post-redesign):
 *   - Left-click character        → toggle chat
 *   - Hold + drag on character    → reposition within overlay (persists in localStorage)
 *   - Right-click character       → open PetContextMenu at cursor
 *   - The desktop⇄pet toggle is rendered at the App level, not inside this view.
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
  EYE_TARGET_DISTANCE: 1.5,
}));

vi.mock('../renderer/vrm-loader', () => ({
  loadVRMSafe: vi.fn().mockResolvedValue(null),
}));

vi.mock('../renderer/character-animator', () => ({
  CharacterAnimator: vi.fn().mockImplementation(() => ({
    setVRM: vi.fn(),
    setState: vi.fn(),
    update: vi.fn(),
    forceIdlePose: vi.fn(),
    onIdlePoseChange: vi.fn(),
  })),
}));

import { useChatExpansion } from '../composables/useChatExpansion';

describe('PetOverlayView', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset().mockResolvedValue(undefined);
    // Reset shared module-level chat expansion state to default
    const { setPetChatExpanded, setChatDrawerExpanded } = useChatExpansion();
    setPetChatExpanded(false);
    setChatDrawerExpanded(false);
    // Clear persisted pet position / onboarding so each test starts fresh
    try {
      localStorage.removeItem('ts.pet.character_position');
      localStorage.removeItem('ts.pet.onboarded');
    } catch {
      /* ignore */
    }
  });

  it('renders the pet overlay container', () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true, PetContextMenu: true } },
    });
    expect(wrapper.find('.pet-overlay').exists()).toBe(true);
  });

  it('renders the character area', () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true, PetContextMenu: true } },
    });
    expect(wrapper.find('.pet-character').exists()).toBe(true);
  });

  it('does NOT render a mode toggle inside the overlay (lives at App level)', () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true, PetContextMenu: true } },
    });
    expect(wrapper.find('.pet-mode-toggle').exists()).toBe(false);
    expect(wrapper.find('.pet-mode-switch').exists()).toBe(false);
  });

  it('left-click on character toggles chat', async () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true, PetContextMenu: true } },
    });
    // Chat is collapsed by default in the redesigned overlay
    expect(wrapper.find('.pet-chat').exists()).toBe(false);

    // Simulate a quick left click: mousedown on character, then mouseup at document level.
    const character = wrapper.find('.pet-character');
    await character.trigger('mousedown', { button: 0, clientX: 100, clientY: 100 });
    // The overlay listens to document-level mouseup (once handler) to end the press.
    document.dispatchEvent(new MouseEvent('mouseup', { clientX: 100, clientY: 100 }));
    await wrapper.vm.$nextTick();

    expect(wrapper.find('.pet-chat').exists()).toBe(true);

    // Click again via the in-chat close button
    await wrapper.find('.pet-chat-close').trigger('click');
    expect(wrapper.find('.pet-chat').exists()).toBe(false);
  });

  it('chat input is present when expanded', async () => {
    const { setPetChatExpanded } = useChatExpansion();
    setPetChatExpanded(true);
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true, PetContextMenu: true } },
    });
    const input = wrapper.find('.pet-chat-input input');
    expect(input.exists()).toBe(true);
  });

  it('right-click on character opens the context menu', async () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true, PetContextMenu: true } },
    });

    const character = wrapper.find('.pet-character');
    await character.trigger('contextmenu', { clientX: 200, clientY: 300 });

    const ctxMenu = wrapper.findComponent({ name: 'PetContextMenu' });
    expect(ctxMenu.exists()).toBe(true);
    expect(ctxMenu.props('visible')).toBe(true);
    expect(ctxMenu.props('x')).toBe(200);
    expect(ctxMenu.props('y')).toBe(300);
  });

  it('does not show bubble when no messages', () => {
    const wrapper = mount(PetOverlayView, {
      global: { stubs: { CharacterViewport: true, PetContextMenu: true } },
    });
    expect(wrapper.find('.pet-bubble').exists()).toBe(false);
  });
});
