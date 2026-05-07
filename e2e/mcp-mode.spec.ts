/**
 * MCP Mode E2E — tests the MCP mode UI/UX.
 *
 * Simulates MCP mode by injecting isMcpMode=true into the Pinia window store
 * after the app initialises. This triggers Vue reactivity to show:
 *  1. McpActivityPanel (visible and toggleable)
 *  2. Full tabbed UI instead of pet mode
 *  3. Brain/Memory tabs accessible and rendering
 *  4. Chat functional (can ask TerranSoul questions)
 *  5. No critical console errors
 */
import { test, expect } from '@playwright/test';
import {
  collectConsoleErrors,
  assertNoCrashErrors,
  waitForAppReady,
  TIMEOUTS,
} from './helpers';

/**
 * Inject MCP mode flag into the Pinia window store at runtime.
 * Vue reactivity causes the UI to re-render with MCP-specific elements.
 */
async function enableMcpMode(page: import('@playwright/test').Page) {
  await page.evaluate(() => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    if (!app) return;
    const pinia = app.config.globalProperties.$pinia;
    if (!pinia) return;
    const windowState = pinia.state.value.window;
    if (windowState) {
      windowState.isMcpMode = true;
    }
  });
  // Give Vue a tick to re-render
  await page.waitForTimeout(100);
}

test.describe('MCP Mode UI', () => {
  test.beforeEach(async ({ page }) => {
    const errors = collectConsoleErrors(page);
    await page.goto('/');
    await waitForAppReady(page);
    await enableMcpMode(page);
  });

  test('MCP Activity Panel is visible and toggleable', async ({ page }) => {
    const errors = collectConsoleErrors(page);

    // The McpActivityPanel should be rendered
    const panel = page.locator('[data-testid="mcp-activity-panel"]');
    await expect(panel).toBeVisible({ timeout: 3_000 });

    // Should show MCP mode label
    await expect(panel.locator('.mcp-activity__mode')).toContainText('MCP');

    // Should show the message area (expanded by default)
    const message = panel.locator('.mcp-activity__message');
    await expect(message).toBeVisible();

    // Click the toggle button to collapse
    const toggle = page.locator('[data-testid="mcp-activity-toggle"]');
    await expect(toggle).toBeVisible();
    await toggle.click();

    // Message should now be hidden (collapsed)
    await expect(message).not.toBeVisible();

    // Panel itself still visible (just the header)
    await expect(panel).toBeVisible();

    // Click toggle again to expand
    await toggle.click();
    await expect(message).toBeVisible();

    assertNoCrashErrors(errors);
  });

  test('Full tabbed UI shows in MCP mode (not pet mode)', async ({ page }) => {
    const errors = collectConsoleErrors(page);

    // Desktop nav should be visible
    const desktopNav = page.locator('.desktop-nav');
    await expect(desktopNav).toBeVisible({ timeout: 3_000 });

    // Should have Brain and Memory tabs
    const brainTab = page.locator('.nav-btn', { hasText: 'Brain' });
    const memoryTab = page.locator('.nav-btn', { hasText: 'Memory' });
    await expect(brainTab).toBeVisible();
    await expect(memoryTab).toBeVisible();

    // Should NOT be in pet mode wrapper
    const petWrapper = page.locator('.pet-mode-wrapper');
    await expect(petWrapper).not.toBeVisible();

    assertNoCrashErrors(errors);
  });

  test('Brain tab loads in MCP mode', async ({ page }) => {
    const errors = collectConsoleErrors(page);

    // Navigate to Brain tab
    const brainTab = page.locator('.nav-btn', { hasText: 'Brain' });
    await brainTab.click();

    // BrainView should appear (uses data-testid or class)
    const brainView = page.locator('[data-testid="brain-view"]');
    await expect(brainView).toBeVisible({ timeout: 5_000 });

    assertNoCrashErrors(errors);
  });

  test('Memory tab loads in MCP mode', async ({ page }) => {
    const errors = collectConsoleErrors(page);

    // Navigate to Memory tab
    const memoryTab = page.locator('.nav-btn', { hasText: 'Memory' });
    await memoryTab.click();

    // MemoryView should appear
    const memoryView = page.locator('.memory-view');
    await expect(memoryView).toBeVisible({ timeout: 5_000 });

    assertNoCrashErrors(errors);
  });

  test('Chat is functional in MCP mode', async ({ page }) => {
    const errors = collectConsoleErrors(page);

    // Chat view should be visible (it's the default active tab)
    const chatView = page.locator('.chat-view');
    await expect(chatView).toBeVisible();

    // Chat input should be present and enabled
    const chatInput = page.locator('.chat-input');
    await expect(chatInput).toBeVisible({ timeout: 3_000 });
    await expect(chatInput).toBeEnabled();

    // Can type into the input
    await chatInput.fill('What are you working on?');
    await expect(chatInput).toHaveValue('What are you working on?');

    // Send button should be enabled when text is present
    const sendBtn = page.locator('.send-btn');
    await expect(sendBtn).toBeEnabled();

    assertNoCrashErrors(errors);
  });

  test('MCP badge shown in nav sidebar', async ({ page }) => {
    const errors = collectConsoleErrors(page);

    // The MCP badge should appear in the nav
    const mcpBadge = page.locator('.nav-mcp-badge');
    await expect(mcpBadge).toBeVisible({ timeout: 3_000 });
    await expect(mcpBadge).toContainText('MCP');

    assertNoCrashErrors(errors);
  });
});
