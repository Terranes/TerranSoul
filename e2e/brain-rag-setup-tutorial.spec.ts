/**
 * Brain RAG setup tutorial E2E coverage guard.
 *
 * This spec is local-only under the current Playwright GitHub Actions grep
 * (`desktop-flow` only). It keeps the tutorial contract wired into the E2E
 * suite so screenshots/text drift is caught before manual RAG validation.
 *
 * Three layers:
 *   1. Tutorial markdown contract — required strings in the .md file.
 *   2. Code behaviour contract — the source files referenced by the tutorial
 *      must contain the patterns that make the documented flow actually work.
 *   3. Playwright browser tests — send "Learn from my documents" in the real
 *      app (with mocked Tauri invoke) and verify the quest UI appears, not a
 *      normal chat response.
 */
import { test, expect } from '@playwright/test';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import {
  collectConsoleErrors,
  assertNoCrashErrors,
  waitForAppReady,
  getPiniaState,
  sendMessage,
  waitForAssistantResponse,
  TIMEOUTS,
} from './helpers';

const TUTORIAL_PATH = path.join(process.cwd(), 'tutorials', 'brain-rag-setup-tutorial.md');

const REQUIRED_TUTORIAL_CONTRACT = [
  'Brain + Memory + RAG - Pet Mode Tutorial',
  'Learn from my documents',
  'classify_intent',
  'Scholar\'s Quest',
  'vietnamese-civil-code.html',
  'article-429-commentary.txt',
  'What is the statute of limitations for contract disputes under Vietnamese law?',
  'Article 429',
  '09-ask-rag-question-chinese.png',
  'learn_with_docs',
];

test('brain-rag-setup-tutorial.md remains covered by local E2E tests', async () => {
  expect(existsSync(TUTORIAL_PATH)).toBe(true);
  const tutorial = readFileSync(TUTORIAL_PATH, 'utf8');

  for (const requiredText of REQUIRED_TUTORIAL_CONTRACT) {
    expect(tutorial).toContain(requiredText);
  }
});

/**
 * Code-level contract: the conversation store must handle learn_with_docs
 * from the background classifier even after streaming text has arrived.
 * Without this, LocalOllama's concurrent chat stream races the classifier
 * and the quest silently never appears.
 */
test('conversation store handles background learn_with_docs after stream starts', async () => {
  const convPath = path.join(process.cwd(), 'src', 'stores', 'conversation.ts');
  expect(existsSync(convPath)).toBe(true);
  const conv = readFileSync(convPath, 'utf8');

  // The background classifier handler must exist and call startLearnDocsFlow.
  expect(conv).toContain('startLearnDocsFlow');

  // learn_with_docs must NOT be gated by !streaming.streamText — the old
  // race-condition guard that silently dropped the quest when the LLM
  // started streaming before the classifier resolved.
  // The correct pattern: only gated_setup checks streaming.streamText.
  const bgHandlerMatch = conv.match(
    /classifyPromise\.then\(\(d\)\s*=>\s*\{[\s\S]{0,800}learn_with_docs/,
  );
  expect(bgHandlerMatch).toBeTruthy();

  // Verify the old broken guard is gone: learn_with_docs must not appear
  // after a condition that checks `!streaming.streamText`.
  const brokenGuard = conv.match(
    /!streaming\.streamText[\s\S]{0,200}learn_with_docs/,
  );
  expect(brokenGuard).toBeFalsy();

  // Post-stream classifier check: after the stream completes, the code must
  // await the classifier result and handle side-channel intents BEFORE
  // pushing the assistant message. Without this, the chat response appears
  // instead of the quest when LocalOllama serializes requests.
  const postStreamCheck = conv.match(
    /Post-stream classifier check[\s\S]{0,1500}classifyPromise[\s\S]{0,500}learn_with_docs[\s\S]{0,200}startLearnDocsFlow/,
  );
  expect(postStreamCheck).toBeTruthy();
});

/**
 * Code-level contract: the Rust intent classifier must give LocalOllama
 * enough timeout to respond under GPU contention (when the chat stream
 * holds the model). The default cloud timeout (1500ms) is too short.
 */
test('intent classifier has a local-model timeout longer than the cloud timeout', async () => {
  const classifierPath = path.join(
    process.cwd(), 'src-tauri', 'src', 'brain', 'intent_classifier.rs',
  );
  expect(existsSync(classifierPath)).toBe(true);
  const classifier = readFileSync(classifierPath, 'utf8');

  // Cloud timeout
  const cloudMatch = classifier.match(/CLASSIFY_TIMEOUT:\s*Duration\s*=\s*Duration::from_millis\((\d+)\)/);
  expect(cloudMatch).toBeTruthy();
  const cloudMs = Number(cloudMatch![1]);

  // Local timeout must exist and be longer than the cloud timeout
  const localMatch = classifier.match(/LOCAL_CLASSIFY_TIMEOUT:\s*Duration\s*=\s*Duration::from_millis\((\d+)\)/);
  expect(localMatch).toBeTruthy();
  const localMs = Number(localMatch![1]);

  expect(localMs).toBeGreaterThan(cloudMs);

  // The local timeout must actually be used for LocalOllama/LocalLmStudio
  expect(classifier).toContain('LocalOllama');
  expect(classifier).toContain('LOCAL_CLASSIFY_TIMEOUT');
});

// ═══════════════════════════════════════════════════════════════════════════
// Layer 3 — Playwright browser tests
//
// These tests mock `window.__TAURI_INTERNALS__` so the conversation store's
// `classify_intent` invoke resolves synchronously with `learn_with_docs`.
// The classifier result short-circuits BEFORE the E2E persona fallback and
// BEFORE the streaming path, so we verify the real quest UI end-to-end.
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Install a deterministic Tauri invoke mock for the tutorial flow:
 * - classifier intent routing
 * - Scholar's Quest verification/ingestion commands
 * - fallback chat answer after ingestion (query test)
 */
async function mockTutorialKnowledgeFlow(page: import('@playwright/test').Page) {
  await page.evaluate(() => {
    const pinia = (document.querySelector('#app') as any)?.__vue_app__?.config?.globalProperties?.$pinia;
    if (pinia?.state?.value?.brain) {
      pinia.state.value.brain = {
        ...pinia.state.value.brain,
        hasBrain: true,
        brainMode: {
          mode: 'free_api',
          provider_id: 'mock-provider',
          model: 'mock-model',
        },
      };
    }

    const memories: any[] = [
      {
        id: 9001,
        content: 'Article 429: The statute of limitations for contractual disputes is three years from when rights were infringed.',
        tags: 'vietnamese-law,article-429,contract',
        importance: 5,
        memory_type: 'fact',
        created_at: Date.now() - 10_000,
        last_accessed: null,
        access_count: 0,
        tier: 'long',
        decay_score: 0.95,
        session_id: null,
        parent_id: null,
        token_count: 28,
      },
    ];
    let nextTask = 1;

    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: any) => {
        if (cmd === 'classify_intent') {
          const text = String(args?.text ?? '').toLowerCase();
          if (text.includes('learn') && text.includes('document')) {
            return Promise.resolve({ kind: 'learn_with_docs', topic: 'my documents' });
          }
          return Promise.resolve({ kind: 'chat' });
        }

        if (cmd === 'get_memories') return Promise.resolve(memories);
        if (cmd === 'get_all_tasks') return Promise.resolve([]);

        if (cmd === 'add_memory') {
          const entry = {
            id: Date.now(),
            content: args?.content ?? 'memory',
            tags: args?.tags ?? '',
            importance: args?.importance ?? 3,
            memory_type: args?.memoryType ?? 'fact',
            created_at: Date.now(),
            last_accessed: null,
            access_count: 0,
            tier: args?.tier ?? 'short',
            decay_score: 1,
            session_id: null,
            parent_id: null,
            token_count: 20,
          };
          memories.push(entry);
          return Promise.resolve(entry);
        }

        if (cmd === 'ingest_document') {
          const src = String(args?.source ?? '');
          const sourceName = src.length ? src : `source-${nextTask}`;
          memories.push({
            id: Date.now() + nextTask,
            content: `Learned source: ${sourceName}. Article 429 covers a three-year statute of limitations for contract disputes.`,
            tags: String(args?.tags ?? 'knowledge'),
            importance: Number(args?.importance ?? 5),
            memory_type: 'fact',
            created_at: Date.now(),
            last_accessed: null,
            access_count: 0,
            tier: 'long',
            decay_score: 0.98,
            session_id: null,
            parent_id: null,
            token_count: 30,
          });
          const task_id = `task-${nextTask++}`;
          return Promise.resolve({ task_id, source: sourceName, source_type: src.startsWith('http') ? 'url' : 'file' });
        }

        // Force streaming path to fail fast so conversation store uses send_message fallback.
        if (cmd === 'send_message_stream') {
          return Promise.reject(new Error('mock stream unavailable'));
        }

        if (cmd === 'send_message') {
          const message = String(args?.message ?? '').toLowerCase();
          if (message.includes('statute of limitations') || message.includes('article 429')) {
            return Promise.resolve({
              id: `assistant-${Date.now()}`,
              role: 'assistant',
              content: 'Article 429 says the statute of limitations for contract disputes is three years from when the claimant knew or should have known their rights were infringed.',
              agentName: 'TerranSoul',
              sentiment: 'neutral',
              timestamp: Date.now(),
            });
          }
          return Promise.resolve({
            id: `assistant-${Date.now()}`,
            role: 'assistant',
            content: 'I learned your submitted sources and I am ready for questions.',
            agentName: 'TerranSoul',
            sentiment: 'neutral',
            timestamp: Date.now(),
          });
        }

        if (cmd === 'plugin:event|listen') return Promise.resolve(0);
        if (cmd === 'plugin:event|unlisten') return Promise.resolve();

        // Default permissive mock: avoid unrelated startup noise failing the test.
        return Promise.resolve(null);
      },
    };
  });
}

/** Remove the Tauri mock so subsequent tests are clean. */
async function clearTauriMock(page: import('@playwright/test').Page) {
  await page.evaluate(() => {
    delete (window as any).__TAURI_INTERNALS__;
  });
}

/**
 * Tutorial scenario: "Learn from my documents" triggers Scholar's Quest.
 *
 * Matches tutorial step 5 → 6:
 *   5. Type "Learn from my documents" and send.
 *   6. Scholar's Quest appears with source upload buttons.
 */
test('Learn from my documents triggers Scholar\'s Quest UI', async ({ page }) => {
  const errors = collectConsoleErrors(page);
  await page.goto('/');
  await waitForAppReady(page);

  await mockTutorialKnowledgeFlow(page);

  // Send the exact phrase from the tutorial
  await sendMessage(page, 'Learn from my documents');

  // Wait for a quest message with actionable choices.
  await expect(async () => {
    const state = (await getPiniaState(page, 'conversation')) as any;
    const messages = state?.messages ?? [];
    const lastAssistant = [...messages].reverse().find((m: any) => m.role === 'assistant');
    expect(lastAssistant).toBeTruthy();
    // Must be a quest message (scholar-quest or learn-docs-missing), not plain chat
    expect(lastAssistant.questId).toBeTruthy();
    expect(lastAssistant.questChoices?.length).toBeGreaterThan(0);
  }).toPass({ timeout: TIMEOUTS.response });

  // Verify the quest choice overlay is visible in the UI
  const hotseat = page.locator('.hotseat-strip');
  await expect(hotseat).toBeVisible({ timeout: 5_000 });

  // Some states first ask for auto-install; complete it to reach Scholar's Quest.
  const labels = await hotseat.locator('.hotseat-tile-label').allTextContents();
  if (labels.some((l) => l.toLowerCase().includes('auto install all'))) {
    await hotseat.locator('.hotseat-tile', { hasText: 'Auto install all' }).click();
    await expect(hotseat.locator('.hotseat-tile-label', { hasText: 'Start Knowledge Quest' })).toBeVisible({ timeout: 8_000 });
  }

  await hotseat.locator('.hotseat-tile', { hasText: 'Start Knowledge Quest' }).click();

  const questDialog = page.locator('.kq-dialog');
  await expect(questDialog).toBeVisible({ timeout: 5_000 });

  // Step 1 -> 2
  await questDialog.locator('.kq-btn-primary', { hasText: 'Continue' }).click();

  // Step 2: submit a source (knowledge submission).
  await questDialog.locator('.kq-url-field').fill('https://example.com/vietnamese-civil-code.html');
  await questDialog.locator('.kq-url-add').click();
  await questDialog.locator('.kq-btn-primary', { hasText: 'Start Learning' }).click();

  // Step 3 auto-advances when tasks complete, then user confirms "Ask Questions".
  await expect(questDialog.locator('.kq-btn-glow', { hasText: 'Ask Questions' })).toBeVisible({ timeout: 12_000 });
  await questDialog.locator('.kq-btn-glow', { hasText: 'Ask Questions' }).click();
  await expect(questDialog).not.toBeVisible({ timeout: 5_000 });

  // Completion message proves submission flow finished.
  await expect(async () => {
    const state = (await getPiniaState(page, 'conversation')) as any;
    const messages = state?.messages ?? [];
    const lastAssistant = [...messages].reverse().find((m: any) => m.role === 'assistant');
    expect(lastAssistant?.content ?? '').toContain("Scholar's Quest Complete");
  }).toPass({ timeout: TIMEOUTS.response });

  // Query the newly submitted knowledge and verify a grounded answer.
  await sendMessage(page, 'What is the statute of limitations for contract disputes under Vietnamese law?');
  const answer = await waitForAssistantResponse(page);
  expect(answer).toContain('Article 429');
  expect(answer.toLowerCase()).toContain('three years');

  // The answer must NOT be a no-brain persona fallback.
  expect(answer).not.toContain('Hello! I\'m TerranSoul');

  await clearTauriMock(page);
  assertNoCrashErrors(errors);
});

/**
 * Phrase variations that should all trigger the same learn-with-docs flow.
 * The tutorial says "Learn from my documents" but users may say it differently.
 */
const LEARN_DOC_PHRASES = [
  'Learn my documents',
  'Learn documents',
  'Please look at my provided documents and learn it',
];

for (const phrase of LEARN_DOC_PHRASES) {
  test(`phrase variation "${phrase}" triggers quest UI`, async ({ page }) => {
    const errors = collectConsoleErrors(page);
    await page.goto('/');
    await waitForAppReady(page);

    await mockTutorialKnowledgeFlow(page);

    await sendMessage(page, phrase);

    // Quest message must appear with choices
    await expect(async () => {
      const state = (await getPiniaState(page, 'conversation')) as any;
      const messages = state?.messages ?? [];
      const lastAssistant = [...messages].reverse().find((m: any) => m.role === 'assistant');
      expect(lastAssistant).toBeTruthy();
      expect(lastAssistant.questId).toBeTruthy();
      expect(lastAssistant.questChoices?.length).toBeGreaterThan(0);
    }).toPass({ timeout: TIMEOUTS.response });

    // Hotseat overlay must be visible
    await expect(page.locator('.hotseat-strip')).toBeVisible({ timeout: 5_000 });

    await clearTauriMock(page);
    assertNoCrashErrors(errors);
  });
}

/**
 * Negative test: a plain chat message must NOT trigger the quest flow.
 * When the classifier returns "chat", the user should get a normal response.
 */
test('plain chat does not trigger quest flow', async ({ page }) => {
  const errors = collectConsoleErrors(page);
  await page.goto('/');
  await waitForAppReady(page);

  // Don't mock — default browser mode returns { kind: 'chat' } for everything
  await sendMessage(page, 'Hello there!');

  // Wait for a normal assistant response
  await expect(async () => {
    const state = (await getPiniaState(page, 'conversation')) as any;
    const messages = state?.messages ?? [];
    const lastAssistant = [...messages].reverse().find((m: any) => m.role === 'assistant');
    expect(lastAssistant).toBeTruthy();
    expect(lastAssistant.content?.length).toBeGreaterThan(0);
    // Must NOT have quest choices — this is a normal chat turn
    expect(lastAssistant.questId).toBeFalsy();
  }).toPass({ timeout: TIMEOUTS.response });

  // Hotseat should NOT be visible
  const hotseat = page.locator('.hotseat-strip');
  await expect(hotseat).not.toBeVisible();

  assertNoCrashErrors(errors);
});
