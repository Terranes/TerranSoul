/**
 * TerranSoul E2E tests.
 *
 * These run against the Vite dev server (no Tauri backend).
 * The @tauri-apps/api invoke() calls fail with a "window.__TAURI_INTERNALS__"
 * error, which the stores handle gracefully (error messages in chat).
 */
import { test, expect } from '@playwright/test';

test.describe('TerranSoul App', () => {
  test('app loads and shows main layout', async ({ page }) => {
    await page.goto('/');

    // The chat view should be visible
    const chatView = page.locator('.chat-view');
    await expect(chatView).toBeVisible();

    // Viewport section and chat section should both exist
    await expect(page.locator('.viewport-section')).toBeVisible();
    await expect(page.locator('.chat-section')).toBeVisible();
  });

  test('chat input is visible and interactive', async ({ page }) => {
    await page.goto('/');

    // Chat input should be visible
    const input = page.locator('.chat-input');
    await expect(input).toBeVisible();
    await expect(input).toBeEnabled();

    // Placeholder text should be present
    await expect(input).toHaveAttribute('placeholder', 'Type a message…');

    // Send button should be visible
    const sendBtn = page.locator('.send-btn');
    await expect(sendBtn).toBeVisible();

    // Send button should be disabled when input is empty
    await expect(sendBtn).toBeDisabled();

    // Type text — send button should become enabled
    await input.fill('hello');
    await expect(sendBtn).toBeEnabled();
  });

  test('sending a message shows user message and response in chat', async ({ page }) => {
    await page.goto('/');

    const input = page.locator('.chat-input');
    const sendBtn = page.locator('.send-btn');

    // Type and send a message
    await input.fill('hello world');
    await sendBtn.click();

    // User message should appear in the message list
    const userMsg = page.locator('.message-row.user').first();
    await expect(userMsg).toBeVisible({ timeout: 5000 });
    await expect(userMsg).toContainText('hello world');

    // An assistant response should eventually appear (either real or error)
    const assistantMsg = page.locator('.message-row.assistant').first();
    await expect(assistantMsg).toBeVisible({ timeout: 10000 });

    // Input should be cleared after sending
    await expect(input).toHaveValue('');
  });

  test('3D viewport canvas renders', async ({ page }) => {
    await page.goto('/');

    // Canvas element should be present in the viewport
    const canvas = page.locator('.viewport-canvas');
    await expect(canvas).toBeVisible();

    // Canvas should have actual dimensions
    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThan(100);
    expect(box!.height).toBeGreaterThan(100);
  });

  test('character state badge is visible', async ({ page }) => {
    await page.goto('/');

    // The state badge should show "idle" initially
    const badge = page.locator('.state-badge');
    await expect(badge).toBeVisible();
    await expect(badge).toContainText('idle');
  });

  test('model panel toggle button works', async ({ page }) => {
    await page.goto('/');

    // Model panel toggle button should be visible
    const toggleBtn = page.locator('.model-panel-toggle');
    await expect(toggleBtn).toBeVisible();

    // Model panel should be hidden initially
    await expect(page.locator('.model-panel')).not.toBeVisible();

    // Click toggle — panel should appear
    await toggleBtn.click();
    await expect(page.locator('.model-panel')).toBeVisible({ timeout: 2000 });

    // Panel should have "3D Models" header
    await expect(page.locator('.panel-header h3')).toContainText('3D Models');
  });
});
