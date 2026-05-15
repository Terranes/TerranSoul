import { describe, it, expect, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import PanelShell from './PanelShell.vue';

describe('PanelShell', () => {
  it('renders the title in the default header', () => {
    const wrapper = mount(PanelShell, {
      props: { title: 'System Info', testId: 'sys' },
    });
    expect(wrapper.find('[data-testid="sys"]').exists()).toBe(true);
    expect(wrapper.get('.panel-shell__title').text()).toBe('System Info');
  });

  it('hides the close button when onClose is not provided', () => {
    const wrapper = mount(PanelShell, { props: { title: 'Hello' } });
    expect(wrapper.find('[data-testid="panel-shell-close"]').exists()).toBe(false);
  });

  it('renders the close button when onClose is provided and invokes it', async () => {
    const onClose = vi.fn();
    const wrapper = mount(PanelShell, { props: { title: 'Hello', onClose } });
    const btn = wrapper.get('[data-testid="panel-shell-close"]');
    await btn.trigger('click');
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted('close')).toHaveLength(1);
  });

  it('closes on backdrop click for overlay variants when onClose is provided', async () => {
    const onClose = vi.fn();
    const wrapper = mount(PanelShell, {
      props: { variant: 'overlay-fixed', title: 'X', onClose },
    });
    await wrapper.trigger('click');
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('does not close on backdrop click when closeOnBackdrop=false', async () => {
    const onClose = vi.fn();
    const wrapper = mount(PanelShell, {
      props: { variant: 'overlay-fixed', title: 'X', onClose, closeOnBackdrop: false },
    });
    await wrapper.trigger('click');
    expect(onClose).not.toHaveBeenCalled();
  });

  it('does not close when clicking inside the card', async () => {
    const onClose = vi.fn();
    const wrapper = mount(PanelShell, {
      props: { variant: 'overlay-fixed', title: 'X', onClose },
      slots: { default: '<p class="inner">content</p>' },
    });
    await wrapper.get('.inner').trigger('click');
    expect(onClose).not.toHaveBeenCalled();
  });

  it('applies the variant modifier class', () => {
    const wrapper = mount(PanelShell, {
      props: { variant: 'embedded', title: 'Embed' },
    });
    expect(wrapper.classes()).toContain('panel-shell--embedded');
    expect(wrapper.element.tagName.toLowerCase()).toBe('section');
  });

  it('renders custom header slot in place of default title', () => {
    const wrapper = mount(PanelShell, {
      slots: { header: '<span class="custom-header">Custom</span>' },
    });
    expect(wrapper.find('.custom-header').exists()).toBe(true);
    expect(wrapper.find('.panel-shell__title').exists()).toBe(false);
  });

  it('renders actions and footer slots', () => {
    const wrapper = mount(PanelShell, {
      props: { title: 't', onClose: () => {} },
      slots: {
        actions: '<button class="act">Act</button>',
        footer: '<button class="ft">OK</button>',
      },
    });
    expect(wrapper.find('.act').exists()).toBe(true);
    expect(wrapper.find('.ft').exists()).toBe(true);
    expect(wrapper.find('.panel-shell__footer').exists()).toBe(true);
  });

  it('omits the header entirely when no title/slots/onClose given', () => {
    const wrapper = mount(PanelShell, { props: { variant: 'embedded' } });
    expect(wrapper.find('.panel-shell__header').exists()).toBe(false);
  });

  it('omits the footer when the footer slot is empty', () => {
    const wrapper = mount(PanelShell, { props: { title: 't' } });
    expect(wrapper.find('.panel-shell__footer').exists()).toBe(false);
  });

  it('forwards default slot content into the body', () => {
    const wrapper = mount(PanelShell, {
      props: { title: 't' },
      slots: { default: '<p class="body-content">Hello body</p>' },
    });
    expect(wrapper.get('.panel-shell__body .body-content').text()).toBe('Hello body');
  });

  it('honors the `as` prop to override the root element tag', () => {
    const wrapper = mount(PanelShell, {
      props: { variant: 'embedded', as: 'aside', title: 't' },
    });
    expect(wrapper.element.tagName.toLowerCase()).toBe('aside');
  });
});
