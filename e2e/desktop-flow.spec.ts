/**
 * Desktop E2E — single comprehensive test covering the full app flow.
 *
 * Consolidates all desktop assertions (app layout, chat, brain/voice auto-config,
 * 3D model, BGM, marketplace, LLM switching, quest system, subtitles, error
 * resilience) into one sequential test to minimize page navigations and CI time.
 *
 * Runs against the Vite dev server (no Tauri backend).
 */
import { test, expect, type Page } from '@playwright/test';

// ─── Timeouts ────────────────────────────────────────────────────────────────
const MESSAGE_TIMEOUT = 5_000;
const RESPONSE_TIMEOUT = 15_000;
const PANEL_TIMEOUT = 3_000;
const VRM_LOAD_TIMEOUT = 30_000;

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

async function openDrawer(page: Page) {
  const drawer = page.locator('.chat-history');
  if (!(await drawer.isVisible().catch(() => false))) {
    await page.locator('.chat-drawer-toggle').click({ force: true });
    await expect(drawer).toBeVisible({ timeout: PANEL_TIMEOUT });
  }
}

async function closeDrawer(page: Page) {
  const drawer = page.locator('.chat-history');
  if (await drawer.isVisible().catch(() => false)) {
    await page.locator('.chat-drawer-toggle').click({ force: true });
    await expect(drawer).not.toBeVisible({ timeout: PANEL_TIMEOUT });
  }
}

async function sendMessage(page: Page, text: string) {
  const input = page.locator('.chat-input');
  const sendBtn = page.locator('.send-btn');
  await input.fill(text);
  await sendBtn.click();
}

async function waitForModelLoaded(page: Page) {
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

// ─── Test ────────────────────────────────────────────────────────────────────

test('desktop: full end-to-end flow', { timeout: 120_000 }, async ({ page }) => {
  const errors = collectConsoleErrors(page);
  await page.goto('/');

  // ── 1. App loads and shows main layout ──────────────────────────────────
  const chatView = page.locator('.chat-view');
  await expect(chatView).toBeVisible({ timeout: 5_000 });
  await expect(page.locator('.viewport-layer')).toBeVisible();
  await expect(page.locator('.input-footer')).toBeVisible();

  // AI state pill starts at Idle
  const badge = page.locator('.ai-state-pill');
  await expect(badge).toBeVisible();
  await expect(badge).toContainText('Idle');

  // ── 2. Chat input is interactive ────────────────────────────────────────
  const input = page.locator('.chat-input');
  const sendBtn = page.locator('.send-btn');
  await expect(input).toBeVisible();
  await expect(input).toBeEnabled();
  await expect(input).toHaveAttribute('placeholder', 'Type a message…');
  await expect(sendBtn).toBeVisible();
  await expect(sendBtn).toBeDisabled(); // disabled when empty
  await input.fill('hello');
  await expect(sendBtn).toBeEnabled();
  await input.fill(''); // reset

  // ── 3. Free LLM Brain auto-configured ───────────────────────────────────
  await expect(page.locator('.brain-setup')).not.toBeVisible();
  await expect(page.locator('text=Ollama not running')).not.toBeVisible();

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
  expect(brainState!.brainMode?.mode).toBe('free_api');
  expect(brainState!.freeProviders).toBeGreaterThan(0);

  // ── 4. Voice auto-configured ────────────────────────────────────────────
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

  // ── 5. Send message → get response ──────────────────────────────────────
  await sendMessage(page, 'Hello there!');
  await openDrawer(page);

  const userMsg = page.locator('.message-row.user').first();
  await expect(userMsg).toBeVisible({ timeout: MESSAGE_TIMEOUT });
  await expect(userMsg).toContainText('Hello there!');
  await expect(input).toHaveValue(''); // cleared after send

  const assistantMsg = page.locator('.message-row.assistant').first();
  await expect(assistantMsg).toBeVisible({ timeout: RESPONSE_TIMEOUT });
  const responseText = await assistantMsg.textContent();
  expect(responseText).not.toContain('Error:');
  expect(responseText).toContain('TerranSoul');

  // ── 6. Subtitle appears and has no raw markdown ─────────────────────────
  const subtitle = page.locator('.subtitle-overlay');
  await expect(subtitle).toBeVisible({ timeout: RESPONSE_TIMEOUT });
  const subtitleText = page.locator('.subtitle-text');
  const subtitleContent = await subtitleText.textContent();
  expect(subtitleContent).toBeTruthy();
  expect(subtitleContent!.length).toBeGreaterThan(0);
  const subtitleHtml = await subtitleText.innerHTML();
  expect(subtitleHtml).not.toMatch(/\*\*[^*]+\*\*/);

  // ── 7. 3D canvas visible ────────────────────────────────────────────────
  await closeDrawer(page);
  // Wait for subtitle overlay to auto-dismiss before checking canvas/music bar
  await expect(page.locator('.subtitle-overlay')).toBeHidden({ timeout: 15_000 });
  const canvas = page.locator('.viewport-canvas');
  await expect(canvas).toBeVisible();
  const box = await canvas.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeGreaterThan(100);
  expect(box!.height).toBeGreaterThan(100);

  // ── 8. VRM model loaded (Annabelle) ─────────────────────────────────────
  const debugOverlay = await waitForModelLoaded(page);
  const triangleText = await debugOverlay.locator('span').nth(1).textContent();
  expect(parseInt(triangleText?.replace(/[^\d]/g, '') ?? '0', 10)).toBeGreaterThan(0);
  await expect(page.locator('.loading-overlay')).toBeHidden({ timeout: VRM_LOAD_TIMEOUT });

  const hasVrm = await page.evaluate(() => !!(window as any).__terransoul_vrm__);
  expect(hasVrm).toBe(true);

  // Close debug overlay
  await page.keyboard.press('Control+d');

  // ── 9. BGM music bar ────────────────────────────────────────────────────
  const musicToggle = page.locator('.music-bar-toggle');
  await musicToggle.click({ force: true });
  const musicPanel = page.locator('.music-bar-panel');
  await expect(musicPanel).toBeVisible({ timeout: PANEL_TIMEOUT });
  await expect(musicPanel.locator('.play-btn')).toBeVisible();
  await expect(musicPanel.locator('.music-track-name')).toBeVisible();
  await expect(musicPanel.locator('.music-vol-slider')).toBeVisible();

  // Play/pause toggle
  const playBtn = page.locator('.music-btn.play-btn');
  await expect(playBtn).toContainText('▶️');
  await playBtn.click();
  await expect(playBtn).toContainText('⏸');
  await expect(page.locator('.music-bar')).toHaveClass(/playing/);
  await playBtn.click();
  await expect(playBtn).toContainText('▶️');

  // Next track cycles
  const trackName = page.locator('.music-track-name');
  const initialTrack = await trackName.textContent();
  const nextBtn = page.locator('.music-bar-panel .music-btn').nth(1);
  await nextBtn.click();
  await expect(async () => {
    const newTrack = await trackName.textContent();
    expect(newTrack).not.toBe(initialTrack);
  }).toPass({ timeout: 1_000 });

  // Close music bar by clicking toggle again
  await musicToggle.click();

  // ── 10. Navigate to Marketplace → LLM config visible ───────────────────
  const mpTab = page.locator('.nav-btn', { hasText: 'Market' }).first();
  await mpTab.click();
  await expect(page.locator('.marketplace-view')).toBeVisible({ timeout: PANEL_TIMEOUT });
  const llmConfigHeader = page.locator('.llm-config-header');
  await expect(llmConfigHeader).toBeVisible({ timeout: PANEL_TIMEOUT });
  await expect(llmConfigHeader).toContainText('Configure LLM');

  // Navigate back to Chat
  const chatTab = page.locator('.nav-btn', { hasText: 'Chat' }).first();
  await chatTab.click();
  await expect(page.locator('.chat-view')).toBeVisible({ timeout: PANEL_TIMEOUT });

  // ── 11. Chat-based LLM switching ────────────────────────────────────────
  await sendMessage(page, 'switch to pollinations');
  await openDrawer(page);
  const switchUserMsg = page.locator('.message-row.user').last();
  await expect(switchUserMsg).toContainText('switch to pollinations');
  const switchAssistantMsg = page.locator('.message-row.assistant').last();
  await expect(switchAssistantMsg).toBeVisible({ timeout: RESPONSE_TIMEOUT });
  await expect(switchAssistantMsg).toContainText('Pollinations');

  // ── 12. Quest system — trigger phrase shows response ──────────────────
  await closeDrawer(page);
  await sendMessage(page, 'Where can I start?');
  await openDrawer(page);

  // Wait for assistant response
  const questAssistant = page.locator('.message-row.assistant').last();
  await expect(questAssistant).toBeVisible({ timeout: RESPONSE_TIMEOUT });

  // Typing indicator must clear (app is not stuck)
  await expect(page.locator('.typing-indicator')).not.toBeVisible({ timeout: 3_000 });

  // If quest overlay appeared (requires live LLM), walk through the flow.
  // In environments without LLM access, a provider-warning hotseat may appear
  // instead — that's fine, we just verify no crash.
  const questOverlay = page.locator('.hotseat-strip');
  const hasQuest = await questOverlay.isVisible().catch(() => false);
  if (hasQuest) {
    const tiles = questOverlay.locator('.hotseat-tile');
    const labels = (await tiles.allTextContents()).join(' ');

    // Quest tiles have Accept/Tell me more/Maybe later.
    // Provider-warning tiles have Install/Upgrade/Retry — skip those.
    if (labels.includes('Accept') && labels.includes('Maybe later')) {
      // Accept the quest
      for (let i = 0; i < await tiles.count(); i++) {
        const txt = await tiles.nth(i).textContent();
        if (txt && txt.includes('Accept')) {
          await tiles.nth(i).click();
          break;
        }
      }
      await page.waitForTimeout(500);

      // Acceptance confirmation
      const acceptMsg = page.locator('.message-row.assistant').last();
      await expect(acceptMsg).toContainText('Quest Accepted', { timeout: RESPONSE_TIMEOUT });

      // BGM options may appear
      const bgmStrip = page.locator('.hotseat-strip');
      const hasBgm = await bgmStrip.isVisible().catch(() => false);
      if (hasBgm) {
        const bgmTiles = bgmStrip.locator('.hotseat-tile');
        const bgmLabels = (await bgmTiles.allTextContents()).join(' ');
        if (bgmLabels.includes('Autoplay BGM')) {
          for (let i = 0; i < await bgmTiles.count(); i++) {
            const txt = await bgmTiles.nth(i).textContent();
            if (txt && txt.includes('Autoplay BGM')) {
              await bgmTiles.nth(i).click();
              break;
            }
          }
          await page.waitForTimeout(1_000);
          const musicBarPlaying = await page.evaluate(() => {
            const musicBar = document.querySelector('.music-bar');
            return musicBar?.classList.contains('playing') ?? false;
          });
          expect(musicBarPlaying).toBe(true);
        }
      }
    }
  }

  // ── 13. No critical console errors ──────────────────────────────────────
  const crashErrors = errors.filter(e =>
    e.includes('Cannot read properties of undefined') ||
    e.includes('UNCAUGHT') ||
    e.includes('Unhandled error'),
  );
  expect(crashErrors).toHaveLength(0);
});
