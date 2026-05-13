#!/usr/bin/env node
// Kill stale `npm run dev` / `cargo tauri dev` TerranSoul processes WITHOUT
// touching the MCP tray / headless MCP runtime.
//
// Rationale: `npm run dev` previously did `taskkill /IM terransoul.exe /F`
// which kills every TerranSoul process including the MCP tray
// (`target-mcp\release\terransoul.exe` on :7423). The MCP tray must stay
// alive across dev restarts so AI coding agents keep their brain context.
//
// We filter by ExecutablePath so only processes from `target\debug\` or
// `target\release\` (the regular Tauri app build dirs) are killed.
// `target-mcp\…`, `target-test\…`, and `target-tauri-desktop-e2e\…` are
// left running.

import { spawnSync } from 'node:child_process'

const PROTECT_PATH_FRAGMENTS = [
  'target-mcp',
  'target-test',
  'target-tauri-desktop-e2e',
  'target-copilot-bench',
  'target-ci',
]

const KILL_PATH_FRAGMENTS = [
  'target\\debug\\',
  'target\\release\\',
  'target/debug/',
  'target/release/',
]

const isWindows = process.platform === 'win32'

if (!isWindows) {
  // On non-Windows, fall back to pkill targeted at the regular target dir.
  // Right now TerranSoul dev workflow is Windows-only per the npm scripts,
  // so this is a stub for portability.
  const res = spawnSync('pkill', ['-f', 'target/(debug|release)/terransoul'], {
    stdio: 'inherit',
  })
  process.exit(res.status === 1 ? 0 : (res.status ?? 0))
}

const ps = spawnSync(
  'powershell',
  [
    '-NoProfile',
    '-ExecutionPolicy',
    'Bypass',
    '-Command',
    "Get-CimInstance Win32_Process -Filter \"Name='terransoul.exe'\" -ErrorAction SilentlyContinue | Select-Object ProcessId, ExecutablePath | ConvertTo-Json -Compress",
  ],
  { encoding: 'utf8' },
)

if (ps.status !== 0) {
  // No processes / error — silently succeed so `npm run dev` keeps going.
  process.exit(0)
}

const raw = (ps.stdout || '').trim()
if (!raw) process.exit(0)

let entries
try {
  const parsed = JSON.parse(raw)
  entries = Array.isArray(parsed) ? parsed : [parsed]
} catch {
  process.exit(0)
}

const toKill = []
const protectedPids = []

for (const entry of entries) {
  if (!entry || typeof entry.ProcessId !== 'number') continue
  const path = String(entry.ExecutablePath || '')
  const lower = path.toLowerCase()

  const isProtected = PROTECT_PATH_FRAGMENTS.some(frag =>
    lower.includes(frag.toLowerCase()),
  )
  if (isProtected) {
    protectedPids.push({ pid: entry.ProcessId, path })
    continue
  }

  const isDevBuild = KILL_PATH_FRAGMENTS.some(frag =>
    lower.includes(frag.toLowerCase()),
  )
  // If we can't tell (empty path / access denied), be conservative and skip.
  if (!isDevBuild) continue

  toKill.push({ pid: entry.ProcessId, path })
}

if (protectedPids.length > 0) {
  for (const { pid, path } of protectedPids) {
    console.log(`[kill-dev-terransoul] keep alive: pid=${pid} ${path}`)
  }
}

if (toKill.length === 0) {
  process.exit(0)
}

for (const { pid, path } of toKill) {
  console.log(`[kill-dev-terransoul] killing dev build: pid=${pid} ${path}`)
  spawnSync('taskkill', ['/PID', String(pid), '/F', '/T'], { stdio: 'ignore' })
}

process.exit(0)
