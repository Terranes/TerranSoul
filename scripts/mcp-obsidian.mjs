#!/usr/bin/env node
/**
 * mcp-obsidian.mjs — Open the MCP brain vault in Obsidian.
 *
 * Usage: npm run mcp-obsidian
 *
 * - Ensures the vault directory exists at mcp-data/wiki/TerranSoul/
 * - Auto-installs Obsidian if not found (winget on Windows, brew on
 *   macOS, flatpak/snap on Linux)
 * - Opens the vault using the `obsidian://open` URI scheme
 *
 * The vault is populated automatically by the MCP maintenance
 * scheduler (ObsidianExport job) whenever the headless MCP server
 * or the Tauri app is running.
 */

import { execSync, spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const repoRoot = process.cwd()
const vaultDir = path.join(repoRoot, 'mcp-data', 'wiki', 'TerranSoul')
const platform = process.platform

// ── Helpers ──────────────────────────────────────────────────────────

function which(cmd) {
  try {
    const out = execSync(
      platform === 'win32' ? `where ${cmd} 2>nul` : `command -v ${cmd} 2>/dev/null`,
      { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] },
    )
    return out.trim().split(/\r?\n/)[0] || null
  } catch {
    return null
  }
}

function run(cmd, opts = {}) {
  console.log(`[mcp-obsidian] $ ${cmd}`)
  try {
    execSync(cmd, { stdio: 'inherit', ...opts })
    return true
  } catch {
    return false
  }
}

function isObsidianInstalled() {
  if (platform === 'win32') {
    // Check common install paths + winget list
    const paths = [
      path.join(process.env.LOCALAPPDATA || '', 'Obsidian', 'Obsidian.exe'),
      path.join(process.env.PROGRAMFILES || '', 'Obsidian', 'Obsidian.exe'),
    ]
    if (paths.some((p) => fs.existsSync(p))) return true
    try {
      const out = execSync('winget list --id Obsidian.Obsidian 2>nul', {
        encoding: 'utf8',
        stdio: ['pipe', 'pipe', 'pipe'],
      })
      return out.includes('Obsidian')
    } catch {
      return false
    }
  }
  if (platform === 'darwin') {
    return (
      fs.existsSync('/Applications/Obsidian.app') ||
      fs.existsSync(path.join(process.env.HOME || '', 'Applications', 'Obsidian.app'))
    )
  }
  // Linux: check flatpak, snap, or PATH
  return !!(which('obsidian') || which('flatpak') && (() => {
    try {
      return execSync('flatpak list --app 2>/dev/null', {
        encoding: 'utf8',
        stdio: ['pipe', 'pipe', 'pipe'],
      }).includes('md.obsidian.Obsidian')
    } catch {
      return false
    }
  })())
}

function installObsidian() {
  console.log('[mcp-obsidian] Obsidian not found — installing...')
  if (platform === 'win32') {
    if (which('winget')) {
      if (run('winget install --id Obsidian.Obsidian --accept-package-agreements --accept-source-agreements')) return true
    }
    console.error('[mcp-obsidian] Please install Obsidian from https://obsidian.md/download')
    return false
  }
  if (platform === 'darwin') {
    if (which('brew')) {
      if (run('brew install --cask obsidian')) return true
    }
    console.error('[mcp-obsidian] Please install Obsidian: brew install --cask obsidian')
    return false
  }
  // Linux
  if (which('flatpak')) {
    if (run('flatpak install -y flathub md.obsidian.Obsidian')) return true
  }
  if (which('snap')) {
    if (run('sudo snap install obsidian --classic')) return true
  }
  console.error('[mcp-obsidian] Please install Obsidian from https://obsidian.md/download')
  return false
}

function openVault(vaultPath) {
  // Obsidian URI: obsidian://open?path=<absolute-path>
  const uri = `obsidian://open?path=${encodeURIComponent(vaultPath)}`

  if (platform === 'win32') {
    spawn('cmd', ['/c', 'start', '', uri], { detached: true, stdio: 'ignore' }).unref()
  } else if (platform === 'darwin') {
    spawn('open', [uri], { detached: true, stdio: 'ignore' }).unref()
  } else {
    // Linux: xdg-open or flatpak
    const opener = which('xdg-open') ? 'xdg-open' : 'flatpak'
    if (opener === 'flatpak') {
      spawn('flatpak', ['run', 'md.obsidian.Obsidian', uri], { detached: true, stdio: 'ignore' }).unref()
    } else {
      spawn(opener, [uri], { detached: true, stdio: 'ignore' }).unref()
    }
  }
}

// ── Main ─────────────────────────────────────────────────────────────

// 1. Ensure the vault directory exists (maintenance job creates files,
//    but we need the folder for Obsidian to open)
fs.mkdirSync(vaultDir, { recursive: true })

// 2. Ensure .obsidian config dir exists (tells Obsidian this is a vault)
const obsidianConfigDir = path.join(vaultDir, '..', '.obsidian')
if (!fs.existsSync(obsidianConfigDir)) {
  fs.mkdirSync(obsidianConfigDir, { recursive: true })
  // Write minimal Obsidian config so it recognises the vault
  fs.writeFileSync(
    path.join(obsidianConfigDir, 'app.json'),
    JSON.stringify({ readableLineLength: true, showFrontmatter: true }, null, 2),
  )
  fs.writeFileSync(
    path.join(obsidianConfigDir, 'appearance.json'),
    JSON.stringify({ baseFontSize: 16, theme: 'obsidian' }, null, 2),
  )
}

// 3. Install Obsidian if needed
if (!isObsidianInstalled()) {
  if (!installObsidian()) {
    process.exit(1)
  }
  // Give installer a moment to register URI handler
  console.log('[mcp-obsidian] Waiting for Obsidian installation to settle...')
  await new Promise((r) => setTimeout(r, 3000))
}

// 4. Open the vault
const wikiRoot = path.join(repoRoot, 'mcp-data', 'wiki')
console.log(`[mcp-obsidian] Opening vault at ${wikiRoot}`)
openVault(wikiRoot)
console.log('[mcp-obsidian] Obsidian launched. The vault syncs automatically via MCP maintenance.')
