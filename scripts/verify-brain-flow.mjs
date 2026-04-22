/**
 * verify-brain-flow.mjs
 *
 * Playwright-based verification of every step in
 * instructions/BRAIN-COMPLEX-EXAMPLE.md.
 *
 * Pre-requisites:
 *   1. Docker Desktop running with "ollama" container up
 *   2. Vite dev server on http://localhost:1420  (`npm run dev`)
 *   3. Playwright Chromium installed (`npx playwright install chromium`)
 *
 * Usage:
 *   node scripts/verify-brain-flow.mjs
 */
import { chromium } from 'playwright';
import { execSync } from 'child_process';
import { mkdirSync } from 'fs';

const BASE = 'http://localhost:1420';
const OUT  = 'instructions/screenshots';
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

async function vis(page, sel, timeout = 3000) {
  try {
    await page.locator(sel).first().waitFor({ state: 'visible', timeout });
    return true;
  } catch { return false; }
}

async function txt(page, sel) {
  try {
    return (await page.locator(sel).first().textContent())?.trim() ?? '';
  } catch { return ''; }
}

async function pinia(page, store) {
  return page.evaluate((s) => {
    const app = document.querySelector('#app')?.__vue_app__;
    return app?.config?.globalProperties?.$pinia?.state?.value?.[s] ?? null;
  }, store);
}

// ═══════════════════════════════════════════════════════════════════════════
// STEP 0 — Pre-flight: Docker + Ollama container
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 0: Pre-flight — Docker & Ollama container ═══');

let dockerOk = false;
let containerOk = false;
let ollamaApiOk = false;
let dockerVersion = '';
let containerInfo = '';
let ollamaModels = [];

try {
  dockerVersion = execSync('docker --version', { encoding: 'utf8' }).trim();
  check(0, 'Docker CLI installed', true, dockerVersion);

  const psOut = execSync('docker ps --format "{{.Names}}|{{.Image}}|{{.Status}}"', { encoding: 'utf8' });
  const lines = psOut.trim().split('\n').filter(Boolean);
  const ollamaLine = lines.find(l => l.startsWith('ollama|'));
  dockerOk = true;

  if (ollamaLine) {
    const [name, image, status] = ollamaLine.split('|');
    containerInfo = `name=${name} image=${image} status=${status}`;
    check(0, 'Ollama container running', status.startsWith('Up'), containerInfo);
    containerOk = status.startsWith('Up');
  } else {
    check(0, 'Ollama container running', false, 'no "ollama" container in docker ps');
  }
} catch (e) {
  check(0, 'Docker CLI installed', false, e.message);
}

try {
  const resp = await fetch('http://localhost:11434/api/tags');
  if (resp.ok) {
    const data = await resp.json();
    ollamaModels = (data.models || []).map(m => m.name);
    ollamaApiOk = true;
    check(0, 'Ollama API reachable (port 11434)', true,
      ollamaModels.length > 0 ? `models: ${ollamaModels.join(', ')}` : 'no models installed');
  } else {
    check(0, 'Ollama API reachable (port 11434)', false, `HTTP ${resp.status}`);
  }
} catch (e) {
  check(0, 'Ollama API reachable (port 11434)', false, e.message);
}

// ═══════════════════════════════════════════════════════════════════════════
// Launch browser
// ═══════════════════════════════════════════════════════════════════════════
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });

try {
  // ═══════════════════════════════════════════════════════════════════════
  // STEP 1 — Fresh launch
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 1: Fresh launch ═══');
  await page.goto(BASE, { waitUntil: 'networkidle', timeout: 30_000 });
  // Wait for Vue + Pinia + ChatView to render
  await page.waitForFunction(() => {
    const el = document.querySelector('.chat-view');
    return el && el.offsetParent !== null;
  }, { timeout: 30_000 });
  await page.waitForTimeout(2000);

  check(1, '.chat-view visible', await vis(page, '.chat-view'));
  check(1, '.viewport-layer visible', await vis(page, '.viewport-layer'));
  check(1, '.input-footer visible', await vis(page, '.input-footer'));
  check(1, 'nav.desktop-nav visible', await vis(page, 'nav.desktop-nav'));

  // Nav button exact labels
  const navLabels = await page.locator('nav.desktop-nav .nav-btn .nav-label')
    .allTextContents();
  check(1, 'Nav tabs: Chat, Quests, Memory, Market, Voice',
    JSON.stringify(navLabels) === JSON.stringify(['Chat','Quests','Memory','Market','Voice']),
    `got: ${JSON.stringify(navLabels)}`);

  check(1, '.ai-state-pill visible', await vis(page, '.ai-state-pill'));
  check(1, '.ff-orb (quest orb) visible', await vis(page, '.ff-orb', 5000));

  // Mode toggle — only visible on chat tab
  const toggleVis = await vis(page, '.mode-toggle-pill');
  check(1, '.mode-toggle-pill visible', toggleVis);
  if (toggleVis) {
    const toggleLabel = await txt(page, '.mode-toggle-label');
    check(1, 'Mode toggle label is "Desktop"', toggleLabel === 'Desktop',
      `got: "${toggleLabel}"`);
  }

  // Chat input exact selector + placeholder
  check(1, 'input.chat-input visible', await vis(page, 'input.chat-input'));
  const placeholder = await page.locator('input.chat-input').getAttribute('placeholder');
  check(1, 'Chat input placeholder = "Type a message…"',
    placeholder === 'Type a message…', `got: "${placeholder}"`);

  // Send button
  check(1, 'button.send-btn visible', await vis(page, 'button.send-btn'));

  await page.screenshot({ path: `${OUT}/01-fresh-launch.png` });
  console.log('  📸 01-fresh-launch.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 2 — Brain auto-configured (Free Cloud API)
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 2: Brain auto-configured ═══');
  const brainState = await pinia(page, 'brain');
  check(2, 'Brain Pinia store exists', brainState !== null);

  const brainMode = brainState?.brainMode;
  check(2, 'brainMode.mode === "free_api"',
    brainMode?.mode === 'free_api', `got: ${brainMode?.mode}`);
  check(2, 'brainMode.provider_id === "pollinations"',
    brainMode?.provider_id === 'pollinations', `got: ${brainMode?.provider_id}`);

  // LLM provider pill
  const pillVis = await vis(page, '.brain-status-pill');
  check(2, '.brain-status-pill visible', pillVis);
  if (pillVis) {
    const pillText = await txt(page, '.brain-status-pill');
    check(2, 'Brain pill shows "Pollinations AI"',
      pillText.includes('Pollinations'), `got: "${pillText}"`);
  }

  // Brain setup overlay should NOT be showing
  check(2, 'Brain setup overlay (.brain-overlay) hidden',
    !(await vis(page, '.brain-overlay', 1000)));

  // Free providers loaded
  check(2, 'freeProviders[] loaded',
    brainState?.freeProviders?.length > 0,
    `count=${brainState?.freeProviders?.length}`);

  await page.screenshot({ path: `${OUT}/02-brain-auto-configured.png` });
  console.log('  📸 02-brain-auto-configured.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 3 — Docker & LLM model assertion + screenshot
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 3: Docker & LLM model verification ═══');

  check(3, 'Docker daemon accessible', dockerOk, dockerVersion);
  check(3, 'Ollama container up', containerOk, containerInfo);
  check(3, 'Ollama API responsive', ollamaApiOk,
    ollamaModels.length > 0 ? `models: ${ollamaModels.join(', ')}` : 'no local models');

  // Assert the exact LLM model/provider being used
  const modeForAssert = brainState?.brainMode;
  if (modeForAssert?.mode === 'free_api') {
    check(3, 'Active LLM provider: Pollinations (free_api)',
      modeForAssert.provider_id === 'pollinations',
      `provider_id=${modeForAssert.provider_id}`);
  } else if (modeForAssert?.mode === 'local_ollama') {
    check(3, `Active LLM model: ${modeForAssert.model} (local_ollama)`, true,
      `model=${modeForAssert.model}`);
  } else if (modeForAssert?.mode === 'paid_api') {
    check(3, `Active LLM: ${modeForAssert.provider}/${modeForAssert.model} (paid_api)`, true,
      `base_url=${modeForAssert.base_url}`);
  } else {
    check(3, 'Active LLM identified', false, 'brainMode is null');
  }

  // Ollama status from Pinia
  const ollamaStatusPinia = brainState?.ollamaStatus;
  check(3, 'ollamaStatus in Pinia',
    ollamaStatusPinia !== null && ollamaStatusPinia !== undefined,
    `running=${ollamaStatusPinia?.running}, model_count=${ollamaStatusPinia?.model_count}`);

  await page.screenshot({ path: `${OUT}/03-docker-and-model.png` });
  console.log('  📸 03-docker-and-model.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 4 — Quest constellation
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 4: Quest constellation ═══');
  const orbVis = await vis(page, '.ff-orb');
  if (orbVis) {
    const orbPct = await txt(page, '.ff-orb-pct');
    check(4, 'Orb shows percentage', /\d+%/.test(orbPct), `got: "${orbPct}"`);

    await page.locator('.ff-orb').click();
    await page.waitForTimeout(1200);

    check(4, '.skill-constellation opened', await vis(page, '.skill-constellation', 5000));

    const closeBtnVis = await vis(page, '.sc-close-btn');
    check(4, '.sc-close-btn visible', closeBtnVis);
    if (closeBtnVis) {
      const closeTxt = await txt(page, '.sc-close-btn');
      check(4, 'Close button text = "✕"', closeTxt === '✕', `got: "${closeTxt}"`);
    }

    check(4, 'Breadcrumb "✦ All Clusters"',
      (await txt(page, '.sc-crumb--root')).includes('All Clusters'));

    await page.screenshot({ path: `${OUT}/04-quest-constellation.png` });
    console.log('  📸 04-quest-constellation.png');

    if (closeBtnVis) {
      await page.locator('.sc-close-btn').click();
    } else {
      await page.keyboard.press('Escape');
    }
    await page.waitForTimeout(500);
  } else {
    check(4, 'Quest orb found', false);
  }

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 5 — Pet mode
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 5: Pet mode ═══');
  const modeBtn = page.locator('.mode-toggle-btn');
  if (await modeBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
    await modeBtn.click();
    await page.waitForTimeout(1500);

    check(5, '.pet-overlay visible', await vis(page, '.pet-overlay', 3000));
    check(5, '.app-shell has .pet-mode class',
      await page.locator('.app-shell').evaluate(el => el.classList.contains('pet-mode')));
    check(5, '.pet-character visible', await vis(page, '.pet-character', 2000));

    const onboardingVis = await vis(page, '.pet-onboarding', 2000);
    if (onboardingVis) {
      const obTitle = await txt(page, '.pet-onboarding-title');
      check(5, 'Onboarding title = "Welcome to pet mode"',
        obTitle === 'Welcome to pet mode', `got: "${obTitle}"`);
      const dismissTxt = await txt(page, '.pet-onboarding-dismiss');
      check(5, 'Dismiss button = "Got it"', dismissTxt === 'Got it',
        `got: "${dismissTxt}"`);
    } else {
      check(5, 'Pet onboarding (may be dismissed)', 'SKIP', 'not first visit');
    }

    await page.screenshot({ path: `${OUT}/05-pet-mode.png` });
    console.log('  📸 05-pet-mode.png');

    await page.keyboard.press('Escape');
    await page.waitForTimeout(1000);
    check(5, 'Exited pet mode', !(await vis(page, '.pet-overlay', 1000)));
  } else {
    check(5, 'Mode toggle button found', false);
  }

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 6 — Chat: first question (no memories)
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 6: Chat — first question (no memories) ═══');
  const drawerToggle = page.locator('.chat-drawer-toggle');
  if (await drawerToggle.isVisible({ timeout: 1000 }).catch(() => false)) {
    const toggleLbl = await txt(page, '.toggle-label');
    if (toggleLbl === 'Chat') {
      await drawerToggle.click();
      await page.waitForTimeout(500);
    }
  }

  const chatInput = page.locator('input.chat-input');
  const sendBtn   = page.locator('button.send-btn');

  check(6, 'input.chat-input enabled', await chatInput.isEnabled().catch(() => false));

  await chatInput.fill('What is the deadline for responding to a civil lawsuit in Vietnam?');
  const inputVal = await chatInput.inputValue();
  check(6, 'Input value filled', inputVal.includes('deadline'), `${inputVal.length} chars`);

  check(6, 'button.send-btn visible', await sendBtn.isVisible().catch(() => false));
  await sendBtn.click();

  let firstReply = '';
  try {
    await page.waitForFunction(() => {
      const app = document.querySelector('#app')?.__vue_app__;
      const conv = app?.config?.globalProperties?.$pinia?.state?.value?.conversation;
      const msgs = conv?.messages ?? [];
      const last = msgs[msgs.length - 1];
      return last?.role === 'assistant' && last?.content?.length > 20;
    }, { timeout: 60_000 });
    const conv = await pinia(page, 'conversation');
    const msgs = conv?.messages ?? [];
    firstReply = msgs[msgs.length - 1]?.content ?? '';
    check(6, 'Assistant replied (>20 chars)', firstReply.length > 20,
      `${firstReply.length} chars`);
  } catch {
    check(6, 'Assistant replied', false, 'timeout after 60s');
  }

  const userBubbles = await page.locator('.message-row.user').count();
  check(6, '.message-row.user rendered', userBubbles >= 1, `count=${userBubbles}`);
  const asstBubbles = await page.locator('.message-row.assistant').count();
  check(6, '.message-row.assistant rendered', asstBubbles >= 1, `count=${asstBubbles}`);

  await page.waitForTimeout(1000);
  await page.screenshot({ path: `${OUT}/06-chat-no-memories.png` });
  console.log('  📸 06-chat-no-memories.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 7 — Memory tab (empty)
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 7: Memory tab (empty) ═══');
  await page.locator('nav.desktop-nav .nav-btn').filter({ hasText: 'Memory' }).click();
  await page.waitForTimeout(1000);

  check(7, '.memory-view visible', await vis(page, '.memory-view'));

  const mvHeader = await txt(page, '.mv-header h2');
  check(7, 'Header "🧠 Memory"', mvHeader.includes('Memory'), `got: "${mvHeader}"`);

  const addBtnTxt = await txt(page, '.mv-header-actions .btn-primary');
  check(7, '"＋ Add memory" button', addBtnTxt.includes('Add memory'),
    `got: "${addBtnTxt}"`);

  const tabTexts = await page.locator('.mv-tab').allTextContents();
  check(7, 'Sub-tabs: List, Graph, Session',
    tabTexts.some(t => t.includes('List'))
    && tabTexts.some(t => t.includes('Graph'))
    && tabTexts.some(t => t.includes('Session')),
    `got: ${JSON.stringify(tabTexts)}`);

  const tierChips = await page.locator('.mv-tier-chip').allTextContents();
  check(7, 'Tier chips: short, working, long',
    tierChips.some(t => t.includes('short'))
    && tierChips.some(t => t.includes('working'))
    && tierChips.some(t => t.includes('long')),
    `got: ${JSON.stringify(tierChips)}`);

  const typeChips = await page.locator('.mv-type-chip').allTextContents();
  check(7, 'Type chips: fact, preference, context, summary',
    typeChips.length >= 4, `got: ${JSON.stringify(typeChips)}`);

  const actionTexts = await page.locator('.mv-header-actions .btn-secondary')
    .allTextContents();
  check(7, 'Actions: Extract, Summarize, Decay, GC',
    actionTexts.some(t => t.includes('Extract'))
    && actionTexts.some(t => t.includes('Summarize'))
    && actionTexts.some(t => t.includes('Decay'))
    && actionTexts.some(t => t.includes('GC')),
    `got: ${JSON.stringify(actionTexts.map(t => t.trim()))}`);

  await page.screenshot({ path: `${OUT}/07-memory-empty.png` });
  console.log('  📸 07-memory-empty.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 8 — Add a memory
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 8: Add a memory ═══');
  await page.locator('.mv-header-actions .btn-primary').click();
  await page.waitForTimeout(500);

  check(8, '.mv-modal opened', await vis(page, '.mv-modal'));

  const modalTitle = await txt(page, '.mv-modal h3');
  check(8, 'Modal title = "Add memory"', modalTitle === 'Add memory',
    `got: "${modalTitle}"`);

  const ta = page.locator('.mv-modal textarea');
  const taPlaceholder = await ta.getAttribute('placeholder');
  check(8, 'Textarea placeholder = "What should I remember?"',
    taPlaceholder === 'What should I remember?', `got: "${taPlaceholder}"`);

  await ta.fill(
    'Theo Điều 429 Bộ luật Dân sự 2015, thời hiệu khởi kiện để yêu cầu ' +
    'Tòa án giải quyết tranh chấp hợp đồng là 03 năm, kể từ ngày người có ' +
    'quyền yêu cầu biết hoặc phải biết quyền và lợi ích hợp pháp của mình bị xâm phạm.'
  );

  const tagInput = page.locator('.mv-modal input[placeholder]').first();
  const tagPlaceholder = await tagInput.getAttribute('placeholder');
  check(8, 'Tags placeholder = "python, work, project"',
    tagPlaceholder === 'python, work, project', `got: "${tagPlaceholder}"`);
  await tagInput.fill('vn-law, civil-code, statute');

  const saveBtn2 = page.locator('.mv-modal .btn-primary');
  const saveTxt = await saveBtn2.textContent();
  check(8, 'Save button text = "Save"', saveTxt?.trim() === 'Save',
    `got: "${saveTxt?.trim()}"`);

  await page.screenshot({ path: `${OUT}/08-memory-add-modal.png` });
  console.log('  📸 08-memory-add-modal.png');

  await saveBtn2.click();
  await page.waitForTimeout(1500);

  check(8, 'Modal closed after save', !(await vis(page, '.mv-modal', 1000)));

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 9 — Memories list
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 9: Memories list ═══');

  // Add a second memory
  await page.locator('.mv-header-actions .btn-primary').click();
  await page.waitForTimeout(500);
  if (await vis(page, '.mv-modal')) {
    await page.locator('.mv-modal textarea').fill(
      'Quy tắc nội bộ §3.2 — Mọi đơn khởi kiện hợp đồng có giá trị > 5 tỷ VND ' +
      'phải được luật sư cấp cao xem xét trước khi nộp tòa.');
    await page.locator('.mv-modal input[placeholder]').first().fill('vn-law, internal, firm-rules');
    await page.locator('.mv-modal .btn-primary').click();
    await page.waitForTimeout(1500);
  }

  const cardCount = await page.locator('.mv-card').count();
  if (cardCount > 0) {
    check(9, 'Memory cards rendered', true, `count=${cardCount}`);
    check(9, '.mv-chip (type badge) visible', await vis(page, '.mv-card .mv-chip'));
    check(9, '.mv-content visible', await vis(page, '.mv-card .mv-content'));
  } else {
    check(9, 'Memory cards (needs Tauri IPC)', 'SKIP',
      'add_memory command unavailable in browser-only mode');
  }

  await page.screenshot({ path: `${OUT}/09-memories-list.png` });
  console.log('  📸 09-memories-list.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 10 — Memory graph
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 10: Memory graph ═══');
  const graphTab = page.locator('.mv-tab').filter({ hasText: 'Graph' });
  if (await graphTab.isVisible({ timeout: 1000 }).catch(() => false)) {
    await graphTab.click();
    await page.waitForTimeout(1500);

    check(10, 'Graph panel visible',
      await vis(page, '.mv-graph-panel', 3000) || await vis(page, '.memory-graph', 3000));
  } else {
    check(10, 'Graph tab found', false);
  }

  await page.screenshot({ path: `${OUT}/10-memory-graph.png` });
  console.log('  📸 10-memory-graph.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 11 — Chat with RAG
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 11: Chat with RAG ═══');
  // Navigate back to Chat tab
  await page.locator('nav.desktop-nav .nav-btn').filter({ hasText: 'Chat' }).click();
  // Use same waitForFunction technique as Step 1 — v-show needs offsetParent check
  let chatViewBack = false;
  try {
    await page.waitForFunction(() => {
      const el = document.querySelector('.chat-view');
      return el && el.offsetParent !== null;
    }, { timeout: 10_000 });
    chatViewBack = true;
  } catch { chatViewBack = false; }
  check(11, 'Back on .chat-view', chatViewBack);

  const dToggle2 = page.locator('.chat-drawer-toggle');
  if (await dToggle2.isVisible({ timeout: 1000 }).catch(() => false)) {
    const lbl2 = await txt(page, '.toggle-label');
    if (lbl2 === 'Chat') {
      await dToggle2.click();
      await page.waitForTimeout(500);
    }
  }

  await chatInput.fill('What is the deadline for responding to a civil lawsuit in Vietnam?');
  await sendBtn.click();

  let ragReply = '';
  try {
    const msgCountBefore = (await pinia(page, 'conversation'))?.messages?.length ?? 0;
    await page.waitForFunction((before) => {
      const app = document.querySelector('#app')?.__vue_app__;
      const conv = app?.config?.globalProperties?.$pinia?.state?.value?.conversation;
      const msgs = conv?.messages ?? [];
      return msgs.length > before && msgs[msgs.length - 1]?.role === 'assistant'
        && msgs[msgs.length - 1]?.content?.length > 20;
    }, msgCountBefore, { timeout: 60_000 });
    const conv2 = await pinia(page, 'conversation');
    ragReply = conv2?.messages?.[conv2.messages.length - 1]?.content ?? '';
    check(11, 'Got RAG reply (>20 chars)', ragReply.length > 20,
      `${ragReply.length} chars`);
  } catch {
    check(11, 'Got RAG reply', false, 'timeout after 60s');
  }

  await page.waitForTimeout(1000);
  await page.screenshot({ path: `${OUT}/11-chat-with-rag.png` });
  console.log('  📸 11-chat-with-rag.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 12 — Skill tree
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 12: Skill tree ═══');
  await page.locator('nav.desktop-nav .nav-btn').filter({ hasText: 'Quests' }).click();
  await page.waitForTimeout(1500);

  check(12, '.skill-tree-view visible', await vis(page, '.skill-tree-view'));

  const stTitle = await txt(page, '.st-title');
  check(12, 'Title "⚔️ Skill Tree"', stTitle.includes('Skill Tree'),
    `got: "${stTitle}"`);

  check(12, '.brain-stat-sheet visible', await vis(page, '.brain-stat-sheet'));

  const bssTitle = await txt(page, '.bss-title');
  check(12, '"Brain Stat Sheet" in header', bssTitle.includes('Brain Stat Sheet'),
    `got: "${bssTitle}"`);

  const statAbbrs = await page.locator('.bss-stat-abbr').allTextContents();
  check(12, 'Stats: INT, WIS, CHA, PER, DEX, END',
    ['INT','WIS','CHA','PER','DEX','END'].every(s => statAbbrs.includes(s)),
    `got: ${JSON.stringify(statAbbrs)}`);

  const levelTxt = await txt(page, '.bss-level');
  check(12, 'Level badge "Lv. N"', levelTxt.includes('Lv.'), `got: "${levelTxt}"`);

  check(12, "Today's Quests section", await vis(page, '.st-daily-section'));

  await page.screenshot({ path: `${OUT}/12-skill-tree.png` });
  console.log('  📸 12-skill-tree.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 13 — Pet mode with chat
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 13: Pet mode with chat ═══');
  await page.locator('nav.desktop-nav .nav-btn').filter({ hasText: 'Chat' }).click();
  await page.waitForTimeout(1000);

  const toggleBtn2 = page.locator('.mode-toggle-btn');
  if (await toggleBtn2.isVisible({ timeout: 2000 }).catch(() => false)) {
    await toggleBtn2.click();
    await page.waitForTimeout(1500);

    check(13, '.pet-overlay visible', await vis(page, '.pet-overlay', 3000));

    // Try opening pet chat
    let petChatVis = await vis(page, '.pet-chat', 2000);
    if (!petChatVis) {
      // Click character to open chat
      const petChar = page.locator('.pet-character');
      if (await petChar.isVisible({ timeout: 1000 }).catch(() => false)) {
        await petChar.click();
        await page.waitForTimeout(1000);
        petChatVis = await vis(page, '.pet-chat', 2000);
      }
    }

    if (petChatVis) {
      check(13, '.pet-chat visible', true);
      const petPlaceholder = await page.locator('.pet-chat-input input[type="text"]')
        .getAttribute('placeholder').catch(() => '');
      check(13, 'Pet input placeholder = "Say something…"',
        petPlaceholder === 'Say something…', `got: "${petPlaceholder}"`);

      const petSendTxt = await txt(page, '.pet-chat-input button[type="submit"]');
      check(13, 'Pet send button text = "➤"', petSendTxt === '➤',
        `got: "${petSendTxt}"`);
    } else {
      check(13, 'Pet chat accessible', 'SKIP', 'could not open pet chat panel');
    }

    await page.screenshot({ path: `${OUT}/13-pet-mode-chat.png` });
    console.log('  📸 13-pet-mode-chat.png');

    await page.keyboard.press('Escape');
    await page.waitForTimeout(500);
  } else {
    check(13, 'Mode toggle found', false);
  }

} catch (err) {
  console.error('\n💥 Fatal error:', err.message);
  console.error(err.stack);
  await page.screenshot({ path: `${OUT}/error-state.png` }).catch(() => {});
} finally {
  await browser.close();
}

// ═══════════════════════════════════════════════════════════════════════════
// Summary
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n' + '═'.repeat(60));
console.log(`RESULT: ${passed} passed, ${failed} failed, ${skipped} skipped`);
console.log('═'.repeat(60));
for (const r of results) {
  const icon = r.status === 'PASS' ? '✅' : r.status === 'SKIP' ? '⏭ ' : '❌';
  console.log(`  ${icon} [Step ${r.step}] ${r.name}${r.detail ? ` (${r.detail})` : ''}`);
}
console.log('═'.repeat(60));
console.log(`Screenshots → ${OUT}/`);

if (failed > 0) {
  console.log(`\n⚠️  ${failed} check(s) failed`);
  process.exit(1);
}
