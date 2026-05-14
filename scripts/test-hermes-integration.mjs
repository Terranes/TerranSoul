#!/usr/bin/env node
/**
 * Live integration test: Hermes ↔ TerranSoul MCP + shared Ollama LLM.
 *
 * Verifies that the Hermes config is correctly wired to:
 *   1. TerranSoul's MCP brain server (HTTP, bearer token auth)
 *   2. The shared local Ollama LLM instance
 *
 * This test makes REAL HTTP calls — it requires:
 *   - TerranSoul MCP running on :7421, :7422, or :7423
 *   - Ollama running on :11434
 *   - Hermes cli-config.yaml at %LOCALAPPDATA%\hermes\ (Windows)
 *     or ~/.hermes/ (macOS/Linux)
 *
 * Usage:
 *   node scripts/test-hermes-integration.mjs
 *   node scripts/test-hermes-integration.mjs --verbose
 */
import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';

const VERBOSE = process.argv.includes('--verbose');
let passed = 0;
let failed = 0;
const results = [];

function log(msg) { console.log(msg); }
function verbose(msg) { if (VERBOSE) console.log(`  [verbose] ${msg}`); }

function assert(label, condition, detail) {
  if (condition) {
    passed++;
    results.push({ label, status: 'PASS' });
    log(`  ✅ ${label}`);
  } else {
    failed++;
    results.push({ label, status: 'FAIL', detail });
    log(`  ❌ ${label}: ${detail || 'assertion failed'}`);
  }
}

// ─── Config discovery ───────────────────────────────────────────
function findHermesConfig() {
  const candidates = [];
  if (process.platform === 'win32') {
    const localAppData = process.env.LOCALAPPDATA;
    if (localAppData) {
      candidates.push(join(localAppData, 'hermes', 'cli-config.yaml'));
    }
  }
  candidates.push(join(homedir(), '.hermes', 'cli-config.yaml'));
  for (const p of candidates) {
    if (existsSync(p)) return p;
  }
  return null;
}

// ─── MCP JSON-RPC helper ────────────────────────────────────────
async function mcpCall(url, token, method, params = {}) {
  const body = JSON.stringify({
    jsonrpc: '2.0',
    id: Date.now(),
    method,
    params,
  });
  const resp = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${token}`,
    },
    body,
  });
  if (!resp.ok) throw new Error(`HTTP ${resp.status}: ${await resp.text()}`);
  return resp.json();
}

// ─── Test groups ────────────────────────────────────────────────

async function testConfigExists() {
  log('\n[1/5] Hermes config file');
  const configPath = findHermesConfig();
  assert('cli-config.yaml exists', !!configPath,
    'No cli-config.yaml found. Run setup_hermes_mcp from TerranSoul first.');
  if (!configPath) return null;

  const content = readFileSync(configPath, 'utf-8');
  verbose(`Config at: ${configPath} (${content.length} chars)`);
  return { path: configPath, content };
}

function testConfigContent(config) {
  log('\n[2/5] Config content validation');
  if (!config) { assert('config loaded', false, 'skipped — no config'); return null; }

  const { content } = config;

  // MCP markers
  const hasBegin = content.includes('>>> TerranSoul MCP auto-config');
  const hasEnd = content.includes('<<< TerranSoul MCP auto-config');
  assert('TerranSoul marker block present', hasBegin && hasEnd,
    `begin=${hasBegin}, end=${hasEnd}`);

  // MCP server entry
  const hasMcpUrl = /url:\s*"http:\/\/127\.0\.0\.1:\d{4}\/mcp"/.test(content);
  assert('MCP server URL configured', hasMcpUrl);

  // Bearer token
  const hasBearer = /Authorization:\s*"Bearer\s+\S+"/.test(content);
  assert('Bearer token configured', hasBearer);

  // Extract URL and token for later tests
  const urlMatch = content.match(/url:\s*"(http:\/\/127\.0\.0\.1:\d{4}\/mcp)"/);
  const tokenMatch = content.match(/Authorization:\s*"Bearer\s+(\S+)"/);
  const mcpUrl = urlMatch?.[1] || null;
  const mcpToken = tokenMatch?.[1] || null;

  // Ollama LLM config
  const hasOllamaProvider = /provider:\s*"(?:ollama|custom)"/.test(content);
  assert('Ollama LLM provider configured', hasOllamaProvider);

  const hasOllamaUrl = /base_url:\s*"http:\/\/127\.0\.0\.1:11434/.test(content);
  assert('Ollama base_url points to localhost:11434', hasOllamaUrl);

  return { mcpUrl, mcpToken };
}

async function testOllamaReachable() {
  log('\n[3/5] Ollama LLM reachability');
  try {
    const resp = await fetch('http://127.0.0.1:11434/api/version');
    const data = await resp.json();
    assert('Ollama HTTP reachable', resp.ok, `status=${resp.status}`);
    assert('Ollama version returned', !!data.version, JSON.stringify(data));
    verbose(`Ollama version: ${data.version}`);

    // Check that the shared model (gemma3:4b) is available
    const modelsResp = await fetch('http://127.0.0.1:11434/api/tags');
    const models = await modelsResp.json();
    const modelNames = (models.models || []).map(m => m.name);
    const hasGemma = modelNames.some(n => n.includes('gemma3'));
    assert('Shared model (gemma3) available in Ollama', hasGemma,
      `Available: ${modelNames.join(', ')}`);
    verbose(`Models: ${modelNames.join(', ')}`);
  } catch (err) {
    assert('Ollama HTTP reachable', false, err.message);
  }
}

async function testMcpHealth(mcpUrl, mcpToken) {
  log('\n[4/5] TerranSoul MCP brain (as Hermes would call it)');
  if (!mcpUrl || !mcpToken) {
    assert('MCP URL+token extracted', false, 'skipped — no URL/token from config');
    return;
  }

  try {
    // 1. Call brain_health via MCP JSON-RPC
    const healthResult = await mcpCall(mcpUrl, mcpToken, 'tools/call', {
      name: 'brain_health',
      arguments: {},
    });
    verbose(`brain_health raw: ${JSON.stringify(healthResult).slice(0, 300)}`);

    const healthContent = healthResult?.result?.content?.[0]?.text;
    assert('brain_health responds', !!healthContent, JSON.stringify(healthResult).slice(0, 200));

    if (healthContent) {
      const health = JSON.parse(healthContent);
      assert('Brain provider is ollama', health.brain_provider === 'ollama',
        `provider=${health.brain_provider}`);
      assert('Memory count > 0', (health.memory_total || 0) > 0,
        `count=${health.memory_total}`);
      assert('RAG quality 100%', health.rag_quality_pct === 100,
        `rag=${health.rag_quality_pct}%`);
      verbose(`Memories: ${health.memory_total}, RAG: ${health.rag_quality_pct}%`);
    }
  } catch (err) {
    assert('brain_health responds', false, err.message);
  }
}

async function testMcpSearch(mcpUrl, mcpToken) {
  log('\n[5/5] TerranSoul brain_search (simulated Hermes query)');
  if (!mcpUrl || !mcpToken) {
    assert('MCP URL+token available', false, 'skipped');
    return;
  }

  try {
    // Simulate what Hermes would do: search TerranSoul's brain for a topic
    const searchResult = await mcpCall(mcpUrl, mcpToken, 'tools/call', {
      name: 'brain_search',
      arguments: {
        query: 'What is the default embedding model used by TerranSoul?',
        limit: 3,
        rerank: false,
      },
    });
    verbose(`brain_search raw: ${JSON.stringify(searchResult).slice(0, 500)}`);

    const searchContent = searchResult?.result?.content?.[0]?.text;
    assert('brain_search returns results', !!searchContent,
      JSON.stringify(searchResult).slice(0, 200));

    if (searchContent) {
      const memories = JSON.parse(searchContent);
      assert('Search returned ≥1 memory', Array.isArray(memories) && memories.length > 0,
        `count=${Array.isArray(memories) ? memories.length : 'not array'}`);
      if (Array.isArray(memories) && memories.length > 0) {
        assert('Results contain content with scores', 
          memories[0].content?.length > 0 && memories[0].score !== undefined,
          `content_len=${memories[0]?.content?.length}, score=${memories[0]?.score}`);
      }
    }
  } catch (err) {
    assert('brain_search returns results', false, err.message);
  }
}

// ─── Main ───────────────────────────────────────────────────────
async function main() {
  log('═══════════════════════════════════════════════════════════');
  log('  Hermes ↔ TerranSoul Integration Test (Live Production)');
  log('═══════════════════════════════════════════════════════════');

  const config = await testConfigExists();
  const extracted = testConfigContent(config);
  await testOllamaReachable();
  await testMcpHealth(extracted?.mcpUrl, extracted?.mcpToken);
  await testMcpSearch(extracted?.mcpUrl, extracted?.mcpToken);

  log('\n═══════════════════════════════════════════════════════════');
  log(`  Results: ${passed} passed, ${failed} failed (${passed + failed} total)`);
  log('═══════════════════════════════════════════════════════════');

  if (failed > 0) {
    log('\n⚠️  Some tests failed. Check output above for details.');
    process.exitCode = 1;
  } else {
    log('\n🎉 All tests passed! Hermes can use TerranSoul brain + shared Ollama.');
    process.exitCode = 0;
  }
}

main().catch(err => {
  console.error('Fatal:', err);
  process.exitCode = 2;
});
