#!/usr/bin/env node
/**
 * TerranSoul — Cross-platform prerequisite checker & auto-installer.
 *
 * Usage:
 *   node scripts/setup-prerequisites.mjs          # Check & prompt to install
 *   node scripts/setup-prerequisites.mjs --auto   # Auto-install everything
 *
 * Checks: Node.js ≥20, Rust (stable), Tauri CLI, WebView2 (Win), npm deps.
 */

import { execSync, spawnSync } from 'child_process';
import { platform } from 'os';
import { existsSync } from 'fs';
import { resolve } from 'path';

const IS_WIN = platform() === 'win32';
const IS_MAC = platform() === 'darwin';
const IS_LINUX = platform() === 'linux';
const AUTO = process.argv.includes('--auto');

const results = [];

function run(cmd, opts = {}) {
  try {
    return execSync(cmd, { encoding: 'utf8', stdio: 'pipe', ...opts }).trim();
  } catch {
    return null;
  }
}

function check(name, test, install) {
  const result = test();
  results.push({ name, ...result, install });
  return result;
}

function printStatus(name, ok, version) {
  const icon = ok ? '✅' : '❌';
  const ver = version ? ` (${version})` : '';
  console.log(`  ${icon} ${name}${ver}`);
}

// --- Checks ---

console.log('\n🔍 Checking TerranSoul prerequisites...\n');

// 1. Node.js ≥ 20
check('Node.js ≥ 20', () => {
  const ver = run('node -v');
  if (!ver) return { ok: false, version: null };
  const major = parseInt(ver.replace('v', '').split('.')[0], 10);
  return { ok: major >= 20, version: ver };
}, () => {
  if (IS_WIN) run('winget install OpenJS.NodeJS.LTS --accept-source-agreements --accept-package-agreements');
  else if (IS_MAC) run('brew install node@20');
  else run('curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash - && sudo apt-get install -y nodejs');
});

// 2. Rust (stable)
check('Rust (stable)', () => {
  const ver = run('rustc --version');
  if (!ver) return { ok: false, version: null };
  return { ok: true, version: ver.split(' ')[1] };
}, () => {
  if (IS_WIN) run('winget install Rustlang.Rustup --accept-source-agreements --accept-package-agreements');
  else run('curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y');
});

// 3. Tauri CLI
check('Tauri CLI', () => {
  const ver = run('npx tauri --version 2>/dev/null') || run('cargo tauri --version 2>/dev/null');
  if (!ver) return { ok: false, version: null };
  return { ok: true, version: ver.replace('tauri-cli ', '') };
}, () => {
  run('cargo install tauri-cli');
});

// 4. WebView2 (Windows only)
if (IS_WIN) {
  check('WebView2 Runtime', () => {
    const regCheck = run(
      'reg query "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv 2>nul'
    );
    if (regCheck && !regCheck.includes('0.0.0.0')) {
      const match = regCheck.match(/pv\s+REG_SZ\s+(.+)/);
      return { ok: true, version: match ? match[1].trim() : 'installed' };
    }
    return { ok: false, version: null };
  }, () => {
    run('winget install Microsoft.EdgeWebView2Runtime --accept-source-agreements --accept-package-agreements');
  });
}

// 5. npm dependencies
check('npm dependencies', () => {
  const nmPath = resolve(process.cwd(), 'node_modules');
  return { ok: existsSync(nmPath), version: existsSync(nmPath) ? 'installed' : null };
}, () => {
  run('npm install', { cwd: process.cwd() });
});

// --- Report ---

console.log('');
for (const r of results) {
  printStatus(r.name, r.ok, r.version);
}

const missing = results.filter(r => !r.ok);

if (missing.length === 0) {
  console.log('\n✨ All prerequisites satisfied! Run:\n');
  console.log('   npm run dev         # Frontend only (Vite :1420)');
  console.log('   cargo tauri dev     # Full app with hot-reload');
  console.log('   npm run mcp         # Headless MCP brain server\n');
  process.exit(0);
}

console.log(`\n⚠️  ${missing.length} prerequisite(s) missing:\n`);
for (const m of missing) {
  console.log(`   • ${m.name}`);
}

if (AUTO) {
  console.log('\n🔧 Auto-installing...\n');
  for (const m of missing) {
    console.log(`   Installing ${m.name}...`);
    try {
      m.install();
      console.log(`   ✅ ${m.name} installed`);
    } catch (e) {
      console.log(`   ❌ ${m.name} failed: ${e.message}`);
    }
  }
  console.log('\n🔄 Re-run this script to verify: node scripts/setup-prerequisites.mjs\n');
} else {
  console.log('\n💡 Run with --auto to install automatically:');
  console.log('   node scripts/setup-prerequisites.mjs --auto\n');
  console.log('   Or use your AI agent:');
  console.log('   • VS Code Copilot: /setup-prerequisites');
  console.log('   • Cursor/Claude/Codex: "Run setup-prerequisites"\n');
}

process.exit(missing.length > 0 ? 1 : 0);
