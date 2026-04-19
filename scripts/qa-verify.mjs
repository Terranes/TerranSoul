import { chromium } from 'playwright';

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 450, height: 750 } });
await page.goto('http://localhost:1421', { waitUntil: 'networkidle', timeout: 30000 });
await page.waitForTimeout(2000);

const petBtn = await page.$('.mobile-bottom-nav .mobile-pet-toggle');
console.log('Pet toggle in mobile nav exists:', !!petBtn);

// In browser mode tauriAvailable=false, so v-if hides it
// Verify the comment placeholder is in the DOM (Vue renders <!--v-if-->)
const mobileNavHtml = await page.$eval('.mobile-bottom-nav', el => el.innerHTML);
console.log('Mobile nav HTML includes v-if comment:', mobileNavHtml.includes('v-if'));

const tabCount = await page.$$eval('.mobile-tab', els => els.length);
console.log('Mobile tabs count:', tabCount);

const footer = await page.$eval('.input-footer', el => {
  const r = el.getBoundingClientRect();
  return { top: Math.round(r.top), bottom: Math.round(r.bottom), height: Math.round(r.height) };
});
console.log('Input footer:', footer);

const mobileNav = await page.$eval('.mobile-bottom-nav', el => {
  const r = el.getBoundingClientRect();
  return { top: Math.round(r.top), bottom: Math.round(r.bottom) };
});
console.log('Mobile nav:', mobileNav);
console.log('Footer sits above nav:', footer.bottom <= mobileNav.top + 1);

await page.screenshot({ path: 'qa-screenshots/450x750-after-fix.png' });
await browser.close();
