/**
 * capture-new-flow-screenshots.mjs
 *
 * Lightweight Playwright capture for the three steps whose UI changed in
 * the new don't-know / provide-your-own-context flow:
 *
 *   04-alice-asks-law       — Alice asks a factual law question
 *   05-dont-know-prompt     — System offers Gemini upgrade or own-context
 *   06-provide-context      — scholar-quest suggestion after Alice's command
 *
 * Unlike verify-brain-flow.mjs, this script does NOT require Docker, Ollama
 * or Tauri. It runs against a plain Vite dev server (npm run dev) and
 * injects Pinia state directly to synthesise the conversation history, so
 * the screenshots reflect exactly the markup the real flow produces.
 *
 * Usage:
 *   npm run dev                             # in terminal 1
 *   node scripts/capture-new-flow-screenshots.mjs
 */
import { chromium } from 'playwright';
import { mkdirSync } from 'fs';

const VITE_URL = 'http://localhost:1420';
const OUT = 'instructions/screenshots';
mkdirSync(OUT, { recursive: true });

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 420, height: 700 } });
const page = await context.newPage();

await page.goto(VITE_URL, { waitUntil: 'networkidle', timeout: 30000 });
await page.waitForSelector('.chat-view', { state: 'visible', timeout: 15000 });

// Seed the brain store so the status pill renders instead of the overlay.
await page.evaluate(() => {
  const app = document.querySelector('#app')?.__vue_app__;
  const p = app?.config?.globalProperties?.$pinia;
  if (!p) return;
  p.state.value.brain.brainMode = { mode: 'local_ollama', model: 'gemma3:4b' };
  p.state.value.brain.ollamaStatus = { running: true, model_count: 1 };
  p.state.value.brain.hasBrain = true;
});

// Expand the chat drawer so the history is visible.
try {
  const toggle = page.locator('.chat-drawer-toggle');
  if (await toggle.isVisible({ timeout: 1000 }).catch(() => false)) {
    const expanded = await page.locator('.chat-history').isVisible().catch(() => false);
    if (!expanded) await toggle.click();
  }
} catch { /* ok */ }
await page.waitForTimeout(400);

/**
 * Replace conversation.messages with the given synthetic list.
 * Uses the same shape the conversation store itself produces.
 */
async function setMessages(msgs) {
  await page.evaluate((messages) => {
    const app = document.querySelector('#app')?.__vue_app__;
    const p = app?.config?.globalProperties?.$pinia;
    if (!p) return;
    p.state.value.conversation.messages = messages;
    p.state.value.conversation.isThinking = false;
    p.state.value.conversation.isStreaming = false;
    p.state.value.conversation.streamingText = '';
  }, msgs);
  await page.waitForTimeout(300);
}

const USER_QUESTION =
  'What is the statute of limitations for contract disputes under Vietnamese civil law?';

// ── Step 4 — Alice asks, LLM answers, NO auto-quest ───────────────────────
await setMessages([
  {
    id: 'u1', role: 'user', content: USER_QUESTION, timestamp: Date.now() - 20000,
  },
  {
    id: 'a1', role: 'assistant',
    content:
      `Vietnamese civil law sets the general statute of limitations for ` +
      `contract disputes at three years under Article 429 of the 2015 Civil Code, ` +
      `running from the date the claimant knew (or should have known) of the breach.`,
    agentName: 'TerranSoul',
    sentiment: 'neutral',
    timestamp: Date.now() - 19000,
  },
]);
await page.screenshot({ path: `${OUT}/04-alice-asks-law.png`, fullPage: false });
console.log('  ✅ 04-alice-asks-law.png');

// ── Step 5 — Don't-know branch pushes System prompt with two commands ─────
await setMessages([
  {
    id: 'u1', role: 'user', content: USER_QUESTION, timestamp: Date.now() - 30000,
  },
  {
    id: 'a1', role: 'assistant',
    content:
      `I don't have reliable information on the exact statute of limitations ` +
      `for Vietnamese contract disputes — my training data is limited here.`,
    agentName: 'TerranSoul',
    sentiment: 'sad',
    timestamp: Date.now() - 29000,
  },
  {
    id: 's1', role: 'assistant',
    content:
      `I don't have reliable information on that — my current model's knowledge is limited here.\n\n` +
      `Two ways forward:\n` +
      `• Type **"upgrade to Gemini model"** — I'll switch to Google Gemini with Search grounding (API key needed).\n` +
      `• Type **"provide your own context"** — I'll start a **📚 Scholar's Quest** so you can feed me URLs / files.`,
    agentName: 'System',
    sentiment: 'neutral',
    timestamp: Date.now() - 28000,
    questId: 'dont-know',
    questChoices: [
      { label: 'Upgrade to Gemini model', value: 'command:upgrade to Gemini model', icon: '🔮' },
      { label: 'Provide your own context', value: 'command:provide your own context', icon: '📚' },
      { label: 'Dismiss', value: 'dismiss', icon: '💤' },
    ],
  },
]);
await page.waitForTimeout(600);
await page.screenshot({ path: `${OUT}/05-dont-know-prompt.png`, fullPage: false });
console.log('  ✅ 05-dont-know-prompt.png');

// ── Step 6 — Alice types the gated command → scholar-quest suggestion ────
await setMessages([
  {
    id: 'u1', role: 'user', content: USER_QUESTION, timestamp: Date.now() - 40000,
  },
  {
    id: 'a1', role: 'assistant',
    content:
      `I don't have reliable information on the exact statute of limitations ` +
      `for Vietnamese contract disputes — my training data is limited here.`,
    agentName: 'TerranSoul',
    sentiment: 'sad',
    timestamp: Date.now() - 39000,
  },
  {
    id: 'u2', role: 'user', content: 'provide your own context', timestamp: Date.now() - 20000,
  },
  {
    id: 'a2', role: 'assistant',
    content:
      `📚 Let's set up your own context. I'll run a **Scholar's Quest** that walks you ` +
      `through adding URLs and files — I'll chunk, embed and store them in long-term memory ` +
      `so my next answers are grounded in your sources.`,
    agentName: 'TerranSoul',
    sentiment: 'happy',
    timestamp: Date.now() - 19000,
    questId: 'scholar-quest',
    questChoices: [
      { label: 'Start Knowledge Quest', value: 'knowledge-quest-start', icon: '⚔️' },
      { label: 'No thanks', value: 'dismiss', icon: '💤' },
    ],
  },
]);
await page.waitForTimeout(600);
await page.screenshot({ path: `${OUT}/06-provide-context.png`, fullPage: false });
console.log('  ✅ 06-provide-context.png');

// ── Step 7 — Hotseat overlay (shifted from old step 6) ───────────────────
// The scholar-quest message from Step 6 renders the hotseat strip directly;
// no extra state change is needed.  Wait a tick for the overlay to settle.
await page.waitForTimeout(400);
await page.screenshot({ path: `${OUT}/07-quest-choice-overlay.png`, fullPage: false });
console.log('  ✅ 07-quest-choice-overlay.png');

await browser.close();
console.log('\nDone — 3 fresh screenshots written to', OUT);
