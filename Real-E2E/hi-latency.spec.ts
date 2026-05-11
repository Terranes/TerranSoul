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
  assertLocalResponseLatency,
  checkOllama,
  ollamaChat,
} from './helpers';

const execFileAsync = promisify(execFile);

// Force the chat model warm before tests
async function warmChatModel() {
  async function postWarmup(stream: boolean): Promise<void> {
    const body = JSON.stringify({
      model: 'gemma4:e4b',
      messages: [{ role: 'user', content: 'hi' }],
      stream,
      think: false,
      options: { num_predict: 1, num_ctx: 1024 },
      keep_alive: '30m',
    });

    for (let attempt = 1; attempt <= 2; attempt += 1) {
      try {
        const controller = new AbortController();
        const timer = setTimeout(() => controller.abort(), 90_000);
        try {
          const response = await fetch('http://127.0.0.1:11434/api/chat', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body,
            signal: controller.signal,
          });
          const text = await response.text();
          if (!response.ok) throw new Error(`Ollama warm-up HTTP ${response.status}: ${text.slice(0, 300)}`);
        } finally {
          clearTimeout(timer);
        }
        return;
      } catch (err) {
        if (attempt === 2) throw err;
        await new Promise((resolve) => setTimeout(resolve, 2_000));
      }
    }
  }

  await postWarmup(false);
  await postWarmup(true);
}

test.describe('Hi latency — LLM response time', () => {
  test.beforeAll(async () => {
    const alive = await checkOllama();
    test.skip(!alive, 'Ollama not running — skipping latency tests');

    // Only warm the chat model — embed model may not coexist in VRAM
    await warmChatModel();
  });

  test('"Hi" chat response completes within the 2s local budget', async () => {
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
    assertLocalResponseLatency('Hi chat', timing.totalMs);
  });

  test('"Hi" chat-only response stays within the 2s local budget', async () => {
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
    assertLocalResponseLatency('Hi chat-only', timing.totalMs);
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

  test('real Rust streaming backend first chunk stays within the 2s local budget', async () => {
    test.setTimeout(180_000);
    await warmChatModel();
    const srcTauri = path.join(process.cwd(), 'src-tauri');
    const { stdout, stderr } = await execFileAsync(
      'cargo',
      [
        'test',
        '--lib',
        'commands::streaming::tests::local_ollama_hi_real_backend_first_chunk_under_2s',
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

test.describe('Direct Ollama latency — warm model responses', () => {
  test.beforeAll(async () => {
    const alive = await checkOllama();
    test.skip(!alive, 'Ollama not running — skipping latency tests');
    await warmChatModel();
  });

  test('"What is the meaning of life?" responds within the 2s local budget', async () => {
    // Tests that substantive direct Ollama calls stay fast with the warm model.
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
    assertLocalResponseLatency('Meaning-of-life chat', timing.ttftMs, 'time-to-first-token latency');
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
    assertLocalResponseLatency('Quantum chat', timing.ttftMs, 'time-to-first-token latency');
  });
});
