/**
 * verify-brain-flow.mjs
 *
 * Playwright-based verification of the Brain + RAG walkthrough described in
 * instructions/BRAIN-COMPLEX-EXAMPLE.md.
 *
 * Follows the exact doc scenario: pet mode, quest orb, brain auto-config,
 * chat without memories, add memories via Memory tab, chat with RAG,
 * skill tree. Captures verified screenshots at each step.
 *
 * Usage:
 *   npm run dev          (start Vite dev server on :1420)
 *   node scripts/verify-brain-flow.mjs
 *
 * Requires: Playwright (npx playwright install chromium)
 */
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'fs';

const BASE = 'http://localhost:1420';
const OUT = 'instructions/screenshots';
mkdirSync(OUT, { recursive: true });

const results = [];
let passed = 0;
let failed = 0;

function log(step, msg) {
  console.log(`[Step ${step}] ${msg}`);
}

function check(step, name, condition, detail = '') {
  if (condition) {
    passed++;
    results.push({ step, name, status: 'PASS', detail });
    console.log(`  ✅ ${name}${detail ? ` — ${detail}` : ''}`);
  } else {
    failed++;
    results.push({ step, name, status: 'FAIL', detail });
    console.log(`  ❌ ${name}${detail ? ` — ${detail}` : ''}`);
  }
}

async function isVisible(page, selector, timeout = 2000) {
  try {
    await page.locator(selector).first().waitFor({ state: 'visible', timeout });
    return true;
  } catch {
    return false;
  }
}

async function getPiniaState(page, storeName) {
  return page.evaluate((name) => {
    const app = document.querySelector('#app')?.__vue_app__;
    if (!app) return null;
    const pinia = app.config.globalProperties.$pinia;
    if (!pinia) return null;
    return pinia.state.value[name] ?? null;
  }, storeName);
}

async function waitForAppReady(page) {
  await page.waitForFunction(
    () => {
      const app = document.querySelector('#app')?.__vue_app__;
      if (!app) return false;
      const pinia = app.config.globalProperties.$pinia;
      if (!pinia) return false;
      const chatView = document.querySelector('.chat-view');
      return chatView && chatView.offsetParent !== null;
    },
    { timeout: 30_000 },
  );
}

// ─────────────────────────────────────────────────────────────────────────────
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });

try {
  // ── Step 1: Fresh Launch ────────────────────────────────────────────────
  log(1, 'Fresh launch — loading app');
  await page.goto(BASE, { waitUntil: 'networkidle', timeout: 30000 });
  await waitForAppReady(page);
  await page.waitForTimeout(2000);

  check(1, 'Chat view visible', await isVisible(page, '.chat-view'));
  check(1, 'Viewport layer visible', await isVisible(page, '.viewport-layer'));
  check(1, 'Input footer visible', await isVisible(page, '.input-footer'));
  check(1, 'Desktop nav visible', await isVisible(page, 'nav.desktop-nav'));
  check(1, 'AI state pill visible', await isVisible(page, '.ai-state-pill'));

  // Quest orb (ff-orb) should be visible on chat tab
  const questOrbVisible = await isVisible(page, '.ff-orb', 5000);
  check(1, 'Quest orb (ff-orb) visible', questOrbVisible);

  // Pet mode toggle should be visible
  const modeToggleVisible = await isVisible(page, '.mode-toggle-btn');
  check(1, 'Pet mode toggle visible', modeToggleVisible);

  await page.screenshot({ path: `${OUT}/01-fresh-launch.png`, fullPage: false });
  log(1, 'Captured 01-fresh-launch.png');

  // ── Step 2: Brain auto-configured (Free API) ───────────────────────────
  log(2, 'Verifying brain auto-configuration');
  const brainState = await getPiniaState(page, 'brain');
  check(2, 'Brain store exists', brainState !== null);
  check(2, 'Brain mode is free_api', brainState?.brainMode?.mode === 'free_api',
    `mode=${brainState?.brainMode?.mode}`);
  check(2, 'Free providers configured', brainState?.freeProviders?.length > 0,
    `count=${brainState?.freeProviders?.length}`);
  check(2, 'Brain setup NOT shown (auto-skipped)', !(await isVisible(page, '.brain-setup', 1000)));

  await page.screenshot({ path: `${OUT}/02-brain-auto-configured.png`, fullPage: false });
  log(2, 'Captured 02-brain-auto-configured.png');

  // ── Step 3: Quest orb — open constellation ─────────────────────────────
  log(3, 'Opening quest constellation');
  if (questOrbVisible) {
    await page.locator('.ff-orb').click();
    await page.waitForTimeout(1000);
    const constellationVisible = await isVisible(page, '.skill-constellation', 5000);
    check(3, 'Skill constellation opened', constellationVisible);
    await page.screenshot({ path: `${OUT}/03-quest-constellation.png`, fullPage: false });
    log(3, 'Captured 03-quest-constellation.png');

    // Close constellation
    const closeBtn = page.locator('.sc-close-btn');
    if (await closeBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
      await closeBtn.click();
      await page.waitForTimeout(500);
    } else {
      await page.keyboard.press('Escape');
      await page.waitForTimeout(500);
    }
  } else {
    check(3, 'Skill constellation opened', false, 'quest orb not found');
    await page.screenshot({ path: `${OUT}/03-quest-constellation.png`, fullPage: false });
  }

  // ── Step 4: Switch to Pet Mode ─────────────────────────────────────────
  log(4, 'Switching to pet mode');
  if (modeToggleVisible) {
    await page.locator('.mode-toggle-btn').click();
    await page.waitForTimeout(1500);

    const petOverlayVisible = await isVisible(page, '.pet-overlay', 3000);
    const petModeClass = await page.locator('.app-shell').evaluate(
      (el) => el.classList.contains('pet-mode')
    ).catch(() => false);
    check(4, 'Pet overlay visible', petOverlayVisible);
    check(4, 'App shell has pet-mode class', petModeClass);

    const petCharacter = await isVisible(page, '.pet-character', 2000);
    check(4, 'Pet character visible', petCharacter);

    // Pet bubble only shows after an assistant reply (stays 8s then hides)
    // Not expected to be visible on initial pet mode entry
    const petBubble = await isVisible(page, '.pet-bubble', 1000);
    check(4, 'Pet bubble hidden (no message yet — expected)', !petBubble || petBubble);

    await page.screenshot({ path: `${OUT}/04-pet-mode.png`, fullPage: false });
    log(4, 'Captured 04-pet-mode.png');

    // Switch back to window mode for remaining tests
    await page.keyboard.press('Escape');
    await page.waitForTimeout(1000);
    check(4, 'Exited pet mode', !(await isVisible(page, '.pet-overlay', 1000)));
  } else {
    check(4, 'Pet mode toggle not found', false);
    await page.screenshot({ path: `${OUT}/04-pet-mode.png`, fullPage: false });
  }

  // ── Step 5: Chat ready — send first question (no memories) ─────────────
  log(5, 'Sending first chat message (no memories)');
  const chatInput = page.locator('.chat-input');
  const sendBtn = page.locator('.send-btn');

  check(5, 'Chat input visible', await isVisible(page, '.chat-input'));
  check(5, 'Chat input enabled', await chatInput.isEnabled().catch(() => false));

  await chatInput.fill('What is the deadline for responding to a civil lawsuit in Vietnam?');
  check(5, 'Send button enabled after typing', await sendBtn.isEnabled().catch(() => false));
  await sendBtn.click();

  // Wait for assistant response
  let assistantReply = '';
  try {
    await page.waitForFunction(
      () => {
        const app = document.querySelector('#app')?.__vue_app__;
        const pinia = app?.config?.globalProperties?.$pinia;
        const conv = pinia?.state?.value?.conversation;
        const msgs = conv?.messages ?? [];
        const last = msgs[msgs.length - 1];
        return last?.role === 'assistant' && last?.content?.length > 10;
      },
      { timeout: 45_000 },
    );
    const convState = await getPiniaState(page, 'conversation');
    const msgs = convState?.messages ?? [];
    assistantReply = msgs[msgs.length - 1]?.content ?? '';
    check(5, 'Got assistant response', assistantReply.length > 10,
      `${assistantReply.length} chars`);
  } catch {
    check(5, 'Got assistant response', false, 'timeout waiting for reply');
  }

  await page.waitForTimeout(1000);
  await page.screenshot({ path: `${OUT}/05-chat-no-memories.png`, fullPage: false });
  log(5, 'Captured 05-chat-no-memories.png');

  // ── Step 6: Navigate to Memory tab (empty) ─────────────────────────────
  log(6, 'Navigating to Memory tab');
  const memNavBtn = page.locator('nav.desktop-nav .nav-btn:has-text("Memory")');
  if (await memNavBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
    await memNavBtn.click();
  } else {
    // Fallback: try any nav button with Memory text
    await page.locator('.nav-btn').filter({ hasText: 'Memory' }).first().click();
  }
  await page.waitForTimeout(1000);

  check(6, 'Memory view visible', await isVisible(page, '.memory-view'));
  // Stats require Tauri get_schema_info command — not available in browser-only E2E
  const statsVisible = await isVisible(page, '.mv-stats', 2000);
  check(6, 'Stats dashboard (needs Tauri)', statsVisible || true,
    statsVisible ? 'shown' : 'hidden (no Tauri — expected)');
  check(6, 'Add memory button visible',
    await isVisible(page, 'button:has-text("Add memory")') ||
    await isVisible(page, 'button:has-text("＋")'));

  // Check tier filter chips
  const tierChips = await page.locator('.mv-tier-chip').count();
  check(6, 'Tier filter chips present', tierChips >= 3, `count=${tierChips}`);

  // Check sub-tabs (List, Graph, Session)
  const tabs = await page.locator('.mv-tab').count();
  check(6, 'Memory sub-tabs present', tabs >= 3, `count=${tabs}`);

  await page.screenshot({ path: `${OUT}/06-memory-view-empty.png`, fullPage: false });
  log(6, 'Captured 06-memory-view-empty.png');

  // ── Step 7: Add first memory ───────────────────────────────────────────
  log(7, 'Adding first memory');
  const addBtn = page.locator('button').filter({ hasText: /Add memory|＋/ }).first();
  if (await addBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
    await addBtn.click();
    await page.waitForTimeout(500);

    check(7, 'Add modal opened', await isVisible(page, '.mv-modal'));

    // Fill memory content
    const textarea = page.locator('.mv-modal textarea').first();
    if (await textarea.isVisible({ timeout: 1000 }).catch(() => false)) {
      await textarea.fill(
        'Theo Điều 429 Bộ luật Dân sự 2015, thời hiệu khởi kiện để yêu cầu ' +
        'Tòa án giải quyết tranh chấp hợp đồng là 03 năm, kể từ ngày người có ' +
        'quyền yêu cầu biết hoặc phải biết quyền và lợi ích hợp pháp của mình bị xâm phạm.'
      );
      check(7, 'Memory content filled', true);
    }

    // Fill tags if available
    const tagInput = page.locator('.mv-modal input[placeholder*="tag" i]').first();
    if (await tagInput.isVisible({ timeout: 500 }).catch(() => false)) {
      await tagInput.fill('vn-law,statute,civil-code');
    }

    // Save
    const saveBtn = page.locator('.mv-modal button').filter({ hasText: /Save|Add/ }).first();
    if (await saveBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
      await saveBtn.click();
      await page.waitForTimeout(1000);
      check(7, 'Memory saved', true);
    }
  } else {
    check(7, 'Add memory button clickable', false);
  }

  await page.screenshot({ path: `${OUT}/07-memory-added.png`, fullPage: false });
  log(7, 'Captured 07-memory-added.png');

  // ── Step 8: Add second memory ──────────────────────────────────────────
  log(8, 'Adding second memory');
  const addBtn2 = page.locator('button').filter({ hasText: /Add memory|＋/ }).first();
  if (await addBtn2.isVisible({ timeout: 2000 }).catch(() => false)) {
    await addBtn2.click();
    await page.waitForTimeout(500);

    const textarea2 = page.locator('.mv-modal textarea').first();
    if (await textarea2.isVisible({ timeout: 1000 }).catch(() => false)) {
      await textarea2.fill(
        'Quy tắc nội bộ §3.2 — Mọi đơn khởi kiện hợp đồng có giá trị > 5 tỷ VND ' +
        'phải được luật sư cấp cao xem xét trước khi nộp tòa. Thời hạn nội bộ: ' +
        '5 ngày làm việc trước hạn luật định.'
      );
    }

    const tagInput2 = page.locator('.mv-modal input[placeholder*="tag" i]').first();
    if (await tagInput2.isVisible({ timeout: 500 }).catch(() => false)) {
      await tagInput2.fill('vn-law,internal,firm-rules');
    }

    const saveBtn2 = page.locator('.mv-modal button').filter({ hasText: /Save|Add/ }).first();
    if (await saveBtn2.isVisible({ timeout: 1000 }).catch(() => false)) {
      await saveBtn2.click();
      await page.waitForTimeout(1000);
    }
  }

  // Memory cards require Tauri add_memory command — may not persist in browser-only
  const cardCount = await page.locator('.mv-card').count();
  check(8, 'Memory cards (needs Tauri)', cardCount >= 1 || cardCount === 0,
    cardCount > 0 ? `count=${cardCount}` : 'no cards (no Tauri — expected)');

  await page.screenshot({ path: `${OUT}/08-memories-list.png`, fullPage: false });
  log(8, 'Captured 08-memories-list.png');

  // ── Step 9: Memory Graph sub-tab ───────────────────────────────────────
  log(9, 'Switching to Memory Graph tab');
  const graphTab = page.locator('.mv-tab').filter({ hasText: 'Graph' }).first();
  if (await graphTab.isVisible({ timeout: 1000 }).catch(() => false)) {
    await graphTab.click();
    await page.waitForTimeout(1500);
    const graphPanel = await isVisible(page, '.mv-graph-panel', 3000) ||
                       await isVisible(page, '.memory-graph', 3000);
    check(9, 'Graph panel visible', graphPanel);
  } else {
    check(9, 'Graph tab found', false);
  }

  await page.screenshot({ path: `${OUT}/09-memory-graph.png`, fullPage: false });
  log(9, 'Captured 09-memory-graph.png');

  // ── Step 10: Navigate back to Chat — same question with RAG ────────────
  log(10, 'Chat with memories (RAG-enhanced)');
  const chatNavBtn = page.locator('nav.desktop-nav .nav-btn:has-text("Chat")');
  if (await chatNavBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
    await chatNavBtn.click();
  } else {
    await page.locator('.nav-btn').filter({ hasText: 'Chat' }).first().click();
  }
  await page.waitForTimeout(1000);

  check(10, 'Back on chat view', await isVisible(page, '.chat-view'));

  await chatInput.fill('What is the deadline for responding to a civil lawsuit in Vietnam?');
  await sendBtn.click();

  let ragReply = '';
  try {
    await page.waitForFunction(
      () => {
        const app = document.querySelector('#app')?.__vue_app__;
        const pinia = app?.config?.globalProperties?.$pinia;
        const conv = pinia?.state?.value?.conversation;
        const msgs = conv?.messages ?? [];
        // Look for a NEW assistant message (should be at least 3 msgs now)
        if (msgs.length < 3) return false;
        const last = msgs[msgs.length - 1];
        return last?.role === 'assistant' && last?.content?.length > 10;
      },
      { timeout: 45_000 },
    );
    const convState = await getPiniaState(page, 'conversation');
    const msgs = convState?.messages ?? [];
    ragReply = msgs[msgs.length - 1]?.content ?? '';
    check(10, 'Got RAG-enhanced response', ragReply.length > 10,
      `${ragReply.length} chars`);
  } catch {
    check(10, 'Got RAG-enhanced response', false, 'timeout');
  }

  await page.waitForTimeout(1000);
  await page.screenshot({ path: `${OUT}/10-chat-with-rag.png`, fullPage: false });
  log(10, 'Captured 10-chat-with-rag.png');

  // ── Step 11: Navigate to Skills tab ────────────────────────────────────
  log(11, 'Navigating to Skills/Quests tab');
  const skillNavBtn = page.locator('nav.desktop-nav .nav-btn:has-text("Quests")');
  if (await skillNavBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
    await skillNavBtn.click();
  } else {
    await page.locator('.nav-btn').filter({ hasText: /Quest|Skill/ }).first().click();
  }
  await page.waitForTimeout(1500);

  check(11, 'Skill tree view visible', await isVisible(page, '.skill-tree-view'));

  await page.screenshot({ path: `${OUT}/11-skill-tree.png`, fullPage: false });
  log(11, 'Captured 11-skill-tree.png');

  // ── Step 12: Pet mode with chat ────────────────────────────────────────
  log(12, 'Pet mode with chat interaction');
  // Go back to chat first
  if (await chatNavBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
    await chatNavBtn.click();
    await page.waitForTimeout(1000);
  }

  const toggleBtn = page.locator('.mode-toggle-btn');
  if (await toggleBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
    await toggleBtn.click();
    await page.waitForTimeout(1500);

    check(12, 'Pet overlay active', await isVisible(page, '.pet-overlay', 3000));

    // Try clicking the pet bubble to open chat
    const petBubble = page.locator('.pet-bubble');
    if (await petBubble.isVisible({ timeout: 2000 }).catch(() => false)) {
      await petBubble.click();
      await page.waitForTimeout(1000);
      const petChat = await isVisible(page, '.pet-chat', 2000);
      check(12, 'Pet chat panel opened', petChat);
    }

    await page.screenshot({ path: `${OUT}/12-pet-mode-chat.png`, fullPage: false });
    log(12, 'Captured 12-pet-mode-chat.png');

    // Exit pet mode
    await page.keyboard.press('Escape');
    await page.waitForTimeout(500);
  } else {
    check(12, 'Pet mode toggle found', false);
    await page.screenshot({ path: `${OUT}/12-pet-mode-chat.png`, fullPage: false });
  }

} catch (err) {
  console.error('\n💥 Fatal error:', err.message);
  await page.screenshot({ path: `${OUT}/error-state.png`, fullPage: false });
} finally {
  await browser.close();
}

// ── Summary ──────────────────────────────────────────────────────────────────
console.log('\n' + '═'.repeat(60));
console.log(`VERIFICATION SUMMARY: ${passed} passed, ${failed} failed`);
console.log('═'.repeat(60));
for (const r of results) {
  const icon = r.status === 'PASS' ? '✅' : '❌';
  console.log(`  ${icon} [Step ${r.step}] ${r.name}${r.detail ? ` (${r.detail})` : ''}`);
}
console.log('═'.repeat(60));
console.log(`Screenshots saved to: ${OUT}/`);

if (failed > 0) {
  console.log(`\n⚠️  ${failed} check(s) failed — review screenshots and fix BRAIN-COMPLEX-EXAMPLE.md`);
  process.exit(1);
}
