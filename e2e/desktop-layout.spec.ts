/**
 * Desktop layout regression checks for narrow Tauri windows.
 *
 * A desktop window can be narrow enough to use the responsive bottom tab bar.
 * In that mode the 3D chat input sits just above the tab bar, without the
 * shell's reserved mobile padding becoming a visible blank band.
 */
import { test, expect } from '@playwright/test';
import { connectToDesktopApp } from './helpers';

test('desktop narrow 3D chat input docks to bottom navigation', async () => {
  const { browser, page } = await connectToDesktopApp();
  try {
    await page.setViewportSize({ width: 588, height: 1152 });
    await page.waitForSelector('.chat-view', { timeout: 30_000 });
    await page.waitForSelector('.input-footer', { timeout: 30_000 });

    const runtime = await page.evaluate(() => ({
      tauriAvailable: typeof (window as any).__TAURI_INTERNALS__?.invoke === 'function',
      width: window.innerWidth,
    }));
    expect(runtime.tauriAvailable).toBe(true);
    expect(runtime.width).toBeLessThanOrEqual(640);

    const bottomNav = page.locator('.mobile-bottom-nav');
    const inputBar = page.locator('.chat-input-bar');

    await expect(page.locator('.chat-view')).toBeVisible();
    await expect(bottomNav).toBeVisible();
    await expect(inputBar).toBeVisible();

    await expect(async () => {
      const navBox = await bottomNav.boundingBox();
      const inputBox = await inputBar.boundingBox();

      expect(navBox).not.toBeNull();
      expect(inputBox).not.toBeNull();
      const gap = navBox!.y - (inputBox!.y + inputBox!.height);
      expect(gap).toBeGreaterThanOrEqual(2);
      expect(gap).toBeLessThanOrEqual(5);
    }).toPass({ timeout: 5_000 });
  } finally {
    await browser.close();
  }
});