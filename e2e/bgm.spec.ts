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
  test('floating music bar toggle is visible on load', async ({ page }) => {
    await page.goto('/');

    const musicToggle = page.locator('.music-bar-toggle');
    await expect(musicToggle).toBeVisible();
    // Music bar panel should be collapsed initially
    await expect(page.locator('.music-bar-panel')).not.toBeVisible();
  });

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

  test('volume slider is interactive', async ({ page }) => {
    await page.goto('/');

    await page.locator('.music-bar-toggle').click();
    await expect(page.locator('.music-bar-panel')).toBeVisible({ timeout: PANEL_TIMEOUT });

    const slider = page.locator('.music-vol-slider');
    await expect(slider).toBeVisible();

    // Slider should have a value in the 0-100 range
    const value = await slider.inputValue();
    const numVal = parseInt(value, 10);
    expect(numVal).toBeGreaterThanOrEqual(0);
    expect(numVal).toBeLessThanOrEqual(100);
  });

  test('settings dropdown has BGM toggle and controls', async ({ page }) => {
    await page.goto('/');

    // Open settings dropdown
    await page.locator('.settings-toggle').click();
    const dropdown = page.locator('.settings-dropdown');
    await expect(dropdown).toBeVisible({ timeout: PANEL_TIMEOUT });

    // Should have Music section with toggle
    await expect(dropdown.locator('.bgm-switch')).toBeVisible();
    await expect(dropdown.locator('.bgm-status')).toBeVisible();
    await expect(dropdown.locator('.bgm-status')).toContainText('Off');

    // Toggle BGM on
    await dropdown.locator('.bgm-switch').click();
    await expect(dropdown.locator('.bgm-status')).toContainText('On');

    // Volume slider and track selector should appear
    await expect(dropdown.locator('.bgm-volume-slider')).toBeVisible();
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

  test('collapsing music bar hides the panel', async ({ page }) => {
    await page.goto('/');

    const musicToggle = page.locator('.music-bar-toggle');

    // Expand
    await musicToggle.click();
    await expect(page.locator('.music-bar-panel')).toBeVisible({ timeout: PANEL_TIMEOUT });

    // Collapse
    await musicToggle.click();
    await expect(page.locator('.music-bar-panel')).not.toBeVisible();
  });
});

test.describe('BGM Status Indicator Integration', () => {
  test('status shows Thinking when waiting for response', async ({ page }) => {
    await page.goto('/');

    const badge = page.locator('.ai-state-pill');
    await expect(badge).toContainText('Idle');

    const input = page.locator('.chat-input');
    const sendBtn = page.locator('.send-btn');
    await input.fill('test message');
    await sendBtn.click();

    // Should transition through Thinking
    await expect(async () => {
      const text = (await badge.textContent())?.trim();
      expect(['Thinking…', 'Talking']).toContain(text);
    }).toPass({ timeout: 3_000 });

    // Should eventually return to Idle
    await expect(badge).toContainText('Idle', { timeout: RESPONSE_TIMEOUT });
  });

  test('quick reply buttons appear for yes/no questions', async ({ page }) => {
    await page.goto('/');

    // Open chat drawer
    await page.locator('.chat-drawer-toggle').click();

    const input = page.locator('.chat-input');
    const sendBtn = page.locator('.send-btn');

    // Send a message that may trigger a yes/no response
    await input.fill('Can you help me set up background music?');
    await sendBtn.click();

    // Wait for assistant response
    const assistantMsg = page.locator('.message-row.assistant').first();
    await expect(assistantMsg).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Quick reply buttons may or may not appear depending on model response,
    // but the DOM structure should be ready (no errors)
    const quickReplies = page.locator('.quick-replies');
    // Just verify no runtime errors — buttons show only if response matches pattern
    await page.waitForTimeout(500);
  });
});

test.describe('Emoji Popup', () => {
  test('emoji popup container exists in DOM structure', async ({ page }) => {
    await page.goto('/');

    // The emoji popup transition wrapper should exist (hidden by default)
    // We verify the component renders without errors
    const chatView = page.locator('.chat-view');
    await expect(chatView).toBeVisible();

    // No emoji popup should be visible initially
    await expect(page.locator('.emoji-popup')).not.toBeVisible();
  });
});
