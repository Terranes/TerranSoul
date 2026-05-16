import { describe, it, expect, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import AppChromeActions from './AppChromeActions.vue';
import { useNotificationsStore } from '../../stores/notifications';

// Mount with Teleport disabled so the buttons render in-place for queries.
function makeWrapper() {
  return mount(AppChromeActions, {
    global: {
      stubs: {
        // Allow Teleport contents to render inline for testing.
        Teleport: { template: '<div><slot /></div>' },
      },
    },
  });
}

beforeEach(() => {
  setActivePinia(createPinia());
});

describe('AppChromeActions', () => {
  it('renders both settings and notifications buttons', () => {
    const wrapper = makeWrapper();
    expect(wrapper.find('[data-testid="app-chrome-settings"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="app-chrome-notifications"]').exists()).toBe(true);
  });

  it('emits open-settings when the gear button is clicked', async () => {
    const wrapper = makeWrapper();
    await wrapper.find('[data-testid="app-chrome-settings"]').trigger('click');
    expect(wrapper.emitted('open-settings')).toBeTruthy();
    expect(wrapper.emitted('open-settings')!.length).toBe(1);
  });

  it('toggles the notifications panel and emits open-notifications when bell clicked', async () => {
    const store = useNotificationsStore();
    const before = store.panelOpen;
    const wrapper = makeWrapper();
    await wrapper.find('[data-testid="app-chrome-notifications"]').trigger('click');
    expect(wrapper.emitted('open-notifications')).toBeTruthy();
    expect(store.panelOpen).toBe(!before);
  });

  it('shows the unread badge when the store has unread notifications', async () => {
    const store = useNotificationsStore();
    // Push an unread notification through the public API.
    store.pushNotification({
      kind: 'info',
      title: 'Test',
      body: 'A test notification',
    });
    const wrapper = makeWrapper();
    const badge = wrapper.find('[data-testid="app-chrome-notifications-badge"]');
    expect(badge.exists()).toBe(true);
    expect(badge.text()).toBe(String(store.unreadCount));
  });

  it('hides the unread badge when the store has no unread notifications', () => {
    const wrapper = makeWrapper();
    expect(
      wrapper.find('[data-testid="app-chrome-notifications-badge"]').exists(),
    ).toBe(false);
  });
});
