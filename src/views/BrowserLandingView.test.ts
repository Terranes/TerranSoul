import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import BrowserLandingView from './BrowserLandingView.vue';
import { useCharacterStore } from '../stores/character';

vi.mock('../components/BrowserPetCompanion.vue', () => ({
  default: {
    name: 'BrowserPetCompanion',
    emits: ['request-provider-connect'],
    template: '<button class="browser-pet-companion-stub" @click="$emit(\'request-provider-connect\')">Pet</button>',
  },
}));

describe('BrowserLandingView', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.removeItem('ts.browser.auth.session');
    localStorage.removeItem('ts.browser.brain.mode');
  });

  it('renders the browser landing content and docs anchors', () => {
    const wrapper = mount(BrowserLandingView, {
      global: { stubs: { CharacterViewport: true } },
    });

    expect(wrapper.find('.browser-landing').exists()).toBe(true);
    expect(wrapper.get('#landing-title').text()).toBe('TerranSoul');
    expect(wrapper.find('a[href="#features"]').exists()).toBe(true);
    expect(wrapper.find('a[href="#brain"]').exists()).toBe(true);
    expect(wrapper.find('a[href="#quests"]').exists()).toBe(true);
    expect(wrapper.find('a[href="#browser-docs"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('Memory pipeline');
    expect(wrapper.text()).toContain('Setup is a progression system');
    expect(wrapper.find('img[alt="TerranSoul skill tree showing connected quest nodes"]').exists()).toBe(true);
  });

  it('renders the live pet companion stage', () => {
    const wrapper = mount(BrowserLandingView, {
      global: { stubs: { CharacterViewport: true } },
    });

    expect(wrapper.findComponent({ name: 'BrowserPetCompanion' }).exists()).toBe(true);
    expect(wrapper.get('.pet-stage').attributes('aria-label')).toBe('Live TerranSoul pet companion');
  });

  it('renders the browser pet companion area when the avatar is emotional', () => {
    const character = useCharacterStore();
    character.setState('happy');
    const wrapper = mount(BrowserLandingView, {
      global: { stubs: { CharacterViewport: true } },
    });

    expect(wrapper.findComponent({ name: 'BrowserPetCompanion' }).exists()).toBe(true);
  });

  it('keeps provider choices out of the landing page until the button is clicked', async () => {
    const wrapper = mount(BrowserLandingView, {
      global: { stubs: { CharacterViewport: true } },
    });

    expect(wrapper.text()).not.toContain('Choose your web LLM');
    expect(wrapper.text()).not.toContain('Try instantly');

    await wrapper.get('.secondary-action').trigger('click');

    expect(wrapper.text()).toContain('Choose your web LLM');
    expect(wrapper.text()).toContain('Authorize with ChatGPT');
    expect(wrapper.text()).toContain('Authorize with Gemini');
    expect(wrapper.text()).toContain('Authorize with OpenRouter');
    expect(wrapper.text()).toContain('Authorize with NVIDIA');
    expect(wrapper.text()).toContain('Authorize with Pollinations');
    expect(wrapper.text()).not.toContain('Try instantly');
    expect(wrapper.text()).toContain('Manual API key option');
  });

  it('remembers a manually connected OpenRouter browser provider', async () => {
    const wrapper = mount(BrowserLandingView, {
      global: { stubs: { CharacterViewport: true } },
    });

    await wrapper.get('.secondary-action').trigger('click');

    const manual = wrapper.findAll('button').find((button) => button.text().includes('Manual API key option'));
    expect(manual).toBeTruthy();
    await manual!.trigger('click');

    await wrapper.get('input[type="password"]').setValue('sk-or-test');
    await wrapper.findAll('button').find((button) => button.text().includes('Connect with this key'))!.trigger('click');

    expect(JSON.parse(localStorage.getItem('ts.browser.auth.session') ?? '{}')).toMatchObject({
      providerId: 'openrouter',
    });
  });

  it('opens the provider chooser when pet mode requests a provider', async () => {
    const wrapper = mount(BrowserLandingView, {
      global: { stubs: { CharacterViewport: true } },
    });

    await wrapper.get('.browser-pet-companion-stub').trigger('click');

    expect(wrapper.text()).toContain('Choose your web LLM');
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
