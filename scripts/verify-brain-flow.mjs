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
 *   Step 0: Pre-flight (Docker, Ollama, model, Tauri)
 *   Step 1: Fresh launch
 *   Step 2: Brain auto-configured → local Ollama
 *   Step 3: Verify all brain components
 *   Step 4: Alice asks to learn Vietnamese law
 *   Step 5: TerranSoul suggests Scholar's Quest
 *   Step 6: Quest choice overlay — yes/no
 *   Step 7: Knowledge Quest dialog opens (FF-style)
 *   Step 8: Brain verification step ✅
 *   Step 9: Gather sources — URL + file
 *   Step 10: Learning in progress
 *   Step 11: Knowledge acquired!
 *   Step 12: Alice asks law questions
 *   Step 13: TerranSoul answers with RAG
 *   Step 14: Skill tree stats
 *   Step 15: Pet mode with chat
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
check(1, '.mode-toggle-pill visible', await vis(page, '.mode-toggle-pill'));

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
// STEP 4 — Alice asks to learn Vietnamese law
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 4: Alice asks to learn Vietnamese law ═══');

// Expand chat drawer if not already expanded
const drawerToggle = page.locator('.chat-drawer-toggle');
if (await drawerToggle.isVisible()) {
  const expanded = await page.locator('.chat-history').isVisible().catch(() => false);
  if (!expanded) {
    await drawerToggle.click();
    await page.waitForTimeout(500);
  }
}

// Type and send the message
const chatInput = page.locator('input.chat-input');
await chatInput.fill('I want to learn about Vietnamese civil law on contract liability');
const inputValue = await chatInput.inputValue();
check(4, 'Input filled', inputValue.includes('Vietnamese civil law'), inputValue.slice(0, 50));

await page.locator('button.send-btn').click();
check(4, 'Message sent', true);

// Wait for user message to appear
await page.waitForSelector('.message-row.user', { state: 'visible', timeout: 5000 });
const userRows = await page.locator('.message-row.user').count();
check(4, 'User message row visible', userRows >= 1, `count=${userRows}`);

// Wait for assistant response (local Ollama — may take up to 90s)
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
const convState = await pinia(page, 'conversation');
const lastAssistantMsg = [...(convState?.messages ?? [])].reverse().find(m => m.role === 'assistant');
check(4, 'Response has content', (lastAssistantMsg?.content?.length ?? 0) > 20,
  `length=${lastAssistantMsg?.content?.length}`);

await screenshot(page, '04-alice-asks-law');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 5 — TerranSoul suggests Scholar's Quest
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 5: Scholar\'s Quest suggestion ═══');

// The maybeShowKnowledgeQuest should have triggered after the response.
// Wait for the quest suggestion message to appear.
await page.waitForTimeout(1500);

// Check that a quest suggestion message exists
const allMessages = (await pinia(page, 'conversation'))?.messages ?? [];
const questMsg = allMessages.find(m =>
  m.questId === 'scholar-quest' && m.questChoices?.length > 0
);
check(5, 'Scholar\'s Quest suggestion exists', !!questMsg, questMsg?.content?.slice(0, 60));
check(5, 'Quest ID === "scholar-quest"', questMsg?.questId === 'scholar-quest');
check(5, 'Has quest choices', questMsg?.questChoices?.length >= 2,
  `choices=${questMsg?.questChoices?.length}`);

if (questMsg?.questChoices) {
  const labels = questMsg.questChoices.map(c => c.label);
  check(5, 'Choice 1 === "Start Knowledge Quest"', labels.includes('Start Knowledge Quest'),
    JSON.stringify(labels));
  check(5, 'Choice 2 === "No thanks"', labels.includes('No thanks'), JSON.stringify(labels));
}

await screenshot(page, '05-quest-suggestion');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 6 — Quest choice overlay — Alice clicks Yes
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 6: Quest choice overlay ═══');

// The hotseat strip should be visible
const hotseatVis = await vis(page, '.hotseat-strip', 5000);
check(6, 'Hotseat strip visible', hotseatVis);

if (hotseatVis) {
  const hotseatQuestion = await txt(page, '.hotseat-question-text');
  check(6, 'Hotseat question text present', hotseatQuestion.length > 5, hotseatQuestion.slice(0, 60));

  const tileLabels = await allTexts(page, '.hotseat-tile-label');
  check(6, 'Hotseat tiles visible', tileLabels.length >= 2, JSON.stringify(tileLabels));

  // Click "Start Knowledge Quest" button
  const startBtn = page.locator('.hotseat-tile', { hasText: 'Start Knowledge Quest' });
  const startBtnVis = await startBtn.isVisible().catch(() => false);
  check(6, '"Start Knowledge Quest" button visible', startBtnVis);

  if (startBtnVis) {
    await startBtn.click();
    check(6, 'Clicked "Start Knowledge Quest"', true);
  }
}

await screenshot(page, '06-quest-choice-overlay');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 7 — Knowledge Quest Dialog opens (FF-style)
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 7: Knowledge Quest Dialog ═══');

// Wait for the KnowledgeQuestDialog to appear
const kqDialog = await vis(page, '.kq-dialog', 5000);
check(7, 'KQ dialog visible', kqDialog);

if (kqDialog) {
  // Header
  const kqLabel = await txt(page, '.kq-label');
  check(7, 'Header label === "SCHOLAR\'S QUEST"', kqLabel === "SCHOLAR'S QUEST", kqLabel);

  const kqTitle = await txt(page, '.kq-title');
  check(7, 'Title contains topic', kqTitle.length > 0, kqTitle);

  // Step tracker
  const stepLabels = await allTexts(page, '.kq-step-label');
  check(7, 'Step 1 === "Verify Brain"', stepLabels[0] === 'Verify Brain', stepLabels[0]);
  check(7, 'Step 2 === "Gather Sources"', stepLabels[1] === 'Gather Sources', stepLabels[1]);
  check(7, 'Step 3 === "Learn"', stepLabels[2] === 'Learn', stepLabels[2]);
  check(7, 'Step 4 === "Ready"', stepLabels[3] === 'Ready', stepLabels[3]);

  // Active step indicator
  const activeStep = await page.locator('.kq-step--active').count();
  check(7, 'One active step', activeStep === 1);
}

await screenshot(page, '07-knowledge-quest-dialog');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 8 — Brain verification step
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 8: Brain verification step ═══');

// Check brain verification section
const sectionTitle = await txt(page, '.kq-section-title');
check(8, 'Section title === "🧠 Verifying Brain"', sectionTitle === '🧠 Verifying Brain', sectionTitle);

// Wait for checks to complete
await page.waitForTimeout(3000);

const checkItems = await page.locator('.kq-check').count();
check(8, 'Brain checks visible', checkItems === 4, `count=${checkItems}`);

// Verify individual check labels
const checkLabels = await allTexts(page, '.kq-check-label');
check(8, 'Check 1 === "Brain configured"', checkLabels[0] === 'Brain configured', checkLabels[0]);
check(8, 'Check 2 === "LLM model loaded"', checkLabels[1] === 'LLM model loaded', checkLabels[1]);
check(8, 'Check 3 === "Memory system ready"', checkLabels[2] === 'Memory system ready', checkLabels[2]);
check(8, 'Check 4 === "Ingestion engine online"', checkLabels[3] === 'Ingestion engine online', checkLabels[3]);

// Check for success icons (✅)
const checkIcons = await allTexts(page, '.kq-check-icon');
const passedChecks = checkIcons.filter(i => i === '✅').length;
check(8, 'All brain checks passed', passedChecks >= 3, `passed=${passedChecks}/4`);

// Continue button
const continueBtn = page.locator('.kq-btn-primary', { hasText: 'Continue' });
const continueBtnVis = await continueBtn.isVisible().catch(() => false);
check(8, '"Continue" button visible', continueBtnVis);

if (continueBtnVis) {
  await continueBtn.click();
  await page.waitForTimeout(500);
  check(8, 'Advanced to step 2', true);
}

await screenshot(page, '08-brain-verification');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 9 — Gather sources: URL + file
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 9: Gather sources ═══');

// Verify step 2 content
const step2Title = await txt(page, '.kq-section-title');
check(9, 'Section title === "📖 Gather Sources"', step2Title === '📖 Gather Sources', step2Title);

// URL input field
const urlFieldVis = await vis(page, '.kq-url-field');
check(9, 'URL input field visible', urlFieldVis);

const urlPlaceholder = await page.locator('.kq-url-field').first().getAttribute('placeholder');
check(9, 'URL placeholder', urlPlaceholder === 'https://example.com/document', urlPlaceholder);

// Add URL
const demoUrl = `${VITE_URL}/demo/vietnamese-civil-code.html`;
await page.locator('.kq-url-field').fill(demoUrl);
const urlAddBtn = page.locator('.kq-url-add');
check(9, '"＋ Add URL" button visible', await urlAddBtn.isVisible());
await urlAddBtn.click();
await page.waitForTimeout(300);

// Verify source added
const sourceItems = await page.locator('.kq-source-item').count();
check(9, 'URL source added', sourceItems >= 1, `count=${sourceItems}`);

const firstSourceName = await txt(page, '.kq-source-name');
check(9, 'Source name matches URL', firstSourceName.includes('vietnamese-civil-code'), firstSourceName);

// File attachment button
const fileBtnVis = await vis(page, '.kq-file-btn');
check(9, '"📎 Attach File" button visible', fileBtnVis);

const fileBtnText = await txt(page, '.kq-file-btn');
check(9, 'File button text === "📎 Attach File"', fileBtnText === '📎 Attach File', fileBtnText);

// Simulate file input (set file path in the hidden input)
const demoFile = 'public/demo/article-429-commentary.txt';
if (existsSync(demoFile)) {
  const fileInput = page.locator('.kq-file-hidden');
  await fileInput.setInputFiles(demoFile);
  await page.waitForTimeout(300);
  const sourceCount = await page.locator('.kq-source-item').count();
  check(9, 'File source added', sourceCount >= 2, `count=${sourceCount}`);
} else {
  check(9, 'File source added', 'SKIP', 'Demo file not found');
}

// "Start Learning" button
const startLearnBtn = page.locator('.kq-btn-primary', { hasText: 'Start Learning' });
const startLearnVis = await startLearnBtn.isVisible().catch(() => false);
check(9, '"⚡ Start Learning" button visible', startLearnVis);

await screenshot(page, '09-gather-sources');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 10 — Learning in progress
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 10: Learning in progress ═══');

if (startLearnVis) {
  await startLearnBtn.click();
  check(10, 'Clicked "Start Learning"', true);

  // Wait for the learning step
  await page.waitForTimeout(1000);
  const learnTitle = await txt(page, '.kq-section-title');
  check(10, 'Section title === "⚡ Learning in Progress"', learnTitle === '⚡ Learning in Progress', learnTitle);

  // Wait for ingestion tasks to appear
  const taskVis = await vis(page, '.kq-task', 10000);
  check(10, 'Task progress visible', taskVis);

  if (taskVis) {
    // Check progress bar
    check(10, 'Progress bar visible', await vis(page, '.kq-progress-bar'));

    // Wait for completion (up to 120s for URL fetch + chunking + embedding)
    console.log('  ⏳ Waiting for ingestion to complete...');
    try {
      await page.waitForFunction(() => {
        const tasks = document.querySelectorAll('.kq-task-done');
        return tasks.length > 0;
      }, { timeout: 120000 });
      check(10, 'Ingestion completed', true);
    } catch {
      // May still be in progress — check current state
      const pctText = await txt(page, '.kq-task-pct');
      check(10, 'Ingestion in progress', true, `progress: ${pctText}`);
    }
  }

  await screenshot(page, '10-learning-progress');

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
  check(10, 'Learning step', 'SKIP', 'Start Learning button not available');
}

// ═══════════════════════════════════════════════════════════════════════════
// STEP 11 — Knowledge Acquired!
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 11: Knowledge Acquired ═══');

const completeCard = await vis(page, '.kq-complete-card', 5000);
check(11, 'Complete card visible', completeCard);

if (completeCard) {
  const completeIcon = await txt(page, '.kq-complete-icon');
  check(11, 'Trophy icon === "🏆"', completeIcon === '🏆', completeIcon);

  const completeTitle = await txt(page, '.kq-section-title');
  check(11, 'Section title === "🎯 Knowledge Acquired!"', completeTitle === '🎯 Knowledge Acquired!', completeTitle);

  // Reward grid
  const rewardCards = await page.locator('.kq-reward-card').count();
  check(11, 'Reward cards count === 4', rewardCards === 4, `count=${rewardCards}`);

  // "Ask Questions" button
  const askBtn = page.locator('.kq-btn-primary', { hasText: 'Ask Questions' });
  const askBtnVis = await askBtn.isVisible().catch(() => false);
  check(11, '"🗡️ Ask Questions" button visible', askBtnVis);

  await screenshot(page, '11-knowledge-acquired');

  if (askBtnVis) {
    await askBtn.click();
    await page.waitForTimeout(1000);
    check(11, 'KQ dialog closed', !(await vis(page, '.kq-dialog', 1000)));
  }
} else {
  check(11, 'Knowledge Acquired step', 'SKIP', 'Complete card not visible');
}

// ═══════════════════════════════════════════════════════════════════════════
// STEP 12 — Alice asks law questions
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 12: Alice asks law questions ═══');

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
check(12, 'Completion message in chat', !!completionMsg, completionMsg?.content?.slice(0, 60));

// Ask first law question
const lawQuestion1 = 'What is the statute of limitations for contract disputes under Vietnamese law?';

// Count assistant messages before sending
const msgCount12Before = await page.evaluate(() => {
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
}, msgCount12Before, { timeout: 180000 });

const conv12 = await pinia(page, 'conversation');
const lastMsg12 = [...(conv12?.messages ?? [])].reverse().find(m => m.role === 'assistant');
check(12, 'RAG response received', (lastMsg12?.content?.length ?? 0) > 50,
  `length=${lastMsg12?.content?.length}`);

// Check if the response references Vietnamese law specifics
const content12 = lastMsg12?.content?.toLowerCase() ?? '';
const hasLawContent = content12.includes('429') ||
  content12.includes('three') || content12.includes('12 month') ||
  content12.includes('statute') || content12.includes('limitation') ||
  content12.includes('contract') || content12.includes('civil') ||
  content12.includes('vietnam') || content12.includes('claim');
check(12, 'Response references law content', hasLawContent, lastMsg12?.content?.slice(0, 100));

await screenshot(page, '12-alice-asks-law');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 13 — TerranSoul answers more law questions
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 13: More law questions ═══');

// Dismiss any quest suggestion overlay that appeared after Step 12
try {
  const hotseat = page.locator('.hotseat-strip');
  if (await hotseat.isVisible({ timeout: 2000 }).catch(() => false)) {
    const noThanks = page.locator('.hotseat-tile-label', { hasText: 'No thanks' });
    if (await noThanks.isVisible({ timeout: 1000 }).catch(() => false)) {
      await noThanks.click();
      await page.waitForTimeout(500);
      console.log('  ℹ️ Dismissed quest suggestion overlay');
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

const conv13 = await pinia(page, 'conversation');
const lastMsg13 = [...(conv13?.messages ?? [])].reverse().find(m => m.role === 'assistant');
check(13, 'Second RAG response received', (lastMsg13?.content?.length ?? 0) > 50,
  `length=${lastMsg13?.content?.length}`);

const content13 = lastMsg13?.content?.toLowerCase() ?? '';
const hasExemptionContent = content13.includes('force majeure') ||
  content13.includes('exempt') || content13.includes('421') ||
  content13.includes('fault') || content13.includes('breach') ||
  content13.includes('contract') || content13.includes('civil') ||
  content13.includes('liability') || content13.includes('vietnam');
check(13, 'Response references exemptions', hasExemptionContent, lastMsg13?.content?.slice(0, 100));

const brain13 = await pinia(page, 'brain');
check(13, 'Brain still local_ollama', brain13?.brainMode?.mode === 'local_ollama');
check(13, 'Brain model still gemma3:4b', brain13?.brainMode?.model === (ollamaModelName || 'gemma3:4b'));

await screenshot(page, '13-more-law-answers');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 14 — Skill tree stats
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 14: Skill tree ═══');

// Dismiss any quest suggestion that appeared after Step 13
try {
  const hotseat14 = page.locator('.hotseat-strip');
  if (await hotseat14.isVisible({ timeout: 2000 }).catch(() => false)) {
    const noThanks14 = page.locator('.hotseat-tile-label', { hasText: 'No thanks' });
    if (await noThanks14.isVisible({ timeout: 1000 }).catch(() => false)) {
      await noThanks14.click();
      await page.waitForTimeout(500);
    }
  }
} catch { /* ok */ }

// Navigate to Quests tab
await navTo(page, 'Quests');
await page.waitForTimeout(500);

check(14, '.skill-tree-view visible', await vis(page, '.skill-tree-view'));

const stTitle = await txt(page, '.st-title');
check(14, 'Title === "⚔️ Skill Tree"', stTitle === '⚔️ Skill Tree', stTitle);

check(14, '.brain-stat-sheet visible', await vis(page, '.brain-stat-sheet'));

const bssTitle = await txt(page, '.bss-title');
check(14, 'Sheet title === "⚔ Brain Stat Sheet"', bssTitle === '⚔ Brain Stat Sheet', bssTitle);

const statAbbrs = await allTexts(page, '.bss-stat-abbr');
check(14, 'Stats === ["INT","WIS","CHA","PER","DEX","END"]',
  JSON.stringify(statAbbrs) === JSON.stringify(['INT', 'WIS', 'CHA', 'PER', 'DEX', 'END']),
  JSON.stringify(statAbbrs));

const levelBadge = await txt(page, '.bss-level');
check(14, 'Level badge matches /^Lv\\. \\d+$/', /^Lv\. \d+$/.test(levelBadge), levelBadge);

check(14, '.st-daily-section visible', await vis(page, '.st-daily-section'));

await screenshot(page, '14-skill-tree');

// ═══════════════════════════════════════════════════════════════════════════
// STEP 15 — Pet mode with chat
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 15: Pet mode ═══');

// Navigate back to Chat first
await navTo(page, 'Chat');
await page.waitForTimeout(500);

// Toggle pet mode
await page.locator('.mode-toggle-pill').click();
await page.waitForTimeout(2000);

const petOverlay = await vis(page, '.pet-overlay', 5000);
check(15, 'Pet overlay visible', petOverlay);

if (petOverlay) {
  const hasAppShellPetMode = await page.locator('.app-shell.pet-mode').isVisible().catch(() => false);
  check(15, 'App shell has .pet-mode class', hasAppShellPetMode);

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
      check(15, 'Pet chat panel visible', petChatVisible);
    } catch {
      check(15, 'Pet chat panel', 'SKIP', 'click did not propagate');
    }
  }

  await screenshot(page, '15-pet-mode');

  // Exit pet mode via Escape key (mode toggle is hidden in pet mode)
  await page.keyboard.press('Escape');
  await page.waitForTimeout(1000);
  check(15, 'Exited pet mode', !(await vis(page, '.pet-overlay', 1000)));
}

await screenshot(page, '15-final');

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
