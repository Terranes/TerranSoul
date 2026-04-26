import { chromium } from 'playwright';
import { mkdirSync } from 'fs';

const BASE = 'http://localhost:1420';
const OUT = 'qa-screenshots';
mkdirSync(OUT, { recursive: true });

const viewports = [
  { name: 'desktop', w: 1280, h: 800 },
  { name: 'tablet', w: 768, h: 1024 },
  { name: 'mobile', w: 375, h: 667 },
];

// Tab label text as shown in the nav
const tabLabels = {
  chat: 'Chat',
  brain: 'Brain',
  memory: 'Memory',
  marketplace: 'Market',
};

const browser = await chromium.launch({ headless: true });

for (const vp of viewports) {
  const page = await browser.newPage({ viewport: { width: vp.w, height: vp.h } });
  await page.goto(BASE, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);

  // Inject Pinia state to skip wizard
  await page.evaluate(() => {
    const app = document.querySelector('#app')?.__vue_app__;
    if (!app) return;
    const pinia = app.config.globalProperties.$pinia;
    if (!pinia) return;
    const settings = pinia.state.value.settings;
    if (settings) { settings.firstLaunchDone = true; settings.hasCompletedSetup = true; }
    const brain = pinia.state.value.brain;
    if (brain) { brain.brainMode = 'FreeApi'; brain.hasBrain = true; }
  });
  await page.waitForTimeout(500);

  for (const [tab, label] of Object.entries(tabLabels)) {
    // Click the nav button containing this label
    const navContainer = vp.w > 640 ? '.desktop-nav' : '.mobile-bottom-nav';
    const btn = await page.$(`${navContainer} button:has-text("${label}")`);
    if (btn) {
      await btn.click();
      await page.waitForTimeout(1500);
    } else {
      console.log(`  ⚠ No button found for "${label}" in ${navContainer}`);
    }
    await page.screenshot({ path: `${OUT}/${vp.name}-${tab}.png`, fullPage: false });
    console.log(`${vp.name}-${tab} captured`);
  }
  await page.close();
}

await browser.close();
console.log('Done!');
