/**
 * Playwright config for Real-E2E tests.
 *
 * These tests require a RUNNING Tauri dev app (`npm run dev`) and Ollama
 * with the recommended models loaded. They are **excluded from CI** and
 * intended for manual local validation only.
 *
 * Prerequisites:
 *   1. Ollama running with nomic-embed-text + gemma4:e4b (or your recommended model)
 *   2. `npm run dev:desktop-e2e` running (Tauri WebView CDP on :9222)
 *   3. Run: `npx playwright test --config Real-E2E/playwright.config.ts`
 *      or:  `npm run test:e2e:real`
 */
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: '.',
  fullyParallel: false,
  forbidOnly: true,
  retries: 0,
  workers: 1,
  reporter: [['list'], ['html', { outputFolder: '../real-e2e-report', open: 'never' }]],
  // Real LLM calls can take a while — generous timeout
  timeout: 120_000,
  use: {
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { browserName: 'chromium' },
    },
  ],
  // No webServer: Real-E2E must attach to the running Tauri WebView through
  // WebView2 CDP. Starting browser-mode Vite here would make desktop checks
  // look valid while exercising the wrong UI surface.
});
