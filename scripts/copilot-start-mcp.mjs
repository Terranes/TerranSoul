#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const repoRoot = process.cwd()
const port = Number.parseInt(process.env.TERRANSOUL_MCP_PORT ?? '7423', 10)
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

async function waitForDown(targetPort, seconds) {
  const deadline = Date.now() + seconds * 1000
  while (Date.now() < deadline) {
    if (!(await isHealthy(targetPort))) return true
    await new Promise((resolve) => setTimeout(resolve, 500))
  }
  return false
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

function stopManagedMcpProcess() {
  if (!fs.existsSync(pidPath)) {
    return false
  }

  const pidRaw = fs.readFileSync(pidPath, 'utf8').trim()
  const pid = Number.parseInt(pidRaw, 10)
  if (!Number.isFinite(pid) || pid <= 0) {
    fs.rmSync(pidPath, { force: true })
    return false
  }

  let stopped = false
  try {
    if (process.platform === 'win32') {
      process.kill(pid)
      stopped = true
    } else {
      try {
        process.kill(-pid, 'SIGTERM')
      } catch {
        process.kill(pid, 'SIGTERM')
      }
      stopped = true
    }
  } catch {
    // Process may already be gone; clear stale pid file below.
  }

  fs.rmSync(pidPath, { force: true })
  return stopped
}

function stopStaleTargetMcpProcesses() {
  if (process.platform !== 'win32') {
    return false
  }

  // Query Win32_Process for stable executable-path matching.
  // Get-Process Path filtering is less reliable across escaping/casing variations.
  const probe = spawnSync(
    'powershell',
    [
      '-NoProfile',
      '-Command',
      `(Get-CimInstance Win32_Process -Filter "Name='terransoul.exe'" -ErrorAction SilentlyContinue | Where-Object { $_.ExecutablePath -and $_.ExecutablePath -like '*target-mcp*release*terransoul.exe' } | Select-Object -ExpandProperty ProcessId) -join "\n"`,
    ],
    { cwd: repoRoot, env: process.env, encoding: 'utf8' },
  )

  if (probe.status !== 0) {
    return false
  }

  const pids = (probe.stdout ?? '')
    .split(/\r?\n/)
    .map((s) => Number.parseInt(s.trim(), 10))
    .filter((n) => Number.isFinite(n) && n > 0)

  if (pids.length === 0) {
    return false
  }

  let stopped = false
  for (const pid of pids) {
    const kill = spawnSync('taskkill', ['/PID', String(pid), '/F'], {
      cwd: repoRoot,
      env: process.env,
      stdio: 'ignore',
    })
    if (kill.status === 0) {
      stopped = true
    }
  }

  return stopped
}

function stopPortOwner(targetPort) {
  if (process.platform !== 'win32') {
    return false
  }

  const probe = spawnSync(
    'powershell',
    [
      '-NoProfile',
      '-Command',
      `(Get-NetTCPConnection -LocalAddress 127.0.0.1 -LocalPort ${targetPort} -State Listen -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess) -join "\n"`,
    ],
    { cwd: repoRoot, env: process.env, encoding: 'utf8' },
  )

  if (probe.status !== 0) {
    return false
  }

  const pids = (probe.stdout ?? '')
    .split(/\r?\n/)
    .map((s) => Number.parseInt(s.trim(), 10))
    .filter((n) => Number.isFinite(n) && n > 0)

  if (pids.length === 0) {
    return false
  }

  let stopped = false
  for (const pid of pids) {
    const kill = spawnSync('taskkill', ['/PID', String(pid), '/F'], {
      cwd: repoRoot,
      env: process.env,
      stdio: 'ignore',
    })
    if (kill.status === 0) {
      stopped = true
    }
  }

  return stopped
}

// Always launch/manage the dedicated MCP tray runtime on 7423 so its
// lifecycle and tray visibility are deterministic for coding sessions.

const targetOutdated = process.env.TERRANSOUL_MCP_SKIP_BUILD !== '1' && isTargetMcpOutdated()

if (await isHealthy(port)) {
  if (!targetOutdated) {
    console.log(`[copilot-mcp] MCP full UI runtime is already healthy on ${port}; reusing it.`)
    process.exit(0)
  }

  console.log('[copilot-mcp] detected stale target-mcp while MCP is running; terminating for rebuild/relaunch.')
  if (!(stopManagedMcpProcess() || stopPortOwner(port))) {
    console.error('[copilot-mcp] target-mcp is stale but no managed pid could be terminated; stop MCP manually and retry.')
    process.exit(1)
  }
  if (!(await waitForDown(port, 20))) {
    console.error('[copilot-mcp] target-mcp is stale and MCP did not shut down in time; stop it manually and retry.')
    process.exit(1)
  }
}

rotateLogIfNeeded(logPath)
const log = fs.openSync(logPath, 'a')

if (process.env.TERRANSOUL_MCP_SKIP_BUILD !== '1') {
  console.log(`[copilot-mcp] warming MCP Rust build (target-mcp) before startup; log=${logPath}`)
  let build = buildTargetMcp(log)
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
      console.log('[copilot-mcp] detected locked target-mcp binary on Windows; attempting managed MCP stop + rebuild retry.')
      const stopped = stopManagedMcpProcess() || stopStaleTargetMcpProcesses() || stopPortOwner(port)
      if (!stopped) {
        console.error('[copilot-mcp] could not terminate managed/stale MCP process for rebuild retry.')
      } else if (!(await waitForDown(port, 20))) {
        console.error('[copilot-mcp] managed MCP process did not shut down in time for rebuild retry.')
      } else {
        build = buildTargetMcp(log)
      }
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
console.log(`[copilot-mcp] started MCP full UI runtime as pid ${child.pid}; log=${logPath}`)

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

console.log(`[copilot-mcp] MCP full UI runtime is healthy on ${port}`)
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
