#!/usr/bin/env node
import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const repoRoot = process.cwd()
const statusPath = process.env.TERRANSOUL_MCP_FRONTEND_STATUS ?? path.join(repoRoot, 'mcp-data', 'frontend-build-status.json')
const logPath = process.env.TERRANSOUL_MCP_FRONTEND_BUILD_LOG ?? path.join(repoRoot, 'mcp-data', 'frontend-build.log')
const distIndex = process.env.TERRANSOUL_MCP_FRONTEND_DIST ?? path.join(repoRoot, 'dist', 'index.html')
const ifNeeded = process.argv.includes('--if-needed')

const frontendSources = [
  path.join(repoRoot, 'package.json'),
  path.join(repoRoot, 'package-lock.json'),
  path.join(repoRoot, 'index.html'),
  path.join(repoRoot, 'vite.config.ts'),
  path.join(repoRoot, 'tsconfig.json'),
  path.join(repoRoot, 'tsconfig.node.json'),
  path.join(repoRoot, 'src'),
  path.join(repoRoot, 'public'),
]

function newestMtimeMs(targetPath) {
  if (!fs.existsSync(targetPath)) return 0
  const stat = fs.statSync(targetPath)
  if (stat.isFile()) return stat.mtimeMs
  if (!stat.isDirectory()) return 0

  let newest = stat.mtimeMs
  for (const entry of fs.readdirSync(targetPath, { withFileTypes: true })) {
    newest = Math.max(newest, newestMtimeMs(path.join(targetPath, entry.name)))
  }
  return newest
}

function distIsFresh() {
  if (!fs.existsSync(distIndex)) return false
  const distMtime = fs.statSync(distIndex).mtimeMs
  const newestSource = frontendSources.reduce(
    (max, targetPath) => Math.max(max, newestMtimeMs(targetPath)),
    0,
  )
  return distMtime >= newestSource
}

function writeStatus(status, message, extra = {}) {
  fs.mkdirSync(path.dirname(statusPath), { recursive: true })
  fs.writeFileSync(statusPath, `${JSON.stringify({
    status,
    message,
    distIndex,
    updatedAt: new Date().toISOString(),
    ...extra,
  }, null, 2)}\n`)
}

if (ifNeeded && distIsFresh()) {
  writeStatus('ready', 'Frontend build is current.')
  console.log('[mcp-frontend] frontend build is current; skipping vite build.')
  process.exit(0)
}

writeStatus('building', 'Building frontend UI for MCP tray.')
fs.mkdirSync(path.dirname(logPath), { recursive: true })
const log = fs.createWriteStream(logPath, { flags: 'a' })
log.write(`\n[mcp-frontend] ${new Date().toISOString()} starting vite build\n`)

const viteBin = path.join(repoRoot, 'node_modules', 'vite', 'bin', 'vite.js')
const command = fs.existsSync(viteBin) ? process.execPath : process.platform === 'win32' ? 'cmd.exe' : 'npx'
const args = fs.existsSync(viteBin)
  ? [viteBin, 'build']
  : process.platform === 'win32'
    ? ['/d', '/s', '/c', 'npx vite build']
    : ['vite', 'build']

let child
try {
  log.write(`[mcp-frontend] command: ${command} ${args.join(' ')}\n`)
  child = spawn(command, args, {
    cwd: repoRoot,
    env: process.env,
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  })
} catch (error) {
  log.write(`[mcp-frontend] failed to spawn vite build: ${error.message}\n`)
  writeStatus('failed', `Frontend UI build failed to start: ${error.message}`)
  log.end()
  process.exit(1)
}

child.stdout.pipe(log, { end: false })
child.stderr.pipe(log, { end: false })

child.on('exit', (code, signal) => {
  log.write(`[mcp-frontend] ${new Date().toISOString()} exited code=${code ?? 'null'} signal=${signal ?? 'null'}\n`)
  if (code === 0) {
    writeStatus('ready', 'Frontend UI is ready.')
    log.end()
    return
  }
  writeStatus('failed', 'Frontend UI build failed.', { code, signal })
  log.end()
  process.exit(code ?? 1)
})

child.on('error', (error) => {
  log.write(`[mcp-frontend] failed to start vite build: ${error.message}\n`)
  writeStatus('failed', `Frontend UI build failed to start: ${error.message}`)
  log.end()
  process.exit(1)
})

process.on('uncaughtException', (error) => {
  log.write(`[mcp-frontend] uncaught error: ${error.message}\n`)
  writeStatus('failed', `Frontend UI build crashed: ${error.message}`)
  log.end()
  process.exit(1)
})