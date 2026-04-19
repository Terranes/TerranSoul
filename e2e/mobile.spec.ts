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

  test('mobile bottom tab bar is visible and switches tabs', async ({ page }) => {
    await page.goto('/');

    // Mobile bottom tab bar should be visible at 375px width
    const bottomNav = page.locator('.mobile-bottom-nav');
    await expect(bottomNav).toBeVisible();

    // Desktop sidebar should be hidden on mobile
    await expect(page.locator('.desktop-nav')).not.toBeVisible();

    // Should have tab buttons for Chat, Quests, Memory, Market, Voice
    const tabs = bottomNav.locator('.mobile-tab');
    await expect(tabs).toHaveCount(5);

    // Chat tab should be active by default
    const chatTab = bottomNav.locator('.mobile-tab', { hasText: 'Chat' });
    await expect(chatTab).toHaveClass(/active/);
  });

  test('mobile tab navigation switches views', async ({ page }) => {
    await page.goto('/');

    // Verify we start on chat view
    await expect(page.locator('.chat-view')).toBeVisible();

    // Tap Memory tab
    const bottomNav = page.locator('.mobile-bottom-nav');
    const memoryTab = bottomNav.locator('.mobile-tab', { hasText: 'Memory' });
    await memoryTab.click();

    // Memory tab should now be active
    await expect(memoryTab).toHaveClass(/active/);
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
