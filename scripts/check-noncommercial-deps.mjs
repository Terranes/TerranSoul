#!/usr/bin/env node
// scripts/check-noncommercial-deps.mjs
// CI guardrail: verifies no noncommercial dependencies have been introduced.
// Exit code 1 if violations found.

import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import { execSync } from 'child_process';

const ROOT = new URL('..', import.meta.url).pathname.replace(/^\/([A-Z]:)/, '$1');

const BANNED_SUBSTRINGS = ['gitnexus'];
const CHECKED_FILES = [
  'src-tauri/Cargo.toml',
  'package.json',
  'Dockerfile.mcp',
  'docker-compose.mcp.yml',
];

let violations = 0;

// Check 1: No banned substrings in dependency manifests
for (const rel of CHECKED_FILES) {
  const abs = join(ROOT, rel);
  if (!existsSync(abs)) continue;
  const content = readFileSync(abs, 'utf8').toLowerCase();
  for (const banned of BANNED_SUBSTRINGS) {
    if (content.includes(banned)) {
      console.error(`VIOLATION: "${banned}" found in ${rel}`);
      violations++;
    }
  }
}

// Check 2: No gitnexus/sidecar in Tauri command or MCP tool names
try {
  const grepResult = execSync(
    'git grep -l "gitnexus" -- "src-tauri/src/commands/" "src-tauri/src/ai_integrations/"',
    { cwd: ROOT, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] },
  ).trim();
  if (grepResult) {
    for (const file of grepResult.split('\n').filter(Boolean)) {
      console.error(`VIOLATION: "gitnexus" reference in command/MCP layer: ${file}`);
      violations++;
    }
  }
} catch { /* git grep found nothing or not available — OK */ }

// Check 3: No gitnexus-specific sidecar patterns in Tauri command names
try {
  const sidecarGrep = execSync(
    'git grep -n "gitnexus_sidecar\\|gitnexus-sidecar" -- "src-tauri/src/commands/"',
    { cwd: ROOT, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] },
  ).trim();
  if (sidecarGrep) {
    for (const line of sidecarGrep.split('\n').filter(Boolean)) {
      if (line.includes('//') || line.includes('removed') || line.includes('Removed')) continue;
      console.error(`VIOLATION: gitnexus sidecar reference in command layer: ${line}`);
      violations++;
    }
  }
} catch { /* git grep found nothing or not available — OK */ }

if (violations > 0) {
  console.error(`\n${violations} noncommercial dependency violation(s) found.`);
  process.exit(1);
} else {
  console.log('✓ No noncommercial dependency violations detected.');
}
