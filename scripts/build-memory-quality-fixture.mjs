#!/usr/bin/env node
// SPDX-License-Identifier: MIT
//
// Build a JSON fixture for the memory-quality retrieval benchmark by
// downloading an MIT-licensed concept-tagged corpus (originally published by
// rohitg00/agentmemory at a pinned commit), transpiling it with esbuild, and
// running its generateDataset() to produce the canonical 240-observation /
// 20-query corpus. The same fixture is later evaluated against multiple
// top-tier memory systems in benchmark/COMPARISON.md.
//
// Output: src-tauri/benches/memory_quality_fixture.json
//
// Upstream: https://github.com/rohitg00/agentmemory/blob/main/benchmark/dataset.ts
// License of dataset: MIT (see CREDITS.md entry for rohitg00/agentmemory).

import { writeFileSync, mkdirSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { build } from 'esbuild';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '..');

// Pinned commit so the fixture is reproducible across sessions.
// Update by checking https://github.com/rohitg00/agentmemory/commits/main/benchmark/dataset.ts
const PINNED_COMMIT = 'ae8f061cd66093d7be1539c24da6d3e595531dd2';
const DATASET_URL = `https://raw.githubusercontent.com/rohitg00/agentmemory/${PINNED_COMMIT}/benchmark/dataset.ts`;

const CACHE_DIR = resolve(REPO_ROOT, '.cache', 'memory-quality-bench');
const SOURCE_PATH = resolve(CACHE_DIR, 'dataset.ts');
const STUB_TYPES_PATH = resolve(CACHE_DIR, 'types.ts');
const BUILT_PATH = resolve(CACHE_DIR, 'dataset.mjs');
const FIXTURE_PATH = resolve(
  REPO_ROOT,
  'src-tauri',
  'benches',
  'memory_quality_fixture.json',
);

async function downloadSource() {
  mkdirSync(CACHE_DIR, { recursive: true });
  const res = await fetch(DATASET_URL);
  if (!res.ok) {
    throw new Error(`failed to fetch dataset.ts (HTTP ${res.status}): ${DATASET_URL}`);
  }
  let src = await res.text();
  // Replace the upstream import of `../src/types.js` with a local stub —
  // we only need the type names, not any runtime code from that module.
  src = src.replace(
    /from\s+"\.\.\/src\/types\.js"/g,
    `from "./types.js"`,
  );
  writeFileSync(SOURCE_PATH, src, 'utf8');
  // Stub the only type the dataset uses: CompressedObservation, GraphNode,
  // GraphEdge, GraphEdgeType. Runtime sees them as `any`.
  writeFileSync(
    STUB_TYPES_PATH,
    'export type CompressedObservation = any;\n' +
      'export type GraphNode = any;\n' +
      'export type GraphEdge = any;\n' +
      'export type GraphEdgeType = any;\n',
    'utf8',
  );
}

async function transpile() {
  await build({
    entryPoints: [SOURCE_PATH],
    bundle: true,
    format: 'esm',
    platform: 'node',
    target: 'node20',
    outfile: BUILT_PATH,
    logLevel: 'silent',
  });
}

async function emitFixture() {
  const mod = await import(pathToFileURL(BUILT_PATH).href);
  const { observations, queries, sessions } = mod.generateDataset();

  // Compute relevant_count per query and assemble a stable JSON payload.
  // The upstream dataset randomises some session timestamps via Date.now();
  // we re-anchor every timestamp to a fixed epoch so reruns are byte-stable.
  const ANCHOR = Date.parse('2026-01-01T00:00:00.000Z');
  for (const obs of observations) {
    // Replace absolute timestamps with a stable offset (days_ago is encoded
    // in the original generator, so we keep the relative spacing).
    const dt = Date.parse(obs.timestamp);
    obs.timestamp = new Date(
      ANCHOR - (Date.now() - dt),
    ).toISOString();
  }

  const fixture = {
    source: DATASET_URL,
    pinned_commit: PINNED_COMMIT,
    license: 'MIT (rohitg00/agentmemory)',
    generated_at: new Date(ANCHOR).toISOString(),
    observations,
    queries,
    sessions: Object.fromEntries(sessions),
  };

  mkdirSync(dirname(FIXTURE_PATH), { recursive: true });
  writeFileSync(FIXTURE_PATH, JSON.stringify(fixture, null, 2), 'utf8');

  return {
    observations: observations.length,
    queries: queries.length,
    sessions: sessions.size,
  };
}

async function main() {
  console.log(`[fixture] downloading ${DATASET_URL}`);
  await downloadSource();
  console.log(`[fixture] transpiling with esbuild → ${BUILT_PATH}`);
  await transpile();
  console.log(`[fixture] running generateDataset()`);
  const stats = await emitFixture();
  console.log(
    `[fixture] wrote ${FIXTURE_PATH}\n` +
      `          observations=${stats.observations} queries=${stats.queries} sessions=${stats.sessions}`,
  );

  // Sanity-check the result: every query must have at least 1 relevant id and
  // every relevant id must exist in observations.
  const raw = JSON.parse(readFileSync(FIXTURE_PATH, 'utf8'));
  const obsIds = new Set(raw.observations.map((o) => o.id));
  let totalRel = 0;
  for (const q of raw.queries) {
    if (q.relevantObsIds.length === 0) {
      throw new Error(`[fixture] query has no relevant ids: ${q.query}`);
    }
    for (const id of q.relevantObsIds) {
      if (!obsIds.has(id)) {
        throw new Error(`[fixture] query "${q.query}" references unknown id ${id}`);
      }
    }
    totalRel += q.relevantObsIds.length;
  }
  console.log(
    `[fixture] sanity OK: avg ${(totalRel / raw.queries.length).toFixed(1)} relevant ids/query`,
  );
}

main().catch((err) => {
  console.error('[fixture] failed:', err);
  process.exit(1);
});
