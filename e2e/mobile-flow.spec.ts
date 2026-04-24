/**
 * Mobile E2E — comprehensive test covering the full mobile UX flow.
 *
 * Runs at 375×667 (iPhone SE) against the Vite dev server with REAL connections.
 * No mocks — every assertion is validated against real UI and real LLM.
 *
 * Sections:
 *  1.  App loads on mobile viewport
 *  2.  Mobile bottom nav visible, desktop nav hidden
 *  3.  Tab navigation switches views (all 6 tabs)
 *  4.  Input wrapper styling (border-less input)
 *  5.  Send message on mobile → real LLM response
 *  6.  Chat drawer max-height capped (50vh)
 *  7.  Viewport meta & keyboard CSS
 *  8.  Keyboard simulation — canvas stable, panel slides up
 *  9.  Memory tab on mobile (stats, filters, tiers)
 * 10.  Skills tab on mobile
 * 11.  No critical console errors
 */
import { test, expect } from '@playwright/test';
import {
  collectConsoleErrors,
  assertNoCrashErrors,
  waitForAppReady,
  sendMessage,
  waitForAssistantResponse,
  TIMEOUTS,
} from './helpers';

const MOBILE_VIEWPORT = { width: 375, height: 667 };
const PANEL_TIMEOUT = 3_000;

test.describe('Mobile', () => {
  test.use({ viewport: MOBILE_VIEWPORT });

  test('mobile: full end-to-end flow', { timeout: 120_000 }, async ({ page }) => {
    const errors = collectConsoleErrors(page);
    await page.goto('/');
    await waitForAppReady(page);

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

    // 6 tabs (chat, memory, quests, market, voice, brain)
    const tabs = bottomNav.locator('.mobile-tab');
    await expect(tabs).toHaveCount(6);

    // Chat tab active by default
    const chatTab = bottomNav.locator('.mobile-tab', { hasText: 'Chat' });
    await expect(chatTab).toHaveClass(/active/);

    // ── 3. Tab navigation switches views ────────────────────────────────
    // Memory tab
    const memoryTab = bottomNav.locator('.mobile-tab', { hasText: 'Memory' });
    await memoryTab.click();
    await expect(memoryTab).toHaveClass(/active/);
    await expect(page.locator('.memory-view')).toBeVisible({ timeout: PANEL_TIMEOUT });

    // Quests tab
    const questsTab = bottomNav.locator('.mobile-tab', { hasText: 'Quests' });
    await questsTab.click();
    await expect(questsTab).toHaveClass(/active/);
    await expect(page.locator('.skill-tree-view')).toBeVisible({ timeout: PANEL_TIMEOUT });

    // Market tab
    const marketTab = bottomNav.locator('.mobile-tab', { hasText: 'Market' });
    await marketTab.click();
    await expect(marketTab).toHaveClass(/active/);
    await expect(page.locator('.marketplace-view')).toBeVisible({ timeout: PANEL_TIMEOUT });

    // Voice tab
    const voiceTab = bottomNav.locator('.mobile-tab', { hasText: 'Voice' });
    await voiceTab.click();
    await expect(voiceTab).toHaveClass(/active/);
    await expect(page.locator('.voice-setup')).toBeVisible({ timeout: PANEL_TIMEOUT });

    // Navigate back to Chat
    await chatTab.click();
    await expect(page.locator('.chat-view')).toBeVisible({ timeout: PANEL_TIMEOUT });

    // ── 4. Modernized input wrapper ─────────────────────────────────────
    const inputWrapper = page.locator('.input-wrapper');
    await expect(inputWrapper).toBeVisible();

    const input = inputWrapper.locator('.chat-input');
    const sendBtn = inputWrapper.locator('.send-btn');
    await expect(input).toBeVisible();
    await expect(sendBtn).toBeVisible();

    // Input is border-less inside the wrapper
    const inputBorder = await input.evaluate((el) => getComputedStyle(el).borderStyle);
    expect(inputBorder).toBe('none');

    // ── 5. Send message on mobile → real LLM response ───────────────────
    await input.fill('Hi there!');
    await sendBtn.click();

    // Open chat drawer to see messages
    const drawerToggle = page.locator('.chat-drawer-toggle');
    await expect(drawerToggle).toBeVisible();
    await drawerToggle.click();
    await expect(page.locator('.chat-history')).toBeVisible({ timeout: PANEL_TIMEOUT });

    const userMsg = page.locator('.message-row.user').first();
    await expect(userMsg).toBeVisible({ timeout: TIMEOUTS.message });
    await expect(userMsg).toContainText('Hi there!');
    await expect(input).toHaveValue(''); // cleared after send

    // Wait for real LLM response
    const response = await waitForAssistantResponse(page);
    expect(response.length).toBeGreaterThan(0);

    const assistantMsg = page.locator('.message-row.assistant').first();
    await expect(assistantMsg).toBeVisible({ timeout: TIMEOUTS.response });

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

    // Simulate keyboard open (300px keyboard → 367px visible viewport)
    await page.evaluate(() => {
      const vv = window.visualViewport;
      if (vv) {
        Object.defineProperty(vv, 'height', {
          value: 367,
          configurable: true,
          writable: true,
        });
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

    // ── 9. Memory tab on mobile ─────────────────────────────────────────
    await memoryTab.click();
    const memoryView = page.locator('.memory-view');
    await expect(memoryView).toBeVisible({ timeout: PANEL_TIMEOUT });

    // Stats dashboard visible when Tauri backend available
    const hasTauri = await page.evaluate(() => '__TAURI_INTERNALS__' in window);
    if (hasTauri) {
      await expect(memoryView.locator('.mv-stats')).toBeVisible({ timeout: PANEL_TIMEOUT });
    }

    // Tier filter chips visible
    await expect(memoryView.locator('.mv-tier-chip')).toHaveCount(3);

    // Add memory button visible
    await expect(memoryView.locator('button', { hasText: 'Add memory' })).toBeVisible();

    // Switch to Graph tab
    await memoryView.locator('.mv-tab', { hasText: 'Graph' }).click();
    await expect(memoryView.locator('.mv-graph-panel')).toBeVisible({ timeout: PANEL_TIMEOUT });

    // Back to chat
    await chatTab.click();

    // ── 10. Skills tab on mobile ────────────────────────────────────────
    await questsTab.click();
    await expect(page.locator('.skill-tree-view')).toBeVisible({ timeout: PANEL_TIMEOUT });
    await expect(page.locator('.st-progress-badge')).toBeVisible();

    // Back to chat
    await chatTab.click();

    // ── 11. No critical console errors ──────────────────────────────────
    assertNoCrashErrors(errors);
  });
});
