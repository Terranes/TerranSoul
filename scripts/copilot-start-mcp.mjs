#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const repoRoot = process.cwd()
const port = Number.parseInt(process.env.TERRANSOUL_MCP_PORT ?? '7423', 10)
const releasePort = 7421
const devPort = 7422
const waitSeconds = Number.parseInt(process.argv.find((arg) => /^\d+$/.test(arg)) ?? '240', 10)
const smoke = process.argv.includes('--smoke')

// --resume <name> : pass a memorable session name to the MCP binary so
// it resumes prior context instead of starting a fresh session.
const resumeIdx = process.argv.indexOf('--resume')
const resumeName = resumeIdx >= 0 ? process.argv[resumeIdx + 1] ?? null : null

// --idle-timeout <secs> : override the default idle timeout.
// Default is 0 (no idle shutdown) so MCP stays available for coding sessions.
const idleIdx = process.argv.indexOf('--idle-timeout')
const idleTimeout = idleIdx >= 0 ? process.argv[idleIdx + 1] ?? '0' : '0'
const maxLogBytes = 1024 * 1024
const logPath = process.env.TERRANSOUL_MCP_LOG ?? path.join(repoRoot, 'mcp-data', 'self_improve_mcp_process.log')
const pidPath = process.env.TERRANSOUL_MCP_PID ?? path.join(repoRoot, 'mcp-data', 'self_improve_mcp_process.pid')
const mcpBinary = path.join(repoRoot, 'target-mcp', 'release', process.platform === 'win32' ? 'terransoul.exe' : 'terransoul')
const sourceRootsForFreshness = [
  path.join(repoRoot, 'src-tauri', 'Cargo.toml'),
  path.join(repoRoot, 'src-tauri', 'Cargo.lock'),
  path.join(repoRoot, 'src-tauri', 'build.rs'),
  path.join(repoRoot, 'src-tauri', 'tauri.conf.json'),
  path.join(repoRoot, 'src-tauri', 'src'),
  path.join(repoRoot, 'dist'),
]

function buildTargetMcp(logFd) {
  return spawnSync('cargo', ['build', '--release', '--no-default-features', '--features', 'headless-mcp', '--manifest-path', 'src-tauri/Cargo.toml', '--target-dir', 'target-mcp'], {
    cwd: repoRoot,
    env: process.env,
    stdio: ['ignore', logFd, logFd],
  })
}

function archivePath(filePath) {
  return `${filePath}.001`
}

function rotateLogIfNeeded(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true })
  const archive = archivePath(filePath)
  const prefix = `${path.basename(filePath)}.`

  for (const entry of fs.readdirSync(path.dirname(filePath), { withFileTypes: true })) {
    if (!entry.isFile()) continue
    const entryPath = path.join(path.dirname(filePath), entry.name)
    if (entry.name !== path.basename(archive) && entry.name.startsWith(prefix)) {
      fs.rmSync(entryPath, { force: true })
    }
  }

  if (!fs.existsSync(filePath) || fs.statSync(filePath).size < maxLogBytes) return
  fs.rmSync(archive, { force: true })
  fs.renameSync(filePath, archive)
}

async function get(url) {
  try {
    const response = await fetch(url)
    if (!response.ok) return null
    return await response.text()
  } catch {
    return null
  }
}

async function getJson(url, token) {
  try {
    const response = await fetch(url, {
      headers: token ? { Authorization: `Bearer ${token}` } : {},
    })
    if (!response.ok) return null
    return await response.json()
  } catch {
    return null
  }
}

async function isHealthy(targetPort) {
  return (await get(`http://127.0.0.1:${targetPort}/health`)) !== null
}

async function waitForHealth(targetPort, seconds) {
  const deadline = Date.now() + seconds * 1000
  while (Date.now() < deadline) {
    if (await isHealthy(targetPort)) return true
    await new Promise((resolve) => setTimeout(resolve, 1000))
  }
  return false
}

function appDataRoot() {
  if (process.platform === 'win32') {
    return process.env.APPDATA || path.join(os.homedir(), 'AppData', 'Roaming')
  }
  if (process.platform === 'darwin') {
    return path.join(os.homedir(), 'Library', 'Application Support')
  }
  return process.env.XDG_DATA_HOME || path.join(os.homedir(), '.local', 'share')
}

function readToken(candidate) {
  for (const envName of candidate.tokenEnv ?? []) {
    const token = process.env[envName]?.trim()
    if (token) return token
  }

  for (const tokenPath of candidate.tokenPaths) {
    try {
      const token = fs.readFileSync(tokenPath, 'utf8').trim()
      if (token) return token
    } catch {
      // Try the next known token location.
    }
  }

  return null
}

function serverCandidates() {
  const appRoot = path.join(appDataRoot(), 'com.terranes.terransoul')
  return [
    {
      label: 'release app',
      buildMode: 'release',
      port: releasePort,
      tokenEnv: ['TERRANSOUL_MCP_TOKEN'],
      tokenPaths: [path.join(appRoot, 'mcp-token.txt')],
    },
    {
      label: 'MCP tray',
      buildMode: 'mcp',
      port,
      tokenEnv: ['TERRANSOUL_MCP_TOKEN_MCP'],
      tokenPaths: [
        path.join(repoRoot, 'mcp-data', 'mcp-token.txt'),
        path.join(repoRoot, '.vscode', '.mcp-token'),
      ],
    },
    {
      label: 'dev app',
      buildMode: 'dev',
      port: devPort,
      tokenEnv: ['TERRANSOUL_MCP_TOKEN_DEV'],
      tokenPaths: [path.join(appRoot, 'dev', 'mcp-token.txt')],
    },
  ]
}

async function probeExistingMcpServer(candidate) {
  const token = readToken(candidate)
  if (!token) return null

  const status = await getJson(`http://127.0.0.1:${candidate.port}/status`, token)
  if (!status || typeof status.name !== 'string' || !status.name.startsWith('terransoul-brain')) {
    return null
  }

  return {
    ...candidate,
    token,
    name: status.name,
    actualPort: status.actual_port ?? candidate.port,
  }
}

async function findExistingMcpServer() {
  for (const candidate of serverCandidates()) {
    const server = await probeExistingMcpServer(candidate)
    if (server) return server
  }
  return null
}

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

function isTargetMcpOutdated() {
  if (!fs.existsSync(mcpBinary)) return true
  const binaryMtime = fs.statSync(mcpBinary).mtimeMs
  const newestSource = sourceRootsForFreshness.reduce(
    (max, targetPath) => Math.max(max, newestMtimeMs(targetPath)),
    0,
  )
  return newestSource > binaryMtime
}

// Prefer release first, then an already-open MCP tray, then dev.
// Only build/start target-mcp when nothing usable is serving MCP yet.

const existingServer = await findExistingMcpServer()
if (existingServer) {
  console.log(
    `[copilot-mcp] ${existingServer.label} is already serving MCP as ${existingServer.name} on ${existingServer.actualPort}; reusing it.`,
  )
  if (smoke) {
    console.log('[copilot-mcp] smoke mode reused the existing MCP server; nothing to stop.')
  }
  process.exit(0)
}

if (await isHealthy(port)) {
  console.error(
    `[copilot-mcp] something is listening on ${port}, but TerranSoul /status did not authenticate with the known tray token.`,
  )
  console.error('[copilot-mcp] leave the existing tray running; check mcp-data/mcp-token.txt or .vscode/.mcp-token, then retry.')
  process.exit(1)
}

const targetOutdated = process.env.TERRANSOUL_MCP_SKIP_BUILD !== '1' && isTargetMcpOutdated()

rotateLogIfNeeded(logPath)
const log = fs.openSync(logPath, 'a')

if (targetOutdated) {
  console.log(`[copilot-mcp] warming MCP Rust build (target-mcp) before startup; log=${logPath}`)
  const build = buildTargetMcp(log)
  if (build.status !== 0) {
    const tail1 = fs.existsSync(logPath)
      ? fs.readFileSync(logPath, 'utf8').split('\n').slice(-120).join('\n')
      : ''
    const lockError =
      process.platform === 'win32' &&
      tail1.includes('failed to remove file') &&
      tail1.includes('target-mcp\\release\\terransoul.exe') &&
      tail1.includes('Access is denied')

    if (lockError) {
      console.error('[copilot-mcp] target-mcp binary is locked, but no authenticated MCP server was reusable.')
      console.error('[copilot-mcp] keep the tray open if it is intentional; otherwise close stale TerranSoul processes manually and retry.')
    }
  }
  if (build.status !== 0) {
    const tail = fs.existsSync(logPath)
      ? fs.readFileSync(logPath, 'utf8').split('\n').slice(-80).join('\n')
      : ''
    console.error(`[copilot-mcp] cargo build failed with exit code ${build.status ?? 'unknown'}`)
    if (tail.trim()) {
      console.error(`[copilot-mcp] log tail:\n${tail}`)
    }
    process.exit(build.status ?? 1)
  }
} else if (process.env.TERRANSOUL_MCP_SKIP_BUILD === '1') {
  console.log('[copilot-mcp] TERRANSOUL_MCP_SKIP_BUILD=1; using existing target-mcp binary.')
} else {
  console.log('[copilot-mcp] target-mcp release binary is current; skipping cargo build.')
}

const childArgs = ['--mcp-tray']
if (resumeName) childArgs.push('--resume', resumeName)

const childEnv = {
  ...process.env,
  TERRANSOUL_MCP_PORT: String(port),
  TERRANSOUL_MCP_IDLE_TIMEOUT: idleTimeout,
}

const child = spawn(mcpBinary, childArgs, {
  cwd: repoRoot,
  detached: true,
  env: childEnv,
  stdio: ['ignore', log, log],
})
child.unref()
fs.writeFileSync(pidPath, `${child.pid}\n`)
console.log(`[copilot-mcp] started MCP tray runtime as pid ${child.pid}; log=${logPath}`)

if (!(await waitForHealth(port, waitSeconds))) {
  const tail = fs.existsSync(logPath)
    ? fs.readFileSync(logPath, 'utf8').split('\n').slice(-80).join('\n')
    : ''
  console.error(`[copilot-mcp] timed out waiting for http://127.0.0.1:${port}/health`)
  if (tail.trim()) {
    console.error(`[copilot-mcp] log tail:\n${tail}`)
  }
  process.exit(1)
}

console.log(`[copilot-mcp] MCP tray runtime is healthy on ${port}`)
for (const tokenPath of ['.vscode/.mcp-token', 'mcp-data/mcp-token.txt']) {
  if (fs.existsSync(tokenPath)) {
    console.log(`[copilot-mcp] token available at ${tokenPath}`)
  }
}

if (smoke) {
  try {
    if (process.platform === 'win32') {
      process.kill(child.pid)
    } else {
      process.kill(-child.pid, 'SIGTERM')
    }
    console.log(`[copilot-mcp] smoke mode stopped MCP process group ${child.pid}`)
  } catch (error) {
    console.log(`[copilot-mcp] smoke mode could not stop MCP process group ${child.pid}: ${error.message}`)
  }
}
