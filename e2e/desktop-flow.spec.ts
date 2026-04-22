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
 * 10.  Desktop nav → Marketplace tab
 * 11.  Desktop nav → Memory tab (stats dashboard, tier filters, search UI)
 * 12.  Desktop nav → Skills tab (skill tree renders)
 * 13.  Desktop nav → Voice tab (voice setup renders)
 * 14.  Chat-based LLM switching
 * 15.  Quest system (trigger phrase, accept quest)
 * 16.  No critical console errors
 */
import { test, expect } from '@playwright/test';
import {
  collectConsoleErrors,
  assertNoCrashErrors,
  waitForAppReady,
  getPiniaState,
  sendMessage,
  openDrawer,
  closeDrawer,
  waitForAssistantResponse,
  waitForModelLoaded,
  navigateToTab,
  TIMEOUTS,
} from './helpers';

test('desktop: full end-to-end flow', { timeout: 180_000 }, async ({ page }) => {
  const errors = collectConsoleErrors(page);
  await page.goto('/');
  await waitForAppReady(page);

  // ── 1. App loads and shows main layout ──────────────────────────────────
  await expect(page.locator('.chat-view')).toBeVisible();
  await expect(page.locator('.viewport-layer')).toBeVisible();
  await expect(page.locator('.input-footer')).toBeVisible();
  await expect(page.locator('.desktop-nav')).toBeVisible();

  // AI state pill starts at Idle
  const badge = page.locator('.ai-state-pill');
  await expect(badge).toBeVisible();
  await expect(badge).toContainText('Idle');

  // ── 2. Free LLM Brain auto-configured ───────────────────────────────────
  await expect(page.locator('.brain-setup')).not.toBeVisible();

  const brainState = (await getPiniaState(page, 'brain')) as any;
  expect(brainState).not.toBeNull();
  expect(brainState.brainMode?.mode).toBe('free_api');
  expect(brainState.freeProviders?.length).toBeGreaterThan(0);

  // ── 3. Voice auto-configured ────────────────────────────────────────────
  const voiceState = (await getPiniaState(page, 'voice')) as any;
  expect(voiceState).not.toBeNull();
  expect(voiceState.config?.asr_provider).toBe('web-speech');
  expect(voiceState.config?.tts_provider).toBe('edge-tts');

  // ── 4. Chat input is interactive ────────────────────────────────────────
  const input = page.locator('.chat-input');
  const sendBtn = page.locator('.send-btn');
  await expect(input).toBeVisible();
  await expect(input).toBeEnabled();
  await expect(input).toHaveAttribute('placeholder', 'Type a message…');
  await expect(sendBtn).toBeDisabled(); // disabled when empty
  await input.fill('hello');
  await expect(sendBtn).toBeEnabled();
  await input.fill(''); // reset

  // Attach button exists
  await expect(page.locator('.attach-btn')).toBeVisible();

  // ── 5. Send message → get REAL LLM response ────────────────────────────
  await sendMessage(page, 'Hello there!');
  await expect(input).toHaveValue(''); // cleared after send

  // Wait for real LLM response from Pollinations (before opening drawer)
  const responseContent = await waitForAssistantResponse(page);
  expect(responseContent.length).toBeGreaterThan(0);

  // Now open drawer to verify messages are displayed
  await openDrawer(page);

  const userMsg = page.locator('.message-row.user').first();
  await expect(userMsg).toBeVisible({ timeout: TIMEOUTS.message });
  await expect(userMsg).toContainText('Hello there!');

  const assistantMsg = page.locator('.message-row.assistant').first();
  await expect(assistantMsg).toBeVisible({ timeout: TIMEOUTS.response });
  const responseText = await assistantMsg.textContent();
  expect(responseText).not.toContain('Error:');

  // ── 6. Subtitle appears and has no raw markdown ─────────────────────────
  const subtitle = page.locator('.subtitle-overlay');
  await expect(subtitle).toBeVisible({ timeout: TIMEOUTS.response });
  const subtitleText = page.locator('.subtitle-text');
  const subtitleContent = await subtitleText.textContent();
  expect(subtitleContent).toBeTruthy();
  expect(subtitleContent!.length).toBeGreaterThan(0);
  // No raw markdown asterisks
  const subtitleHtml = await subtitleText.innerHTML();
  expect(subtitleHtml).not.toMatch(/\*\*[^*]+\*\*/);

  // ── 7. 3D canvas visible ────────────────────────────────────────────────
  await closeDrawer(page);
  // Subtitle may persist for long TTS playback — wait generously or skip
  await expect(page.locator('.subtitle-overlay')).toBeHidden({ timeout: 60_000 }).catch(() => {
    // Subtitle still showing (long TTS) — that's fine, test canvas anyway
  });
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
  await musicToggle.click({ force: true });
  const musicPanel = page.locator('.music-bar-panel');
  await expect(musicPanel).toBeVisible({ timeout: TIMEOUTS.panel });
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
  }).toPass({ timeout: 2_000 });

  await musicToggle.click();

  // ── 10. Navigate to Marketplace ─────────────────────────────────────────
  await navigateToTab(page, 'Market');
  await expect(page.locator('.marketplace-view')).toBeVisible({ timeout: TIMEOUTS.panel });
  const llmConfigHeader = page.locator('.llm-config-header');
  await expect(llmConfigHeader).toBeVisible({ timeout: TIMEOUTS.panel });
  await expect(llmConfigHeader).toContainText('Configure LLM');

  // ── 11. Navigate to Memory tab ──────────────────────────────────────────
  await navigateToTab(page, 'Memory');
  const memoryView = page.locator('.memory-view');
  await expect(memoryView).toBeVisible({ timeout: TIMEOUTS.panel });

  // Stats dashboard loads when Tauri backend is available
  const hasTauri = await page.evaluate(() => '__TAURI_INTERNALS__' in window);
  if (hasTauri) {
    const statsPanel = memoryView.locator('.mv-stats');
    await expect(statsPanel).toBeVisible({ timeout: TIMEOUTS.panel });
    await expect(statsPanel.locator('.mv-stat')).toHaveCount(6);
  }

  // Memory tabs visible (List, Graph, Session)
  await expect(memoryView.locator('.mv-tab')).toHaveCount(3);

  // List tab active by default
  await expect(memoryView.locator('.mv-list-panel')).toBeVisible();

  // Search bar functional
  const memSearch = memoryView.locator('.mv-search');
  await expect(memSearch).toBeVisible();
  await expect(memSearch).toHaveAttribute('placeholder', 'Search memories…');

  // Filter chips: 4 type + 3 tier
  const filterRow = memoryView.locator('.mv-filter-row');
  await expect(filterRow.locator('.mv-type-chip')).toHaveCount(4);
  await expect(filterRow.locator('.mv-tier-chip')).toHaveCount(3);

  // Header action buttons visible
  await expect(memoryView.locator('button', { hasText: 'Extract from session' })).toBeVisible();
  await expect(memoryView.locator('button', { hasText: 'Decay' })).toBeVisible();
  await expect(memoryView.locator('button', { hasText: 'GC' })).toBeVisible();
  await expect(memoryView.locator('button', { hasText: 'Add memory' })).toBeVisible();

  // Switch to Graph tab
  await memoryView.locator('.mv-tab', { hasText: 'Graph' }).click();
  await expect(memoryView.locator('.mv-graph-panel')).toBeVisible({ timeout: TIMEOUTS.panel });

  // Switch to Session tab
  await memoryView.locator('.mv-tab', { hasText: 'Session' }).click();
  await expect(memoryView.locator('.mv-session-panel')).toBeVisible({ timeout: TIMEOUTS.panel });

  // ── 12. Navigate to Skills tab ──────────────────────────────────────────
  await navigateToTab(page, 'Quests');
  await expect(page.locator('.skill-tree-view')).toBeVisible({ timeout: TIMEOUTS.panel });
  await expect(page.locator('.st-header')).toBeVisible();
  await expect(page.locator('.st-progress-badge')).toBeVisible();

  // ── 13. Navigate to Voice tab ───────────────────────────────────────────
  await navigateToTab(page, 'Voice');
  await expect(page.locator('.voice-setup')).toBeVisible({ timeout: TIMEOUTS.panel });
  await expect(page.locator('.vs-steps')).toBeVisible();

  // Voice mode tiers should be shown (browser, cloud, groq, text-only)
  await expect(page.locator('.vs-tier')).toHaveCount(4);

  // Navigate back to Chat
  await navigateToTab(page, 'Chat');
  await expect(page.locator('.chat-view')).toBeVisible({ timeout: TIMEOUTS.panel });

  // ── 14. Chat-based LLM switching ────────────────────────────────────────
  await sendMessage(page, 'switch to pollinations');
  await openDrawer(page);
  const switchUserMsg = page.locator('.message-row.user').last();
  await expect(switchUserMsg).toContainText('switch to pollinations');
  const switchResponse = await waitForAssistantResponse(page);
  expect(switchResponse.length).toBeGreaterThan(0);
  await closeDrawer(page);

  // ── 15. Quest system — trigger phrase ───────────────────────────────────
  await sendMessage(page, 'Where can I start?');
  await openDrawer(page);

  const questAssistant = page.locator('.message-row.assistant').last();
  await expect(questAssistant).toBeVisible({ timeout: TIMEOUTS.response });

  // Typing indicator clears (not stuck)
  await expect(page.locator('.typing-indicator')).not.toBeVisible({ timeout: 5_000 });

  // If quest overlay appeared, walk through the accept flow
  const questOverlay = page.locator('.hotseat-strip');
  const hasQuest = await questOverlay.isVisible().catch(() => false);
  if (hasQuest) {
    const tiles = questOverlay.locator('.hotseat-tile');
    const labels = (await tiles.allTextContents()).join(' ');

    if (labels.includes('Accept') && labels.includes('Maybe later')) {
      for (let i = 0; i < (await tiles.count()); i++) {
        const txt = await tiles.nth(i).textContent();
        if (txt && txt.includes('Accept')) {
          await tiles.nth(i).click();
          break;
        }
      }
      await page.waitForTimeout(500);

      const acceptMsg = page.locator('.message-row.assistant').last();
      await expect(acceptMsg).toContainText('Quest Accepted', { timeout: TIMEOUTS.response });
    }
  }
  await closeDrawer(page);

  // ── 16. No critical console errors ──────────────────────────────────────
  assertNoCrashErrors(errors);
});
