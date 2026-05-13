#!/usr/bin/env node
// Retrieval-only adapter for the MTEB LoCoMo text-retrieval dataset.

import { spawn } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  renameSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { asyncBufferFromFile, parquetReadObjects } from 'hyparquet';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '..');
const DATASET_REPO = 'mteb/LoCoMo';
const DATASET_REV = '02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac';
const DEFAULT_DATA_DIR = resolve(REPO_ROOT, 'target-copilot-bench', 'locomo-mteb');
const DEFAULT_OUT_DIR = resolve(REPO_ROOT, 'target-copilot-bench', 'bench-results');
const DEFAULT_TARGET_DIR = resolve(REPO_ROOT, 'target-copilot-bench');
const DEFAULT_TASKS = [
  'single_hop',
  'multi_hop',
  'temporal_reasoning',
  'open_domain',
  'adversarial',
];
const DEFAULT_SYSTEMS = ['search', 'rrf'];
const ALL_SYSTEMS = new Set(['search', 'rrf', 'emb', 'rrf_emb', 'search_emb', 'best', 'rrf_rerank', 'rrf_hyde', 'rrf_hyde_rerank', 'rrf_ctx', 'rrf_ctx_rerank', 'rrf_kg', 'rrf_kg_rerank']);
const EMB_SYSTEMS = new Set(['emb', 'rrf_emb', 'search_emb', 'best', 'rrf_rerank', 'rrf_hyde', 'rrf_hyde_rerank', 'rrf_ctx', 'rrf_ctx_rerank', 'rrf_kg', 'rrf_kg_rerank']);
const RERANK_SYSTEMS = new Set(['rrf_rerank', 'rrf_hyde_rerank', 'rrf_ctx_rerank', 'rrf_kg_rerank']);
const HYDE_SYSTEMS = new Set(['rrf_hyde', 'rrf_hyde_rerank']);
const CTX_SYSTEMS = new Set(['rrf_ctx', 'rrf_ctx_rerank']);
const KG_SYSTEMS = new Set(['rrf_kg', 'rrf_kg_rerank']);
const DATA_KINDS = ['corpus', 'queries', 'qrels'];
const METRIC_KS = [1, 5, 10, 20, 100];

function command() {
  const raw = process.argv[2];
  if (!raw || raw.startsWith('--')) return 'help';
  return raw;
}

function option(name, defaultValue) {
  const prefix = `--${name}=`;
  const raw = process.argv.slice(3).find(arg => arg.startsWith(prefix));
  return raw ? raw.slice(prefix.length) : defaultValue;
}

function hasFlag(name) {
  return process.argv.slice(3).includes(`--${name}`);
}

function numberOption(name, defaultValue) {
  const raw = option(name, String(defaultValue));
  const parsed = Number(raw);
  if (!Number.isInteger(parsed) || parsed < 0) {
    throw new Error(`--${name} must be a non-negative integer, got ${raw}`);
  }
  return parsed;
}

function positiveNumberOption(name, defaultValue) {
  const raw = option(name, String(defaultValue));
  const parsed = Number(raw);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw new Error(`--${name} must be a positive integer, got ${raw}`);
  }
  return parsed;
}

function listOption(name, fallback) {
  return option(name, fallback.join(','))
    .split(',')
    .map(value => value.trim())
    .filter(Boolean);
}

function printHelp() {
  console.log(`TerranSoul MTEB LoCoMo retrieval adapter

Usage:
  npm run brain:locomo:prepare
  npm run brain:locomo:sample
  npm run brain:locomo:run -- --tasks=single_hop,multi_hop --systems=search,rrf

Commands:
  prepare   Download pinned MTEB LoCoMo parquet files into target-copilot-bench/locomo-mteb
  sample    Run a small single_hop smoke pass (default: 10 queries)
  run       Run retrieval-only scoring through TerranSoul MemoryStore
  help      Print this help

Options:
  --tasks=<csv>       Tasks to run: ${DEFAULT_TASKS.join(',')} (default: all; sample defaults to single_hop)
  --systems=<csv>     Systems: search,rrf,emb,rrf_emb (default: search,rrf)
  --limit=<n>         Queries per task; 0 means all (default: sample=10, run=0)
  --top-k=<n>         Retrieval depth requested from MemoryStore (default: 100)
  --data-dir=<path>   Download/cache directory (default: target-copilot-bench/locomo-mteb)
  --out-dir=<path>    Report directory (default: target-copilot-bench/bench-results)
  --no-download       Fail if a parquet file is missing instead of downloading it
  --force-download    Re-download parquet files even if they already exist

This is a retrieval benchmark over MTEB qrels. It is not LoCoMo end-to-end QA accuracy.
`);
}

function assertKnownTasks(tasks) {
  const allowed = new Set(DEFAULT_TASKS);
  for (const task of tasks) {
    if (!allowed.has(task)) {
      throw new Error(`unknown task ${task}; use one of ${DEFAULT_TASKS.join(', ')}`);
    }
  }
}

function assertKnownSystems(systems) {
  for (const system of systems) {
    if (!ALL_SYSTEMS.has(system)) {
      throw new Error(`unknown system ${system}; use search, rrf, emb, rrf_emb, search_emb, best, rrf_rerank, rrf_hyde, rrf_hyde_rerank, rrf_ctx, or rrf_ctx_rerank`);
    }
  }
}

function needsEmbedding(systems) {
  return systems.some(s => EMB_SYSTEMS.has(s)) || hasFlag('embed');
}

function needsRerank(systems) {
  return systems.some(s => RERANK_SYSTEMS.has(s));
}

function needsHyde(systems) {
  return systems.some(s => HYDE_SYSTEMS.has(s));
}

function needsContextualize(systems) {
  return systems.some(s => CTX_SYSTEMS.has(s));
}

function needsKg(systems) {
  return systems.some(s => KG_SYSTEMS.has(s));
}

function parquetUrl(task, kind) {
  const config = `${task}-${kind}`;
  return `https://huggingface.co/datasets/${DATASET_REPO}/resolve/${DATASET_REV}/${config}/test-00000-of-00001.parquet`;
}

function parquetPath(dataDir, task, kind) {
  return resolve(dataDir, `${task}-${kind}`, 'test-00000-of-00001.parquet');
}

async function downloadFile(url, path, force) {
  if (existsSync(path) && !force) {
    return { path, downloaded: false, bytes: statSync(path).size };
  }

  mkdirSync(dirname(path), { recursive: true });
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`failed to download ${url} (HTTP ${response.status})`);
  }
  const data = Buffer.from(await response.arrayBuffer());
  const tempPath = `${path}.tmp`;
  writeFileSync(tempPath, data);
  renameSync(tempPath, path);
  return { path, downloaded: true, bytes: data.byteLength };
}

async function ensureParquetFiles(tasks, options) {
  const manifest = [];
  for (const task of tasks) {
    for (const kind of DATA_KINDS) {
      const path = parquetPath(options.dataDir, task, kind);
      if (!existsSync(path) && options.noDownload) {
        throw new Error(`missing ${path}; run npm run brain:locomo:prepare`);
      }
      const result = await downloadFile(parquetUrl(task, kind), path, options.forceDownload);
      manifest.push({ task, kind, ...result });
      const verb = result.downloaded ? 'downloaded' : 'cached';
      console.log(`[locomo] ${verb} ${task}-${kind} (${result.bytes.toLocaleString('en-US')} bytes)`);
    }
  }
  mkdirSync(options.dataDir, { recursive: true });
  writeFileSync(
    resolve(options.dataDir, 'manifest.json'),
    JSON.stringify({ dataset: DATASET_REPO, revision: DATASET_REV, files: manifest }, null, 2),
    'utf8',
  );
}

async function readParquetRows(path) {
  const file = await asyncBufferFromFile(path);
  return parquetReadObjects({ file });
}

function normalizeCorpusRows(rows) {
  return rows.map(row => ({
    id: String(row.id),
    title: String(row.title ?? ''),
    text: String(row.text ?? ''),
  }));
}

function normalizeQueryRows(rows) {
  return rows.map(row => ({
    id: String(row.id),
    text: String(row.text ?? ''),
  }));
}

function qrelKey(row, name) {
  return row[name] ?? row[name.replace('-', '_')];
}

function groupQrels(rows) {
  const grouped = new Map();
  for (const row of rows) {
    const queryId = String(qrelKey(row, 'query-id'));
    const corpusId = String(qrelKey(row, 'corpus-id'));
    const score = Number(row.score ?? 1);
    if (!grouped.has(queryId)) grouped.set(queryId, new Map());
    grouped.get(queryId).set(corpusId, Number.isFinite(score) ? score : 1);
  }
  return grouped;
}

async function loadTaskData(task, options) {
  const [corpusRows, queryRows, qrelRows] = await Promise.all([
    readParquetRows(parquetPath(options.dataDir, task, 'corpus')),
    readParquetRows(parquetPath(options.dataDir, task, 'queries')),
    readParquetRows(parquetPath(options.dataDir, task, 'qrels')),
  ]);
  const corpus = normalizeCorpusRows(corpusRows);
  const queries = normalizeQueryRows(queryRows);
  const qrels = groupQrels(qrelRows);
  const filteredQueries = queries.filter(query => qrels.has(query.id));
  const limitedQueries = options.limit > 0 ? filteredQueries.slice(0, options.limit) : filteredQueries;
  return { corpus, queries: limitedQueries, allQueryCount: filteredQueries.length, qrels };
}

function sessionText(row) {
  return [row.title ? `Title: ${row.title}` : '', row.text].filter(Boolean).join('\n');
}

function sessionPayloads(corpus) {
  return corpus.map(row => ({
    session_id: row.id,
    text: sessionText(row),
    date: null,
    turn_count: 1,
  }));
}

function gain(score) {
  return (2 ** score) - 1;
}

function dcg(scores, k) {
  let total = 0;
  for (let index = 0; index < Math.min(k, scores.length); index += 1) {
    const score = scores[index];
    if (score > 0) total += gain(score) / Math.log2(index + 2);
  }
  return total;
}

function metricForQuery(retrievedIds, qrels, k) {
  const top = retrievedIds.slice(0, k);
  const relevantCount = qrels.size;
  const hits = top.filter(id => qrels.has(id)).length;
  const scores = top.map(id => qrels.get(id) ?? 0);
  const ideal = [...qrels.values()].sort((a, b) => b - a).slice(0, k);
  const idealDcg = dcg(ideal, k);
  let precisionSum = 0;
  let seenRelevant = 0;
  for (let index = 0; index < top.length; index += 1) {
    if (qrels.has(top[index])) {
      seenRelevant += 1;
      precisionSum += seenRelevant / (index + 1);
    }
  }
  const firstRelevantIndex = top.findIndex(id => qrels.has(id));
  return {
    [`recall_at_${k}`]: relevantCount === 0 ? 0 : hits / relevantCount,
    [`hit_at_${k}`]: hits > 0 ? 1 : 0,
    [`ndcg_at_${k}`]: idealDcg === 0 ? 0 : dcg(scores, k) / idealDcg,
    [`map_at_${k}`]: relevantCount === 0 ? 0 : precisionSum / Math.min(relevantCount, k),
    [`mrr_at_${k}`]: firstRelevantIndex < 0 ? 0 : 1 / (firstRelevantIndex + 1),
  };
}

function scoreQuery(retrievedIds, qrels) {
  const metrics = {
    relevant_count: qrels.size,
    retrieved_count: retrievedIds.length,
  };
  for (const k of METRIC_KS) {
    Object.assign(metrics, metricForQuery(retrievedIds, qrels, k));
  }
  return metrics;
}

function avg(values) {
  return values.length ? values.reduce((sum, value) => sum + value, 0) / values.length : 0;
}

function aggregate(system, task, rows) {
  const metricNames = [
    ...METRIC_KS.flatMap(k => [`recall_at_${k}`, `hit_at_${k}`]),
    'ndcg_at_10',
    'map_at_10',
    'mrr_at_100',
    'latency_ms',
    'retrieved_tokens',
  ];
  const aggregateRow = { system, task, queries: rows.length, per_query: rows };
  for (const metric of metricNames) {
    aggregateRow[metric] = avg(rows.map(row => Number(row[metric] ?? 0)));
  }
  return aggregateRow;
}

function aggregateOverall(system, taskRows) {
  const perQuery = taskRows.flatMap(row => row.per_query);
  return aggregate(system, 'overall', perQuery);
}

function pct(value) {
  return `${(value * 100).toFixed(1)}%`;
}

function ms(value) {
  return `${value.toFixed(2)}ms`;
}

function markdownTable(rows) {
  const lines = [
    '| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |',
    '|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|',
  ];
  for (const row of rows) {
    lines.push(
      `| ${row.task} | ${row.system} | ${row.queries} | ${pct(row.recall_at_1)} | ${pct(row.recall_at_5)} | ${pct(row.recall_at_10)} | ${pct(row.recall_at_20)} | ${pct(row.recall_at_100)} | ${pct(row.ndcg_at_10)} | ${pct(row.map_at_10)} | ${pct(row.mrr_at_100)} | ${ms(row.latency_ms)} | ${Math.round(row.retrieved_tokens).toLocaleString('en-US')} |`,
    );
  }
  return lines.join('\n');
}

function markdownReport(report) {
  const lines = [];
  lines.push('# TerranSoul MTEB LoCoMo Retrieval Report');
  lines.push('');
  lines.push(`Date: ${report.generated_at}`);
  lines.push(`Dataset: ${report.dataset} @ ${report.revision}`);
  lines.push(`Systems: ${report.systems.join(', ')}`);
  lines.push(`Tasks: ${report.tasks.join(', ')}`);
  lines.push(`Top K requested: ${report.top_k}`);
  lines.push('');
  lines.push('This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.');
  lines.push('');
  lines.push('## Overall');
  lines.push('');
  lines.push(markdownTable(report.overall));
  lines.push('');
  lines.push('## By Task');
  lines.push('');
  lines.push(markdownTable(report.by_task));
  lines.push('');
  lines.push('## Methodology Notes');
  lines.push('');
  lines.push('- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.');
  lines.push('- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.');
  lines.push('- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.');
  lines.push('- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.');
  return `${lines.join('\n')}\n`;
}

function writeReports(report, options) {
  mkdirSync(options.outDir, { recursive: true });
  const suffix = report.limit > 0 ? `_${report.total_queries}q` : '';
  const jsonPath = resolve(options.outDir, `locomo_mteb_terransoul${suffix}.json`);
  const mdPath = resolve(options.outDir, `locomo_mteb_terransoul${suffix}.md`);
  writeFileSync(jsonPath, JSON.stringify(report, null, 2), 'utf8');
  writeFileSync(mdPath, markdownReport(report), 'utf8');
  console.log(`[locomo] wrote ${jsonPath}`);
  console.log(`[locomo] wrote ${mdPath}`);
}

class JsonlClient {
  constructor({ embed = false, rerank = false, hyde = false, contextualize = false, kg = false } = {}) {
    this.nextId = 1;
    this.pending = new Map();
    this.buffer = '';
    this.embedEnv = {
      ...(embed ? { LONGMEM_EMBED: '1' } : {}),
      ...(rerank ? { LONGMEM_RERANK: '1' } : {}),
      ...(hyde ? { LONGMEM_HYDE: '1' } : {}),
      ...(contextualize ? { LONGMEM_CONTEXTUALIZE: '1' } : {}),
      ...(kg ? { LONGMEM_KG_EDGES: '1' } : {}),
    };
    this.proc = spawn('cargo', [
      'run',
      '--quiet',
      '--manifest-path',
      resolve(REPO_ROOT, 'src-tauri', 'Cargo.toml'),
      '--features', 'bench-million', '--bin',
      'longmemeval-ipc',
      '--target-dir',
      DEFAULT_TARGET_DIR,
    ], {
      cwd: REPO_ROOT,
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, ...(this.embedEnv || {}) },
    });

    this.proc.stdout.setEncoding('utf8');
    this.proc.stdout.on('data', chunk => this.onStdout(chunk));
    this.proc.stderr.setEncoding('utf8');
    this.proc.stderr.on('data', chunk => process.stderr.write(chunk));
    this.proc.on('exit', code => {
      for (const { reject } of this.pending.values()) {
        reject(new Error(`IPC process exited before response (code ${code})`));
      }
      this.pending.clear();
    });
  }

  onStdout(chunk) {
    this.buffer += chunk;
    let newline = this.buffer.indexOf('\n');
    while (newline >= 0) {
      const line = this.buffer.slice(0, newline).trim();
      this.buffer = this.buffer.slice(newline + 1);
      if (line) this.handleLine(line);
      newline = this.buffer.indexOf('\n');
    }
  }

  handleLine(line) {
    let message;
    try {
      message = JSON.parse(line);
    } catch (err) {
      throw new Error(`invalid IPC JSON: ${line}\n${err.message}`);
    }
    const pending = this.pending.get(message.id);
    if (!pending) return;
    this.pending.delete(message.id);
    if (message.ok) pending.resolve(message.data);
    else pending.reject(new Error(message.error ?? `IPC request ${message.id} failed`));
  }

  send(payload) {
    const id = this.nextId;
    this.nextId += 1;
    const line = JSON.stringify({ id, ...payload });
    return new Promise((resolvePromise, rejectPromise) => {
      this.pending.set(id, { resolve: resolvePromise, reject: rejectPromise });
      this.proc.stdin.write(`${line}\n`, err => {
        if (err) {
          this.pending.delete(id);
          rejectPromise(err);
        }
      });
    });
  }

  async close() {
    if (this.proc.killed) return;
    try {
      await this.send({ op: 'shutdown' });
    } catch {
      this.proc.kill();
    }
  }
}

async function searchQuery(client, system, query, topK) {
  const start = performance.now();
  const response = await client.send({
    op: 'search',
    query,
    mode: system,
    limit: topK,
  });
  const latencyMs = performance.now() - start;
  const results = response.results ?? [];
  return {
    latencyMs,
    retrievedIds: results.map(result => result.session_id).filter(Boolean),
    retrievedTokens: results.reduce((sum, result) => sum + (result.token_count ?? 0), 0),
  };
}

async function runTask(client, task, options) {
  const data = await loadTaskData(task, options);
  console.log(
    `[locomo] ${task}: corpus=${data.corpus.length.toLocaleString('en-US')} queries=${data.queries.length}/${data.allQueryCount} qrels=${data.qrels.size}`,
  );

  await client.send({ op: 'reset' });
  await client.send({
    op: 'add_sessions',
    question_id: `locomo-${task}`,
    sessions: sessionPayloads(data.corpus),
  });

  const taskSystemRows = [];
  for (const system of options.systems) {
    const perQuery = [];
    for (let index = 0; index < data.queries.length; index += 1) {
      const query = data.queries[index];
      const qrels = data.qrels.get(query.id);
      const result = await searchQuery(client, system, query.text, options.topK);
      perQuery.push({
        task,
        system,
        query_id: query.id,
        query: query.text,
        latency_ms: result.latencyMs,
        retrieved_tokens: result.retrievedTokens,
        retrieved_ids: result.retrievedIds,
        gold_ids: [...qrels.keys()],
        ...scoreQuery(result.retrievedIds, qrels),
      });
      const processed = index + 1;
      if (processed % 50 === 0 || processed === data.queries.length) {
        const soFar = aggregate(system, task, perQuery);
        console.log(
          `[locomo] ${task}/${system} ${processed}/${data.queries.length}: R@10=${pct(soFar.recall_at_10)} NDCG@10=${pct(soFar.ndcg_at_10)} MRR@100=${pct(soFar.mrr_at_100)}`,
        );
      }
    }
    taskSystemRows.push(aggregate(system, task, perQuery));
  }
  return taskSystemRows;
}

async function run(options) {
  await ensureParquetFiles(options.tasks, options);
  const embed = needsEmbedding(options.systems);
  const rerank = needsRerank(options.systems);
  const hyde = needsHyde(options.systems);
  const contextualize = needsContextualize(options.systems);
  const kg = needsKg(options.systems);
  const client = new JsonlClient({ embed, rerank, hyde, contextualize, kg });
  try {
    const byTask = [];
    for (const task of options.tasks) {
      byTask.push(...await runTask(client, task, options));
    }
    const overall = options.systems.map(system => aggregateOverall(
      system,
      byTask.filter(row => row.system === system),
    ));
    return {
      benchmark: 'MTEB LoCoMo retrieval-only',
      generated_at: new Date().toISOString(),
      dataset: DATASET_REPO,
      revision: DATASET_REV,
      tasks: options.tasks,
      systems: options.systems,
      top_k: options.topK,
      limit: options.limit,
      total_queries: overall.reduce((sum, row) => Math.max(sum, row.queries), 0),
      overall,
      by_task: byTask,
    };
  } finally {
    await client.close();
  }
}

async function main() {
  const cmd = command();
  if (cmd === 'help' || hasFlag('help')) {
    printHelp();
    return;
  }

  const sampleMode = cmd === 'sample';
  const tasks = sampleMode
    ? listOption('tasks', ['single_hop'])
    : listOption('tasks', DEFAULT_TASKS);
  const systems = listOption('systems', DEFAULT_SYSTEMS);
  assertKnownTasks(tasks);
  assertKnownSystems(systems);

  const options = {
    tasks,
    systems,
    limit: numberOption('limit', sampleMode ? 10 : 0),
    topK: positiveNumberOption('top-k', 100),
    dataDir: resolve(REPO_ROOT, option('data-dir', DEFAULT_DATA_DIR)),
    outDir: resolve(REPO_ROOT, option('out-dir', DEFAULT_OUT_DIR)),
    noDownload: hasFlag('no-download'),
    forceDownload: hasFlag('force-download'),
  };

  if (cmd === 'prepare') {
    await ensureParquetFiles(options.tasks, options);
    return;
  }
  if (cmd !== 'run' && cmd !== 'sample') {
    throw new Error(`unknown command: ${cmd}`);
  }

  console.log(
    `[locomo] tasks=${options.tasks.join(',')} systems=${options.systems.join(',')} limit=${options.limit || 'all'} top_k=${options.topK}`,
  );
  const report = await run(options);
  writeReports(report, options);
}

main().catch(err => {
  console.error(`[locomo] failed: ${err.message}`);
  process.exit(1);
});
