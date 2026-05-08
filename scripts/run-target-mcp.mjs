#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process'
import path from 'node:path'
import process from 'node:process'

const repoRoot = process.cwd()
const binary = path.join(
  repoRoot,
  'target-mcp',
  'release',
  process.platform === 'win32' ? 'terransoul.exe' : 'terransoul',
)
const binaryArgs = process.argv.slice(2)

const build = spawnSync(
  'cargo',
  ['build', '--release', '--manifest-path', 'src-tauri/Cargo.toml', '--target-dir', 'target-mcp'],
  {
    cwd: repoRoot,
    env: process.env,
    stdio: 'inherit',
  },
)

if (build.status !== 0) {
  process.exit(build.status ?? 1)
}

const child = spawn(binary, binaryArgs, {
  cwd: repoRoot,
  env: process.env,
  stdio: 'inherit',
})

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal)
    return
  }
  process.exit(code ?? 0)
})

child.on('error', (error) => {
  console.error(`[run-target-mcp] failed to start ${binary}: ${error.message}`)
  process.exit(1)
})
