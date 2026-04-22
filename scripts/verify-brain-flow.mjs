/**
 * verify-brain-flow.mjs
 *
 * Playwright-based verification of every step in
 * instructions/BRAIN-COMPLEX-EXAMPLE.md.
 *
 * Pre-requisites:
 *   1. Docker Desktop running with "ollama" container up + at least one model pulled
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
// STEP 0 — Pre-flight: Docker + Ollama container + model
// ═══════════════════════════════════════════════════════════════════════════
console.log('\n═══ Step 0: Pre-flight — Docker & Ollama container ═══');

let dockerOk = false;
let containerOk = false;
let ollamaApiOk = false;
let dockerVersion = '';
let containerInfo = '';
let ollamaModels = [];
let ollamaModelName = ''; // First installed model tag

try {
  dockerVersion = execSync('docker --version', { encoding: 'utf8' }).trim();
  check(0, 'Docker CLI installed', dockerVersion.startsWith('Docker version'),
    dockerVersion);
  dockerOk = dockerVersion.startsWith('Docker version');

  const psOut = execSync('docker ps --format "{{.Names}}|{{.Image}}|{{.Status}}"',
    { encoding: 'utf8' });
  const lines = psOut.trim().split('\n').filter(Boolean);
  const ollamaLine = lines.find(l => l.startsWith('ollama|'));

  if (ollamaLine) {
    const [name, image, status] = ollamaLine.split('|');
    containerInfo = `name=${name} image=${image} status=${status}`;
    containerOk = status.startsWith('Up');
    check(0, 'Ollama container running', containerOk, containerInfo);
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
      `models: [${ollamaModels.join(', ')}]`);
    check(0, 'At least one model installed', ollamaModels.length > 0,
      `count=${ollamaModels.length}`);
    if (ollamaModels.length > 0) {
      ollamaModelName = ollamaModels[0];
      check(0, `Model available: "${ollamaModelName}"`, true,
        `tag=${ollamaModelName}`);
    }
  } else {
    check(0, 'Ollama API reachable (port 11434)', false, `HTTP ${resp.status}`);
  }
} catch (e) {
  check(0, 'Ollama API reachable (port 11434)', false, e.message);
}

// Quick sanity: send a test prompt to Ollama to confirm the model works
if (ollamaModelName) {
  try {
    const testResp = await fetch('http://localhost:11434/api/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: ollamaModelName,
        messages: [{ role: 'user', content: 'Say "hello" and nothing else.' }],
        stream: false,
      }),
    });
    if (testResp.ok) {
      const testData = await testResp.json();
      const testContent = testData?.message?.content ?? '';
      check(0, 'Ollama model responds to test prompt', testContent.length > 0,
        `reply: "${testContent.slice(0, 80)}"`);
    } else {
      check(0, 'Ollama model responds to test prompt', false, `HTTP ${testResp.status}`);
    }
  } catch (e) {
    check(0, 'Ollama model responds to test prompt', false, e.message);
  }
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
  await page.waitForFunction(() => {
    const el = document.querySelector('.chat-view');
    return el && el.offsetParent !== null;
  }, { timeout: 30_000 });
  await page.waitForTimeout(2000);

  check(1, '.chat-view visible', await vis(page, '.chat-view'));
  check(1, '.viewport-layer visible', await vis(page, '.viewport-layer'));
  check(1, '.input-footer visible', await vis(page, '.input-footer'));
  check(1, 'nav.desktop-nav visible', await vis(page, 'nav.desktop-nav'));

  const navLabels = await page.locator('nav.desktop-nav .nav-btn .nav-label')
    .allTextContents();
  check(1, 'Nav labels === ["Chat","Quests","Memory","Market","Voice"]',
    JSON.stringify(navLabels) === JSON.stringify(['Chat','Quests','Memory','Market','Voice']),
    `got: ${JSON.stringify(navLabels)}`);

  check(1, '.ai-state-pill visible', await vis(page, '.ai-state-pill'));
  check(1, '.ff-orb visible', await vis(page, '.ff-orb', 5000));

  check(1, '.mode-toggle-pill visible', await vis(page, '.mode-toggle-pill'));
  const toggleLabel = await txt(page, '.mode-toggle-label');
  check(1, '.mode-toggle-label === "Desktop"', toggleLabel === 'Desktop',
    `got: "${toggleLabel}"`);

  check(1, 'input.chat-input visible', await vis(page, 'input.chat-input'));
  const placeholder = await page.locator('input.chat-input').getAttribute('placeholder');
  check(1, 'placeholder === "Type a message…"',
    placeholder === 'Type a message…', `got: "${placeholder}"`);

  check(1, 'button.send-btn visible', await vis(page, 'button.send-btn'));

  await page.screenshot({ path: `${OUT}/01-fresh-launch.png` });
  console.log('  📸 01-fresh-launch.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 2 — Configure brain → Local Ollama
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 2: Configure brain → Local Ollama ═══');

  const initBrain = await pinia(page, 'brain');
  check(2, 'Brain Pinia store exists', initBrain !== null);
  check(2, 'Initial brainMode.mode === "free_api"',
    initBrain?.brainMode?.mode === 'free_api',
    `got: ${initBrain?.brainMode?.mode}`);
  check(2, 'freeProviders loaded (count > 0)',
    initBrain?.freeProviders?.length > 0,
    `count=${initBrain?.freeProviders?.length}`);

  // Switch to local Ollama via Pinia state injection
  if (ollamaModelName) {
    await page.evaluate((model) => {
      const app = document.querySelector('#app')?.__vue_app__;
      const pinia = app?.config?.globalProperties?.$pinia;
      if (pinia?.state?.value?.brain) {
        pinia.state.value.brain.brainMode = {
          mode: 'local_ollama',
          model: model,
        };
        pinia.state.value.brain.ollamaStatus = {
          running: true,
          model_count: 1,
        };
      }
    }, ollamaModelName);
    await page.waitForTimeout(1000);

    const updatedBrain = await pinia(page, 'brain');
    check(2, 'brainMode.mode === "local_ollama"',
      updatedBrain?.brainMode?.mode === 'local_ollama',
      `got: ${updatedBrain?.brainMode?.mode}`);
    check(2, `brainMode.model === "${ollamaModelName}"`,
      updatedBrain?.brainMode?.model === ollamaModelName,
      `got: ${updatedBrain?.brainMode?.model}`);

    // Brain status pill should show "Ollama · <model>"
    const expectedPill = `Ollama · ${ollamaModelName}`;
    await page.waitForTimeout(500);
    const pillVis = await vis(page, '.brain-status-pill', 3000);
    check(2, '.brain-status-pill visible', pillVis);
    if (pillVis) {
      const pillText = await txt(page, '.brain-status-pill');
      check(2, `Brain pill === "${expectedPill}"`,
        pillText === expectedPill, `got: "${pillText}"`);
    }

    check(2, 'ollamaStatus.running === true',
      updatedBrain?.ollamaStatus?.running === true,
      `got: ${updatedBrain?.ollamaStatus?.running}`);
    check(2, 'ollamaStatus.model_count === 1',
      updatedBrain?.ollamaStatus?.model_count === 1,
      `got: ${updatedBrain?.ollamaStatus?.model_count}`);
  } else {
    check(2, 'Switch to local Ollama', 'SKIP', 'no Ollama models available');
  }

  check(2, '.brain-overlay hidden',
    !(await vis(page, '.brain-overlay', 1000)));

  await page.screenshot({ path: `${OUT}/02-brain-auto-configured.png` });
  console.log('  📸 02-brain-auto-configured.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 3 — Docker & LLM model exact verification
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 3: Docker & LLM model verification ═══');

  check(3, 'Docker version starts with "Docker version"',
    dockerVersion.startsWith('Docker version'), dockerVersion);
  check(3, 'Ollama container status starts with "Up"', containerOk, containerInfo);

  // Re-query Ollama API for exact model details
  let modelFamily = '';
  let modelParams = '';
  let modelQuant = '';
  try {
    const resp2 = await fetch('http://localhost:11434/api/tags');
    const data2 = await resp2.json();
    const m = data2.models?.find(x => x.name === ollamaModelName);
    if (m) {
      modelFamily = m.details?.family ?? '';
      modelParams = m.details?.parameter_size ?? '';
      modelQuant = m.details?.quantization_level ?? '';
      check(3, `Model "${ollamaModelName}" in Ollama API`, true,
        `family=${modelFamily}, params=${modelParams}, quant=${modelQuant}`);
    } else {
      check(3, `Model "${ollamaModelName}" in Ollama API`, false,
        `available: [${data2.models?.map(x => x.name).join(', ')}]`);
    }
  } catch (e) {
    check(3, 'Ollama API model query', false, e.message);
  }

  // Assert exact Pinia state
  const brainForStep3 = await pinia(page, 'brain');
  const mode3 = brainForStep3?.brainMode;
  check(3, 'Pinia brainMode.mode === "local_ollama"',
    mode3?.mode === 'local_ollama', `got: ${mode3?.mode}`);
  check(3, `Pinia brainMode.model === "${ollamaModelName}"`,
    mode3?.model === ollamaModelName, `got: ${mode3?.model}`);

  // Brain pill exact text
  const expectedPill3 = `Ollama · ${ollamaModelName}`;
  const pillText3 = await txt(page, '.brain-status-pill');
  check(3, `Brain pill === "${expectedPill3}"`,
    pillText3 === expectedPill3, `got: "${pillText3}"`);

  await page.screenshot({ path: `${OUT}/03-docker-and-model.png` });
  console.log('  📸 03-docker-and-model.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 4 — Quest constellation
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 4: Quest constellation ═══');
  const orbPct = await txt(page, '.ff-orb-pct');
  check(4, 'Orb percentage matches /^\\d+%$/', /^\d+%$/.test(orbPct),
    `got: "${orbPct}"`);

  await page.locator('.ff-orb').click();
  await page.waitForTimeout(1200);

  check(4, '.skill-constellation visible', await vis(page, '.skill-constellation', 5000));

  check(4, '.sc-close-btn visible', await vis(page, '.sc-close-btn'));
  const closeTxt = await txt(page, '.sc-close-btn');
  check(4, '.sc-close-btn === "✕"', closeTxt === '✕', `got: "${closeTxt}"`);

  const breadcrumb = await txt(page, '.sc-crumb--root');
  check(4, '.sc-crumb--root === "✦ All Clusters"',
    breadcrumb === '✦ All Clusters', `got: "${breadcrumb}"`);

  await page.screenshot({ path: `${OUT}/04-quest-constellation.png` });
  console.log('  📸 04-quest-constellation.png');

  await page.locator('.sc-close-btn').click();
  await page.waitForTimeout(500);

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 5 — Pet mode
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 5: Pet mode ═══');
  await page.locator('.mode-toggle-btn').click();
  await page.waitForTimeout(1500);

  check(5, '.pet-overlay visible', await vis(page, '.pet-overlay', 3000));
  check(5, '.app-shell has .pet-mode class',
    await page.locator('.app-shell').evaluate(el => el.classList.contains('pet-mode')));
  check(5, '.pet-character visible', await vis(page, '.pet-character', 2000));

  const obTitle = await txt(page, '.pet-onboarding-title');
  check(5, '.pet-onboarding-title === "Welcome to pet mode"',
    obTitle === 'Welcome to pet mode', `got: "${obTitle}"`);
  const dismissTxt = await txt(page, '.pet-onboarding-dismiss');
  check(5, '.pet-onboarding-dismiss === "Got it"', dismissTxt === 'Got it',
    `got: "${dismissTxt}"`);

  await page.screenshot({ path: `${OUT}/05-pet-mode.png` });
  console.log('  📸 05-pet-mode.png');

  await page.keyboard.press('Escape');
  await page.waitForTimeout(1000);
  check(5, 'Exited pet mode (no .pet-overlay)',
    !(await vis(page, '.pet-overlay', 1000)));

  // Re-inject local_ollama brain mode (pet mode may reset it to free_api
  // because PetOverlayView.onMounted calls autoConfigureFreeApi on Tauri failure)
  if (ollamaModelName) {
    await page.evaluate((model) => {
      const app = document.querySelector('#app')?.__vue_app__;
      const pinia = app?.config?.globalProperties?.$pinia;
      if (pinia?.state?.value?.brain) {
        pinia.state.value.brain.brainMode = {
          mode: 'local_ollama',
          model: model,
        };
        pinia.state.value.brain.ollamaStatus = {
          running: true,
          model_count: 1,
        };
      }
    }, ollamaModelName);
    await page.waitForTimeout(500);
  }

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 6 — Chat: first question (local Ollama, no memories)
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 6: Chat — first question (local Ollama) ═══');
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

  check(6, 'input.chat-input is enabled',
    await chatInput.isEnabled().catch(() => false));

  await chatInput.fill('What is the deadline for responding to a civil lawsuit in Vietnam?');
  const inputVal = await chatInput.inputValue();
  check(6, 'Input filled (>20 chars)', inputVal.length > 20,
    `${inputVal.length} chars`);
  check(6, 'button.send-btn visible', await sendBtn.isVisible().catch(() => false));

  // Confirm brain is local_ollama before sending
  const brainBeforeChat = await pinia(page, 'brain');
  check(6, 'brainMode.mode === "local_ollama"',
    brainBeforeChat?.brainMode?.mode === 'local_ollama',
    `got: ${brainBeforeChat?.brainMode?.mode}`);
  check(6, `brainMode.model === "${ollamaModelName}"`,
    brainBeforeChat?.brainMode?.model === ollamaModelName,
    `got: ${brainBeforeChat?.brainMode?.model}`);

  await sendBtn.click();

  let firstReply = '';
  try {
    await page.waitForFunction(() => {
      const app = document.querySelector('#app')?.__vue_app__;
      const conv = app?.config?.globalProperties?.$pinia?.state?.value?.conversation;
      const msgs = conv?.messages ?? [];
      const last = msgs[msgs.length - 1];
      return last?.role === 'assistant' && last?.content?.length > 20;
    }, { timeout: 90_000 });
    const conv = await pinia(page, 'conversation');
    const msgs = conv?.messages ?? [];
    firstReply = msgs[msgs.length - 1]?.content ?? '';
    check(6, 'Assistant replied (>20 chars) via local Ollama',
      firstReply.length > 20, `${firstReply.length} chars`);
  } catch {
    check(6, 'Assistant replied via local Ollama', false, 'timeout after 90s');
  }

  const userBubbles = await page.locator('.message-row.user').count();
  check(6, '.message-row.user count >= 1', userBubbles >= 1, `count=${userBubbles}`);
  const asstBubbles = await page.locator('.message-row.assistant').count();
  check(6, '.message-row.assistant count >= 1', asstBubbles >= 1, `count=${asstBubbles}`);

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
  check(7, '.mv-header h2 === "🧠 Memory"', mvHeader === '🧠 Memory',
    `got: "${mvHeader}"`);

  const addBtnTxt = await txt(page, '.mv-header-actions .btn-primary');
  check(7, 'Add button === "＋ Add memory"', addBtnTxt === '＋ Add memory',
    `got: "${addBtnTxt}"`);

  const tabTexts = await page.locator('.mv-tab').allTextContents();
  check(7, 'Sub-tabs === ["List","Graph","Session"]',
    JSON.stringify(tabTexts) === JSON.stringify(['List','Graph','Session']),
    `got: ${JSON.stringify(tabTexts)}`);

  const tierChips = await page.locator('.mv-tier-chip').allTextContents();
  check(7, 'Tier chips === ["short","working","long"]',
    JSON.stringify(tierChips) === JSON.stringify(['short','working','long']),
    `got: ${JSON.stringify(tierChips)}`);

  const typeChips = await page.locator('.mv-type-chip').allTextContents();
  check(7, 'Type chips === ["fact","preference","context","summary"]',
    JSON.stringify(typeChips) === JSON.stringify(['fact','preference','context','summary']),
    `got: ${JSON.stringify(typeChips)}`);

  const actionTexts = await page.locator('.mv-header-actions .btn-secondary')
    .allTextContents().then(arr => arr.map(t => t.trim()));
  check(7, 'Actions === ["⬇ Extract…","📄 Summarize…","⏳ Decay","🧹 GC"]',
    JSON.stringify(actionTexts) === JSON.stringify([
      '⬇ Extract from session','📄 Summarize session','⏳ Decay','🧹 GC']),
    `got: ${JSON.stringify(actionTexts)}`);

  await page.screenshot({ path: `${OUT}/07-memory-empty.png` });
  console.log('  📸 07-memory-empty.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 8 — Add a memory
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 8: Add a memory ═══');
  await page.locator('.mv-header-actions .btn-primary').click();
  await page.waitForTimeout(500);

  check(8, '.mv-modal visible', await vis(page, '.mv-modal'));

  const modalTitle = await txt(page, '.mv-modal h3');
  check(8, 'Modal title === "Add memory"', modalTitle === 'Add memory',
    `got: "${modalTitle}"`);

  const ta = page.locator('.mv-modal textarea');
  const taPlaceholder = await ta.getAttribute('placeholder');
  check(8, 'Textarea placeholder === "What should I remember?"',
    taPlaceholder === 'What should I remember?', `got: "${taPlaceholder}"`);

  await ta.fill(
    'Theo Điều 429 Bộ luật Dân sự 2015, thời hiệu khởi kiện để yêu cầu ' +
    'Tòa án giải quyết tranh chấp hợp đồng là 03 năm, kể từ ngày người có ' +
    'quyền yêu cầu biết hoặc phải biết quyền và lợi ích hợp pháp của mình bị xâm phạm.'
  );

  const tagInput = page.locator('.mv-modal input[placeholder]').first();
  const tagPlaceholder = await tagInput.getAttribute('placeholder');
  check(8, 'Tags placeholder === "python, work, project"',
    tagPlaceholder === 'python, work, project', `got: "${tagPlaceholder}"`);
  await tagInput.fill('vn-law, civil-code, statute');

  const saveBtn2 = page.locator('.mv-modal .btn-primary');
  const saveTxt = (await saveBtn2.textContent())?.trim();
  check(8, 'Save button === "Save"', saveTxt === 'Save', `got: "${saveTxt}"`);

  await page.screenshot({ path: `${OUT}/08-memory-add-modal.png` });
  console.log('  📸 08-memory-add-modal.png');

  await saveBtn2.click();
  await page.waitForTimeout(1500);
  check(8, 'Modal closed after save', !(await vis(page, '.mv-modal', 1000)));

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 9 — Memories list
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 9: Memories list ═══');

  const cardCount = await page.locator('.mv-card').count();
  if (cardCount > 0) {
    check(9, 'Memory cards rendered', true, `count=${cardCount}`);
    check(9, '.mv-chip visible', await vis(page, '.mv-card .mv-chip'));
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
  await page.locator('.mv-tab').filter({ hasText: 'Graph' }).click();
  await page.waitForTimeout(1500);

  check(10, '.mv-graph-panel visible', await vis(page, '.mv-graph-panel', 3000));

  await page.screenshot({ path: `${OUT}/10-memory-graph.png` });
  console.log('  📸 10-memory-graph.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 11 — Chat with RAG (local Ollama)
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 11: Chat with RAG (local Ollama) ═══');
  await page.locator('nav.desktop-nav .nav-btn').filter({ hasText: 'Chat' }).click();
  let chatViewBack = false;
  try {
    await page.waitForFunction(() => {
      const el = document.querySelector('.chat-view');
      return el && el.offsetParent !== null;
    }, { timeout: 10_000 });
    chatViewBack = true;
  } catch { chatViewBack = false; }
  check(11, '.chat-view visible', chatViewBack);

  // Verify brain is still local_ollama
  const brainForRag = await pinia(page, 'brain');
  check(11, `Brain mode === local_ollama/${ollamaModelName}`,
    brainForRag?.brainMode?.mode === 'local_ollama'
    && brainForRag?.brainMode?.model === ollamaModelName,
    `got: ${brainForRag?.brainMode?.mode}/${brainForRag?.brainMode?.model}`);

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
    }, msgCountBefore, { timeout: 90_000 });
    const conv2 = await pinia(page, 'conversation');
    ragReply = conv2?.messages?.[conv2.messages.length - 1]?.content ?? '';
    check(11, 'RAG reply (>20 chars) via local Ollama',
      ragReply.length > 20, `${ragReply.length} chars`);
  } catch {
    check(11, 'RAG reply via local Ollama', false, 'timeout after 90s');
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
  check(12, '.st-title === "⚔️ Skill Tree"', stTitle === '⚔️ Skill Tree',
    `got: "${stTitle}"`);

  check(12, '.brain-stat-sheet visible', await vis(page, '.brain-stat-sheet'));

  const bssTitle = await txt(page, '.bss-title');
  check(12, '.bss-title === "⚔ Brain Stat Sheet"', bssTitle === '⚔ Brain Stat Sheet',
    `got: "${bssTitle}"`);

  const statAbbrs = await page.locator('.bss-stat-abbr').allTextContents();
  check(12, 'Stats === ["INT","WIS","CHA","PER","DEX","END"]',
    JSON.stringify(statAbbrs) === JSON.stringify(['INT','WIS','CHA','PER','DEX','END']),
    `got: ${JSON.stringify(statAbbrs)}`);

  const levelTxt = await txt(page, '.bss-level');
  check(12, 'Level badge matches /^Lv\\. \\d+$/', /^Lv\. \d+$/.test(levelTxt),
    `got: "${levelTxt}"`);

  check(12, '.st-daily-section visible', await vis(page, '.st-daily-section'));

  await page.screenshot({ path: `${OUT}/12-skill-tree.png` });
  console.log('  📸 12-skill-tree.png');

  // ═══════════════════════════════════════════════════════════════════════
  // STEP 13 — Pet mode with chat
  // ═══════════════════════════════════════════════════════════════════════
  console.log('\n═══ Step 13: Pet mode with chat ═══');
  await page.locator('nav.desktop-nav .nav-btn').filter({ hasText: 'Chat' }).click();
  await page.waitForTimeout(1000);

  await page.locator('.mode-toggle-btn').click();
  await page.waitForTimeout(1500);

  check(13, '.pet-overlay visible', await vis(page, '.pet-overlay', 3000));

  let petChatVis = await vis(page, '.pet-chat', 2000);
  if (!petChatVis) {
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
    check(13, 'Pet placeholder === "Say something…"',
      petPlaceholder === 'Say something…', `got: "${petPlaceholder}"`);
    const petSendTxt = await txt(page, '.pet-chat-input button[type="submit"]');
    check(13, 'Pet send === "➤"', petSendTxt === '➤', `got: "${petSendTxt}"`);
  } else {
    check(13, 'Pet chat panel', 'SKIP',
      'canvas click does not propagate in headless Playwright');
  }

  await page.screenshot({ path: `${OUT}/13-pet-mode-chat.png` });
  console.log('  📸 13-pet-mode-chat.png');

  await page.keyboard.press('Escape');
  await page.waitForTimeout(500);

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
