import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import PetContextMenu from './PetContextMenu.vue';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

describe('PetContextMenu — panel items', () => {
  let wrapper: VueWrapper;

  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    wrapper?.unmount();
  });

  function mountMenu() {
    wrapper = mount(PetContextMenu, {
      props: { visible: true, x: 100, y: 100, resizeActive: false },
      attachTo: document.body,
    });
    return wrapper;
  }

  /** Query the teleported menu from document.body */
  function menuItems() {
    return document.querySelectorAll('.pet-ctx-menu .ctx-item');
  }

  function findByLabel(label: string) {
    return Array.from(menuItems()).find(
      (el) => el.querySelector('.ctx-label')?.textContent === label,
    ) as HTMLElement | undefined;
  }

  it('renders Panels submenu item', () => {
    mountMenu();
    const labels = Array.from(menuItems()).map(
      (el) => el.querySelector('.ctx-label')?.textContent,
    );
    expect(labels).toContain('Panels');
  });

  it('shows panel entries when Panels is clicked', async () => {
    mountMenu();
    const panelsItem = findByLabel('Panels');
    expect(panelsItem).toBeTruthy();
    panelsItem!.click();
    await wrapper.vm.$nextTick();

    const subLabels = Array.from(
      document.querySelectorAll('.pet-ctx-inline-sub .ctx-label'),
    ).map((el) => el.textContent);
    expect(subLabels).toContain('Brain');
    expect(subLabels).toContain('Knowledge');
    expect(subLabels).toContain('Quests');
    expect(subLabels).toContain('Marketplace');
    expect(subLabels).toContain('Voice');
  });

  it('emits close when a panel entry is clicked', async () => {
    mountMenu();
    // Open the Panels submenu
    findByLabel('Panels')!.click();
    await wrapper.vm.$nextTick();

    // Click the Brain entry
    const brainItem = Array.from(
      document.querySelectorAll('.ctx-item--sub'),
    ).find((el) => el.querySelector('.ctx-label')?.textContent === 'Brain') as HTMLElement;
    expect(brainItem).toBeTruthy();
    brainItem.click();
    await wrapper.vm.$nextTick();

    expect(wrapper.emitted('close')).toBeTruthy();
  });
});
