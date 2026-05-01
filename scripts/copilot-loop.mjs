#!/usr/bin/env node
/**
 * copilot-loop.mjs — Auto-resume Copilot agent sessions.
 *
 * This script is a lightweight wrapper that helps track and resume
 * long-running Copilot agent sessions. It works alongside VS Code's
 * built-in session management (chat.viewSessions.enabled) and
 * conversation history summarization.
 *
 * What it does:
 *   1. Reads the current milestones to identify the next chunk
 *   2. Generates a "Continue" prompt with context summary
 *   3. Copies the prompt to clipboard for easy paste into Copilot Chat
 *   4. Tracks session progress in a log file
 *
 * Usage:
 *   node scripts/copilot-loop.mjs [--status] [--next] [--log]
 *
 * Options:
 *   --status   Show current session progress
 *   --next     Generate and copy the next "Continue" prompt
 *   --log      Show the session log
 *   (default)  Interactive mode — shows status then generates prompt
 *
 * The auto-resume flow:
 *   1. Copilot runs a multi-chunk session with autopilot enabled
 *   2. When context fills up, VS Code auto-summarizes (built-in)
 *   3. If the session stalls, run this script to get a fresh prompt
 *   4. Paste into a new Copilot Chat session to resume
 *
 * Note: Actual session monitoring and auto-prompting requires VS Code
 * extension API access (not available from a script). This script
 * provides the manual-assist path. For full automation, see the
 * "Copilot: Continue Session" task in .vscode/tasks.json.
 */

import { readFileSync, appendFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');

// ── Config ─────────────────────────────────────────────────────────

const MILESTONES_PATH = join(root, 'rules', 'milestones.md');
const COMPLETION_LOG_PATH = join(root, 'rules', 'completion-log.md');
const SESSION_LOG_PATH = join(root, '.vscode', 'copilot-session.log');

// ── Helpers ────────────────────────────────────────────────────────

function readMilestones() {
  if (!existsSync(MILESTONES_PATH)) return { raw: '', chunks: [] };
  const raw = readFileSync(MILESTONES_PATH, 'utf-8');

  // Parse table rows for not-started chunks
  const chunks = [];
  for (const line of raw.split('\n')) {
    const m = line.match(
      /\|\s*[\d.]+\s*\|\s*\*\*(.+?)\*\*\s*[—–-]\s*(.+?)\s*\|\s*(not-started|in-progress)\s*\|/
    );
    if (m) {
      chunks.push({
        name: m[1].trim(),
        description: m[2].trim(),
        status: m[3].trim(),
      });
    }
  }
  return { raw, chunks };
}

function getRecentCompletions(n = 3) {
  if (!existsSync(COMPLETION_LOG_PATH)) return [];
  const raw = readFileSync(COMPLETION_LOG_PATH, 'utf-8');
  const entries = [];
  for (const line of raw.split('\n')) {
    const m = line.match(/\|\s*\[(.+?)\]\(.+?\)\s*\|\s*(\d{4}-\d{2}-\d{2})\s*\|/);
    if (m) entries.push({ name: m[1], date: m[2] });
    if (entries.length >= n) break;
  }
  return entries;
}

function getSessionCount() {
  if (!existsSync(SESSION_LOG_PATH)) return 0;
  return readFileSync(SESSION_LOG_PATH, 'utf-8')
    .split('\n')
    .filter((l) => l.startsWith('[')).length;
}

function logSession(action, detail) {
  const ts = new Date().toISOString();
  const line = `[${ts}] ${action}: ${detail}\n`;
  appendFileSync(SESSION_LOG_PATH, line);
}

function copyToClipboard(text) {
  try {
    if (process.platform === 'win32') {
      execSync('clip', { input: text });
    } else if (process.platform === 'darwin') {
      execSync('pbcopy', { input: text });
    } else {
      execSync('xclip -selection clipboard', { input: text });
    }
    return true;
  } catch {
    return false;
  }
}

// ── Commands ───────────────────────────────────────────────────────

function showStatus() {
  const { chunks } = readMilestones();
  const recent = getRecentCompletions();
  const sessions = getSessionCount();

  console.log('\n📊 TerranSoul Copilot Session Status');
  console.log('═'.repeat(50));

  if (recent.length > 0) {
    console.log('\n✅ Recently completed:');
    for (const r of recent) {
      console.log(`   ${r.date}  ${r.name}`);
    }
  }

  const inProgress = chunks.filter((c) => c.status === 'in-progress');
  const notStarted = chunks.filter((c) => c.status === 'not-started');

  if (inProgress.length > 0) {
    console.log('\n🔄 In progress:');
    for (const c of inProgress) {
      console.log(`   ${c.name} — ${c.description}`);
    }
  }

  if (notStarted.length > 0) {
    console.log(`\n📋 Next up (${notStarted.length} chunks):`);
    for (const c of notStarted.slice(0, 5)) {
      console.log(`   ${c.name} — ${c.description}`);
    }
    if (notStarted.length > 5) {
      console.log(`   ... and ${notStarted.length - 5} more`);
    }
  }

  console.log(`\n📝 Session log entries: ${sessions}`);
  console.log('');
}

function generateContinuePrompt() {
  const { chunks } = readMilestones();
  const recent = getRecentCompletions(2);
  const notStarted = chunks.filter((c) => c.status === 'not-started');
  const inProgress = chunks.filter((c) => c.status === 'in-progress');

  let prompt = 'Continue next chunks';

  // Add context about what was recently done
  if (recent.length > 0) {
    prompt += `. Recently completed: ${recent.map((r) => r.name).join(', ')}`;
  }

  // Add what's in progress
  if (inProgress.length > 0) {
    prompt += `. In progress: ${inProgress.map((c) => c.name).join(', ')}`;
  }

  // Suggest next chunk
  if (notStarted.length > 0) {
    prompt += `. Next: ${notStarted[0].name} — ${notStarted[0].description}`;
  }

  prompt +=
    '. Focus on long run task, auto resume, progress tracking, mcp server, self learning.';

  return prompt;
}

function next() {
  const prompt = generateContinuePrompt();

  console.log('\n📋 Generated continue prompt:');
  console.log('─'.repeat(50));
  console.log(prompt);
  console.log('─'.repeat(50));

  if (copyToClipboard(prompt)) {
    console.log('✅ Copied to clipboard — paste into Copilot Chat');
  } else {
    console.log('⚠️  Could not copy to clipboard — copy manually above');
  }

  logSession('PROMPT', prompt.substring(0, 100) + '...');
  console.log('');
}

function showLog() {
  if (!existsSync(SESSION_LOG_PATH)) {
    console.log('No session log yet.');
    return;
  }
  const log = readFileSync(SESSION_LOG_PATH, 'utf-8');
  console.log('\n📝 Session Log:');
  console.log('─'.repeat(50));
  console.log(log || '(empty)');
}

// ── Main ───────────────────────────────────────────────────────────

const args = process.argv.slice(2);

if (args.includes('--status')) {
  showStatus();
} else if (args.includes('--next')) {
  next();
} else if (args.includes('--log')) {
  showLog();
} else {
  // Interactive default: status + prompt
  showStatus();
  next();
}
