import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import SelfImprovePanel from './SelfImprovePanel.vue';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue(null) }));

describe('SelfImprovePanel', () => {
  let wrapper: VueWrapper;

  beforeEach(() => setActivePinia(createPinia()));
  afterEach(() => wrapper?.unmount());

  it('renders title, progress bar, roadmap, and action buttons', () => {
    wrapper = mount(SelfImprovePanel);
    expect(wrapper.find('.si-panel-title').text()).toContain('Self-Improve');
    expect(wrapper.find('[role="progressbar"]').exists()).toBe(true);
    expect(wrapper.findAll('.si-phase').length).toBeGreaterThanOrEqual(7);
    expect(wrapper.text()).toContain('Configure Coding LLM');
  });

  it('renders self-improve improvement chunks for research and Redis scouting', () => {
    wrapper = mount(SelfImprovePanel);
    expect(wrapper.find('.si-improvement').exists()).toBe(true);
    expect(wrapper.text()).toContain('Improvement chunks');
    expect(wrapper.text()).toContain('Redis');
    expect(wrapper.text()).toMatch(/model\/tool\/API|model, tool, and API/i);
  });

  it('renders finished, working, backlog, and coding workflow trackers', () => {
    wrapper = mount(SelfImprovePanel);
    expect(wrapper.find('.si-queue').exists()).toBe(true);
    expect(wrapper.text()).toContain('Finished');
    expect(wrapper.text()).toContain('Working on');
    expect(wrapper.text()).toContain('Backlog');
    expect(wrapper.find('.si-workflow').exists()).toBe(true);
    expect(wrapper.text()).toContain('Coding workflow');
    expect(wrapper.text()).toContain('Select chunk');
    expect(wrapper.text()).toContain('Validate, archive, and PR');
  });

  it('disables Enable button when no coding LLM is configured', () => {
    wrapper = mount(SelfImprovePanel);
    const enableBtn = wrapper
      .findAll('button')
      .find((b) => b.text().includes('Enable self-improve'));
    expect(enableBtn).toBeTruthy();
    expect(enableBtn!.attributes('disabled')).toBeDefined();
  });

  it('shows OFF status pill when disabled', () => {
    wrapper = mount(SelfImprovePanel);
    expect(wrapper.find('.si-status-pill').text()).toBe('OFF');
  });

  it('renders observability stat cards (runs / success / failure / latency)', () => {
    wrapper = mount(SelfImprovePanel);
    const stats = wrapper.findAll('.si-stat');
    expect(stats.length).toBe(4);
    const labels = stats.map((s) => s.find('.si-stat-label').text().toLowerCase());
    expect(labels.some((l) => l.includes('runs'))).toBe(true);
    expect(labels.some((l) => l.includes('success'))).toBe(true);
    expect(labels.some((l) => l.includes('failure'))).toBe(true);
    expect(labels.some((l) => l.includes('latency'))).toBe(true);
  });

  it('renders empty-state message when no runs are persisted', () => {
    wrapper = mount(SelfImprovePanel);
    expect(wrapper.find('.si-runs-empty').exists()).toBe(true);
    expect(wrapper.find('.si-runs-empty').text()).toMatch(/no runs yet/i);
  });
});
