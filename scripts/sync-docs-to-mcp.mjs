#!/usr/bin/env node
/**
 * sync-docs-to-mcp.mjs — keep TerranSoul MCP brain in sync with the
 * canonical doc/rule/tutorial/instruction corpus.
 *
 * Walks a fixed set of repo directories, hashes each Markdown file, and
 * ingests **only changed/new** files into the running MCP brain via the
 * `brain_ingest_url` JSON-RPC tool (which routes through the same
 * chunking + embedding + KG pipeline as in-app ingestion). A small
 * manifest at `mcp-data/shared/doc-corpus.manifest.json` tracks
 * `path → sha256` so re-running this script is cheap and idempotent.
 *
 * Why: AI coding agents working in this repo can't keep every rule and
 * tutorial in their context window — the brain is the long-term memory.
 * If a doc isn't ingested, `brain_search` won't find it, and the agent
 * effectively forgets the rule.
 *
 * Usage:
 *   node scripts/sync-docs-to-mcp.mjs              # incremental
 *   node scripts/sync-docs-to-mcp.mjs --force      # re-ingest everything
 *   node scripts/sync-docs-to-mcp.mjs --dry-run    # report only
 *
 * Environment:
 *   TERRANSOUL_MCP_PORT   — default 7423 (headless MCP runner)
 *   TERRANSOUL_MCP_TOKEN  — bearer token; falls back to .vscode/.mcp-token
 */

import fs from 'node:fs'
import path from 'node:path'
import crypto from 'node:crypto'

const repoRoot = process.cwd()
const port = Number.parseInt(process.env.TERRANSOUL_MCP_PORT ?? '7423', 10)
const tokenPath = path.join(repoRoot, '.vscode', '.mcp-token')
const manifestPath = path.join(repoRoot, 'mcp-data', 'shared', 'doc-corpus.manifest.json')
const force = process.argv.includes('--force')
const dryRun = process.argv.includes('--dry-run')

// ─── Corpus definition ─────────────────────────────────────────────────────
// Each entry: { dir, recurse, tags, importance, ext }
// Order is the order things show up in agent search results when scores tie,
// so put the most authoritative rules first.
const corpus = [
  { dir: 'rules',                  recurse: true,  tags: 'rules,governance',     importance: 5, ext: '.md' },
  { dir: '.github/instructions',   recurse: true,  tags: 'instructions,agent',   importance: 5, ext: '.md' },
  { dir: 'docs',                   recurse: false, tags: 'docs,design',          importance: 4, ext: '.md' },
  { dir: 'tutorials',              recurse: false, tags: 'tutorial,user-guide',  importance: 4, ext: '.md' },
  { dir: 'instructions',           recurse: false, tags: 'instructions,how-to',  importance: 3, ext: '.md' },
]

// Top-level agent / contributor files
const rootFiles = [
  { file: 'README.md',     tags: 'readme,overview',           importance: 5 },
  { file: 'AGENTS.md',     tags: 'agents,instructions',       importance: 5 },
  { file: 'CLAUDE.md',     tags: 'agents,instructions',       importance: 5 },
  { file: 'CREDITS.md',    tags: 'credits,attribution',       importance: 3 },
  { file: 'SECURITY.md',   tags: 'security,policy',           importance: 4 },
]

// ─── Helpers ───────────────────────────────────────────────────────────────

function getToken() {
  if (process.env.TERRANSOUL_MCP_TOKEN) return process.env.TERRANSOUL_MCP_TOKEN.trim()
  if (!fs.existsSync(tokenPath)) {
    throw new Error(`MCP token not found. Set TERRANSOUL_MCP_TOKEN or write to ${tokenPath}.`)
  }
  return fs.readFileSync(tokenPath, 'utf8').trim()
}

function loadManifest() {
  if (!fs.existsSync(manifestPath)) return { version: 1, entries: {} }
  try {
    return JSON.parse(fs.readFileSync(manifestPath, 'utf8'))
  } catch {
    return { version: 1, entries: {} }
  }
}

function saveManifest(manifest) {
  fs.mkdirSync(path.dirname(manifestPath), { recursive: true })
  fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + '\n', 'utf8')
}

function hashFile(absPath) {
  const data = fs.readFileSync(absPath)
  return crypto.createHash('sha256').update(data).digest('hex')
}

function* walk(absDir, recurse) {
  if (!fs.existsSync(absDir)) return
  for (const entry of fs.readdirSync(absDir, { withFileTypes: true })) {
    const entryAbs = path.join(absDir, entry.name)
    if (entry.isDirectory()) {
      if (recurse) yield* walk(entryAbs, true)
    } else if (entry.isFile()) {
      yield entryAbs
    }
  }
}

function collectFiles() {
  const out = []
  for (const group of corpus) {
    const absDir = path.join(repoRoot, group.dir)
    for (const abs of walk(absDir, group.recurse)) {
      if (group.ext && !abs.endsWith(group.ext)) continue
      out.push({
        abs,
        rel: path.relative(repoRoot, abs).replaceAll('\\', '/'),
        tags: group.tags,
        importance: group.importance,
      })
    }
  }
  for (const r of rootFiles) {
    const abs = path.join(repoRoot, r.file)
    if (fs.existsSync(abs)) {
      out.push({
        abs,
        rel: r.file,
        tags: r.tags,
        importance: r.importance,
      })
    }
  }
  // Stable order for predictable diffs.
  out.sort((a, b) => a.rel.localeCompare(b.rel))
  return out
}

let nextId = 1
async function rpc(method, params) {
  const token = getToken()
  const body = JSON.stringify({ jsonrpc: '2.0', id: nextId++, method, params })
  const res = await fetch(`http://127.0.0.1:${port}/mcp`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` },
    body,
  })
  if (!res.ok) throw new Error(`MCP HTTP ${res.status}: ${await res.text()}`)
  const json = await res.json()
  if (json.error) throw new Error(`MCP error: ${JSON.stringify(json.error)}`)
  return json.result
}

async function ensureMcpHealthy() {
  const res = await fetch(`http://127.0.0.1:${port}/health`).catch(() => null)
  if (!res || !res.ok) {
    throw new Error(`MCP not reachable on http://127.0.0.1:${port}/health — start it with 'npm run mcp' first.`)
  }
  const j = await res.json()
  console.log(`[doc-sync] MCP healthy: provider=${j.brain_provider} model=${j.brain_model} memories=${j.memory_total}`)
  return j
}

async function ingestPath(item) {
  // brain_ingest_url tool — the underlying engine treats any
  // non-(http|https|crawl) source as a file path. Tags get a stable
  // `doc-corpus,<rel>` prefix so we can find/update entries later.
  const tags = `doc-corpus,${item.rel},${item.tags}`
  const result = await rpc('tools/call', {
    name: 'brain_ingest_url',
    arguments: { url: item.abs, tags, importance: item.importance },
  })
  // result.content[0].text is JSON-stringified IngestUrlResponse.
  const text = result?.content?.[0]?.text ?? ''
  return text
}

// ─── Main ──────────────────────────────────────────────────────────────────

async function main() {
  await ensureMcpHealthy()

  const files = collectFiles()
  console.log(`[doc-sync] Found ${files.length} doc files in corpus.`)

  const manifest = force ? { version: 1, entries: {} } : loadManifest()
  const next = { version: 1, entries: {} }

  let changed = 0
  let unchanged = 0
  let failed = 0

  for (const item of files) {
    const sha = hashFile(item.abs)
    next.entries[item.rel] = { sha, importance: item.importance, tags: item.tags }
    const prev = manifest.entries[item.rel]
    if (!force && prev && prev.sha === sha) {
      unchanged++
      continue
    }
    if (dryRun) {
      console.log(`[dry-run] would ingest: ${item.rel}`)
      changed++
      continue
    }
    try {
      await ingestPath(item)
      console.log(`[doc-sync] ingested: ${item.rel}`)
      changed++
    } catch (err) {
      console.error(`[doc-sync] FAILED ${item.rel}: ${err.message}`)
      failed++
      // Keep prior manifest entry so the next run retries this file.
      if (prev) next.entries[item.rel] = prev
    }
  }

  if (!dryRun) saveManifest(next)

  console.log('───────────────────────────────────────────────')
  console.log(`[doc-sync] changed: ${changed}  unchanged: ${unchanged}  failed: ${failed}`)
  console.log(`[doc-sync] manifest: ${path.relative(repoRoot, manifestPath)}`)
  if (failed > 0) process.exit(1)
}

main().catch((err) => {
  console.error('[doc-sync] fatal:', err.message)
  process.exit(2)
})
