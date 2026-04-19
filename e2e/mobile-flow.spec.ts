/**
 * Mobile E2E — single comprehensive test covering the full mobile UX flow.
 *
 * Consolidates all mobile assertions (layout, bottom tab bar, chat, keyboard
 * handling, CSS properties, viewport meta) into one sequential test to minimize
 * page navigations and CI time.
 *
 * Runs at 375×667 (iPhone SE) against the Vite dev server (no Tauri backend).
 */
import { test, expect, type Page } from '@playwright/test';

const MOBILE_VIEWPORT = { width: 375, height: 667 };
const MESSAGE_TIMEOUT = 5_000;
const PANEL_TIMEOUT = 2_000;

// ─── Helpers ─────────────────────────────────────────────────────────────────

function collectConsoleErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      const text = msg.text();
      if (text.includes('__TAURI_INTERNALS__')) return;
      if (text.includes('process_prompt_silently')) return;
      if (text.includes('Vercel')) return;
      errors.push(text);
    }
  });
  page.on('pageerror', err => {
    errors.push(`UNCAUGHT: ${err.message}`);
  });
  return errors;
}

// ─── Test ────────────────────────────────────────────────────────────────────

test.describe('Mobile', () => {
  test.use({ viewport: MOBILE_VIEWPORT });

  test('mobile: full end-to-end flow', { timeout: 60_000 }, async ({ page }) => {
    const errors = collectConsoleErrors(page);
    await page.goto('/');

    // ── 1. App loads on mobile ──────────────────────────────────────────
    await expect(page.locator('.chat-view')).toBeVisible();
    await expect(page.locator('.viewport-layer')).toBeVisible();
    await expect(page.locator('.input-footer')).toBeVisible();

    // 3D canvas fills the mobile viewport
    const canvas = page.locator('.viewport-canvas');
    await expect(canvas).toBeVisible();
    const canvasBox = await canvas.boundingBox();
    expect(canvasBox).not.toBeNull();
    expect(canvasBox!.width).toBeGreaterThan(300);
    expect(canvasBox!.height).toBeGreaterThan(300);

    // ── 2. Mobile bottom tab bar visible, desktop nav hidden ────────────
    const bottomNav = page.locator('.mobile-bottom-nav');
    await expect(bottomNav).toBeVisible();
    await expect(page.locator('.desktop-nav')).not.toBeVisible();

    // 5 tabs: Chat, Quests, Memory, Market, Voice
    const tabs = bottomNav.locator('.mobile-tab');
    await expect(tabs).toHaveCount(5);

    // Chat tab active by default
    const chatTab = bottomNav.locator('.mobile-tab', { hasText: 'Chat' });
    await expect(chatTab).toHaveClass(/active/);

    // ── 3. Tab navigation switches views ────────────────────────────────
    const memoryTab = bottomNav.locator('.mobile-tab', { hasText: 'Memory' });
    await memoryTab.click();
    await expect(memoryTab).toHaveClass(/active/);

    // Navigate back to Chat
    await chatTab.click();
    await expect(page.locator('.chat-view')).toBeVisible();

    // ── 4. Modernized input wrapper ─────────────────────────────────────
    const inputWrapper = page.locator('.input-wrapper');
    await expect(inputWrapper).toBeVisible();

    const input = inputWrapper.locator('.chat-input');
    const sendBtn = inputWrapper.locator('.send-btn');
    await expect(input).toBeVisible();
    await expect(sendBtn).toBeVisible();

    // Input is border-less inside the wrapper
    const inputBorder = await input.evaluate(el => getComputedStyle(el).borderStyle);
    expect(inputBorder).toBe('none');

    // ── 5. Send message on mobile ───────────────────────────────────────
    await input.fill('Hi there!');
    await sendBtn.click();

    // Open chat drawer to see messages
    const drawerToggle = page.locator('.chat-drawer-toggle');
    await expect(drawerToggle).toBeVisible();
    await drawerToggle.click();
    await expect(page.locator('.chat-history')).toBeVisible({ timeout: PANEL_TIMEOUT });

    const userMsg = page.locator('.message-row.user').first();
    await expect(userMsg).toBeVisible({ timeout: MESSAGE_TIMEOUT });
    await expect(userMsg).toContainText('Hi there!');
    await expect(input).toHaveValue(''); // cleared after send

    // ── 6. Chat drawer max-height capped (50vh) ────────────────────────
    const panel = page.locator('.bottom-panel');
    const panelBox = await panel.boundingBox();
    expect(panelBox).not.toBeNull();
    expect(panelBox!.height).toBeLessThanOrEqual(MOBILE_VIEWPORT.height * 0.55 + 4);

    // Viewport layer still partially visible above
    const vpBox = await page.locator('.viewport-layer').boundingBox();
    expect(vpBox).not.toBeNull();
    expect(vpBox!.y).toBeCloseTo(0, 0);

    // Close drawer
    await drawerToggle.click();
    await expect(page.locator('.chat-history')).not.toBeVisible({ timeout: PANEL_TIMEOUT });

    // ── 7. Keyboard handling — CSS properties ───────────────────────────
    // viewport meta
    const viewportContent = await page.evaluate(() => {
      const meta = document.querySelector('meta[name="viewport"]');
      return meta?.getAttribute('content') ?? '';
    });
    expect(viewportContent).toContain('maximum-scale=1.0');
    expect(viewportContent).toContain('user-scalable=no');
    expect(viewportContent).toContain('interactive-widget=overlays-content');

    // touch-action: manipulation on html
    const htmlTouchAction = await page.evaluate(() =>
      getComputedStyle(document.documentElement).touchAction,
    );
    expect(htmlTouchAction).toBe('manipulation');

    // body is position:fixed (iOS keyboard shift prevention)
    const bodyPosition = await page.evaluate(() =>
      getComputedStyle(document.body).position,
    );
    expect(bodyPosition).toBe('fixed');

    // ── 8. Keyboard simulation — canvas stable, panel slides up ─────────
    const initialCanvasBox = await canvas.boundingBox();
    expect(initialCanvasBox).not.toBeNull();
    const initialCanvasTop = initialCanvasBox!.y;
    const initialPanelBox = await panel.boundingBox();
    expect(initialPanelBox).not.toBeNull();
    const initialPanelBottom = initialPanelBox!.y + initialPanelBox!.height;

    // Simulate keyboard open (300px keyboard)
    await page.evaluate(() => {
      const vv = window.visualViewport;
      if (vv) {
        Object.defineProperty(vv, 'height', { value: 367, configurable: true, writable: true });
        vv.dispatchEvent(new Event('resize'));
      }
    });
    await page.waitForTimeout(400);

    // Canvas should NOT have moved
    const afterCanvasBox = await canvas.boundingBox();
    expect(afterCanvasBox).not.toBeNull();
    expect(Math.abs(afterCanvasBox!.y - initialCanvasTop)).toBeLessThanOrEqual(2);

    // Panel bottom should have moved up significantly
    const afterPanelBox = await panel.boundingBox();
    if (afterPanelBox) {
      const afterPanelBottom = afterPanelBox.y + afterPanelBox.height;
      expect(initialPanelBottom - afterPanelBottom).toBeGreaterThan(100);
    }

    // Page scroll stays at 0,0
    const scrollPos = await page.evaluate(() => ({
      x: window.scrollX,
      y: window.scrollY,
    }));
    expect(scrollPos.x).toBe(0);
    expect(scrollPos.y).toBe(0);

    // ── 9. No critical console errors ───────────────────────────────────
    const crashErrors = errors.filter(e =>
      e.includes('Cannot read properties of undefined') ||
      e.includes('UNCAUGHT') ||
      e.includes('Unhandled error'),
    );
    expect(crashErrors).toHaveLength(0);
  });
});
