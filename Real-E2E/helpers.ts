/**
 * Shared helpers for Real-E2E tests.
 *
 * Unlike the regular e2e/ helpers, these assume a REAL Tauri backend is
 * running. Ollama HTTP API is called directly for latency measurements.
 *
 * Uses Node.js `http` module instead of `fetch` to avoid IPv6 `localhost`
 * resolution issues on Windows (Ollama binds to 127.0.0.1 only).
 */
import { expect, type Page } from '@playwright/test';
import http from 'node:http';

// ─── Ollama direct API ───────────────────────────────────────────────────────

// Force IPv4 — Ollama binds 127.0.0.1, not [::1]
export const OLLAMA_URL = process.env.OLLAMA_URL ?? 'http://127.0.0.1:11434';

export interface OllamaTiming {
  embedMs: number;
  promptMs: number;
  genMs: number;
  totalMs: number;
  ttftMs: number; // embed + prompt_eval = time-to-first-token
  evalCount: number;
}

/** Per-request timeout for Ollama calls (ms). */
const OLLAMA_TIMEOUT = 30_000;

/** Low-level HTTP POST using Node.js http module (avoids IPv6 issues). */
function httpPost(url: string, body: string, timeout = OLLAMA_TIMEOUT): Promise<string> {
  return new Promise((resolve, reject) => {
    const parsed = new URL(url);
    const req = http.request(
      {
        hostname: parsed.hostname,
        port: Number(parsed.port),
        path: parsed.pathname,
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) },
        timeout,
      },
      (res) => {
        const chunks: Buffer[] = [];
        res.on('data', (c) => chunks.push(c));
        res.on('end', () => resolve(Buffer.concat(chunks).toString()));
        res.on('error', reject);
      },
    );
    req.on('timeout', () => { req.destroy(); reject(new Error(`Ollama timeout after ${timeout}ms`)); });
    req.on('error', reject);
    req.write(body);
    req.end();
  });
}

/** Low-level HTTP GET. */
function httpGet(url: string, timeout = 3000): Promise<{ ok: boolean; body: string }> {
  return new Promise((resolve, reject) => {
    const parsed = new URL(url);
    const req = http.get(
      { hostname: parsed.hostname, port: Number(parsed.port), path: parsed.pathname, timeout },
      (res) => {
        const chunks: Buffer[] = [];
        res.on('data', (c) => chunks.push(c));
        res.on('end', () => resolve({ ok: (res.statusCode ?? 500) < 400, body: Buffer.concat(chunks).toString() }));
        res.on('error', reject);
      },
    );
    req.on('timeout', () => { req.destroy(); reject(new Error('timeout')); });
    req.on('error', reject);
  });
}

/** Embed a query via Ollama and return the elapsed time in ms. */
export async function embedQuery(
  text: string,
  model = 'nomic-embed-text',
): Promise<{ ms: number; embedding: number[] }> {
  const start = performance.now();
  const raw = await httpPost(
    `${OLLAMA_URL}/api/embed`,
    JSON.stringify({ model, input: text, keep_alive: '30m' }),
  );
  const ms = performance.now() - start;
  const json = JSON.parse(raw);
  return { ms, embedding: json.embeddings?.[0] ?? [] };
}

/** Send a chat request to Ollama (non-streaming) and return timing info. */
export async function ollamaChat(
  messages: Array<{ role: string; content: string }>,
  model = 'gemma4:e4b',
  numPredict = 80,
): Promise<{ timing: OllamaTiming; content: string }> {
  const embedStart = performance.now();

  const raw = await httpPost(
    `${OLLAMA_URL}/api/chat`,
    JSON.stringify({
      model,
      messages,
      stream: false,
      think: false, // Gemma 4 thinking mode eats num_predict budget — disable for tests
      options: { num_predict: numPredict },
      keep_alive: '30m',
    }),
  );
  const wallMs = performance.now() - embedStart;
  const json = JSON.parse(raw);

  const promptMs = Math.round((json.prompt_eval_duration ?? 0) / 1e6);
  const genMs = Math.round((json.eval_duration ?? 0) / 1e6);

  return {
    timing: {
      embedMs: 0,  // caller sets this from embedQuery
      promptMs,
      genMs,
      totalMs: Math.round(wallMs),
      ttftMs: promptMs,
      evalCount: json.eval_count ?? 0,
    },
    content: json.message?.content ?? '',
  };
}

/** Full RAG pipeline test: embed query + LLM chat with memory context. */
export async function ragPipeline(
  query: string,
  memoryContext: string,
  opts?: { chatModel?: string; embedModel?: string; numPredict?: number },
): Promise<{ timing: OllamaTiming; content: string }> {
  const chatModel = opts?.chatModel ?? 'gemma4:e4b';
  const embedModel = opts?.embedModel ?? 'nomic-embed-text';
  const numPredict = opts?.numPredict ?? 80;

  // Stage 1: Embed
  const embed = await embedQuery(query, embedModel);

  // Stage 2: Chat with injected memory (simulates hybrid_search results)
  const { timing, content } = await ollamaChat(
    [
      {
        role: 'system',
        content: `You are TerranSoul, a helpful AI companion. Reply concisely.\n\n[LONG-TERM MEMORY]\n${memoryContext}\n[/LONG-TERM MEMORY]`,
      },
      { role: 'user', content: query },
    ],
    chatModel,
    numPredict,
  );

  return {
    timing: {
      ...timing,
      embedMs: Math.round(embed.ms),
      ttftMs: Math.round(embed.ms) + timing.promptMs,
    },
    content,
  };
}

// ─── Ollama health & warmup ──────────────────────────────────────────────────

/** Check if Ollama is reachable. */
export async function checkOllama(): Promise<boolean> {
  try {
    const { ok } = await httpGet(`${OLLAMA_URL}/api/version`);
    return ok;
  } catch {
    return false;
  }
}

/** Warm up models by sending a tiny request to each. */
export async function warmModels(
  chatModel = 'gemma4:e4b',
  embedModel = 'nomic-embed-text',
): Promise<void> {
  // Warmup can take a while if models need to load into VRAM — generous 60s timeout
  const warmTimeout = 60_000;
  await httpPost(
    `${OLLAMA_URL}/api/embed`,
    JSON.stringify({ model: embedModel, input: 'warmup', keep_alive: '30m' }),
    warmTimeout,
  );
  await httpPost(
    `${OLLAMA_URL}/api/chat`,
    JSON.stringify({
      model: chatModel,
      messages: [{ role: 'user', content: 'hi' }],
      stream: false,
      think: false,
      options: { num_predict: 1 },
      keep_alive: '30m',
    }),
    warmTimeout,
  );
}

// ─── App helpers (browser context) ───────────────────────────────────────────

export const TIMEOUTS = {
  appInit: 30_000,
  panel: 5_000,
  response: 60_000,
} as const;

/** Wait for the Vue app to fully initialize. */
export async function waitForAppReady(page: Page) {
  await page.waitForFunction(
    () => {
      const app = (document.querySelector('#app') as any)?.__vue_app__;
      if (!app) return false;
      const pinia = app.config.globalProperties.$pinia;
      if (!pinia) return false;
      const chatView = document.querySelector('.chat-view');
      return chatView && (chatView as HTMLElement).offsetParent !== null;
    },
    { timeout: TIMEOUTS.appInit },
  );
  await expect(page.locator('.chat-view')).toBeVisible({ timeout: 5_000 });
}

/** Navigate to a tab. */
export async function navigateToTab(page: Page, tabName: string) {
  const desktopTab = page.locator('.nav-btn', { hasText: tabName }).first();
  if (await desktopTab.isVisible().catch(() => false)) {
    await desktopTab.click();
    return;
  }
  const mobileTab = page.locator('.mobile-tab', { hasText: tabName }).first();
  await mobileTab.click();
}

/** Send a chat message via the UI. */
export async function sendMessage(page: Page, text: string) {
  const input = page.locator('.chat-input');
  const sendBtn = page.locator('.send-btn');
  await input.fill(text);
  await expect(sendBtn).toBeEnabled({ timeout: 2_000 });
  await sendBtn.click();
}

/** Read Pinia store state. */
export async function getPiniaState(page: Page, storeName: string) {
  return page.evaluate((name) => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    if (!app) return null;
    const pinia = app.config.globalProperties.$pinia;
    if (!pinia) return null;
    return pinia.state.value[name] ?? null;
  }, storeName);
}

/** Patch Pinia store state. */
export async function setPinia(
  page: Page,
  patch: Record<string, unknown>,
) {
  await page.evaluate((data) => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    const pinia = app?.config?.globalProperties?.$pinia;
    if (!pinia) return;
    for (const [store, values] of Object.entries(data)) {
      const s = pinia._s.get(store);
      if (s) {
        s.$patch(values as any);
      } else {
        if (!pinia.state.value[store]) pinia.state.value[store] = {};
        Object.assign(pinia.state.value[store], values);
      }
    }
  }, patch);
  await page.waitForTimeout(300);
}

/** Collect console errors, ignoring known benign ones. */
export function collectConsoleErrors(page: Page): string[] {
  const IGNORED = [
    '__TAURI_INTERNALS__',
    'process_prompt_silently',
    'Vercel',
    'net::ERR_',
    'Failed to fetch',
  ];
  const errors: string[] = [];
  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      const text = msg.text();
      if (IGNORED.some((p) => text.includes(p))) return;
      errors.push(text);
    }
  });
  page.on('pageerror', (err) => {
    const text = err.message;
    if (IGNORED.some((p) => text.includes(p))) return;
    errors.push(`UNCAUGHT: ${text}`);
  });
  return errors;
}
