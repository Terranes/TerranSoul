#!/usr/bin/env node
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import process from 'node:process'
import readline from 'node:readline'

const workspaceRoot = process.env.TERRANSOUL_MCP_WORKSPACE || findWorkspaceRoot(process.cwd())
const waitSeconds = Number.parseInt(process.env.TERRANSOUL_MCP_PROXY_WAIT || '240', 10)
const requestTimeoutMs = Number.parseInt(process.env.TERRANSOUL_MCP_PROXY_REQUEST_TIMEOUT_MS || '0', 10)
const releasePort = 7421
const devPort = 7422
const trayPort = Number.parseInt(process.env.TERRANSOUL_MCP_PORT || '7423', 10)

function findWorkspaceRoot(start) {
  let current = path.resolve(start)
  while (true) {
    if (fs.existsSync(path.join(current, 'package.json')) && fs.existsSync(path.join(current, 'src-tauri'))) {
      return current
    }
    const parent = path.dirname(current)
    if (parent === current) return path.resolve(start)
    current = parent
  }
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
  for (const envName of candidate.tokenEnv || []) {
    const value = process.env[envName]?.trim()
    if (value) return value
  }

  for (const tokenPath of candidate.tokenPaths) {
    try {
      const value = fs.readFileSync(tokenPath, 'utf8').trim()
      if (value) return value
    } catch {
      // Try the next known token location.
    }
  }

  return null
}

function candidates() {
  const appRoot = path.join(appDataRoot(), 'com.terranes.terransoul')
  return [
    {
      label: 'release',
      port: releasePort,
      tokenEnv: ['TERRANSOUL_MCP_TOKEN'],
      tokenPaths: [path.join(appRoot, 'mcp-token.txt')],
    },
    {
      label: 'mcp-tray',
      port: trayPort,
      tokenEnv: ['TERRANSOUL_MCP_TOKEN_MCP'],
      tokenPaths: [
        path.join(workspaceRoot, 'mcp-data', 'mcp-token.txt'),
        path.join(workspaceRoot, '.vscode', '.mcp-token'),
      ],
    },
    {
      label: 'dev',
      port: devPort,
      tokenEnv: ['TERRANSOUL_MCP_TOKEN_DEV'],
      tokenPaths: [path.join(appRoot, 'dev', 'mcp-token.txt')],
    },
  ]
}

async function fetchJson(url, options = {}) {
  const controller = requestTimeoutMs > 0 ? new AbortController() : null
  const timer = controller
    ? setTimeout(() => controller.abort(), requestTimeoutMs)
    : null

  try {
    const response = await fetch(url, {
      ...options,
      signal: controller?.signal,
    })
    const text = await response.text()
    let body = null
    if (text.trim()) {
      try {
        body = JSON.parse(text)
      } catch {
        body = text
      }
    }
    return { response, body, text }
  } catch {
    return null
  } finally {
    if (timer) clearTimeout(timer)
  }
}

async function probeCandidate(candidate) {
  const token = readToken(candidate)
  if (!token) return null

  const status = await fetchJson(`http://127.0.0.1:${candidate.port}/status`, {
    headers: { Authorization: `Bearer ${token}` },
  })

  const name = typeof status?.body === 'object' && status.body !== null ? status.body.name : null
  if (!status?.response.ok || typeof name !== 'string' || !name.startsWith('terransoul-brain')) {
    return null
  }

  return {
    ...candidate,
    token,
    name,
    url: `http://127.0.0.1:${candidate.port}/mcp`,
  }
}

async function selectTarget(maxWaitSeconds = waitSeconds) {
  const deadline = Date.now() + Math.max(0, maxWaitSeconds) * 1000
  do {
    for (const candidate of candidates()) {
      const target = await probeCandidate(candidate)
      if (target) return target
    }

    if (Date.now() >= deadline) return null
    await new Promise((resolve) => setTimeout(resolve, 1000))
  } while (Date.now() < deadline)

  return null
}

function jsonRpcError(id, code, message) {
  return JSON.stringify({
    jsonrpc: '2.0',
    id,
    error: { code, message },
  })
}

function parseEnvelope(line) {
  try {
    const value = JSON.parse(line)
    return {
      ok: true,
      id: Object.prototype.hasOwnProperty.call(value, 'id') ? value.id : null,
      notification: !Object.prototype.hasOwnProperty.call(value, 'id'),
    }
  } catch (error) {
    return { ok: false, message: error.message }
  }
}

async function forwardLine(target, line) {
  const envelope = parseEnvelope(line)
  if (!envelope.ok) {
    process.stdout.write(`${jsonRpcError(null, -32700, `parse error: ${envelope.message}`)}\n`)
    return target
  }

  let active = target
  for (let attempt = 0; attempt < 2; attempt += 1) {
    const result = await fetchJson(active.url, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${active.token}`,
        'Content-Type': 'application/json',
      },
      body: line,
    })

    if (result?.response.status === 202 || envelope.notification) {
      if (!result?.response.ok && result?.response.status !== 202) {
        console.error(`[mcp-proxy] notification failed through ${active.label}: HTTP ${result?.response.status ?? 'unreachable'}`)
      }
      return active
    }

    if (result?.response.ok && result.text.trim()) {
      process.stdout.write(`${result.text.trimEnd()}\n`)
      return active
    }

    const refreshed = attempt === 0 ? await selectTarget(3) : null
    if (refreshed && (refreshed.port !== active.port || refreshed.token !== active.token)) {
      console.error(`[mcp-proxy] switched from ${active.label} to ${refreshed.label}`)
      active = refreshed
      continue
    }

    const status = result?.response.status ?? 'unreachable'
    const detail = typeof result?.body === 'string'
      ? result.body
      : JSON.stringify(result?.body ?? {})
    process.stdout.write(`${jsonRpcError(envelope.id, -32000, `TerranSoul MCP proxy request failed via ${active.label} (${status}): ${detail}`)}\n`)
    return active
  }

  return active
}

if (process.argv.includes('--probe')) {
  const target = await selectTarget(0)
  if (!target) {
    console.error('[mcp-proxy] no running TerranSoul MCP server found')
    process.exit(1)
  }
  console.log(`${target.label} ${target.name} ${target.url}`)
  process.exit(0)
}

let target = await selectTarget()
if (!target) {
  console.error('[mcp-proxy] no running TerranSoul MCP server found. Open the release app, run `npm run mcp` to start the MCP tray, or start the dev app.')
  process.exit(1)
}

console.error(`[mcp-proxy] connected to ${target.label} (${target.name}) on ${target.url}`)

const input = readline.createInterface({
  input: process.stdin,
  crlfDelay: Number.POSITIVE_INFINITY,
})

for await (const line of input) {
  const trimmed = line.trim()
  if (!trimmed) continue
  target = await forwardLine(target, trimmed)
}
