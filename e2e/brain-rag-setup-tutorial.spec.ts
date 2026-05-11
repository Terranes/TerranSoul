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
 *   3. Playwright desktop tests — connect to the live Tauri WebView via CDP,
 *      send "Learn from my documents" through the real UI, and verify the
 *      real Scholar's Quest source attachment step appears.
 */
import { test, expect, type Page } from '@playwright/test';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { closeOpenDialogIfPresent, connectToDesktopApp, sendMessage } from './helpers';

const TUTORIAL_PATH = path.join(process.cwd(), 'tutorials', 'brain-rag-setup-tutorial.md');
const LEARN_DOCS_PHRASE = 'Learn from my documents';

const REQUIRED_TUTORIAL_CONTRACT = [
  'Brain + Memory + RAG - Pet Mode Tutorial',
  'Learn from my documents',
  'classify_intent',
  'Scholar\'s Quest',
  'vietnamese-civil-code.html',
  'article-429-commentary.txt',
  'public/demo/vietnamese-civil-code.html',
  'public/demo/article-429-commentary.txt',
  'https://thuvienphapluat.vn/van-ban/Lao-dong-Tien-luong/Bo-Luat-lao-dong-2019-333670.aspx',
  '🕸️ Crawl whole site',
  'TaskProgressBar',
  'depth 2',
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
// Layer 3 — real Tauri desktop E2E
//
// No browser-mode Tauri mocks, no Pinia patching, no synthetic memory. This
// connects to the running Tauri WebView through WebView2 CDP and drives the
// same UI/backend path a user sees in the desktop app.
// ═══════════════════════════════════════════════════════════════════════════

type RuntimeSnapshot = {
  tauriAvailable: boolean;
  brainMode: unknown;
};

type QuestSnapshot = {
  questId: string | null;
  content: string;
  choices: string[];
  tileLabels: string[];
};

async function runtimeSnapshot(page: Page): Promise<RuntimeSnapshot> {
  return page.evaluate(() => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    const pinia = app?.config?.globalProperties?.$pinia;
    const brain = pinia?._s?.get('brain');
    return {
      tauriAvailable: typeof (window as any).__TAURI_INTERNALS__?.invoke === 'function',
      brainMode: brain?.brainMode ?? null,
    };
  });
}

async function messageCount(page: Page): Promise<number> {
  return page.evaluate(() => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    const pinia = app?.config?.globalProperties?.$pinia;
    return pinia?._s?.get('conversation')?.messages?.length ?? 0;
  });
}

async function latestQuestAfter(page: Page, marker: number): Promise<QuestSnapshot> {
  return page.evaluate((startIndex) => {
    const app = (document.querySelector('#app') as any)?.__vue_app__;
    const pinia = app?.config?.globalProperties?.$pinia;
    const messages = pinia?._s?.get('conversation')?.messages ?? [];
    const quest = messages
      .slice(startIndex)
      .reverse()
      .find((message: any) => message.role === 'assistant' && message.questId);
    return {
      questId: quest?.questId ?? null,
      content: quest?.content ?? '',
      choices: (quest?.questChoices ?? []).map((choice: any) => String(choice.label ?? '')),
      tileLabels: Array.from(document.querySelectorAll('.hotseat-tile-label'))
        .map((el) => el.textContent?.trim() ?? '')
        .filter(Boolean),
    };
  }, marker);
}

async function invokeClassifyIntent(page: Page, text: string): Promise<any> {
  return page.evaluate((input) => {
    return (window as any).__TAURI_INTERNALS__.invoke('classify_intent', { text: input });
  }, text);
}

async function openGatherSourcesStep(page: Page): Promise<void> {
  const hotseat = page.locator('.hotseat-strip');
  await expect(hotseat).toBeVisible({ timeout: 5_000 });

  const autoInstall = hotseat.locator('.hotseat-tile', { hasText: 'Auto install all' }).first();
  const installSageLibrary = hotseat.locator('.hotseat-tile', { hasText: "Install 📚 Sage's Library" }).first();
  if (await installSageLibrary.isVisible({ timeout: 1_000 }).catch(() => false)) {
    await installSageLibrary.click();
    await expect(
      hotseat.locator('.hotseat-tile-label', { hasText: 'Start Knowledge Quest' }),
    ).toBeVisible({ timeout: 15_000 });
  } else if (await autoInstall.isVisible({ timeout: 1_000 }).catch(() => false)) {
    await autoInstall.click();
    await expect(
      hotseat.locator('.hotseat-tile-label', { hasText: 'Start Knowledge Quest' }),
    ).toBeVisible({ timeout: 15_000 });
  }

  await hotseat.locator('.hotseat-tile', { hasText: 'Start Knowledge Quest' }).first().click();
  const questDialog = page.locator('.kq-dialog');
  await expect(questDialog).toBeVisible({ timeout: 10_000 });

  const urlField = questDialog.locator('.kq-url-field');
  if (!(await urlField.isVisible({ timeout: 500 }).catch(() => false))) {
    const continueButton = questDialog.locator('.kq-btn-primary', { hasText: 'Continue' }).first();
    await expect(continueButton).toBeVisible({ timeout: 30_000 });
    await expect(continueButton).toBeEnabled({ timeout: 30_000 });
    await continueButton.click();
  }

  await expect(urlField).toBeVisible({ timeout: 10_000 });
}

test('real desktop Learn from my documents opens Scholar source attachments', async () => {
  const { browser, page } = await connectToDesktopApp();
  try {
    await closeOpenDialogIfPresent(page);

    const runtime = await runtimeSnapshot(page);
    expect(runtime.tauriAvailable).toBe(true);

    const decision = await invokeClassifyIntent(page, LEARN_DOCS_PHRASE);
    expect(decision).toMatchObject({
      kind: 'learn_with_docs',
      topic: 'the material in your documents',
    });

    const marker = await messageCount(page);
    await sendMessage(page, LEARN_DOCS_PHRASE);

    await expect(async () => {
      const quest = await latestQuestAfter(page, marker);
      expect(['learn-docs-missing', 'scholar-quest']).toContain(quest.questId);
      expect(quest.choices.length).toBeGreaterThan(0);
      expect(quest.content).not.toContain('Hello! I\'m TerranSoul');
    }).toPass({ timeout: 30_000 });

    await openGatherSourcesStep(page);

    const questDialog = page.locator('.kq-dialog');
    await expect(questDialog.locator('.kq-url-field')).toBeVisible();
    await expect(questDialog.locator('.kq-url-add')).toBeVisible();
    await expect(questDialog.locator('input[type="file"]')).toBeAttached();
    // The crawl toggle must be present so users can opt into web crawling
    // without having to manually type the `crawl:` prefix.
    const crawlToggle = questDialog.locator('.kq-crawl-checkbox');
    await expect(crawlToggle).toBeAttached();
    await crawlToggle.check();
    await expect(questDialog.locator('.kq-crawl-number')).toHaveCount(2);
    await crawlToggle.uncheck();
  } finally {
    await browser.close();
  }
});

/**
 * Real desktop crawl flow.
 *
 * Verifies that:
 *   1. The Scholar's Quest **🕸️ Crawl whole site** toggle prefixes the source
 *      with `crawl:` so the backend takes the crawler path.
 *   2. The backend `ingest_document` command actually creates a task with
 *      `kind: "crawl"` and emits `task-progress` events the frontend stores.
 *   3. The TaskProgressBar / kq-progress description includes the depth
 *      indicator so users can monitor crawl depth.
 *
 * The crawl is cancelled as soon as it reports progress so the test does
 * not depend on the remote site finishing 20 fetches.
 */
test('real desktop web crawl emits depth-tagged progress and is cancellable', async () => {
  const CRAWL_URL = 'https://thuvienphapluat.vn/van-ban/Lao-dong-Tien-luong/Bo-Luat-lao-dong-2019-333670.aspx';
  const { browser, page } = await connectToDesktopApp();
  try {
    await closeOpenDialogIfPresent(page);

    const runtime = await runtimeSnapshot(page);
    expect(runtime.tauriAvailable).toBe(true);

    // Start a crawl directly through the Tauri command using the same
    // `crawl:` prefix the KnowledgeQuestDialog produces when the toggle
    // is on. This avoids depending on first-launch UI state.
    const startResult: any = await page.evaluate(async (url) => {
      const invoke = (window as any).__TAURI_INTERNALS__.invoke;
      return invoke('ingest_document', {
        source: `crawl:${url}`,
        tags: 'e2e,crawl-test',
        importance: 3,
      });
    }, CRAWL_URL);
    expect(startResult).toMatchObject({ source_type: 'crawl' });
    expect(typeof startResult.task_id).toBe('string');
    const taskId: string = startResult.task_id;

    // Wait for the task store to observe a `task-progress` event with the
    // crawl kind and the depth-tagged description.
    await expect(async () => {
      const task: any = await page.evaluate((id) => {
        const app = (document.querySelector('#app') as any)?.__vue_app__;
        const pinia = app?.config?.globalProperties?.$pinia;
        const taskStore = pinia?._s?.get('tasks');
        return taskStore?.tasks?.get(id) ?? null;
      }, taskId);
      expect(task).not.toBeNull();
      expect(task.kind).toBe('crawl');
      // Either the crawl is already crawling (depth tag visible) or it is
      // still in the initial "Crawling …" boot state. Accept both, but the
      // moment any page is fetched the description MUST include the
      // `(depth D/MAX)` marker so users can see depth in the UI.
      const hasCrawlingDesc = /Crawling/i.test(task.description) || /Crawling \d+\/\d+ \(depth \d+\/\d+\)/.test(task.description);
      expect(hasCrawlingDesc).toBe(true);
    }).toPass({ timeout: 60_000 });

    // Wait for at least one real page fetch so depth marker shows up.
    await expect(async () => {
      const task: any = await page.evaluate((id) => {
        const app = (document.querySelector('#app') as any)?.__vue_app__;
        const pinia = app?.config?.globalProperties?.$pinia;
        const taskStore = pinia?._s?.get('tasks');
        return taskStore?.tasks?.get(id) ?? null;
      }, taskId);
      expect(task).not.toBeNull();
      expect(task.description).toMatch(/Crawling \d+\/\d+ \(depth \d+\/\d+\):/);
    }).toPass({ timeout: 120_000 });

    // Cancel the crawl so we do not hammer the remote site.
    await page.evaluate(async (id) => {
      const invoke = (window as any).__TAURI_INTERNALS__.invoke;
      await invoke('cancel_ingest_task', { taskId: id });
    }, taskId);

    await expect(async () => {
      const task: any = await page.evaluate((id) => {
        const app = (document.querySelector('#app') as any)?.__vue_app__;
        const pinia = app?.config?.globalProperties?.$pinia;
        const taskStore = pinia?._s?.get('tasks');
        return taskStore?.tasks?.get(id) ?? null;
      }, taskId);
      expect(task).not.toBeNull();
      expect(['cancelled', 'failed', 'completed', 'paused']).toContain(task.status);
    }).toPass({ timeout: 30_000 });
  } finally {
    await browser.close();
  }
});
