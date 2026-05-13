/**
 * Tests for the GraphControlPanel search-input keyboard shortcuts and the
 * three match-action buttons. These shortcuts are how both MemoryGraph and
 * MemoryGraph3D let users build the persistent selection without the mouse:
 *
 *   Enter        → select-matches  (replace selection with current matches)
 *   Shift+Enter  → add-matches     (union into selection)
 *   Alt+Enter    → remove-matches  (subtract from selection)
 *
 * The button row mirrors the same three verbs so users who don't know the
 * shortcuts still have first-class access.
 */
import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import GraphControlPanel from './GraphControlPanel.vue';

function baseProps(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    collapsed: false,
    nodeCount: 10,
    edgeCount: 5,
    showOrphans: true,
    minDegree: 0,
    searchText: 'foo',
    searchMode: 'contains',
    searchFields: { label: true, tags: true, body: false, community: true },
    highlightFilterActive: true,
    matchCount: 3,
    selectedCount: 2,
    visibleNodeCount: 10,
    legend: [],
    showLabels: true,
    showArrows: false,
    textFadeThreshold: 0.5,
    nodeSizeMul: 1,
    linkWidthMul: 1,
    repulsion: -200,
    linkDistance: 60,
    gravity: 0.05,
    ...overrides,
  };
}

describe('GraphControlPanel — search input keyboard shortcuts', () => {
  it('Enter emits select-matches', async () => {
    const wrapper = mount(GraphControlPanel, { props: baseProps() });
    const input = wrapper.find('[data-testid="gcp-search-input"]');
    await input.trigger('keydown', { key: 'Enter' });
    expect(wrapper.emitted('select-matches')).toBeTruthy();
    expect(wrapper.emitted('add-matches')).toBeFalsy();
    expect(wrapper.emitted('remove-matches')).toBeFalsy();
  });

  it('Shift+Enter emits add-matches', async () => {
    const wrapper = mount(GraphControlPanel, { props: baseProps() });
    const input = wrapper.find('[data-testid="gcp-search-input"]');
    await input.trigger('keydown', { key: 'Enter', shiftKey: true });
    expect(wrapper.emitted('add-matches')).toBeTruthy();
    expect(wrapper.emitted('select-matches')).toBeFalsy();
    expect(wrapper.emitted('remove-matches')).toBeFalsy();
  });

  it('Alt+Enter emits remove-matches', async () => {
    const wrapper = mount(GraphControlPanel, { props: baseProps() });
    const input = wrapper.find('[data-testid="gcp-search-input"]');
    await input.trigger('keydown', { key: 'Enter', altKey: true });
    expect(wrapper.emitted('remove-matches')).toBeTruthy();
    expect(wrapper.emitted('select-matches')).toBeFalsy();
    expect(wrapper.emitted('add-matches')).toBeFalsy();
  });

  it('Escape clears the search text only (emits update:searchText with empty string)', async () => {
    const wrapper = mount(GraphControlPanel, { props: baseProps() });
    const input = wrapper.find('[data-testid="gcp-search-input"]');
    await input.trigger('keydown', { key: 'Escape' });
    const evt = wrapper.emitted('update:searchText') as unknown[][] | undefined;
    expect(evt).toBeTruthy();
    expect(evt![0]).toEqual(['']);
  });

  it('plain typing emits update:searchText with the input value', async () => {
    const wrapper = mount(GraphControlPanel, { props: baseProps() });
    const input = wrapper.find<HTMLInputElement>('[data-testid="gcp-search-input"]');
    await input.setValue('bar');
    const evt = wrapper.emitted('update:searchText') as unknown[][] | undefined;
    expect(evt).toBeTruthy();
    expect(evt![evt!.length - 1]).toEqual(['bar']);
  });
});

describe('GraphControlPanel — match action buttons', () => {
  it('Select matches button emits select-matches', async () => {
    const wrapper = mount(GraphControlPanel, { props: baseProps() });
    await wrapper.find('[data-testid="gcp-select-matches"]').trigger('click');
    expect(wrapper.emitted('select-matches')).toBeTruthy();
  });

  it('Add matches button emits add-matches', async () => {
    const wrapper = mount(GraphControlPanel, { props: baseProps() });
    await wrapper.find('[data-testid="gcp-add-matches"]').trigger('click');
    expect(wrapper.emitted('add-matches')).toBeTruthy();
  });

  it('Remove matches button emits remove-matches', async () => {
    const wrapper = mount(GraphControlPanel, { props: baseProps() });
    await wrapper.find('[data-testid="gcp-remove-matches"]').trigger('click');
    expect(wrapper.emitted('remove-matches')).toBeTruthy();
  });

  it('all three buttons are disabled when there are zero matches', () => {
    const wrapper = mount(GraphControlPanel, { props: baseProps({ matchCount: 0 }) });
    const sel = wrapper.find<HTMLButtonElement>('[data-testid="gcp-select-matches"]');
    const add = wrapper.find<HTMLButtonElement>('[data-testid="gcp-add-matches"]');
    const rem = wrapper.find<HTMLButtonElement>('[data-testid="gcp-remove-matches"]');
    expect(sel.element.disabled).toBe(true);
    expect(add.element.disabled).toBe(true);
    expect(rem.element.disabled).toBe(true);
  });

  it('Remove matches stays disabled when selection is empty even if matches exist', () => {
    const wrapper = mount(GraphControlPanel, {
      props: baseProps({ matchCount: 3, selectedCount: 0 }),
    });
    const rem = wrapper.find<HTMLButtonElement>('[data-testid="gcp-remove-matches"]');
    expect(rem.element.disabled).toBe(true);
  });
});

describe('GraphControlPanel — search-field scope checkboxes', () => {
  it('toggling Label emits update:searchField with {field:"label", value:false}', async () => {
    const wrapper = mount(GraphControlPanel, { props: baseProps() });
    const cb = wrapper.find<HTMLInputElement>('[data-testid="gcp-field-label"]');
    expect(cb.element.checked).toBe(true);
    cb.element.checked = false;
    await cb.trigger('change');
    const evt = wrapper.emitted('update:searchField') as unknown[][] | undefined;
    expect(evt).toBeTruthy();
    expect(evt![0]).toEqual([{ field: 'label', value: false }]);
  });

  it('toggling Body checkbox on emits {field:"body", value:true}', async () => {
    const wrapper = mount(GraphControlPanel, { props: baseProps() });
    const cb = wrapper.find<HTMLInputElement>('[data-testid="gcp-field-body"]');
    expect(cb.element.checked).toBe(false);
    cb.element.checked = true;
    await cb.trigger('change');
    const evt = wrapper.emitted('update:searchField') as unknown[][] | undefined;
    expect(evt).toBeTruthy();
    expect(evt![0]).toEqual([{ field: 'body', value: true }]);
  });
});
