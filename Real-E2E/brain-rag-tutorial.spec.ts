/**
 * Real E2E: Brain RAG Tutorial Flow
 *
 * Tests the full brain-rag-setup-tutorial.md flow with a REAL Ollama backend.
 * Measures actual pipeline latency (embed + LLM) and asserts TTFT < 1s.
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
import {
  checkOllama,
  warmModels,
  ragPipeline,
  embedQuery,
  ollamaChat,
  collectConsoleErrors,
  waitForAppReady,
  navigateToTab,
  setPinia,
  type OllamaTiming,
} from './helpers';

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
    expectInReply: ['Article'],
  },
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
    // time doesn't block the user. We just verify it completes in a
    // reasonable window (the 1500ms timeout in the app).
    expect(timing.promptMs).toBeLessThan(1000);
  });

  for (const step of TUTORIAL_STEPS) {
    test(`Step ${step.step}: TTFT < 1s & correct answer`, async () => {
      const result = await ragPipeline(step.query, MEMORY_CONTEXT);

      console.log(
        `  Step ${step.step}: embed=${result.timing.embedMs}ms prompt=${result.timing.promptMs}ms TTFT=${result.timing.ttftMs}ms gen=${result.timing.genMs}ms (${result.timing.evalCount} tok)`,
      );

      // TTFT (embed + prompt eval) must be under 1 second
      expect(result.timing.ttftMs).toBeLessThan(1000);

      // Verify the answer references expected content
      const lower = result.content.toLowerCase();
      const hasExpected = step.expectInReply.some((kw) => lower.includes(kw.toLowerCase()));
      expect(hasExpected).toBe(true);
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
    expect(max).toBeLessThan(500);
  });
});

test.describe('Brain RAG Tutorial — UI Flow', () => {
  test('app loads and chat UI is functional', async ({ page }) => {
    const errors = collectConsoleErrors(page);
    await page.goto('/');
    await waitForAppReady(page);

    // Dismiss first-launch wizard if present
    const continueBtn = page.locator('button:has-text("Continue ▸")');
    while (await continueBtn.isVisible({ timeout: 500 }).catch(() => false)) {
      await continueBtn.click();
      await page.waitForTimeout(500);
    }
    const skipBtn = page.locator('button:has-text("Skip")');
    if (await skipBtn.isVisible({ timeout: 500 }).catch(() => false)) {
      await skipBtn.click();
      await page.waitForTimeout(500);
    }

    // Verify chat input is enabled and not disabled
    const chatInput = page.locator('.chat-input');
    await expect(chatInput).toBeVisible({ timeout: 5_000 });
    await expect(chatInput).toBeEnabled();

    // Verify attach button is not disabled
    const attachBtn = page.locator('.attach-btn');
    if (await attachBtn.isVisible({ timeout: 1_000 }).catch(() => false)) {
      await expect(attachBtn).toBeEnabled();
    }

    // Verify brain tab navigable
    await navigateToTab(page, 'Brain');
    await expect(page.locator('.brain-view')).toBeVisible({ timeout: 5_000 });

    // Verify memory tab navigable
    await navigateToTab(page, 'Memory');
    await expect(page.locator('.memory-view')).toBeVisible({ timeout: 5_000 });

    // Back to chat
    await navigateToTab(page, 'Chat');
    await expect(page.locator('.chat-view')).toBeVisible({ timeout: 5_000 });

    // No crash errors
    const crashes = errors.filter(
      (e) =>
        e.includes('Cannot read properties of undefined') ||
        e.includes('Cannot read properties of null') ||
        e.includes('UNCAUGHT') ||
        e.includes('is not a function'),
    );
    expect(crashes).toHaveLength(0);
  });

  test('chat messages render correctly with RAG content', async ({ page }) => {
    await page.goto('/');
    await waitForAppReady(page);

    // Dismiss wizard
    const continueBtn = page.locator('button:has-text("Continue ▸")');
    while (await continueBtn.isVisible({ timeout: 500 }).catch(() => false)) {
      await continueBtn.click();
      await page.waitForTimeout(500);
    }
    const skipBtn = page.locator('button:has-text("Skip")');
    if (await skipBtn.isVisible({ timeout: 500 }).catch(() => false)) {
      await skipBtn.click();
      await page.waitForTimeout(500);
    }

    // Switch to Chat-only mode (the default 3D mode hides the message list)
    const chatModeBtn = page.locator('.mode-seg-btn', { hasText: 'Chat' });
    if (await chatModeBtn.isVisible({ timeout: 2_000 }).catch(() => false)) {
      await chatModeBtn.click();
      await page.waitForTimeout(500);
    }

    // Inject a RAG Q&A conversation via Pinia
    await setPinia(page, {
      conversation: {
        messages: [
          {
            id: 'u1',
            role: 'user',
            content: 'What is the statute of limitations for contract disputes under Vietnamese law?',
            timestamp: Date.now() - 10000,
          },
          {
            id: 'a1',
            role: 'assistant',
            content:
              '**Article 429** of the 2015 Civil Code sets the statute of limitations at **three (3) years** from the date the claimant "knew or should have known" of the breach.\n\n' +
              '📚 Sources: `vietnamese-civil-code.html` (Articles 351, 429)',
            agentName: 'TerranSoul',
            sentiment: 'neutral',
            timestamp: Date.now() - 5000,
          },
        ],
        isThinking: false,
        isStreaming: false,
        streamingText: '',
      },
    });
    await page.waitForTimeout(600);

    // Verify the message renders
    await expect(
      page.locator('.chat-view').getByText('Article 429', { exact: false }).first(),
    ).toBeVisible();
    await expect(
      page.locator('.chat-view').getByText('three (3) years', { exact: false }).first(),
    ).toBeVisible();
  });
});
