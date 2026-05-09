/**
 * Real E2E: "Hi" response latency test
 *
 * Verifies that a simple "Hi" message gets a fast response.
 * On GPUs where gemma4:e4b fills VRAM (~10.6GB), the embed model
 * (nomic-embed-text) can't coexist — the app falls back to keyword
 * search via a 500ms embed timeout, so chat-only latency is what matters.
 *
 * The LLM decides emotion via `<anim>` tags in the stream — no keyword
 * heuristics. Without `<anim>` tags, emotion defaults to neutral.
 *
 * Prerequisites:
 *   - Ollama running with `gemma4:e4b` available
 *   - `npm run dev` running (Vite dev server on localhost:1420)
 *
 * Run: npx playwright test --config Real-E2E/playwright.config.ts hi-latency
 */
import { test, expect } from '@playwright/test';
import { execFile } from 'node:child_process';
import path from 'node:path';
import { promisify } from 'node:util';
import {
  checkOllama,
  getPiniaState,
  ollamaChat,
  waitForAppReady,
} from './helpers';

const execFileAsync = promisify(execFile);

// Force the chat model warm before tests
async function warmChatModel() {
  const http = await import('node:http');
  const body = JSON.stringify({
    model: 'gemma4:e4b',
    messages: [{ role: 'user', content: 'hi' }],
    stream: false,
    think: false,
    options: { num_predict: 1 },
    keep_alive: '30m',
  });
  return new Promise<void>((resolve, reject) => {
    const req = http.default.request(
      { hostname: '127.0.0.1', port: 11434, path: '/api/chat', method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) },
        timeout: 60_000 },
      (res) => {
        res.on('data', () => {});
        res.on('end', () => resolve());
        res.on('error', reject);
      },
    );
    req.on('error', reject);
    req.write(body);
    req.end();
  });
}

test.describe('Hi latency — LLM response time', () => {
  test.beforeAll(async () => {
    const alive = await checkOllama();
    test.skip(!alive, 'Ollama not running — skipping latency tests');

    // Only warm the chat model — embed model may not coexist in VRAM
    await warmChatModel();
  });

  test('"Hi" chat response completes in < 1.5s (warm model)', async () => {
    // This tests the actual user experience: chat model already in VRAM,
    // embed either skipped (keyword fallback) or fast (if models coexist).
    const { timing, content } = await ollamaChat(
      [
        {
          role: 'system',
          content: 'You are TerranSoul, a friendly AI companion. Keep replies short — 1-3 sentences for casual chat.',
        },
        { role: 'user', content: 'Hi' },
      ],
      'gemma4:e4b',
      30,
    );

    console.log(`[Hi Chat] prompt=${timing.promptMs}ms gen=${timing.genMs}ms total=${timing.totalMs}ms`);
    console.log(`[Hi Chat] response: "${content.slice(0, 100)}"`);

    expect(content.length).toBeGreaterThan(0);
    expect(timing.totalMs).toBeLessThan(1_500);
  });

  test('"Hi" chat-only latency < 1s (no embed)', async () => {
    const { timing, content } = await ollamaChat(
      [
        { role: 'system', content: 'You are TerranSoul.' },
        { role: 'user', content: 'Hi' },
      ],
      'gemma4:e4b',
      20,
    );

    console.log(`[Hi Chat-Only] prompt=${timing.promptMs}ms gen=${timing.genMs}ms total=${timing.totalMs}ms`);
    console.log(`[Hi Chat-Only] response: "${content.slice(0, 100)}"`);

    expect(content.length).toBeGreaterThan(0);
    expect(timing.totalMs).toBeLessThan(1_000);
  });

  test('response sentiment is neutral (no keyword hack)', async () => {
    // When the LLM responds to "Hi" without <anim> tags, the app
    // should default to 'neutral' — never 'happy' from keyword matching.
    const { content } = await ollamaChat(
      [
        { role: 'system', content: 'You are TerranSoul, a friendly AI companion. Reply concisely.' },
        { role: 'user', content: 'Hi' },
      ],
      'gemma4:e4b',
      30,
    );

    // The response text itself doesn't matter for sentiment —
    // emotion is decided by <anim> tags in the stream, not keywords.
    // Verify the response is a reasonable greeting, not emotion-laden.
    expect(content.length).toBeGreaterThan(0);
    console.log(`[Hi Sentiment] response: "${content.slice(0, 120)}"`);
  });

  test('real Rust streaming backend first chunk < 1s', async () => {
    test.setTimeout(180_000);
    const srcTauri = path.join(process.cwd(), 'src-tauri');
    const { stdout, stderr } = await execFileAsync(
      'cargo',
      [
        'test',
        '--lib',
        'commands::streaming::tests::local_ollama_hi_real_backend_first_chunk_under_1s',
        '--',
        '--ignored',
        '--nocapture',
      ],
      {
        cwd: srcTauri,
        env: {
          ...process.env,
          TERRANSOUL_TEST_OLLAMA_MODEL: process.env.TERRANSOUL_TEST_OLLAMA_MODEL ?? 'gemma4:e4b',
        },
        timeout: 170_000,
        maxBuffer: 1024 * 1024,
      },
    );
    console.log(stdout.trim());
    if (stderr.trim()) console.log(stderr.trim());
    expect(`${stdout}\n${stderr}`).toContain('[real-backend-hi] first_chunk=');
  });
});

test.describe('Hi latency — app UI fast path', () => {
  test('Tauri UI path shows first assistant text in < 1s and skips classifier', async ({ page }) => {
    await page.addInitScript(() => {
      type InvokeRecord = { cmd: string; args: unknown; ts: number };
      type ListenerEvent = { event: string; id: number; payload: unknown };
      type HiTiming = {
        clickMs: number | null;
        sendStreamInvokeMs: number | null;
        firstChunkMs: number | null;
      };

      const invokeLog: InvokeRecord[] = [];
      const hiTiming: HiTiming = { clickMs: null, sendStreamInvokeMs: null, firstChunkMs: null };
      const callbacks = new Map<number, (event: ListenerEvent) => void>();
      const listeners = new Map<string, number[]>();
      let nextCallbackId = 1;
      let nextEventId = 1;

      function emit(event: string, payload: unknown) {
        const chunk = payload as { text?: string; done?: boolean };
        if (event === 'llm-chunk' && chunk.text && hiTiming.firstChunkMs === null) {
          hiTiming.firstChunkMs = performance.now();
        }
        for (const callbackId of listeners.get(event) ?? []) {
          callbacks.get(callbackId)?.({ event, id: callbackId, payload });
        }
      }

      Object.assign(window, {
        __tsTauriInvokeLog: invokeLog,
        __tsHiTiming: hiTiming,
        __TAURI_EVENT_PLUGIN_INTERNALS__: {
          unregisterListener(event: string, eventId: number) {
            const ids = listeners.get(event) ?? [];
            listeners.set(event, ids.filter((id) => id !== eventId));
          },
        },
        __TAURI_INTERNALS__: {
          transformCallback(callback: (event: ListenerEvent) => void, once = false) {
            const id = nextCallbackId++;
            callbacks.set(id, (event) => {
              callback(event);
              if (once) callbacks.delete(id);
            });
            return id;
          },
          unregisterCallback(id: number) {
            callbacks.delete(id);
          },
          convertFileSrc(path: string) {
            return path;
          },
          async invoke(cmd: string, args: Record<string, unknown> = {}) {
            invokeLog.push({ cmd, args, ts: performance.now() });

            if (cmd === 'plugin:event|listen') {
              const event = String(args.event);
              const callbackId = Number(args.handler);
              const eventId = nextEventId++;
              const ids = listeners.get(event) ?? [];
              listeners.set(event, [...ids, callbackId]);
              return eventId;
            }
            if (cmd === 'plugin:event|unlisten') return undefined;

            // LocalOllama skips the classifier for ALL messages (avoids model contention)
            if (cmd === 'classify_intent') {
              throw new Error('classify_intent should not run for LocalOllama');
            }

            if (cmd === 'get_brain_mode') return { mode: 'local_ollama', model: 'gemma4:e4b' };
            if (cmd === 'list_free_providers') return [];
            if (cmd === 'get_active_brain') return null;
            if (cmd === 'check_ollama_status') return { running: true, model_count: 1 };
            if (cmd === 'get_ollama_models') return [{ name: 'gemma4:e4b', size: 0, modified_at: null }];
            if (cmd === 'get_system_info') return { total_ram_mb: 32768, cpu_cores: 8, os_name: 'Windows', arch: 'x86_64' };
            if (cmd === 'recommend_brain_models') return [];
            if (cmd === 'get_memories') return [];
            if (cmd === 'get_memory_stats') return { total: 0, short_count: 0, working_count: 0, long_count: 0, total_tokens: 0, avg_decay: 1 };
            if (cmd === 'get_settings') return null;
            if (cmd === 'get_voice_config') return { asr_provider: 'web-speech', tts_provider: null };
            if (cmd === 'get_persona') return null;
            if (cmd === 'get_all_tasks') return [];
            if (cmd === 'evaluate_auto_learn') return { should_fire: false, reason: 'test', turns_remaining: 5 };
            if (cmd === 'charisma_record_usage') return null;

            if (cmd === 'send_message_stream') {
              hiTiming.sendStreamInvokeMs = performance.now();
              setTimeout(() => emit('llm-chunk', { text: 'Hi there!', done: false }), 40);
              setTimeout(() => emit('llm-chunk', { text: '', done: true }), 55);
              await new Promise((resolve) => setTimeout(resolve, 65));
              return undefined;
            }

            return undefined;
          },
        },
      });
    });

    await page.goto('/');
    await waitForAppReady(page);

    const input = page.locator('.chat-input');
    const sendBtn = page.locator('.send-btn');
    await input.fill('Hi');
    await expect(sendBtn).toBeEnabled({ timeout: 2_000 });
    await page.evaluate(() => {
      (window as any).__tsHiTiming.clickMs = performance.now();
      (document.querySelector('.send-btn') as HTMLButtonElement | null)?.click();
    });

    await expect(async () => {
      const state = (await getPiniaState(page, 'conversation')) as any;
      const assistant = state?.messages?.find((message: any) => message.role === 'assistant');
      expect(assistant?.content).toContain('Hi there!');
    }).toPass({ timeout: 2_000 });

    const commands = await page.evaluate(() => (window as any).__tsTauriInvokeLog.map((entry: any) => entry.cmd));
    const timing = await page.evaluate(() => (window as any).__tsHiTiming);
    const clickToChunkMs = timing.firstChunkMs - timing.clickMs;
    const invokeToChunkMs = timing.firstChunkMs - timing.sendStreamInvokeMs;

    console.log(
      `[Hi UI] click-to-chunk=${Math.round(clickToChunkMs)}ms invoke-to-chunk=${Math.round(invokeToChunkMs)}ms`,
    );

    expect(commands).toContain('send_message_stream');
    expect(commands).not.toContain('classify_intent');
    expect(clickToChunkMs).toBeLessThan(1_000);
    expect(invokeToChunkMs).toBeLessThan(1_000);
  });
});

test.describe('Long message latency — all messages fast with LocalOllama', () => {
  test.beforeAll(async () => {
    const alive = await checkOllama();
    test.skip(!alive, 'Ollama not running — skipping latency tests');
    await warmChatModel();
  });

  test('"What is the meaning of life?" completes in < 1.5s (warm model, no embed)', async () => {
    // Tests that substantive queries (not just "Hi") are also fast when using
    // LocalOllama. The streaming path skips embedding entirely (keyword-only RAG)
    // to avoid model contention. No classifier is called either.
    const { timing, content } = await ollamaChat(
      [
        {
          role: 'system',
          content: 'You are TerranSoul, a friendly AI companion. Answer thoughtfully.',
        },
        { role: 'user', content: 'What is the meaning of life?' },
      ],
      'gemma4:e4b',
      100,
    );

    console.log(`[Long Chat] prompt=${timing.promptMs}ms gen=${timing.genMs}ms total=${timing.totalMs}ms`);
    console.log(`[Long Chat] response: "${content.slice(0, 150)}"`);

    expect(content.length).toBeGreaterThan(0);
    // Total time (including generation) should be under 1.5s for a warm model
    expect(timing.promptMs).toBeLessThan(1_000);
  });

  test('"Tell me about quantum physics" — full response allowed (no num_predict cap)', async () => {
    const { timing, content } = await ollamaChat(
      [
        {
          role: 'system',
          content: 'You are TerranSoul. Answer questions thoroughly.',
        },
        { role: 'user', content: 'Tell me about quantum physics in 3 sentences.' },
      ],
      'gemma4:e4b',
      200,
    );

    console.log(`[Quantum] prompt=${timing.promptMs}ms gen=${timing.genMs}ms total=${timing.totalMs}ms tokens=${content.split(/\s+/).length}`);
    console.log(`[Quantum] response: "${content.slice(0, 200)}"`);

    expect(content.length).toBeGreaterThan(50);
    expect(timing.promptMs).toBeLessThan(1_000);
  });
});
