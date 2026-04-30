import { describe, it, expect, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import SelfImproveConfirmDialog from './SelfImproveConfirmDialog.vue';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

describe('SelfImproveConfirmDialog', () => {
  it('renders nothing when not visible', () => {
    const wrapper = mount(SelfImproveConfirmDialog, {
      props: { visible: false, hasCodingLlm: true, providerLabel: 'anthropic' },
      attachTo: document.body,
    });
    expect(document.querySelector('.si-confirm-card')).toBeNull();
    wrapper.unmount();
  });

  it('renders warning content when visible', async () => {
    const wrapper = mount(SelfImproveConfirmDialog, {
      props: { visible: true, hasCodingLlm: true, providerLabel: 'anthropic · sonnet' },
      attachTo: document.body,
    });
    await wrapper.vm.$nextTick();
    const card = document.querySelector('.si-confirm-card');
    expect(card).not.toBeNull();
    const text = card!.textContent ?? '';
    expect(text).toContain('Enable Self-Improve?');
    expect(text).toContain('autonomous coding');
    expect(text).toContain('feature branch');
    expect(text).toContain('anthropic · sonnet');
    wrapper.unmount();
  });

  it('shows the missing-LLM warning when hasCodingLlm is false', async () => {
    const wrapper = mount(SelfImproveConfirmDialog, {
      props: { visible: true, hasCodingLlm: false, providerLabel: 'unset' },
      attachTo: document.body,
    });
    await wrapper.vm.$nextTick();
    expect(document.querySelector('.si-warn')).not.toBeNull();
    wrapper.unmount();
  });

  it('emits confirm and cancel from buttons', async () => {
    const wrapper = mount(SelfImproveConfirmDialog, {
      props: { visible: true, hasCodingLlm: true, providerLabel: 'x' },
      attachTo: document.body,
    });
    await wrapper.vm.$nextTick();
    const buttons = document.querySelectorAll('.si-confirm-card button');
    const cancel = Array.from(buttons).find((b) => b.textContent?.includes('No')) as HTMLButtonElement;
    const confirm = Array.from(buttons).find((b) => b.textContent?.includes('Yes')) as HTMLButtonElement;
    cancel.click();
    confirm.click();
    expect(wrapper.emitted('cancel')).toBeTruthy();
    expect(wrapper.emitted('confirm')).toBeTruthy();
    wrapper.unmount();
  });
});
