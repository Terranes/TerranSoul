/**
 * Unit tests for `CompanionsPanel.vue` (chunk INTEGRATE-5).
 *
 * These hermetic tests assert the security/UX contract of the
 * quest-based guided installer surface:
 *
 *   1. `companions_run_guided_install` is NEVER called on mount or tab
 *      switch — it only ever runs after an explicit Install click.
 *   2. Clicking Install calls the Tauri command exactly once with the
 *      correct companion id, and the UI reflects an OS-elevation
 *      message so the user can see what is about to happen.
 *   3. Detect runs only on explicit click (no background polling).
 *   4. The Temporal.io card is rendered as design-reference-only —
 *      no Install button is present for it.
 *
 * This pairs with the Rust-side hermetic tests in
 * `src-tauri/src/integrations/companions.rs` and
 * `src-tauri/src/commands/companions.rs`, which assert the registry
 * shape and the subprocess-runner injection point. Together they cover
 * the INTEGRATE-5 acceptance criterion "OS-elevation step reachable
 * but the install command never runs without an explicit click"
 * without depending on a real OS UAC / pkexec / osascript dialog
 * (which is intentionally not Playwright-automatable).
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import CompanionsPanel from './CompanionsPanel.vue';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const sampleCompanions = [
  {
    id: 'hermes-desktop',
    display_name: 'Hermes Desktop',
    role: 'desktop AI assistant',
    official_url: 'https://example.invalid/hermes-desktop',
    windows_install: { program: 'winget', args: ['install', 'hermes'], description: 'winget install' },
    macos_install: null,
    linux_install: null,
    detect: { program: 'hermes', args: ['--version'], description: 'detect' },
    requires_elevation: true,
  },
  {
    id: 'hermes-agent',
    display_name: 'Hermes Agent',
    role: 'background AI agent',
    official_url: 'https://example.invalid/hermes-agent',
    windows_install: null,
    macos_install: null,
    linux_install: null,
    detect: null,
    requires_elevation: false,
  },
  {
    id: 'openclaw-cli',
    display_name: 'OpenClaw CLI',
    role: 'coding CLI',
    official_url: 'https://example.invalid/openclaw',
    windows_install: null,
    macos_install: null,
    linux_install: { program: 'sh', args: ['install.sh'], description: 'shell install' },
    detect: { program: 'openclaw', args: ['--version'], description: 'detect' },
    requires_elevation: false,
  },
];

describe('CompanionsPanel', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  afterEach(() => {
    mockInvoke.mockReset();
  });

  it('loads companions on mount via companions_list and never auto-installs', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'companions_list') return sampleCompanions;
      throw new Error(`Unmocked command: ${cmd}`);
    });

    const wrapper = mount(CompanionsPanel);
    await flushPromises();

    // companions_list called exactly once on mount
    const listCalls = mockInvoke.mock.calls.filter(c => c[0] === 'companions_list');
    expect(listCalls).toHaveLength(1);

    // companions_run_guided_install MUST NOT have been called on mount
    const installCalls = mockInvoke.mock.calls.filter(c => c[0] === 'companions_run_guided_install');
    expect(installCalls).toHaveLength(0);

    // companions_detect_one MUST NOT have been called on mount either
    const detectCalls = mockInvoke.mock.calls.filter(c => c[0] === 'companions_detect_one');
    expect(detectCalls).toHaveLength(0);

    // All 3 verified companion cards render
    for (const app of sampleCompanions) {
      expect(wrapper.find(`[data-testid="companion-card-${app.id}"]`).exists()).toBe(true);
    }

    // Temporal design-reference card renders
    expect(wrapper.find('[data-testid="companion-card-temporal-io"]').exists()).toBe(true);
  });

  it('Install button invokes companions_run_guided_install exactly once with correct id', async () => {
    mockInvoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'companions_list') return sampleCompanions;
      if (cmd === 'companions_run_guided_install') {
        expect(args).toEqual({ id: 'hermes-desktop' });
        return { RequiresElevation: { command: { program: 'winget', args: ['install'], description: 'x' } } };
      }
      throw new Error(`Unmocked command: ${cmd}`);
    });

    const wrapper = mount(CompanionsPanel);
    await flushPromises();

    await wrapper.find('[data-testid="companion-install-hermes-desktop"]').trigger('click');
    await flushPromises();

    const installCalls = mockInvoke.mock.calls.filter(c => c[0] === 'companions_run_guided_install');
    expect(installCalls).toHaveLength(1);
    expect(installCalls[0][1]).toEqual({ id: 'hermes-desktop' });

    // The UI must surface an OS-elevation message so the user sees what is
    // about to happen — this is the "OS-elevation step reachable" contract.
    const actionLine = wrapper.find('[data-testid="companion-action-hermes-desktop"]');
    expect(actionLine.exists()).toBe(true);
    expect(actionLine.text().toLowerCase()).toContain('elevation');
  });

  it('Detect button invokes companions_detect_one only on explicit click', async () => {
    mockInvoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'companions_list') return sampleCompanions;
      if (cmd === 'companions_detect_one') {
        expect(args).toEqual({ id: 'openclaw-cli' });
        return { Installed: { version: '1.2.3' } };
      }
      throw new Error(`Unmocked command: ${cmd}`);
    });

    const wrapper = mount(CompanionsPanel);
    await flushPromises();

    await wrapper.find('[data-testid="companion-detect-openclaw-cli"]').trigger('click');
    await flushPromises();

    const detectCalls = mockInvoke.mock.calls.filter(c => c[0] === 'companions_detect_one');
    expect(detectCalls).toHaveLength(1);

    const status = wrapper.find('[data-testid="companion-status-openclaw-cli"]');
    expect(status.text()).toContain('Installed');
    expect(status.text()).toContain('1.2.3');
  });

  it('Temporal design-reference card has NO Install button', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'companions_list') return sampleCompanions;
      throw new Error(`Unmocked command: ${cmd}`);
    });

    const wrapper = mount(CompanionsPanel);
    await flushPromises();

    expect(wrapper.find('[data-testid="companion-install-temporal-io"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="companion-detect-temporal-io"]').exists()).toBe(false);

    const card = wrapper.find('[data-testid="companion-card-temporal-io"]');
    expect(card.text().toLowerCase()).toContain('design reference only');
  });

  it('surfaces a load error if companions_list fails — and does not crash', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'companions_list') throw new Error('Tauri unavailable');
      throw new Error(`Unmocked: ${cmd}`);
    });

    const wrapper = mount(CompanionsPanel);
    await flushPromises();

    const err = wrapper.find('[data-testid="companions-load-error"]');
    expect(err.exists()).toBe(true);
    expect(err.text()).toContain('Tauri unavailable');
  });
});
