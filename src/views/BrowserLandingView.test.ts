import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it } from 'vitest';
import BrowserLandingView from './BrowserLandingView.vue';

describe('BrowserLandingView', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders the browser landing content and docs anchors', () => {
    const wrapper = mount(BrowserLandingView, {
      global: { stubs: { CharacterViewport: true } },
    });

    expect(wrapper.find('.browser-landing').exists()).toBe(true);
    expect(wrapper.get('#landing-title').text()).toContain('soul');
    expect(wrapper.find('a[href="#features"]').exists()).toBe(true);
    expect(wrapper.find('a[href="#missions"]').exists()).toBe(true);
    expect(wrapper.find('a[href="#browser-docs"]').exists()).toBe(true);
  });

  it('uses the real character viewport as a forced pet preview', () => {
    const wrapper = mount(BrowserLandingView, {
      global: { stubs: { CharacterViewport: true } },
    });

    const viewport = wrapper.findComponent({ name: 'CharacterViewport' });
    expect(viewport.exists()).toBe(true);
    expect(viewport.props('forcePet')).toBe(true);
    expect(wrapper.get('.pet-stage').attributes('aria-label')).toBe('Live TerranSoul pet companion');
    expect(wrapper.text()).toContain('Live voice');
    expect(wrapper.text()).toContain('Translator demo');
    expect(wrapper.text()).toContain('From');
    expect(wrapper.text()).toContain('To');
  });

  it('emits open-app-window from both browser launch buttons', async () => {
    const wrapper = mount(BrowserLandingView, {
      global: { stubs: { CharacterViewport: true } },
    });

    await wrapper.get('.nav-cta').trigger('click');
    await wrapper.get('.primary-action').trigger('click');

    expect(wrapper.emitted('open-app-window')).toHaveLength(2);
  });
});
