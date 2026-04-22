/**
 * brain-flow-screenshots.mjs
 *
 * Captures real screenshots of the entire Brain + RAG flow described in
 * BRAIN-COMPLEX-EXAMPLE.md — from first launch to memory-augmented chat.
 *
 * Usage: node scripts/brain-flow-screenshots.mjs
 * Requires: Playwright, TerranSoul running on localhost:1420
 */
import { chromium } from 'playwright';
import { mkdirSync } from 'fs';

const BASE = 'http://localhost:1420';
const OUT = 'instructions/screenshots';
mkdirSync(OUT, { recursive: true });

async function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });

console.log('=== Step 1: Fresh Launch — Chat View ===');
await page.goto(BASE, { waitUntil: 'networkidle', timeout: 30000 });
await sleep(3000);
await page.screenshot({ path: `${OUT}/01-fresh-launch.png`, fullPage: false });
console.log('  ✓ Captured 01-fresh-launch.png');

console.log('=== Step 2: Navigate to Brain Setup ===');
// Look for the brain setup link/button — try multiple selectors
const brainSetupSelectors = [
  '[data-testid="brain-setup"]',
  'a[href*="brain"]',
  'button:has-text("Brain")',
  '.nav-item:has-text("Brain")',
  '[aria-label*="brain"]',
  '[aria-label*="Brain"]',
  'a:has-text("Brain")',
  '.sidebar a:nth-child(2)',
];
let brainClicked = false;
for (const sel of brainSetupSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      brainClicked = true;
      console.log(`  Clicked: ${sel}`);
      break;
    }
  } catch { /* try next */ }
}
if (!brainClicked) {
  // Try navigating directly
  await page.goto(`${BASE}/#/brain`, { waitUntil: 'networkidle', timeout: 10000 });
  console.log('  Navigated directly to #/brain');
}
await sleep(2000);
await page.screenshot({ path: `${OUT}/02-brain-setup-wizard.png`, fullPage: false });
console.log('  ✓ Captured 02-brain-setup-wizard.png');

console.log('=== Step 3: Select Free Cloud API ===');
const freeApiSelectors = [
  'button:has-text("Free")',
  'button:has-text("free")',
  '[data-testid="free-api"]',
  '.brain-option:first-child',
  'button:has-text("Cloud")',
  'button:has-text("Instant")',
];
for (const sel of freeApiSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      console.log(`  Clicked: ${sel}`);
      break;
    }
  } catch { /* try next */ }
}
await sleep(2000);
await page.screenshot({ path: `${OUT}/03-free-api-selected.png`, fullPage: false });
console.log('  ✓ Captured 03-free-api-selected.png');

console.log('=== Step 4: Brain Connected ===');
// Look for activation/next buttons
const activateSelectors = [
  'button:has-text("Activate")',
  'button:has-text("activate")',
  'button:has-text("Start")',
  'button:has-text("Connect")',
  'button:has-text("Next")',
  'button:has-text("Done")',
];
for (const sel of activateSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      console.log(`  Clicked: ${sel}`);
      break;
    }
  } catch { /* try next */ }
}
await sleep(3000);
await page.screenshot({ path: `${OUT}/04-brain-connected.png`, fullPage: false });
console.log('  ✓ Captured 04-brain-connected.png');

console.log('=== Step 5: Navigate to Chat ===');
// Go back to chat view
const chatSelectors = [
  'a[href*="chat"]',
  'a[href="/"]',
  'a[href="#/"]',
  'button:has-text("Chat")',
  'button:has-text("Start chatting")',
  '.nav-item:first-child',
];
for (const sel of chatSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      console.log(`  Clicked: ${sel}`);
      break;
    }
  } catch { /* try next */ }
}
await sleep(2000);
await page.screenshot({ path: `${OUT}/05-chat-ready.png`, fullPage: false });
console.log('  ✓ Captured 05-chat-ready.png');

console.log('=== Step 6: Send First Chat (No Memories) ===');
const chatInputSelectors = [
  'textarea',
  'input[type="text"]',
  '[data-testid="chat-input"]',
  '.chat-input textarea',
  '.chat-input input',
];
const sendSelectors = [
  'button[type="submit"]',
  'button:has-text("Send")',
  '[data-testid="send-button"]',
  '.send-btn',
];
let inputFound = false;
for (const sel of chatInputSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.fill('What is the deadline for responding to a family law motion?');
      inputFound = true;
      console.log(`  Filled input: ${sel}`);
      break;
    }
  } catch { /* try next */ }
}
if (inputFound) {
  for (const sel of sendSelectors) {
    try {
      const el = page.locator(sel).first();
      if (await el.isVisible({ timeout: 1000 })) {
        await el.click();
        console.log(`  Clicked send: ${sel}`);
        break;
      }
    } catch { /* try next */ }
  }
  // Wait for response
  await sleep(15000);
}
await page.screenshot({ path: `${OUT}/06-chat-no-memories.png`, fullPage: false });
console.log('  ✓ Captured 06-chat-no-memories.png');

console.log('=== Step 7: Navigate to Memory View ===');
const memorySelectors = [
  'a[href*="memory"]',
  'button:has-text("Memory")',
  '.nav-item:has-text("Memory")',
  '[aria-label*="memory"]',
  '[aria-label*="Memory"]',
];
for (const sel of memorySelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      console.log(`  Clicked: ${sel}`);
      break;
    }
  } catch { /* try next */ }
}
await sleep(2000);
await page.screenshot({ path: `${OUT}/07-memory-view-empty.png`, fullPage: false });
console.log('  ✓ Captured 07-memory-view-empty.png');

console.log('=== Step 8: Add Memories ===');
const addMemorySelectors = [
  'button:has-text("Add")',
  'button:has-text("add")',
  '[data-testid="add-memory"]',
  '.add-memory-btn',
];
for (const sel of addMemorySelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      console.log(`  Clicked add: ${sel}`);
      break;
    }
  } catch { /* try next */ }
}
await sleep(1000);

// Try to find the memory content textarea and fill it
const memTextSelectors = [
  'textarea',
  'input[placeholder*="memory"]',
  'input[placeholder*="content"]',
  '.memory-content textarea',
];
for (const sel of memTextSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.fill('Cook County Family Law Rule 14.3: Responses to motions must be filed within 30 days of service. Filing must include proof of service and use the e-filing system.');
      console.log(`  Filled memory: ${sel}`);
      break;
    }
  } catch { /* try next */ }
}
await sleep(500);

// Save the memory
const saveSelectors = [
  'button:has-text("Save")',
  'button:has-text("save")',
  'button:has-text("Add")',
  'button[type="submit"]',
];
for (const sel of saveSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      console.log(`  Clicked save: ${sel}`);
      break;
    }
  } catch { /* try next */ }
}
await sleep(2000);
await page.screenshot({ path: `${OUT}/08-memory-added.png`, fullPage: false });
console.log('  ✓ Captured 08-memory-added.png');

// Add a second memory
console.log('=== Step 8b: Add Second Memory ===');
for (const sel of addMemorySelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      break;
    }
  } catch { /* try next */ }
}
await sleep(1000);
for (const sel of memTextSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.fill('Section 7.2: All motions require a certificate of service filed simultaneously. Judge Martinez courtroom requires certificate of compliance.');
      break;
    }
  } catch { /* try next */ }
}
await sleep(500);
for (const sel of saveSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      break;
    }
  } catch { /* try next */ }
}
await sleep(2000);
await page.screenshot({ path: `${OUT}/09-memories-list.png`, fullPage: false });
console.log('  ✓ Captured 09-memories-list.png');

console.log('=== Step 9: Navigate Back to Chat ===');
for (const sel of chatSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      break;
    }
  } catch { /* try next */ }
}
await sleep(2000);

console.log('=== Step 10: Same Question — Now With RAG ===');
for (const sel of chatInputSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.fill('What is the deadline for responding to a family law motion?');
      break;
    }
  } catch { /* try next */ }
}
for (const sel of sendSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      break;
    }
  } catch { /* try next */ }
}
await sleep(15000);
await page.screenshot({ path: `${OUT}/10-chat-with-rag.png`, fullPage: false });
console.log('  ✓ Captured 10-chat-with-rag.png');

console.log('=== Step 11: Skill Tree / Quest View ===');
const skillSelectors = [
  'a[href*="skill"]',
  'a[href*="quest"]',
  'button:has-text("Quest")',
  'button:has-text("Skill")',
  '.quest-bubble',
  '[data-testid="quest"]',
];
for (const sel of skillSelectors) {
  try {
    const el = page.locator(sel).first();
    if (await el.isVisible({ timeout: 1000 })) {
      await el.click();
      console.log(`  Clicked: ${sel}`);
      break;
    }
  } catch { /* try next */ }
}
await sleep(2000);
await page.screenshot({ path: `${OUT}/11-skill-tree.png`, fullPage: false });
console.log('  ✓ Captured 11-skill-tree.png');

console.log('\n=== Done! Screenshots saved to instructions/screenshots/ ===');
await browser.close();
