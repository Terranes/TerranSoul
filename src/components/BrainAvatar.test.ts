import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import BrainAvatar from './BrainAvatar.vue';
import type { BrainMode } from '../types';

describe('BrainAvatar', () => {
  it('renders in no-brain state when mode is null', () => {
    const w = mount(BrainAvatar, { props: { mode: null } });
    const root = w.find('[data-testid="brain-avatar"]');
    expect(root.exists()).toBe(true);
    expect(root.classes()).toContain('mood-none');
    expect(root.classes()).toContain('no-brain');
    // No synapse dots when no brain.
    expect(w.find('[data-testid="brain-synapses"]').exists()).toBe(false);
  });

  it('maps free_api mode to the green palette', () => {
    const mode: BrainMode = { mode: 'free_api', provider_id: 'pollinations', api_key: null };
    const w = mount(BrainAvatar, { props: { mode } });
    expect(w.find('[data-testid="brain-avatar"]').classes()).toContain('mood-free');
  });

  it('maps paid_api mode to the blue palette', () => {
    const mode: BrainMode = { mode: 'paid_api', provider: 'openai', base_url: 'https://api.openai.com', model: 'gpt-4', api_key: 'sk-xxx' };
    const w = mount(BrainAvatar, { props: { mode } });
    expect(w.find('[data-testid="brain-avatar"]').classes()).toContain('mood-paid');
  });

  it('maps local_ollama mode to the purple palette', () => {
    const mode: BrainMode = { mode: 'local_ollama', model: 'gemma3:1b' };
    const w = mount(BrainAvatar, { props: { mode } });
    expect(w.find('[data-testid="brain-avatar"]').classes()).toContain('mood-local');
  });

  it('scales synapse dot count with the square root of memoryCount', () => {
    const mode: BrainMode = { mode: 'free_api', provider_id: 'pollinations', api_key: null };
    // 0 → 0, 4 → 2, 16 → 4, 100 → 10, 1000 → 12 (capped)
    const cases: Array<[number, number]> = [
      [0, 0],
      [4, 2],
      [16, 4],
      [100, 10],
      [1000, 12],
    ];
    for (const [count, expected] of cases) {
      const w = mount(BrainAvatar, { props: { mode, memoryCount: count } });
      const dots = w.findAll('.brain-synapse-dot');
      expect(dots.length, `memoryCount=${count}`).toBe(expected);
    }
  });

  it('renders thought bubbles when expression is "thinking"', () => {
    const mode: BrainMode = { mode: 'free_api', provider_id: 'pollinations', api_key: null };
    const w = mount(BrainAvatar, { props: { mode, expression: 'thinking' } });
    expect(w.find('[data-testid="brain-thoughts"]').exists()).toBe(true);
    expect(w.find('[data-testid="brain-avatar"]').classes()).toContain('expression-thinking');
  });

  it('exposes an aria-label that summarises mode + memories + expression', () => {
    const mode: BrainMode = { mode: 'local_ollama', model: 'gemma3:1b' };
    const w = mount(BrainAvatar, { props: { mode, memoryCount: 7, expression: 'happy' } });
    const label = w.find('[data-testid="brain-avatar"]').attributes('aria-label') ?? '';
    expect(label).toContain('Local LLM');
    expect(label).toContain('7 memories');
    expect(label).toContain('happy');
  });

  it('respects custom size prop', () => {
    const w = mount(BrainAvatar, { props: { mode: null, size: 64 } });
    const svg = w.find('svg.brain-svg');
    expect(svg.attributes('width')).toBe('64');
    expect(svg.attributes('height')).toBe('64');
  });

  it('renders closed eyes when sleepy (no-brain default)', () => {
    const w = mount(BrainAvatar, { props: { mode: null, expression: 'sleepy' } });
    expect(w.find('.brain-eyes-closed').exists()).toBe(true);
    expect(w.find('.brain-eyes').exists()).toBe(false);
  });
});
