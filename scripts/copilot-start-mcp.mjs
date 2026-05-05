#!/usr/bin/env node
import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const repoRoot = process.cwd()
const port = Number.parseInt(process.env.TERRANSOUL_MCP_PORT ?? '7423', 10)
const waitSeconds = Number.parseInt(process.argv[2] ?? '240', 10)
const logPath = process.env.TERRANSOUL_MCP_LOG ?? '/tmp/terransoul-mcp-copilot.log'
const pidPath = process.env.TERRANSOUL_MCP_PID ?? '/tmp/terransoul-mcp-copilot.pid'

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

for (const appPort of [7421, 7422]) {
  if (await isHealthy(appPort)) {
    console.log(`[copilot-mcp] TerranSoul app MCP is already healthy on ${appPort}; reusing it.`)
    process.exit(0)
  }
}

if (await isHealthy(port)) {
  console.log(`[copilot-mcp] headless MCP is already healthy on ${port}; reusing it.`)
  process.exit(0)
}

fs.mkdirSync(path.dirname(logPath), { recursive: true })
const log = fs.openSync(logPath, 'a')
const child = spawn('npm', ['run', 'mcp'], {
  cwd: repoRoot,
  detached: true,
  env: { ...process.env, TERRANSOUL_MCP_PORT: String(port) },
  stdio: ['ignore', log, log],
})
child.unref()
fs.writeFileSync(pidPath, `${child.pid}\n`)
console.log(`[copilot-mcp] started npm run mcp as pid ${child.pid}; log=${logPath}`)

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

console.log(`[copilot-mcp] headless MCP is healthy on ${port}`)
for (const tokenPath of ['.vscode/.mcp-token', 'mcp-data/mcp-token.txt']) {
  if (fs.existsSync(tokenPath)) {
    console.log(`[copilot-mcp] token available at ${tokenPath}`)
  }
}
