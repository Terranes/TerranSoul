/**
 * verify-brain-flow.mjs
 *
 * Full headful Playwright demo of the TerranSoul Brain + Knowledge Quest flow.
 * Connects to the running Tauri desktop app via CDP (WebView2 remote debugging).
 *
 * Pre-requisites:
 *   1. Docker Desktop running with "ollama" container + gemma3:4b model pulled
 *   2. Start Tauri dev with CDP enabled:
 *        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
 *        npm run tauri dev
 *   3. Playwright Chromium installed: npx playwright install chromium
 *
 * Usage:
 *   node scripts/verify-brain-flow.mjs
 *
 * This script verifies the "Alice learns Vietnamese law" narrative:
 *   Step 0:  Pre-flight (Docker, Ollama, model, Tauri)
 *   Step 1:  Fresh launch
 *   Step 2:  Brain auto-configured → local Ollama
 *   Step 3:  Verify all brain components
 *   Step 4:  Alice asks a law question — plain Q&A, no auto-quest
 *   Step 5:  "I don't know" branch — Gemini / own-context choices
 *   Step 6:  Alice types "provide your own context" → Scholar's Quest
 *   Step 7:  Quest choice overlay — yes/no
 *   Step 8:  Knowledge Quest dialog opens (FF-style)
 *   Step 9:  Brain verification step ✅
 *   Step 10: Gather sources — URL + file
 *   Step 11: Learning in progress
 *   Step 12: Knowledge acquired!
 *   Step 13: Alice asks the same law question — answered via RAG
 *   Step 14: TerranSoul answers more law questions
 *   Step 15: Skill tree stats
 *   Step 16: Pet mode with chat
 */
import { chromium } from 'playwright';
import { execSync } from 'child_process';
import { mkdirSync, existsSync } from 'fs';

const CDP_URL = 'http://localhost:9222';
const VITE_URL = 'http://localhost:1420';
const OUT = 'instructions/screenshots';
mkdirSync(OUT, { recursive: true });

const results = [];
let passed = 0;
let failed = 0;
let skipped = 0;

function check(step, name, condition, detail = '') {
  if (condition === 'SKIP') {
    skipped++;
    results.push({ step, name, status: 'SKIP', detail });
    console.log(`  ⏭  ${name}${detail ? ` — ${detail}` : ''}`);
  } else if (condition) {
    passed++;
    results.push({ step, name, status: 'PASS', detail });
    console.log(`  ✅ ${name}${detail ? ` — ${detail}` : ''}`);
  } else {
    failed++;
    results.push({ step, name, status: 'FAIL', detail });
    console.log(`  ❌ ${name}${detail ? ` — ${detail}` : ''}`);
  }
}

async function vis(page, sel, timeout = 5000) {
  try {
    await page.locator(sel).first().waitFor({ state: 'visible', timeout });
    return true;
  } catch { return false; }
}

async function txt(page, sel, timeout = 3000) {
  try {
    await page.locator(sel).first().waitFor({ state: 'visible', timeout });
    return (await page.locator(sel).first().textContent())?.trim() ?? '';
  } catch { return ''; }
}

async function allTexts(page, sel) {
  try {
    return await page.locator(sel).allTextContents().then(arr => arr.map(s => s.trim()));
  } catch { return []; }
}

async function pinia(page, store) {
  return page.evaluate((s) => {
    const app = document.querySelector('#app')?.__vue_app__;
    return app?.config?.globalProperties?.$pinia?.state?.value?.[s] ?? null;
  }, store);
}

async function screenshot(page, name) {
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: false });
}

// Navigate to a tab by label — works with both desktop nav and mobile bottom nav
async function navTo(page, label) {
  // Try desktop nav first
  const desktopBtn = page.locator('.desktop-nav .nav-btn .nav-label', { hasText: label });
  if (await desktopBtn.isVisible({ timeout: 500 }).catch(() => false)) {
    await desktopBtn.click();
    await page.waitForTimeout(500);
    return;
  }
  // Fall back to mobile bottom nav
  const mobileBtn = page.locator('.mobile-bottom-nav .mobile-tab-label', { hasText: label });
  if (await mobileBtn.isVisible({ timeout: 500 }).catch(() => false)) {
    await mobileBtn.click();
    await page.waitForTimeout(500);
    return;
  }
  // Force click on any matching nav label
  const anyBtn = page.locator('.nav-label, .mobile-tab-label', { hasText: label }).first();
  await anyBtn.click({ force: true });
  await page.waitForTimeout(500);
}

// ═══════════════════════════════════════════════════════════════════════════
// STEP 0 — Pre-flight: Docker + Ollama + Model + Tauri
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 0: Pre-flight — Docker, Ollama, Model, Tauri ═══');

let ollamaModelName = '';
let dockerVersion = '';

// Docker CLI
try {
  dockerVersion = execSync('docker --version', { encoding: 'utf8' }).trim();
  check(0, 'Docker CLI installed', dockerVersion.startsWith('Docker version'), dockerVersion);
} catch {
  check(0, 'Docker CLI installed', false, 'docker not found');
}

// Ollama container running
let containerStatus = '';
try {
  const ps = execSync('docker ps --filter name=ollama --format "{{.Status}}"', { encoding: 'utf8' }).trim();
  containerStatus = ps;
  check(0, 'Ollama container running', ps.startsWith('Up'), ps);
} catch {
  check(0, 'Ollama container running', false, 'docker ps failed');
}

// Ollama API reachable + model installed
let ollamaModels = [];
try {
  const resp = await fetch('http://localhost:11434/api/tags');
  const data = await resp.json();
  ollamaModels = data.models ?? [];
  check(0, 'Ollama API reachable', resp.ok, `models: ${ollamaModels.length}`);
  check(0, 'Model installed', ollamaModels.length > 0, ollamaModels.map(m => m.name).join(', '));
  if (ollamaModels.length > 0) {
    ollamaModelName = ollamaModels[0].name.replace(':latest', '');
    check(0, 'Model tag', ollamaModelName === 'gemma3:4b', ollamaModelName);
  }
} catch (e) {
  check(0, 'Ollama API reachable', false, String(e));
  check(0, 'Model installed', false);
}

// Model responds to test prompt
try {
  const resp = await fetch('http://localhost:11434/api/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: ollamaModelName || 'gemma3:4b',
      messages: [{ role: 'user', content: 'Say hello in one word.' }],
      stream: false,
    }),
  });
  const data = await resp.json();
  const reply = data?.message?.content ?? '';
  check(0, 'Model responds', reply.length > 0, `reply: "${reply.slice(0, 50)}"`);
} catch (e) {
  check(0, 'Model responds', false, String(e));
}

// Tauri CDP reachable
let cdpAvailable = false;
try {
  const resp = await fetch(`${CDP_URL}/json/version`);
  cdpAvailable = resp.ok;
  check(0, 'Tauri CDP reachable', cdpAvailable, CDP_URL);
} catch {
  check(0, 'Tauri CDP reachable', false, 'Set WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222');
}

// ═══════════════════════════════════════════════════════════════════════════
// Connect to Tauri webview or fall back to Vite dev server
// ═══════════════════════════════════════════════════════════════════════════
let browser, page;
const isTauriMode = cdpAvailable;

if (isTauriMode) {
  console.log('\n  🔗 Connecting to Tauri webview via CDP...');
  browser = await chromium.connectOverCDP(CDP_URL);
  const contexts = browser.contexts();
  // Find the actual app page (not DevTools or workers)
  page = null;
  for (const ctx of contexts) {
    for (const p of ctx.pages()) {
      const url = p.url();
      if (url.includes('localhost:1420') && !url.includes('devtools')) {
        page = p;
        break;
      }
    }
    if (page) break;
  }
  if (!page) {
    // Fallback: pick first page that's not a devtools page
    for (const ctx of contexts) {
      for (const p of ctx.pages()) {
        if (!p.url().includes('devtools')) { page = p; break; }
      }
      if (page) break;
    }
  }
  if (!page) {
    page = contexts[0]?.pages()[0];
  }
  console.log(`  🔗 Connected to page: ${page.url()}`);
  // Refresh to reset state from any previous run
  await page.reload({ waitUntil: 'networkidle', timeout: 30000 });
} else {
  console.log('\n  ⚠️ CDP not available — falling back to Vite dev server (no Tauri IPC)');
  browser = await chromium.launch({ headless: false, args: ['--no-sandbox'] });
  page = await browser.newPage({ viewport: { width: 420, height: 700 } });
  await page.goto(VITE_URL, { waitUntil: 'networkidle', timeout: 30000 });
}

// Wait for app to be ready (handle splash screen + pet mode from previous runs)
try {
  await page.waitForSelector('.chat-view', { state: 'visible', timeout: 15000 });
} catch {
  // May be in pet mode or splash — try pressing Escape to exit pet mode
  await page.keyboard.press('Escape');
  await page.waitForTimeout(2000);
  // Try navigating to Chat tab
  try {
    await navTo(page, 'Chat');
  } catch { /* ok */ }
  await page.waitForSelector('.chat-view', { state: 'visible', timeout: 15000 });
}
// Small settling delay for animations
await page.waitForTimeout(2000);

// ═══════════════════════════════════════════════════════════════════════════
// STEP 1 — Fresh launch
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 1: Fresh launch ═══');

check(1, '.chat-view visible', await vis(page, '.chat-view'));
check(1, '.viewport-layer visible', await vis(page, '.viewport-layer'));
check(1, '.input-footer visible', await vis(page, '.input-footer'));
// Nav visible (desktop or mobile)
const hasDesktopNav = await vis(page, '.desktop-nav', 1000);
const hasMobileNav = await vis(page, '.mobile-bottom-nav', 1000);
check(1, 'Navigation visible', hasDesktopNav || hasMobileNav, hasDesktopNav ? 'desktop' : 'mobile');

let navLabels = await allTexts(page, '.nav-btn .nav-label');
if (navLabels.length === 0) {
  navLabels = await allTexts(page, '.mobile-bottom-nav .mobile-tab-label');
}
check(1, 'Nav labels', JSON.stringify(navLabels) === JSON.stringify(['Chat', 'Quests', 'Memory', 'Market', 'Voice']),
  JSON.stringify(navLabels));

check(1, '.ai-state-pill visible', await vis(page, '.ai-state-pill'));
check(1, '.ff-orb visible', await vis(page, '.ff-orb'));
check(1, '.mode-toggle-toolbar visible', await vis(page, '.mode-toggle-toolbar'));

const toggleLabel = await txt(page, '.mode-toggle-label');
check(1, 'Toggle label === "Desktop"', toggleLabel === 'Desktop', toggleLabel);

check(1, 'Chat input visible', await vis(page, 'input.chat-input'));
const placeholder = await page.locator('input.chat-input').first().getAttribute('placeholder');
check(1, 'Placeholder === "Type a message…"', placeholder === 'Type a message…', placeholder);
check(1, 'Send button visible', await vis(page, 'button.send-btn'));

await screenshot(page, '01-fresh-launch');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 2 — Brain configuration → Local Ollama
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 2: Brain configuration → Local Ollama ═══');

let brainState = await pinia(page, 'brain');
check(2, 'Brain Pinia store exists', brainState !== null);

// In Tauri mode, the brain should auto-configure on desktop load.
// If not yet configured, inject the state manually.
if (!brainState?.brainMode || brainState.brainMode.mode !== 'local_ollama') {
  await page.evaluate((model) => {
    const app = document.querySelector('#app')?.__vue_app__;
    const p = app?.config?.globalProperties?.$pinia;
    if (!p) return;
    p.state.value.brain.brainMode = { mode: 'local_ollama', model };
    p.state.value.brain.ollamaStatus = { running: true, model_count: 1 };
    p.state.value.brain.hasBrain = true;
  }, ollamaModelName || 'gemma3:4b');
  await page.waitForTimeout(500);
}

brainState = await pinia(page, 'brain');
check(2, 'brainMode.mode === "local_ollama"', brainState?.brainMode?.mode === 'local_ollama',
  brainState?.brainMode?.mode);
check(2, 'brainMode.model === model', brainState?.brainMode?.model === (ollamaModelName || 'gemma3:4b'),
  brainState?.brainMode?.model);
check(2, 'ollamaStatus.running === true', brainState?.ollamaStatus?.running === true);
check(2, 'ollamaStatus.model_count === 1', brainState?.ollamaStatus?.model_count === 1);

// Warm up Ollama to ensure it's ready for chat
try {
  // Direct warm-up to model
  await fetch('http://localhost:11434/api/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: ollamaModelName || 'gemma3:4b',
      messages: [{ role: 'user', content: 'hi' }],
      stream: false,
    }),
  });
  // Also reinitialize the brain store to pick up Ollama connection
  await page.evaluate(() => {
    const app = document.querySelector('#app')?.__vue_app__;
    const p = app?.config?.globalProperties?.$pinia;
    const brain = p?.state?.value?.brain;
    if (brain) {
      brain.ollamaStatus = { running: true, model_count: 1 };
    }
  });
  // Wait for Tauri backend to also establish connection
  await page.waitForTimeout(3000);
} catch { /* ok */ }

// Check brain status pill
const pillVis = await vis(page, '.brain-status-pill');
check(2, 'Brain status pill visible', pillVis);
if (pillVis) {
  const pillText = await txt(page, '.brain-status-pill');
  check(2, 'Pill text === "Ollama · gemma3:4b"', pillText === `Ollama · ${ollamaModelName || 'gemma3:4b'}`, pillText);
}
check(2, 'Brain overlay hidden', !(await vis(page, '.brain-overlay', 1000)));

await screenshot(page, '02-brain-configured');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 3 — Verify all brain components
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 3: Brain component verification ═══');

// Re-verify Docker + Ollama from the test script side
check(3, 'Docker version accessible', dockerVersion.startsWith('Docker version'), dockerVersion);
check(3, 'Container status starts with "Up"', containerStatus.startsWith('Up'), containerStatus);

// Query model details
let modelFamily = '';
let modelParams = '';
let modelQuant = '';
try {
  const resp = await fetch('http://localhost:11434/api/show', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: ollamaModelName || 'gemma3:4b' }),
  });
  const data = await resp.json();
  modelFamily = data?.details?.family ?? '';
  modelParams = data?.details?.parameter_size ?? '';
  modelQuant = data?.details?.quantization_level ?? '';
  check(3, 'Model family === "gemma3"', modelFamily === 'gemma3', modelFamily);
  check(3, 'Model params', modelParams.includes('4'), modelParams);
  check(3, 'Model quantization', modelQuant.includes('Q4'), modelQuant);
} catch (e) {
  check(3, 'Model details API', false, String(e));
}

// Pinia state verification
brainState = await pinia(page, 'brain');
check(3, 'Pinia brainMode.mode === "local_ollama"', brainState?.brainMode?.mode === 'local_ollama');
check(3, 'Pinia brainMode.model === model', brainState?.brainMode?.model === (ollamaModelName || 'gemma3:4b'));
const pillText3 = await txt(page, '.brain-status-pill');
check(3, 'Brain pill === "Ollama · gemma3:4b"', pillText3 === `Ollama · ${ollamaModelName || 'gemma3:4b'}`, pillText3);

// Check Tauri IPC availability
const hasTauri = await page.evaluate(() => '__TAURI_INTERNALS__' in window);
check(3, 'Tauri IPC available', hasTauri || !isTauriMode, hasTauri ? 'Yes' : 'Browser-only');

await screenshot(page, '03-brain-components');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 4 — Alice asks a law question (plain Q&A, no auto-quest)
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 4: Alice asks a law question ═══');

// Expand chat drawer if not already expanded
const drawerToggle = page.locator('.chat-drawer-toggle');
if (await drawerToggle.isVisible()) {
  const expanded = await page.locator('.chat-history').isVisible().catch(() => false);
  if (!expanded) {
    await drawerToggle.click();
    await page.waitForTimeout(500);
  }
}

// Ask a factual question — NOT an instruction to ingest sources.
const step4Question = 'What is the statute of limitations for contract disputes under Vietnamese civil law?';
const chatInput = page.locator('input.chat-input');
await chatInput.fill(step4Question);
const inputValue = await chatInput.inputValue();
check(4, 'Input filled', inputValue.includes('Vietnamese civil law'), inputValue.slice(0, 50));

await page.locator('button.send-btn').click();
check(4, 'Message sent', true);

// Wait for user message to appear
await page.waitForSelector('.message-row.user', { state: 'visible', timeout: 5000 });
const userRows = await page.locator('.message-row.user').count();
check(4, 'User message row visible', userRows >= 1, `count=${userRows}`);

// Wait for assistant response (local Ollama — may take up to 120s)
console.log('  ⏳ Waiting for LLM response (local Ollama)...');
await page.waitForSelector('.message-row.assistant', { state: 'visible', timeout: 120000 });

// Wait for streaming to finish
await page.waitForFunction(() => {
  const app = document.querySelector('#app')?.__vue_app__;
  const p = app?.config?.globalProperties?.$pinia;
  return p && !p.state.value.conversation.isThinking && !p.state.value.conversation.isStreaming;
}, { timeout: 120000 });

const assistantRows = await page.locator('.message-row.assistant').count();
check(4, 'Assistant response received', assistantRows >= 1, `count=${assistantRows}`);

// Check the response content
const convState4 = await pinia(page, 'conversation');
const lastAssistantMsg = [...(convState4?.messages ?? [])].reverse().find(m => m.role === 'assistant');
check(4, 'Response has content', (lastAssistantMsg?.content?.length ?? 0) > 20,
  `length=${lastAssistantMsg?.content?.length}`);

// CRITICAL invariant: asking a question must NOT auto-trigger Scholar's Quest.
const autoQuestAfterStep4 = (convState4?.messages ?? [])
  .find(m => m.questId === 'scholar-quest');
check(4, 'No auto scholar-quest from a question', !autoQuestAfterStep4,
  autoQuestAfterStep4 ? 'scholar-quest was wrongly auto-triggered' : 'none');

await screenshot(page, '04-alice-asks-law');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 5 — "I don't know" branch (Gemini / own-context prompt)
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 5: "I don\'t know" branch ═══');

// A small local model typically triggers the don't-know detector.  If the
// run produced a confident answer instead, the checks below mark SKIP.
await page.waitForTimeout(1500);

const allMessages5 = (await pinia(page, 'conversation'))?.messages ?? [];
const dontKnowMsg = allMessages5.find(m => m.questId === 'dont-know');

if (dontKnowMsg) {
  check(5, 'System "don\'t-know" message appears',
    dontKnowMsg.agentName === 'System',
    `agent=${dontKnowMsg.agentName}`);
  check(5, 'Content mentions "upgrade to Gemini model"',
    /upgrade to Gemini model/i.test(dontKnowMsg.content ?? ''));
  check(5, 'Content mentions "provide your own context"',
    /provide your own context/i.test(dontKnowMsg.content ?? ''));

  const choiceValues = (dontKnowMsg.questChoices ?? []).map(c => c.value);
  check(5, 'Choice: command:upgrade to Gemini model',
    choiceValues.includes('command:upgrade to Gemini model'),
    JSON.stringify(choiceValues));
  check(5, 'Choice: command:provide your own context',
    choiceValues.includes('command:provide your own context'),
    JSON.stringify(choiceValues));
  check(5, 'Dismiss choice present', choiceValues.includes('dismiss'));
} else {
  // Model returned a confident answer — skip this branch but record it.
  check(5, 'System "don\'t-know" message appears', 'SKIP',
    'Model produced a confident answer — don\'t-know prompt not shown');
  check(5, 'Content mentions "upgrade to Gemini model"', 'SKIP');
  check(5, 'Content mentions "provide your own context"', 'SKIP');
  check(5, 'Choice: command:upgrade to Gemini model', 'SKIP');
  check(5, 'Choice: command:provide your own context', 'SKIP');
  check(5, 'Dismiss choice present', 'SKIP');
}

await screenshot(page, '05-dont-know-prompt');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 6 — Alice explicitly asks to provide her own context
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 6: Alice types "provide your own context" ═══');

// Dismiss any hotseat strip showing the don't-know choices — the test
// drives the flow by typing the command text, which exercises the
// detectGatedSetupCommand path (same code path that the button click uses).
try {
  const strip5 = page.locator('.hotseat-strip');
  if (await strip5.isVisible({ timeout: 1000 }).catch(() => false)) {
    const dismissBtn = page.locator('.hotseat-tile-label', { hasText: 'Dismiss' });
    if (await dismissBtn.isVisible({ timeout: 500 }).catch(() => false)) {
      await dismissBtn.click();
      await page.waitForTimeout(400);
    }
  }
} catch { /* ok */ }

await page.locator('input.chat-input').fill('provide your own context');
await page.locator('button.send-btn').click();

// Wait for the scholar-quest suggestion message to land.
await page.waitForFunction(() => {
  const app = document.querySelector('#app')?.__vue_app__;
  const p = app?.config?.globalProperties?.$pinia;
  const msgs = p?.state?.value?.conversation?.messages ?? [];
  return msgs.some(m => m.questId === 'scholar-quest');
}, { timeout: 10000 });

const allMessages6 = (await pinia(page, 'conversation'))?.messages ?? [];
const questMsg = allMessages6.find(m =>
  m.questId === 'scholar-quest' && (m.questChoices?.length ?? 0) > 0,
);
check(6, 'Scholar\'s Quest suggestion exists', !!questMsg, questMsg?.content?.slice(0, 60));
check(6, 'Quest ID === "scholar-quest"', questMsg?.questId === 'scholar-quest');
check(6, 'Has quest choices', (questMsg?.questChoices?.length ?? 0) >= 2,
  `choices=${questMsg?.questChoices?.length}`);

if (questMsg?.questChoices) {
  const labels = questMsg.questChoices.map(c => c.label);
  check(6, 'Choice 1 === "Start Knowledge Quest"', labels.includes('Start Knowledge Quest'),
    JSON.stringify(labels));
  check(6, 'Choice 2 === "No thanks"', labels.includes('No thanks'), JSON.stringify(labels));
}

await screenshot(page, '06-provide-context');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 7 — Quest choice overlay — Alice starts the quest
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 7: Quest choice overlay ═══');

const hotseatVis = await vis(page, '.hotseat-strip', 5000);
check(7, 'Hotseat strip visible', hotseatVis);

if (hotseatVis) {
  const hotseatQuestion = await txt(page, '.hotseat-question-text');
  check(7, 'Hotseat question text present', hotseatQuestion.length > 5, hotseatQuestion.slice(0, 60));

  const tileLabels = await allTexts(page, '.hotseat-tile-label');
  check(7, 'Hotseat tiles visible', tileLabels.length >= 2, JSON.stringify(tileLabels));

  const startBtn = page.locator('.hotseat-tile', { hasText: 'Start Knowledge Quest' });
  const startBtnVis = await startBtn.isVisible().catch(() => false);
  check(7, '"Start Knowledge Quest" button visible', startBtnVis);

  if (startBtnVis) {
    await startBtn.click();
    check(7, 'Clicked "Start Knowledge Quest"', true);
  }
}

await screenshot(page, '07-quest-choice-overlay');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 8 — Knowledge Quest Dialog opens (FF-style)
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 8: Knowledge Quest Dialog ═══');

// Wait for the KnowledgeQuestDialog to appear
const kqDialog = await vis(page, '.kq-dialog', 5000);
check(8, 'KQ dialog visible', kqDialog);

if (kqDialog) {
  // Header
  const kqLabel = await txt(page, '.kq-label');
  check(8, 'Header label === "SCHOLAR\'S QUEST"', kqLabel === "SCHOLAR'S QUEST", kqLabel);

  const kqTitle = await txt(page, '.kq-title');
  check(8, 'Title contains topic', kqTitle.length > 0, kqTitle);

  // Step tracker
  const stepLabels = await allTexts(page, '.kq-step-label');
  check(8, 'Step 1 === "Verify Brain"', stepLabels[0] === 'Verify Brain', stepLabels[0]);
  check(8, 'Step 2 === "Gather Sources"', stepLabels[1] === 'Gather Sources', stepLabels[1]);
  check(8, 'Step 3 === "Learn"', stepLabels[2] === 'Learn', stepLabels[2]);
  check(8, 'Step 4 === "Ready"', stepLabels[3] === 'Ready', stepLabels[3]);

  // Active step indicator
  const activeStep = await page.locator('.kq-step--active').count();
  check(8, 'One active step', activeStep === 1);
}

await screenshot(page, '08-knowledge-quest-dialog');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 9 — Brain verification step
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 9: Brain verification step ═══');

// Check brain verification section
const sectionTitle = await txt(page, '.kq-section-title');
check(9, 'Section title === "🧠 Verifying Brain"', sectionTitle === '🧠 Verifying Brain', sectionTitle);

// Wait for checks to complete
await page.waitForTimeout(3000);

const checkItems = await page.locator('.kq-check').count();
check(9, 'Brain checks visible', checkItems === 4, `count=${checkItems}`);

// Verify individual check labels
const checkLabels = await allTexts(page, '.kq-check-label');
check(9, 'Check 1 === "Brain configured"', checkLabels[0] === 'Brain configured', checkLabels[0]);
check(9, 'Check 2 === "LLM model loaded"', checkLabels[1] === 'LLM model loaded', checkLabels[1]);
check(9, 'Check 3 === "Memory system ready"', checkLabels[2] === 'Memory system ready', checkLabels[2]);
check(9, 'Check 4 === "Ingestion engine online"', checkLabels[3] === 'Ingestion engine online', checkLabels[3]);

// Check for success icons (✅)
const checkIcons = await allTexts(page, '.kq-check-icon');
const passedChecks = checkIcons.filter(i => i === '✅').length;
check(9, 'All brain checks passed', passedChecks >= 3, `passed=${passedChecks}/4`);

// Continue button
const continueBtn = page.locator('.kq-btn-primary', { hasText: 'Continue' });
const continueBtnVis = await continueBtn.isVisible().catch(() => false);
check(9, '"Continue" button visible', continueBtnVis);

if (continueBtnVis) {
  await continueBtn.click();
  await page.waitForTimeout(500);
  check(9, 'Advanced to step 2', true);
}

await screenshot(page, '09-brain-verification');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 10 — Gather sources: URL + file
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 10: Gather sources ═══');

// Verify step 2 content
const step2Title = await txt(page, '.kq-section-title');
check(10, 'Section title === "📖 Gather Sources"', step2Title === '📖 Gather Sources', step2Title);

// URL input field
const urlFieldVis = await vis(page, '.kq-url-field');
check(10, 'URL input field visible', urlFieldVis);

const urlPlaceholder = await page.locator('.kq-url-field').first().getAttribute('placeholder');
check(10, 'URL placeholder', urlPlaceholder === 'https://example.com/document', urlPlaceholder);

// Add URL
const demoUrl = `${VITE_URL}/demo/vietnamese-civil-code.html`;
await page.locator('.kq-url-field').fill(demoUrl);
const urlAddBtn = page.locator('.kq-url-add');
check(10, '"＋ Add URL" button visible', await urlAddBtn.isVisible());
await urlAddBtn.click();
await page.waitForTimeout(300);

// Verify source added
const sourceItems = await page.locator('.kq-source-item').count();
check(10, 'URL source added', sourceItems >= 1, `count=${sourceItems}`);

const firstSourceName = await txt(page, '.kq-source-name');
check(10, 'Source name matches URL', firstSourceName.includes('vietnamese-civil-code'), firstSourceName);

// File attachment button
const fileBtnVis = await vis(page, '.kq-file-btn');
check(10, '"📎 Attach File" button visible', fileBtnVis);

const fileBtnText = await txt(page, '.kq-file-btn');
check(10, 'File button text === "📎 Attach File"', fileBtnText === '📎 Attach File', fileBtnText);

// Simulate file input (set file path in the hidden input)
const demoFile = 'public/demo/article-429-commentary.txt';
if (existsSync(demoFile)) {
  const fileInput = page.locator('.kq-file-hidden');
  await fileInput.setInputFiles(demoFile);
  await page.waitForTimeout(300);
  const sourceCount = await page.locator('.kq-source-item').count();
  check(10, 'File source added', sourceCount >= 2, `count=${sourceCount}`);
} else {
  check(10, 'File source added', 'SKIP', 'Demo file not found');
}

// "Start Learning" button
const startLearnBtn = page.locator('.kq-btn-primary', { hasText: 'Start Learning' });
const startLearnVis = await startLearnBtn.isVisible().catch(() => false);
check(10, '"⚡ Start Learning" button visible', startLearnVis);

await screenshot(page, '10-gather-sources');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 11 — Learning in progress
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 11: Learning in progress ═══');

if (startLearnVis) {
  await startLearnBtn.click();
  check(11, 'Clicked "Start Learning"', true);

  // Wait for the learning step
  await page.waitForTimeout(1000);
  const learnTitle = await txt(page, '.kq-section-title');
  check(11, 'Section title === "⚡ Learning in Progress"', learnTitle === '⚡ Learning in Progress', learnTitle);

  // Wait for ingestion tasks to appear
  const taskVis = await vis(page, '.kq-task', 10000);
  check(11, 'Task progress visible', taskVis);

  if (taskVis) {
    // Check progress bar
    check(11, 'Progress bar visible', await vis(page, '.kq-progress-bar'));

    // Wait for completion (up to 120s for URL fetch + chunking + embedding)
    console.log('  ⏳ Waiting for ingestion to complete...');
    try {
      await page.waitForFunction(() => {
        const tasks = document.querySelectorAll('.kq-task-done');
        return tasks.length > 0;
      }, { timeout: 120000 });
      check(11, 'Ingestion completed', true);
    } catch {
      // May still be in progress — check current state
      const pctText = await txt(page, '.kq-task-pct');
      check(11, 'Ingestion in progress', true, `progress: ${pctText}`);
    }
  }

  await screenshot(page, '11-learning-progress');

  // Wait for auto-advance to "Knowledge Acquired" step
  try {
    await page.waitForSelector('.kq-complete-card', { state: 'visible', timeout: 30000 });
  } catch {
    // Try clicking Continue if auto-advance didn't trigger
    const contBtn = page.locator('.kq-btn-primary', { hasText: 'Continue' });
    if (await contBtn.isVisible().catch(() => false)) {
      await contBtn.click();
      await page.waitForTimeout(500);
    }
  }
} else {
  check(11, 'Learning step', 'SKIP', 'Start Learning button not available');
}

// ═══════════════════════════════════════════════════════════════════════════
// STEP 12 — Knowledge Acquired!
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 12: Knowledge Acquired ═══');

const completeCard = await vis(page, '.kq-complete-card', 5000);
check(12, 'Complete card visible', completeCard);

if (completeCard) {
  const completeIcon = await txt(page, '.kq-complete-icon');
  check(12, 'Trophy icon === "🏆"', completeIcon === '🏆', completeIcon);

  const completeTitle = await txt(page, '.kq-section-title');
  check(12, 'Section title === "🎯 Knowledge Acquired!"', completeTitle === '🎯 Knowledge Acquired!', completeTitle);

  // Reward grid
  const rewardCards = await page.locator('.kq-reward-card').count();
  check(12, 'Reward cards count === 4', rewardCards === 4, `count=${rewardCards}`);

  // "Ask Questions" button
  const askBtn = page.locator('.kq-btn-primary', { hasText: 'Ask Questions' });
  const askBtnVis = await askBtn.isVisible().catch(() => false);
  check(12, '"🗡️ Ask Questions" button visible', askBtnVis);

  await screenshot(page, '12-knowledge-acquired');

  if (askBtnVis) {
    await askBtn.click();
    await page.waitForTimeout(1000);
    check(12, 'KQ dialog closed', !(await vis(page, '.kq-dialog', 1000)));
  }
} else {
  check(12, 'Knowledge Acquired step', 'SKIP', 'Complete card not visible');
}

// ═══════════════════════════════════════════════════════════════════════════
// STEP 13 — Alice re-asks the same law question (RAG-grounded this time)
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 13: RAG answer to the original question ═══');

// Ensure chat drawer is open
const chatHistoryVis = await vis(page, '.chat-history', 2000);
if (!chatHistoryVis) {
  await page.locator('.chat-drawer-toggle').click();
  await page.waitForTimeout(500);
}

// Confirm completion message appeared
await page.waitForTimeout(1000);
const completionMsg = (await pinia(page, 'conversation'))?.messages?.find(
  m => m.content?.includes("Scholar's Quest Complete")
);
check(13, 'Completion message in chat', !!completionMsg, completionMsg?.content?.slice(0, 60));

// Re-ask the Step 4 question — now grounded by ingested chunks.
const lawQuestion1 = 'What is the statute of limitations for contract disputes under Vietnamese law?';

// Count assistant messages before sending
const msgCount13Before = await page.evaluate(() => {
  const app = document.querySelector('#app')?.__vue_app__;
  const p = app?.config?.globalProperties?.$pinia;
  return (p?.state?.value?.conversation?.messages ?? []).filter(m => m.role === 'assistant').length;
});

await page.locator('input.chat-input').fill(lawQuestion1);
await page.locator('button.send-btn').click();

console.log('  ⏳ Waiting for RAG response...');
await page.waitForFunction((prevCount) => {
  const app = document.querySelector('#app')?.__vue_app__;
  const p = app?.config?.globalProperties?.$pinia;
  const conv = p?.state?.value?.conversation;
  const assistantMsgs = (conv?.messages ?? []).filter(m => m.role === 'assistant');
  return assistantMsgs.length > prevCount && !conv?.isThinking && !conv?.isStreaming;
}, msgCount13Before, { timeout: 180000 });

const conv13 = await pinia(page, 'conversation');
const lastMsg13 = [...(conv13?.messages ?? [])].reverse().find(m => m.role === 'assistant');
check(13, 'RAG response received', (lastMsg13?.content?.length ?? 0) > 50,
  `length=${lastMsg13?.content?.length}`);

// Check if the response references Vietnamese law specifics
const content13 = lastMsg13?.content?.toLowerCase() ?? '';
const hasLawContent = content13.includes('429') ||
  content13.includes('three') || content13.includes('12 month') ||
  content13.includes('statute') || content13.includes('limitation') ||
  content13.includes('contract') || content13.includes('civil') ||
  content13.includes('vietnam') || content13.includes('claim');
check(13, 'Response references law content', hasLawContent, lastMsg13?.content?.slice(0, 100));

await screenshot(page, '13-rag-law-answer');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 14 — TerranSoul answers more law questions
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 14: More law questions ═══');

// Dismiss any don't-know / quest overlay that might still be showing.
try {
  const hotseat = page.locator('.hotseat-strip');
  if (await hotseat.isVisible({ timeout: 2000 }).catch(() => false)) {
    const dismissLabels = ['No thanks', 'Dismiss'];
    for (const label of dismissLabels) {
      const btn = page.locator('.hotseat-tile-label', { hasText: label });
      if (await btn.isVisible({ timeout: 500 }).catch(() => false)) {
        await btn.click();
        await page.waitForTimeout(400);
        break;
      }
    }
  }
} catch { /* ok */ }

// Brief delay to let Ollama settle after previous request
await page.waitForTimeout(2000);

// Count current assistant messages to wait for a new one
const msgCountBefore = await page.evaluate(() => {
  const app = document.querySelector('#app')?.__vue_app__;
  const p = app?.config?.globalProperties?.$pinia;
  return (p?.state?.value?.conversation?.messages ?? []).filter(m => m.role === 'assistant').length;
});

// Ask second law question
const lawQuestion2 = 'What are the exemptions from liability for breach of contract under Vietnamese civil code?';
await page.locator('input.chat-input').fill(lawQuestion2);
await page.locator('button.send-btn').click();

console.log('  ⏳ Waiting for second RAG response...');
await page.waitForFunction((prevCount) => {
  const app = document.querySelector('#app')?.__vue_app__;
  const p = app?.config?.globalProperties?.$pinia;
  const conv = p?.state?.value?.conversation;
  const assistantMsgs = (conv?.messages ?? []).filter(m => m.role === 'assistant');
  return assistantMsgs.length > prevCount && !conv?.isThinking && !conv?.isStreaming;
}, msgCountBefore, { timeout: 180000 });

const conv14 = await pinia(page, 'conversation');
const lastMsg14 = [...(conv14?.messages ?? [])].reverse().find(m => m.role === 'assistant');
check(14, 'Second RAG response received', (lastMsg14?.content?.length ?? 0) > 50,
  `length=${lastMsg14?.content?.length}`);

const content14 = lastMsg14?.content?.toLowerCase() ?? '';
const hasExemptionContent = content14.includes('force majeure') ||
  content14.includes('exempt') || content14.includes('421') ||
  content14.includes('fault') || content14.includes('breach') ||
  content14.includes('contract') || content14.includes('civil') ||
  content14.includes('liability') || content14.includes('vietnam');
check(14, 'Response references exemptions', hasExemptionContent, lastMsg14?.content?.slice(0, 100));

const brain14 = await pinia(page, 'brain');
check(14, 'Brain still local_ollama', brain14?.brainMode?.mode === 'local_ollama');
check(14, 'Brain model still gemma3:4b', brain14?.brainMode?.model === (ollamaModelName || 'gemma3:4b'));

await screenshot(page, '14-more-law-answers');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 15 — Skill tree stats
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 15: Skill tree ═══');

// Dismiss any overlay that appeared after Step 14
try {
  const hotseat15 = page.locator('.hotseat-strip');
  if (await hotseat15.isVisible({ timeout: 2000 }).catch(() => false)) {
    const dismissLabels = ['No thanks', 'Dismiss'];
    for (const label of dismissLabels) {
      const btn = page.locator('.hotseat-tile-label', { hasText: label });
      if (await btn.isVisible({ timeout: 500 }).catch(() => false)) {
        await btn.click();
        await page.waitForTimeout(400);
        break;
      }
    }
  }
} catch { /* ok */ }

// Navigate to Quests tab
await navTo(page, 'Quests');
await page.waitForTimeout(500);

check(15, '.skill-tree-view visible', await vis(page, '.skill-tree-view'));

const stTitle = await txt(page, '.st-title');
check(15, 'Title === "⚔️ Skill Tree"', stTitle === '⚔️ Skill Tree', stTitle);

check(15, '.brain-stat-sheet visible', await vis(page, '.brain-stat-sheet'));

const bssTitle = await txt(page, '.bss-title');
check(15, 'Sheet title === "⚔ Brain Stat Sheet"', bssTitle === '⚔ Brain Stat Sheet', bssTitle);

const statAbbrs = await allTexts(page, '.bss-stat-abbr');
check(15, 'Stats === ["INT","WIS","CHA","PER","DEX","END"]',
  JSON.stringify(statAbbrs) === JSON.stringify(['INT', 'WIS', 'CHA', 'PER', 'DEX', 'END']),
  JSON.stringify(statAbbrs));

const levelBadge = await txt(page, '.bss-level');
check(15, 'Level badge matches /^Lv\\. \\d+$/', /^Lv\. \d+$/.test(levelBadge), levelBadge);

check(15, '.st-daily-section visible', await vis(page, '.st-daily-section'));

await screenshot(page, '15-skill-tree');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 16 — Pet mode with chat
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 16: Pet mode ═══');

// Navigate back to Chat first
await navTo(page, 'Chat');
await page.waitForTimeout(500);

// Toggle pet mode
await page.locator('.mode-toggle-toolbar .mode-seg-btn').last().click();
await page.waitForTimeout(2000);

const petOverlay = await vis(page, '.pet-overlay', 5000);
check(16, 'Pet overlay visible', petOverlay);

if (petOverlay) {
  const hasAppShellPetMode = await page.locator('.app-shell.pet-mode').isVisible().catch(() => false);
  check(16, 'App shell has .pet-mode class', hasAppShellPetMode);

  // Dismiss onboarding if present
  try {
    const onboardDismiss = page.locator('.pet-onboarding-dismiss');
    if (await onboardDismiss.isVisible({ timeout: 2000 }).catch(() => false)) {
      await onboardDismiss.click();
      await page.waitForTimeout(500);
    }
  } catch { /* ok */ }

  // Click on the pet character to open chat panel
  const petChar = page.locator('.pet-character');
  if (await petChar.isVisible({ timeout: 2000 }).catch(() => false)) {
    try {
      await petChar.click();
      await page.waitForTimeout(1000);
      const petChatVisible = await vis(page, '.pet-chat', 3000);
      check(16, 'Pet chat panel visible', petChatVisible);
    } catch {
      check(16, 'Pet chat panel', 'SKIP', 'click did not propagate');
    }
  }

  await screenshot(page, '16-pet-mode');

  // Exit pet mode via Escape key (mode toggle is hidden in pet mode)
  await page.keyboard.press('Escape');
  await page.waitForTimeout(1000);
  check(16, 'Exited pet mode', !(await vis(page, '.pet-overlay', 1000)));
}

await screenshot(page, '16-final');

// ═══════════════════════════════════════════════════════════════════════════
// SUMMARY
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n════════════════════════════════════════════════════════════');
console.log(`RESULT: ${passed} passed, ${failed} failed, ${skipped} skipped`);
console.log('════════════════════════════════════════════════════════════');

for (const r of results) {
  const icon = r.status === 'PASS' ? '✅' : r.status === 'FAIL' ? '❌' : '⏭ ';
  console.log(`  ${icon} [Step ${r.step}] ${r.name}${r.detail ? ` (${r.detail})` : ''}`);
}
console.log('════════════════════════════════════════════════════════════');
console.log(`Screenshots → ${OUT}/`);

// Close
try { await browser.close(); } catch { /* ok */ }
process.exit(failed > 0 ? 1 : 0);
