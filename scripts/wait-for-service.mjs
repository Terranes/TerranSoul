#!/usr/bin/env node
/**
 * wait-for-service.mjs — Poll a local HTTP endpoint until it responds.
 *
 * Usage:
 *   node scripts/wait-for-service.mjs <url> [timeout_seconds]
 *
 * Examples:
 *   node scripts/wait-for-service.mjs http://localhost:1420 30
 *   node scripts/wait-for-service.mjs http://localhost:11434/api/version 15
 *   node scripts/wait-for-service.mjs http://localhost:7421/mcp 10
 *
 * Exits 0 on success, 1 on timeout.
 * Designed to be wired into .vscode/tasks.json as a dependsOn pre-task
 * so Copilot commands that need a running backend don't race cold starts.
 */

const url = process.argv[2];
const timeoutSec = parseInt(process.argv[3] || '30', 10);

if (!url) {
  console.error('Usage: wait-for-service.mjs <url> [timeout_seconds]');
  process.exit(1);
}

const pollIntervalMs = 1000;
const deadline = Date.now() + timeoutSec * 1000;

async function tryFetch() {
  try {
    const controller = new AbortController();
    const id = setTimeout(() => controller.abort(), 3000);
    const res = await fetch(url, {
      method: 'GET',
      signal: controller.signal,
    });
    clearTimeout(id);
    return res.status < 500; // 2xx, 3xx, 4xx all mean "service is up"
  } catch {
    return false;
  }
}

async function main() {
  console.log(`⏳ Waiting for ${url} (timeout: ${timeoutSec}s)...`);
  let attempt = 0;

  while (Date.now() < deadline) {
    attempt++;
    if (await tryFetch()) {
      console.log(`✅ ${url} is up (attempt ${attempt})`);
      process.exit(0);
    }
    await new Promise((r) => setTimeout(r, pollIntervalMs));
  }

  console.error(`❌ Timeout after ${timeoutSec}s — ${url} not reachable`);
  process.exit(1);
}

main();
