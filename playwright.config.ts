import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  // Spec files run in parallel workers (file-level parallelism); tests within
  // a file remain sequential because they share Pinia state via a shared dev
  // server. 2 workers keeps the total wall-time low without overloading the
  // free LLM API with too many concurrent requests.
  workers: process.env.CI ? 2 : 1,
  reporter: process.env.CI ? [['github'], ['html', { open: 'never' }]] : 'list',
  // Global per-test timeout. LLM-driven specs can issue several real
  // free-API calls (~30s each) plus VRM model load, so we give them headroom.
  timeout: 180_000,
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { browserName: 'chromium' },
    },
  ],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
});
