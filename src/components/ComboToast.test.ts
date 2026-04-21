import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { ref, nextTick, type Ref } from 'vue';
import ComboToast from './ComboToast.vue';
import { useSkillTreeStore } from '../stores/skill-tree';

vi.mock('../stores/skill-tree');

interface FakeCombo {
  sourceSkill: string;
  combo: { name: string; description: string; icon: string; withSkills: string[] };
}

const activeCombos: Ref<FakeCombo[]> = ref([]);
const tracker = ref<{ seenComboKeys: string[] }>({ seenComboKeys: [] });

const mockStore = {
  get activeCombos() { return activeCombos.value; },
  get tracker() { return tracker.value; },
  nodes: [
    { id: 'bgm', name: 'Jukebox' },
    { id: 'pet-mode', name: 'Watch Party' },
  ],
  markCombosSeen: vi.fn((keys: string[]) => {
    tracker.value.seenComboKeys = [...new Set([...tracker.value.seenComboKeys, ...keys])];
  }),
};

beforeEach(() => {
  activeCombos.value = [];
  tracker.value = { seenComboKeys: [] };
  mockStore.markCombosSeen.mockClear();
  vi.mocked(useSkillTreeStore).mockReturnValue(mockStore as never);
});

afterEach(() => {
  document.body.innerHTML = '';
});

describe('ComboToast', () => {
  it('renders the toast stack container', () => {
    const wrapper = mount(ComboToast, { attachTo: document.body });
    expect(document.querySelector('[data-testid="combo-toast-stack"]')).toBeTruthy();
    wrapper.unmount();
  });

  it('shows a toast for a new combo and marks it seen', async () => {
    const wrapper = mount(ComboToast, { attachTo: document.body });

    activeCombos.value = [{
      sourceSkill: 'bgm',
      combo: { name: 'DJ Companion', description: 'Pet picks the music.', icon: '🎧', withSkills: ['pet-mode'] },
    }];
    await nextTick();
    await nextTick();

    expect(document.querySelector('[data-testid="combo-toast-bgm__DJ Companion"]')).toBeTruthy();
    expect(document.body.textContent).toContain('DJ Companion');
    expect(document.body.textContent).toContain('Combo Unlocked');
    expect(mockStore.markCombosSeen).toHaveBeenCalledWith(['bgm::DJ Companion']);
    wrapper.unmount();
  });

  it('does not re-show a combo that has already been seen', async () => {
    tracker.value.seenComboKeys = ['bgm::DJ Companion'];
    const wrapper = mount(ComboToast, { attachTo: document.body });

    activeCombos.value = [{
      sourceSkill: 'bgm',
      combo: { name: 'DJ Companion', description: 'x', icon: '🎧', withSkills: ['pet-mode'] },
    }];
    await nextTick();
    await nextTick();

    expect(document.querySelector('[data-testid^="combo-toast-bgm"]')).toBeNull();
    expect(mockStore.markCombosSeen).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it('dismisses the toast when its close button is clicked', async () => {
    const wrapper = mount(ComboToast, { attachTo: document.body });

    activeCombos.value = [{
      sourceSkill: 'bgm',
      combo: { name: 'DJ Companion', description: 'x', icon: '🎧', withSkills: ['pet-mode'] },
    }];
    await nextTick();
    await nextTick();

    const closeBtn = document.querySelector<HTMLButtonElement>('.ct-close')!;
    expect(closeBtn).toBeTruthy();
    closeBtn.click();
    await nextTick();
    closeBtn.click();
    wrapper.unmount();
  });
});
