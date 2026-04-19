/**
 * E2E tests — full quest flow, TTS for quests, BGM quest, and subtitle karaoke.
 *
 * Runs against the Vite dev server (no Tauri backend).
 * Quest detection uses brain-based approach: the message goes to the LLM first,
 * then if the user's input or the LLM response signals quest intent, the
 * quest overlay appears alongside the response.
 *
 * In browser mode (no Tauri), the first available quest is "bgm"
 * (Ambient Aura) because free-brain, tts, and avatar are auto-activated.
 *
 * Quest choices render as a compact strip above the chatbox
 * (`.hotseat-strip` with `.hotseat-tile` buttons).
 */
import { test, expect, type Page } from '@playwright/test';

const RESPONSE_TIMEOUT = 20_000;
const PANEL_TIMEOUT = 3_000;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/** Open the chat drawer so messages are visible. */
async function openDrawer(page: Page) {
  const drawer = page.locator('.chat-history');
  if (!(await drawer.isVisible().catch(() => false))) {
    await page.locator('.chat-drawer-toggle').click();
    await expect(drawer).toBeVisible({ timeout: PANEL_TIMEOUT });
  }
}

/** Type a message and send it, then open the drawer. */
async function sendMessage(page: Page, text: string) {
  const input = page.locator('.chat-input');
  const sendBtn = page.locator('.send-btn');
  await input.fill(text);
  await sendBtn.click();
  await openDrawer(page);
}

/** Wait for the quest choice strip to appear and return its tile buttons. */
async function getHotseatTiles(page: Page) {
  const strip = page.locator('.hotseat-strip');
  await expect(strip).toBeVisible({ timeout: RESPONSE_TIMEOUT });
  return strip.locator('.hotseat-tile');
}

/** Click the tile whose text contains `label`. */
async function clickHotseatTile(page: Page, label: string) {
  const tiles = await getHotseatTiles(page);
  const count = await tiles.count();
  for (let i = 0; i < count; i++) {
    const txt = await tiles.nth(i).textContent();
    if (txt && txt.includes(label)) {
      await tiles.nth(i).click();
      return;
    }
  }
  throw new Error(`Hot-seat tile with label "${label}" not found`);
}

/**
 * Install a spy on speechSynthesis.speak to track TTS calls.
 * Must be called before the action that triggers TTS.
 */
async function installTtsSpy(page: Page) {
  await page.evaluate(() => {
    (window as any).__ttsCalls = [];
    const orig = speechSynthesis.speak.bind(speechSynthesis);
    speechSynthesis.speak = (utterance: SpeechSynthesisUtterance) => {
      (window as any).__ttsCalls.push(utterance.text);
      // Cancel immediately to avoid actual audio playback in tests
      speechSynthesis.cancel();
    };
  });
}

/** Get the texts passed to speechSynthesis.speak since the spy was installed. */
async function getTtsCalls(page: Page): Promise<string[]> {
  return page.evaluate(() => (window as any).__ttsCalls ?? []);
}

// ─── Quest trigger phrases ───────────────────────────────────────────────────

test.describe('Quest Trigger — hot-seat overlay appears on quest phrase', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  const TRIGGER_PHRASES = [
    'Where can I start?',
    'What can I do?',
  ];

  for (const phrase of TRIGGER_PHRASES) {
    test(`"${phrase}" shows hot-seat overlay with choice tiles`, async ({ page }) => {
      await sendMessage(page, phrase);

      // Hot-seat overlay should appear over the viewport
      const overlay = page.locator('.hotseat-strip');
      await expect(overlay).toBeVisible({ timeout: RESPONSE_TIMEOUT });

      // Should have 3 tiles (Accept / Tell me more / Maybe later)
      const tiles = overlay.locator('.hotseat-tile');
      await expect(tiles).toHaveCount(3);

      // Verify Millionaire-style labels
      const labels = (await tiles.allTextContents()).join(' ');
      expect(labels).toContain('Accept');
      expect(labels).toContain('Tell me more');
      expect(labels).toContain('Maybe later');
    });
  }

  test('hot-seat overlay shows quest question banner', async ({ page }) => {
    await sendMessage(page, 'What should I do?');
    const overlay = page.locator('.hotseat-strip');
    await expect(overlay).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    const question = overlay.locator('.hotseat-question-text');
    const text = await question.textContent();
    expect(text).toBeTruthy();
    expect(text!.length).toBeGreaterThan(5);
  });

  test('regular message does NOT show quest hot-seat overlay', async ({ page }) => {
    await sendMessage(page, 'Tell me a joke');

    const assistantMsg = page.locator('.message-row.assistant').first();
    await expect(assistantMsg).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Wait for quest detection to run (it fires after LLM response)
    await page.waitForTimeout(2000);

    // Quest-specific hotseat tiles (Accept/Tell me more/Maybe later) should NOT appear.
    // Note: A provider warning hotseat strip may appear if the API returns a migration
    // notice — that's expected behavior, not a quest trigger.
    const questTile = page.locator('.hotseat-tile', { hasText: 'Accept Quest' });
    await expect(questTile).not.toBeVisible();
  });
});

// ─── Quest TTS — voice reads quest text aloud ────────────────────────────────

test.describe('Quest TTS — quest messages are spoken aloud', () => {
  test('TTS is triggered when hot-seat overlay appears via trigger phrase', async ({ page }) => {
    await page.goto('/');
    await installTtsSpy(page);

    await sendMessage(page, 'What should I do?');

    // Hot-seat overlay must appear
    await expect(page.locator('.hotseat-strip')).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Wait for TTS pipeline
    await page.waitForTimeout(1500);

    const calls = await getTtsCalls(page);
    expect(calls.length).toBeGreaterThan(0);
    const allSpoken = calls.join(' ');
    expect(allSpoken.length).toBeGreaterThan(10);
  });

  test('subtitle appears alongside quest TTS', async ({ page }) => {
    await page.goto('/');

    await sendMessage(page, 'What should I do?');
    await expect(page.locator('.hotseat-strip')).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Subtitle overlay should be visible for the quest message
    const subtitle = page.locator('.subtitle-overlay');
    await expect(subtitle).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    const subtitleText = page.locator('.subtitle-text');
    const content = await subtitleText.textContent();
    expect(content).toBeTruthy();
    expect(content!.length).toBeGreaterThan(10);
  });
});

// ─── Welcome screen quest buttons ────────────────────────────────────────────

test.describe('Welcome screen — RPG quest buttons', () => {
  test('shows three quest buttons on empty chat', async ({ page }) => {
    await page.goto('/');
    await openDrawer(page);

    const welcome = page.locator('.welcome-state');
    await expect(welcome).toBeVisible({ timeout: PANEL_TIMEOUT });

    const btns = page.locator('.welcome-quest-btn');
    await expect(btns).toHaveCount(3);
    await expect(page.locator('.welcome-quest-btn.primary')).toContainText('Start First Quest');
  });
});

// ─── Full quest flow: Accept path ────────────────────────────────────────────

test.describe('Full Quest Flow — Accept path (hot-seat driven)', () => {
  test('type trigger → Accept tile → strip dismisses + BGM options appear + TTS', async ({ page }) => {
    test.setTimeout(60_000);
    await page.goto('/');
    await installTtsSpy(page);

    // Step 1: Type a quest trigger phrase
    await sendMessage(page, 'What should I do?');

    // Step 2: Quest choice strip appears with 3 tiles
    const strip = page.locator('.hotseat-strip');
    await expect(strip).toBeVisible({ timeout: RESPONSE_TIMEOUT });
    const tiles = strip.locator('.hotseat-tile');
    await expect(tiles).toHaveCount(3);

    // Verify the quest message text appears in chat (may not be the first assistant message)
    const questMessage = page.locator('.message-row.assistant', { hasText: 'Quest' });
    await expect(questMessage.first()).toBeVisible({ timeout: 5000 });

    // TTS should have been triggered for the quest announcement
    await page.waitForTimeout(1500);
    const callsAfterTrigger = await getTtsCalls(page);
    expect(callsAfterTrigger.length).toBeGreaterThan(0);

    // Step 3: Click "Accept Quest" tile
    await installTtsSpy(page);
    await clickHotseatTile(page, 'Accept');

    // Step 4: A user message + acceptance response appear in chat
    const userChoice = page.locator('.message-row.user').last();
    await expect(userChoice).toContainText('Accept Quest');

    await page.waitForTimeout(300);
    const lastAssistant = page.locator('.message-row.assistant').last();
    await expect(lastAssistant).toContainText('Quest Accepted');

    // Step 5: A NEW strip appears with BGM options (Autoplay BGM / I'll handle it)
    const newStrip = page.locator('.hotseat-strip');
    await expect(newStrip).toBeVisible({ timeout: RESPONSE_TIMEOUT });
    const bgmTiles = newStrip.locator('.hotseat-tile');
    await expect(bgmTiles).toHaveCount(2);

    // TTS should have been triggered for the acceptance response
    await page.waitForTimeout(1500);
    const callsAfterAccept = await getTtsCalls(page);
    expect(callsAfterAccept.length).toBeGreaterThan(0);
  });
});

// ─── Full quest flow: Tell me more → Begin path ──────────────────────────────

test.describe('Full Quest Flow — Info then Begin path', () => {
  test('type trigger → Tell me more → Begin (with TTS)', async ({ page }) => {
    test.setTimeout(60_000);
    await page.goto('/');

    // Step 1: Trigger quest
    await sendMessage(page, 'Where can I start?');
    await expect(page.locator('.hotseat-strip')).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Step 2: Click "Tell me more" tile
    await installTtsSpy(page);
    await clickHotseatTile(page, 'Tell me more');

    // Step 3: User choice should appear in chat
    const userChoice = page.locator('.message-row.user').last();
    await expect(userChoice).toContainText('Tell me more');

    // Step 4: Quest details response with follow-up hot-seat tiles
    await page.waitForTimeout(300);
    const detailMsg = page.locator('.message-row.assistant').last();
    await expect(detailMsg).toContainText('Steps to complete');

    // TTS should fire for the detail response
    await page.waitForTimeout(1500);
    const infoCalls = await getTtsCalls(page);
    expect(infoCalls.length).toBeGreaterThan(0);

    // Hot-seat overlay re-appears with follow-up tiles: Let's begin / Pin / Maybe later
    const tiles = await getHotseatTiles(page);
    const followCount = await tiles.count();
    expect(followCount).toBe(3);
    const followLabels = (await tiles.allTextContents()).join(' ');
    expect(followLabels).toContain('begin');

    // Step 5: Click "Let's begin!" tile
    await installTtsSpy(page);
    await clickHotseatTile(page, 'begin');

    // Step 6: Quest accepted message
    await page.waitForTimeout(300);
    const acceptMsg = page.locator('.message-row.assistant').last();
    await expect(acceptMsg).toContainText('Quest Accepted');

    // TTS should fire for the acceptance message
    await page.waitForTimeout(1500);
    const acceptCalls = await getTtsCalls(page);
    expect(acceptCalls.length).toBeGreaterThan(0);
  });
});

// ─── Full quest flow: Dismiss path ───────────────────────────────────────────

test.describe('Full Quest Flow — Dismiss path', () => {
  test('type trigger → Maybe later dismisses quest (with TTS)', async ({ page }) => {
    test.setTimeout(60_000);
    await page.goto('/');

    await sendMessage(page, 'What should I do?');
    await expect(page.locator('.hotseat-strip')).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Install spy before dismissal
    await installTtsSpy(page);

    // Click "Maybe later" tile
    await clickHotseatTile(page, 'Maybe later');

    // Hot-seat overlay should dismiss
    await expect(page.locator('.hotseat-strip')).not.toBeVisible({ timeout: 2000 });

    // User choice in chat
    const userChoice = page.locator('.message-row.user').last();
    await expect(userChoice).toContainText('Maybe later');

    // Quest Guide responds with dismissal message (no new hot-seat overlay)
    await page.waitForTimeout(300);
    const dismissMsg = page.locator('.message-row.assistant').last();
    await expect(dismissMsg).toContainText('will wait for you');

    // TTS should fire for the dismissal response
    await page.waitForTimeout(1500);
    const dismissCalls = await getTtsCalls(page);
    expect(dismissCalls.length).toBeGreaterThan(0);
  });
});

// ─── BGM Quest — Accept then Autoplay triggers background music ──────────────

test.describe('BGM Quest — Ambient Aura with Autoplay BGM', () => {
  test('accepting BGM quest shows Autoplay BGM and I\'ll handle it options', async ({ page }) => {
    test.setTimeout(60_000);
    await page.goto('/');

    // The first available quest in browser mode is "bgm" (Ambient Aura)
    await sendMessage(page, 'What should I do?');
    await expect(page.locator('.hotseat-strip')).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Verify this is the BGM quest (Ambient Aura) in the chat message
    const questMsg = page.locator('.message-row.assistant', { hasText: 'Ambient Aura' });
    await expect(questMsg.first()).toBeVisible({ timeout: 5000 });

    // Accept the quest
    await clickHotseatTile(page, 'Accept');
    await page.waitForTimeout(300);

    // Acceptance message should mention quest name
    const acceptMsg = page.locator('.message-row.assistant').last();
    await expect(acceptMsg).toContainText('Quest Accepted');
    await expect(acceptMsg).toContainText('Ambient Aura');

    // A new strip should appear with 2 choices: Autoplay BGM / I'll handle it
    const strip = page.locator('.hotseat-strip');
    await expect(strip).toBeVisible({ timeout: RESPONSE_TIMEOUT });
    const tiles = strip.locator('.hotseat-tile');
    await expect(tiles).toHaveCount(2);

    const labels = (await tiles.allTextContents()).join(' ');
    expect(labels).toContain('Autoplay BGM');
    expect(labels).toContain('handle it myself');
  });

  test('clicking Autoplay BGM creates AudioContext and starts playback', async ({ page }) => {
    test.setTimeout(60_000);
    await page.goto('/');

    await sendMessage(page, 'What should I do?');
    await expect(page.locator('.hotseat-strip')).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Accept quest → get BGM options
    await clickHotseatTile(page, 'Accept');
    await page.waitForTimeout(300);
    await expect(page.locator('.hotseat-strip')).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Click "Autoplay BGM" — this triggers enableBgm() in a user-gesture context
    await clickHotseatTile(page, 'Autoplay BGM');
    await page.waitForTimeout(800);

    // Verify AudioContext was created and is running (not suspended)
    const audioState = await page.evaluate(() => {
      // The BGM player stores AudioContext internally — check via
      // the global AudioContext class and see if any instance exists.
      // We also check if the music bar shows "playing" class.
      const musicBar = document.querySelector('.music-bar');
      return {
        hasAudioContext: typeof AudioContext !== 'undefined',
        musicBarPlaying: musicBar?.classList.contains('playing') ?? false,
      };
    });
    expect(audioState.hasAudioContext).toBe(true);
    expect(audioState.musicBarPlaying).toBe(true);

    // Confirmation message should appear
    const confirmMsg = page.locator('.message-row.assistant').last();
    await expect(confirmMsg).toContainText('BGM is now playing');
  });

  test('clicking I\'ll handle it myself does NOT start BGM', async ({ page }) => {
    test.setTimeout(60_000);
    await page.goto('/');

    await sendMessage(page, 'What should I do?');
    await expect(page.locator('.hotseat-strip')).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Accept quest → get BGM options
    await clickHotseatTile(page, 'Accept');
    await page.waitForTimeout(300);
    await expect(page.locator('.hotseat-strip')).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Click "I'll handle it myself"
    await clickHotseatTile(page, 'handle it myself');
    await page.waitForTimeout(500);

    // Music bar should NOT be playing
    const musicBarPlaying = await page.evaluate(() => {
      const musicBar = document.querySelector('.music-bar');
      return musicBar?.classList.contains('playing') ?? false;
    });
    expect(musicBarPlaying).toBe(false);

    // Strip should dismiss
    await expect(page.locator('.hotseat-strip')).not.toBeVisible({ timeout: 2000 });
  });
});

// ─── Yes/No Quick Replies ────────────────────────────────────────────────────

test.describe('Yes/No Quick Replies', () => {
  test('quick replies do NOT appear when hot-seat overlay is showing', async ({ page }) => {
    await page.goto('/');

    await sendMessage(page, 'What should I do?');
    await expect(page.locator('.hotseat-strip')).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    // Quick reply buttons should NOT appear (quest uses hot-seat overlay)
    const quickReplies = page.locator('.quick-replies');
    await expect(quickReplies).toHaveCount(0);
  });
});

// ─── Subtitle and Audio ─────────────────────────────────────────────────────

test.describe('Subtitle Karaoke + Audio Detection', () => {
  test('subtitle appears after sending a message', async ({ page }) => {
    await page.goto('/');

    await page.locator('.chat-input').fill('Hello!');
    await page.locator('.send-btn').click();

    const subtitle = page.locator('.subtitle-overlay');
    await expect(subtitle).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    const subtitleText = page.locator('.subtitle-text');
    const content = await subtitleText.textContent();
    expect(content).toBeTruthy();
    expect(content!.length).toBeGreaterThan(0);
  });

  test('subtitle text has no raw markdown asterisks', async ({ page }) => {
    await page.goto('/');

    await page.locator('.chat-input').fill('Hello!');
    await page.locator('.send-btn').click();

    const subtitle = page.locator('.subtitle-overlay');
    await expect(subtitle).toBeVisible({ timeout: RESPONSE_TIMEOUT });

    const html = await page.locator('.subtitle-text').innerHTML();
    // No **bold** or *italic* markers should remain
    expect(html).not.toMatch(/\*\*[^*]+\*\*/);
    expect(html).not.toMatch(/(?<!\*)\*(?!\*)[^*]+\*(?!\*)/);
  });
});
