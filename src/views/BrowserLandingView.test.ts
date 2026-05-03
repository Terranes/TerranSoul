import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it } from 'vitest';
import BrowserLandingView from './BrowserLandingView.vue';
import { useCharacterStore } from '../stores/character';

describe('BrowserLandingView', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.removeItem('ts.browser.auth.session');
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

  it('shows the browser pet manga emotion bubble when the avatar is emotional', () => {
    const character = useCharacterStore();
    character.setState('happy');
    const wrapper = mount(BrowserLandingView, {
      global: { stubs: { CharacterViewport: true } },
    });

    expect(wrapper.find('.pet-emotion-bubble').exists()).toBe(true);
    expect(wrapper.get('.pet-emotion-bubble').attributes('aria-label')).toBe('Happy');
  });

  it('offers zero-backend browser authorisation choices and remembers a click', async () => {
    const wrapper = mount(BrowserLandingView, {
      global: { stubs: { CharacterViewport: true } },
    });

    expect(wrapper.text()).toContain('No installs. No keys to type.');
    expect(wrapper.text()).toContain('Authorize with Google');
    expect(wrapper.text()).toContain('Authorize with ChatGPT');

    await wrapper.findAll('.auth-action')[0].trigger('click');

    expect(wrapper.text()).toContain('Connected: Google-ready browser session');
    expect(JSON.parse(localStorage.getItem('ts.browser.auth.session') ?? '{}')).toMatchObject({
      providerId: 'google',
    });
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
