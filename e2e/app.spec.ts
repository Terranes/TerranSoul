/**
 * TerranSoul E2E tests.
 *
 * These run against the Vite dev server (no Tauri backend).
 * The @tauri-apps/api invoke() calls fail with a "window.__TAURI_INTERNALS__"
 * error, which the stores handle gracefully (error messages in chat).
 */
import { test, expect } from '@playwright/test';

/** Timeout for messages to appear in the chat (accounts for invoke error path). */
const MESSAGE_TIMEOUT = 5_000;
/** Timeout for assistant response after sending (longer for error fallback). */
const RESPONSE_TIMEOUT = 10_000;
/** Timeout for UI panels to animate into view. */
const PANEL_TIMEOUT = 2_000;

test.describe('TerranSoul App', () => {
  test('app loads and shows main layout', async ({ page }) => {
    await page.goto('/');

    // The chat view should be visible
    const chatView = page.locator('.chat-view');
    await expect(chatView).toBeVisible();

    // Viewport section and chat section should both exist
    await expect(page.locator('.viewport-layer')).toBeVisible();
    await expect(page.locator('.input-footer')).toBeVisible();
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

    // Open the chat drawer to see messages
    await page.locator('.chat-drawer-toggle').click();

    // User message should appear in the message list
    const userMsg = page.locator('.message-row.user').first();
    await expect(userMsg).toBeVisible({ timeout: MESSAGE_TIMEOUT });
    await expect(userMsg).toContainText('hello world');

    // An assistant response should eventually appear (either real or error)
    const assistantMsg = page.locator('.message-row.assistant').first();
    await expect(assistantMsg).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Input should be cleared after sending
    await expect(input).toHaveValue('');
  });

});

test.describe('3D Character Loading & Animation', () => {
  /** Wait long enough for VRM model to download and parse. */
  const VRM_LOAD_TIMEOUT = 30_000;

  /** Helper: enable debug overlay and wait for triangle count > 0 */
  async function waitForModelLoaded(page: import('@playwright/test').Page) {
    // Wait for splash screen to disappear before interacting
    await expect(page.locator('.splash')).toBeHidden({ timeout: 10_000 });
    const debugOverlay = page.locator('.debug-overlay');
    if (!(await debugOverlay.isVisible())) {
      await page.keyboard.press('Control+d');
      await page.waitForTimeout(300);
    }
    if (!(await debugOverlay.isVisible())) {
      await page.keyboard.press('Control+d');
    }
    await expect(debugOverlay).toBeVisible({ timeout: 5_000 });
    await expect(async () => {
      const text = await debugOverlay.locator('span').nth(1).textContent();
      expect(parseInt(text?.replace(/[^\d]/g, '') ?? '0', 10)).toBeGreaterThan(0);
    }).toPass({ timeout: VRM_LOAD_TIMEOUT });
    return debugOverlay;
  }

  /** Helper: wait for loading overlay to disappear */
  async function waitForLoadingDone(page: import('@playwright/test').Page) {
    await expect(page.locator('.loading-overlay')).toBeHidden({ timeout: VRM_LOAD_TIMEOUT });
  }

  test('Annabelle (default, cool persona) loads correctly', async ({ page }) => {
    await page.goto('/');

    const debugOverlay = await waitForModelLoaded(page);

    // Verify triangle count is > 0 (geometry rendered)
    const triangleSpan = debugOverlay.locator('span').nth(1);
    const text = await triangleSpan.textContent();
    const triCount = parseInt(text?.replace(/[^\d]/g, '') ?? '0', 10);
    expect(triCount).toBeGreaterThan(0);

    // Loading overlay should be gone
    await waitForLoadingDone(page);

    // VRM instance must be exposed on window for integration testing
    const hasVrm = await page.evaluate(() => !!(window as any).__terransoul_vrm__);
    expect(hasVrm).toBe(true);

    // Model selector should show annabelle (open settings first)
    await page.locator('.settings-toggle').click();
    await expect(page.locator('.model-selector')).toHaveValue('annabelle');
  });

  test('3D canvas is visible with non-zero dimensions', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.splash')).toBeHidden({ timeout: 10_000 });

    const canvas = page.locator('.viewport-canvas');
    await expect(canvas).toBeVisible();
    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThan(100);
    expect(box!.height).toBeGreaterThan(100);
  });

  test('switching models shows loading overlay', async ({ page }) => {
    test.setTimeout(90_000);
    await page.goto('/');

    // Wait for initial model to fully load
    await waitForModelLoaded(page);
    await waitForLoadingDone(page);

    // Switch to M58 — open settings first, then loading overlay should appear
    await page.locator('.settings-toggle').click();
    const selector = page.locator('.model-selector');
    await selector.selectOption('m58');

    // Loading overlay should eventually disappear when M58 is ready
    await waitForLoadingDone(page);
  });

});

test.describe('Animation & AI Emotion', () => {
  test('AI responds with persona (not an error) in browser mode', async ({ page }) => {
    await page.goto('/');

    const input = page.locator('.chat-input');
    const sendBtn = page.locator('.send-btn');

    await input.fill('Hello there!');
    await sendBtn.click();

    // Open the chat drawer to see messages
    await page.locator('.chat-drawer-toggle').click();

    const assistantMsg = page.locator('.message-row.assistant').first();
    await expect(assistantMsg).toBeVisible({ timeout: 5_000 });

    const text = await assistantMsg.textContent();
    expect(text).not.toContain('Error:');
    expect(text).toContain('TerranSoul');
  });

  test('all 8 emotion states cycle correctly across messages', async ({ page }) => {
    test.setTimeout(90_000);
    await page.goto('/');

    const badge = page.locator('.ai-state-pill');
    const input = page.locator('.chat-input');
    const sendBtn = page.locator('.send-btn');

    // happy → idle
    await input.fill('Hey there!');
    await sendBtn.click();
    await expect(badge).toContainText('Happy', { timeout: 5_000 });
    await expect(badge).toContainText('Idle', { timeout: 10_000 });

    // sad → idle
    await input.fill('That makes me sad');
    await sendBtn.click();
    await expect(badge).toContainText('Sad', { timeout: 5_000 });
    await expect(badge).toContainText('Idle', { timeout: 10_000 });

    // angry → idle
    await input.fill('I am so frustrated and angry');
    await sendBtn.click();
    await expect(badge).toContainText('Angry', { timeout: 5_000 });
    await expect(badge).toContainText('Idle', { timeout: 10_000 });

    // relaxed → idle
    await input.fill('Let me relax a bit');
    await sendBtn.click();
    await expect(badge).toContainText('Relaxed', { timeout: 5_000 });
    await expect(badge).toContainText('Idle', { timeout: 10_000 });

    // surprised → idle
    await input.fill('Wow that is amazing!');
    await sendBtn.click();
    await expect(badge).toContainText('Surprised', { timeout: 5_000 });
    await expect(badge).toContainText('Idle', { timeout: 10_000 });

    // happy again
    await input.fill('Actually, I feel awesome!');
    await sendBtn.click();
    await expect(badge).toContainText('Happy', { timeout: 5_000 });
  });
});

// ── Free LLM Brain Auto-Configuration ──────────────────────────────────
//
// NOTE: We intentionally keep only ONE test here to avoid spamming free LLM
// providers during CI/CD runs. All assertions that verify free-API
// auto-configuration are combined into this single test case.

test.describe('Free LLM Brain', () => {
  test('auto-configures free API, skips Ollama setup, and enables chat', async ({ page }) => {
    await page.goto('/');

    // 1. App should skip the brain setup wizard and show chat view directly
    const chatView = page.locator('.chat-view');
    await expect(chatView).toBeVisible({ timeout: 5_000 });
    await expect(page.locator('.brain-setup')).not.toBeVisible();

    // 2. No "Ollama not running" error or brain setup card should appear
    await expect(page.locator('text=Ollama not running')).not.toBeVisible();
    await expect(page.locator('.brain-inline')).not.toBeVisible();

    // 3. Brain store should report free_api mode via Pinia state
    const brainState = await page.evaluate(() => {
      const app = (document.querySelector('#app') as any)?.__vue_app__;
      if (!app) return null;
      const pinia = app.config.globalProperties.$pinia;
      if (!pinia) return null;
      const s = pinia.state.value.brain;
      if (!s) return null;
      return {
        hasBrain: s.activeBrain !== null || s.brainMode !== null,
        brainMode: s.brainMode,
        freeProviders: s.freeProviders?.length ?? 0,
      };
    });
    expect(brainState).not.toBeNull();
    expect(brainState!.hasBrain).toBe(true);
    expect(brainState!.brainMode).not.toBeNull();
    expect(brainState!.brainMode.mode).toBe('free_api');
    expect(brainState!.freeProviders).toBeGreaterThan(0);

    // 4. Chat input should be usable — send a message and get a response
    const input = page.locator('.chat-input');
    await expect(input).toBeVisible();
    await expect(input).toBeEnabled();

    await input.fill('hello');
    const sendBtn = page.locator('.send-btn');
    await expect(sendBtn).toBeEnabled();
    await sendBtn.click();

    // Open the chat drawer to see messages
    await page.locator('.chat-drawer-toggle').click();

    const userMsg = page.locator('.message-row.user').first();
    await expect(userMsg).toBeVisible({ timeout: MESSAGE_TIMEOUT });
    await expect(userMsg).toContainText('hello');

    const assistantMsg = page.locator('.message-row.assistant').first();
    await expect(assistantMsg).toBeVisible({ timeout: RESPONSE_TIMEOUT });
    // Response should NOT mention Ollama — free API path is active
    await expect(assistantMsg).not.toContainText('Ollama');
  });
});

// ── Voice Auto-Configuration ──────────────────────────────────────────────
test.describe('Voice Auto-Configuration', () => {
  test('voice is auto-configured with Web Speech API and Edge TTS by default', async ({ page }) => {
    await page.goto('/');

    // Wait for app to finish initialising
    await expect(page.locator('.chat-view')).toBeVisible({ timeout: 5_000 });

    // Voice store should have auto-configured providers
    const voiceState = await page.evaluate(() => {
      const app = (document.querySelector('#app') as any)?.__vue_app__;
      if (!app) return null;
      const pinia = app.config.globalProperties.$pinia;
      if (!pinia) return null;
      const s = pinia.state.value.voice;
      if (!s) return null;
      return {
        asr_provider: s.config?.asr_provider,
        tts_provider: s.config?.tts_provider,
      };
    });
    expect(voiceState).not.toBeNull();
    expect(voiceState!.asr_provider).toBe('web-speech');
    expect(voiceState!.tts_provider).toBe('edge-tts');
  });

});

// ── Marketplace LLM Configuration ────────────────────────────────────────
test.describe('Marketplace LLM Configuration', () => {
  test('marketplace shows LLM configuration section', async ({ page }) => {
    await page.goto('/');

    // Navigate to marketplace
    // In browser mode, click the marketplace tab
    const mpTab = page.locator('button:has-text("🏪")').first();
    await mpTab.click();

    // Marketplace view should be visible
    await expect(page.locator('.marketplace-view')).toBeVisible({ timeout: 3_000 });

    // LLM configuration section should be visible (the collapsible header)
    const llmConfigHeader = page.locator('.llm-config-header');
    await expect(llmConfigHeader).toBeVisible({ timeout: 3_000 });
    await expect(llmConfigHeader).toContainText('Configure LLM');
  });
});

// ── Chat-based LLM Switching ─────────────────────────────────────────────
test.describe('Chat-based LLM Switching', () => {
  test('sending "switch to pollinations" triggers LLM switch response', async ({ page }) => {
    await page.goto('/');

    // Wait for chat to be ready
    await expect(page.locator('.chat-view')).toBeVisible({ timeout: 5_000 });

    const input = page.locator('.chat-input');
    const sendBtn = page.locator('.send-btn');

    // Send LLM switch command
    await input.fill('switch to pollinations');
    await sendBtn.click();

    // Open chat drawer
    await page.locator('.chat-drawer-toggle').click();

    // User message should appear
    const userMsg = page.locator('.message-row.user').first();
    await expect(userMsg).toBeVisible({ timeout: MESSAGE_TIMEOUT });
    await expect(userMsg).toContainText('switch to pollinations');

    // Assistant should confirm the switch
    const assistantMsg = page.locator('.message-row.assistant').first();
    await expect(assistantMsg).toBeVisible({ timeout: RESPONSE_TIMEOUT });
    await expect(assistantMsg).toContainText('Pollinations');
  });
});
