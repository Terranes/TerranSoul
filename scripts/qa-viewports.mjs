import { chromium } from 'playwright';
import { mkdirSync } from 'fs';

const BASE = 'http://localhost:1421';
const OUT = 'qa-screenshots';
mkdirSync(OUT, { recursive: true });

const viewports = [
  { name: 'mobile-small',   w: 375,  h: 667  },  // iPhone SE
  { name: 'mobile-medium',  w: 390,  h: 844  },  // iPhone 14
  { name: 'mobile-large',   w: 430,  h: 932  },  // iPhone 14 Pro Max
  { name: 'custom-450x750', w: 450,  h: 750  },  // User-reported size
  { name: 'tablet-portrait', w: 768, h: 1024 },  // iPad
  { name: 'tablet-landscape', w: 1024, h: 768 }, // iPad landscape
  { name: 'laptop-small',   w: 1280, h: 720  },  // HD
  { name: 'laptop-medium',  w: 1366, h: 768  },  // Common laptop
  { name: 'desktop',        w: 1920, h: 1080 },  // Full HD
  { name: 'narrow-tall',    w: 420,  h: 700  },  // Default Tauri window
];

const browser = await chromium.launch({ headless: true });

for (const vp of viewports) {
  console.log(`\n=== ${vp.name} (${vp.w}x${vp.h}) ===`);
  const page = await browser.newPage({ viewport: { width: vp.w, height: vp.h } });
  await page.goto(BASE, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(3000);

  // Measure key layout elements
  const measurements = await page.evaluate(() => {
    const results = {};
    
    // App shell
    const shell = document.querySelector('.app-shell');
    if (shell) {
      const r = shell.getBoundingClientRect();
      results.appShell = { top: r.top, height: r.height, bottom: r.bottom };
    }

    // Desktop nav
    const nav = document.querySelector('.app-nav.desktop-nav');
    if (nav) {
      const r = nav.getBoundingClientRect();
      const style = getComputedStyle(nav);
      results.desktopNav = { width: r.width, display: style.display };
    }

    // Mobile nav
    const mobileNav = document.querySelector('.mobile-bottom-nav');
    if (mobileNav) {
      const r = mobileNav.getBoundingClientRect();
      const style = getComputedStyle(mobileNav);
      results.mobileNav = { 
        display: style.display, 
        top: Math.round(r.top), 
        height: Math.round(r.height),
        bottom: Math.round(r.bottom),
      };
    }

    // App main
    const main = document.querySelector('.app-main');
    if (main) {
      const r = main.getBoundingClientRect();
      results.appMain = { top: Math.round(r.top), height: Math.round(r.height), bottom: Math.round(r.bottom) };
    }

    // Chat view
    const chatView = document.querySelector('.chat-view');
    if (chatView) {
      const r = chatView.getBoundingClientRect();
      const style = getComputedStyle(chatView);
      results.chatView = { 
        top: Math.round(r.top), 
        height: Math.round(r.height), 
        bottom: Math.round(r.bottom),
        display: style.display,
      };
    }

    // Bottom panel
    const bottom = document.querySelector('.bottom-panel');
    if (bottom) {
      const r = bottom.getBoundingClientRect();
      results.bottomPanel = { 
        top: Math.round(r.top), 
        height: Math.round(r.height), 
        bottom: Math.round(r.bottom),
      };
    }

    // Input footer
    const footer = document.querySelector('.input-footer');
    if (footer) {
      const r = footer.getBoundingClientRect();
      results.inputFooter = { 
        top: Math.round(r.top), 
        height: Math.round(r.height), 
        bottom: Math.round(r.bottom),
      };
    }

    // Viewport canvas
    const canvas = document.querySelector('.viewport-canvas');
    if (canvas) {
      const r = canvas.getBoundingClientRect();
      results.canvas = { width: Math.round(r.width), height: Math.round(r.height) };
    }

    // Quest bubble
    const quest = document.querySelector('.quest-bubble');
    if (quest) {
      const r = quest.getBoundingClientRect();
      const style = getComputedStyle(quest);
      results.questBubble = { 
        top: Math.round(r.top), 
        right: Math.round(window.innerWidth - r.right),
        display: style.display,
      };
    }

    // Settings corner
    const settings = document.querySelector('.settings-corner');
    if (settings) {
      const r = settings.getBoundingClientRect();
      results.settingsCorner = { top: Math.round(r.top), left: Math.round(r.left) };
    }

    // Music bar portal
    const musicPortal = document.querySelector('#music-bar-portal');
    if (musicPortal) {
      const r = musicPortal.getBoundingClientRect();
      results.musicPortal = { top: Math.round(r.top), left: Math.round(r.left) };
    }

    // Pet toggle button (in nav)
    const petBtn = document.querySelector('.nav-pet-toggle');
    if (petBtn) {
      const style = getComputedStyle(petBtn);
      results.petToggle = { display: style.display };
    }

    // Window dimensions
    results.window = { 
      innerWidth: window.innerWidth, 
      innerHeight: window.innerHeight,
    };

    return results;
  });

  // Key checks
  const issues = [];
  const wh = measurements.window;
  
  if (measurements.inputFooter) {
    const gap = wh.innerHeight - measurements.inputFooter.bottom;
    if (Math.abs(gap) > 2 && (!measurements.mobileNav || measurements.mobileNav.display === 'none')) {
      issues.push(`INPUT_FOOTER_NOT_AT_BOTTOM: gap=${gap}px (footer.bottom=${measurements.inputFooter.bottom}, viewport=${wh.innerHeight})`);
    }
    // On mobile with bottom nav, footer should be above the nav
    if (measurements.mobileNav && measurements.mobileNav.display !== 'none') {
      const navTop = measurements.mobileNav.top;
      const footerBottom = measurements.inputFooter.bottom;
      if (footerBottom > navTop + 2) {
        issues.push(`INPUT_FOOTER_HIDDEN_BY_MOBILE_NAV: footer.bottom=${footerBottom} > nav.top=${navTop}`);
      }
    }
  } else {
    issues.push('INPUT_FOOTER_NOT_FOUND');
  }

  if (measurements.chatView) {
    if (measurements.chatView.height < wh.innerHeight * 0.5) {
      issues.push(`CHAT_VIEW_TOO_SHORT: ${measurements.chatView.height}px (${Math.round(measurements.chatView.height / wh.innerHeight * 100)}% of viewport)`);
    }
  }

  if (measurements.canvas) {
    if (measurements.canvas.width < 50 || measurements.canvas.height < 50) {
      issues.push(`CANVAS_TOO_SMALL: ${measurements.canvas.width}x${measurements.canvas.height}`);
    }
  }

  // Check overlap: settings and music portal
  if (measurements.settingsCorner && measurements.musicPortal) {
    if (Math.abs(measurements.settingsCorner.left - measurements.musicPortal.left) < 40 &&
        Math.abs(measurements.settingsCorner.top - measurements.musicPortal.top) < 40) {
      issues.push(`SETTINGS_MUSIC_OVERLAP: settings(${measurements.settingsCorner.top},${measurements.settingsCorner.left}) music(${measurements.musicPortal.top},${measurements.musicPortal.left})`);
    }
  }

  console.log(JSON.stringify(measurements, null, 2));
  
  if (issues.length > 0) {
    console.log('ISSUES:');
    issues.forEach(i => console.log('  ❌ ' + i));
  } else {
    console.log('  ✅ No issues detected');
  }

  await page.screenshot({ path: `${OUT}/${vp.name}.png`, fullPage: false });
  await page.close();
}

await browser.close();
console.log('\n=== QA Complete ===');
