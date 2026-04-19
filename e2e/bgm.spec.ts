/**
 * Background Music (BGM) E2E tests.
 *
 * Tests the full BGM flow: floating music bar, play/pause/next,
 * volume, settings dropdown, and the "add music" → model conversation flow.
 *
 * Runs against the Vite dev server (no Tauri backend).
 * AudioContext is auto-mocked by Chromium in headless mode.
 */
import { test, expect } from '@playwright/test';

const PANEL_TIMEOUT = 2_000;
const RESPONSE_TIMEOUT = 10_000;

test.describe('Background Music UI', () => {
  test('clicking music bar toggle expands the panel', async ({ page }) => {
    await page.goto('/');

    const musicToggle = page.locator('.music-bar-toggle');
    await musicToggle.click();

    const panel = page.locator('.music-bar-panel');
    await expect(panel).toBeVisible({ timeout: PANEL_TIMEOUT });

    // Panel should contain play/pause, track name, next, volume, and add buttons
    await expect(panel.locator('.play-btn')).toBeVisible();
    await expect(panel.locator('.music-track-name')).toBeVisible();
    await expect(panel.locator('.music-vol-slider')).toBeVisible();
    await expect(panel.locator('.add-btn')).toBeVisible();
  });

  test('play button toggles BGM and updates icon', async ({ page }) => {
    await page.goto('/');

    // Expand the music bar
    await page.locator('.music-bar-toggle').click();
    await expect(page.locator('.music-bar-panel')).toBeVisible({ timeout: PANEL_TIMEOUT });

    const playBtn = page.locator('.music-btn.play-btn');

    // Initially off — should show play icon
    await expect(playBtn).toContainText('▶️');

    // Click to play
    await playBtn.click();

    // Should now show pause icon
    await expect(playBtn).toContainText('⏸');

    // Music bar should get the 'playing' class
    await expect(page.locator('.music-bar')).toHaveClass(/playing/);

    // Click again to pause
    await playBtn.click();
    await expect(playBtn).toContainText('▶️');
    await expect(page.locator('.music-bar')).not.toHaveClass(/playing/);
  });

  test('next track button cycles through tracks', async ({ page }) => {
    await page.goto('/');

    await page.locator('.music-bar-toggle').click();
    await expect(page.locator('.music-bar-panel')).toBeVisible({ timeout: PANEL_TIMEOUT });

    const trackName = page.locator('.music-track-name');
    const initialTrack = await trackName.textContent();

    // Click next track
    const nextBtn = page.locator('.music-bar-panel .music-btn').nth(1);
    await nextBtn.click();

    // Track name should change
    await expect(async () => {
      const newTrack = await trackName.textContent();
      expect(newTrack).not.toBe(initialTrack);
    }).toPass({ timeout: 1000 });
  });

  test('add music button opens chat and sends message to model', async ({ page }) => {
    await page.goto('/');

    // Expand music bar and click add
    await page.locator('.music-bar-toggle').click();
    await expect(page.locator('.music-bar-panel')).toBeVisible({ timeout: PANEL_TIMEOUT });

    await page.locator('.add-btn').click();

    // Chat drawer should expand
    await expect(page.locator('.chat-history')).toBeVisible({ timeout: PANEL_TIMEOUT });

    // A user message about adding music should appear
    const userMsg = page.locator('.message-row.user').first();
    await expect(userMsg).toBeVisible({ timeout: RESPONSE_TIMEOUT });
    await expect(userMsg).toContainText(/background music|music/i);

    // An assistant response should appear
    const assistantMsg = page.locator('.message-row.assistant').first();
    await expect(assistantMsg).toBeVisible({ timeout: RESPONSE_TIMEOUT });
  });
});
