import { describe, it, expect, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import TaskControls from './TaskControls.vue';

describe('TaskControls', () => {
  function factory(props: Partial<{ visible: boolean; queueCount: number }> = {}) {
    return mount(TaskControls, {
      props: {
        visible: true,
        queueCount: 0,
        ...props,
      },
    });
  }

  it('renders when visible', () => {
    const w = factory({ visible: true });
    expect(w.find('.task-controls').exists()).toBe(true);
  });

  it('does not render when not visible', () => {
    const w = factory({ visible: false });
    expect(w.find('.task-controls').exists()).toBe(false);
  });

  it('emits stop on Stop button click', async () => {
    const w = factory();
    await w.find('.task-controls__btn--stop').trigger('click');
    expect(w.emitted('stop')).toHaveLength(1);
  });

  it('emits stop-and-send on Stop & Send button click', async () => {
    const w = factory();
    await w.find('.task-controls__btn--stop-send').trigger('click');
    expect(w.emitted('stop-and-send')).toHaveLength(1);
  });

  it('shows queue badge when queueCount > 0', () => {
    const w = factory({ queueCount: 3 });
    const badge = w.find('.task-controls__badge');
    expect(badge.exists()).toBe(true);
    expect(badge.text()).toBe('3');
  });

  it('hides queue badge when queueCount is 0', () => {
    const w = factory({ queueCount: 0 });
    expect(w.find('.task-controls__badge').exists()).toBe(false);
  });

  it('shows help tooltip on hover', async () => {
    const w = factory();
    const helpBtn = w.find('.task-controls__help-btn');
    expect(w.find('.task-controls__tooltip').exists()).toBe(false);
    await helpBtn.trigger('mouseenter');
    expect(w.find('.task-controls__tooltip').exists()).toBe(true);
    // Contains all four descriptions
    const tooltipText = w.find('.task-controls__tooltip').text();
    expect(tooltipText).toContain('Stop');
    expect(tooltipText).toContain('Stop & Send');
    expect(tooltipText).toContain('Queue');
    expect(tooltipText).toContain('Steer');
  });

  it('hides help tooltip on mouseleave', async () => {
    const w = factory();
    const helpBtn = w.find('.task-controls__help-btn');
    await helpBtn.trigger('mouseenter');
    expect(w.find('.task-controls__tooltip').exists()).toBe(true);
    await helpBtn.trigger('mouseleave');
    expect(w.find('.task-controls__tooltip').exists()).toBe(false);
  });

  it('emits add-to-queue when prompt returns a value', async () => {
    const spy = vi.spyOn(window, 'prompt').mockReturnValue('follow up');
    const w = factory();
    await w.find('.task-controls__btn--queue').trigger('click');
    expect(w.emitted('add-to-queue')).toHaveLength(1);
    expect(w.emitted('add-to-queue')![0]).toEqual(['follow up']);
    spy.mockRestore();
  });

  it('does not emit add-to-queue when prompt is cancelled', async () => {
    const spy = vi.spyOn(window, 'prompt').mockReturnValue(null);
    const w = factory();
    await w.find('.task-controls__btn--queue').trigger('click');
    expect(w.emitted('add-to-queue')).toBeUndefined();
    spy.mockRestore();
  });

  it('emits steer when prompt returns a value', async () => {
    const spy = vi.spyOn(window, 'prompt').mockReturnValue('change direction');
    const w = factory();
    await w.find('.task-controls__btn--steer').trigger('click');
    expect(w.emitted('steer')).toHaveLength(1);
    expect(w.emitted('steer')![0]).toEqual(['change direction']);
    spy.mockRestore();
  });

  it('has accessible aria-labels on all buttons', () => {
    const w = factory();
    const buttons = w.findAll('button[aria-label]');
    // 4 action buttons + 1 help button
    expect(buttons.length).toBe(5);
  });

  it('help button has title attributes for native browser tooltips', () => {
    const w = factory();
    const stopBtn = w.find('.task-controls__btn--stop');
    expect(stopBtn.attributes('title')).toBeTruthy();
    expect(stopBtn.attributes('title')!.length).toBeGreaterThan(10);
  });
});
