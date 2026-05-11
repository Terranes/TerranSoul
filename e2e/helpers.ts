/**
 * Shared E2E test helpers for TerranSoul Playwright tests.
 *
 * Browser/mobile tests run against the Vite page. Desktop-shell tests attach
 * to the running Tauri WebView through WebView2 CDP.
 */
import { chromium, expect, type Browser, type Page } from '@playwright/test';

export const TAURI_CDP_ENDPOINT = process.env.TERRANSOUL_TAURI_CDP_ENDPOINT
  ?? process.env.TERRANSOUL_TAURI_CDP
  ?? 'http://127.0.0.1:9222';
export const DESKTOP_URL_PREFIX = process.env.TERRANSOUL_DESKTOP_URL_PREFIX ?? 'http://localhost:1420';

// ─── Timeouts ────────────────────────────────────────────────────────────────
export const TIMEOUTS = {
  message: 5_000,
  response: 30_000,
  panel: 3_000,
  vrmLoad: 30_000,
  appInit: 30_000,
} as const;

export const LOCAL_E2E_RESPONSE_LATENCY_BUDGET_MS = 2_000;

function shouldEnforceLocalLatencyBudget() {
  return !process.env.CI;
}

function latencyFailureMessage(actualMs: number) {
  return [
    `Assistant response latency was ${Math.round(actualMs)}ms, above the local E2E budget of ${LOCAL_E2E_RESPONSE_LATENCY_BUDGET_MS}ms.`,
    'Investigate and fix the latency path: model warmup, provider selection, RAG retrieval, streaming first chunk, or UI state propagation.',
    'Do not fix this by increasing Playwright timeouts or relaxing the latency assertion.',
  ].join(' ');
}

// ─── Console error collector ─────────────────────────────────────────────────

/** Known benign errors that fire when running outside Tauri shell. */
const IGNORED_PATTERNS = [
  '__TAURI_INTERNALS__',
  'process_prompt_silently',
  'Vercel',
  'net::ERR_',        // network errors from optional resources
  'Failed to fetch',  // optional Ollama health checks
];

export function collectConsoleErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      const text = msg.text();
      if (IGNORED_PATTERNS.some((p) => text.includes(p))) return;
      errors.push(text);
    }
  });
  page.on('pageerror', (err) => {
    const text = err.message;
    if (IGNORED_PATTERNS.some((p) => text.includes(p))) return;
    errors.push(`UNCAUGHT: ${text}`);
  });
  return errors;
}

/** Assert no critical crash-level errors were captured. */
export function assertNoCrashErrors(errors: string[]) {
  const crash = errors.filter(
    (e) =>
      e.includes('Cannot read properties of undefined') ||
      e.includes('Cannot read properties of null') ||
      e.includes('UNCAUGHT') ||
      e.includes('Unhandled error') ||
      e.includes('is not a function'),
  );
  expect(crash).toHaveLength(0);
}

// ─── App initialization ──────────────────────────────────────────────────────

export async function connectToDesktopApp(): Promise<{ browser: Browser; page: Page }> {
  let browser: Browser;
  try {
    browser = await chromium.connectOverCDP(TAURI_CDP_ENDPOINT);
  } catch (err) {
    throw new Error(
      `Real desktop E2E requires the Tauri WebView CDP endpoint at ${TAURI_CDP_ENDPOINT}. `
      + `Start the desktop app with npm run dev:desktop-e2e, then retry. Original error: ${String(err)}`,
    );
  }

  const pages = browser
    .contexts()
    .flatMap((context) => context.pages())
    .filter((page) => !page.url().startsWith('devtools://'));
  const page = pages.find((candidate) => candidate.url().startsWith(DESKTOP_URL_PREFIX)) ?? pages[0];
  if (!page) {
    await browser.close();
    throw new Error(`No Tauri WebView page found on ${TAURI_CDP_ENDPOINT}.`);
  }

  await page.setViewportSize({ width: 1280, height: 720 });

  // Auto-accept all confirm/prompt dialogs. WebView2 CDP does not auto-accept
  // them — without this handler, confirm() returns false and actions like
  // memory delete are silently cancelled.
  page.on('dialog', (dialog) => dialog.accept().catch(() => {}));

  await completeFirstLaunchRecommendedIfPresent(page);
  await resetDesktopE2EState(page);
  await closeOpenDialogIfPresent(page);
  await page.waitForSelector('.app-shell', { timeout: TIMEOUTS.appInit });
  await page.waitForSelector('.chat-view', { state: 'attached', timeout: TIMEOUTS.appInit });
  await page.waitForSelector('.chat-input', { state: 'attached', timeout: TIMEOUTS.appInit });
  return { browser, page };
}

async function resetDesktopE2EState(page: Page): Promise<void> {
  await page.evaluate(async () => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    const pinia = app?.config?.globalProperties?.$pinia;
    const windowStore = pinia?._s?.get('window');
    if (windowStore) {
      windowStore.mode = 'window';
      windowStore.isMcpMode = false;
    }
    const conversationStore = pinia?._s?.get('conversation');
    if (conversationStore) {
      conversationStore.stopGeneration?.();
      const deadline = Date.now() + 5_000;
      while (conversationStore.generationActive && Date.now() < deadline) {
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
      conversationStore.messages = [];
      conversationStore.isThinking = false;
      conversationStore.isStreaming = false;
      conversationStore.streamingText = '';
      conversationStore.generationActive = false;
      conversationStore.messageQueue = [];
    }
    if (pinia?.state?.value?.window) {
      pinia.state.value.window.mode = 'window';
      pinia.state.value.window.isMcpMode = false;
    }
    if (pinia?.state?.value?.conversation) {
      pinia.state.value.conversation.messages = [];
      pinia.state.value.conversation.isThinking = false;
      pinia.state.value.conversation.isStreaming = false;
      pinia.state.value.conversation.streamingText = '';
      pinia.state.value.conversation.messageQueue = [];
    }
    delete (window as any).__tsE2ELastSend;
  }).catch(() => undefined);
}

export async function closeOpenDialogIfPresent(page: Page): Promise<void> {
  const closeButton = page.locator('.kq-dialog .kq-close, .kq-dialog button[aria-label="Close"]').first();
  if (await closeButton.isVisible({ timeout: 500 }).catch(() => false)) {
    await closeButton.click();
    await expect(page.locator('.kq-dialog')).not.toBeVisible({ timeout: 5_000 });
  }
}

export async function completeFirstLaunchRecommendedIfPresent(page: Page): Promise<void> {
  const wizard = page.locator('.flw-dialog').first();
  if (!(await wizard.isVisible({ timeout: 1_000 }).catch(() => false))) return;

  await wizard.locator('.flw-option--recommended').click();

  const diskContinue = wizard.locator('.flw-done-btn', { hasText: 'Continue with Download' });
  if (await diskContinue.isVisible({ timeout: 5_000 }).catch(() => false)) {
    await diskContinue.click();
  }

  const doneBtn = wizard.locator('.flw-done-btn', { hasText: /Start Chatting|Continue Anyway/ });
  await expect(doneBtn).toBeVisible({ timeout: 120_000 });
  await doneBtn.click();
  await expect(page.locator('.flw-backdrop')).not.toBeVisible({ timeout: 5_000 });
}

export async function ensureDesktopTab(page: Page, tabName = 'Chat'): Promise<void> {
  const tab = page.locator('.nav-btn', { hasText: tabName }).first();
  await expect(tab).toBeVisible({ timeout: TIMEOUTS.appInit });
  await tab.click();
  if (tabName === 'Chat') {
    await stopGenerationIfPresent(page);
  }
}

export async function stopGenerationIfPresent(page: Page): Promise<void> {
  const stopButton = page.getByRole('button', { name: /^Stop generation$/ }).first();
  if (await stopButton.isVisible({ timeout: 500 }).catch(() => false)) {
    await stopButton.click();
    await expect(stopButton).not.toBeVisible({ timeout: 5_000 });
  }
}

/** Wait for the Vue app to fully initialize (splash hidden, chat-view visible). */
export async function waitForAppReady(page: Page) {
  const landing = page.locator('.browser-landing');
  if (await landing.isVisible({ timeout: 5_000 }).catch(() => false)) {
    const openAppButton = landing.locator('.primary-action, .nav-cta').first();
    await expect(openAppButton).toBeVisible({ timeout: 5_000 });
    await openAppButton.click();
  }

  const browserAppWindow = page.locator('.browser-app-window');
  if ((await browserAppWindow.count()) > 0) {
    await expect(browserAppWindow).toBeVisible({ timeout: 5_000 });
  }

  await page.waitForFunction(
    () => {
      const app = (document.querySelector('#app') as any)?.__vue_app__;
      if (!app) return false;
      const pinia = app.config.globalProperties.$pinia;
      if (!pinia) return false;
      if (document.querySelector('.browser-landing') && !document.querySelector('.browser-app-window')) {
        return false;
      }
      const shell = document.querySelector('.browser-app-window') ?? document;
      const chatView = shell.querySelector('.chat-view');
      return chatView && (chatView as HTMLElement).offsetParent !== null;
    },
    { timeout: TIMEOUTS.appInit },
  );
  const appScope = (await browserAppWindow.count()) > 0 ? browserAppWindow : page.locator('body');
  await expect(appScope.locator('.chat-view')).toBeVisible({ timeout: 5_000 });
}

// ─── Pinia state access ──────────────────────────────────────────────────────

/** Read a Pinia store's reactive state from the browser context. */
export async function getPiniaState(page: Page, storeName: string) {
  return page.evaluate((name) => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    if (!app) return null;
    const pinia = app.config.globalProperties.$pinia;
    if (!pinia) return null;
    return pinia.state.value[name] ?? null;
  }, storeName);
}

// ─── Chat helpers ────────────────────────────────────────────────────────────

/** Type a message and submit it through the real textarea keyboard path. */
export async function sendMessage(page: Page, text: string) {
  const browserWindow = page.locator('.browser-app-window');
  const inBrowserWindow = (await browserWindow.count()) > 0;
  const scope = inBrowserWindow ? browserWindow : page.locator('body');
  const input = scope.locator('.chat-input').first();
  const sendBtn = scope.locator('.send-btn').first();
  await input.fill(text);
  await expect(sendBtn).toBeEnabled({ timeout: 2_000 });
  await page.evaluate(() => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    const messages = app?.config.globalProperties.$pinia?.state.value?.conversation?.messages ?? [];
    (window as any).__tsE2ELastSend = {
      at: performance.now(),
      assistantCount: messages.filter((message: any) => message.role === 'assistant').length,
    };
  });
  await input.press('Enter');
  await expect(input).toHaveValue('', { timeout: 5_000 });
}

/** Open the chat history drawer if it's not already open. */
export async function openDrawer(page: Page) {
  if (await page.locator('.chatbox-messages').first().isVisible({ timeout: 500 }).catch(() => false)) {
    return;
  }

  const drawer = page.locator('.chat-history');

  // Click the toggle via raw DOM `el.click()` rather than Playwright's
  // actionable click. This is intentional: the input row may briefly be
  // overlapped by streaming overlays / quest hotseat tiles whose pointer
  // events confuse Playwright's click trial. Dispatching the click via JS
  // bypasses those checks and goes straight to Vue's `@click` handler.
  await expect(async () => {
    // If the element is not yet in the DOM, click the toggle to open it.
    // We use count() > 0 (not isVisible) because `.chat-history` is a
    // full-width block element: even during the enter-from transition its
    // width > 0, so Playwright's isVisible() returns true even at height = 0.
    // Note: locator.isAttached() does not exist in Playwright 1.59.x — use count().
    if ((await drawer.count()) === 0) {
      const toggle = page.getByRole('button', { name: 'Show chat' }).first();
      await expect(toggle).toBeVisible({ timeout: 2_000 });
      await toggle.evaluate((el) => (el as HTMLElement).click());
    }
    // Wait for the element to be mounted in the DOM …
    await expect(drawer).toBeAttached({ timeout: 2_000 });
    // … and for the Vue enter-transition to fully finish (all enter-* classes
    // removed). Returning while the animation is still running would cause
    // closeDrawer to operate on a half-open drawer, triggering a broken
    // interrupt → restart cycle. Bump from 1s → 4s because slow CI runners
    // (GitHub Actions ubuntu-latest) routinely take 1.5-2s to finish the
    // 350ms transition once layout-thrash from streaming is included.
    await expect(drawer).not.toHaveClass(/chat-panel-enter/, { timeout: 4_000 });
  }).toPass({ timeout: 15_000 });
}

/** Close the chat history drawer if it's open. */
export async function closeDrawer(page: Page) {
  const drawer = page.locator('.chat-history');

  // Guard: if element is already absent from the DOM, nothing to do.
  // locator.isAttached() does not exist in Playwright 1.59.x — use count().
  if ((await drawer.count()) === 0) return;

  // We use not.toBeAttached() (not not.toBeVisible()) for two reasons:
  //   1. Vue's `v-if` directive is what controls the drawer. After the leave
  //      transition completes the element is *removed from the DOM entirely*.
  //   2. `.chat-history` is a full-width block: its width is always > 0, so
  //      Playwright's isVisible() check (width > 0 || height > 0) reports
  //      "visible" even while max-height is 0 during the leave transition.
  //      not.toBeAttached() avoids this false-positive completely.
  await expect(async () => {
    if ((await drawer.count()) > 0) {
      const toggle = page.getByRole('button', { name: 'Hide chat' }).first();
      await expect(toggle).toBeVisible({ timeout: 2_000 });
      await toggle.evaluate((el) => (el as HTMLElement).click());
    }
    // Leave transition takes ≤ 350 ms; 3 s gives ample headroom on slow CI.
    await expect(drawer).not.toBeAttached({ timeout: 3_000 });
  }).toPass({ timeout: 15_000 });
}

/**
 * Start observing the subtitle overlay before triggering an LLM response.
 *
 * The subtitle is shown while streaming and hidden ~3s after TTS finishes.
 * In a non-Tauri browser context TTS may not run at all, so by the time
 * `waitForAssistantResponse` resolves the subtitle may already be gone.
 *
 * Call this *before* `sendMessage` and `await` the returned promise after
 * `waitForAssistantResponse` to capture the subtitle HTML at the moment it
 * first contained content. Returns `null` if the subtitle never appeared.
 */
export function captureSubtitleOnce(page: Page, timeout = 30_000): Promise<string | null> {
  return page
    .waitForFunction(
      () => {
        const el = document.querySelector('.subtitle-overlay');
        if (!el) return false;
        const text = (el.querySelector('.subtitle-text') as HTMLElement | null)?.innerHTML ?? '';
        return text.length > 0 ? text : false;
      },
      { timeout },
    )
    .then((handle) => handle.jsonValue() as Promise<string>)
    .catch(() => null);
}

type WaitForAssistantResponseOptions = {
  enforceLatencyBudget?: boolean;
  timeoutMs?: number;
};

/** Wait for the last assistant message to appear and return its content. */
export async function waitForAssistantResponse(
  page: Page,
  options: WaitForAssistantResponseOptions = {},
): Promise<string> {
  const waitStartedAt = Date.now();
  const timeoutMs = options.timeoutMs ?? TIMEOUTS.response;
  const marker = await page
    .evaluate(() => (window as any).__tsE2ELastSend ?? null)
    .catch(() => null);

  const firstVisible = await page
    .waitForFunction(
      (sendMarker) => {
        const app = (document.querySelector('#app') as any)?.__vue_app__;
        const conv = app?.config.globalProperties.$pinia?.state.value?.conversation;
        if (!conv) return false;
        const messages = conv.messages ?? [];
        const assistantCount = messages.filter((message: any) => message.role === 'assistant').length;
        const hasNewAssistant = typeof sendMarker?.assistantCount === 'number'
          ? assistantCount > sendMarker.assistantCount
          : messages[messages.length - 1]?.role === 'assistant';
        const streamingText = typeof conv.streamingText === 'string' ? conv.streamingText : '';
        if (streamingText.length === 0 && !hasNewAssistant) return false;
        return {
          elapsedMs: typeof sendMarker?.at === 'number' ? performance.now() - sendMarker.at : null,
        };
      },
      marker,
      { timeout: timeoutMs },
    )
    .then((handle) => handle.jsonValue() as Promise<{ elapsedMs: number | null }>)
    .catch(() => ({ elapsedMs: null }));

  const firstVisibleElapsedMs = firstVisible.elapsedMs ?? Date.now() - waitStartedAt;
  if (options.enforceLatencyBudget !== false && shouldEnforceLocalLatencyBudget()) {
    expect(firstVisibleElapsedMs, latencyFailureMessage(firstVisibleElapsedMs)).toBeLessThanOrEqual(
      LOCAL_E2E_RESPONSE_LATENCY_BUDGET_MS,
    );
  }

  await expect(async () => {
    const msgs = (await getPiniaState(page, 'conversation')) as any;
    const messages = msgs?.messages ?? [];
    if (typeof marker?.assistantCount === 'number') {
      const assistantCount = messages.filter((message: any) => message.role === 'assistant').length;
      expect(assistantCount).toBeGreaterThan(marker.assistantCount);
    }
    const lastMsg = messages[messages.length - 1];
    expect(lastMsg?.role).toBe('assistant');
    expect(lastMsg?.content?.length).toBeGreaterThan(0);
  }).toPass({ timeout: timeoutMs });

  const result = await page.evaluate((sendMarker) => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    const messages = app?.config.globalProperties.$pinia?.state.value?.conversation?.messages ?? [];
    const lastMsg = messages[messages.length - 1];
    return {
      content: lastMsg?.content ?? '',
      elapsedMs: typeof sendMarker?.at === 'number' ? performance.now() - sendMarker.at : null,
    };
  }, marker);

  return result.content;
}

/** Get the last assistant message object from pinia. */
export async function getLastAssistantMessage(page: Page) {
  const msgs = (await getPiniaState(page, 'conversation')) as any;
  const messages = msgs?.messages ?? [];
  for (let i = messages.length - 1; i >= 0; i--) {
    if (messages[i].role === 'assistant') return messages[i];
  }
  return null;
}

// ─── VRM / 3D helpers ────────────────────────────────────────────────────────

/** Wait for the VRM model to load (triangle count > 0 in debug overlay). */
export async function waitForModelLoaded(page: Page) {
  const browserWindow = page.locator('.browser-app-window');
  const inBrowserWindow = (await browserWindow.count()) > 0;
  const scope = inBrowserWindow ? browserWindow : page.locator('body');
  await expect(scope.locator('.splash')).toBeHidden({ timeout: 10_000 });

  const debugOverlay = scope.locator('.debug-overlay').first();
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
  }).toPass({ timeout: TIMEOUTS.vrmLoad });

  return debugOverlay;
}

// ─── Navigation helpers ──────────────────────────────────────────────────────

/** Switch to a tab by name (works for both desktop and mobile nav). */
export async function navigateToTab(page: Page, tabName: string) {
  // Try desktop nav first
  const desktopTab = page.locator('.nav-btn', { hasText: tabName }).first();
  if (await desktopTab.isVisible().catch(() => false)) {
    await desktopTab.click();
    if (tabName === 'Chat') {
      await stopGenerationIfPresent(page);
    }
    return;
  }
  // Fall back to mobile nav
  const mobileTab = page.locator('.mobile-tab', { hasText: tabName }).first();
  await mobileTab.click();
  if (tabName === 'Chat') {
    await stopGenerationIfPresent(page);
  }
}
