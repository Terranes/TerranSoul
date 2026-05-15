import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import ModelPanel from './ModelPanel.vue';

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock @tauri-apps/plugin-shell
vi.mock('@tauri-apps/plugin-shell', () => ({
  open: vi.fn(),
}));

describe('ModelPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders panel header', () => {
    const wrapper = mount(ModelPanel);
    expect(wrapper.find('.panel-shell__title').text()).toBe('3D Models');
  });

  it('renders import button', () => {
    const wrapper = mount(ModelPanel);
    const btn = wrapper.find('.import-btn');
    expect(btn.exists()).toBe(true);
    expect(btn.text()).toContain('Import VRM Model');
  });

  it('renders model select dropdown with default models', () => {
    const wrapper = mount(ModelPanel);
    const select = wrapper.find('.model-select');
    expect(select.exists()).toBe(true);
    const options = select.findAll('option');
    expect(options.length).toBe(2);
    expect(options[0].text()).toBe('Shinra');
    expect(options[1].text()).toBe('Komori');
  });

  it('renders model cards for default models', () => {
    const wrapper = mount(ModelPanel);
    const cards = wrapper.findAll('.model-card');
    expect(cards.length).toBe(2);
    expect(cards[0].text()).toContain('Shinra');
    expect(cards[1].text()).toContain('Komori');
  });

  it('emits close on overlay click', async () => {
    const wrapper = mount(ModelPanel);
    await wrapper.find('[data-testid="model-panel"]').trigger('click');
    expect(wrapper.emitted('close')).toBeTruthy();
  });

  it('emits close on close button click', async () => {
    const wrapper = mount(ModelPanel);
    await wrapper.find('[data-testid="panel-shell-close"]').trigger('click');
    expect(wrapper.emitted('close')).toBeTruthy();
  });

  it('shows VRM persistence hint text', () => {
    const wrapper = mount(ModelPanel);
    expect(wrapper.text()).toContain('persist');
  });

  it('shows instructions folder reference', () => {
    const wrapper = mount(ModelPanel);
    expect(wrapper.text()).toContain('instructions/');
  });

  it('first model card is active by default (shinra selected)', () => {
    const wrapper = mount(ModelPanel);
    const cards = wrapper.findAll('.model-card');
    expect(cards[0].classes()).toContain('active');
  });

});
