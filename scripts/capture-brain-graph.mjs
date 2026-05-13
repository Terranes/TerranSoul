#!/usr/bin/env node
/**
 * capture-brain-graph.mjs
 *
 * Iterative visual capture for MemoryGraph3D ("3D" memory graph, formerly BrainGraphViewport).
 *
 * Launches a Chromium instance via Playwright against the Vite dev server,
 * injects synthetic memories + edges directly into the Pinia memory store,
 * switches to the Memory → Graph → 3D view, and saves a PNG screenshot.
 *
 * Usage:
 *   node scripts/capture-brain-graph.mjs [out.png]
 *
 * Assumes the Vite dev server is already running on http://localhost:1420
 * (start with `npm run dev`).
 */
import { chromium } from 'playwright';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const outPath = process.argv[2]
  ? path.resolve(process.cwd(), process.argv[2])
  : path.resolve(__dirname, '..', 'recording', 'brain-graph-3d.png');

const URL = process.env.TERRANSOUL_DEV_URL || 'http://localhost:1420/?panel=memory';
const WIDTH = Number(process.env.TS_GRAPH_WIDTH || 1280);
const HEIGHT = Number(process.env.TS_GRAPH_HEIGHT || 800);
const SETTLE_MS = Number(process.env.TS_GRAPH_SETTLE_MS || 3500);
const COUNT = Number(process.env.TS_GRAPH_COUNT || 140);

function buildFakeMemories(count) {
  const kinds = ['semantic', 'procedural', 'episodic', 'analytical', 'principle'];
  const tagTopics = [
    'rust', 'vue', 'three', 'webgl', 'memory', 'graph', 'embed',
    'router', 'shader', 'ann', 'rag', 'mcp', 'persona', 'voice',
    'quest', 'skill', 'ui', 'css', 'tauri', 'sqlite',
  ];
  const memories = [];
  for (let i = 0; i < count; i++) {
    const kind = kinds[i % kinds.length];
    const topic = tagTopics[i % tagTopics.length];
    // dominantTag() picks the first "x:y" tag, so use `topic:detail` to make
    // each topic become its own community/cluster.
    memories.push({
      id: i + 1,
      content: `Synthetic ${kind} memory #${i + 1} about ${topic}`,
      summary: null,
      tags: `${topic}:item-${i}, kind:${kind}`,
      tier: 'long',
      memory_type: kind,
      importance: 1 + (i % 9),
      decay_score: Math.random() * 0.6,
      created_at: Date.now() - i * 60000,
      updated_at: Date.now() - i * 60000,
      last_accessed: Date.now() - i * 30000,
      access_count: i % 10,
      source: 'synthetic',
      conversation_id: null,
      embedding_status: 'embedded',
    });
  }
  return memories;
}

function buildFakeEdges(memories) {
  const edges = [];
  let id = 1;
  // Connect each node to its nearest neighbours by topic + a few random links.
  const byTopic = new Map();
  for (const m of memories) {
    const topic = (m.tags || '').split(',')[0].trim();
    if (!byTopic.has(topic)) byTopic.set(topic, []);
    byTopic.get(topic).push(m.id);
  }
  for (const ids of byTopic.values()) {
    for (let i = 0; i < ids.length - 1; i++) {
      edges.push({
        id: id++,
        src_id: ids[i],
        dst_id: ids[i + 1],
        rel_type: 'related-to',
        weight: 0.6,
        created_at: Date.now(),
      });
    }
  }
  // Random cross-topic edges for inter-cluster links.
  for (let i = 0; i < memories.length / 4; i++) {
    const a = memories[Math.floor(Math.random() * memories.length)].id;
    const b = memories[Math.floor(Math.random() * memories.length)].id;
    if (a === b) continue;
    edges.push({
      id: id++,
      src_id: a,
      dst_id: b,
      rel_type: ['references', 'contradicts', 'supports', 'derives-from'][i % 4],
      weight: 0.3 + Math.random() * 0.4,
      created_at: Date.now(),
    });
  }
  return edges;
}

async function main() {
  console.log(`[capture] Launching Chromium → ${URL}`);
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: WIDTH, height: HEIGHT },
    deviceScaleFactor: 2,
  });
  const page = await context.newPage();

  page.on('pageerror', (err) => console.error('[page error]', err.message));
  page.on('console', (msg) => {
    const t = msg.text();
    if (t.includes('[BrainGraph]')) console.log('[browser]', t);
    if (msg.type() === 'error') console.error('[console.error]', t);
  });

  await page.goto(URL, { waitUntil: 'domcontentloaded', timeout: 60_000 });
  await page.waitForSelector('#app', { timeout: 30_000 });
  // Give Vue + Pinia time to mount.
  await page.waitForTimeout(1500);

  // panel=memory bypasses the landing + first-launch wizard — wait for MV.
  await page.locator('.memory-view, .mv-tabs').first().waitFor({ state: 'visible', timeout: 30_000 });
  const scope = page.locator('body');

  console.log(`[capture] Injecting ${COUNT} synthetic memories + edges`);
  const memories = buildFakeMemories(COUNT);
  const edges = buildFakeEdges(memories);
  await page.evaluate(({ memories: fakeMemories, edges: fakeEdges }) => {
    const app = document.querySelector('#app')?.__vue_app__;
    const pinia = app?.config?.globalProperties?.$pinia;
    const memoryStore = pinia?._s?.get('memory');
    if (!memoryStore) throw new Error('memory store not found on pinia');
    memoryStore.memories = fakeMemories;
    memoryStore.edges = fakeEdges;
    memoryStore.edgeStats = { total_edges: fakeEdges.length, connected_memories: fakeMemories.length };
  }, { memories, edges });

  // Switch to Graph sub-tab.
  const graphTab = scope.locator('.mv-tab', { hasText: 'Graph' }).first();
  await graphTab.waitFor({ state: 'visible', timeout: 10_000 });
  await graphTab.click();

  // Click 3D toggle.
  const threeDBtn = scope.locator('[data-testid="mv-graph-3d-toggle"]').first();
  await threeDBtn.waitFor({ state: 'visible', timeout: 5000 });
  await threeDBtn.click();

  // Wait for the WebGL canvas inside MemoryGraph3D to mount and settle.
  const canvas = scope.locator('.brain-graph-viewport canvas').first();
  await canvas.waitFor({ state: 'visible', timeout: 10_000 });
  await page.waitForTimeout(SETTLE_MS);

  // Probe rendered scene state for diagnostics.
  const probe = await page.evaluate(() => {
    const c = document.querySelector('.brain-graph-viewport canvas');
    const r = c?.getBoundingClientRect();
    return { width: r?.width, height: r?.height };
  });
  console.log('[capture] canvas size:', probe);

  console.log(`[capture] Saving → ${outPath}`);
  const container = scope.locator('.brain-graph-viewport').first();
  await container.screenshot({ path: outPath });

  await browser.close();
  console.log('[capture] Done.');
}

main().catch((err) => {
  console.error('[capture] FATAL:', err);
  process.exit(1);
});
