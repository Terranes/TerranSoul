/**
 * Desktop E2E — comprehensive test covering the full desktop app flow.
 *
 * Runs against the Vite dev server with REAL free API connections (Pollinations).
 * No mocks, no fakes — every assertion hits real services.
 *
 * Sections:
 *  1.  App loads, layout visible
 *  2.  Brain auto-configured (free API)
 *  3.  Voice auto-configured
 *  4.  Chat input UX (disabled/enabled, placeholder, clear after send)
 *  5.  Send message → real LLM response
 *  6.  Subtitle overlay (appears, clean — no markdown)
 *  7.  3D canvas visible with correct dimensions
 *  8.  VRM model loaded (triangle count > 0)
 *  9.  BGM music bar (play/pause, track cycling, volume)
 * 10.  Desktop nav switches all 6 tabs (high-level navigation only — detailed
 *      Memory tab coverage lives in memory-flow.spec.ts to avoid duplication)
 * 11.  No critical console errors
 *
 * NOTE: Detailed per-tab coverage (Memory, Skills, Voice, Marketplace) is
 * covered by dedicated specs (memory-flow.spec.ts) and by mobile-flow.spec.ts
 * which exercises the same Vue views via the bottom nav. Re-running them
 * here would burn ~30s of LLM time without adding signal.
 */
import { test, expect } from '@playwright/test';
import {
  collectConsoleErrors,
  assertNoCrashErrors,
  connectToDesktopApp,
  ensureDesktopTab,
  getPiniaState,
  sendMessage,
  openDrawer,
  closeDrawer,
  waitForAssistantResponse,
  waitForModelLoaded,
  navigateToTab,
  captureSubtitleOnce,
  TIMEOUTS,
} from './helpers';

const REAL_LOCAL_RESPONSE_TIMEOUT_MS = 90_000;

test('desktop: full end-to-end flow', async () => {
  const { page } = await connectToDesktopApp();
  await ensureDesktopTab(page, 'Chat');
  const errors = collectConsoleErrors(page);

  // ── 1. App loads and shows main layout ──────────────────────────────────
  await expect(page.locator('.chat-view')).toBeVisible();
  await expect(page.locator('.viewport-layer')).toBeVisible();
  await expect(page.locator('.input-footer')).toBeVisible();
  await expect(page.locator('.desktop-nav')).toBeVisible();

  // Settings dropdown opens and closes from the viewport toolbar.
  const settingsBtn = page.locator('.settings-toggle');
  await expect(settingsBtn).toBeVisible();
  await settingsBtn.click();
  await expect(page.locator('.settings-dropdown')).toBeVisible();
  const closeBtn = page.locator('.settings-close-btn');
  await closeBtn.click();
  await expect(page.locator('.settings-dropdown')).not.toBeVisible();

  // ── 2. LLM Brain is configured ──────────────────────────────────────────
  await expect(page.locator('.brain-setup')).not.toBeVisible();

  const brainState = (await getPiniaState(page, 'brain')) as any;
  expect(brainState).not.toBeNull();
  expect(['free_api', 'local_ollama', 'paid_api']).toContain(brainState.brainMode?.mode);
  if (brainState.brainMode?.mode === 'free_api') {
    expect(brainState.freeProviders?.length).toBeGreaterThan(0);
  }
  if (brainState.brainMode?.mode === 'local_ollama') {
    expect(brainState.brainMode?.model).toBeTruthy();
  }

  // ── 3. Voice config uses the real built-in providers ───────────────────
  await page.evaluate(async () => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    const voiceStore = app?.config.globalProperties.$pinia?._s?.get('voice');
    if (voiceStore && (!voiceStore.config?.asr_provider || !voiceStore.config?.tts_provider)) {
      await voiceStore.autoConfigureVoice();
    }
  });
  const voiceState = (await getPiniaState(page, 'voice')) as any;
  expect(voiceState).not.toBeNull();
  expect(voiceState.config?.asr_provider).toBe('web-speech');
  expect(voiceState.config?.tts_provider).toBe('web-speech');

  // ── 4. Chat input is interactive ────────────────────────────────────────
  const input = page.locator('.chat-input');
  const sendBtn = page.locator('.send-btn');
  await expect(input).toBeVisible();
  await expect(input).toBeEnabled();
  await expect(input).toHaveAttribute('placeholder', /Type a message/);
  await expect(sendBtn).toBeDisabled(); // disabled when empty
  await input.fill('hello');
  await expect(sendBtn).toBeEnabled();
  await input.fill(''); // reset

  // Attach button exists
  await expect(page.locator('.attach-btn')).toBeVisible();

  // ── 5 + 6. Send message → real LLM response, capture subtitle in-flight ─
  await page.evaluate(async () => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    const brainStore = app?.config.globalProperties.$pinia?._s?.get('brain');
    if (brainStore?.brainMode?.mode === 'local_ollama') {
      await brainStore.warmupLocalOllama(brainStore.brainMode.model);
    }
  });
  // Start the subtitle observer BEFORE sending: the overlay shows while
  // streaming and hides ~3s after TTS finishes (which never starts in a
  // browser context), so it's a race to assert visibility after the fact.
  const subtitleSeen = captureSubtitleOnce(page, REAL_LOCAL_RESPONSE_TIMEOUT_MS);

  await sendMessage(page, 'Reply only: ok');
  await expect(input).toHaveValue(''); // cleared after send

  // Wait for real LLM response from Pollinations (before opening drawer)
  const responseContent = await waitForAssistantResponse(page, {
    enforceLatencyBudget: false,
    timeoutMs: REAL_LOCAL_RESPONSE_TIMEOUT_MS,
  });
  expect(responseContent.length).toBeGreaterThan(0);

  // Now open drawer to verify messages are displayed
  await openDrawer(page);

  const userMsg = page.locator('.message-row.user').last();
  await expect(userMsg).toBeVisible({ timeout: TIMEOUTS.message });
  await expect(userMsg).toContainText('Reply only: ok');

  const assistantMsg = page.locator('.message-row.assistant').last();
  await expect(assistantMsg).toBeVisible({ timeout: TIMEOUTS.response });
  const responseText = await assistantMsg.textContent();
  expect(responseText).not.toContain('Error:');

  // ── 6. Subtitle was shown and contained no raw markdown ─────────────────
  const subtitleHtml = await subtitleSeen;
  expect(subtitleHtml, 'subtitle should have appeared during streaming').not.toBeNull();
  expect(subtitleHtml!.length).toBeGreaterThan(0);
  expect(subtitleHtml!).not.toMatch(/\*\*[^*]+\*\*/);

  // ── 7. 3D canvas visible ────────────────────────────────────────────────
  await closeDrawer(page);
  const canvas = page.locator('.viewport-canvas');
  await expect(canvas).toBeVisible();
  const box = await canvas.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeGreaterThan(100);
  expect(box!.height).toBeGreaterThan(100);

  // ── 8. VRM model loaded (triangle count > 0) ───────────────────────────
  const debugOverlay = await waitForModelLoaded(page);
  const triText = await debugOverlay.locator('span').nth(1).textContent();
  expect(parseInt(triText?.replace(/[^\d]/g, '') ?? '0', 10)).toBeGreaterThan(0);
  await expect(page.locator('.loading-overlay')).toBeHidden({ timeout: TIMEOUTS.vrmLoad });

  const hasVrm = await page.evaluate(() => !!(window as any).__terransoul_vrm__);
  expect(hasVrm).toBe(true);

  // Close debug overlay
  await page.keyboard.press('Control+d');

  // ── 9. BGM music bar ────────────────────────────────────────────────────
  const musicToggle = page.locator('.music-bar-toggle');
  if ((await musicToggle.count()) > 0) {
    await musicToggle.click({ force: true });
    const musicPanel = page.locator('.music-bar-panel');
    await expect(musicPanel).toBeVisible({ timeout: TIMEOUTS.panel });
    await expect(musicPanel.locator('.play-btn')).toBeVisible();
    await expect(musicPanel.locator('.music-track-name')).toBeVisible();
    await expect(musicPanel.locator('.music-vol-slider')).toBeVisible();

    const playBtn = page.locator('.music-btn.play-btn');
    await expect(playBtn).toContainText('▶️');
    await playBtn.click();
    await expect(playBtn).toContainText('⏸');
    await expect(page.locator('.music-bar')).toHaveClass(/playing/);
    await playBtn.click();
    await expect(playBtn).toContainText('▶️');

    const trackName = page.locator('.music-track-name');
    const initialTrack = await trackName.textContent();
    const nextBtn = page.locator('.music-bar-panel button[title="Next track"]');
    await nextBtn.click();
    await expect(async () => {
      const newTrack = await trackName.textContent();
      expect(newTrack).not.toBe(initialTrack);
    }).toPass({ timeout: 2_000 });

    const muteBtn = page.locator('.music-bar-panel button.mute-btn');
    await expect(muteBtn).toBeVisible();
    await expect(muteBtn).toContainText('🔊');
    await muteBtn.click();
    await expect(muteBtn).toContainText('🔇');
    await expect(muteBtn).toHaveClass(/active/);
    await muteBtn.click();
    await expect(muteBtn).toContainText('🔊');
    await expect(muteBtn).not.toHaveClass(/active/);

    await musicToggle.click();
  }

  // ── 10. Desktop nav — high-level tab switching ──────────────────────────
  // Each tab is verified to render its top-level view container. Detailed
  // per-tab assertions live in memory-flow.spec.ts and mobile-flow.spec.ts.
  await navigateToTab(page, 'Market');
  await expect(page.locator('.marketplace-view')).toBeVisible({ timeout: TIMEOUTS.panel });
  await expect(page.locator('.llm-config-header')).toContainText('Configure LLM');

  await navigateToTab(page, 'Memory');
  await expect(page.locator('.memory-view')).toBeVisible({ timeout: TIMEOUTS.panel });

  await navigateToTab(page, 'Brain');
  await expect(page.locator('[data-testid="brain-view"]')).toBeVisible({ timeout: TIMEOUTS.panel });
  await expect(page.locator('[data-testid="brain-avatar"]')).toBeVisible();
  await expect(page.locator('[data-testid="bv-mode-switcher"]')).toBeVisible();
  await expect(page.locator('[data-testid="bv-rag-capability"]')).toBeVisible();

  await navigateToTab(page, 'Quests');
  await expect(page.locator('.skill-tree-view')).toBeVisible({ timeout: TIMEOUTS.panel });
  await expect(page.locator('.st-progress-badge')).toBeVisible();

  await navigateToTab(page, 'Voice');
  await expect(page.locator('.voice-setup')).toBeVisible({ timeout: TIMEOUTS.panel });

  await navigateToTab(page, 'Chat');
  await expect(page.locator('.chat-view')).toBeVisible({ timeout: TIMEOUTS.panel });

  // ── 11. No critical console errors ──────────────────────────────────────
  assertNoCrashErrors(errors);
});
