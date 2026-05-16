import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import ThemePicker from './ThemePicker.vue';
import { BUILTIN_THEMES } from '../config/themes';
import { useTheme } from '../composables/useTheme';

describe('ThemePicker', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.removeItem('ts-active-theme');
    const { setTheme } = useTheme();
    setTheme('default');
  });

  afterEach(() => {
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.style.cssText = '';
    localStorage.removeItem('ts-active-theme');
  });

  it('renders a card for each built-in theme', () => {
    const wrapper = mount(ThemePicker);
    const cards = wrapper.findAll('.tp-card');
    expect(cards.length).toBe(BUILTIN_THEMES.length);
  });

  it('marks the default theme as active', () => {
    const wrapper = mount(ThemePicker);
    const activeCard = wrapper.find('.tp-card--active');
    expect(activeCard.exists()).toBe(true);
    expect(activeCard.attributes('data-testid')).toBe('theme-default');
  });

  it('switches theme when a card is clicked', async () => {
    const wrapper = mount(ThemePicker);
    const corpCard = wrapper.find('[data-testid="theme-corporate"]');
    await corpCard.trigger('click');

    const { themeId } = useTheme();
    expect(themeId.value).toBe('corporate');
  });

  it('shows theme label and icon', () => {
    const wrapper = mount(ThemePicker);
    const labels = wrapper.findAll('.tp-label');
    const icons = wrapper.findAll('.tp-icon');
    expect(labels.length).toBe(BUILTIN_THEMES.length);
    expect(icons.length).toBe(BUILTIN_THEMES.length);
    expect(labels[0].text()).toBe('Soul of TerranSoul');
    expect(icons[0].text()).toBe('🔷');
  });

  it('shows color preview dots', () => {
    const wrapper = mount(ThemePicker);
    const dots = wrapper.findAll('.tp-dot');
    // 3 dots per theme card
    expect(dots.length).toBe(BUILTIN_THEMES.length * 3);
  });

  it('has a header with title', () => {
    const wrapper = mount(ThemePicker);
    expect(wrapper.find('.tp-title').text()).toContain('Appearance');
  });

  it('active card changes when switching theme', async () => {
    const wrapper = mount(ThemePicker);
    const catCard = wrapper.find('[data-testid="theme-cat"]');
    await catCard.trigger('click');

    await wrapper.vm.$nextTick();
    const active = wrapper.find('.tp-card--active');
    expect(active.attributes('data-testid')).toBe('theme-cat');
  });
});
