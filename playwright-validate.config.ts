import { defineConfig } from '@playwright/test';
import path from 'path';

/**
 * Playwright configuration specifically for the self-validation recording.
 *
 * Records a video of the entire test run and saves it to the `recording/` directory.
 * Only the latest video is kept — older files are removed before each run.
 */
export default defineConfig({
  testDir: './e2e',
  testMatch: 'validate-and-record.spec.ts',
  fullyParallel: false,
  retries: 0,
  workers: 1,
  reporter: [['list'], ['html', { open: 'never', outputFolder: 'playwright-report' }]],
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on',
    video: {
      mode: 'on',
      size: { width: 1280, height: 720 },
    },
    viewport: { width: 1280, height: 720 },
  },
  projects: [
    {
      name: 'validation-recording',
      use: { browserName: 'chromium' },
    },
  ],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
  outputDir: path.resolve('test-results/validation'),
});
