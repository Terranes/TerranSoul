import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import SkillConstellation from './SkillConstellation.vue';
import { useSkillTreeStore } from '../stores/skill-tree';

vi.mock('../stores/skill-tree');

interface FakeNode {
  id: string;
  name: string;
  tagline: string;
  description: string;
  icon: string;
  tier: 'foundation' | 'advanced' | 'ultimate';
  category: 'brain' | 'voice' | 'avatar' | 'social' | 'utility';
  requires: string[];
  rewards: string[];
  rewardIcons: string[];
  questSteps: { label: string; action: string; target?: string }[];
  combos: unknown[];
}

const NODES: FakeNode[] = [
  // Brain cluster — covers all three rings + a prereq edge.
  {
    id: 'free-brain', name: 'Awaken the Mind', tagline: 'Connect to free AI', description: 'Free brain.',
    icon: '🧠', tier: 'foundation', category: 'brain',
    requires: [], rewards: ['Chat'], rewardIcons: ['💬'],
    questSteps: [{ label: 'Auto configures', action: 'info' }, { label: 'Open Setup', action: 'navigate', target: 'brain-setup' }],
    combos: [],
  },
  {
    id: 'paid-brain', name: 'Sharpen the Mind', tagline: 'Paid LLM', description: 'Paid brain.',
    icon: '💎', tier: 'advanced', category: 'brain',
    requires: ['free-brain'], rewards: ['Smarter chat'], rewardIcons: ['🧠'],
    questSteps: [], combos: [],
  },
  {
    id: 'local-brain', name: 'Inner Sage', tagline: 'Offline Ollama', description: 'Local brain.',
    icon: '🏔️', tier: 'ultimate', category: 'brain',
    requires: ['paid-brain'], rewards: ['Offline AI'], rewardIcons: ['🔒'],
    questSteps: [], combos: [],
  },
  // Voice cluster
  {
    id: 'tts', name: 'Gift of Speech', tagline: 'TTS', description: 'Speak aloud.',
    icon: '🗣️', tier: 'foundation', category: 'voice',
    requires: [], rewards: ['Voice'], rewardIcons: ['🔊'],
    questSteps: [], combos: [],
  },
  // Avatar cluster
  {
    id: 'avatar', name: 'Embodied', tagline: 'VRM avatar', description: 'Avatar.',
    icon: '✨', tier: 'foundation', category: 'avatar',
    requires: [], rewards: ['Avatar'], rewardIcons: ['👤'],
    questSteps: [], combos: [],
  },
  // Social cluster
  {
    id: 'device-link', name: 'Hive Link', tagline: 'Pair devices', description: 'Cross device.',
    icon: '🔗', tier: 'advanced', category: 'social',
    requires: [], rewards: ['Sync'], rewardIcons: ['🔁'],
    questSteps: [], combos: [],
  },
  // Utility cluster
  {
    id: 'bgm', name: 'Ambient Crystal', tagline: 'BGM', description: 'Music.',
    icon: '📀', tier: 'foundation', category: 'utility',
    requires: [], rewards: ['BGM'], rewardIcons: ['🎵'],
    questSteps: [], combos: [],
  },
];

const skillStatus = new Map<string, 'locked' | 'available' | 'active'>([
  ['free-brain', 'active'],
  ['paid-brain', 'available'],
  ['local-brain', 'locked'],
  ['tts', 'available'],
  ['avatar', 'active'],
  ['device-link', 'available'],
  ['bgm', 'available'],
]);

const mockSkillTreeStore = {
  nodes: NODES,
  tracker: { pinnedQuestIds: [] as string[] },
  getSkillStatus: vi.fn((id: string) => skillStatus.get(id) ?? 'locked'),
  pinQuest: vi.fn(),
  unpinQuest: vi.fn(),
};

beforeEach(() => {
  vi.mocked(useSkillTreeStore).mockReturnValue(mockSkillTreeStore as never);
  mockSkillTreeStore.pinQuest.mockClear();
  mockSkillTreeStore.unpinQuest.mockClear();
  // jsdom doesn't implement ResizeObserver — provide a stub.
  if (typeof (globalThis as unknown as { ResizeObserver?: unknown }).ResizeObserver === 'undefined') {
    (globalThis as unknown as { ResizeObserver: unknown }).ResizeObserver = class {
      observe() {}
      unobserve() {}
      disconnect() {}
    };
  }
});

describe('SkillConstellation', () => {
  it('renders nothing when not visible', () => {
    const wrapper = mount(SkillConstellation, { props: { visible: false }, attachTo: document.body });
    expect(document.querySelector('.skill-constellation')).toBeNull();
    wrapper.unmount();
  });

  it('renders the constellation, breadcrumb, and minimap when visible', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    expect(document.querySelector('.skill-constellation')).toBeTruthy();
    expect(document.querySelector('[data-testid="constellation-breadcrumb"]')).toBeTruthy();
    expect(document.querySelector('[data-testid="constellation-minimap"]')).toBeTruthy();
    // Breadcrumb root crumb is shown
    expect(document.querySelector('[data-testid="constellation-breadcrumb"]')!.textContent)
      .toContain('All Clusters');
    wrapper.unmount();
  });

  it('renders all five cluster emblems', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    for (const id of ['brain', 'voice', 'avatar', 'social', 'utility']) {
      expect(document.querySelector(`[data-testid="cluster-emblem-${id}"]`)).toBeTruthy();
      expect(document.querySelector(`[data-testid="minimap-cluster-${id}"]`)).toBeTruthy();
    }
    wrapper.unmount();
  });

  it('places each skill node from the store', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    for (const node of NODES) {
      expect(document.querySelector(`[data-testid="skill-node-${node.id}"]`)).toBeTruthy();
    }
    wrapper.unmount();
  });

  it('applies status classes to nodes (locked/available/active)', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    expect(document.querySelector('[data-testid="skill-node-free-brain"]')!.className)
      .toContain('sc-node--active');
    expect(document.querySelector('[data-testid="skill-node-paid-brain"]')!.className)
      .toContain('sc-node--available');
    expect(document.querySelector('[data-testid="skill-node-local-brain"]')!.className)
      .toContain('sc-node--locked');
    wrapper.unmount();
  });

  it('zooms into a cluster and updates the breadcrumb when the emblem is clicked', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    const emblem = document.querySelector<HTMLButtonElement>('[data-testid="cluster-emblem-brain"]')!;
    emblem.click();
    await nextTick();

    const crumb = document.querySelector('[data-testid="constellation-breadcrumb"]')!.textContent ?? '';
    expect(crumb).toContain('Brain');
    // Back button should now exist.
    expect(document.querySelector('[data-testid="constellation-back"]')).toBeTruthy();
    wrapper.unmount();
  });

  it('opens the detail overlay when a node is clicked', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    const node = document.querySelector<HTMLButtonElement>('[data-testid="skill-node-paid-brain"]')!;
    node.click();
    await nextTick();

    const detail = document.querySelector('[data-testid="constellation-detail"]')!;
    expect(detail).toBeTruthy();
    expect(detail.textContent).toContain('Sharpen the Mind');
    expect(detail.textContent).toContain('Prerequisites');
    expect(detail.textContent).toContain('Awaken the Mind'); // prereq listed
    wrapper.unmount();
  });

  it('emits begin when the Begin Quest button is clicked', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    document.querySelector<HTMLButtonElement>('[data-testid="skill-node-paid-brain"]')!.click();
    await nextTick();

    document.querySelector<HTMLButtonElement>('[data-testid="constellation-begin"]')!.click();
    await nextTick();

    expect(wrapper.emitted('begin')).toBeTruthy();
    expect(wrapper.emitted('begin')![0]).toEqual(['paid-brain']);
    wrapper.unmount();
  });

  it('does not show Begin Quest for locked nodes', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    document.querySelector<HTMLButtonElement>('[data-testid="skill-node-local-brain"]')!.click();
    await nextTick();

    const detail = document.querySelector('[data-testid="constellation-detail"]')!;
    expect(detail).toBeTruthy();
    expect(detail.querySelector('[data-testid="constellation-begin"]')).toBeNull();
    wrapper.unmount();
  });

  it('emits navigate when a step Go button is clicked', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    document.querySelector<HTMLButtonElement>('[data-testid="skill-node-free-brain"]')!.click();
    await nextTick();

    const goBtn = document.querySelector<HTMLButtonElement>('.sc-step-go');
    expect(goBtn).toBeTruthy();
    goBtn!.click();
    await nextTick();

    expect(wrapper.emitted('navigate')).toBeTruthy();
    expect(wrapper.emitted('navigate')![0]).toEqual(['brain-setup']);
    wrapper.unmount();
  });

  it('back button steps from detail → cluster → all clusters', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    document.querySelector<HTMLButtonElement>('[data-testid="skill-node-paid-brain"]')!.click();
    await nextTick();
    expect(document.querySelector('[data-testid="constellation-detail"]')).toBeTruthy();

    // First back: closes detail (still focused on cluster)
    document.querySelector<HTMLButtonElement>('[data-testid="constellation-back"]')!.click();
    await nextTick();
    expect(document.querySelector('[data-testid="constellation-detail"]')).toBeNull();
    expect(document.querySelector('[data-testid="constellation-breadcrumb"]')!.textContent)
      .toContain('Brain');

    // Second back: clears cluster focus
    document.querySelector<HTMLButtonElement>('[data-testid="constellation-back"]')!.click();
    await nextTick();
    expect(document.querySelector('[data-testid="constellation-back"]')).toBeNull();
    wrapper.unmount();
  });

  it('zoom in / out / reset buttons modify the world transform scale', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    const world = document.querySelector<HTMLElement>('[data-testid="constellation-world"]')!;
    const initial = world.style.transform;

    document.querySelector<HTMLButtonElement>('[data-testid="zoom-in"]')!.click();
    await nextTick();
    expect(world.style.transform).not.toBe(initial);

    document.querySelector<HTMLButtonElement>('[data-testid="zoom-reset"]')!.click();
    await nextTick();
    expect(world.style.transform).toContain('scale(');
    wrapper.unmount();
  });

  it('emits close when the close button is clicked', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    document.querySelector<HTMLButtonElement>('.sc-close-btn')!.click();
    await nextTick();

    expect(wrapper.emitted('close')).toBeTruthy();
    wrapper.unmount();
  });

  it('pin/unpin actions delegate to the store', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    document.querySelector<HTMLButtonElement>('[data-testid="skill-node-paid-brain"]')!.click();
    await nextTick();

    const pinBtn = document.querySelector<HTMLButtonElement>('.sc-btn--secondary');
    expect(pinBtn).toBeTruthy();
    pinBtn!.click();
    await nextTick();
    expect(mockSkillTreeStore.pinQuest).toHaveBeenCalledWith('paid-brain');
    wrapper.unmount();
  });

  it('renders a viewport rectangle in the minimap', async () => {
    const wrapper = mount(SkillConstellation, { props: { visible: true }, attachTo: document.body });
    await nextTick();

    expect(document.querySelector('.sc-minimap-viewport')).toBeTruthy();
    wrapper.unmount();
  });
});
