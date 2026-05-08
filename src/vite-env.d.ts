/// <reference types="vite/client" />

interface ImportMetaEnv {
  /**
   * Set to '1' by playwright.config.ts webServer env when running E2E tests.
   * When present, App.vue skips the browser landing redirect so the normal
   * app shell (with .chat-view) is rendered, which is what the Playwright
   * helpers expect (waitForAppReady looks for .chat-view).
   */
  readonly VITE_E2E?: string;
  /**
   * Set to '1' to simulate MCP mode in the browser/E2E without Tauri.
   * Causes the window store to report isMcpMode=true and shows the
   * McpActivityPanel + full tabbed UI.
   */
  readonly VITE_MCP_MODE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
