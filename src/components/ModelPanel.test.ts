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

  it('renders default placeholder card', () => {
    const wrapper = mount(ModelPanel);
    const card = wrapper.find('.model-card.default');
    expect(card.exists()).toBe(true);
    expect(card.text()).toContain('Default Placeholder');
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

  it('shows VRM format hint text', () => {
    const wrapper = mount(ModelPanel);
    expect(wrapper.text()).toContain('Supports .vrm files');
  });

  it('shows instructions folder reference', () => {
    const wrapper = mount(ModelPanel);
    expect(wrapper.text()).toContain('instructions/');
  });

  it('default placeholder card is active when no VRM loaded', () => {
    const wrapper = mount(ModelPanel);
    const card = wrapper.find('.model-card.default');
    expect(card.classes()).toContain('active');
  });
});
