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
    expect(wrapper.find('.panel-header h3').text()).toBe('3D Models');
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
    expect(options.length).toBeGreaterThanOrEqual(2);
    expect(options[0].text()).toBe('Annabelle the Sorcerer');
    expect(options[1].text()).toBe('M58');
  });

  it('renders model cards for default models', () => {
    const wrapper = mount(ModelPanel);
    const cards = wrapper.findAll('.model-card');
    expect(cards.length).toBeGreaterThanOrEqual(2);
    expect(cards[0].text()).toContain('Annabelle the Sorcerer');
    expect(cards[1].text()).toContain('M58');
  });

  it('emits close on overlay click', async () => {
    const wrapper = mount(ModelPanel);
    await wrapper.find('.model-panel-overlay').trigger('click');
    expect(wrapper.emitted('close')).toBeTruthy();
  });

  it('emits close on close button click', async () => {
    const wrapper = mount(ModelPanel);
    await wrapper.find('.close-btn').trigger('click');
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

  it('first model card is active by default (annabelle selected)', () => {
    const wrapper = mount(ModelPanel);
    const cards = wrapper.findAll('.model-card');
    expect(cards[0].classes()).toContain('active');
  });

  it('renders thumbnail images for models with thumbnails', () => {
    const wrapper = mount(ModelPanel);
    const thumbs = wrapper.findAll('.model-thumb');
    expect(thumbs.length).toBeGreaterThanOrEqual(2);
  });
});
