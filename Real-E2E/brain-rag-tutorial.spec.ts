/**
 * Real E2E: Brain RAG Tutorial Flow
 *
 * Tests the full brain-rag-setup-tutorial.md flow with a REAL Ollama backend.
 * Measures actual pipeline latency (embed + LLM) and asserts response
 * latency stays within the local 2s budget.
 *
 * Prerequisites:
 *   - Ollama running with `nomic-embed-text` and `gemma4:e4b` loaded
 *   - `npm run dev` running (Vite dev server on localhost:1420)
 *
 * Run: npm run test:e2e:real
 *
 * This test is EXCLUDED from CI — it requires local GPU hardware.
 */
import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import {
  assertLocalResponseLatency,
  checkOllama,
  warmModels,
  ragPipeline,
  embedQuery,
  ollamaChat,
  collectConsoleErrors,
  completeFirstLaunchRecommendedIfPresent,
  connectToDesktopApp,
  closeOpenDialogIfPresent,
  waitForAppReady,
  navigateToTab,
} from './helpers';

const TUTORIAL_PATH = path.join(process.cwd(), 'tutorials', 'brain-rag-setup-tutorial.md');

// ─── RAG memory context (same as tutorial Steps 9-15) ───────────────────────

const MEMORY_CONTEXT = [
  'Article 429 of the 2015 Vietnamese Civil Code: Statute of limitations for contract disputes is 3 years from when claimant knew or should have known of breach.',
  'Article 351: Strict liability — no need to prove fault for breach of obligation.',
  'Article 352: Full compensation for breach of obligation.',
  'Article 420: Penalty clauses — may claim both penalty AND damages for breach.',
  'Article 419: Material + spiritual losses including lost benefits.',
  'Article 421: Exemption in force majeure cases.',
].map((line) => `- ${line}`).join('\n');

// ─── Test data for each tutorial step ───────────────────────────────────────

interface TutorialStep {
  step: string;
  query: string;
  embedQuery: string;
  expectInReply: string[];
  expectGroups?: string[][];
}

const TUTORIAL_STEPS: TutorialStep[] = [
  {
    step: '9  (EN)',
    query: 'What is the statute of limitations for contract disputes under Vietnamese law?',
    embedQuery: 'What is the statute of limitations for contract disputes under Vietnamese law?',
    expectInReply: ['429', 'three', '3'],
  },
  {
    step: '10 (EN)',
    query: 'Can a party claim both a penalty and damages for breach of contract?',
    embedQuery: 'Can a party claim both a penalty and damages for breach of contract?',
    expectInReply: ['420', 'penalty', 'damages'],
  },
  {
    step: '11 (VN)',
    query: 'Thời hiệu khởi kiện tranh chấp hợp đồng theo pháp luật Việt Nam là bao lâu?',
    embedQuery: 'statute of limitations contract dispute Vietnam',
    expectInReply: ['429', '3'],
  },
  {
    step: '12 (CN)',
    query: '越南法律中合同纠纷的诉讼时效是多长？',
    embedQuery: 'contract dispute statute limitations Chinese',
    expectInReply: ['429', '3'],
  },
  {
    step: '13 (RU)',
    query: 'Каков срок исковой давности по договорным спорам по вьетнамскому праву?',
    embedQuery: 'limitation period contract disputes Russian',
    expectInReply: ['429', '3'],
  },
  {
    step: '14 (JP)',
    query: 'ベトナム法における契約紛争の出訴期限はどのくらいですか？',
    embedQuery: 'contract dispute limitation Japanese',
    expectInReply: ['429', '3'],
  },
  {
    step: '15 (KR)',
    query: '베트남 법률에서 계약 분쟁의 소멸시효는 얼마입니까?',
    embedQuery: 'contract dispute limitation Korean',
    expectInReply: ['429', '3'],
  },
  {
    step: '18 (EN)',
    query: 'Summarize what you know about me and my documents.',
    embedQuery: 'Summarize what you know about me and my documents',
    expectInReply: [],
    expectGroups: [
      ['vietnamese', 'civil code', '2015'],
      ['contract', 'obligation', 'penalty', 'damages', 'force majeure', 'limitations'],
    ],
  },
];

const TUTORIAL_REQUIRED_QUERIES = [
  'Learn from my documents',
  'What is the statute of limitations for contract disputes under Vietnamese law?',
  'Thời hiệu khởi kiện tranh chấp hợp đồng theo pháp luật Việt Nam là bao lâu?',
  '越南法律中合同纠纷的诉讼时效是多长？',
];

// ─── Test ────────────────────────────────────────────────────────────────────

test.describe('Brain RAG Tutorial — Real Ollama Pipeline', () => {
  test.beforeAll(async () => {
    const ok = await checkOllama();
    test.skip(!ok, 'Ollama is not running at localhost:11434');
  });

  test('warm up models', async () => {
    test.setTimeout(90_000); // model load can be slow
    await warmModels();
  });

  test('tutorial document is covered by this E2E spec', async () => {
    const tutorial = readFileSync(TUTORIAL_PATH, 'utf8');
    expect(tutorial).toContain('Learn from my documents');
    expect(tutorial).toContain('Scholar\'s Quest');
    expect(tutorial).toContain('Vietnamese Civil Code');
    // Keep this contract tied to prompts explicitly present in the tutorial text.
    for (const query of TUTORIAL_REQUIRED_QUERIES) {
      expect(tutorial).toContain(query);
    }
  });

  test('Step 2: intent classification latency', async () => {
    const { timing } = await ollamaChat(
      [
        {
          role: 'system',
          content:
            'Classify user intent as JSON: {"kind":"chat"} or {"kind":"learn_with_docs","topic":"..."} or {"kind":"teach_ingest","topic":"..."}. Reply ONLY with JSON.',
        },
        {
          role: 'user',
          content: 'Learn Vietnamese laws using my provided documents',
        },
      ],
      'gemma4:e4b',
      30,
    );

    console.log(`  Intent classify: prompt=${timing.promptMs}ms gen=${timing.genMs}ms total=${timing.totalMs}ms`);

    // Intent classification runs concurrently with streaming, so total
    // time doesn't block the user. The prompt-eval latency must still stay
    // within the local 2s budget; if it regresses, fix the latency path.
    assertLocalResponseLatency('Intent classifier', timing.promptMs, 'prompt-eval latency');
  });

  for (const step of TUTORIAL_STEPS) {
    test(`Step ${step.step}: response latency < 2s & correct answer`, async () => {
      const result = await ragPipeline(step.query, MEMORY_CONTEXT);

      console.log(
        `  Step ${step.step}: embed=${result.timing.embedMs}ms prompt=${result.timing.promptMs}ms TTFT=${result.timing.ttftMs}ms gen=${result.timing.genMs}ms (${result.timing.evalCount} tok)`,
      );
      console.log(`  Step ${step.step} response: "${result.content.slice(0, 180)}"`);

      assertLocalResponseLatency(
        `Brain RAG tutorial Step ${step.step}`,
        result.timing.ttftMs,
        'time-to-first-token latency',
      );

      // Verify the answer references expected content
      const lower = result.content.toLowerCase();
      if (step.expectGroups) {
        for (const group of step.expectGroups) {
          const hasGroupMatch = group.some((kw) => lower.includes(kw.toLowerCase()));
          expect(hasGroupMatch).toBe(true);
        }
      } else {
        const hasExpected = step.expectInReply.some((kw) => lower.includes(kw.toLowerCase()));
        expect(hasExpected).toBe(true);
      }
    });
  }

  test('embedding model latency benchmark', async () => {
    const queries = [
      'What is the statute of limitations?',
      'penalty and damages for breach',
      'force majeure exemption',
      'contract formation requirements',
      'interest rate overdue payment',
    ];

    const times: number[] = [];
    for (const q of queries) {
      const { ms } = await embedQuery(q);
      times.push(Math.round(ms));
    }

    const avg = Math.round(times.reduce((a, b) => a + b, 0) / times.length);
    const max = Math.max(...times);

    console.log(`  Embed benchmark: avg=${avg}ms max=${max}ms samples=[${times.join(',')}]`);
    assertLocalResponseLatency('Embedding model benchmark', max, 'embedding latency');
  });
});

test.describe('Brain RAG Tutorial — UI Flow', () => {
  test('app loads and chat UI is functional through the Tauri WebView', async () => {
    const { browser, page } = await connectToDesktopApp();
    try {
      const errors = collectConsoleErrors(page);
      await waitForAppReady(page);
      await completeFirstLaunchRecommendedIfPresent(page);
      await closeOpenDialogIfPresent(page);

      const runtime = await page.evaluate(() => ({
        tauriAvailable: typeof (window as any).__TAURI_INTERNALS__?.invoke === 'function',
        href: window.location.href,
      }));
      expect(runtime.tauriAvailable).toBe(true);
      expect(runtime.href).toContain('localhost:1420');

      const chatInput = page.locator('.chat-input');
      await expect(chatInput).toBeVisible({ timeout: 5_000 });
      await expect(chatInput).toBeEnabled();

      const attachBtn = page.locator('.attach-btn');
      if (await attachBtn.isVisible({ timeout: 1_000 }).catch(() => false)) {
        await expect(attachBtn).toBeEnabled();
      }

      await navigateToTab(page, 'Brain');
      await expect(page.locator('.brain-view')).toBeVisible({ timeout: 5_000 });

      await navigateToTab(page, 'Memory');
      await expect(page.locator('.memory-view')).toBeVisible({ timeout: 5_000 });

      await navigateToTab(page, 'Chat');
      await expect(page.locator('.chat-view')).toBeVisible({ timeout: 5_000 });

      const crashes = errors.filter(
        (e) =>
          e.includes('Cannot read properties of undefined') ||
          e.includes('Cannot read properties of null') ||
          e.includes('UNCAUGHT') ||
          e.includes('is not a function'),
      );
      expect(crashes).toHaveLength(0);
    } finally {
      await browser.close();
    }
  });

});
