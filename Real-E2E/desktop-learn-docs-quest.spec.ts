/**
 * Real desktop E2E: document-learning quest trigger.
 *
 * Connects to the running Tauri WebView through WebView2 CDP and drives
 * the actual UI/backend path. No Tauri mocks, no Pinia patching, no synthetic
 * memories.
 */
import { test, expect, type Page } from '@playwright/test';
import { completeFirstLaunchRecommendedIfPresent, connectToDesktopApp } from './helpers';

const LEARN_DOCS_PHRASE = 'Learn from my documents';

type RuntimeSnapshot = {
  tauriAvailable: boolean;
  brainMode: unknown;
};

type QuestSnapshot = {
  questId: string | null;
  content: string;
  choices: string[];
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

async function closeExistingQuestUi(page: Page): Promise<void> {
  await completeFirstLaunchRecommendedIfPresent(page);

  const closeButton = page.locator('.kq-dialog .kq-close').first();
  if (await closeButton.isVisible({ timeout: 500 }).catch(() => false)) {
    await closeButton.click();
    await expect(page.locator('.kq-dialog')).not.toBeVisible({ timeout: 5_000 });
  }

  const cancelTile = page.locator('.hotseat-tile', { hasText: 'Cancel' }).first();
  if (await cancelTile.isVisible({ timeout: 500 }).catch(() => false)) {
    await cancelTile.click();
  }
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
    };
  }, marker);
}

async function invokeClassifyIntent(page: Page, text: string): Promise<any> {
  return page.evaluate((input) => {
    return (window as any).__TAURI_INTERNALS__.invoke('classify_intent', { text: input });
  }, text);
}

async function sendDesktopMessage(page: Page, text: string): Promise<void> {
  const input = page.locator('.chat-input').first();
  const sendButton = page.locator('.send-btn').first();
  await input.fill(text);
  await expect(sendButton).toBeEnabled({ timeout: 2_000 });
  await sendButton.click();
}

async function openGatherSourcesStep(page: Page): Promise<void> {
  const hotseat = page.locator('.hotseat-strip');
  await expect(hotseat).toBeVisible({ timeout: 5_000 });

  const autoInstall = hotseat.locator('.hotseat-tile', { hasText: 'Auto install all' }).first();
  if (await autoInstall.isVisible({ timeout: 1_000 }).catch(() => false)) {
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

test('real desktop Learn from my documents opens URL and file attachment controls', async () => {
  test.setTimeout(90_000);
  const { browser, page } = await connectToDesktopApp();
  try {
    await closeExistingQuestUi(page);

    const runtime = await runtimeSnapshot(page);
    expect(runtime.tauriAvailable).toBe(true);

    const decision = await invokeClassifyIntent(page, LEARN_DOCS_PHRASE);
    expect(decision).toMatchObject({
      kind: 'learn_with_docs',
      topic: 'the material in your documents',
    });

    const marker = await messageCount(page);
    await sendDesktopMessage(page, LEARN_DOCS_PHRASE);

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
  } finally {
    await browser.close();
  }
});
