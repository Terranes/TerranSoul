/**
 * Self-Validation E2E Test — TerranSoul Agent
 *
 * This test validates that TerranSoul is working as expected by exercising
 * every major user-facing feature. Playwright records a video of the entire
 * session; the CI pipeline then saves the video to `recording/validation.webm`
 * so stakeholders can visually confirm the app's health.
 *
 * Run locally:  npm run test:validate
 * Run in CI:    triggered automatically via the validation-recording.yml workflow
 */
import { test, expect } from '@playwright/test';

const MSG_TIMEOUT = 5_000;
const RESPONSE_TIMEOUT = 10_000;
const PANEL_TIMEOUT = 2_000;
const ANIMATION_PAUSE = 1_500; // Let the user see the state in the video

test.describe('TerranSoul Self-Validation', () => {
  test('validates full app functionality and records video', async ({ page }) => {
    // ──────────────────────────────────────────────────────────
    // 1. App loads — main layout is visible
    // ──────────────────────────────────────────────────────────
    await page.goto('/');
    await expect(page.locator('.chat-view')).toBeVisible();
    await expect(page.locator('.viewport-section')).toBeVisible();
    await expect(page.locator('.chat-section')).toBeVisible();

    // Pause so the video shows the initial state
    await page.waitForTimeout(ANIMATION_PAUSE);

    // ──────────────────────────────────────────────────────────
    // 2. 3D viewport renders a canvas with real dimensions
    // ──────────────────────────────────────────────────────────
    const canvas = page.locator('.viewport-canvas');
    await expect(canvas).toBeVisible();
    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThan(100);
    expect(box!.height).toBeGreaterThan(100);

    // ──────────────────────────────────────────────────────────
    // 3. Character state badge shows "idle" initially
    // ──────────────────────────────────────────────────────────
    const badge = page.locator('.state-badge');
    await expect(badge).toBeVisible();
    await expect(badge).toContainText('idle');

    // ──────────────────────────────────────────────────────────
    // 4. Chat input is visible, enabled, and interactive
    // ──────────────────────────────────────────────────────────
    const input = page.locator('.chat-input');
    const sendBtn = page.locator('.send-btn');

    await expect(input).toBeVisible();
    await expect(input).toBeEnabled();
    await expect(input).toHaveAttribute('placeholder', 'Type a message…');
    await expect(sendBtn).toBeDisabled(); // disabled when empty

    // Type and verify the button enables
    await input.fill('Hello from TerranSoul self-validation!');
    await expect(sendBtn).toBeEnabled();
    await page.waitForTimeout(ANIMATION_PAUSE);

    // ──────────────────────────────────────────────────────────
    // 5. Send a message — user bubble appears, assistant responds
    // ──────────────────────────────────────────────────────────
    await sendBtn.click();

    const userMsg = page.locator('.message-row.user').first();
    await expect(userMsg).toBeVisible({ timeout: MSG_TIMEOUT });
    await expect(userMsg).toContainText('Hello from TerranSoul self-validation!');

    const assistantMsg = page.locator('.message-row.assistant').first();
    await expect(assistantMsg).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Input should clear after send
    await expect(input).toHaveValue('');

    await page.waitForTimeout(ANIMATION_PAUSE);

    // ──────────────────────────────────────────────────────────
    // 6. Model panel toggle works
    // ──────────────────────────────────────────────────────────
    const toggleBtn = page.locator('.model-panel-toggle');
    await expect(toggleBtn).toBeVisible();

    // Panel hidden initially
    await expect(page.locator('.model-panel')).not.toBeVisible();

    // Open the panel
    await toggleBtn.click();
    await expect(page.locator('.model-panel')).toBeVisible({ timeout: PANEL_TIMEOUT });
    await expect(page.locator('.panel-header h3')).toContainText('3D Models');

    await page.waitForTimeout(ANIMATION_PAUSE);

    // Close the panel
    await page.locator('.close-btn').click();
    await expect(page.locator('.model-panel')).not.toBeVisible({ timeout: PANEL_TIMEOUT });

    // ──────────────────────────────────────────────────────────
    // 7. Send another message to show ongoing chat capability
    // ──────────────────────────────────────────────────────────
    await input.fill('Second validation message');
    await sendBtn.click();

    const userMsg2 = page.locator('.message-row.user').nth(1);
    await expect(userMsg2).toBeVisible({ timeout: MSG_TIMEOUT });
    await expect(userMsg2).toContainText('Second validation message');

    // Wait for the second assistant response
    const assistantMsg2 = page.locator('.message-row.assistant').nth(1);
    await expect(assistantMsg2).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Final pause to let the video capture the completed state
    await page.waitForTimeout(ANIMATION_PAUSE * 2);
  });
});
