#!/usr/bin/env node
/**
 * File-size quality check for TerranSoul.
 *
 * Walks `src-tauri/src/**\/*.rs` and `src/**\/*.vue` and fails CI when any
 * file exceeds its language threshold:
 *
 *   - Rust:  1000 lines per file
 *   - Vue:    800 lines per file
 *
 * Files in `scripts/file-size-allowlist.json` are temporarily exempt:
 * each entry pins the recorded size at allowlist time, and the script
 * fails if the file *grows beyond* that pinned size. This keeps existing
 * oversized files from blocking the world while preventing further bloat.
 *
 * Usage:
 *   node scripts/check-file-sizes.mjs           # check + report
 *   node scripts/check-file-sizes.mjs --update  # rewrite allowlist with
 *                                               # current sizes (use with
 *                                               # care — only when an
 *                                               # intentional refactor
 *                                               # has shrunk a file
 *                                               # below threshold)
 *
 * Exit codes:
 *   0 — all files within budget
 *   1 — at least one file violates threshold or grew beyond its allowlist
 *   2 — usage / IO error
 */

import { readFileSync, writeFileSync, statSync } from 'node:fs';
import { join, relative, resolve, sep } from 'node:path';
import { readdir } from 'node:fs/promises';

const REPO_ROOT = resolve(new URL('..', import.meta.url).pathname);
const ALLOWLIST_PATH = join(REPO_ROOT, 'scripts', 'file-size-allowlist.json');

const THRESHOLDS = {
  rust: { ext: '.rs', root: 'src-tauri/src', maxLines: 1000 },
  vue: { ext: '.vue', root: 'src', maxLines: 800 },
};

// Directories we never descend into.
const SKIP_DIRS = new Set([
  'node_modules',
  'target',
  'dist',
  'build',
  '.git',
  '.next',
  'gen',
]);

/**
 * Recursively walk `dir` and yield absolute paths of files matching `ext`.
 */
async function* walk(dir, ext) {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    if (SKIP_DIRS.has(entry.name)) continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      yield* walk(full, ext);
    } else if (entry.isFile() && entry.name.endsWith(ext)) {
      yield full;
    }
  }
}

function countLines(absPath) {
  // Use a streaming-friendly count for big files. statSync first to skip
  // empty / huge binary files quickly.
  const size = statSync(absPath).size;
  if (size === 0) return 0;
  const buf = readFileSync(absPath);
  let n = 0;
  for (let i = 0; i < buf.length; i++) {
    if (buf[i] === 0x0a) n++;
  }
  // Trailing-newline-less files still count their last line.
  if (buf[buf.length - 1] !== 0x0a) n++;
  return n;
}

function loadAllowlist() {
  try {
    return JSON.parse(readFileSync(ALLOWLIST_PATH, 'utf8'));
  } catch {
    return {};
  }
}

function saveAllowlist(allow) {
  const sorted = Object.fromEntries(
    Object.entries(allow).sort(([a], [b]) => a.localeCompare(b)),
  );
  writeFileSync(ALLOWLIST_PATH, JSON.stringify(sorted, null, 2) + '\n');
}

function normalisePath(absPath) {
  // Repo-relative POSIX path so the allowlist works on every platform.
  return relative(REPO_ROOT, absPath).split(sep).join('/');
}

async function main() {
  const args = new Set(process.argv.slice(2));
  const updateMode = args.has('--update');

  const allowlist = loadAllowlist();
  const measurements = []; // { rel, lines, language, threshold }
  const violations = []; // { rel, lines, threshold, allowed?, reason }
  const oversized = []; // for --update mode

  for (const [language, cfg] of Object.entries(THRESHOLDS)) {
    const root = join(REPO_ROOT, cfg.root);
    for await (const abs of walk(root, cfg.ext)) {
      const rel = normalisePath(abs);
      const lines = countLines(abs);
      measurements.push({ rel, lines, language, threshold: cfg.maxLines });

      if (lines > cfg.maxLines) {
        const pinned = allowlist[rel];
        if (pinned == null) {
          violations.push({
            rel,
            lines,
            threshold: cfg.maxLines,
            reason: `exceeds ${language} budget of ${cfg.maxLines} lines (no allowlist entry)`,
          });
        } else if (lines > pinned) {
          violations.push({
            rel,
            lines,
            threshold: cfg.maxLines,
            allowed: pinned,
            reason: `grew beyond pinned allowlist size (${lines} > ${pinned})`,
          });
        }
        oversized.push({ rel, lines });
      }
    }
  }

  if (updateMode) {
    const next = {};
    for (const o of oversized) next[o.rel] = o.lines;
    saveAllowlist(next);
    console.log(
      `Wrote allowlist with ${Object.keys(next).length} entries to scripts/file-size-allowlist.json`,
    );
    return;
  }

  // Pretty report.
  measurements.sort((a, b) => b.lines - a.lines);
  const top = measurements.slice(0, 5);
  console.log('Top 5 largest files:');
  for (const m of top) {
    const tag = m.lines > m.threshold ? '⚠️ ' : '   ';
    console.log(`  ${tag}${m.rel}  ${m.lines} lines  (budget ${m.threshold})`);
  }

  if (violations.length === 0) {
    console.log('\n✅ file-size check passed');
    process.exit(0);
  }

  console.error(`\n❌ file-size check failed — ${violations.length} violation(s):`);
  for (const v of violations) {
    console.error(`  • ${v.rel}: ${v.reason}`);
  }
  console.error(
    '\nResolve by:\n' +
      '  1. Splitting the file (preferred) — extract submodules / sub-components.\n' +
      '  2. If the growth is justified (e.g. a generated file or a single tightly-\n' +
      '     coupled module), open a PR that updates scripts/file-size-allowlist.json\n' +
      '     and explains why in the PR description. Allowlist entries are temporary —\n' +
      '     the long-term goal is for this file to shrink to zero entries.\n',
  );
  process.exit(1);
}

main().catch((err) => {
  console.error('check-file-sizes: fatal error:', err);
  process.exit(2);
});
