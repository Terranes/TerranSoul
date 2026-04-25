/**
 * capture-pet-mode.mjs — Capture pet mode screenshot for BRAIN-COMPLEX-EXAMPLE.md
 *
 * Usage: node scripts/capture-pet-mode.mjs
 * Requires: dev server running on localhost:1420
 */
import { chromium } from 'playwright';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCREENSHOTS = join(__dirname, '..', 'instructions', 'screenshots');
const BASE = 'http://localhost:1420';

async function main() {
  const browser = await chromium.launch({ headless: false });
  const ctx = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1,
  });
  const page = await ctx.newPage();

  console.log('Loading app...');
  await page.goto(BASE, { waitUntil: 'networkidle', timeout: 30_000 });
  await page.waitForTimeout(3000); // let VRM model load

  // Dismiss any quest dialogs
  const continueBtn = page.locator('button:has-text("Continue ▸")');
  while (await continueBtn.isVisible({ timeout: 500 }).catch(() => false)) {
    await continueBtn.click();
    await page.waitForTimeout(500);
  }

  // Dismiss first-launch wizard if present
  const skipBtn = page.locator('button:has-text("Skip")');
  if (await skipBtn.isVisible({ timeout: 500 }).catch(() => false)) {
    await skipBtn.click();
    await page.waitForTimeout(500);
  }

  // Inject conversation messages for the pet mode chat
  console.log('Injecting conversation messages...');
  await page.evaluate(() => {
    const app = document.querySelector('#app').__vue_app__;
    const pinia = app.config.globalProperties.$pinia;
    const conv = pinia._s.get('conversation');
    const now = Date.now();
    conv.messages = [
      { id: 'p1', role: 'user', content: 'What is the statute of limitations for contract disputes under Vietnamese law?', timestamp: now - 120000, model: '' },
      { id: 'p2', role: 'assistant', content: 'Under Article 429 of the 2015 Civil Code, the statute of limitations for contractual disputes is three (3) years from the date the claimant "knew or should have known" of the breach. 📚', timestamp: now - 90000, model: 'free' },
      { id: 'p3', role: 'user', content: 'Can penalty and damages be claimed together?', timestamp: now - 30000, model: '' },
      { id: 'p4', role: 'assistant', content: 'Yes! Under Article 420, if no agreement specifies otherwise, the aggrieved party may claim both the contractual penalty AND full compensation for damages. 🧠', timestamp: now, model: 'free' },
    ];
  });
  await page.waitForTimeout(300);

  // Switch to pet mode
  console.log('Switching to pet mode...');
  await page.evaluate(() => {
    const app = document.querySelector('#app').__vue_app__;
    const pinia = app.config.globalProperties.$pinia;
    const win = pinia._s.get('window');
    win.mode = 'pet';
  });
  await page.waitForTimeout(2000); // Let VRM re-render in pet mode

  // Dismiss pet mode onboarding if present
  const gotItBtn = page.locator('button:has-text("Got it")');
  if (await gotItBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
    await gotItBtn.click();
    await page.waitForTimeout(500);
  }

  // Click the character to open chat panel
  console.log('Opening chat panel...');
  const charEl = page.locator('.pet-character');
  if (await charEl.isVisible({ timeout: 1000 }).catch(() => false)) {
    await charEl.click({ position: { x: 150, y: 150 } });
    await page.waitForTimeout(1000);
  }

  // Open the context menu via right-click on the character
  console.log('Opening context menu...');
  await charEl.click({ button: 'right', position: { x: 150, y: 250 } });
  await page.waitForTimeout(500);

  // Expand the Panels submenu
  const panelsItem = page.locator('.ctx-label:has-text("Panels")');
  if (await panelsItem.isVisible({ timeout: 1000 }).catch(() => false)) {
    await panelsItem.click();
    await page.waitForTimeout(300);
  }

  // Add a simulated desktop background for the screenshot
  await page.evaluate(() => {
    const overlay = document.querySelector('.pet-overlay');
    if (overlay) {
      overlay.style.background = 'linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%)';
    }
  });
  await page.waitForTimeout(500);

  // Screenshot
  console.log('Capturing screenshot...');
  await page.screenshot({
    path: join(SCREENSHOTS, '14-pet-mode.png'),
    type: 'png',
  });
  console.log('Saved: 14-pet-mode.png');

  await browser.close();
  console.log('Done!');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
