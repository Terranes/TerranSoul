/**
 * capture-brain-example-screenshots.mjs
 *
 * Captures all screenshots for BRAIN-COMPLEX-EXAMPLE.md with step-by-step
 * validation at each stage. Each screenshot asserts the expected DOM state
 * before capturing.
 *
 * Flow:
 *   01. Fresh launch
 *   02. Alice asks to learn Vietnamese
 *   03. Missing prerequisites prompt
 *   04. Auto-install progress
 *   05. Brain tab fully configured (asserts hero title, mode, config)
 *   06. Attach documents dialog
 *   07. Ingestion progress
 *   08. Memory tab with 15 entries
 *   09. RAG answer (English)
 *   10. Follow-up answer (English)
 *   11. Vietnamese Q&A
 *   12. Chinese Q&A
 *   13. Russian Q&A
 *   14. Japanese Q&A
 *   15. Korean Q&A
 *   16. Brain dashboard with RAG active
 *   17. Skill Tree
 *   18. Final state
 *
 * Usage:
 *   npm run dev           # terminal 1
 *   node scripts/capture-brain-example-screenshots.mjs
 */
import { chromium } from 'playwright';
import { mkdirSync } from 'fs';
import assert from 'assert';

const VITE_URL = 'http://localhost:1420';
const OUT = 'instructions/screenshots';
mkdirSync(OUT, { recursive: true });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const browser = await chromium.launch({ headless: true });
const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
const page = await ctx.newPage();

await page.goto(VITE_URL, { waitUntil: 'networkidle', timeout: 30000 });
await sleep(3000);

// ── Helpers ────────────────────────────────────────────────────────────────

async function setPinia(patch) {
  await page.evaluate((data) => {
    const app = document.querySelector('#app')?.__vue_app__;
    const pinia = app?.config?.globalProperties?.$pinia;
    if (!pinia) return;
    for (const [store, values] of Object.entries(data)) {
      const s = pinia._s.get(store);
      if (s) {
        // Use $patch for proper reactivity on Pinia store instances
        s.$patch(values);
      } else {
        // Fallback: create state directly
        if (!pinia.state.value[store]) pinia.state.value[store] = {};
        Object.assign(pinia.state.value[store], values);
      }
    }
  }, patch);
  await sleep(400);
}

async function setMessages(msgs) {
  await setPinia({
    conversation: {
      messages: msgs,
      isThinking: false,
      isStreaming: false,
      streamingText: '',
    },
  });
}

async function navigateTo(tabName) {
  // Force-click the nav button, bypassing any overlay (splash, combo toasts)
  const btn = page.locator(`button:has-text("${tabName}")`).first();
  try {
    await btn.click({ force: true, timeout: 3000 });
  } catch {
    // Fallback: dispatch click event via JS
    await page.evaluate((name) => {
      const btns = [...document.querySelectorAll('button')];
      const b = btns.find(b => b.textContent.includes(name));
      if (b) b.click();
    }, tabName);
  }
  await sleep(1000);
}

/** Assert text is visible on page. Throws if not found. */
async function assertVisible(text, timeout = 3000) {
  const found = await page.evaluate((t) => {
    return document.body.innerText.includes(t);
  }, text);
  if (!found) {
    throw new Error(`ASSERT FAILED: "${text}" not visible on page`);
  }
}

/** Assert a CSS selector is visible. */
async function assertSelector(selector, timeout = 3000) {
  try {
    await page.locator(selector).first().waitFor({ state: 'visible', timeout });
  } catch {
    throw new Error(`ASSERT FAILED: selector "${selector}" not visible`);
  }
}

async function screenshot(name) {
  await page.screenshot({ path: `${OUT}/${name}`, fullPage: false });
  console.log(`  ✅ ${name}`);
}

// ── Dismiss splash and any initial dialogs ────────────────────────────────
// Wait for splash to auto-dismiss (no Tauri backend → autoConfigureFreeApi → appLoading=false)
await sleep(2000);

const continueBtn = page.locator('button:has-text("Continue ▸")');
while (await continueBtn.isVisible({ timeout: 500 }).catch(() => false)) {
  await continueBtn.click();
  await sleep(500);
}
const skipBtn = page.locator('button:has-text("Skip")');
if (await skipBtn.isVisible({ timeout: 500 }).catch(() => false)) {
  await skipBtn.click();
  await sleep(500);
}

// Dismiss any combo notifications
const dismissBtns = page.locator('button[aria-label*="Dismiss"], button:has-text("✕")');
const dismissCount = await dismissBtns.count();
for (let i = 0; i < dismissCount; i++) {
  try { await dismissBtns.nth(i).click({ force: true, timeout: 500 }); } catch { /* ok */ }
  await sleep(200);
}

const NOW = Date.now();

// ── Brain state preset (correct BrainMode object format) ─────────────────
const BRAIN_FREE_STATE = {
  brain: {
    brainMode: {
      mode: 'free_api',
      provider_id: 'pollinations',
      api_key: null,
    },
    hasBrain: true,
    activeBrain: null,
    isLoading: false,
    ollamaStatus: { running: false, model_count: 0 },
    freeProviders: [
      {
        id: 'pollinations', display_name: 'Pollinations AI',
        base_url: 'https://text.pollinations.ai/openai', model: 'openai',
        rpm_limit: 30, rpd_limit: 0, requires_api_key: false,
        notes: 'Free, no API key needed',
      },
      {
        id: 'groq', display_name: 'Groq',
        base_url: 'https://api.groq.com/openai', model: 'llama-3.3-70b-versatile',
        rpm_limit: 30, rpd_limit: 1000, requires_api_key: true,
        notes: 'Fast inference, free tier',
      },
    ],
    systemInfo: {
      total_ram_mb: 32768, ram_tier_label: 'Large (32 GB)',
      cpu_cores: 12, cpu_name: 'Intel Core i7-12700K',
      os_name: 'Windows 11', arch: 'x86_64',
      gpu_name: 'NVIDIA RTX 3080',
    },
  },
};

// ── Sample memories for memory tab ───────────────────────────────────────
const SAMPLE_MEMORIES = [
  { id: 1, content: 'Article 429: The statute of limitations for filing lawsuits related to contractual disputes is three years, from the date the claimant knew or should have known that their rights were infringed.', tags: 'vietnamese-law,contract,statute-of-limitations', importance: 5, memory_type: 'fact', created_at: NOW - 60000, last_accessed: NOW - 5000, access_count: 3, tier: 'long', decay_score: 0.95, session_id: null, parent_id: null, token_count: 52 },
  { id: 2, content: 'Article 351: A party that fails to perform or improperly performs a civil obligation shall bear civil liability. Liability for breach is strict — the aggrieved party need not prove fault.', tags: 'vietnamese-law,liability,breach', importance: 5, memory_type: 'fact', created_at: NOW - 59000, last_accessed: NOW - 8000, access_count: 2, tier: 'long', decay_score: 0.92, session_id: null, parent_id: null, token_count: 45 },
  { id: 3, content: 'Article 352: A party causing damage through breach of a civil obligation shall compensate for all the damage, unless otherwise agreed or prescribed by law.', tags: 'vietnamese-law,damages,compensation', importance: 5, memory_type: 'fact', created_at: NOW - 58000, last_accessed: null, access_count: 0, tier: 'long', decay_score: 0.90, session_id: null, parent_id: null, token_count: 38 },
  { id: 4, content: 'Article 360: In case of breach of a contractual obligation, the aggrieved party may claim compensation for the interests it would have derived from performance.', tags: 'vietnamese-law,contract,damages', importance: 5, memory_type: 'fact', created_at: NOW - 57000, last_accessed: null, access_count: 0, tier: 'long', decay_score: 0.89, session_id: null, parent_id: null, token_count: 42 },
  { id: 5, content: 'Article 419: Compensation for damage due to breach includes material and spiritual loss actually suffered, including lost benefits.', tags: 'vietnamese-law,compensation,breach', importance: 5, memory_type: 'fact', created_at: NOW - 56000, last_accessed: null, access_count: 0, tier: 'long', decay_score: 0.88, session_id: null, parent_id: null, token_count: 44 },
  { id: 6, content: 'Article 420: Parties may agree on a penalty for breach. If no agreement on relationship between penalty and compensation, the aggrieved party may claim both.', tags: 'vietnamese-law,penalty,contract', importance: 5, memory_type: 'fact', created_at: NOW - 55000, last_accessed: null, access_count: 0, tier: 'long', decay_score: 0.87, session_id: null, parent_id: null, token_count: 40 },
  { id: 7, content: 'Article 421: Exemption from civil liability for breach in cases of force majeure, entirely the fault of the aggrieved party, or other grounds as agreed by parties.', tags: 'vietnamese-law,exemption,force-majeure', importance: 4, memory_type: 'fact', created_at: NOW - 54000, last_accessed: null, access_count: 0, tier: 'long', decay_score: 0.86, session_id: null, parent_id: null, token_count: 38 },
  { id: 8, content: 'Article 468: Default interest rate for overdue payment is 10% per year when parties have no agreement.', tags: 'vietnamese-law,interest,payment', importance: 4, memory_type: 'fact', created_at: NOW - 53000, last_accessed: null, access_count: 0, tier: 'long', decay_score: 0.85, session_id: null, parent_id: null, token_count: 30 },
  { id: 9, content: 'Commentary: Article 429 "should have known" standard — courts may determine a party should have known of a breach even if not actually discovered.', tags: 'vietnamese-law,statute-of-limitations,commentary', importance: 4, memory_type: 'fact', created_at: NOW - 52000, last_accessed: null, access_count: 0, tier: 'long', decay_score: 0.84, session_id: null, parent_id: null, token_count: 48 },
  { id: 10, content: 'Commentary: Statute of limitations may be suspended during force majeure (Article 156) or when entitled person is a minor without a legal representative.', tags: 'vietnamese-law,tolling,commentary', importance: 4, memory_type: 'fact', created_at: NOW - 51000, last_accessed: null, access_count: 0, tier: 'long', decay_score: 0.83, session_id: null, parent_id: null, token_count: 35 },
  { id: 11, content: 'Article 385-390: Formation of contracts requires offer, acceptance, and meeting of minds. Contracts may be in writing, verbal, or implied by conduct.', tags: 'vietnamese-law,contract-formation', importance: 5, memory_type: 'fact', created_at: NOW - 50000, last_accessed: null, access_count: 0, tier: 'long', decay_score: 0.82, session_id: null, parent_id: null, token_count: 36 },
  { id: 12, content: 'Article 398: Modification of a contract requires agreement of all parties. Unilateral modification is only allowed when prescribed by law.', tags: 'vietnamese-law,contract-modification', importance: 4, memory_type: 'fact', created_at: NOW - 49000, last_accessed: null, access_count: 0, tier: 'long', decay_score: 0.81, session_id: null, parent_id: null, token_count: 32 },
  { id: 13, content: 'Alice is a law student studying Vietnamese civil code, focusing on contract law Articles 385–429.', tags: 'personal,auto-extracted', importance: 3, memory_type: 'fact', created_at: NOW - 45000, last_accessed: NOW - 3000, access_count: 5, tier: 'long', decay_score: 0.97, session_id: null, parent_id: null, token_count: 20 },
  { id: 14, content: 'Alice prefers concise explanations with specific article citations.', tags: 'personal,preference,auto-extracted', importance: 3, memory_type: 'preference', created_at: NOW - 44000, last_accessed: NOW - 4000, access_count: 3, tier: 'long', decay_score: 0.94, session_id: null, parent_id: null, token_count: 12 },
  { id: 15, content: 'Alice studies at a Vietnamese law school and frequently asks about breach of contract provisions.', tags: 'personal,auto-extracted', importance: 3, memory_type: 'fact', created_at: NOW - 43000, last_accessed: NOW - 2000, access_count: 2, tier: 'long', decay_score: 0.93, session_id: null, parent_id: null, token_count: 18 },
];

const MEMORY_STATE = {
  memory: {
    memories: SAMPLE_MEMORIES,
    isLoading: false,
    error: null,
    stats: {
      total: 15, short_count: 0, working_count: 0, long_count: 15,
      total_tokens: 550, avg_decay: 0.88,
    },
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// 01 — Fresh Launch
// ═══════════════════════════════════════════════════════════════════════════
console.log('01 — Fresh Launch');
await setPinia({ settings: { firstLaunchDone: true, hasCompletedSetup: true }, ...BRAIN_FREE_STATE });
await navigateTo('Chat');
await sleep(500);
await assertSelector('.app-nav');
await screenshot('01-fresh-launch.png');

// ═══════════════════════════════════════════════════════════════════════════
// 02 — Alice asks to learn Vietnamese
// ═══════════════════════════════════════════════════════════════════════════
console.log('02 — Alice asks to learn Vietnamese');
await setMessages([
  { id: 'u1', role: 'user', content: 'Learn Vietnamese laws using my provided documents', timestamp: NOW - 60000 },
]);
await sleep(600);
await assertVisible('Learn Vietnamese laws');
await screenshot('02-alice-learn-request.png');

// ═══════════════════════════════════════════════════════════════════════════
// 03 — Missing prerequisites + 3 buttons
// ═══════════════════════════════════════════════════════════════════════════
console.log('03 — Missing prerequisites prompt');
await setMessages([
  { id: 'u1', role: 'user', content: 'Learn Vietnamese laws using my provided documents', timestamp: NOW - 60000 },
  {
    id: 'sys-missing', role: 'assistant',
    content: 'To learn **Vietnamese laws** from your documents I need a few quests to be active first:\n\n'
      + '• 🧠 **Awaken the Mind** — Connect to a free cloud AI\n'
      + '• 📖 **Long-Term Memory** — Persistent memory across sessions\n'
      + '• 📚 **Sage\'s Library** — Local semantic-search RAG\n'
      + '• 📚 **Scholar\'s Quest** — Document ingestion pipeline\n\n'
      + 'How would you like to proceed?',
    agentName: 'System', sentiment: 'neutral', timestamp: NOW - 59000,
    questId: 'learn-docs-missing',
    questChoices: [
      { label: 'Install all', value: 'learn-docs:install-all:Vietnamese%20laws', icon: '⚡' },
      { label: 'Install one by one', value: 'learn-docs:install-each:Vietnamese%20laws', icon: '📋' },
      { label: 'Cancel', value: 'dismiss', icon: '❌' },
    ],
  },
]);
await sleep(600);
await assertVisible('Awaken the Mind');
await assertVisible('Install all');
await assertVisible('Cancel');
await screenshot('03-missing-prereqs.png');

// ═══════════════════════════════════════════════════════════════════════════
// 04 — Auto-install progress
// ═══════════════════════════════════════════════════════════════════════════
console.log('04 — Auto-install progress');
await setMessages([
  { id: 'u1', role: 'user', content: 'Learn Vietnamese laws using my provided documents', timestamp: NOW - 60000 },
  { id: 'sys-auto', role: 'assistant', content: '⚡ Auto-installing all required quests…', agentName: 'System', sentiment: 'happy', timestamp: NOW - 55000 },
  { id: 'q1', role: 'assistant', content: '✅ **Awaken the Mind** activated — Free cloud AI connected (Pollinations / Groq)', agentName: 'System', sentiment: 'happy', timestamp: NOW - 54000 },
  { id: 'q2', role: 'assistant', content: '✅ **Long-Term Memory** activated — SQLite memory store online', agentName: 'System', sentiment: 'happy', timestamp: NOW - 53000 },
  { id: 'q3', role: 'assistant', content: '✅ **Sage\'s Library** activated — 6-signal hybrid RAG pipeline ready', agentName: 'System', sentiment: 'happy', timestamp: NOW - 52000 },
  { id: 'q4', role: 'assistant', content: '✅ **Scholar\'s Quest** activated — document ingestion unlocked', agentName: 'System', sentiment: 'happy', timestamp: NOW - 51000 },
  {
    id: 'ready', role: 'assistant',
    content: '🎉 All 4 quests installed! Your brain is fully configured:\n\n'
      + '- **Brain:** Free cloud AI (auto-rotating providers)\n'
      + '- **Memory:** SQLite long-term store\n'
      + '- **RAG:** Hybrid 6-signal search (vector + keyword + recency + importance + decay + tier)\n'
      + '- **Ingestion:** Semantic chunking + embedding pipeline\n\n'
      + 'Ready to import your Vietnamese law documents!',
    agentName: 'TerranSoul', sentiment: 'happy', timestamp: NOW - 50000,
    questId: 'scholar-quest-ready',
    questChoices: [
      { label: 'Start Knowledge Quest', value: 'knowledge-quest-start', icon: '⚔️' },
      { label: 'No thanks', value: 'dismiss', icon: '💤' },
    ],
  },
]);
await sleep(600);
await assertVisible('Awaken the Mind');
await assertVisible('Sage\'s Library');
await assertVisible('All 4 quests installed');
await assertVisible('Start Knowledge Quest');
await screenshot('04-auto-install.png');

// ═══════════════════════════════════════════════════════════════════════════
// 05 — Brain tab fully configured
// ═══════════════════════════════════════════════════════════════════════════
console.log('05 — Brain tab fully configured');
await navigateTo('Brain');
// Ensure brain + memory state is set correctly
await setPinia({
  ...BRAIN_FREE_STATE,
  ...MEMORY_STATE,
});
await sleep(1200);

// ASSERT: the brain page shows "alive" not "No brain configured"
await assertVisible('Your brain is alive');
await assertSelector('[data-testid="bv-mode-switcher"]');
await assertSelector('[data-testid="bv-card-config"]');
await assertSelector('[data-testid="bv-card-hardware"]');
await assertSelector('[data-testid="bv-card-memory"]');
// Assert config card content
await assertVisible('Configuration');
await assertVisible('Free Cloud API');
await assertVisible('Pollinations AI');
// Assert hardware card
await assertVisible('Hardware');
await assertVisible('Intel Core i7-12700K');
await assertVisible('NVIDIA RTX 3080');
// Assert memory health
await assertVisible('Memory health');

console.log('  ✓ All brain page assertions passed');
await screenshot('05-brain-configured.png');

// ═══════════════════════════════════════════════════════════════════════════
// 06 — Attach documents (Knowledge Quest dialog)
// ═══════════════════════════════════════════════════════════════════════════
console.log('06 — Attach documents dialog');
await navigateTo('Chat');
await setMessages([
  { id: 'u1', role: 'user', content: 'Learn Vietnamese laws using my provided documents', timestamp: NOW - 60000 },
  {
    id: 'attach-prompt', role: 'assistant',
    content: '📚 **Scholar\'s Quest — Step 1: Gather Sources**\n\n'
      + 'Add URLs or local files with your Vietnamese law content.\n'
      + 'Supported formats: `.md`, `.txt`, `.csv`, `.json`, `.xml`, `.html`, `.pdf`\n\n'
      + '**Sources added:**\n'
      + '1. 📄 `vietnamese-civil-code.html` — Articles 351–468 (Liability + Contracts)\n'
      + '2. 📄 `article-429-commentary.txt` — Statute of Limitations commentary\n\n'
      + 'Click **⚡ Start Learning** when ready.',
    agentName: 'TerranSoul', sentiment: 'happy', timestamp: NOW - 45000,
    questId: 'scholar-quest-sources',
    questChoices: [
      { label: '⚡ Start Learning', value: 'knowledge-quest-ingest', icon: '📚' },
      { label: 'Add more sources', value: 'knowledge-quest-add', icon: '➕' },
      { label: 'Cancel', value: 'dismiss', icon: '❌' },
    ],
  },
]);
await sleep(600);
await assertVisible('Gather Sources');
await assertVisible('vietnamese-civil-code.html');
await assertVisible('article-429-commentary.txt');
await assertVisible('Start Learning');
await screenshot('06-attach-documents.png');

// ═══════════════════════════════════════════════════════════════════════════
// 07 — Ingestion progress
// ═══════════════════════════════════════════════════════════════════════════
console.log('07 — Ingestion progress');
await setMessages([
  { id: 'u1', role: 'user', content: 'Learn Vietnamese laws using my provided documents', timestamp: NOW - 60000 },
  {
    id: 'ingest-progress', role: 'assistant',
    content: '📚 **Scholar\'s Quest — Step 2: Learning in Progress**\n\n'
      + '**Source 1:** `vietnamese-civil-code.html`\n'
      + '  ✅ Fetched (Articles 351–468)\n'
      + '  ✅ Chunked → 12 semantic pieces (~500–800 tokens each)\n'
      + '  ✅ Embedded → 12/12 vectors (cloud `/v1/embeddings`)\n'
      + '  ✅ Stored → 12 long-term memories with tags `vietnamese-law,contract`\n\n'
      + '**Source 2:** `article-429-commentary.txt`\n'
      + '  ✅ Read file\n'
      + '  ✅ Chunked → 3 semantic pieces\n'
      + '  ✅ Embedded → 3/3 vectors\n'
      + '  ✅ Stored → 3 long-term memories with tags `vietnamese-law,statute-of-limitations`\n\n'
      + '**Result:** 15 memories created · 15 embedded · 0 duplicates skipped\n'
      + 'Your Vietnamese law knowledge base is ready! 🎉',
    agentName: 'TerranSoul', sentiment: 'happy', timestamp: NOW - 40000,
  },
]);
await sleep(600);
await assertVisible('Learning in Progress');
await assertVisible('15 memories created');
await screenshot('07-ingestion-progress.png');

// ═══════════════════════════════════════════════════════════════════════════
// 08 — Memory tab with 15 entries
// ═══════════════════════════════════════════════════════════════════════════
console.log('08 — Memory tab with knowledge entries');
await navigateTo('Memory');
await setPinia(MEMORY_STATE);
await sleep(600);
await assertVisible('Memory');
await assertVisible('Article 429');
await assertVisible('Article 351');
await screenshot('08-memory-tab.png');

// ═══════════════════════════════════════════════════════════════════════════
// 09 — RAG answer (English)
// ═══════════════════════════════════════════════════════════════════════════
console.log('09 — RAG law answer (English)');
await navigateTo('Chat');
await setMessages([
  { id: 'u-rag', role: 'user', content: 'What is the statute of limitations for contract disputes under Vietnamese law?', timestamp: NOW - 10000 },
  {
    id: 'a-rag', role: 'assistant',
    content: 'Based on your imported Vietnamese Civil Code documents:\n\n'
      + '**Article 429** of the 2015 Civil Code (Law No. 91/2015/QH13) sets the statute of limitations for contractual disputes at **three (3) years**, running from the date the claimant "knew or should have known" that their lawful rights and interests were infringed.\n\n'
      + '**Key details from your sources:**\n\n'
      + '1. **Scope:** Applies to all contractual disputes — sales, services, leases, loans\n'
      + '2. **"Should have known" standard:** Courts may determine knowledge constructively (Article 429 Commentary)\n'
      + '3. **Tolling:** Suspended during force majeure (Article 156) or when the claimant is a minor without legal representation\n'
      + '4. **Related:** Strict liability under Article 351 means the aggrieved party need not prove fault\n\n'
      + '> 📚 **Sources:** `vietnamese-civil-code.html` (Articles 351, 429), `article-429-commentary.txt`',
    agentName: 'TerranSoul', sentiment: 'neutral', timestamp: NOW - 5000,
  },
]);
await sleep(600);
await assertVisible('Article 429');
await assertVisible('three (3) years');
await assertVisible('Sources');
await screenshot('09-rag-answer.png');

// ═══════════════════════════════════════════════════════════════════════════
// 10 — Follow-up answer (English)
// ═══════════════════════════════════════════════════════════════════════════
console.log('10 — Follow-up answer (English)');
await setMessages([
  { id: 'u-rag', role: 'user', content: 'What is the statute of limitations for contract disputes?', timestamp: NOW - 30000 },
  { id: 'a-rag', role: 'assistant', content: 'Article 429 sets the limitation at **three years** from the date the claimant knew of the breach.\n\n> 📚 Sources: `vietnamese-civil-code.html`, `article-429-commentary.txt`', agentName: 'TerranSoul', sentiment: 'neutral', timestamp: NOW - 25000 },
  { id: 'u2', role: 'user', content: 'Can a party claim both a penalty and damages for breach of contract?', timestamp: NOW - 20000 },
  {
    id: 'a2', role: 'assistant',
    content: 'Yes! Under **Article 420** of the Vietnamese Civil Code 2015:\n\n'
      + '1. Parties may agree on a **penalty clause** for breach of contract\n'
      + '2. The penalty amount is freely negotiated between parties\n'
      + '3. **If no agreement exists** on the relationship between penalty and compensation, the aggrieved party **may claim both** the penalty AND full compensation\n\n'
      + 'Related: Under Article 419, compensation covers both **material** and **spiritual** losses, including lost benefits.\n\n'
      + '> 📚 **Sources:** `vietnamese-civil-code.html` (Articles 419, 420)',
    agentName: 'TerranSoul', sentiment: 'neutral', timestamp: NOW - 15000,
  },
]);
await sleep(600);
await assertVisible('Article 420');
await assertVisible('may claim both');
await screenshot('10-followup-answer.png');

// ═══════════════════════════════════════════════════════════════════════════
// 11 — Vietnamese Q&A
// ═══════════════════════════════════════════════════════════════════════════
console.log('11 — Vietnamese Q&A');
await setMessages([
  { id: 'u-vi', role: 'user', content: 'Thời hiệu khởi kiện tranh chấp hợp đồng theo pháp luật Việt Nam là bao lâu?', timestamp: NOW - 10000 },
  {
    id: 'a-vi', role: 'assistant',
    content: 'Theo tài liệu Bộ luật Dân sự Việt Nam của bạn:\n\n'
      + '**Điều 429** Bộ luật Dân sự 2015 (Luật số 91/2015/QH13) quy định thời hiệu khởi kiện để yêu cầu giải quyết tranh chấp hợp đồng là **ba (3) năm**, kể từ ngày người có quyền yêu cầu "biết hoặc phải biết" quyền và lợi ích hợp pháp của mình bị xâm phạm.\n\n'
      + '**Chi tiết quan trọng:**\n'
      + '1. **Phạm vi:** Áp dụng cho mọi tranh chấp hợp đồng — mua bán, dịch vụ, thuê, vay\n'
      + '2. **Tiêu chuẩn "phải biết":** Tòa án có thể xác định một bên phải biết về vi phạm (Bình luận Điều 429)\n'
      + '3. **Tạm ngừng:** Tạm dừng trong trường hợp bất khả kháng (Điều 156) hoặc khi người có quyền là người chưa thành niên\n\n'
      + '> 📚 **Nguồn:** `vietnamese-civil-code.html` (Điều 351, 429), `article-429-commentary.txt`',
    agentName: 'TerranSoul', sentiment: 'neutral', timestamp: NOW - 5000,
  },
]);
await sleep(600);
await assertVisible('Điều 429');
await assertVisible('ba (3) năm');
await screenshot('11-vietnamese-answer.png');

// ═══════════════════════════════════════════════════════════════════════════
// 12 — Chinese Q&A
// ═══════════════════════════════════════════════════════════════════════════
console.log('12 — Chinese Q&A');
await setMessages([
  { id: 'u-zh', role: 'user', content: '越南法律中合同纠纷的诉讼时效是多长？', timestamp: NOW - 10000 },
  {
    id: 'a-zh', role: 'assistant',
    content: '根据您导入的越南民法典文件：\n\n'
      + '**第429条**（2015年民法典，法律编号91/2015/QH13）规定合同纠纷的诉讼时效为**三（3）年**，从权利人"知道或应当知道"其合法权益受到侵害之日起计算。\n\n'
      + '**关键细节：**\n'
      + '1. **适用范围：** 适用于所有合同纠纷——买卖、服务、租赁、贷款\n'
      + '2. **"应当知道"标准：** 法院可推定当事人应当知道违约行为（第429条评论）\n'
      + '3. **中止：** 在不可抗力（第156条）或权利人为无法定代理人的未成年人时暂停\n\n'
      + '> 📚 **来源：** `vietnamese-civil-code.html`（第351、429条），`article-429-commentary.txt`',
    agentName: 'TerranSoul', sentiment: 'neutral', timestamp: NOW - 5000,
  },
]);
await sleep(600);
await assertVisible('第429条');
await assertVisible('三（3）年');
await screenshot('12-chinese-answer.png');

// ═══════════════════════════════════════════════════════════════════════════
// 13 — Russian Q&A
// ═══════════════════════════════════════════════════════════════════════════
console.log('13 — Russian Q&A');
await setMessages([
  { id: 'u-ru', role: 'user', content: 'Каков срок исковой давности по договорным спорам по вьетнамскому праву?', timestamp: NOW - 10000 },
  {
    id: 'a-ru', role: 'assistant',
    content: 'Согласно импортированным документам Гражданского кодекса Вьетнама:\n\n'
      + '**Статья 429** Гражданского кодекса 2015 года (Закон № 91/2015/QH13) устанавливает срок исковой давности по договорным спорам в **три (3) года** с даты, когда истец "узнал или должен был узнать" о нарушении своих прав.\n\n'
      + '**Ключевые детали:**\n'
      + '1. **Область применения:** Все договорные споры — купля-продажа, услуги, аренда, заём\n'
      + '2. **Стандарт "должен был знать":** Суд может определить осведомлённость конструктивно\n'
      + '3. **Приостановление:** При форс-мажоре (ст. 156) или когда истец — несовершеннолетний без представителя\n\n'
      + '> 📚 **Источники:** `vietnamese-civil-code.html` (ст. 351, 429), `article-429-commentary.txt`',
    agentName: 'TerranSoul', sentiment: 'neutral', timestamp: NOW - 5000,
  },
]);
await sleep(600);
await assertVisible('Статья 429');
await assertVisible('три (3) года');
await screenshot('13-russian-answer.png');

// ═══════════════════════════════════════════════════════════════════════════
// 14 — Japanese Q&A
// ═══════════════════════════════════════════════════════════════════════════
console.log('14 — Japanese Q&A');
await setMessages([
  { id: 'u-ja', role: 'user', content: 'ベトナム法における契約紛争の出訴期限はどのくらいですか？', timestamp: NOW - 10000 },
  {
    id: 'a-ja', role: 'assistant',
    content: 'インポートされたベトナム民法典の文書に基づいて：\n\n'
      + '**第429条**（2015年民法典、法律番号91/2015/QH13）は、契約紛争の出訴期限を**3年間**と定めています。権利者が権利侵害を「知った、または知るべきであった」日から起算されます。\n\n'
      + '**重要な詳細：**\n'
      + '1. **適用範囲：** すべての契約紛争 — 売買、サービス、賃貸、貸付\n'
      + '2. **「知るべきであった」基準：** 裁判所は推定的認識を認定可能（第429条解説）\n'
      + '3. **停止：** 不可抗力（第156条）または権利者が法定代理人のない未成年者の場合\n\n'
      + '> 📚 **出典：** `vietnamese-civil-code.html`（第351条、429条）、`article-429-commentary.txt`',
    agentName: 'TerranSoul', sentiment: 'neutral', timestamp: NOW - 5000,
  },
]);
await sleep(600);
await assertVisible('第429条');
await assertVisible('3年間');
await screenshot('14-japanese-answer.png');

// ═══════════════════════════════════════════════════════════════════════════
// 15 — Korean Q&A
// ═══════════════════════════════════════════════════════════════════════════
console.log('15 — Korean Q&A');
await setMessages([
  { id: 'u-ko', role: 'user', content: '베트남 법률에서 계약 분쟁의 소멸시효는 얼마입니까?', timestamp: NOW - 10000 },
  {
    id: 'a-ko', role: 'assistant',
    content: '가져온 베트남 민법전 문서를 기반으로:\n\n'
      + '**제429조** (2015년 민법전, 법률 번호 91/2015/QH13)는 계약 분쟁의 소멸시효를 **3년**으로 규정하고 있으며, 권리자가 자신의 합법적 권리가 침해된 것을 "알았거나 알았어야 하는" 날부터 기산합니다.\n\n'
      + '**주요 세부사항:**\n'
      + '1. **적용 범위:** 모든 계약 분쟁 — 매매, 서비스, 임대, 대출\n'
      + '2. **"알았어야 하는" 기준:** 법원은 추정적 인식을 인정할 수 있음 (제429조 해설)\n'
      + '3. **정지:** 불가항력 (제156조) 또는 법정대리인이 없는 미성년자인 경우\n\n'
      + '> 📚 **출처:** `vietnamese-civil-code.html` (제351조, 429조), `article-429-commentary.txt`',
    agentName: 'TerranSoul', sentiment: 'neutral', timestamp: NOW - 5000,
  },
]);
await sleep(600);
await assertVisible('제429조');
await assertVisible('3년');
await screenshot('15-korean-answer.png');

// ═══════════════════════════════════════════════════════════════════════════
// 16 — Brain dashboard with RAG active
// ═══════════════════════════════════════════════════════════════════════════
console.log('16 — Brain tab with RAG pipeline active');
await navigateTo('Brain');
await setPinia({ ...BRAIN_FREE_STATE, ...MEMORY_STATE });
await sleep(800);
await assertVisible('Your brain is alive');
await assertVisible('15');
await screenshot('16-brain-rag-active.png');

// ═══════════════════════════════════════════════════════════════════════════
// 17 — Skill Tree
// ═══════════════════════════════════════════════════════════════════════════
console.log('17 — Skill Tree');
await navigateTo('Quests');
await sleep(1000);
await screenshot('17-skill-tree.png');

// ═══════════════════════════════════════════════════════════════════════════
// 18 — Final state
// ═══════════════════════════════════════════════════════════════════════════
console.log('18 — Final state');
await navigateTo('Chat');
await setMessages([
  { id: 'u-final', role: 'user', content: 'Summarize what you know about me and my documents.', timestamp: NOW - 5000 },
  {
    id: 'a-final', role: 'assistant',
    content: "Here's what I know about you and your study materials:\n\n"
      + '**About you:**\n'
      + "- You're a **law student** studying Vietnamese civil code\n"
      + '- Your focus is **contract law**, Articles 385–429\n'
      + '- You prefer **concise explanations** with specific article citations\n\n'
      + '**Your knowledge base (15 memories):**\n'
      + '- 12 chunks from the Vietnamese Civil Code (Articles 351–468)\n'
      + '- 3 auto-extracted personal facts from our conversation\n\n'
      + '**Multilingual capability:**\n'
      + '- Answers verified in English, Vietnamese, Chinese, Russian, Japanese, and Korean\n'
      + '- Same RAG pipeline retrieves the correct sources regardless of query language\n\n'
      + 'All indexed for instant RAG retrieval in every future conversation. 🧠',
    agentName: 'TerranSoul', sentiment: 'happy', timestamp: NOW - 2000,
    motion: 'greeting',
  },
]);
await sleep(600);
await assertVisible('law student');
await assertVisible('15 memories');
await assertVisible('Multilingual');
await screenshot('18-final.png');

await browser.close();
console.log(`\nDone — 18 screenshots written to ${OUT}/`);
