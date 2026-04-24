/**
 * Shared E2E test helpers for TerranSoul Playwright tests.
 *
 * All tests run against the Vite dev server (no Tauri backend).
 * The app auto-configures free API brain when Tauri is absent.
 */
import { expect, type Page } from '@playwright/test';

// ─── Timeouts ────────────────────────────────────────────────────────────────
export const TIMEOUTS = {
  message: 5_000,
  response: 30_000,
  panel: 3_000,
  vrmLoad: 30_000,
  appInit: 30_000,
} as const;

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

/** Wait for the Vue app to fully initialize (splash hidden, chat-view visible). */
export async function waitForAppReady(page: Page) {
  await page.waitForFunction(
    () => {
      const app = (document.querySelector('#app') as any)?.__vue_app__;
      if (!app) return false;
      const pinia = app.config.globalProperties.$pinia;
      if (!pinia) return false;
      const chatView = document.querySelector('.chat-view');
      return chatView && (chatView as HTMLElement).offsetParent !== null;
    },
    { timeout: TIMEOUTS.appInit },
  );
  await expect(page.locator('.chat-view')).toBeVisible({ timeout: 5_000 });
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

/** Type a message and click Send. */
export async function sendMessage(page: Page, text: string) {
  const input = page.locator('.chat-input');
  const sendBtn = page.locator('.send-btn');
  await input.fill(text);
  await expect(sendBtn).toBeEnabled({ timeout: 2_000 });
  await sendBtn.click();
}

/** Open the chat history drawer if it's not already open. */
export async function openDrawer(page: Page) {
  const drawer = page.locator('.chat-history');

  // Click the toggle via raw DOM `el.click()` rather than Playwright's
  // actionable click. This is intentional: the input row may briefly be
  // overlapped by streaming overlays / quest hotseat tiles whose pointer
  // events confuse Playwright's click trial. Dispatching the click via JS
  // bypasses those checks and goes straight to Vue's `@click` handler.
  await expect(async () => {
    // If the element is not yet in the DOM, click the toggle to open it.
    // We check attachment rather than visibility because `.chat-history` is a
    // full-width block element: even during the enter-from transition its
    // width > 0, so Playwright's isVisible() returns true even at height = 0.
    if (!(await drawer.isAttached().catch(() => false))) {
      const toggle = page.locator('.chat-drawer-toggle');
      await expect(toggle).toBeVisible({ timeout: 2_000 });
      await toggle.evaluate((el) => (el as HTMLElement).click());
    }
    // Wait for the element to be mounted in the DOM …
    await expect(drawer).toBeAttached({ timeout: 2_000 });
    // … and for the Vue enter-transition to fully finish (all enter-* classes
    // removed). Returning while the animation is still running would cause
    // closeDrawer to operate on a half-open drawer, triggering a broken
    // interrupt → restart cycle.
    await expect(drawer).not.toHaveClass(/chat-panel-enter/, { timeout: 1_000 });
  }).toPass({ timeout: 10_000 });
}

/** Close the chat history drawer if it's open. */
export async function closeDrawer(page: Page) {
  const drawer = page.locator('.chat-history');

  // Guard: if element is already absent from the DOM, nothing to do.
  if (!(await drawer.isAttached().catch(() => false))) return;

  // We use not.toBeAttached() (not not.toBeVisible()) for two reasons:
  //   1. Vue's `v-if` directive is what controls the drawer. After the leave
  //      transition completes the element is *removed from the DOM entirely*.
  //   2. `.chat-history` is a full-width block: its width is always > 0, so
  //      Playwright's isVisible() check (width > 0 || height > 0) reports
  //      "visible" even while max-height is 0 during the leave transition.
  //      not.toBeAttached() avoids this false-positive completely.
  await expect(async () => {
    if (await drawer.isAttached().catch(() => false)) {
      const toggle = page.locator('.chat-drawer-toggle');
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

/** Wait for the last assistant message to appear and return its content. */
export async function waitForAssistantResponse(page: Page): Promise<string> {
  await expect(async () => {
    const msgs = (await getPiniaState(page, 'conversation')) as any;
    const messages = msgs?.messages ?? [];
    const lastMsg = messages[messages.length - 1];
    expect(lastMsg?.role).toBe('assistant');
    expect(lastMsg?.content?.length).toBeGreaterThan(0);
  }).toPass({ timeout: TIMEOUTS.response });

  const msgs = (await getPiniaState(page, 'conversation')) as any;
  const messages = msgs?.messages ?? [];
  return messages[messages.length - 1]?.content ?? '';
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
    return;
  }
  // Fall back to mobile nav
  const mobileTab = page.locator('.mobile-tab', { hasText: tabName }).first();
  await mobileTab.click();
}
