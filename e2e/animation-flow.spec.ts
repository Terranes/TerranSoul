/**
 * Animation E2E — verifies LLM-driven emotion and body animation flow.
 *
 * Runs against the Vite dev server with REAL free API (Pollinations).
 * Tests that when the user prompts for specific emotions, the LLM responds
 * appropriately, <anim> tags are stripped from display, and the character
 * state updates.
 *
 * Sections:
 *  1.  App loads, VRM model ready, brain configured
 *  2.  "Clap" prompt → response, clean text (no raw <anim> tags)
 *  3.  "Angry" prompt → response, anger detected in state/sentiment/content
 *  4.  "Happy" prompt → response, positive sentiment
 *  5.  No critical console errors
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
  getLastAssistantMessage,
  waitForModelLoaded,
  TIMEOUTS,
} from './helpers';

test('animation: LLM responds with clap, angry, and happy emotions', { timeout: 180_000 }, async ({ page }) => {
  const errors = collectConsoleErrors(page);
  await page.goto('/');
  await waitForAppReady(page);

  // ── 1. VRM model loaded, brain configured ─────────────────────────────
  await waitForModelLoaded(page);
  // Close debug overlay
  await page.keyboard.press('Control+d');

  const brainState = (await getPiniaState(page, 'brain')) as any;
  expect(brainState?.brainMode?.mode).toBe('free_api');

  // ── 2. Ask model to clap ──────────────────────────────────────────────
  await sendMessage(page, 'Please clap your hands for me!');

  const clapResponse = await waitForAssistantResponse(page);
  expect(clapResponse.length).toBeGreaterThan(0);

  // Wait for emotion/animation pipeline to process
  await page.waitForTimeout(1_000);

  // Verify assistant message in drawer
  await openDrawer(page);
  const clapAssistant = page.locator('.message-row.assistant').last();
  await expect(clapAssistant).toBeVisible({ timeout: 5_000 });
  const clapText = await clapAssistant.textContent();
  expect(clapText).toBeTruthy();
  expect(clapText!.length).toBeGreaterThan(0);
  // No raw <anim> tags leaked into display
  expect(clapText).not.toContain('<anim>');
  expect(clapText).not.toContain('</anim>');
  await closeDrawer(page);

  // Verify message stored correctly
  const clapMsg = await getLastAssistantMessage(page);
  expect(clapMsg).not.toBeNull();
  expect(clapMsg.content).not.toContain('<anim>');

  // ── 3. Ask model to be angry ──────────────────────────────────────────
  await sendMessage(page, 'Be really angry at me! Yell at me!');

  const angryResponse = await waitForAssistantResponse(page);
  expect(angryResponse.length).toBeGreaterThan(0);

  await page.waitForTimeout(500);

  // Verify response in drawer
  await openDrawer(page);
  const angryAssistant = page.locator('.message-row.assistant').last();
  await expect(angryAssistant).toBeVisible({ timeout: 5_000 });
  const angryText = await angryAssistant.textContent();
  expect(angryText).toBeTruthy();
  expect(angryText).not.toContain('<anim>');
  await closeDrawer(page);

  // Check for anger signal: character state, sentiment, motion, or content
  await expect(async () => {
    const charState = (await getPiniaState(page, 'character')) as any;
    const lastMsg = await getLastAssistantMessage(page);
    expect(lastMsg).not.toBeNull();
    expect(lastMsg.content.length).toBeGreaterThan(0);

    const isAngryState = charState?.state === 'angry';
    const hasAngrySentiment = lastMsg.sentiment === 'angry';
    const hasAngryMotion = lastMsg.motion === 'angry';
    const responseContainsAnger = /ang|mad|fury|furi|upset|yell/i.test(lastMsg.content);
    expect(isAngryState || hasAngrySentiment || hasAngryMotion || responseContainsAnger).toBe(true);
  }).toPass({ timeout: 5_000 });

  // ── 4. Ask model to be happy ──────────────────────────────────────────
  await sendMessage(page, 'Now be super happy and excited! Jump for joy!');

  const happyResponse = await waitForAssistantResponse(page);
  expect(happyResponse.length).toBeGreaterThan(0);

  await page.waitForTimeout(500);

  // Verify response
  await openDrawer(page);
  const happyAssistant = page.locator('.message-row.assistant').last();
  await expect(happyAssistant).toBeVisible({ timeout: 5_000 });
  const happyText = await happyAssistant.textContent();
  expect(happyText).toBeTruthy();
  expect(happyText).not.toContain('<anim>');
  await closeDrawer(page);

  // Check for happiness signal
  await expect(async () => {
    const charState = (await getPiniaState(page, 'character')) as any;
    const lastMsg = await getLastAssistantMessage(page);
    expect(lastMsg).not.toBeNull();
    expect(lastMsg.content.length).toBeGreaterThan(0);

    const isHappyState = charState?.state === 'happy' || charState?.state === 'excited';
    const hasHappySentiment = lastMsg.sentiment === 'happy' || lastMsg.sentiment === 'excited';
    const hasHappyMotion = /happy|excited|jump|joy/i.test(lastMsg.motion ?? '');
    const responseContainsHappy = /happy|excit|joy|yay|hooray|wonderful/i.test(lastMsg.content);
    expect(isHappyState || hasHappySentiment || hasHappyMotion || responseContainsHappy).toBe(true);
  }).toPass({ timeout: 5_000 });

  // ── 5. No critical console errors ─────────────────────────────────────
  assertNoCrashErrors(errors);
});
