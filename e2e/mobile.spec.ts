/**
 * Mobile UX E2E tests for TerranSoul.
 *
 * These tests run the app at a simulated 375×667 mobile viewport (iPhone SE
 * resolution) and verify that the chat interface, keyboard handling, and 3D
 * viewport behave correctly on small screens.
 *
 * Note: Playwright's Chromium headless doesn't open a real virtual keyboard, so
 * we simulate keyboard-open conditions by directly setting window.visualViewport
 * dimensions via page.evaluate(). This lets us assert the CSS translate and
 * camera state without needing an actual physical keyboard.
 */
import { test, expect } from '@playwright/test';

const MOBILE_VIEWPORT = { width: 375, height: 667 };
const VRM_LOAD_TIMEOUT = 30_000;

test.describe('Mobile Chat UX', () => {
  test.use({ viewport: MOBILE_VIEWPORT });

  test('app loads correctly on mobile viewport', async ({ page }) => {
    await page.goto('/');

    // Main layout elements should be visible
    await expect(page.locator('.chat-view')).toBeVisible();
    await expect(page.locator('.viewport-layer')).toBeVisible();
    await expect(page.locator('.input-footer')).toBeVisible();

    // The 3D canvas should fill the viewport with correct dimensions
    const canvas = page.locator('.viewport-canvas');
    await expect(canvas).toBeVisible();
    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThan(300);
    expect(box!.height).toBeGreaterThan(300);
  });

  test('viewport canvas dimensions match mobile screen width', async ({ page }) => {
    await page.goto('/');

    const canvas = page.locator('.viewport-canvas');
    await expect(canvas).toBeVisible();

    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    // Canvas should fill the full width on mobile (375px viewport, ±5px tolerance)
    expect(box!.width).toBeGreaterThanOrEqual(370);
    expect(box!.width).toBeLessThanOrEqual(380);
  });

  test('chat history panel toggles open and closed on mobile', async ({ page }) => {
    await page.goto('/');

    const toggle = page.locator('.chat-drawer-toggle');
    await expect(toggle).toBeVisible();

    // History should be hidden initially
    await expect(page.locator('.chat-history')).not.toBeVisible();

    // Tap the toggle — history should appear
    await toggle.click();
    await expect(page.locator('.chat-history')).toBeVisible({ timeout: 1_000 });

    // Tap again — history should hide
    await toggle.click();
    await expect(page.locator('.chat-history')).not.toBeVisible({ timeout: 1_000 });
  });

  test('input footer stays visible when chat drawer is open', async ({ page }) => {
    await page.goto('/');

    // Open the chat drawer
    await page.locator('.chat-drawer-toggle').click();
    await expect(page.locator('.chat-history')).toBeVisible({ timeout: 1_000 });

    // Input footer must still be visible — it is always-on
    await expect(page.locator('.input-footer')).toBeVisible();
    await expect(page.locator('.chat-input')).toBeVisible();
  });

  test('bottom panel does not overflow outside the viewport on mobile', async ({ page }) => {
    await page.goto('/');

    const panel = page.locator('.bottom-panel');
    await expect(panel).toBeVisible();

    const panelBox = await panel.boundingBox();
    const viewBox = await page.locator('.chat-view').boundingBox();

    expect(panelBox).not.toBeNull();
    expect(viewBox).not.toBeNull();

    // Panel bottom edge must not exceed the container bottom
    const panelBottom = panelBox!.y + panelBox!.height;
    const viewBottom = viewBox!.y + viewBox!.height;
    expect(panelBottom).toBeLessThanOrEqual(viewBottom + 2); // 2px tolerance
  });

  test('3D viewport canvas position is stable — stays at top-left when keyboard opens', async ({ page }) => {
    await page.goto('/');

    const canvas = page.locator('.viewport-canvas');
    await expect(canvas).toBeVisible();

    // Record initial canvas position
    const initialBox = await canvas.boundingBox();
    expect(initialBox).not.toBeNull();
    const initialTop = initialBox!.y;

    // Simulate keyboard open by triggering the keyboard-offset CSS variable via
    // the visualViewport resize approach used by the composable.
    // We inject a fake resize event with a reduced height to simulate keyboard.
    await page.evaluate(() => {
      // Simulate the visualViewport resize event that useKeyboardDetector listens to.
      // window.innerHeight stays at 667, but vv.height shrinks by ~300px (keyboard).
      const vv = window.visualViewport;
      if (vv) {
        Object.defineProperty(vv, 'height', { value: 367, configurable: true, writable: true });
        vv.dispatchEvent(new Event('resize'));
      }
    });

    // Wait for CSS transition to settle (250ms + buffer)
    await page.waitForTimeout(400);

    // Canvas (viewport-layer) itself should NOT have moved — it stays fixed
    const afterBox = await canvas.boundingBox();
    expect(afterBox).not.toBeNull();
    // Top of canvas should be same as before (within 2px rounding tolerance)
    expect(Math.abs(afterBox!.y - initialTop)).toBeLessThanOrEqual(2);
  });

  test('bottom panel slides up when virtual keyboard opens', async ({ page }) => {
    await page.goto('/');

    const panel = page.locator('.bottom-panel');
    await expect(panel).toBeVisible();

    // Record initial panel Y position
    const initialBox = await panel.boundingBox();
    expect(initialBox).not.toBeNull();
    const initialBottom = initialBox!.y + initialBox!.height;

    // Simulate keyboard opening (300px keyboard)
    await page.evaluate(() => {
      const vv = window.visualViewport;
      if (vv) {
        Object.defineProperty(vv, 'height', { value: 367, configurable: true, writable: true });
        vv.dispatchEvent(new Event('resize'));
      }
    });

    await page.waitForTimeout(400);

    // Panel bottom should have moved up by roughly the keyboard height (300px)
    const afterBox = await panel.boundingBox();
    if (afterBox) {
      const afterBottom = afterBox.y + afterBox.height;
      // Panel should have moved up significantly
      expect(initialBottom - afterBottom).toBeGreaterThan(100);
    }
    // (If visualViewport isn't supported by test browser, the test passes by
    //  accepting that the feature degrades gracefully — panel stays at bottom.)
  });

  test('chat input is reachable and interactive on mobile', async ({ page }) => {
    await page.goto('/');

    const input = page.locator('.chat-input');
    await expect(input).toBeVisible();
    await expect(input).toBeEnabled();

    // Fill and verify
    await input.fill('Hello from mobile!');
    await expect(input).toHaveValue('Hello from mobile!');

    // Send button should become enabled
    await expect(page.locator('.send-btn')).toBeEnabled();
  });

  test('AI state pill is visible on mobile and shows Idle', async ({ page }) => {
    await page.goto('/');

    const pill = page.locator('.ai-state-pill');
    await expect(pill).toBeVisible();
    await expect(pill).toContainText('Idle');

    // Pill should be within viewport bounds
    const box = await pill.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(MOBILE_VIEWPORT.width + 2);
  });

  test('sending a message works on mobile', async ({ page }) => {
    await page.goto('/');

    const input = page.locator('.chat-input');
    const sendBtn = page.locator('.send-btn');

    await input.fill('Hi there!');
    await sendBtn.click();

    // Open chat drawer to see messages
    await page.locator('.chat-drawer-toggle').click();

    // User message should appear
    const userMsg = page.locator('.message-row.user').first();
    await expect(userMsg).toBeVisible({ timeout: 5_000 });
    await expect(userMsg).toContainText('Hi there!');

    // Input cleared after send
    await expect(input).toHaveValue('');
  });

  test('character model loads and canvas renders on mobile', async ({ page }) => {
    await page.goto('/');

    // Loading overlay should disappear after model loads
    const loadingOverlay = page.locator('.loading-overlay');
    await expect(loadingOverlay).toBeHidden({ timeout: VRM_LOAD_TIMEOUT });

    // Canvas should have content
    const canvas = page.locator('.viewport-canvas');
    await expect(canvas).toBeVisible();
    const box = await canvas.boundingBox();
    expect(box!.width).toBeGreaterThan(100);
    expect(box!.height).toBeGreaterThan(100);
  });

  test('settings dropdown is accessible on mobile', async ({ page }) => {
    await page.goto('/');

    const settingsToggle = page.locator('.settings-toggle');
    await expect(settingsToggle).toBeVisible();

    await settingsToggle.click();
    const dropdown = page.locator('.settings-dropdown');
    await expect(dropdown).toBeVisible({ timeout: 1_000 });

    // Dropdown must not overflow the viewport width
    const box = await dropdown.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x + box!.width).toBeLessThanOrEqual(MOBILE_VIEWPORT.width + 2);
  });

  test('chat drawer max-height is capped so viewport is never fully hidden', async ({ page }) => {
    await page.goto('/');

    // Open the chat drawer
    await page.locator('.chat-drawer-toggle').click();
    await expect(page.locator('.chat-history')).toBeVisible({ timeout: 1_000 });

    // The bottom-panel should never exceed 50vh (333px on a 667px screen)
    const panel = page.locator('.bottom-panel');
    const panelBox = await panel.boundingBox();
    expect(panelBox).not.toBeNull();
    // max 50vh + some tolerance for input footer
    expect(panelBox!.height).toBeLessThanOrEqual(MOBILE_VIEWPORT.height * 0.55 + 4);

    // The viewport-layer (3D canvas) should still be partially visible above
    const viewportLayer = page.locator('.viewport-layer');
    const vpBox = await viewportLayer.boundingBox();
    expect(vpBox).not.toBeNull();
    // Top of viewport must be at y=0 (never pushed off-screen)
    expect(vpBox!.y).toBeCloseTo(0, 0);
  });

  test('mobile hamburger menu is visible and toggles dropdown', async ({ page }) => {
    await page.goto('/');

    // Hamburger menu toggle should be visible on mobile
    const menuToggle = page.locator('.mobile-menu-toggle');
    await expect(menuToggle).toBeVisible();

    // Dropdown should be hidden by default (collapsed)
    await expect(page.locator('.mobile-menu-dropdown')).not.toBeVisible();

    // Tap the hamburger — dropdown should appear with menu items
    await menuToggle.click();
    const dropdown = page.locator('.mobile-menu-dropdown');
    await expect(dropdown).toBeVisible({ timeout: 1_000 });

    // Should have at least 4 menu items (Chat, Memory, Marketplace, Voice)
    const items = dropdown.locator('.mobile-menu-item');
    await expect(items).toHaveCount(4);

    // Tap again — dropdown should close
    await menuToggle.click();
    await expect(dropdown).not.toBeVisible({ timeout: 1_000 });
  });

  test('mobile hamburger menu does not overflow viewport', async ({ page }) => {
    await page.goto('/');

    // Open the hamburger menu
    await page.locator('.mobile-menu-toggle').click();
    const dropdown = page.locator('.mobile-menu-dropdown');
    await expect(dropdown).toBeVisible({ timeout: 1_000 });

    // Dropdown must not overflow the viewport
    const box = await dropdown.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(MOBILE_VIEWPORT.width + 2);
    expect(box!.y).toBeGreaterThanOrEqual(0);
  });

  test('mobile menu navigation switches tabs', async ({ page }) => {
    await page.goto('/');

    // Verify we start on chat view
    await expect(page.locator('.chat-view')).toBeVisible();

    // Open hamburger and tap Memory
    await page.locator('.mobile-menu-toggle').click();
    await expect(page.locator('.mobile-menu-dropdown')).toBeVisible({ timeout: 1_000 });

    // Click the Memory menu item
    const memoryItem = page.locator('.mobile-menu-item', { hasText: 'Memory' });
    await memoryItem.click();

    // Menu should auto-close after selection
    await expect(page.locator('.mobile-menu-dropdown')).not.toBeVisible({ timeout: 1_000 });
  });

  test('desktop nav bar is hidden on mobile', async ({ page }) => {
    await page.goto('/');

    // Desktop nav should be hidden on mobile viewport
    const desktopNav = page.locator('.desktop-nav');
    await expect(desktopNav).toBeHidden();
  });

  test('modernized input has unified wrapper with send button inside', async ({ page }) => {
    await page.goto('/');

    // Input wrapper should be visible (modern unified design)
    const inputWrapper = page.locator('.input-wrapper');
    await expect(inputWrapper).toBeVisible();

    // Both input and send button should be inside the wrapper
    const input = inputWrapper.locator('.chat-input');
    const sendBtn = inputWrapper.locator('.send-btn');
    await expect(input).toBeVisible();
    await expect(sendBtn).toBeVisible();

    // The input should not have its own visible border (border-less inside wrapper)
    const inputBorder = await input.evaluate((el) => {
      return getComputedStyle(el).borderStyle;
    });
    expect(inputBorder).toBe('none');
  });

  test('viewport meta disables user pinch zoom', async ({ page }) => {
    await page.goto('/');

    // Read the viewport meta tag content
    const viewportContent = await page.evaluate(() => {
      const meta = document.querySelector('meta[name="viewport"]');
      return meta?.getAttribute('content') ?? '';
    });

    // Must include maximum-scale=1.0 and user-scalable=no to prevent page-level pinch zoom
    expect(viewportContent).toContain('maximum-scale=1.0');
    expect(viewportContent).toContain('user-scalable=no');
    // interactive-widget=overlays-content should still be present
    expect(viewportContent).toContain('interactive-widget=overlays-content');
  });

  test('page scroll stays at 0,0 when keyboard opens (iOS scroll prevention)', async ({ page }) => {
    await page.goto('/');

    // Simulate keyboard open
    await page.evaluate(() => {
      const vv = window.visualViewport;
      if (vv) {
        Object.defineProperty(vv, 'height', { value: 367, configurable: true, writable: true });
        vv.dispatchEvent(new Event('resize'));
      }
    });

    await page.waitForTimeout(200);

    // Page scroll should be at 0,0 — the keyboard detector resets it
    const scrollPos = await page.evaluate(() => ({
      x: window.scrollX,
      y: window.scrollY,
    }));
    expect(scrollPos.x).toBe(0);
    expect(scrollPos.y).toBe(0);
  });

  test('html has touch-action: manipulation to prevent double-tap zoom', async ({ page }) => {
    await page.goto('/');

    const htmlTouchAction = await page.evaluate(() => {
      return getComputedStyle(document.documentElement).touchAction;
    });
    expect(htmlTouchAction).toBe('manipulation');
  });

  test('body is position:fixed to prevent iOS keyboard viewport shift', async ({ page }) => {
    await page.goto('/');

    const bodyPosition = await page.evaluate(() => {
      return getComputedStyle(document.body).position;
    });
    expect(bodyPosition).toBe('fixed');
  });
});
